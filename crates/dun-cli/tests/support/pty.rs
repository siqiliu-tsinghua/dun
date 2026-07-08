#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{fmt::Write as _, thread};

pub const CTRL_Q: &[u8] = b"\x11";
const DEFAULT_PTY_ROWS: u16 = 24;
const DEFAULT_PTY_COLS: u16 = 80;
static PTY_TEST_LOCK: Mutex<()> = Mutex::new(());

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
}

#[derive(Debug)]
pub struct PtyRun {
    pub status: ExitStatus,
    pub output: String,
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
        .arg(&script_path)
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

    Ok(PtyRun {
        status,
        output: String::from_utf8_lossy(&bytes).into_owned(),
    })
}

fn expect_script_for_command(
    case: TerminalCase,
    binary: &OsStr,
    args: &[&OsStr],
    ready_marker: &str,
    input: &[u8],
) -> String {
    format!(
        "\
set timeout 10
log_user 1
spawn -noecho /bin/sh -lc {}
expect {{
    -re {} {{}}
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
if {{$exit_code eq \"\"}} {{
    exit 1
}}
exit $exit_code
",
        tcl_brace_quote(&shell_command_for_binary(case, binary, args)),
        tcl_brace_quote(&regex_literal(ready_marker)),
        tcl_escaped_bytes(input),
        tcl_escaped_bytes(input)
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

pub fn temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    env::temp_dir().join(format!("{prefix}-{}-{nanos}.{suffix}", std::process::id()))
}
