use std::io::{self, IsTerminal, Read, Write};
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

use dun_term::{AmbiguousWidth, EncodingProfile};
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};

use super::vt::output::{self as vt_output, Sequence};

const PROBE_BYTES: &[u8] = b"\r\xe2\x94\x80\x1b[6n\x1b[c";
const PROBE_TIMEOUT: Duration = Duration::from_millis(500);
const MAX_RESPONSE_BYTES: usize = 256;
const MAX_CSI_BYTES: usize = 32;
const STDIN_TOKEN: Token = Token(0);

pub(crate) fn detect_ambiguous_width(encoding: EncodingProfile) -> AmbiguousWidth {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if !should_probe(encoding, stdin.is_terminal(), stdout.is_terminal()) {
        return AmbiguousWidth::Narrow;
    }

    let deadline = Instant::now() + PROBE_TIMEOUT;
    let mut output = stdout.lock();
    let result = if write_probe(&mut output).is_ok() {
        read_responses(&stdin, deadline)
    } else {
        AmbiguousWidth::Narrow
    };
    if clear_probe(&mut output).is_err() {
        AmbiguousWidth::Narrow
    } else {
        result
    }
}

fn should_probe(encoding: EncodingProfile, stdin_is_tty: bool, stdout_is_tty: bool) -> bool {
    matches!(encoding, EncodingProfile::Utf8) && stdin_is_tty && stdout_is_tty
}

fn write_probe(output: &mut impl Write) -> io::Result<()> {
    output.write_all(PROBE_BYTES)?;
    output.flush()
}

fn clear_probe(output: &mut impl Write) -> io::Result<()> {
    let move_result = vt_output::queue(output, Sequence::MoveToColumnZero);
    let clear_result = vt_output::queue(output, Sequence::ClearCurrentLine);
    let flush_result = output.flush();
    move_result.and(clear_result).and(flush_result)
}

fn read_responses(stdin: &io::Stdin, deadline: Instant) -> AmbiguousWidth {
    let raw_fd = stdin.as_raw_fd();
    let mut source = SourceFd(&raw_fd);
    let mut poll = match Poll::new() {
        Ok(poll) => poll,
        Err(_) => return AmbiguousWidth::Narrow,
    };
    if poll
        .registry()
        .register(&mut source, STDIN_TOKEN, Interest::READABLE)
        .is_err()
    {
        return AmbiguousWidth::Narrow;
    }

    let mut events = Events::with_capacity(1);
    let mut input = stdin.lock();
    let mut buffer = [0_u8; MAX_RESPONSE_BYTES];
    let mut responses = ProbeResponses::new();

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return responses.finish();
        }
        events.clear();
        if poll.poll(&mut events, Some(remaining)).is_err() {
            return AmbiguousWidth::Narrow;
        }
        if events.is_empty() {
            return responses.finish();
        }
        if !events
            .iter()
            .any(|event| event.token() == STDIN_TOKEN && event.is_readable())
        {
            continue;
        }

        let capacity = responses.remaining_capacity();
        if capacity == 0 {
            return AmbiguousWidth::Narrow;
        }
        let count = match input.read(&mut buffer[..capacity]) {
            Ok(0) | Err(_) => return AmbiguousWidth::Narrow,
            Ok(count) => count,
        };
        if let Some(result) = responses.push(&buffer[..count]) {
            return result;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParseState {
    Ground,
    Escape,
    Csi,
    OversizedCsi,
}

struct ProbeResponses {
    state: ParseState,
    csi: [u8; MAX_CSI_BYTES - 2],
    csi_len: usize,
    total_bytes: usize,
    cpr: Option<AmbiguousWidth>,
    malformed: bool,
}

impl ProbeResponses {
    const fn new() -> Self {
        Self {
            state: ParseState::Ground,
            csi: [0; MAX_CSI_BYTES - 2],
            csi_len: 0,
            total_bytes: 0,
            cpr: None,
            malformed: false,
        }
    }

    const fn remaining_capacity(&self) -> usize {
        MAX_RESPONSE_BYTES - self.total_bytes
    }

    fn push(&mut self, bytes: &[u8]) -> Option<AmbiguousWidth> {
        for &byte in bytes {
            if self.total_bytes == MAX_RESPONSE_BYTES {
                return Some(AmbiguousWidth::Narrow);
            }
            self.total_bytes += 1;

            let result = match self.state {
                ParseState::Ground => {
                    if byte == b'\x1b' {
                        self.state = ParseState::Escape;
                    }
                    None
                }
                ParseState::Escape => {
                    if byte == b'[' {
                        self.state = ParseState::Csi;
                        self.csi_len = 0;
                    } else if byte != b'\x1b' {
                        self.state = ParseState::Ground;
                    }
                    None
                }
                ParseState::Csi => self.push_csi_byte(byte),
                ParseState::OversizedCsi => {
                    if byte == b'\x1b' {
                        self.state = ParseState::Escape;
                    } else if is_csi_final(byte) {
                        self.state = ParseState::Ground;
                    }
                    None
                }
            };
            if result.is_some() {
                return result;
            }
        }

        (self.total_bytes == MAX_RESPONSE_BYTES).then_some(AmbiguousWidth::Narrow)
    }

    fn push_csi_byte(&mut self, byte: u8) -> Option<AmbiguousWidth> {
        if byte == b'\x1b' {
            self.malformed = true;
            self.state = ParseState::Escape;
            return None;
        }
        if self.csi_len == self.csi.len() {
            self.malformed = true;
            self.state = if is_csi_final(byte) {
                ParseState::Ground
            } else {
                ParseState::OversizedCsi
            };
            return None;
        }
        if !is_csi_parameter_or_intermediate(byte) && !is_csi_final(byte) {
            self.malformed = true;
            self.state = ParseState::Ground;
            return None;
        }

        self.csi[self.csi_len] = byte;
        self.csi_len += 1;
        if !is_csi_final(byte) {
            return None;
        }

        self.state = ParseState::Ground;
        let csi = &self.csi[..self.csi_len];
        if !is_syntactically_valid_csi(csi) {
            self.malformed = true;
            return None;
        }

        match csi.last() {
            Some(b'R') => match parse_cpr(csi) {
                Some(ambiguous_width) => self.cpr = Some(ambiguous_width),
                None => self.malformed = true,
            },
            Some(b'c') if csi.first() == Some(&b'?') => {
                if !is_valid_da1(csi) {
                    self.malformed = true;
                    return None;
                }
                return Some(if self.malformed {
                    AmbiguousWidth::Narrow
                } else {
                    self.cpr.unwrap_or(AmbiguousWidth::Narrow)
                });
            }
            _ => {}
        }
        None
    }

    const fn finish(&self) -> AmbiguousWidth {
        AmbiguousWidth::Narrow
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

#[cfg(test)]
mod tests {
    use super::*;

    const DA1: &[u8] = b"\x1b[?1;2c";

    fn completed_response(bytes: &[u8]) -> AmbiguousWidth {
        let mut responses = ProbeResponses::new();
        responses.push(bytes).unwrap_or_else(|| responses.finish())
    }

    #[test]
    fn probe_writes_the_exact_query_bytes() {
        let mut output = Vec::new();

        write_probe(&mut output).expect("write probe");

        assert_eq!(output, b"\r\xe2\x94\x80\x1b[6n\x1b[c");
    }

    #[test]
    fn probe_requires_utf8_stdin_tty_and_stdout_tty() {
        assert!(should_probe(EncodingProfile::Utf8, true, true));
        assert!(!should_probe(EncodingProfile::Ascii, true, true));
        assert!(!should_probe(EncodingProfile::Utf8, false, true));
        assert!(!should_probe(EncodingProfile::Utf8, true, false));
    }

    #[test]
    fn fragmented_narrow_cpr_waits_for_da1() {
        let mut responses = ProbeResponses::new();

        assert_eq!(responses.push(b"ignored\x1b[31m\x1b[1;"), None);
        assert_eq!(responses.push(b"2R\x1b"), None);
        assert_eq!(responses.push(b"[?1;2c"), Some(AmbiguousWidth::Narrow));
    }

    #[test]
    fn fragmented_wide_cpr_waits_for_da1() {
        let mut responses = ProbeResponses::new();

        assert_eq!(responses.push(b"\x1b"), None);
        assert_eq!(responses.push(b"[1;3"), None);
        assert_eq!(responses.push(b"R"), None);
        assert_eq!(responses.push(DA1), Some(AmbiguousWidth::Wide));
    }

    #[test]
    fn da1_without_cpr_is_narrow() {
        assert_eq!(completed_response(DA1), AmbiguousWidth::Narrow);
    }

    #[test]
    fn cpr_without_da1_times_out_to_narrow() {
        let mut responses = ProbeResponses::new();

        assert_eq!(responses.push(b"\x1b[1;3R"), None);
        assert_eq!(responses.finish(), AmbiguousWidth::Narrow);
    }

    #[test]
    fn malformed_cpr_is_narrow() {
        assert_eq!(
            completed_response(b"\x1b[1;;3R\x1b[?1;2c"),
            AmbiguousWidth::Narrow
        );
    }

    #[test]
    fn out_of_range_cpr_column_is_narrow() {
        assert_eq!(
            completed_response(b"\x1b[1;4R\x1b[?1;2c"),
            AmbiguousWidth::Narrow
        );
    }

    #[test]
    fn oversized_csi_is_narrow() {
        let mut bytes = b"\x1b[".to_vec();
        bytes.extend(std::iter::repeat_n(b'1', MAX_CSI_BYTES - 2));
        bytes.push(b'R');
        bytes.extend_from_slice(DA1);

        assert_eq!(completed_response(&bytes), AmbiguousWidth::Narrow);
    }

    #[test]
    fn exhausted_response_buffer_is_narrow() {
        let mut responses = ProbeResponses::new();
        let bytes = [b'x'; MAX_RESPONSE_BYTES];

        assert_eq!(responses.push(&bytes), Some(AmbiguousWidth::Narrow));
    }
}
