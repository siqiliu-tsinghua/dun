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
const PTY_ROWS: u16 = 24;
const PTY_COLS: u16 = 80;
static PTY_TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug)]
struct TerminalCase {
    name: &'static str,
    term: &'static str,
    lang: &'static str,
    lc_ctype: &'static str,
    no_color: bool,
    expected_profile: &'static str,
}

#[derive(Debug)]
struct PtyRun {
    status: ExitStatus,
    output: String,
}

#[test]
fn pty_smoke_quits_cleanly_for_common_terminal_profiles() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY smoke test: expect(1) is not on PATH");
        return Ok(());
    };

    let cases = [
        TerminalCase {
            name: "xterm-256color utf8",
            term: "xterm-256color",
            lang: "en_US.UTF-8",
            lc_ctype: "en_US.UTF-8",
            no_color: false,
            expected_profile: "UTF-8/256",
        },
        TerminalCase {
            name: "screen-256color utf8",
            term: "screen-256color",
            lang: "en_US.UTF-8",
            lc_ctype: "en_US.UTF-8",
            no_color: false,
            expected_profile: "UTF-8/256",
        },
        TerminalCase {
            name: "vt100 ascii",
            term: "vt100",
            lang: "C",
            lc_ctype: "C",
            no_color: false,
            expected_profile: "ASCII/16",
        },
    ];

    for case in cases {
        let run = run_dun_in_pty(&expect, case, &[], CTRL_Q)?;
        assert!(
            run.status.success(),
            "{} failed with status {:?}\n{}",
            case.name,
            run.status,
            run.output
        );
        assert_output_contains(&run.output, "Untitled", case.name);
        assert_output_contains(&run.output, "Ln 1/1, Col 1", case.name);
        assert_output_contains(&run.output, case.expected_profile, case.name);
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

    let case = TerminalCase {
        name: "xterm-256color file open",
        term: "xterm-256color",
        lang: "en_US.UTF-8",
        lc_ctype: "en_US.UTF-8",
        no_color: false,
        expected_profile: "UTF-8/256",
    };

    let run = run_dun_in_pty(&expect, case, &[file_path.as_os_str()], CTRL_Q);
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

    Ok(())
}

fn run_dun_in_pty(
    expect: &Path,
    case: TerminalCase,
    args: &[&OsStr],
    input: &[u8],
) -> io::Result<PtyRun> {
    let script_path = temp_path("dun-pty-expect", "tcl");
    fs::write(&script_path, expect_script_for_dun(args, input))?;

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
        .env("COLUMNS", PTY_COLS.to_string())
        .env("LINES", PTY_ROWS.to_string())
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

fn expect_script_for_dun(args: &[&OsStr], input: &[u8]) -> String {
    format!(
        "\
set timeout 5
log_user 1
spawn -noecho /bin/sh -lc {}
after 500
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
        tcl_brace_quote(&shell_command_for_dun(args)),
        tcl_escaped_bytes(input)
    )
}

fn shell_command_for_dun(args: &[&OsStr]) -> String {
    let mut command = format!("stty rows {PTY_ROWS} cols {PTY_COLS}; exec ");
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
