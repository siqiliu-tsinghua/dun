use std::io::{self, Write};

use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

/// Restore the terminal before the process dies on a panic. The release
/// profile uses `panic = "abort"`, so `TerminalGuard::drop` never runs on
/// panic; without this hook any panic leaves the user's terminal in raw
/// mode on the alternate screen. Panic hooks run before the abort.
pub(crate) fn install_panic_terminal_restore() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, DisableMouseCapture);
        let _ = execute!(stdout, DisableBracketedPaste);
        let _ = execute!(stdout, LeaveAlternateScreen);
        let _ = stdout.flush();
        let _ = disable_raw_mode();
        default_hook(info);
    }));
}

/// Keep the first error of a best-effort sequence while still running the rest.
/// A restore path must attempt every step -- above all `disable_raw_mode` -- and
/// only afterwards report what went wrong.
fn record_first_error(slot: &mut Option<io::Error>, result: io::Result<()>) {
    if let Err(error) = result {
        if slot.is_none() {
            *slot = Some(error);
        }
    }
}

pub(crate) struct TerminalGuard {
    mouse_enabled: bool,
    bracketed_paste_enabled: bool,
    active: bool,
}

impl TerminalGuard {
    pub(crate) fn enter(mouse_enabled: bool) -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = Self::enter_modes(mouse_enabled) {
            // A failed escape-sequence write means stdout is already broken --
            // and every "rollback" would write LeaveAlternateScreen and friends
            // to that same broken stdout, so it could not succeed. The one undo
            // that reaches a different fd (raw mode is a tcsetattr on /dev/tty)
            // and the one piece of state that survives the process is raw mode,
            // so undo just that and bail. See the module notes on why the
            // stacked escape rollbacks this replaced were theatre.
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self {
            mouse_enabled,
            bracketed_paste_enabled: true,
            active: true,
        })
    }

    /// Switch stdout into the modes an active editor needs. Shared by `enter`
    /// and `resume`, which used to carry identical copies. Raw mode is the
    /// caller's responsibility -- it lives on a different fd and a different
    /// lifecycle.
    fn enter_modes(mouse_enabled: bool) -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        execute!(stdout, EnableBracketedPaste)?;
        if mouse_enabled {
            execute!(stdout, EnableMouseCapture)?;
        }
        Ok(())
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
            execute!(stdout, EnableMouseCapture)?;
        } else {
            execute!(stdout, DisableMouseCapture)?;
        }
        self.mouse_enabled = enabled;
        Ok(())
    }

    pub(crate) fn suspend(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        // Suspend hands the terminal back to a shell, so like `drop` it must
        // reach `disable_raw_mode` no matter what: bailing on the first failed
        // escape write (the previous `?` behaviour) could return with raw mode
        // still on and `active` still true -- a shell left unusable and a guard
        // that thinks it still owns the terminal. Attempt every escape sequence,
        // keep the first error, always undo raw mode, settle the state, then
        // report. Raw mode is the load-bearing undo; the escape writes are
        // best-effort because a broken stdout cannot receive them anyway.
        let mut stdout = io::stdout();
        let mut first_error = None;
        if self.mouse_enabled {
            record_first_error(&mut first_error, execute!(stdout, DisableMouseCapture));
        }
        if self.bracketed_paste_enabled {
            record_first_error(&mut first_error, execute!(stdout, DisableBracketedPaste));
        }
        record_first_error(&mut first_error, execute!(stdout, LeaveAlternateScreen));
        record_first_error(&mut first_error, stdout.flush());
        record_first_error(&mut first_error, disable_raw_mode());
        self.active = false;

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub(crate) fn resume(&mut self, mouse_enabled: bool) -> io::Result<()> {
        if self.active {
            self.set_mouse_enabled(mouse_enabled)?;
            return Ok(());
        }

        enable_raw_mode()?;
        if let Err(error) = Self::enter_modes(mouse_enabled) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        self.mouse_enabled = mouse_enabled;
        self.bracketed_paste_enabled = true;
        self.active = true;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        let mut stdout = io::stdout();
        if self.mouse_enabled {
            let _ = execute!(stdout, DisableMouseCapture);
        }
        if self.bracketed_paste_enabled {
            let _ = execute!(stdout, DisableBracketedPaste);
        }
        let _ = execute!(stdout, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::record_first_error;
    use std::io;

    fn err(kind: io::ErrorKind) -> io::Result<()> {
        Err(io::Error::new(kind, "boom"))
    }

    /// The restore path's correctness rests on this: it must run every cleanup
    /// step (above all `disable_raw_mode`) and still surface a failure, keeping
    /// the *first* one so the earliest, most relevant cause is what the caller
    /// sees. The `TerminalGuard` methods that use it need a real tty and so
    /// cannot be unit-tested; this pins the logic they delegate to.
    #[test]
    fn record_first_error_keeps_the_first_and_ignores_later_ones() {
        let mut slot = None;

        record_first_error(&mut slot, Ok(()));
        assert!(slot.is_none(), "a success must not set an error");

        record_first_error(&mut slot, err(io::ErrorKind::BrokenPipe));
        record_first_error(&mut slot, err(io::ErrorKind::Other));
        record_first_error(&mut slot, Ok(()));

        assert_eq!(
            slot.expect("an error was recorded").kind(),
            io::ErrorKind::BrokenPipe,
            "the first error must win, and later steps must still have run"
        );
    }
}
