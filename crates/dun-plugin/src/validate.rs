//! Input snapshots and the `overlay-write` capability's output validator.
//!
//! Validators are keyed by capability, not by role: `validate_spans` is the
//! validator for `Capability::OverlayWrite`, the channel `SyntaxHighlight`
//! returns through (`Role::output_capability`). Later slices add each new
//! output capability's validator beside this one (e.g. `surface-write`).
//!
//! Rejected output must leave editor state unchanged; validation therefore
//! returns an error message instead of applying anything.

use crate::json::Json;
use crate::proto::Policy;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputSnapshot {
    pub buffer_revision: u64,
    pub language: String,
    pub first_line: u32,
    pub lines: Vec<String>,
}

/// A bounded chunk of a `dun`-managed output stream, fed to a host granted the
/// `stream-read` capability. `stream_id` names the source stream, `chunk_index`
/// orders chunks within it, and `final_chunk` marks the last one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamChunk {
    pub stream_id: String,
    pub chunk_index: u64,
    pub lines: Vec<String>,
    pub final_chunk: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleId {
    Keyword,
    Comment,
    StringLiteral,
    Number,
    Emphasis,
}

impl StyleId {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "keyword" => Some(Self::Keyword),
            "comment" => Some(Self::Comment),
            "string" => Some(Self::StringLiteral),
            "number" => Some(Self::Number),
            "emphasis" => Some(Self::Emphasis),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyleSpan {
    pub line: u32,
    pub start_col: u32,
    pub end_col: u32,
    pub style: StyleId,
}

pub fn validate_spans(
    snapshot: &InputSnapshot,
    payload: &Json,
    policy: &Policy,
) -> Result<Vec<StyleSpan>, &'static str> {
    let spans = payload
        .get("spans")
        .and_then(Json::as_arr)
        .ok_or("response payload has no span list")?;
    if spans.len() > policy.max_spans {
        return Err("span count exceeds policy limit");
    }
    let mut validated = Vec::with_capacity(spans.len());
    for span in spans {
        validated.push(validate_span(snapshot, span)?);
    }
    validated.sort_unstable_by_key(|span| (span.line, span.start_col, span.end_col));
    Ok(validated)
}

/// The `surface-write` capability's output validator: a bounded list of text
/// lines the host wants shown in its own surface window. Control bytes are not
/// rejected here — the lines enter a `dun` buffer and are escaped by the
/// `DisplaySanitizer` at render time, exactly like any other buffer text; this
/// validator only bounds the line count and rejects non-string entries.
pub fn validate_surface(payload: &Json, policy: &Policy) -> Result<Vec<String>, &'static str> {
    let lines = payload
        .get("lines")
        .and_then(Json::as_arr)
        .ok_or("surface payload has no line list")?;
    if lines.len() > policy.max_surface_lines {
        return Err("surface line count exceeds policy limit");
    }
    let mut validated = Vec::with_capacity(lines.len());
    for line in lines {
        let text = line.as_str().ok_or("surface line is not a string")?;
        validated.push(text.to_string());
    }
    Ok(validated)
}

/// The `stream-read` capability's response validator: a host granted
/// `stream-read` receives a [`StreamChunk`] and answers with one keep/drop
/// verdict per input line. The verdict list must have exactly one boolean per
/// line — a host cannot invent or drop rows — so `dun` can apply the decisions
/// positionally without trusting a length the host chose.
pub fn validate_stream_verdict(
    payload: &Json,
    line_count: usize,
) -> Result<Vec<bool>, &'static str> {
    let keep = payload
        .get("keep")
        .and_then(Json::as_arr)
        .ok_or("stream verdict has no keep list")?;
    if keep.len() != line_count {
        return Err("stream verdict length does not match the chunk line count");
    }
    let mut verdicts = Vec::with_capacity(keep.len());
    for value in keep {
        verdicts.push(
            value
                .as_bool()
                .ok_or("stream verdict entry is not a boolean")?,
        );
    }
    Ok(verdicts)
}

fn validate_span(snapshot: &InputSnapshot, span: &Json) -> Result<StyleSpan, &'static str> {
    let line = span_field(span, "line")?;
    let start_col = span_field(span, "start_col")?;
    let end_col = span_field(span, "end_col")?;
    let style_id = span
        .get("style")
        .and_then(Json::as_str)
        .ok_or("span has no style id")?;
    let style = StyleId::from_id(style_id).ok_or("unknown style id")?;

    let line_count = u32::try_from(snapshot.lines.len()).map_err(|_| "snapshot too large")?;
    let end_line = snapshot
        .first_line
        .checked_add(line_count)
        .ok_or("line range overflow")?;
    if line < snapshot.first_line || line >= end_line {
        return Err("span line outside snapshot");
    }
    let text = &snapshot.lines[(line - snapshot.first_line) as usize];
    let char_count = u32::try_from(text.chars().count()).map_err(|_| "line too long")?;
    if start_col >= end_col {
        return Err("span is empty or reversed");
    }
    if end_col > char_count {
        return Err("span column outside line");
    }
    Ok(StyleSpan {
        line,
        start_col,
        end_col,
        style,
    })
}

fn span_field(span: &Json, key: &'static str) -> Result<u32, &'static str> {
    let value = span
        .get(key)
        .and_then(Json::as_u64)
        .ok_or("span field missing or invalid")?;
    u32::try_from(value).map_err(|_| "span field out of range")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json;

    fn snapshot() -> InputSnapshot {
        InputSnapshot {
            buffer_revision: 1,
            language: "rust".to_string(),
            first_line: 10,
            lines: vec!["fn main() {".to_string(), "}".to_string()],
        }
    }

    fn span_json(line: u64, start: u64, end: u64, style: &str) -> Json {
        json::obj([
            ("line", json::num(line)),
            ("start_col", json::num(start)),
            ("end_col", json::num(end)),
            ("style", json::str(style)),
        ])
    }

    #[test]
    fn accepts_and_sorts_valid_spans() {
        let payload = json::obj([(
            "spans",
            Json::Arr(vec![
                span_json(11, 0, 1, "comment"),
                span_json(10, 0, 2, "keyword"),
            ]),
        )]);
        let spans = validate_spans(&snapshot(), &payload, &Policy::default()).expect("valid");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].line, 10);
        assert_eq!(spans[0].style, StyleId::Keyword);
        assert_eq!(spans[1].line, 11);
    }

    #[test]
    fn rejects_out_of_range_spans() {
        let policy = Policy::default();
        let base = snapshot();
        let cases = [
            span_json(9, 0, 1, "keyword"),
            span_json(12, 0, 1, "keyword"),
            span_json(10, 2, 2, "keyword"),
            span_json(10, 0, 99, "keyword"),
            span_json(10, 0, 1, "blink"),
        ];
        for case in cases {
            let payload = json::obj([("spans", Json::Arr(vec![case]))]);
            assert!(validate_spans(&base, &payload, &policy).is_err());
        }
    }

    #[test]
    fn rejects_span_floods_and_missing_lists() {
        let policy = Policy {
            max_spans: 1,
            ..Policy::default()
        };
        let payload = json::obj([(
            "spans",
            Json::Arr(vec![
                span_json(10, 0, 1, "keyword"),
                span_json(10, 1, 2, "keyword"),
            ]),
        )]);
        assert!(validate_spans(&snapshot(), &payload, &policy).is_err());
        assert!(validate_spans(&snapshot(), &Json::Null, &policy).is_err());
    }

    fn surface_payload(lines: &[&str]) -> Json {
        json::obj([(
            "lines",
            Json::Arr(lines.iter().map(|line| json::str(line)).collect()),
        )])
    }

    #[test]
    fn surface_accepts_a_bounded_line_list() {
        let lines = validate_surface(&surface_payload(&["a", "b"]), &Policy::default())
            .expect("valid surface lines");
        assert_eq!(lines, vec!["a", "b"]);
    }

    #[test]
    fn surface_rejects_flood_non_strings_and_missing_list() {
        let policy = Policy {
            max_surface_lines: 1,
            ..Policy::default()
        };
        assert!(validate_surface(&surface_payload(&["a", "b"]), &policy).is_err());

        let non_string = json::obj([("lines", Json::Arr(vec![json::num(1)]))]);
        assert!(validate_surface(&non_string, &Policy::default()).is_err());

        assert!(validate_surface(&Json::Null, &Policy::default()).is_err());
    }

    fn keep_payload(keeps: &[bool]) -> Json {
        json::obj([(
            "keep",
            Json::Arr(keeps.iter().map(|b| json::bool(*b)).collect()),
        )])
    }

    #[test]
    fn stream_verdict_accepts_one_boolean_per_line() {
        let verdict =
            validate_stream_verdict(&keep_payload(&[true, false, true]), 3).expect("valid verdict");
        assert_eq!(verdict, vec![true, false, true]);
    }

    #[test]
    fn stream_verdict_rejects_length_mismatch_non_bool_and_missing_list() {
        // One verdict too few / too many for the chunk's line count.
        assert!(validate_stream_verdict(&keep_payload(&[true]), 2).is_err());
        assert!(validate_stream_verdict(&keep_payload(&[true, false]), 1).is_err());

        let non_bool = json::obj([("keep", Json::Arr(vec![json::num(1)]))]);
        assert!(validate_stream_verdict(&non_bool, 1).is_err());

        assert!(validate_stream_verdict(&Json::Null, 0).is_err());
    }
}
