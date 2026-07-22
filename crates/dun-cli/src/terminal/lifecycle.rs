use std::io::{self, Write};
use std::sync::Arc;

use super::Terminal;
use super::vt::output::{self as vt_output, Sequence};

trait RawMode {
    fn enter_raw(&self) -> io::Result<()>;
    fn restore_raw(&self) -> io::Result<()>;
}

impl RawMode for Terminal {
    fn enter_raw(&self) -> io::Result<()> {
        Terminal::enter_raw(self)
    }

    fn restore_raw(&self) -> io::Result<()> {
        Terminal::restore_raw(self)
    }
}

/// Restore the terminal before the process dies on a panic. The release
/// profile uses `panic = "abort"`, so `TerminalGuard::drop` never runs on
/// panic; without this hook any panic leaves the user's terminal in raw
/// mode on the alternate screen. Panic hooks run before the abort.
pub(crate) fn install_panic_terminal_restore(terminal: Arc<Terminal>) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut stdout = io::stdout();
        let _ = RestoreExecutor::new(&mut stdout, terminal.as_ref()).run(true, true);
        default_hook(info);
    }));
}

/// Keep the first error of a best-effort sequence while still running the rest.
/// A restore path must attempt every step -- above all raw-mode restoration --
/// and only afterwards report what went wrong.
fn record_first_error(slot: &mut Option<io::Error>, result: io::Result<()>) {
    if let Err(error) = result {
        if slot.is_none() {
            *slot = Some(error);
        }
    }
}

struct RestoreExecutor<'a, W: Write + ?Sized, R: RawMode + ?Sized> {
    writer: &'a mut W,
    raw_mode: &'a R,
    first_error: Option<io::Error>,
}

impl<'a, W: Write + ?Sized, R: RawMode + ?Sized> RestoreExecutor<'a, W, R> {
    fn new(writer: &'a mut W, raw_mode: &'a R) -> Self {
        Self {
            writer,
            raw_mode,
            first_error: None,
        }
    }

    fn sequence(&mut self, sequence: Sequence) {
        let result = vt_output::execute(self.writer, sequence);
        record_first_error(&mut self.first_error, result);
    }

    fn run(mut self, mouse_enabled: bool, bracketed_paste_enabled: bool) -> io::Result<()> {
        if mouse_enabled {
            self.sequence(Sequence::DisableMouseCapture);
        }
        if bracketed_paste_enabled {
            self.sequence(Sequence::DisableBracketedPaste);
        }
        self.sequence(Sequence::LeaveAlternateScreen);
        let flush_result = self.writer.flush();
        record_first_error(&mut self.first_error, flush_result);
        let raw_result = self.raw_mode.restore_raw();
        record_first_error(&mut self.first_error, raw_result);

        match self.first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

fn enter_modes(writer: &mut (impl Write + ?Sized), mouse_enabled: bool) -> io::Result<()> {
    vt_output::execute(writer, Sequence::EnterAlternateScreen)?;
    vt_output::execute(writer, Sequence::EnableBracketedPaste)?;
    if mouse_enabled {
        vt_output::execute(writer, Sequence::EnableMouseCapture)?;
    }
    Ok(())
}

fn enter_with(
    raw_mode: &(impl RawMode + ?Sized),
    writer: &mut (impl Write + ?Sized),
    mouse_enabled: bool,
) -> io::Result<()> {
    raw_mode.enter_raw()?;
    if let Err(error) = enter_modes(writer, mouse_enabled) {
        let _ = raw_mode.restore_raw();
        return Err(error);
    }
    Ok(())
}

pub(crate) struct TerminalGuard {
    terminal: Arc<Terminal>,
    mouse_enabled: bool,
    bracketed_paste_enabled: bool,
    active: bool,
}

impl TerminalGuard {
    pub(crate) fn enter(terminal: Arc<Terminal>, mouse_enabled: bool) -> io::Result<Self> {
        let mut stdout = io::stdout();
        enter_with(terminal.as_ref(), &mut stdout, mouse_enabled)?;
        Ok(Self {
            terminal,
            mouse_enabled,
            bracketed_paste_enabled: true,
            active: true,
        })
    }

    pub(crate) fn set_mouse_enabled(&mut self, enabled: bool) -> io::Result<()> {
        if self.mouse_enabled == enabled {
            return Ok(());
        }
        if !self.active {
            self.mouse_enabled = enabled;
            return Ok(());
        }

        let mut stdout = io::stdout();
        if enabled {
            vt_output::execute(&mut stdout, Sequence::EnableMouseCapture)?;
        } else {
            vt_output::execute(&mut stdout, Sequence::DisableMouseCapture)?;
        }
        self.mouse_enabled = enabled;
        Ok(())
    }

    pub(crate) fn suspend(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        let mut stdout = io::stdout();
        let result = RestoreExecutor::new(&mut stdout, self.terminal.as_ref())
            .run(self.mouse_enabled, self.bracketed_paste_enabled);
        self.active = false;
        result
    }

    pub(crate) fn resume(&mut self, mouse_enabled: bool) -> io::Result<()> {
        if self.active {
            self.set_mouse_enabled(mouse_enabled)?;
            return Ok(());
        }

        let mut stdout = io::stdout();
        enter_with(self.terminal.as_ref(), &mut stdout, mouse_enabled)?;
        self.mouse_enabled = mouse_enabled;
        self.bracketed_paste_enabled = true;
        self.active = true;
        Ok(())
    }

    fn restore(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout();
        let result = RestoreExecutor::new(&mut stdout, self.terminal.as_ref())
            .run(self.mouse_enabled, self.bracketed_paste_enabled);
        self.active = false;
        result
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = self.restore();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RawMode, RestoreExecutor, enter_with};
    use std::cell::RefCell;
    use std::io::{self, Write};

    #[derive(Default)]
    struct RecordingRawMode {
        calls: RefCell<Vec<&'static str>>,
        restore_error: Option<io::ErrorKind>,
    }

    impl RawMode for RecordingRawMode {
        fn enter_raw(&self) -> io::Result<()> {
            self.calls.borrow_mut().push("enter_raw");
            Ok(())
        }

        fn restore_raw(&self) -> io::Result<()> {
            self.calls.borrow_mut().push("restore_raw");
            match self.restore_error {
                Some(kind) => Err(io::Error::new(kind, "raw restore failed")),
                None => Ok(()),
            }
        }
    }

    #[derive(Default)]
    struct RecordingWriter {
        attempted_writes: Vec<Vec<u8>>,
        written: Vec<u8>,
        flushes: usize,
        fail_write: Option<(usize, io::ErrorKind)>,
    }

    impl Write for RecordingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let call = self.attempted_writes.len();
            self.attempted_writes.push(bytes.to_vec());
            if let Some((failed_call, kind)) = self.fail_write {
                if call == failed_call {
                    return Err(io::Error::new(kind, "write failed"));
                }
            }
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }

    #[test]
    fn failed_mode_entry_rolls_raw_mode_back() {
        let raw_mode = RecordingRawMode::default();
        let mut writer = RecordingWriter {
            fail_write: Some((0, io::ErrorKind::BrokenPipe)),
            ..RecordingWriter::default()
        };

        let error = enter_with(&raw_mode, &mut writer, false)
            .expect_err("alternate-screen entry must fail");

        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(
            raw_mode.calls.into_inner(),
            vec!["enter_raw", "restore_raw"]
        );
    }

    #[test]
    fn cleanup_attempts_every_step_and_returns_the_first_error() {
        let raw_mode = RecordingRawMode {
            calls: RefCell::new(Vec::new()),
            restore_error: Some(io::ErrorKind::PermissionDenied),
        };
        let mut writer = RecordingWriter {
            fail_write: Some((0, io::ErrorKind::BrokenPipe)),
            ..RecordingWriter::default()
        };

        let error = RestoreExecutor::new(&mut writer, &raw_mode)
            .run(true, true)
            .expect_err("cleanup must report its first failure");

        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(
            writer.attempted_writes,
            vec![
                b"\x1b[?1006l\x1b[?1002l\x1b[?1000l".to_vec(),
                b"\x1b[?2004l".to_vec(),
                b"\x1b[?1049l".to_vec(),
            ]
        );
        assert_eq!(writer.written, b"\x1b[?2004l\x1b[?1049l");
        assert_eq!(writer.flushes, 3);
        assert_eq!(raw_mode.calls.into_inner(), vec!["restore_raw"]);
    }
}
