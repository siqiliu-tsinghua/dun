use std::io::{self, Stdout, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use dun_term::{ColorProfile, TerminalProfile};

#[derive(Clone)]
pub(crate) struct TerminalColorRewrite {
    rewrite_16_color_sgr: Arc<AtomicBool>,
}

impl TerminalColorRewrite {
    pub(crate) fn new(profile: TerminalProfile) -> Self {
        Self {
            rewrite_16_color_sgr: Arc::new(AtomicBool::new(should_rewrite_16_color_sgr(profile))),
        }
    }

    pub(crate) fn set_profile(&self, profile: TerminalProfile) {
        self.rewrite_16_color_sgr
            .store(should_rewrite_16_color_sgr(profile), Ordering::Relaxed);
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.rewrite_16_color_sgr.load(Ordering::Relaxed)
    }
}

fn should_rewrite_16_color_sgr(profile: TerminalProfile) -> bool {
    matches!(profile.colors, ColorProfile::Color16)
}

pub(crate) struct TerminalWriter {
    inner: Stdout,
    color_rewrite: TerminalColorRewrite,
    pending_escape: Vec<u8>,
}

impl TerminalWriter {
    pub(crate) fn new(inner: Stdout, color_rewrite: TerminalColorRewrite) -> Self {
        Self {
            inner,
            color_rewrite,
            pending_escape: Vec::new(),
        }
    }
}

impl Write for TerminalWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if !self.color_rewrite.is_enabled() {
            return self.inner.write(buffer);
        }

        let rewritten = rewrite_16_color_sgr(buffer, &mut self.pending_escape);
        self.inner.write_all(&rewritten)?;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.color_rewrite.is_enabled() && !self.pending_escape.is_empty() {
            self.inner.write_all(&self.pending_escape)?;
            self.pending_escape.clear();
        }
        self.inner.flush()
    }
}

const MAX_PENDING_ESCAPE_BYTES: usize = 1024;

pub(crate) fn rewrite_16_color_sgr(buffer: &[u8], pending_escape: &mut Vec<u8>) -> Vec<u8> {
    let mut input = Vec::with_capacity(pending_escape.len().saturating_add(buffer.len()));
    if !pending_escape.is_empty() {
        input.extend_from_slice(pending_escape);
        pending_escape.clear();
    }
    input.extend_from_slice(buffer);

    let mut output = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        if input[index] != 0x1b {
            output.push(input[index]);
            index += 1;
            continue;
        }

        if index + 1 >= input.len() {
            pending_escape.extend_from_slice(&input[index..]);
            break;
        }

        if input[index + 1] != b'[' {
            output.push(input[index]);
            output.push(input[index + 1]);
            index += 2;
            continue;
        }

        let mut end = index + 2;
        while end < input.len() && !is_csi_final_byte(input[end]) {
            end += 1;
        }

        if end >= input.len() {
            let pending = &input[index..];
            if pending.len() <= MAX_PENDING_ESCAPE_BYTES {
                pending_escape.extend_from_slice(pending);
            } else {
                output.extend_from_slice(pending);
            }
            break;
        }

        if input[end] == b'm' {
            output.extend_from_slice(&rewrite_16_color_sgr_sequence(&input[index + 2..end]));
        } else {
            output.extend_from_slice(&input[index..=end]);
        }
        index = end + 1;
    }

    output
}

fn is_csi_final_byte(byte: u8) -> bool {
    (0x40..=0x7e).contains(&byte)
}

fn rewrite_16_color_sgr_sequence(params: &[u8]) -> Vec<u8> {
    let Some(values) = parse_sgr_params(params) else {
        return original_sgr_sequence(params);
    };

    let mut rewritten = Vec::with_capacity(values.len());
    let mut index = 0;
    while index < values.len() {
        let value = values[index];
        if matches!(value, 38 | 48)
            && index + 2 < values.len()
            && values[index + 1] == 5
            && values[index + 2] <= 15
        {
            rewritten.push(legacy_16_color_sgr_code(value == 48, values[index + 2]));
            index += 3;
            continue;
        }

        rewritten.push(value);
        index += 1;
    }

    let mut output = Vec::new();
    output.extend_from_slice(b"\x1b[");
    for (index, value) in rewritten.iter().enumerate() {
        if index > 0 {
            output.push(b';');
        }
        output.extend_from_slice(value.to_string().as_bytes());
    }
    output.push(b'm');
    output
}

fn parse_sgr_params(params: &[u8]) -> Option<Vec<u16>> {
    if params.is_empty() {
        return Some(vec![0]);
    }

    let mut values = Vec::new();
    for param in params.split(|byte| *byte == b';') {
        if param.is_empty() {
            values.push(0);
            continue;
        }
        if !param.iter().all(u8::is_ascii_digit) {
            return None;
        }
        let text = std::str::from_utf8(param).ok()?;
        values.push(text.parse().ok()?);
    }
    Some(values)
}

fn legacy_16_color_sgr_code(background: bool, color: u16) -> u16 {
    let base = match (background, color < 8) {
        (false, true) => 30,
        (false, false) => 90,
        (true, true) => 40,
        (true, false) => 100,
    };
    base + (color % 8)
}

fn original_sgr_sequence(params: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(params.len().saturating_add(3));
    output.extend_from_slice(b"\x1b[");
    output.extend_from_slice(params);
    output.push(b'm');
    output
}
