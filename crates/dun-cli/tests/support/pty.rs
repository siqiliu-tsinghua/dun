#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{fmt::Write as _, thread};

use super::terminal_grid::{TerminalGrid, parse_terminal_grid};
use dun_term::AmbiguousWidth;

pub const CTRL_Q: &[u8] = b"\x11";
const DEFAULT_PTY_ROWS: u16 = 24;
const DEFAULT_PTY_COLS: u16 = 80;
const AMBIGUOUS_WIDTH_PROBE_REGEX: &str = r#""\r\u2500\033\\\[6n\033\\\[c""#;
const NARROW_PROBE_RESPONSE: &[u8] = b"\x1b[1;2R\x1b[?1;2c";
const WIDE_PROBE_RESPONSE: &[u8] = b"\x1b[1;3R\x1b[?1;2c";
const PROBE_ELAPSED_MARKER: &str = "DUN_PTY_PROBE_ELAPSED_MS=";
static PTY_TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeResponse {
    Narrow,
    Wide,
    None,
}

#[derive(Clone, Copy, Debug)]
pub struct TerminalCase {
    pub name: &'static str,
    pub term: &'static str,
    pub lang: &'static str,
    pub lc_ctype: &'static str,
    pub no_color: bool,
    pub rows: u16,
    pub cols: u16,
    pub expected_profile: Option<&'static str>,
    probe_response: ProbeResponse,
}

impl TerminalCase {
    pub const fn new(
        name: &'static str,
        term: &'static str,
        lang: &'static str,
        lc_ctype: &'static str,
        no_color: bool,
        expected_profile: &'static str,
    ) -> Self {
        Self {
            name,
            term,
            lang,
            lc_ctype,
            no_color,
            rows: DEFAULT_PTY_ROWS,
            cols: DEFAULT_PTY_COLS,
            expected_profile: Some(expected_profile),
            probe_response: ProbeResponse::Narrow,
        }
    }

    pub const fn sized(mut self, rows: u16, cols: u16) -> Self {
        self.rows = rows;
        self.cols = cols;
        self
    }

    pub const fn without_profile_marker(mut self) -> Self {
        self.expected_profile = None;
        self
    }

    pub const fn with_wide_ambiguous_width(mut self) -> Self {
        self.probe_response = ProbeResponse::Wide;
        self
    }

    pub const fn without_ambiguous_width_probe_response(mut self) -> Self {
        self.probe_response = ProbeResponse::None;
        self
    }

    const fn ambiguous_width(self) -> AmbiguousWidth {
        match self.probe_response {
            ProbeResponse::Wide => AmbiguousWidth::Wide,
            ProbeResponse::Narrow | ProbeResponse::None => AmbiguousWidth::Narrow,
        }
    }
}

#[derive(Debug)]
pub struct PtyRun {
    pub status: ExitStatus,
    pub output: String,
    pub probe_elapsed: Option<Duration>,
}

impl PtyRun {
    pub fn terminal_grid(
        &self,
        cols: u16,
        rows: u16,
        ambiguous_width: AmbiguousWidth,
    ) -> TerminalGrid {
        parse_terminal_grid(&self.output, cols, rows, ambiguous_width, None)
    }

    pub fn terminal_grid_for_case(&self, case: TerminalCase) -> TerminalGrid {
        self.terminal_grid(case.cols, case.rows, case.ambiguous_width())
    }
}

pub fn run_dun_in_pty(
    expect: &Path,
    case: TerminalCase,
    args: &[&OsStr],
    ready_marker: &str,
    input: &[u8],
) -> io::Result<PtyRun> {
    run_dun_in_pty_with_env(expect, case, args, ready_marker, input, &[])
}

pub fn run_dun_in_pty_with_env(
    expect: &Path,
    case: TerminalCase,
    args: &[&OsStr],
    ready_marker: &str,
    input: &[u8],
    extra_env: &[(&str, &OsStr)],
) -> io::Result<PtyRun> {
    run_binary_in_pty(
        expect,
        case,
        dun_binary().as_os_str(),
        args,
        ready_marker,
        input,
        extra_env,
    )
}

pub fn run_binary_in_pty(
    expect: &Path,
    case: TerminalCase,
    binary: &OsStr,
    args: &[&OsStr],
    ready_marker: &str,
    input: &[u8],
    extra_env: &[(&str, &OsStr)],
) -> io::Result<PtyRun> {
    let script_path = temp_path("dun-pty-expect", "tcl");
    fs::write(
        &script_path,
        expect_script_for_command(case, binary, args, ready_marker, input),
    )?;

    let result = run_expect_script(expect, case, &script_path, extra_env);
    let _ = fs::remove_file(&script_path);
    result
}

fn run_expect_script(
    expect: &Path,
    case: TerminalCase,
    script_path: &Path,
    extra_env: &[(&str, &OsStr)],
) -> io::Result<PtyRun> {
    let mut command = Command::new(expect);

    command
        .arg(script_path)
        .env("TERM", case.term)
        .env("LANG", case.lang)
        .env("LC_CTYPE", case.lc_ctype)
        .env("COLUMNS", case.cols.to_string())
        .env("LINES", case.rows.to_string())
        .env_remove("COLORTERM")
        .env_remove("NO_COLOR")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if case.no_color {
        command.env("NO_COLOR", "1");
    }
    for (name, value) in extra_env {
        command.env(name, value);
    }

    let mut child = command.spawn()?;
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let status = wait_with_timeout(&mut child, Duration::from_secs(10))?;

    let mut bytes = Vec::new();
    read_to_end_if_present(&mut stdout, &mut bytes);
    read_to_end_if_present(&mut stderr, &mut bytes);

    let mut output = String::from_utf8_lossy(&bytes).into_owned();
    let probe_elapsed = take_probe_elapsed(&mut output);

    Ok(PtyRun {
        status,
        output,
        probe_elapsed,
    })
}

fn expect_script_for_command(
    case: TerminalCase,
    binary: &OsStr,
    args: &[&OsStr],
    ready_marker: &str,
    input: &[u8],
) -> String {
    let probe_action = match case.probe_response {
        ProbeResponse::Narrow => {
            format!("send -- {}", tcl_escaped_bytes(NARROW_PROBE_RESPONSE))
        }
        ProbeResponse::Wide => {
            format!("send -- {}", tcl_escaped_bytes(WIDE_PROBE_RESPONSE))
        }
        ProbeResponse::None => String::new(),
    };
    format!(
        "\
set timeout 10
log_user 1
set probe_started_ms -1
set probe_elapsed_ms -1
set ambiguous_width_probe {}
spawn -noecho /bin/sh -lc {}
expect {{
    -re $ambiguous_width_probe {{
        set probe_started_ms [clock milliseconds]
        {}
        exp_continue
    }}
    -re {} {{
        if {{$probe_started_ms >= 0}} {{
            set probe_elapsed_ms [expr {{[clock milliseconds] - $probe_started_ms}}]
        }}
    }}
    eof {{
        catch {{wait}}
        exit 125
    }}
    timeout {{
        catch {{close}}
        catch {{wait}}
        exit 124
    }}
}}
after 100
send -- {}
after 100
send -- {}
expect {{
    eof {{}}
    timeout {{
        catch {{close}}
        catch {{wait}}
        exit 124
    }}
}}
set wait_result [wait]
set exit_code [lindex $wait_result 3]
if {{[lindex $wait_result 4] eq \"CHILDKILLED\"}} {{
    exec kill -s [lindex $wait_result 5] [pid]
}}
if {{$exit_code eq \"\"}} {{
    exit 1
}}
puts \"{}$probe_elapsed_ms\"
exit $exit_code
",
        AMBIGUOUS_WIDTH_PROBE_REGEX,
        tcl_brace_quote(&shell_command_for_binary(case, binary, args)),
        probe_action,
        tcl_brace_quote(&regex_literal(ready_marker)),
        tcl_escaped_bytes(input),
        tcl_escaped_bytes(input),
        PROBE_ELAPSED_MARKER,
    )
}

fn shell_command_for_binary(case: TerminalCase, binary: &OsStr, args: &[&OsStr]) -> String {
    let mut command = format!("stty rows {} cols {}; exec ", case.rows, case.cols);
    command.push_str(&shell_quote(binary));
    for arg in args {
        command.push(' ');
        command.push_str(&shell_quote(arg));
    }
    command
}

fn dun_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_dun"))
}

fn shell_quote(value: &OsStr) -> String {
    let text = value.to_string_lossy();
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn tcl_brace_quote(value: &str) -> String {
    let mut quoted = String::from("{");
    for ch in value.chars() {
        match ch {
            '\\' | '{' | '}' => {
                quoted.push('\\');
                quoted.push(ch);
            }
            _ => quoted.push(ch),
        }
    }
    quoted.push('}');
    quoted
}

fn regex_literal(value: &str) -> String {
    let mut literal = String::new();
    for ch in value.chars() {
        if matches!(
            ch,
            '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '\\' | '|'
        ) {
            literal.push('\\');
        }
        literal.push(ch);
    }
    literal
}

fn tcl_escaped_bytes(bytes: &[u8]) -> String {
    let mut escaped = String::from("\"");
    for byte in bytes {
        match byte {
            b'\\' => escaped.push_str("\\\\"),
            b'"' => escaped.push_str("\\\""),
            b'[' | b']' | b'$' => {
                escaped.push('\\');
                escaped.push(*byte as char);
            }
            b'\n' => escaped.push_str("\\n"),
            b'\r' => escaped.push_str("\\r"),
            0x20..=0x7e => escaped.push(*byte as char),
            _ => {
                let _ = write!(escaped, "\\{:03o}", byte);
            }
        }
    }
    escaped.push('"');
    escaped
}

fn take_probe_elapsed(output: &mut String) -> Option<Duration> {
    let marker_start = output.rfind(PROBE_ELAPSED_MARKER)?;
    let value_start = marker_start + PROBE_ELAPSED_MARKER.len();
    let value_end = output[value_start..]
        .find(['\r', '\n'])
        .map_or(output.len(), |offset| value_start + offset);
    let millis = output[value_start..value_end].parse::<u64>().ok();
    let mut marker_end = value_end;
    while output
        .as_bytes()
        .get(marker_end)
        .is_some_and(|byte| matches!(byte, b'\r' | b'\n'))
    {
        marker_end += 1;
    }
    output.replace_range(marker_start..marker_end, "");
    millis.map(Duration::from_millis)
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> io::Result<ExitStatus> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }

        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("PTY child did not exit within {} ms", timeout.as_millis()),
            ));
        }

        thread::sleep(Duration::from_millis(25));
    }
}

fn read_to_end_if_present(reader: &mut Option<impl Read>, bytes: &mut Vec<u8>) {
    if let Some(reader) = reader {
        let _ = reader.read_to_end(bytes);
    }
}

pub fn assert_output_contains(output: &str, needle: &str, case: &str) {
    assert!(
        output.contains(needle),
        "{case} output did not contain {needle:?}\n{output}"
    );
}

pub fn assert_output_not_contains(output: &str, needle: &str, case: &str) {
    assert!(
        !output.contains(needle),
        "{case} output unexpectedly contained {needle:?}\n{output}"
    );
}

pub fn pty_test_guard() -> MutexGuard<'static, ()> {
    PTY_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn command_on_path(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    env::split_paths(&paths)
        .map(|path| path.join(name))
        .find(|path| path.is_file())
}

/// Locate Microsoft Edit on PATH, confirming the `edit` binary really is
/// Microsoft Edit and not another editor exposed under the same name — FreeBSD's
/// `/usr/bin/edit` is `ee`, which would otherwise make the differential tests
/// drive the wrong editor and fail spuriously. Returns `None` (skip) when no
/// Microsoft Edit is present.
pub fn microsoft_edit_on_path() -> Option<PathBuf> {
    let edit = command_on_path("edit")?;
    let output = Command::new(&edit)
        .arg("--help")
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let help = String::from_utf8_lossy(&output.stdout);
    (output.status.success()
        && help.contains("Usage: edit")
        && help.contains("FILE[:LINE[:COLUMN]]"))
    .then_some(edit)
}

pub fn temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    env::temp_dir().join(format!("{prefix}-{}-{nanos}.{suffix}", std::process::id()))
}
