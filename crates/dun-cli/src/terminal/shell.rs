use std::env;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use dun_config::TextCatalog;

use super::{RuntimeAction, SurfaceBackend, TerminalGuard};
use crate::app::AppState;
use crate::command_output::{CapturedCommandStream, CommandRunResult};
use crate::ui_text;

pub(crate) fn handle_runtime_action(
    action: RuntimeAction,
    backend: &mut SurfaceBackend,
    app: &mut AppState,
    guard: &mut TerminalGuard,
) -> io::Result<()> {
    match action {
        RuntimeAction::ShellEscape => run_shell_escape(backend, app, guard),
        RuntimeAction::WriteTerminal(payload) => {
            let mut stdout = io::stdout();
            stdout.write_all(payload.as_bytes())?;
            stdout.flush()?;
            Ok(())
        }
    }
}

fn run_shell_escape(
    backend: &mut SurfaceBackend,
    app: &mut AppState,
    guard: &mut TerminalGuard,
) -> io::Result<()> {
    backend.show_cursor()?;
    guard.suspend()?;
    let status = run_interactive_shell();
    let resume_result = guard.resume(app.mouse_enabled());
    if resume_result.is_ok() {
        backend.clear()?;
    }

    match (status, resume_result) {
        (Ok(status), Ok(())) => {
            let exit = localized_exit_status_text(&app.shell.catalog, status);
            let status =
                ui_text::tr_fmt(&app.shell.catalog, ui_text::STATUS_SHELL_RETURNED, &[&exit]);
            app.set_status(status);
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
    timeout: Duration,
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
    let (status, timed_out) = wait_with_timeout(&mut child, started, timeout)?;
    let elapsed = started.elapsed();
    let stdout = join_captured_stream(stdout_reader)?;
    let stderr = join_captured_stream(stderr_reader)?;

    Ok(CommandRunResult {
        command: command.to_string(),
        shell,
        status,
        elapsed,
        timed_out,
        stdout,
        stderr,
    })
}

/// Poll the child until it exits or the deadline passes; on timeout kill it
/// so a non-terminating command cannot hang the editor. Killing also closes
/// the child's pipes, which unblocks the capture reader threads.
fn wait_with_timeout(
    child: &mut std::process::Child,
    started: Instant,
    timeout: Duration,
) -> io::Result<(ExitStatus, bool)> {
    let deadline = started + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok((status, false));
        }
        if Instant::now() >= deadline {
            child.kill()?;
            let status = child.wait()?;
            return Ok((status, true));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
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

pub(crate) fn command_run_status(catalog: &TextCatalog, result: &CommandRunResult) -> String {
    let truncated = result.stdout.truncated || result.stderr.truncated;
    let duration = duration_status_text(result.elapsed);
    if result.timed_out {
        let key = if truncated {
            ui_text::STATUS_RUN_TIMED_OUT_TRUNCATED
        } else {
            ui_text::STATUS_RUN_TIMED_OUT
        };
        ui_text::tr_fmt(catalog, key, &[&duration])
    } else {
        let key = if truncated {
            ui_text::STATUS_RUN_RETURNED_TRUNCATED
        } else {
            ui_text::STATUS_RUN_RETURNED
        };
        let exit = localized_exit_status_text(catalog, result.status);
        ui_text::tr_fmt(catalog, key, &[&exit, &duration])
    }
}

fn localized_exit_status_text(catalog: &TextCatalog, status: ExitStatus) -> String {
    status
        .code()
        .map(|code| ui_text::tr_fmt(catalog, ui_text::STATUS_RUN_EXIT, &[&code.to_string()]))
        .unwrap_or_else(|| ui_text::tr(catalog, ui_text::STATUS_RUN_TERMINATED).to_string())
}

/// The English form, for Command Output *buffer content* (which is not yet
/// translated). Defined through the localized path with an empty catalog so
/// the two cannot drift apart.
pub(crate) fn exit_status_text(status: ExitStatus) -> String {
    localized_exit_status_text(&TextCatalog::empty(), status)
}

pub(crate) fn duration_status_text(duration: Duration) -> String {
    if duration.as_secs() >= 1 {
        format!("{:.2}s", duration.as_secs_f64())
    } else {
        format!("{}ms", duration.as_millis())
    }
}
