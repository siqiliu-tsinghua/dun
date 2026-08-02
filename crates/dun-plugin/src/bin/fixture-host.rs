#![forbid(unsafe_code)]

//! Minimal fixture host for exercising the plugin protocol client end to end.
//!
//! Handshake misbehavior is selected by the first argument, or — because
//! `HostClient` launches hosts without arguments — by running the binary
//! through a hard link named `fixture-host--<mode>` (the mode is read from
//! the program name). Request misbehavior is selected through the request
//! payload's `language` field.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use dun_plugin::frame::{read_frame, write_frame};
use dun_plugin::json::{self, Json};

const MAX_FRAME_BYTES: usize = 256 * 1024;
const FLOOD_SPAN_COUNT: usize = 4097;
const DIAGNOSTIC_FLOOD_COUNT: usize = 17;
const STDERR_FLOOD_BYTES: usize = 64 * 1024;
const SENTINEL_HELPER_SLEEP: Duration = Duration::from_secs(2);
const SENTINEL_HELPER_MODE: &str = "sentinel-helper";
const SPAWN_SENTINEL_MODE: &str = "spawn-sentinel";

fn main() -> io::Result<()> {
    if std::env::args().nth(1).as_deref() == Some(SENTINEL_HELPER_MODE) {
        return run_sentinel_helper();
    }
    let program = std::env::args_os()
        .next()
        .ok_or_else(|| io::Error::other("fixture host program path is missing"))?;
    let handshake_mode = std::env::args().nth(1).or_else(|| {
        let name = Path::new(&program).file_name()?.to_str()?;
        Some(name.rsplit_once("--")?.1.to_string())
    });
    if handshake_mode.as_deref() == Some(SPAWN_SENTINEL_MODE) {
        spawn_sentinel_helper(Path::new(&program))?;
    }
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
            "hello" => match handshake_mode.as_deref() {
                Some("bad-version") => {
                    let reply = envelope_with_version(
                        9,
                        "hello-ack",
                        request_id,
                        None,
                        hello_payload("user-trusted-external"),
                    );
                    write_frame(&mut output, &reply)?;
                }
                Some("bad-trust") => {
                    let reply = envelope(
                        "hello-ack",
                        request_id,
                        None,
                        hello_payload("unknown-trust"),
                    );
                    write_frame(&mut output, &reply)?;
                }
                Some("no-ack") => {
                    let reply = envelope(
                        "error",
                        request_id,
                        None,
                        json::obj([("message", json::str("handshake rejected"))]),
                    );
                    write_frame(&mut output, &reply)?;
                }
                Some("garbage-frame") => {
                    output.write_all(&32_u32.to_le_bytes())?;
                    output.write_all(b"{}")?;
                    output.flush()?;
                    return Ok(());
                }
                _ => {
                    let reply = envelope(
                        "hello-ack",
                        request_id,
                        None,
                        hello_payload("user-trusted-external"),
                    );
                    write_frame(&mut output, &reply)?;
                }
            },
            "request" => handle_request(&message, request_id, &mut output)?,
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

fn spawn_sentinel_helper(program: &Path) -> io::Result<()> {
    let directory = program
        .parent()
        .ok_or_else(|| io::Error::other("fixture host directory is missing"))?;
    Command::new(std::env::current_exe()?)
        .arg(SENTINEL_HELPER_MODE)
        .arg(directory.join("ready"))
        .arg(directory.join("survived"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn run_sentinel_helper() -> io::Result<()> {
    let ready = std::env::args_os()
        .nth(2)
        .ok_or_else(|| io::Error::other("sentinel ready path is missing"))?;
    let survived = std::env::args_os()
        .nth(3)
        .ok_or_else(|| io::Error::other("sentinel survived path is missing"))?;
    fs::write(ready, b"ready")?;
    thread::sleep(SENTINEL_HELPER_SLEEP);
    fs::write(survived, b"survived")
}

fn handle_request(message: &Json, request_id: u64, output: &mut impl Write) -> io::Result<()> {
    let revision = message.get("revision").and_then(Json::as_u64).unwrap_or(0);
    let payload = message.get("payload");

    // A surface-write action request carries `action_id` (no `language`); the
    // host answers with the lines to show in its own surface window.
    if let Some(action_id) = payload
        .and_then(|value| value.get("action_id"))
        .and_then(Json::as_str)
    {
        let reply = envelope(
            "response",
            request_id,
            None,
            json::obj([(
                "lines",
                Json::Arr(vec![
                    json::str(&format!("surface for action: {action_id}")),
                    json::str("second line"),
                ]),
            )]),
        );
        return write_frame(output, &reply);
    }

    // A stream-read request carries `stream_id` and a `lines` chunk; the host
    // answers with one keep/drop verdict per line (fixture rule: keep non-empty
    // lines).
    if payload
        .and_then(|value| value.get("stream_id"))
        .and_then(Json::as_str)
        .is_some()
    {
        let keep: Vec<Json> = payload
            .and_then(|value| value.get("lines"))
            .and_then(Json::as_arr)
            .unwrap_or(&[])
            .iter()
            .map(|line| json::bool(!line.as_str().unwrap_or("").is_empty()))
            .collect();
        let reply = envelope(
            "response",
            request_id,
            None,
            json::obj([("keep", Json::Arr(keep))]),
        );
        return write_frame(output, &reply);
    }

    // A scratch-input `execute` request carries `snippet`; the host "runs" it
    // and answers with result lines (fixture: echo a one-line summary).
    if let Some(snippet) = payload
        .and_then(|value| value.get("snippet"))
        .and_then(Json::as_str)
    {
        let summary = format!("executed {} chars", snippet.len());
        let reply = envelope(
            "response",
            request_id,
            None,
            json::obj([("lines", Json::Arr(vec![json::str(&summary)]))]),
        );
        return write_frame(output, &reply);
    }

    let language = payload
        .and_then(|value| value.get("language"))
        .and_then(Json::as_str)
        .unwrap_or("");
    let first_line = payload
        .and_then(|value| value.get("first_line"))
        .and_then(Json::as_u64)
        .unwrap_or(0);

    match language {
        "crash-test" => std::process::exit(2),
        "slow-test" => thread::sleep(Duration::from_secs(30)),
        "stderr-test" => write_stderr_flood()?,
        _ => {}
    }

    if language == "malformed-json-test" {
        // A correctly framed payload that is not valid JSON: the client must
        // reject it as a protocol error rather than panic or hang.
        write_frame(output, b"{ not valid json")?;
        return Ok(());
    }
    if language == "non-rfc-number-test" {
        write_frame(
            output,
            br#"{"v":0,"kind":"response","request_id":1,"plugin_id":"fixture","role":"syntax-highlight","revision":41,"payload":{"spans":[],"padding":01}}"#,
        )?;
        return Ok(());
    }
    if language == "duplicate-key-test" {
        write_frame(
            output,
            br#"{"v":0,"kind":"response","request_id":1,"plugin_id":"fixture","role":"syntax-highlight","revision":41,"payload":{"spans":[],"spans":[]}}"#,
        )?;
        return Ok(());
    }

    if language == "diag-flood-test" {
        for _ in 0..DIAGNOSTIC_FLOOD_COUNT {
            let diagnostic = envelope(
                "diagnostic",
                request_id,
                Some(revision),
                json::obj([("message", json::str("fixture diagnostic"))]),
            );
            write_frame(output, &diagnostic)?;
        }
    }

    let reply_revision = if language == "stale-test" {
        revision.wrapping_sub(1)
    } else {
        revision
    };
    let reply_request_id = if language == "wrong-id-test" {
        request_id.wrapping_add(1)
    } else {
        request_id
    };

    let spans = match language {
        "flood-test" => Json::Arr(
            (0..FLOOD_SPAN_COUNT)
                .map(|_| span(first_line, 2, "keyword"))
                .collect(),
        ),
        "badcoord-test" => Json::Arr(vec![span(first_line, 1_000_000, "keyword")]),
        "badstyle-test" => Json::Arr(vec![span(first_line, 2, "blink")]),
        _ => Json::Arr(vec![span(first_line, 2, "keyword")]),
    };
    let mut payload_fields = vec![("spans".to_string(), spans)];
    if language == "bigframe-test" {
        payload_fields.push((
            "padding".to_string(),
            Json::Str("x".repeat(MAX_FRAME_BYTES)),
        ));
    }
    let reply = envelope(
        "response",
        reply_request_id,
        Some(reply_revision),
        Json::Obj(payload_fields),
    );
    write_frame(output, &reply)
}

fn write_stderr_flood() -> io::Result<()> {
    let stderr = io::stderr();
    let mut output = stderr.lock();
    let chunk = [b'x'; 1024];
    for _ in 0..STDERR_FLOOD_BYTES / chunk.len() {
        output.write_all(&chunk)?;
    }
    output.flush()
}

fn hello_payload(trust: &str) -> Json {
    json::obj([
        ("host_id", json::str("fixture")),
        ("trust", json::str(trust)),
        ("menu", menu_contribution()),
        ("keybinding", keybinding_contribution()),
    ])
}

fn keybinding_contribution() -> Json {
    json::obj([
        ("leader", json::str("Ctrl+J")),
        (
            "chords",
            Json::Arr(vec![json::obj([
                ("key", json::str("p")),
                ("action_id", json::str("ping")),
            ])]),
        ),
    ])
}

fn menu_contribution() -> Json {
    json::obj([
        (
            "top_label",
            json::obj([
                ("en_US", json::str("Fixture")),
                ("zh-CN", json::str("夹具")),
            ]),
        ),
        (
            "items",
            Json::Arr(vec![json::obj([
                ("label", json::obj([("en_US", json::str("Ping"))])),
                ("action_id", json::str("ping")),
            ])]),
        ),
    ])
}

fn span(line: u64, end_col: u64, style: &str) -> Json {
    json::obj([
        ("line", json::num(line)),
        ("start_col", json::num(0)),
        ("end_col", json::num(end_col)),
        ("style", json::str(style)),
    ])
}

fn envelope(kind: &str, request_id: u64, revision: Option<u64>, payload: Json) -> Vec<u8> {
    envelope_with_version(0, kind, request_id, revision, payload)
}

fn envelope_with_version(
    version: u64,
    kind: &str,
    request_id: u64,
    revision: Option<u64>,
    payload: Json,
) -> Vec<u8> {
    let mut fields = vec![
        ("v".to_string(), json::num(version)),
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
