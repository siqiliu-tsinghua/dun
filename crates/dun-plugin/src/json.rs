//! Minimal JSON value model, parser, and writer.
//!
//! Hand-rolled so the protocol client carries no external serialization
//! dependency. Frame size is capped before this parser runs; nesting depth
//! and object member counts are capped here. Numbers follow RFC 8259 and
//! non-finite results are rejected.

use std::fmt::{self, Write as _};

pub const MAX_DEPTH: usize = 32;
pub const MAX_OBJECT_MEMBERS: usize = 64;

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        if let Self::Obj(fields) = self {
            fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Self::Str(text) = self {
            Some(text)
        } else {
            None
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        if let Self::Num(number) = self {
            if number.fract() == 0.0 && *number >= 0.0 && *number <= MAX_SAFE_INTEGER {
                return Some(*number as u64);
            }
        }
        None
    }

    pub fn as_arr(&self) -> Option<&[Json]> {
        if let Self::Arr(items) = self {
            Some(items)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(value) = self {
            Some(*value)
        } else {
            None
        }
    }
}

pub fn str(text: &str) -> Json {
    Json::Str(text.to_string())
}

pub fn num(value: u64) -> Json {
    Json::Num(value as f64)
}

pub fn bool(value: bool) -> Json {
    Json::Bool(value)
}

pub fn obj<const N: usize>(fields: [(&str, Json); N]) -> Json {
    Json::Obj(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
    pub message: &'static str,
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid JSON at byte {}: {}",
            self.offset, self.message
        )
    }
}

pub fn parse(bytes: &[u8]) -> Result<Json, JsonError> {
    let mut parser = Parser { bytes, pos: 0 };
    let value = parser.value(0)?;
    parser.skip_ws();
    if parser.pos != bytes.len() {
        return Err(parser.error("trailing data"));
    }
    Ok(value)
}

pub fn to_string(value: &Json) -> String {
    let mut out = String::new();
    write_value(value, &mut out);
    out
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn error(&self, message: &'static str) -> JsonError {
        JsonError {
            offset: self.pos,
            message,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek();
        if byte.is_some() {
            self.pos += 1;
        }
        byte
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, byte: u8, message: &'static str) -> Result<(), JsonError> {
        if self.bump() == Some(byte) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.error("nesting too deep"));
        }
        self.skip_ws();
        match self.peek() {
            Some(b'n') => self.literal(b"null", Json::Null),
            Some(b't') => self.literal(b"true", Json::Bool(true)),
            Some(b'f') => self.literal(b"false", Json::Bool(false)),
            Some(b'"') => self.string().map(Json::Str),
            Some(b'[') => self.array(depth),
            Some(b'{') => self.object(depth),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(self.error("unexpected byte")),
        }
    }

    fn literal(&mut self, text: &'static [u8], value: Json) -> Result<Json, JsonError> {
        for &expected in text {
            if self.bump() != Some(expected) {
                return Err(self.error("invalid literal"));
            }
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.pos;

        if self.peek() == Some(b'-') {
            self.pos += 1;
        }

        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(self.error("invalid number"));
                }
            }
            Some(b'1'..=b'9') => self.digits(),
            _ => return Err(self.error("invalid number")),
        }

        if self.peek() == Some(b'.') {
            self.pos += 1;
            let fraction_start = self.pos;
            self.digits();
            if self.pos == fraction_start {
                return Err(self.error("invalid number"));
            }
        }

        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            let exponent_start = self.pos;
            self.digits();
            if self.pos == exponent_start {
                return Err(self.error("invalid number"));
            }
        }

        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| self.error("invalid number"))?;
        let number: f64 = text.parse().map_err(|_| self.error("invalid number"))?;
        if !number.is_finite() {
            return Err(self.error("non-finite number"));
        }
        Ok(Json::Num(number))
    }

    fn digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"', "expected string")?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let byte = self
                .bump()
                .ok_or_else(|| self.error("unterminated string"))?;
            match byte {
                b'"' => {
                    return String::from_utf8(out)
                        .map_err(|_| self.error("invalid UTF-8 in string"));
                }
                b'\\' => {
                    let ch = self.escape()?;
                    let mut buffer = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
                }
                0x00..=0x1F => return Err(self.error("control byte in string")),
                _ => out.push(byte),
            }
        }
    }

    fn escape(&mut self) -> Result<char, JsonError> {
        let escape = self
            .bump()
            .ok_or_else(|| self.error("unterminated escape"))?;
        Ok(match escape {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{0008}',
            b'f' => '\u{000C}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => {
                let unit = self.hex4()?;
                if (0xD800..0xDC00).contains(&unit) {
                    if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                        return Err(self.error("missing low surrogate"));
                    }
                    let low = self.hex4()?;
                    if !(0xDC00..0xE000).contains(&low) {
                        return Err(self.error("invalid low surrogate"));
                    }
                    let code = 0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00);
                    char::from_u32(code).ok_or_else(|| self.error("invalid code point"))?
                } else {
                    char::from_u32(unit).ok_or_else(|| self.error("invalid code point"))?
                }
            }
            _ => return Err(self.error("invalid escape")),
        })
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let mut value = 0u32;
        for _ in 0..4 {
            let digit = match self.bump() {
                Some(byte @ b'0'..=b'9') => u32::from(byte - b'0'),
                Some(byte @ b'a'..=b'f') => u32::from(byte - b'a') + 10,
                Some(byte @ b'A'..=b'F') => u32::from(byte - b'A') + 10,
                _ => return Err(self.error("invalid unicode escape")),
            };
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn array(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.expect(b'[', "expected array")?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_ws();
            match self.bump() {
                Some(b',') => {}
                Some(b']') => return Ok(Json::Arr(items)),
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.expect(b'{', "expected object")?;
        let mut fields = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Obj(fields));
        }
        loop {
            if fields.len() >= MAX_OBJECT_MEMBERS {
                return Err(self.error("too many object members"));
            }
            self.skip_ws();
            let key = self.string()?;
            if fields.iter().any(|(name, _)| name == &key) {
                return Err(self.error("duplicate object key"));
            }
            self.skip_ws();
            self.expect(b':', "expected ':'")?;
            let value = self.value(depth + 1)?;
            fields.push((key, value));
            self.skip_ws();
            match self.bump() {
                Some(b',') => {}
                Some(b'}') => return Ok(Json::Obj(fields)),
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }
}

fn write_value(value: &Json, out: &mut String) {
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(true) => out.push_str("true"),
        Json::Bool(false) => out.push_str("false"),
        Json::Num(number) => {
            if number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER {
                let _ = write!(out, "{}", *number as i64);
            } else {
                let _ = write!(out, "{number}");
            }
        }
        Json::Str(text) => write_string(text, out),
        Json::Arr(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        Json::Obj(fields) => {
            out.push('{');
            for (index, (key, value)) in fields.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write_value(value, out);
            }
            out.push('}');
        }
    }
}

fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_nested_values() {
        let text = r#"{"a":[1,2.5,-3],"b":{"c":"x\ny é 😀","d":null},"e":true}"#;
        let value = parse(text.as_bytes()).expect("parses");
        let rendered = to_string(&value);
        let reparsed = parse(rendered.as_bytes()).expect("reparses");
        assert_eq!(value, reparsed);
        assert_eq!(
            value
                .get("b")
                .and_then(|inner| inner.get("c"))
                .and_then(Json::as_str),
            Some("x\ny é 😀")
        );
    }

    #[test]
    fn accepts_rfc_8259_number_forms() {
        let valid: &[(&[u8], f64)] = &[
            (b"0", 0.0),
            (b"-0", -0.0),
            (b"123", 123.0),
            (b"-42", -42.0),
            (b"0.25", 0.25),
            (b"-0.5", -0.5),
            (b"1e+2", 100.0),
            (b"1e-2", 0.01),
            (b"1E5", 100_000.0),
        ];

        for &(text, expected) in valid {
            let Json::Num(actual) = parse(text).expect("valid number parses") else {
                panic!("number did not parse as a number: {text:?}");
            };
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "wrong value for {text:?}"
            );
        }
    }

    #[test]
    fn rejects_invalid_number_integer_parts() {
        let invalid: &[&[u8]] = &[
            b"01", b"00.5", b"-01", b"-.5", b"+-1", b"+1", b"-", b".5", b"0123",
        ];

        for &text in invalid {
            assert!(parse(text).is_err(), "invalid number parsed: {text:?}");
        }
    }

    #[test]
    fn rejects_fractions_without_digits() {
        let invalid: &[&[u8]] = &[b"1.", b"-0.", b"1.e2", b"1.2.3"];

        for &text in invalid {
            assert!(parse(text).is_err(), "invalid number parsed: {text:?}");
        }
    }

    #[test]
    fn rejects_exponents_without_digits() {
        let invalid: &[&[u8]] = &[b"1e", b"1e+", b"1e-", b"1E+", b"1e1e1"];

        for &text in invalid {
            assert!(parse(text).is_err(), "invalid number parsed: {text:?}");
        }
    }

    #[test]
    fn rejects_non_finite_number_results() {
        assert!(parse(b"1e400").is_err());
    }

    #[test]
    fn rejects_duplicate_object_keys() {
        let duplicates: &[&[u8]] = &[
            br#"{"a":1,"a":2}"#,
            br#"{"outer":{"a":1,"a":2}}"#,
            br#"{"a":1,"\u0061":2}"#,
        ];

        for &text in duplicates {
            let error = parse(text).expect_err("duplicate key is rejected");
            assert_eq!(error.message, "duplicate object key");
        }
    }

    #[test]
    fn enforces_object_member_limit() {
        let object_with_members = |count: usize| {
            let fields = (0..count)
                .map(|index| format!(r#""key{index}":null"#))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{fields}}}")
        };

        let at_limit = object_with_members(MAX_OBJECT_MEMBERS);
        let Json::Obj(fields) = parse(at_limit.as_bytes()).expect("object at limit parses") else {
            panic!("object did not parse as an object");
        };
        assert_eq!(fields.len(), MAX_OBJECT_MEMBERS);

        let over_limit = object_with_members(MAX_OBJECT_MEMBERS + 1);
        let error = parse(over_limit.as_bytes()).expect_err("object over limit is rejected");
        assert_eq!(error.message, "too many object members");
    }

    #[test]
    fn rejects_trailing_data_and_control_bytes() {
        assert!(parse(b"{} x").is_err());
        assert!(parse(b"\"a\x01b\"").is_err());
        assert!(parse(b"").is_err());
    }

    #[test]
    fn rejects_deep_nesting() {
        let mut text = String::new();
        for _ in 0..(MAX_DEPTH + 2) {
            text.push('[');
        }
        assert!(parse(text.as_bytes()).is_err());
    }

    #[test]
    fn u64_accessor_rejects_fractions_and_negatives() {
        assert_eq!(parse(b"7").unwrap().as_u64(), Some(7));
        assert_eq!(parse(b"7.5").unwrap().as_u64(), None);
        assert_eq!(parse(b"-7").unwrap().as_u64(), None);
    }
}
