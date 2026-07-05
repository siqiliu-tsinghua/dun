use crate::BufferKind;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FileTextEncoding {
    #[default]
    Utf8,
    EscapedBytes,
}

impl FileTextEncoding {
    pub const fn status_label(self) -> &'static str {
        match self {
            Self::Utf8 => "Text UTF-8",
            Self::EscapedBytes => "Escaped bytes",
        }
    }

    pub const fn is_save_safe(self) -> bool {
        matches!(self, Self::Utf8)
    }

    pub const fn buffer_kind(self) -> BufferKind {
        match self {
            Self::Utf8 => BufferKind::File,
            Self::EscapedBytes => BufferKind::ReadOnly,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedFileText {
    pub text: String,
    pub encoding: FileTextEncoding,
}

pub fn decode_file_text(bytes: Vec<u8>) -> DecodedFileText {
    match String::from_utf8(bytes) {
        Ok(text) => DecodedFileText {
            text,
            encoding: FileTextEncoding::Utf8,
        },
        Err(error) => DecodedFileText {
            text: escaped_unknown_bytes(error.as_bytes()),
            encoding: FileTextEncoding::EscapedBytes,
        },
    }
}

fn escaped_unknown_bytes(bytes: &[u8]) -> String {
    let mut output = String::new();
    let mut remaining = bytes;

    while !remaining.is_empty() {
        match std::str::from_utf8(remaining) {
            Ok(valid) => {
                push_fallback_valid_text(&mut output, valid);
                break;
            }
            Err(error) => {
                let valid_up_to = error.valid_up_to();
                let valid = std::str::from_utf8(&remaining[..valid_up_to])
                    .expect("valid prefix reported by Utf8Error should decode");
                push_fallback_valid_text(&mut output, valid);

                let invalid_len = error
                    .error_len()
                    .unwrap_or_else(|| remaining.len() - valid_up_to);
                for byte in &remaining[valid_up_to..valid_up_to + invalid_len] {
                    push_byte_escape(&mut output, *byte);
                }
                remaining = &remaining[valid_up_to + invalid_len..];
            }
        }
    }

    output
}

fn push_fallback_valid_text(output: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '\n' => output.push('\n'),
            '\\' => output.push_str("\\\\"),
            ch if ch.is_control() => {
                let mut bytes = [0; 4];
                for byte in ch.encode_utf8(&mut bytes).as_bytes() {
                    push_byte_escape(output, *byte);
                }
            }
            _ => output.push(ch),
        }
    }
}

fn push_byte_escape(output: &mut String, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    output.push_str("\\x");
    output.push(HEX[(byte >> 4) as usize] as char);
    output.push(HEX[(byte & 0x0f) as usize] as char);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_utf8_decodes_as_save_safe_text() {
        let decoded = decode_file_text("hello\n中".as_bytes().to_vec());

        assert_eq!(decoded.text, "hello\n中");
        assert_eq!(decoded.encoding, FileTextEncoding::Utf8);
        assert_eq!(decoded.encoding.status_label(), "Text UTF-8");
        assert!(decoded.encoding.is_save_safe());
        assert_eq!(decoded.encoding.buffer_kind(), BufferKind::File);
    }

    #[test]
    fn invalid_utf8_decodes_as_read_only_escaped_bytes() {
        let decoded = decode_file_text(vec![b'o', b'k', 0xff, b'\n', b'\\', b'\t', 0xe4]);

        assert_eq!(decoded.text, "ok\\xFF\n\\\\\\x09\\xE4");
        assert_eq!(decoded.encoding, FileTextEncoding::EscapedBytes);
        assert_eq!(decoded.encoding.status_label(), "Escaped bytes");
        assert!(!decoded.encoding.is_save_safe());
        assert_eq!(decoded.encoding.buffer_kind(), BufferKind::ReadOnly);
    }

    #[test]
    fn escaped_bytes_preserve_valid_unicode_segments() {
        let decoded = decode_file_text(vec![b'a', 0xe4, 0xb8, 0xad, 0xff, b'b']);

        assert_eq!(decoded.text, "a中\\xFFb");
        assert_eq!(decoded.encoding, FileTextEncoding::EscapedBytes);
    }

    #[test]
    fn escaped_bytes_never_leave_control_bytes_in_fallback_text() {
        let decoded = decode_file_text(vec![b'a', 0xff, b'\r', b'\x1b', b'\x7f', b'b']);

        assert_eq!(decoded.text, "a\\xFF\\x0D\\x1B\\x7Fb");
    }
}
