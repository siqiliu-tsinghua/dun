use std::collections::VecDeque;
use std::time::{Duration, Instant};

use dun_core::decode_file_text;
use dun_term::AmbiguousWidth;

use crate::terminal::clipboard::base64_decode;

use super::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

mod keys;
mod mouse;

#[cfg(test)]
mod tests;

const ESCAPE_TIMEOUT: Duration = Duration::from_millis(100);
const OSC_FRAME_TIMEOUT: Duration = Duration::from_millis(100);
const CSI_BODY_CAPACITY: usize = 30;
const PROBE_RESPONSE_CAPACITY: usize = 256;
const PASTE_CAPACITY: usize = 16 * 1024 * 1024;
const EVENT_QUEUE_CAPACITY: usize = 1_024;
const PASTE_END: &[u8; 6] = b"\x1b[201~";
const RETAINED_EVENT_VARIANTS: (KeyCode, KeyEventKind, KeyEventKind) =
    (KeyCode::Null, KeyEventKind::Repeat, KeyEventKind::Release);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Input,
    Probe,
}

enum State {
    Ground,
    Escape {
        deadline: Instant,
    },
    Ss3,
    Csi {
        bytes: [u8; CSI_BODY_CAPACITY],
        len: usize,
    },
    OversizedCsi,
    Utf8 {
        bytes: [u8; 4],
        len: usize,
        expected: usize,
        modifiers: KeyModifiers,
    },
    Paste {
        matched: usize,
    },
    DiscardPaste {
        matched: usize,
    },
    OscPrefix {
        matched: usize,
        deadline: Instant,
    },
    Osc52 {
        st_pending: bool,
        deadline: Instant,
    },
    DiscardOsc {
        st_pending: bool,
        deadline: Instant,
    },
    DiscardX10 {
        remaining: u8,
    },
}

struct ProbeState {
    total_bytes: usize,
    cpr: Option<AmbiguousWidth>,
    malformed: bool,
    result: Option<AmbiguousWidth>,
}

impl ProbeState {
    const fn new() -> Self {
        Self {
            total_bytes: 0,
            cpr: None,
            malformed: false,
            result: None,
        }
    }
}

pub(crate) struct Parser {
    mode: Mode,
    state: State,
    events: VecDeque<Event>,
    paste: Vec<u8>,
    osc52_payload: Vec<u8>,
    osc52_limit: Option<usize>,
    probe: ProbeState,
}

impl Parser {
    pub(crate) fn new(mode: Mode) -> Self {
        // Step 3 deliberately retained all owned event variants, although
        // parser-originated events in this step are always Press.
        let _ = RETAINED_EVENT_VARIANTS;
        Self {
            mode,
            state: State::Ground,
            events: VecDeque::with_capacity(EVENT_QUEUE_CAPACITY),
            paste: Vec::new(),
            osc52_payload: Vec::new(),
            osc52_limit: None,
            probe: ProbeState::new(),
        }
    }

    pub(crate) fn feed(&mut self, bytes: &[u8], now: Instant) {
        self.expire_escape(now);
        for &byte in bytes {
            if !self.start_probe_byte() {
                break;
            }
            let state = std::mem::replace(&mut self.state, State::Ground);
            self.state = self.step(state, byte, now);
            self.finish_probe_byte();
            if self.probe.result.is_some() {
                break;
            }
        }
    }

    pub(crate) fn pop_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    pub(crate) fn pop_osc52_response(&mut self) -> Option<String> {
        let index = self
            .events
            .iter()
            .position(|event| matches!(event, Event::Osc52Clipboard(_)))?;
        match self.events.remove(index)? {
            Event::Osc52Clipboard(text) => Some(text),
            _ => None,
        }
    }

    pub(crate) fn begin_osc52_query(&mut self, max_bytes: usize) {
        self.cancel_osc52_query();
        self.osc52_limit = Some(max_bytes);
    }

    pub(crate) fn cancel_osc52_query(&mut self) {
        self.osc52_limit = None;
        self.osc52_payload.clear();
        if matches!(
            &self.state,
            State::OscPrefix { .. } | State::Osc52 { .. } | State::DiscardOsc { .. }
        ) {
            self.state = State::Ground;
        }
    }

    pub(crate) fn pending_escape_deadline(&self) -> Option<Instant> {
        if self.mode != Mode::Input {
            return None;
        }
        match self.state {
            State::Escape { deadline }
            | State::OscPrefix { deadline, .. }
            | State::Osc52 { deadline, .. }
            | State::DiscardOsc { deadline, .. } => Some(deadline),
            _ => None,
        }
    }

    pub(crate) fn expire_escape(&mut self, now: Instant) {
        let (deadline, is_escape) = match &self.state {
            State::Escape { deadline } => (*deadline, true),
            State::OscPrefix { deadline, .. }
            | State::Osc52 { deadline, .. }
            | State::DiscardOsc { deadline, .. } => (*deadline, false),
            _ => return,
        };
        if self.mode == Mode::Input && now >= deadline {
            self.state = State::Ground;
            if is_escape {
                self.push_key(KeyCode::Esc, KeyModifiers::NONE);
            } else {
                self.osc52_payload.clear();
                self.osc52_limit = None;
            }
        }
    }

    pub(crate) fn probe_remaining_capacity(&self) -> usize {
        PROBE_RESPONSE_CAPACITY.saturating_sub(self.probe.total_bytes)
    }

    pub(crate) const fn probe_result(&self) -> Option<AmbiguousWidth> {
        self.probe.result
    }

    pub(crate) const fn finish_probe(&self) -> AmbiguousWidth {
        AmbiguousWidth::Narrow
    }

    fn step(&mut self, state: State, byte: u8, now: Instant) -> State {
        match state {
            State::Ground => self.step_ground(byte, now),
            State::Escape { .. } => self.step_escape(byte, now),
            State::Ss3 => self.step_ss3(byte, now),
            State::Csi { bytes, len } => self.step_csi(bytes, len, byte, now),
            State::OversizedCsi => self.step_oversized_csi(byte, now),
            State::Utf8 {
                bytes,
                len,
                expected,
                modifiers,
            } => self.step_utf8(bytes, len, expected, modifiers, byte),
            State::Paste { matched } => self.step_paste(matched, byte),
            State::DiscardPaste { matched } => self.step_discard_paste(matched, byte),
            State::OscPrefix { matched, deadline } => self.step_osc_prefix(matched, deadline, byte),
            State::Osc52 {
                st_pending,
                deadline,
            } => self.step_osc52(st_pending, deadline, byte),
            State::DiscardOsc {
                st_pending,
                deadline,
            } => self.step_discard_osc(st_pending, deadline, byte),
            State::DiscardX10 { remaining } => {
                if remaining == 1 {
                    State::Ground
                } else {
                    State::DiscardX10 {
                        remaining: remaining - 1,
                    }
                }
            }
        }
    }

    fn step_ground(&mut self, byte: u8, now: Instant) -> State {
        if byte == b'\x1b' {
            return self.escape(now);
        }
        if self.mode == Mode::Probe {
            return State::Ground;
        }
        self.step_input_byte(byte, KeyModifiers::NONE)
    }

    fn step_escape(&mut self, byte: u8, now: Instant) -> State {
        if byte == b'[' {
            return State::Csi {
                bytes: [0; CSI_BODY_CAPACITY],
                len: 0,
            };
        }
        if self.mode == Mode::Probe {
            return if byte == b'\x1b' {
                self.escape(now)
            } else {
                State::Ground
            };
        }
        match byte {
            b'O' => State::Ss3,
            b']' if self.osc52_limit.is_some() => State::OscPrefix {
                matched: 0,
                deadline: now + OSC_FRAME_TIMEOUT,
            },
            b'\x1b' => {
                self.push_key(KeyCode::Esc, KeyModifiers::NONE);
                self.escape(now)
            }
            _ => self.step_input_byte(byte, KeyModifiers::ALT),
        }
    }

    fn step_ss3(&mut self, byte: u8, now: Instant) -> State {
        if byte == b'\x1b' {
            return self.escape(now);
        }
        if let Some(event) = keys::parse_ss3(byte) {
            self.push_event(Event::Key(event));
        }
        State::Ground
    }

    fn step_csi(
        &mut self,
        mut bytes: [u8; CSI_BODY_CAPACITY],
        len: usize,
        byte: u8,
        now: Instant,
    ) -> State {
        if byte == b'\x1b' {
            self.mark_probe_malformed();
            return self.escape(now);
        }
        if len == 1 && bytes[0] == b'[' {
            if let Some(event) = keys::parse_legacy_double_bracket(byte) {
                if self.mode == Mode::Input {
                    self.push_event(Event::Key(event));
                }
            } else {
                self.mark_probe_malformed();
            }
            return State::Ground;
        }
        if len == CSI_BODY_CAPACITY {
            self.mark_probe_malformed();
            return if is_csi_final(byte) {
                State::Ground
            } else {
                State::OversizedCsi
            };
        }
        if !is_csi_parameter_or_intermediate(byte) && !is_csi_final(byte) {
            self.mark_probe_malformed();
            return State::OversizedCsi;
        }

        bytes[len] = byte;
        let new_len = len + 1;
        if len == 0 && byte == b'[' {
            return State::Csi {
                bytes,
                len: new_len,
            };
        }
        if !is_csi_final(byte) {
            return State::Csi {
                bytes,
                len: new_len,
            };
        }

        let body = &bytes[..new_len];
        if !is_syntactically_valid_csi(body) {
            self.mark_probe_malformed();
            return State::Ground;
        }
        self.finish_csi(body)
    }

    fn step_oversized_csi(&mut self, byte: u8, now: Instant) -> State {
        if byte == b'\x1b' {
            self.escape(now)
        } else if is_csi_final(byte) {
            State::Ground
        } else {
            State::OversizedCsi
        }
    }

    fn finish_csi(&mut self, body: &[u8]) -> State {
        if body == b"M" {
            return State::DiscardX10 { remaining: 3 };
        }
        match self.mode {
            Mode::Input => {
                if body == b"200~" {
                    self.paste.clear();
                    return State::Paste { matched: 0 };
                }
                if body.first() == Some(&b'<') {
                    if let Some(event) = mouse::parse_sgr(body) {
                        self.push_event(Event::Mouse(event));
                    }
                } else if let Some(event) = keys::parse_csi(body) {
                    self.push_event(Event::Key(event));
                }
            }
            Mode::Probe => self.finish_probe_csi(body),
        }
        State::Ground
    }

    fn step_utf8(
        &mut self,
        mut bytes: [u8; 4],
        len: usize,
        expected: usize,
        modifiers: KeyModifiers,
        byte: u8,
    ) -> State {
        if byte & 0b1100_0000 != 0b1000_0000 {
            return State::Ground;
        }
        bytes[len] = byte;
        let new_len = len + 1;
        if new_len != expected {
            return State::Utf8 {
                bytes,
                len: new_len,
                expected,
                modifiers,
            };
        }

        if let Ok(text) = std::str::from_utf8(&bytes[..expected]) {
            if let Some(ch) = text.chars().next() {
                self.push_char(ch, modifiers);
            }
        }
        State::Ground
    }

    fn step_input_byte(&mut self, byte: u8, modifiers: KeyModifiers) -> State {
        match byte {
            b'\r' => self.push_key(KeyCode::Enter, modifiers),
            b'\t' => self.push_key(KeyCode::Tab, modifiers),
            b'\x7f' => self.push_key(KeyCode::Backspace, modifiers),
            b'\0' => self.push_key(KeyCode::Char(' '), modifiers | KeyModifiers::CONTROL),
            b'\x01'..=b'\x1a' => self.push_key(
                KeyCode::Char(char::from(byte - 1 + b'a')),
                modifiers | KeyModifiers::CONTROL,
            ),
            b'\x1c'..=b'\x1f' => self.push_key(
                KeyCode::Char(char::from(byte - 0x1c + b'4')),
                modifiers | KeyModifiers::CONTROL,
            ),
            b' '..=b'~' => self.push_char(char::from(byte), modifiers),
            _ => {
                let Some(expected) = utf8_expected(byte) else {
                    return State::Ground;
                };
                let mut bytes = [0; 4];
                bytes[0] = byte;
                return State::Utf8 {
                    bytes,
                    len: 1,
                    expected,
                    modifiers,
                };
            }
        }
        State::Ground
    }

    fn step_paste(&mut self, matched: usize, byte: u8) -> State {
        if byte == PASTE_END[matched] {
            let matched = matched + 1;
            if matched == PASTE_END.len() {
                let text = String::from_utf8_lossy(&self.paste).into_owned();
                self.paste.clear();
                self.push_event(Event::Paste(text));
                return State::Ground;
            }
            return State::Paste { matched };
        }

        if !self.append_paste(&PASTE_END[..matched]) {
            self.paste.clear();
            return self.discard_after_mismatch(byte);
        }
        if byte == PASTE_END[0] {
            State::Paste { matched: 1 }
        } else if self.append_paste(&[byte]) {
            State::Paste { matched: 0 }
        } else {
            self.paste.clear();
            State::DiscardPaste { matched: 0 }
        }
    }

    fn step_discard_paste(&mut self, matched: usize, byte: u8) -> State {
        if byte == PASTE_END[matched] {
            let matched = matched + 1;
            if matched == PASTE_END.len() {
                self.paste.clear();
                State::Ground
            } else {
                State::DiscardPaste { matched }
            }
        } else {
            self.discard_after_mismatch(byte)
        }
    }

    fn discard_after_mismatch(&self, byte: u8) -> State {
        State::DiscardPaste {
            matched: usize::from(byte == PASTE_END[0]),
        }
    }

    fn append_paste(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() > PASTE_CAPACITY.saturating_sub(self.paste.len()) {
            return false;
        }
        self.paste.extend_from_slice(bytes);
        true
    }

    fn step_osc_prefix(&mut self, matched: usize, deadline: Instant, byte: u8) -> State {
        if osc52_prefix_matches(matched, byte) {
            let matched = matched + 1;
            if matched == 5 {
                self.osc52_payload.clear();
                State::Osc52 {
                    st_pending: false,
                    deadline,
                }
            } else {
                State::OscPrefix { matched, deadline }
            }
        } else {
            self.osc52_payload.clear();
            self.step_discard_osc(false, deadline, byte)
        }
    }

    fn step_osc52(&mut self, st_pending: bool, deadline: Instant, byte: u8) -> State {
        if st_pending {
            if byte == b'\\' {
                return self.finish_osc52();
            }
            self.osc52_payload.clear();
            return self.step_discard_osc(false, deadline, byte);
        }
        match byte {
            b'\x07' => self.finish_osc52(),
            b'\x1b' => State::Osc52 {
                st_pending: true,
                deadline,
            },
            _ if self.append_osc52(byte) => State::Osc52 {
                st_pending: false,
                deadline,
            },
            _ => {
                self.osc52_payload.clear();
                State::DiscardOsc {
                    st_pending: false,
                    deadline,
                }
            }
        }
    }

    fn step_discard_osc(&mut self, st_pending: bool, deadline: Instant, byte: u8) -> State {
        if st_pending && byte == b'\\' {
            return self.finish_discard_osc();
        }
        match byte {
            b'\x07' => self.finish_discard_osc(),
            b'\x1b' => State::DiscardOsc {
                st_pending: true,
                deadline,
            },
            _ => State::DiscardOsc {
                st_pending: false,
                deadline,
            },
        }
    }

    fn append_osc52(&mut self, byte: u8) -> bool {
        let Some(encoded_limit) = self.osc52_limit.and_then(osc52_encoded_limit) else {
            return false;
        };
        let Some(next_len) = self.osc52_payload.len().checked_add(1) else {
            return false;
        };
        if next_len > encoded_limit {
            return false;
        }
        self.osc52_payload.push(byte);
        true
    }

    fn finish_osc52(&mut self) -> State {
        let payload = std::mem::take(&mut self.osc52_payload);
        if let Some(limit) = self.osc52_limit.take() {
            if let Some(bytes) = base64_decode(&payload, limit) {
                self.push_event(Event::Osc52Clipboard(decode_file_text(bytes).text));
            }
        }
        State::Ground
    }

    fn finish_discard_osc(&mut self) -> State {
        self.osc52_payload.clear();
        self.osc52_limit = None;
        State::Ground
    }

    fn finish_probe_csi(&mut self, body: &[u8]) {
        match body.last() {
            Some(b'R') => match parse_cpr(body) {
                Some(width) => self.probe.cpr = Some(width),
                None => self.mark_probe_malformed(),
            },
            Some(b'c') if body.first() == Some(&b'?') => {
                if is_valid_da1(body) {
                    self.probe.result = Some(if self.probe.malformed {
                        AmbiguousWidth::Narrow
                    } else {
                        self.probe.cpr.unwrap_or(AmbiguousWidth::Narrow)
                    });
                } else {
                    self.mark_probe_malformed();
                }
            }
            _ => {}
        }
    }

    fn start_probe_byte(&mut self) -> bool {
        if self.mode != Mode::Probe {
            return true;
        }
        if self.probe.total_bytes == PROBE_RESPONSE_CAPACITY {
            self.probe.result = Some(AmbiguousWidth::Narrow);
            return false;
        }
        self.probe.total_bytes += 1;
        true
    }

    fn finish_probe_byte(&mut self) {
        if self.mode == Mode::Probe
            && self.probe.result.is_none()
            && self.probe.total_bytes == PROBE_RESPONSE_CAPACITY
        {
            self.probe.result = Some(AmbiguousWidth::Narrow);
        }
    }

    fn mark_probe_malformed(&mut self) {
        if self.mode == Mode::Probe {
            self.probe.malformed = true;
        }
    }

    fn escape(&self, now: Instant) -> State {
        State::Escape {
            deadline: now + ESCAPE_TIMEOUT,
        }
    }

    fn push_char(&mut self, ch: char, mut modifiers: KeyModifiers) {
        if ch.is_uppercase() {
            modifiers |= KeyModifiers::SHIFT;
        }
        self.push_key(KeyCode::Char(ch), modifiers);
    }

    fn push_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.push_event(Event::Key(KeyEvent::new_with_kind(
            code,
            modifiers,
            KeyEventKind::Press,
        )));
    }

    fn push_event(&mut self, event: Event) {
        if self.events.len() < EVENT_QUEUE_CAPACITY {
            self.events.push_back(event);
        }
    }
}

const fn is_csi_parameter_or_intermediate(byte: u8) -> bool {
    byte >= 0x20 && byte <= 0x3f
}

const fn is_csi_final(byte: u8) -> bool {
    byte >= 0x40 && byte <= 0x7e
}

fn is_syntactically_valid_csi(csi: &[u8]) -> bool {
    let Some((_, body)) = csi.split_last() else {
        return false;
    };
    let mut saw_intermediate = false;
    for &byte in body {
        match byte {
            0x20..=0x2f => saw_intermediate = true,
            0x30..=0x3f if !saw_intermediate => {}
            _ => return false,
        }
    }
    true
}

fn utf8_expected(byte: u8) -> Option<usize> {
    match byte {
        0xc2..=0xdf => Some(2),
        0xe0..=0xef => Some(3),
        0xf0..=0xf4 => Some(4),
        _ => None,
    }
}

const fn osc52_prefix_matches(matched: usize, byte: u8) -> bool {
    match matched {
        0 => byte == b'5',
        1 => byte == b'2',
        2 | 4 => byte == b';',
        3 => byte == b'c' || byte == b'p',
        _ => false,
    }
}

const fn osc52_encoded_limit(max_bytes: usize) -> Option<usize> {
    let groups = max_bytes / 3 + if max_bytes % 3 == 0 { 0 } else { 1 };
    groups.checked_mul(4)
}

fn parse_cpr(csi: &[u8]) -> Option<AmbiguousWidth> {
    let parameters = csi.strip_suffix(b"R")?;
    let mut parts = parameters.split(|&byte| byte == b';');
    let row = parse_decimal(parts.next()?)?;
    let column = parse_decimal(parts.next()?)?;
    if row == 0 || parts.next().is_some() {
        return None;
    }
    match column {
        2 => Some(AmbiguousWidth::Narrow),
        3 => Some(AmbiguousWidth::Wide),
        _ => None,
    }
}

fn parse_decimal(bytes: &[u8]) -> Option<u16> {
    if bytes.is_empty() {
        return None;
    }
    bytes.iter().try_fold(0_u16, |value, &byte| {
        let digit = u16::from(byte.checked_sub(b'0')?);
        if digit > 9 {
            return None;
        }
        value.checked_mul(10)?.checked_add(digit)
    })
}

fn is_valid_da1(csi: &[u8]) -> bool {
    let Some(parameters) = csi
        .strip_suffix(b"c")
        .and_then(|csi| csi.strip_prefix(b"?"))
    else {
        return false;
    };
    !parameters.is_empty()
        && parameters
            .split(|&byte| byte == b';')
            .all(|parameter| !parameter.is_empty() && parameter.iter().all(u8::is_ascii_digit))
}
