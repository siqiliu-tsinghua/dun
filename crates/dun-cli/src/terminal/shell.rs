use std::env;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use super::{RuntimeAction, TerminalGuard, TerminalWriter};
use crate::app::AppState;
use crate::command_output::{CapturedCommandStream, CommandRunResult};

pub(crate) fn handle_runtime_action(
    action: RuntimeAction,
    terminal: &mut Terminal<CrosstermBackend<TerminalWriter>>,
    app: &mut AppState,
    guard: &mut TerminalGuard,
) -> io::Result<()> {
    match action {
        RuntimeAction::ShellEscape => run_shell_escape(terminal, app, guard),
        RuntimeAction::WriteTerminal(payload) => {
            let mut stdout = io::stdout();
            stdout.write_all(payload.as_bytes())?;
            stdout.flush()?;
            Ok(())
        }
    }
}

fn run_shell_escape(
    terminal: &mut Terminal<CrosstermBackend<TerminalWriter>>,
    app: &mut AppState,
    guard: &mut TerminalGuard,
) -> io::Result<()> {
    terminal.show_cursor()?;
    guard.suspend()?;
    let status = run_interactive_shell();
    let resume_result = guard.resume(app.mouse_enabled());
    if resume_result.is_ok() {
        terminal.clear()?;
    }

    match (status, resume_result) {
        (Ok(status), Ok(())) => {
            app.set_status(format!("Shell returned {}", exit_status_text(status)));
            Ok(())
        }
        (Err(error), Ok(())) => {
            app.set_status(format!("Shell failed: {error}"));
            Ok(())
        }
        (_, Err(error)) => Err(error),
    }
}

fn run_interactive_shell() -> io::Result<ExitStatus> {
    Command::new(shell_program()).status()
}

pub(crate) fn run_command_capture(
    command: &str,
    stream_limit: usize,
) -> io::Result<CommandRunResult> {
    let shell = shell_program();
    let started = Instant::now();
    let mut child = Command::new(&shell)
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("failed to capture command stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("failed to capture command stderr"))?;
    let stdout_reader = std::thread::spawn(move || read_capped_stream(stdout, stream_limit));
    let stderr_reader = std::thread::spawn(move || read_capped_stream(stderr, stream_limit));
    let status = child.wait()?;
    let elapsed = started.elapsed();
    let stdout = join_captured_stream(stdout_reader)?;
    let stderr = join_captured_stream(stderr_reader)?;

    Ok(CommandRunResult {
        command: command.to_string(),
        shell,
        status,
        elapsed,
        stdout,
        stderr,
    })
}

fn shell_program() -> OsString {
    env::var_os("SHELL")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| OsString::from("/bin/sh"))
}

fn read_capped_stream<R: Read>(mut reader: R, limit: usize) -> io::Result<CapturedCommandStream> {
    let mut bytes = Vec::new();
    let mut truncated = false;
    let mut chunk = [0u8; 8192];
    loop {
        let read = reader.read(&mut chunk)?;
        if read == 0 {
            break;
        }

        let remaining = limit.saturating_sub(bytes.len());
        if remaining >= read {
            bytes.extend_from_slice(&chunk[..read]);
        } else {
            bytes.extend_from_slice(&chunk[..remaining]);
            truncated = true;
        }
        if remaining == 0 {
            truncated = true;
        }
    }

    Ok(CapturedCommandStream { bytes, truncated })
}

fn join_captured_stream(
    handle: std::thread::JoinHandle<io::Result<CapturedCommandStream>>,
) -> io::Result<CapturedCommandStream> {
    handle
        .join()
        .map_err(|_| io::Error::other("command output reader panicked"))?
}

pub(crate) fn command_run_status(result: &CommandRunResult) -> String {
    let mut status = format!(
        "Command returned {} in {}",
        exit_status_text(result.status),
        duration_status_text(result.elapsed)
    );
    if result.stdout.truncated || result.stderr.truncated {
        status.push_str("; output truncated");
    }
    status
}

pub(crate) fn exit_status_text(status: ExitStatus) -> String {
    status
        .code()
        .map(|code| format!("exit {code}"))
        .unwrap_or_else(|| "terminated".to_string())
}

pub(crate) fn duration_status_text(duration: Duration) -> String {
    if duration.as_secs() >= 1 {
        format!("{:.2}s", duration.as_secs_f64())
    } else {
        format!("{}ms", duration.as_millis())
    }
}
