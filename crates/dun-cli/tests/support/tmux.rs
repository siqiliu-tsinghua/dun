#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::pty::command_on_path;
use super::terminal_grid::{TerminalCursor, parse_terminal_grid};

pub use super::terminal_grid::TerminalGrid as TmuxGrid;

static TMUX_TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub struct TmuxCapture {
    pub text: String,
    pub lines: Vec<String>,
}

impl TmuxCapture {
    fn from_bytes(bytes: Vec<u8>) -> Self {
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let lines = text.lines().map(str::to_string).collect();

        Self { text, lines }
    }

    pub fn line(&self, row: usize) -> &str {
        self.lines.get(row).map(String::as_str).unwrap_or_default()
    }
}

#[derive(Debug)]
pub struct TmuxSession {
    tmux: PathBuf,
    name: String,
    cols: u16,
    rows: u16,
}

impl TmuxSession {
    pub fn start_dun(
        label: &str,
        cols: u16,
        rows: u16,
        args: &[&OsStr],
    ) -> io::Result<Option<Self>> {
        let Some(tmux) = command_on_path("tmux") else {
            eprintln!("skipping tmux grid test: tmux(1) is not on PATH");
            return Ok(None);
        };
        if !Command::new(&tmux).arg("-V").output()?.status.success() {
            eprintln!("skipping tmux grid test: tmux -V failed");
            return Ok(None);
        }

        let name = unique_session_name(label);
        let command = dun_shell_command(args);
        let output = Command::new(&tmux)
            .args([
                "new-session",
                "-d",
                "-s",
                &name,
                "-x",
                &cols.to_string(),
                "-y",
                &rows.to_string(),
            ])
            .arg(command)
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Operation not permitted") {
                eprintln!(
                    "skipping tmux grid test: tmux cannot create its socket in this sandbox: {}",
                    stderr.trim()
                );
                return Ok(None);
            }

            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("tmux new-session failed: {}", stderr.trim()),
            ));
        }

        Ok(Some(Self {
            tmux,
            name,
            cols,
            rows,
        }))
    }

    pub fn send_keys(&self, keys: &[&str]) -> io::Result<()> {
        let mut args = vec!["send-keys", "-t", &self.name];
        args.extend(keys.iter().copied());
        self.checked_status(&args)
    }

    pub fn capture_plain(&self) -> io::Result<TmuxCapture> {
        self.capture(false)
    }

    pub fn capture_sgr(&self) -> io::Result<TmuxCapture> {
        self.capture(true)
    }

    pub fn capture_grid(&self) -> io::Result<TmuxGrid> {
        let capture = self.capture_sgr()?;
        let cursor = self.cursor_position()?;
        Ok(parse_terminal_grid(
            &capture.text,
            self.cols,
            self.rows,
            cursor,
        ))
    }

    pub fn capture_until_contains(
        &self,
        needle: &str,
        timeout: Duration,
    ) -> io::Result<TmuxCapture> {
        let start = Instant::now();
        loop {
            let capture = self.capture_plain()?;
            if capture.text.contains(needle) {
                return Ok(capture);
            }
            if start.elapsed() >= timeout {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("tmux pane did not contain {needle:?} within {timeout:?}"),
                ));
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn capture_stable(&self, timeout: Duration) -> io::Result<TmuxCapture> {
        let start = Instant::now();
        let mut last = self.capture_plain()?.text;
        loop {
            thread::sleep(Duration::from_millis(50));
            let capture = self.capture_plain()?;
            if capture.text == last {
                return Ok(capture);
            }
            if start.elapsed() >= timeout {
                return Ok(capture);
            }
            last = capture.text;
        }
    }

    fn capture(&self, sgr: bool) -> io::Result<TmuxCapture> {
        let end = self.rows.saturating_sub(1).to_string();
        let flag = if sgr { "-ep" } else { "-p" };
        let output = self.checked_output(&[
            "capture-pane",
            flag,
            "-t",
            &self.name,
            "-S",
            "0",
            "-E",
            &end,
        ])?;
        Ok(TmuxCapture::from_bytes(output.stdout))
    }

    fn cursor_position(&self) -> io::Result<Option<TerminalCursor>> {
        let output = self.checked_output(&[
            "display-message",
            "-p",
            "-t",
            &self.name,
            "#{cursor_x},#{cursor_y}",
        ])?;
        let text = String::from_utf8_lossy(&output.stdout);
        let Some((x, y)) = text.trim().split_once(',') else {
            return Ok(None);
        };
        let (Ok(x), Ok(y)) = (x.parse::<u16>(), y.parse::<u16>()) else {
            return Ok(None);
        };
        Ok(Some(TerminalCursor { x, y }))
    }

    fn checked_status(&self, args: &[&str]) -> io::Result<()> {
        let output = self.checked_output(args)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(output_error(args, &output))
        }
    }

    fn checked_output(&self, args: &[&str]) -> io::Result<Output> {
        let output = Command::new(&self.tmux).args(args).output()?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(output_error(args, &output))
        }
    }
}

impl Drop for TmuxSession {
    fn drop(&mut self) {
        let _ = Command::new(&self.tmux)
            .args(["kill-session", "-t", &self.name])
            .status();
    }
}

pub fn tmux_test_guard() -> MutexGuard<'static, ()> {
    TMUX_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn dun_shell_command(args: &[&OsStr]) -> String {
    let mut command =
        String::from("env TERM='xterm-256color' LANG='en_US.UTF-8' LC_CTYPE='en_US.UTF-8'");
    command.push(' ');
    command.push_str(&shell_quote(OsStr::new(env!("CARGO_BIN_EXE_dun"))));
    for arg in args {
        command.push(' ');
        command.push_str(&shell_quote(arg));
    }
    command
}

fn unique_session_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!(
        "dun-{}-{}-{nanos}",
        sanitize_label(label),
        std::process::id()
    )
}

fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn shell_quote(value: &OsStr) -> String {
    let text = value.to_string_lossy();
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn output_error(args: &[&str], output: &Output) -> io::Error {
    io::Error::new(
        io::ErrorKind::Other,
        format!(
            "tmux {:?} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            args,
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}
