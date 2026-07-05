#![cfg(unix)]
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

const CTRL_Q: &[u8] = b"\x11";
const DEFAULT_PTY_ROWS: u16 = 24;
const DEFAULT_PTY_COLS: u16 = 80;
static PTY_TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug)]
struct TerminalCase {
    name: &'static str,
    term: &'static str,
    lang: &'static str,
    lc_ctype: &'static str,
    no_color: bool,
    rows: u16,
    cols: u16,
    expected_profile: Option<&'static str>,
}

impl TerminalCase {
    const fn new(
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

    const fn sized(mut self, rows: u16, cols: u16) -> Self {
        self.rows = rows;
        self.cols = cols;
        self
    }

    const fn without_profile_marker(mut self) -> Self {
        self.expected_profile = None;
        self
    }
}

#[derive(Debug)]
struct PtyRun {
    status: ExitStatus,
    output: String,
}

fn terminal_profile_cases() -> [TerminalCase; 9] {
    [
        TerminalCase::new(
            "xterm-256color utf8",
            "xterm-256color",
            "en_US.UTF-8",
            "en_US.UTF-8",
            false,
            "UTF-8/256",
        ),
        TerminalCase::new(
            "screen-256color utf8",
            "screen-256color",
            "en_US.UTF-8",
            "en_US.UTF-8",
            false,
            "UTF-8/256",
        ),
        TerminalCase::new(
            "tmux-256color utf8",
            "tmux-256color",
            "en_US.UTF-8",
            "en_US.UTF-8",
            false,
            "UTF-8/256",
        ),
        TerminalCase::new(
            "screen utf8",
            "screen",
            "en_US.UTF-8",
            "en_US.UTF-8",
            false,
            "UTF-8/16",
        ),
        TerminalCase::new(
            "xterm-color c locale",
            "xterm-color",
            "C",
            "C",
            false,
            "ASCII/16",
        ),
        TerminalCase::new("vt100 ascii", "vt100", "C", "C", false, "ASCII/16"),
        TerminalCase::new("ansi ascii", "ansi", "C", "C", false, "ASCII/16"),
        TerminalCase::new("dumb ascii mono", "dumb", "C", "C", false, "ASCII/mono"),
        TerminalCase::new(
            "xterm-256color no color",
            "xterm-256color",
            "en_US.UTF-8",
            "en_US.UTF-8",
            true,
            "UTF-8/mono",
        ),
    ]
}

#[test]
fn pty_smoke_quits_cleanly_for_common_terminal_profiles() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    for case in terminal_profile_cases() {
        let run = run_dun_in_pty(&expect, case, &[], "Untitled", CTRL_Q)?;
        assert!(
            run.status.success(),
            "{} failed with status {:?}\n{}",
            case.name,
            run.status,
            run.output
        );
        assert_output_contains(&run.output, "Untitled", case.name);
        assert_output_contains(&run.output, "Ln 1/1, Col 1", case.name);
        if let Some(profile) = case.expected_profile {
            assert_output_contains(&run.output, profile, case.name);
        }
    }

    Ok(())
}

#[test]
fn pty_smoke_opens_utf8_file_and_renders_initial_content() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let file_path = temp_path("dun-pty-open", "txt");
    let file_name = file_path
        .file_name()
        .and_then(OsStr::to_str)
        .expect("temp file name should be valid UTF-8")
        .to_string();
    fs::write(&file_path, "alpha\nbeta\n")?;

    let case = TerminalCase::new(
        "xterm-256color file open",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );

    let run = run_dun_in_pty(&expect, case, &[file_path.as_os_str()], "alpha", CTRL_Q);
    let _ = fs::remove_file(&file_path);
    let run = run?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert_output_contains(&run.output, &file_name, case.name);
    assert_output_contains(&run.output, "alpha", case.name);
    assert_output_contains(&run.output, "beta", case.name);
    assert_output_contains(&run.output, "Opened ", case.name);
    assert_output_contains(&run.output, "Text UTF-8", case.name);

    Ok(())
}

#[test]
fn pty_smoke_handles_small_low_capability_terminal() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let case = TerminalCase::new("small vt100 ascii", "vt100", "C", "C", false, "ASCII/16")
        .sized(12, 40)
        .without_profile_marker();
    let run = run_dun_in_pty(&expect, case, &[], "Untitled", CTRL_Q)?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert_output_contains(&run.output, "Untitled", case.name);

    Ok(())
}

#[test]
fn pty_smoke_renders_escape_payloads_as_text() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let file_path = temp_path("dun-pty-escape", "txt");
    fs::write(&file_path, b"safe\x1b]0;owned\x07\n\x1b[31mred?\x1b[0m\n")?;
    let case = TerminalCase::new(
        "xterm-256color escape payload",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );

    let run = run_dun_in_pty(&expect, case, &[file_path.as_os_str()], "safe", CTRL_Q);
    let _ = fs::remove_file(&file_path);
    let run = run?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert_output_contains(&run.output, "safe", case.name);
    assert_output_contains(&run.output, "red?", case.name);
    assert_output_not_contains(&run.output, "\x1b]0;owned\x07", case.name);
    assert_output_not_contains(&run.output, "\x1b[31mred?\x1b[0m", case.name);

    Ok(())
}

#[test]
fn pty_smoke_opens_invalid_bytes_as_read_only_escapes() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let file_path = temp_path("dun-pty-invalid", "bin");
    fs::write(&file_path, [b'o', b'k', 0xff, b'\n'])?;
    let case = TerminalCase::new(
        "xterm-256color invalid bytes",
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    );

    let run = run_dun_in_pty(
        &expect,
        case,
        &[file_path.as_os_str()],
        "Escaped bytes",
        CTRL_Q,
    );
    let _ = fs::remove_file(&file_path);
    let run = run?;

    assert!(
        run.status.success(),
        "{} failed with status {:?}\n{}",
        case.name,
        run.status,
        run.output
    );
    assert_output_contains(&run.output, "ok\\xFF", case.name);
    assert_output_contains(&run.output, "Escaped bytes", case.name);

    Ok(())
}

fn run_dun_in_pty(
    expect: &Path,
    case: TerminalCase,
    args: &[&OsStr],
    ready_marker: &str,
    input: &[u8],
) -> io::Result<PtyRun> {
    let script_path = temp_path("dun-pty-expect", "tcl");
    fs::write(
        &script_path,
        expect_script_for_dun(case, args, ready_marker, input),
    )?;

    let result = run_expect_script(expect, case, &script_path);
    let _ = fs::remove_file(&script_path);
    result
}

fn run_expect_script(expect: &Path, case: TerminalCase, script_path: &Path) -> io::Result<PtyRun> {
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

fn expect_script_for_dun(
    case: TerminalCase,
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
        tcl_brace_quote(&shell_command_for_dun(case, args)),
        tcl_brace_quote(&regex_literal(ready_marker)),
        tcl_escaped_bytes(input),
        tcl_escaped_bytes(input)
    )
}

fn shell_command_for_dun(case: TerminalCase, args: &[&OsStr]) -> String {
    let mut command = format!("stty rows {} cols {}; exec ", case.rows, case.cols);
    command.push_str(&shell_quote(dun_binary().as_os_str()));
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

fn assert_output_contains(output: &str, needle: &str, case: &str) {
    assert!(
        output.contains(needle),
        "{case} output did not contain {needle:?}\n{output}"
    );
}

fn assert_output_not_contains(output: &str, needle: &str, case: &str) {
    assert!(
        !output.contains(needle),
        "{case} output unexpectedly contained {needle:?}\n{output}"
    );
}

fn pty_test_guard() -> MutexGuard<'static, ()> {
    PTY_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn command_on_path(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    env::split_paths(&paths)
        .map(|path| path.join(name))
        .find(|path| path.is_file())
}

fn temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    env::temp_dir().join(format!("{prefix}-{}-{nanos}.{suffix}", std::process::id()))
}
