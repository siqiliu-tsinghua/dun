use std::io::{self, Write};

use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

pub(crate) struct TerminalGuard {
    mouse_enabled: bool,
    bracketed_paste_enabled: bool,
    active: bool,
}

impl TerminalGuard {
    pub(crate) fn enter(mouse_enabled: bool) -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        if let Err(error) = execute!(stdout, EnableBracketedPaste) {
            let _ = execute!(stdout, LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(error);
        }
        if mouse_enabled {
            if let Err(error) = execute!(stdout, EnableMouseCapture) {
                let _ = execute!(stdout, DisableBracketedPaste);
                let _ = execute!(stdout, LeaveAlternateScreen);
                let _ = disable_raw_mode();
                return Err(error);
            }
        }
        Ok(Self {
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

        let mut stdout = io::stdout();
        if self.mouse_enabled {
            execute!(stdout, DisableMouseCapture)?;
        }
        if self.bracketed_paste_enabled {
            execute!(stdout, DisableBracketedPaste)?;
        }
        execute!(stdout, LeaveAlternateScreen)?;
        stdout.flush()?;
        disable_raw_mode()?;
        self.active = false;
        Ok(())
    }

    pub(crate) fn resume(&mut self, mouse_enabled: bool) -> io::Result<()> {
        if self.active {
            self.set_mouse_enabled(mouse_enabled)?;
            return Ok(());
        }

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        if let Err(error) = execute!(stdout, EnableBracketedPaste) {
            let _ = execute!(stdout, LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(error);
        }
        if mouse_enabled {
            if let Err(error) = execute!(stdout, EnableMouseCapture) {
                let _ = execute!(stdout, DisableBracketedPaste);
                let _ = execute!(stdout, LeaveAlternateScreen);
                let _ = disable_raw_mode();
                return Err(error);
            }
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
