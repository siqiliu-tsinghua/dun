#![forbid(unsafe_code)]

use std::env;
use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent,
    KeyModifiers as CrosstermKeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use dun_config::{Key, KeyModifiers, KeyStroke};
use dun_core::{AppCommand, BufferId, EditorCommand, Rect, TextBuffer, Workspace};
use dun_ui::{BufferView, UiShell};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

fn main() -> io::Result<()> {
    let mut app = AppState::new();
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run_event_loop(&mut terminal, &mut app);
    terminal.show_cursor()?;
    result
}

struct AppState {
    workspace: Workspace,
    buffer: TextBuffer,
    shell: UiShell,
    should_quit: bool,
}

impl AppState {
    fn new() -> Self {
        let config = dun_config::Config::default();
        let detected_profile = detect_terminal_profile();
        let shell = UiShell::from_config(&config, detected_profile);

        Self {
            workspace: Workspace::new_untitled(),
            buffer: TextBuffer::new_untitled(),
            shell,
            should_quit: false,
        }
    }

    fn handle_command(&mut self, command: &EditorCommand) {
        if matches!(command, EditorCommand::App(AppCommand::Quit)) {
            self.should_quit = true;
        }
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
    }
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut AppState,
) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| {
            let area = frame.area();
            let workspace_area = Rect::new(0, 0, area.width, area.height.saturating_sub(2));
            let buffer_view = BufferView::new(BufferId(1), &app.buffer);
            let ui_frame =
                app.shell
                    .frame_for_workspace(&app.workspace, workspace_area, &[buffer_view]);
            app.shell.render(frame, &ui_frame);
        })?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(event) => handle_key_event(app, event),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_key_event(app: &mut AppState, event: CrosstermKeyEvent) {
    let Some(stroke) = key_stroke_from_crossterm(event) else {
        return;
    };

    if let Some(command) = app.shell.command_for_stroke(stroke).cloned() {
        app.handle_command(&command);
    }
}

fn key_stroke_from_crossterm(event: CrosstermKeyEvent) -> Option<KeyStroke> {
    let modifiers = key_modifiers_from_crossterm(event.modifiers);
    let key = match event.code {
        CrosstermKeyCode::Backspace => Key::Backspace,
        CrosstermKeyCode::Enter => Key::Enter,
        CrosstermKeyCode::Left => Key::Left,
        CrosstermKeyCode::Right => Key::Right,
        CrosstermKeyCode::Up => Key::Up,
        CrosstermKeyCode::Down => Key::Down,
        CrosstermKeyCode::Home => Key::Home,
        CrosstermKeyCode::End => Key::End,
        CrosstermKeyCode::PageUp => Key::PageUp,
        CrosstermKeyCode::PageDown => Key::PageDown,
        CrosstermKeyCode::Tab => Key::Tab,
        CrosstermKeyCode::BackTab => Key::BackTab,
        CrosstermKeyCode::Delete => Key::Delete,
        CrosstermKeyCode::Insert => Key::Insert,
        CrosstermKeyCode::F(number) => Key::F(number),
        CrosstermKeyCode::Char(ch) => Key::Char(normalize_event_char(ch, modifiers)),
        CrosstermKeyCode::Esc => Key::Esc,
        _ => return None,
    };

    Some(KeyStroke::new(key, modifiers))
}

fn key_modifiers_from_crossterm(modifiers: CrosstermKeyModifiers) -> KeyModifiers {
    KeyModifiers {
        shift: modifiers.contains(CrosstermKeyModifiers::SHIFT),
        ctrl: modifiers.contains(CrosstermKeyModifiers::CONTROL),
        alt: modifiers.contains(CrosstermKeyModifiers::ALT),
    }
}

fn normalize_event_char(ch: char, modifiers: KeyModifiers) -> char {
    if ch.is_ascii_alphabetic() && modifiers.shift {
        ch.to_ascii_uppercase()
    } else if ch.is_ascii_alphabetic() {
        ch.to_ascii_lowercase()
    } else {
        ch
    }
}

fn detect_terminal_profile() -> dun_term::TerminalProfile {
    let term = env::var("TERM").ok();
    let colorterm = env::var("COLORTERM").ok();
    let lang = env::var("LANG").ok();
    let lc_ctype = env::var("LC_CTYPE").ok();
    let no_color = env::var_os("NO_COLOR").is_some();

    dun_term::TerminalProfile::from_capabilities(
        term.as_deref(),
        colorterm.as_deref(),
        lang.as_deref(),
        lc_ctype.as_deref(),
        no_color,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_ctrl_q_to_config_key_stroke() {
        let event =
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('q'), CrosstermKeyModifiers::CONTROL);

        assert_eq!(
            key_stroke_from_crossterm(event),
            Some(KeyStroke::new(Key::Char('q'), KeyModifiers::CTRL))
        );
    }

    #[test]
    fn translates_shifted_arrow_keys() {
        let event = CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::SHIFT);

        assert_eq!(
            key_stroke_from_crossterm(event),
            Some(KeyStroke::new(Key::Left, KeyModifiers::SHIFT))
        );
    }
}
