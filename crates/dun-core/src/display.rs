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
        } else if is_bidi_formatting(ch) {
            push_segment(
                segments,
                DisplayClass::Escape,
                render_control(ch, self.ascii_only),
            );
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

/// The Unicode bidirectional formatting characters.
///
/// They are **not** `char::is_control()` — they are Cf format characters — so
/// the control check above could not see them, and U+202E RIGHT-TO-LEFT
/// OVERRIDE reached the terminal from every text field the editor draws: buffer
/// body, file name, window title, both halves of the status line, and every
/// part of a modal.
///
/// A bidi override makes rendered text read in an order the underlying bytes do
/// not have. In an editor that is the Trojan Source attack (CVE-2021-42574) —
/// a reviewer trusting their eyes sees code that is not the code that will run.
/// In the Open dialog it disguises a file name. Neither has any legitimate use
/// in a monospace terminal grid, which does not implement bidi anyway.
///
/// This is the same set rustc's `text_direction_codepoint_in_literal` lint
/// rejects.
const BIDI_FORMATTING: [char; 12] = [
    '\u{061c}', // ARABIC LETTER MARK
    '\u{200e}', // LEFT-TO-RIGHT MARK
    '\u{200f}', // RIGHT-TO-LEFT MARK
    '\u{202a}', // LEFT-TO-RIGHT EMBEDDING
    '\u{202b}', // RIGHT-TO-LEFT EMBEDDING
    '\u{202c}', // POP DIRECTIONAL FORMATTING
    '\u{202d}', // LEFT-TO-RIGHT OVERRIDE
    '\u{202e}', // RIGHT-TO-LEFT OVERRIDE
    '\u{2066}', // LEFT-TO-RIGHT ISOLATE
    '\u{2067}', // RIGHT-TO-LEFT ISOLATE
    '\u{2068}', // FIRST STRONG ISOLATE
    '\u{2069}', // POP DIRECTIONAL ISOLATE
];

pub fn is_bidi_formatting(ch: char) -> bool {
    BIDI_FORMATTING.contains(&ch)
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

    struct ControlPayload {
        name: &'static str,
        text: String,
    }

    fn plain(line: SanitizedLine) -> String {
        line.as_plain_text()
    }

    fn control_payloads() -> Vec<ControlPayload> {
        vec![
            ControlPayload {
                name: "ansi_sgr",
                text: "\x1b[31mred\x1b[0m".to_string(),
            },
            ControlPayload {
                name: "ansi_clear_screen",
                text: "\x1b[2J\x1b[Hclear".to_string(),
            },
            ControlPayload {
                name: "osc_title",
                text: "\x1b]0;owned\x07title".to_string(),
            },
            ControlPayload {
                name: "osc_clipboard",
                text: "\x1b]52;c;SGVsbG8=\x07clipboard".to_string(),
            },
            ControlPayload {
                name: "osc_hyperlink",
                text: "\x1b]8;;https://example.invalid\x07link\x1b]8;;\x07".to_string(),
            },
            ControlPayload {
                name: "dcs",
                text: "\x1bPqpayload\x1b\\".to_string(),
            },
            ControlPayload {
                name: "kitty_graphics",
                text: "\x1b_Ga=T,f=100;AAAA\x1b\\".to_string(),
            },
            ControlPayload {
                name: "bracketed_paste",
                text: "\x1b[200~paste\x1b[201~".to_string(),
            },
            ControlPayload {
                name: "carriage_return_backspace",
                text: "safe\roverwrite\x08\x08".to_string(),
            },
            ControlPayload {
                name: "nul_del_tab",
                text: "\0null\t tab\x7fdel".to_string(),
            },
            ControlPayload {
                name: "all_c0_controls",
                text: (0x00..=0x1f).filter_map(char::from_u32).collect::<String>(),
            },
            ControlPayload {
                name: "c1_csi",
                text: "\u{009b}31mred".to_string(),
            },
            ControlPayload {
                name: "all_c1_controls",
                text: (0x80..=0x9f).filter_map(char::from_u32).collect::<String>(),
            },
        ]
    }

    fn assert_no_raw_controls(name: &str, output: &str) {
        assert!(
            !output.chars().any(char::is_control),
            "{name} emitted raw control text: {output:?}"
        );
        assert!(
            !output.contains('\x1b'),
            "{name} emitted raw ESC: {output:?}"
        );
        assert!(
            !output.contains('\u{009b}'),
            "{name} emitted raw C1 CSI: {output:?}"
        );
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
    fn terminal_control_payloads_are_neutralized_in_utf8_mode() {
        let sanitizer = DisplaySanitizer::unlimited_utf8();

        for payload in control_payloads() {
            let sanitized = sanitizer.sanitize_line(&payload.text);
            let output = sanitized.as_plain_text();

            assert_no_raw_controls(payload.name, &output);
            assert!(
                sanitized.has_non_text_segments(),
                "{} should be classified as escaped/control text",
                payload.name
            );
        }
    }

    #[test]
    fn terminal_control_payloads_are_neutralized_in_ascii_mode() {
        let sanitizer = DisplaySanitizer::unlimited_ascii();

        for payload in control_payloads() {
            let sanitized = sanitizer.sanitize_line(&payload.text);
            let output = sanitized.as_plain_text();

            assert_no_raw_controls(payload.name, &output);
            assert!(
                output.is_ascii(),
                "{} emitted non-ASCII fallback text: {output:?}",
                payload.name
            );
            assert!(
                sanitized.has_non_text_segments(),
                "{} should be classified as escaped/control text",
                payload.name
            );
        }
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

    #[test]
    fn every_unicode_scalar_is_safe_in_every_sanitizer_profile() {
        let profiles = [
            ("UTF-8", DisplaySanitizer::unlimited_utf8()),
            ("ASCII-only", DisplaySanitizer::unlimited_ascii()),
            ("one-byte UTF-8", DisplaySanitizer::utf8(1)),
            ("one-byte ASCII-only", DisplaySanitizer::ascii(1)),
        ];

        for (profile, sanitizer) in profiles {
            for ch in '\u{0}'..='\u{10ffff}' {
                let mut encoded = [0; 4];
                let input = ch.encode_utf8(&mut encoded);
                let output = sanitizer.sanitize_line(input).as_plain_text();
                // Control characters are the obvious danger. Bidi formatting
                // characters are the one that got through: they are Cf, not
                // control, so a check for `is_control` alone waves U+202E
                // straight past -- which is exactly what this test used to do,
                // and why the hole had to be found end to end instead.
                let unsafe_output = output.chars().any(|output_ch| {
                    output_ch.is_control()
                        || output_ch == '\u{1b}'
                        || ('\u{80}'..='\u{9f}').contains(&output_ch)
                        || is_bidi_formatting(output_ch)
                });

                assert!(
                    !unsafe_output,
                    "{profile} sanitizer let U+{:04X} reach the terminal as {output:?}",
                    u32::from(ch)
                );
            }
        }
    }

    #[test]
    fn sanitization_composes_character_by_character_for_short_strings() {
        let profiles = [
            ("UTF-8", DisplaySanitizer::unlimited_utf8()),
            ("ASCII-only", DisplaySanitizer::unlimited_ascii()),
        ];
        let cases = [
            ("multi-byte", "café Ω"),
            ("combining marks", "e\u{301} o\u{308}"),
            ("wide CJK", "漢字界"),
            ("emoji", "🙂👩\u{200d}💻"),
            (
                "mixed control and printable",
                "A\u{1b}[2J中\u{301}🙂\u{9b}\rZ",
            ),
        ];

        for (profile, sanitizer) in profiles {
            for (case, input) in cases {
                let sanitized = sanitizer.sanitize_line(input);
                assert!(
                    !sanitized.truncated,
                    "{profile} unexpectedly truncated {case} input {input:?}"
                );
                let whole = sanitized.as_plain_text();
                let mut per_character = String::new();
                for ch in input.chars() {
                    let mut encoded = [0; 4];
                    per_character.push_str(
                        &sanitizer
                            .sanitize_line(ch.encode_utf8(&mut encoded))
                            .as_plain_text(),
                    );
                }

                assert_eq!(
                    whole, per_character,
                    "{profile} sanitization did not compose for {case} input {input:?}"
                );
            }
        }
    }
}
