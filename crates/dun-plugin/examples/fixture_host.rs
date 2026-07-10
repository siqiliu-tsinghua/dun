#![forbid(unsafe_code)]

//! Minimal fixture host for exercising the spike protocol client end to
//! end: `dun --plugin-spike <path-to-this-binary>`.
//!
//! Special request languages: `stale-test` replies with a mismatched
//! revision; `slow-test` sleeps past any reasonable request timeout.

use std::io;
use std::thread;
use std::time::Duration;

use dun_plugin::frame::{read_frame, write_frame};
use dun_plugin::json::{self, Json};

const MAX_FRAME_BYTES: usize = 256 * 1024;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    loop {
        let Ok(payload) = read_frame(&mut input, MAX_FRAME_BYTES) else {
            return Ok(());
        };
        let Ok(message) = json::parse(&payload) else {
            eprintln!("fixture host: malformed frame payload");
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
                        ("host_id", json::str("fixture")),
                        ("trust", json::str("user-trusted-external")),
                    ]),
                );
                write_frame(&mut output, &reply)?;
            }
            "request" => {
                let reply = handle_request(&message, request_id);
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

fn handle_request(message: &Json, request_id: u64) -> Vec<u8> {
    let revision = message.get("revision").and_then(Json::as_u64).unwrap_or(0);
    let payload = message.get("payload");
    let language = payload
        .and_then(|value| value.get("language"))
        .and_then(Json::as_str)
        .unwrap_or("");
    let first_line = payload
        .and_then(|value| value.get("first_line"))
        .and_then(Json::as_u64)
        .unwrap_or(0);

    if language == "slow-test" {
        thread::sleep(Duration::from_secs(30));
    }
    let reply_revision = if language == "stale-test" {
        revision.wrapping_sub(1)
    } else {
        revision
    };

    let spans = Json::Arr(vec![json::obj([
        ("line", json::num(first_line)),
        ("start_col", json::num(0)),
        ("end_col", json::num(2)),
        ("style", json::str("keyword")),
    ])]);
    envelope(
        "response",
        request_id,
        Some(reply_revision),
        json::obj([("spans", spans)]),
    )
}

fn envelope(kind: &str, request_id: u64, revision: Option<u64>, payload: Json) -> Vec<u8> {
    let mut fields = vec![
        ("v".to_string(), json::num(0)),
        ("kind".to_string(), json::str(kind)),
        ("request_id".to_string(), json::num(request_id)),
        ("plugin_id".to_string(), json::str("fixture")),
        ("role".to_string(), json::str("syntax-highlight")),
    ];
    if let Some(revision) = revision {
        fields.push(("revision".to_string(), json::num(revision)));
    }
    fields.push(("payload".to_string(), payload));
    json::to_string(&Json::Obj(fields)).into_bytes()
}
