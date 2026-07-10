#![forbid(unsafe_code)]

//! Dun Plugin Protocol host: syntax highlighting via syntect.
//!
//! A `user-trusted-external` host for the `syntax-highlight` role, reusing
//! dun's own frame and JSON modules over stdio. Configure:
//!
//! ```text
//! plugin.syntect.command = /path/to/dun-syntect-host
//! plugin.syntect.trust = user-trusted-external
//! plugin.syntect.roles = syntax-highlight
//! ```
//!
//! Span columns are character offsets; syntect's byte ranges are converted
//! per line. Parse state carries across the lines of one snapshot, so
//! multi-line constructs highlight correctly within the visible window.

use std::io;

use dun_plugin::frame::{read_frame, write_frame};
use dun_plugin::json::{self, Json};
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

const MAX_FRAME_BYTES: usize = 256 * 1024;
const MAX_SPANS: usize = 4000;
const HOST_ID: &str = "syntect";

fn main() -> io::Result<()> {
    let syntax_set = SyntaxSet::load_defaults_nonewlines();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    loop {
        let Ok(payload) = read_frame(&mut input, MAX_FRAME_BYTES) else {
            return Ok(());
        };
        let Ok(message) = json::parse(&payload) else {
            eprintln!("dun-syntect-host: malformed frame payload");
            return Ok(());
        };
        let kind = message.get("kind").and_then(Json::as_str).unwrap_or("");
        let request_id = message
            .get("request_id")
            .and_then(Json::as_u64)
            .unwrap_or(0);
        match kind {
            "hello" => {
                let reply = envelope(
                    "hello-ack",
                    request_id,
                    None,
                    json::obj([
                        ("host_id", json::str(HOST_ID)),
                        ("trust", json::str("user-trusted-external")),
                    ]),
                );
                write_frame(&mut output, &reply)?;
            }
            "request" => {
                let reply = respond(&syntax_set, &message, request_id);
                write_frame(&mut output, &reply)?;
            }
            "cancel-request" => {}
            "shutdown" => return Ok(()),
            _ => {
                let reply = envelope(
                    "error",
                    request_id,
                    None,
                    json::obj([("message", json::str("unsupported message kind"))]),
                );
                write_frame(&mut output, &reply)?;
            }
        }
    }
}

fn respond(syntax_set: &SyntaxSet, message: &Json, request_id: u64) -> Vec<u8> {
    let revision = message.get("revision").and_then(Json::as_u64);
    let payload = message.get("payload");
    let language = payload
        .and_then(|value| value.get("language"))
        .and_then(Json::as_str)
        .unwrap_or("");
    let first_line = payload
        .and_then(|value| value.get("first_line"))
        .and_then(Json::as_u64)
        .unwrap_or(0);
    let lines: Vec<&str> = payload
        .and_then(|value| value.get("lines"))
        .and_then(Json::as_arr)
        .map(|items| items.iter().filter_map(Json::as_str).collect())
        .unwrap_or_default();

    let spans = highlight(syntax_set, language, first_line, &lines);
    envelope(
        "response",
        request_id,
        revision,
        json::obj([("spans", Json::Arr(spans))]),
    )
}

fn highlight(
    syntax_set: &SyntaxSet,
    language: &str,
    first_line: u64,
    lines: &[&str],
) -> Vec<Json> {
    let Some(syntax) = syntax_set.find_syntax_by_extension(language) else {
        return Vec::new();
    };
    let mut parse_state = ParseState::new(syntax);
    let mut spans = Vec::new();

    for (offset, line) in lines.iter().enumerate() {
        let Ok(ops) = parse_state.parse_line(line, syntax_set) else {
            break;
        };
        let mut stack = ScopeStack::new();
        let mut last_byte = 0usize;
        for (byte_offset, op) in &ops {
            push_span(
                &mut spans,
                line,
                first_line + offset as u64,
                last_byte,
                *byte_offset,
                &stack,
            );
            let _ = stack.apply(op);
            last_byte = *byte_offset;
        }
        push_span(
            &mut spans,
            line,
            first_line + offset as u64,
            last_byte,
            line.len(),
            &stack,
        );
        if spans.len() >= MAX_SPANS {
            spans.truncate(MAX_SPANS);
            break;
        }
    }

    spans
}

fn push_span(
    spans: &mut Vec<Json>,
    line: &str,
    line_number: u64,
    start_byte: usize,
    end_byte: usize,
    stack: &ScopeStack,
) {
    if start_byte >= end_byte || end_byte > line.len() {
        return;
    }
    let Some(style) = classify(stack) else {
        return;
    };
    let start_col = line[..start_byte].chars().count() as u64;
    let end_col = start_col + line[start_byte..end_byte].chars().count() as u64;
    if start_col >= end_col {
        return;
    }
    spans.push(json::obj([
        ("line", json::num(line_number)),
        ("start_col", json::num(start_col)),
        ("end_col", json::num(end_col)),
        ("style", json::str(style)),
    ]));
}

/// Maps the innermost recognizable syntect scope onto dun's five-class
/// style vocabulary.
fn classify(stack: &ScopeStack) -> Option<&'static str> {
    for scope in stack.as_slice().iter().rev() {
        let name = scope.build_string();
        if name.starts_with("comment") {
            return Some("comment");
        }
        if name.starts_with("string") {
            return Some("string");
        }
        if name.starts_with("constant.numeric") {
            return Some("number");
        }
        if name.starts_with("keyword") || name.starts_with("storage") {
            return Some("keyword");
        }
        if name.starts_with("entity.name") || name.starts_with("support.function") {
            return Some("emphasis");
        }
    }
    None
}

fn envelope(kind: &str, request_id: u64, revision: Option<u64>, payload: Json) -> Vec<u8> {
    let mut fields = vec![
        ("v".to_string(), json::num(0)),
        ("kind".to_string(), json::str(kind)),
        ("request_id".to_string(), json::num(request_id)),
        ("plugin_id".to_string(), json::str(HOST_ID)),
        ("role".to_string(), json::str("syntax-highlight")),
    ];
    if let Some(revision) = revision {
        fields.push(("revision".to_string(), json::num(revision)));
    }
    fields.push(("payload".to_string(), payload));
    json::to_string(&Json::Obj(fields)).into_bytes()
}
