use std::env;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::os::fd::AsFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use dun_config::TextCatalog;
use dun_plugin::group_kill_target;
use rustix::event::{PollFd, PollFlags};
use rustix::io::Errno;
use rustix::process::{Signal, getpgrp, kill_process_group};

use super::{EventReader, RuntimeAction, SurfaceBackend, TerminalGuard, osc52_read_query};
use crate::app::AppState;
use crate::command_output::{CapturedCommandStream, CommandRunResult};
use crate::ui_text;

pub(crate) const OSC52_READ_TIMEOUT: Duration = Duration::from_millis(500);

pub(crate) struct PendingOsc52Read {
    pub(crate) deadline: Instant,
    pub(crate) max_bytes: usize,
}

pub(crate) fn handle_runtime_action(
    action: RuntimeAction,
    backend: &mut SurfaceBackend,
    app: &mut AppState,
    guard: &mut TerminalGuard,
    event_reader: &mut EventReader,
) -> io::Result<Option<PendingOsc52Read>> {
    match action {
        RuntimeAction::ShellEscape => {
            run_shell_escape(backend, app, guard)?;
            Ok(None)
        }
        RuntimeAction::WriteTerminal(payload) => {
            write_terminal(&payload)?;
            Ok(None)
        }
        RuntimeAction::QueryOsc52Clipboard { max_bytes } => {
            event_reader.begin_osc52_query(max_bytes);
            write_terminal(osc52_read_query())?;
            Ok(Some(PendingOsc52Read {
                deadline: Instant::now() + OSC52_READ_TIMEOUT,
                max_bytes,
            }))
        }
    }
}

fn write_terminal(payload: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.write_all(payload.as_bytes())?;
    stdout.flush()
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
    let deadline = started + timeout;
    let mut child = Command::new(&shell)
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("failed to capture command stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("failed to capture command stderr"))?;
    let stdout_reader =
        std::thread::spawn(move || read_capped_stream(stdout, stream_limit, deadline));
    let stderr_reader =
        std::thread::spawn(move || read_capped_stream(stderr, stream_limit, deadline));
    let wait_result = wait_with_timeout(&mut child, deadline);
    let stdout_result = join_captured_stream(stdout_reader);
    let stderr_result = join_captured_stream(stderr_reader);
    let elapsed = started.elapsed();
    let (status, timed_out, background_processes_killed) = wait_result?;
    let mut stdout = stdout_result?;
    let mut stderr = stderr_result?;
    if background_processes_killed {
        stdout.truncated = true;
        stderr.truncated = true;
    }

    Ok(CommandRunResult {
        command: command.to_string(),
        shell,
        status,
        elapsed,
        timed_out,
        background_processes_killed,
        stdout,
        stderr,
    })
}

/// Poll the child until it exits or the deadline passes, clean up its process
/// group, and reap the direct child so a command cannot hang the editor or
/// leave descendants behind.
fn wait_with_timeout(
    child: &mut std::process::Child,
    deadline: Instant,
) -> io::Result<(ExitStatus, bool, bool)> {
    loop {
        if let Some(status) = child.try_wait()? {
            let background_processes_killed = kill_command_process_group(child.id())?;
            return Ok((status, false, background_processes_killed));
        }
        if Instant::now() >= deadline {
            let group_kill_result = kill_command_process_group(child.id());
            if !matches!(group_kill_result, Ok(true)) {
                child.kill()?;
            }
            let status = child.wait()?;
            // The group still holds the foreground shell here, so a successful
            // signal proves nothing about *background* processes — it only
            // proves we killed the command we were already reporting as timed
            // out. Claiming otherwise would also force the output to be
            // reported truncated when it had completed.
            return group_kill_result.map(|_| (status, true, false));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn kill_command_process_group(child_pid: u32) -> io::Result<bool> {
    let own_pgid = getpgrp().as_raw_nonzero().get() as u32;
    let Some(target) = group_kill_target(child_pid, own_pgid) else {
        return Ok(false);
    };
    match kill_process_group(target, Signal::Kill) {
        Ok(()) => Ok(true),
        Err(Errno::SRCH) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn shell_program() -> OsString {
    env::var_os("SHELL")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| OsString::from("/bin/sh"))
}

fn read_capped_stream<R: Read + AsFd>(
    mut reader: R,
    limit: usize,
    deadline: Instant,
) -> io::Result<CapturedCommandStream> {
    let mut bytes = Vec::new();
    let mut truncated = false;
    let mut chunk = [0u8; 8192];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            truncated = true;
            break;
        }
        let timeout_ms = remaining.as_millis().clamp(1, i32::MAX as u128) as i32;
        let poll_result = {
            let mut poll_fds = [PollFd::from_borrowed_fd(reader.as_fd(), PollFlags::IN)];
            rustix::event::poll(&mut poll_fds, timeout_ms)
                .map(|ready| (ready, poll_fds[0].revents()))
        };
        let events = match poll_result {
            Ok((0, _)) => continue,
            Ok((_, events)) => events,
            Err(Errno::INTR) => continue,
            Err(error) => return Err(error.into()),
        };
        if !events.intersects(PollFlags::IN | PollFlags::HUP) {
            if events.intersects(PollFlags::NVAL | PollFlags::ERR) {
                return Err(io::Error::other(format!(
                    "command output polling reported {events:?}"
                )));
            }
            continue;
        }

        let read = match reader.read(&mut chunk) {
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
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
        let key = match (truncated, result.background_processes_killed) {
            (false, false) => ui_text::STATUS_RUN_RETURNED,
            (true, false) => ui_text::STATUS_RUN_RETURNED_TRUNCATED,
            (false, true) => ui_text::STATUS_RUN_RETURNED_BACKGROUND_PROCESSES_KILLED,
            (true, true) => ui_text::STATUS_RUN_RETURNED_TRUNCATED_BACKGROUND_PROCESSES_KILLED,
        };
        let exit = localized_exit_status_text(catalog, result.status);
        ui_text::tr_fmt(catalog, key, &[&exit, &duration])
    }
}

pub(crate) fn localized_exit_status_text(catalog: &TextCatalog, status: ExitStatus) -> String {
    status
        .code()
        .map(|code| ui_text::tr_fmt(catalog, ui_text::STATUS_RUN_EXIT, &[&code.to_string()]))
        .unwrap_or_else(|| ui_text::tr(catalog, ui_text::STATUS_RUN_TERMINATED).to_string())
}

pub(crate) fn duration_status_text(duration: Duration) -> String {
    if duration.as_secs() >= 1 {
        format!("{:.2}s", duration.as_secs_f64())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, Duration, Instant, Stdio, read_capped_stream};
    use std::sync::mpsc;

    #[test]
    fn command_capture_deadline_releases_open_pipe() {
        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg("read ignored")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("open-pipe helper should start");
        let held_open = child
            .stdin
            .take()
            .expect("open-pipe helper stdin should be piped");
        let stdout = child
            .stdout
            .take()
            .expect("open-pipe helper stdout should be piped");
        let started = Instant::now();
        let deadline = started + Duration::from_millis(100);
        let (sender, receiver) = mpsc::sync_channel(1);
        let reader = std::thread::spawn(move || {
            let result = read_capped_stream(stdout, 16, deadline);
            sender
                .send((started.elapsed(), result))
                .expect("reader result receiver should remain open");
        });

        let received = receiver.recv_timeout(Duration::from_millis(750));
        drop(held_open);
        child
            .wait()
            .expect("open-pipe helper should exit after stdin closes");
        reader.join().expect("capture reader should not panic");

        let (elapsed, captured) = received.unwrap_or_else(|error| {
            panic!("capture reader missed its deadline and required supervised release: {error}")
        });
        assert!(
            elapsed < Duration::from_millis(500),
            "capture reader returned too late: {elapsed:?}"
        );
        let captured = captured.expect("capture reader should succeed");
        assert!(captured.bytes.is_empty());
        assert!(captured.truncated);
    }
}
