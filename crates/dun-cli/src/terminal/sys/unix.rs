use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::{AsFd, BorrowedFd};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags};
use rustix::io::Errno;
use rustix::termios::{OptionalActions, Termios};

const FALLBACK_COLUMNS: u16 = 80;
const FALLBACK_ROWS: u16 = 24;
const SIZE_RETRIES: u32 = 10;
const MAX_SIZE_RETRY_DELAY_MS: u64 = 90;

enum Input {
    Stdin(io::Stdin),
    Tty(File),
}

impl AsFd for Input {
    fn as_fd(&self) -> BorrowedFd<'_> {
        match self {
            Self::Stdin(stdin) => stdin.as_fd(),
            Self::Tty(tty) => tty.as_fd(),
        }
    }
}

struct RawSnapshot<T> {
    active: Mutex<Option<T>>,
}

impl<T: Clone> RawSnapshot<T> {
    const fn new() -> Self {
        Self {
            active: Mutex::new(None),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<T>> {
        self.active.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn enter_raw(
        &self,
        get: impl FnOnce() -> io::Result<T>,
        make_raw: impl FnOnce(&mut T),
        apply: impl FnOnce(&T) -> io::Result<()>,
    ) -> io::Result<()> {
        let original = get()?;
        let mut raw = original.clone();
        make_raw(&mut raw);
        apply(&raw)?;
        *self.lock() = Some(original);
        Ok(())
    }

    fn restore_raw(&self, apply: impl FnOnce(&T) -> io::Result<()>) -> io::Result<()> {
        let snapshot = { self.lock().clone() };
        let Some(snapshot) = snapshot else {
            return Ok(());
        };

        apply(&snapshot)?;
        *self.lock() = None;
        Ok(())
    }
}

pub(crate) struct Terminal {
    input: Input,
    stdin_is_tty: bool,
    stdout_is_tty: bool,
    raw_snapshot: RawSnapshot<Termios>,
}

impl Terminal {
    pub(crate) fn open() -> io::Result<Arc<Self>> {
        let stdin = io::stdin();
        let stdin_is_tty = rustix::termios::isatty(&stdin);
        let stdout_is_tty = rustix::termios::isatty(io::stdout());
        let input = if stdin_is_tty {
            Input::Stdin(stdin)
        } else {
            Input::Tty(OpenOptions::new().read(true).write(true).open("/dev/tty")?)
        };

        let terminal = Self {
            input,
            stdin_is_tty,
            stdout_is_tty,
            raw_snapshot: RawSnapshot::new(),
        };
        if !stdin_is_tty {
            terminal.validate_pollable()?;
        }
        Ok(Arc::new(terminal))
    }

    pub(crate) fn input_fd(&self) -> BorrowedFd<'_> {
        self.input.as_fd()
    }

    pub(crate) const fn stdin_is_tty(&self) -> bool {
        self.stdin_is_tty
    }

    pub(crate) const fn stdout_is_tty(&self) -> bool {
        self.stdout_is_tty
    }

    pub(crate) fn enter_raw(&self) -> io::Result<()> {
        self.raw_snapshot.enter_raw(
            || rustix::termios::tcgetattr(self.input_fd()).map_err(Into::into),
            Termios::make_raw,
            |termios| {
                rustix::termios::tcsetattr(self.input_fd(), OptionalActions::Now, termios)
                    .map_err(Into::into)
            },
        )
    }

    pub(crate) fn restore_raw(&self) -> io::Result<()> {
        self.raw_snapshot.restore_raw(|termios| {
            rustix::termios::tcsetattr(self.input_fd(), OptionalActions::Now, termios)
                .map_err(Into::into)
        })
    }

    pub(crate) fn size(&self) -> io::Result<(u16, u16)> {
        for retry in 0..=SIZE_RETRIES {
            let size = rustix::termios::tcgetwinsize(self.input_fd()).map_err(io::Error::from)?;
            if size.ws_col != 0 && size.ws_row != 0 {
                return Ok((size.ws_col, size.ws_row));
            }
            if retry == SIZE_RETRIES {
                break;
            }

            let delay_ms = u64::from(retry + 1)
                .saturating_mul(10)
                .min(MAX_SIZE_RETRY_DELAY_MS);
            thread::sleep(Duration::from_millis(delay_ms));
        }

        Ok((FALLBACK_COLUMNS, FALLBACK_ROWS))
    }

    pub(crate) fn poll_readable(&self, deadline: Instant) -> io::Result<Readiness> {
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(Readiness::TimedOut);
            }
            let timeout_ms = remaining.as_millis().clamp(1, i32::MAX as u128) as i32;
            match self.poll_once(timeout_ms) {
                Ok(None) => return Ok(Readiness::TimedOut),
                Ok(Some(events)) => {
                    if let Some(readiness) = Self::readiness(events)? {
                        return Ok(readiness);
                    }
                }
                Err(Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }

    pub(crate) fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        rustix::io::read(self.input_fd(), buffer).map_err(Into::into)
    }

    fn validate_pollable(&self) -> io::Result<()> {
        loop {
            match self.poll_once(0) {
                Ok(None) => return Ok(()),
                Ok(Some(events)) => return Self::readiness(events).map(|_| ()),
                Err(Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn poll_once(&self, timeout_ms: i32) -> Result<Option<PollFlags>, Errno> {
        let mut poll_fds = [PollFd::from_borrowed_fd(self.input_fd(), PollFlags::IN)];
        match rustix::event::poll(&mut poll_fds, timeout_ms)? {
            0 => Ok(None),
            _ => Ok(Some(poll_fds[0].revents())),
        }
    }

    fn readiness(events: PollFlags) -> io::Result<Option<Readiness>> {
        if events.contains(PollFlags::IN) {
            return Ok(Some(Readiness::Readable));
        }
        if events.intersects(PollFlags::NVAL | PollFlags::ERR | PollFlags::HUP) {
            return Err(io::Error::other(format!(
                "terminal input polling reported {events:?}; a real terminal on stdin is required"
            )));
        }
        Ok(None)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Readiness {
    Readable,
    TimedOut,
}

#[cfg(test)]
mod tests {
    use super::{RawSnapshot, Readiness, Terminal};
    use std::cell::RefCell;
    use std::io;

    use rustix::event::PollFlags;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct RecordedTermios {
        canonical: bool,
        echo: bool,
        marker: u8,
    }

    #[test]
    fn raw_snapshot_is_captured_before_modification_and_restored_exactly() {
        let original = RecordedTermios {
            canonical: true,
            echo: true,
            marker: 37,
        };
        let applied = RefCell::new(Vec::new());
        let snapshot = RawSnapshot::new();

        snapshot
            .enter_raw(
                || Ok(original.clone()),
                |termios| {
                    termios.canonical = false;
                    termios.echo = false;
                },
                |termios| {
                    applied.borrow_mut().push(termios.clone());
                    Ok(())
                },
            )
            .expect("enter raw");
        snapshot
            .restore_raw(|termios| {
                applied.borrow_mut().push(termios.clone());
                Ok(())
            })
            .expect("restore raw");

        assert_eq!(
            applied.into_inner(),
            vec![
                RecordedTermios {
                    canonical: false,
                    echo: false,
                    marker: 37,
                },
                original,
            ]
        );
    }

    #[test]
    fn failed_restore_keeps_the_snapshot_for_an_exact_retry() {
        let original = RecordedTermios {
            canonical: true,
            echo: false,
            marker: 91,
        };
        let restored = RefCell::new(Vec::new());
        let snapshot = RawSnapshot::new();
        snapshot
            .enter_raw(
                || Ok(original.clone()),
                |termios| termios.canonical = false,
                |_| Ok(()),
            )
            .expect("enter raw");

        let error = snapshot
            .restore_raw(|termios| {
                restored.borrow_mut().push(termios.clone());
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "first restore"))
            })
            .expect_err("first restore must fail");
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        snapshot
            .restore_raw(|termios| {
                restored.borrow_mut().push(termios.clone());
                Ok(())
            })
            .expect("retry restore");

        assert_eq!(restored.into_inner(), vec![original.clone(), original]);
    }

    #[test]
    fn terminal_poll_errors_require_a_real_terminal_on_stdin() {
        let error = Terminal::readiness(PollFlags::NVAL).expect_err("NVAL must be an error");

        assert!(
            error
                .to_string()
                .contains("a real terminal on stdin is required")
        );
    }

    #[test]
    fn readable_data_is_consumed_before_a_simultaneous_terminal_error() {
        let readiness = Terminal::readiness(PollFlags::IN | PollFlags::HUP)
            .expect("readable data takes precedence");

        assert_eq!(readiness, Some(Readiness::Readable));
    }
}
