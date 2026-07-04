#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayClass {
    Text,
    Control,
    Escape,
    Truncation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplaySegment {
    pub class: DisplayClass,
    pub text: String,
}

impl DisplaySegment {
    pub fn new(class: DisplayClass, text: impl Into<String>) -> Self {
        Self {
            class,
            text: text.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SanitizedLine {
    pub segments: Vec<DisplaySegment>,
    pub truncated: bool,
    pub bytes_consumed: usize,
}

impl SanitizedLine {
    pub fn as_plain_text(&self) -> String {
        let mut out = String::new();
        for segment in &self.segments {
            out.push_str(&segment.text);
        }
        out
    }

    pub fn has_non_text_segments(&self) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.class != DisplayClass::Text)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplaySanitizer {
    pub ascii_only: bool,
    pub max_bytes: usize,
}

impl Default for DisplaySanitizer {
    fn default() -> Self {
        Self {
            ascii_only: false,
            max_bytes: 16 * 1024,
        }
    }
}

impl DisplaySanitizer {
    pub const fn utf8(max_bytes: usize) -> Self {
        Self {
            ascii_only: false,
            max_bytes,
        }
    }

    pub const fn ascii(max_bytes: usize) -> Self {
        Self {
            ascii_only: true,
            max_bytes,
        }
    }

    pub const fn unlimited_utf8() -> Self {
        Self::utf8(usize::MAX)
    }

    pub const fn unlimited_ascii() -> Self {
        Self::ascii(usize::MAX)
    }

    pub fn sanitize_line(&self, line: &str) -> SanitizedLine {
        let mut segments = Vec::new();
        let mut bytes_consumed = 0;

        for (index, ch) in line.char_indices() {
            let next_index = index + ch.len_utf8();
            if next_index > self.max_bytes {
                break;
            }

            bytes_consumed = next_index;
            self.push_char(&mut segments, ch);
        }

        let truncated = bytes_consumed < line.len();
        if truncated {
            push_segment(
                &mut segments,
                DisplayClass::Truncation,
                truncation_marker(self.ascii_only),
            );
        }

        SanitizedLine {
            segments,
            truncated,
            bytes_consumed,
        }
    }

    fn push_char(&self, segments: &mut Vec<DisplaySegment>, ch: char) {
        if ch.is_control() {
            let class = if ch == '\u{1b}' {
                DisplayClass::Escape
            } else {
                DisplayClass::Control
            };
            push_segment(segments, class, render_control(ch, self.ascii_only));
        } else if self.ascii_only && !ch.is_ascii() {
            push_segment(segments, DisplayClass::Escape, render_non_ascii(ch));
        } else {
            let mut text = String::new();
            text.push(ch);
            push_segment(segments, DisplayClass::Text, text);
        }
    }
}

fn push_segment(segments: &mut Vec<DisplaySegment>, class: DisplayClass, text: String) {
    if let Some(last) = segments.last_mut() {
        if last.class == class {
            last.text.push_str(&text);
            return;
        }
    }

    segments.push(DisplaySegment::new(class, text));
}

fn render_control(ch: char, ascii_only: bool) -> String {
    let code = ch as u32;

    if ascii_only {
        return match code {
            0x00..=0x1f => {
                let marker = char::from_u32(code + 0x40).expect("C0 marker should be valid ASCII");
                format!("^{}", marker)
            }
            0x7f => "^?".to_string(),
            _ => format!("<U+{:04X}>", code),
        };
    }

    match code {
        0x00..=0x1f => char::from_u32(0x2400 + code)
            .expect("C0 control picture should be valid Unicode")
            .to_string(),
        0x7f => "\u{2421}".to_string(),
        _ => format!("<U+{:04X}>", code),
    }
}

fn render_non_ascii(ch: char) -> String {
    format!("\\u{{{:x}}}", ch as u32)
}

fn truncation_marker(ascii_only: bool) -> String {
    if ascii_only {
        "...".to_string()
    } else {
        "…".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(line: SanitizedLine) -> String {
        line.as_plain_text()
    }

    #[test]
    fn printable_utf8_text_passes_through() {
        let sanitized = DisplaySanitizer::unlimited_utf8().sanitize_line("hello é");

        assert_eq!(sanitized.as_plain_text(), "hello é");
        assert!(!sanitized.truncated);
        assert!(!sanitized.has_non_text_segments());
    }

    #[test]
    fn ascii_mode_escapes_non_ascii_text() {
        let sanitized = DisplaySanitizer::unlimited_ascii().sanitize_line("é Ω");

        assert_eq!(sanitized.as_plain_text(), "\\u{e9} \\u{3a9}");
        assert!(sanitized.as_plain_text().is_ascii());
        assert!(sanitized.has_non_text_segments());
    }

    #[test]
    fn renders_c0_controls_without_raw_control_bytes() {
        let input = "\0\t\x08\r\x1b\x07\x7f";
        let output = plain(DisplaySanitizer::unlimited_utf8().sanitize_line(input));

        assert_eq!(output, "␀␉␈␍␛␇␡");
        assert!(!output.chars().any(char::is_control));
    }

    #[test]
    fn ascii_mode_renders_controls_with_caret_notation() {
        let input = "\0\t\x08\r\x1b\x07\x7f";
        let output = plain(DisplaySanitizer::unlimited_ascii().sanitize_line(input));

        assert_eq!(output, "^@^I^H^M^[^G^?");
        assert!(output.is_ascii());
        assert!(!output.chars().any(char::is_control));
    }

    #[test]
    fn osc_payload_is_rendered_as_visible_text() {
        let input = "\x1b]0;owned\x07";
        let output = plain(DisplaySanitizer::unlimited_utf8().sanitize_line(input));

        assert_eq!(output, "␛]0;owned␇");
        assert!(!output.contains('\x1b'));
        assert!(!output.contains('\x07'));
        assert!(!output.chars().any(char::is_control));
    }

    #[test]
    fn c1_controls_are_rendered_as_codepoints() {
        let output = plain(DisplaySanitizer::unlimited_utf8().sanitize_line("\u{009b}31m"));

        assert_eq!(output, "<U+009B>31m");
        assert!(!output.chars().any(char::is_control));
    }

    #[test]
    fn long_lines_are_capped_without_splitting_utf8_characters() {
        let sanitized = DisplaySanitizer::utf8(2).sanitize_line("aéz");

        assert_eq!(sanitized.as_plain_text(), "a…");
        assert!(sanitized.truncated);
        assert_eq!(sanitized.bytes_consumed, 1);
    }

    #[test]
    fn ascii_truncation_marker_stays_ascii() {
        let sanitized = DisplaySanitizer::ascii(1).sanitize_line("abcd");

        assert_eq!(sanitized.as_plain_text(), "a...");
        assert!(sanitized.as_plain_text().is_ascii());
        assert!(sanitized.truncated);
    }
}
