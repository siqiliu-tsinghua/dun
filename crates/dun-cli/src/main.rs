#![forbid(unsafe_code)]

use std::env;
use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent,
    KeyEventKind as CrosstermKeyEventKind, KeyModifiers as CrosstermKeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use dun_config::{Key, KeyModifiers, KeySequence, KeyStroke};
use dun_core::{
    AppCommand, Axis, BufferId, Direction, EditCommand, EditorCommand, FileCommand, Position, Rect,
    TextBuffer, WindowCommand, Workspace,
};
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
    buffers: Vec<BufferState>,
    shell: UiShell,
    should_quit: bool,
    workspace_area: Rect,
    pending_keys: Vec<KeyStroke>,
}

impl AppState {
    fn new() -> Self {
        let config = dun_config::Config::default();
        let detected_profile = detect_terminal_profile();
        let shell = UiShell::from_config(&config, detected_profile);

        Self {
            workspace: Workspace::new_untitled(),
            buffers: vec![BufferState::new(BufferId(1), TextBuffer::new_untitled())],
            shell,
            should_quit: false,
            workspace_area: Rect::default(),
            pending_keys: Vec::new(),
        }
    }

    fn buffer_views(&self) -> Vec<BufferView<'_>> {
        self.buffers
            .iter()
            .map(|buffer| BufferView::scrolled(buffer.id, &buffer.buffer, buffer.first_line))
            .collect()
    }

    fn sync_view_for_area(&mut self, area: Rect) {
        self.workspace_area = area;
        let Some((buffer_id, body_height)) = self.focused_buffer_view_context(area) else {
            return;
        };
        let Some(buffer) = self.buffer_state_mut(buffer_id) else {
            return;
        };

        buffer.ensure_cursor_visible(body_height);
    }

    fn handle_command(&mut self, command: &EditorCommand) {
        match command {
            EditorCommand::App(command) => self.handle_app_command(command),
            EditorCommand::Edit(command) => self.handle_edit_command(command),
            EditorCommand::Window(command) => self.handle_window_command(command),
            EditorCommand::File(command) => self.handle_file_command(command),
        }
    }

    fn handle_key_stroke(&mut self, stroke: KeyStroke) -> bool {
        let had_pending_keys = !self.pending_keys.is_empty();
        self.pending_keys.push(stroke);
        let sequence = KeySequence {
            strokes: self.pending_keys.clone(),
        };

        if let Some(command) = self.shell.command_for_sequence(&sequence).cloned() {
            self.pending_keys.clear();
            self.handle_command(&command);
            return true;
        }

        if self.shell.keymap.has_sequence_prefix(&sequence) {
            return true;
        }

        self.pending_keys.clear();
        had_pending_keys
    }

    fn handle_app_command(&mut self, command: &AppCommand) {
        if matches!(command, AppCommand::Quit) {
            self.should_quit = true;
        }
    }

    fn handle_file_command(&mut self, command: &FileCommand) {
        if matches!(command, FileCommand::New) {
            if let Some(buffer) = self.focused_buffer_mut() {
                buffer.buffer = TextBuffer::new_untitled();
                buffer.first_line = 0;
            }
        }
    }

    fn handle_edit_command(&mut self, command: &EditCommand) {
        let Some(buffer) = self.focused_buffer_mut() else {
            return;
        };

        match command {
            EditCommand::Undo => {
                let _ = buffer.buffer.undo();
            }
            EditCommand::Redo => {
                let _ = buffer.buffer.redo();
            }
            EditCommand::SelectAll => {
                let end = buffer_end_position(&buffer.buffer);
                let _ = buffer.buffer.select(Position::zero(), end);
            }
            EditCommand::MoveLeft => {
                buffer.buffer.move_left();
            }
            EditCommand::MoveRight => {
                buffer.buffer.move_right();
            }
            EditCommand::MoveUp => {
                buffer.buffer.move_up();
            }
            EditCommand::MoveDown => {
                buffer.buffer.move_down();
            }
            EditCommand::MoveLineStart => {
                buffer.buffer.move_to_line_start();
            }
            EditCommand::MoveLineEnd => {
                buffer.buffer.move_to_line_end();
            }
            EditCommand::InsertNewline => {
                let _ = buffer.buffer.insert_newline();
            }
            EditCommand::DeleteBackward => {
                let _ = buffer.buffer.delete_backward();
            }
            EditCommand::DeleteForward => {
                let _ = buffer.buffer.delete_forward();
            }
            EditCommand::Cut
            | EditCommand::Copy
            | EditCommand::Paste
            | EditCommand::Find
            | EditCommand::Replace => {}
        }
    }

    fn handle_window_command(&mut self, command: &WindowCommand) {
        match command {
            WindowCommand::SplitHorizontal => self.split_focused(Axis::Horizontal),
            WindowCommand::SplitVertical => self.split_focused(Axis::Vertical),
            WindowCommand::FocusLeft => {
                let _ = self
                    .workspace
                    .focus_direction(Direction::Left, self.workspace_area);
            }
            WindowCommand::FocusRight => {
                let _ = self
                    .workspace
                    .focus_direction(Direction::Right, self.workspace_area);
            }
            WindowCommand::FocusUp => {
                let _ = self
                    .workspace
                    .focus_direction(Direction::Up, self.workspace_area);
            }
            WindowCommand::FocusDown => {
                let _ = self
                    .workspace
                    .focus_direction(Direction::Down, self.workspace_area);
            }
            WindowCommand::ResizeLeft => {
                let _ = self.workspace.resize_focused(Direction::Left);
            }
            WindowCommand::ResizeRight => {
                let _ = self.workspace.resize_focused(Direction::Right);
            }
            WindowCommand::ResizeUp => {
                let _ = self.workspace.resize_focused(Direction::Up);
            }
            WindowCommand::ResizeDown => {
                let _ = self.workspace.resize_focused(Direction::Down);
            }
            WindowCommand::Equalize => self.workspace.equalize(),
            WindowCommand::RotateSplit => {
                let _ = self.workspace.rotate_focused_split();
            }
            WindowCommand::Collapse => {
                let _ = self.workspace.collapse_focused();
            }
            WindowCommand::Expand => {
                let _ = self.workspace.expand_focused();
            }
            WindowCommand::ToggleCollapse => {
                let _ = self.workspace.toggle_focused_collapse();
            }
            WindowCommand::Close => {
                let closing_buffer_id = self
                    .workspace
                    .focused_window()
                    .ok()
                    .map(|window| window.buffer_id);
                if self.workspace.close_focused().is_ok() {
                    if let Some(buffer_id) = closing_buffer_id {
                        self.drop_buffer_if_unreferenced(buffer_id);
                    }
                }
            }
            WindowCommand::Only => {}
        }
    }

    fn handle_text_input(&mut self, ch: char) {
        if let Some(buffer) = self.focused_buffer_mut() {
            let _ = buffer.buffer.insert_char(ch);
        }
    }

    fn split_focused(&mut self, axis: Axis) {
        let Ok(window_id) = self.workspace.split_focused(axis) else {
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            return;
        };

        if self.buffer_state(window.buffer_id).is_none() {
            self.buffers.push(BufferState::new(
                window.buffer_id,
                TextBuffer::new_untitled(),
            ));
        }
    }

    fn focused_buffer_mut(&mut self) -> Option<&mut BufferState> {
        let buffer_id = self.workspace.focused_window().ok()?.buffer_id;
        self.buffer_state_mut(buffer_id)
    }

    fn buffer_state(&self, id: BufferId) -> Option<&BufferState> {
        self.buffers.iter().find(|buffer| buffer.id == id)
    }

    fn buffer_state_mut(&mut self, id: BufferId) -> Option<&mut BufferState> {
        self.buffers.iter_mut().find(|buffer| buffer.id == id)
    }

    fn drop_buffer_if_unreferenced(&mut self, id: BufferId) {
        if self
            .workspace
            .windows
            .iter()
            .any(|window| window.buffer_id == id)
        {
            return;
        }

        self.buffers.retain(|buffer| buffer.id != id);
    }

    fn focused_buffer_view_context(&self, area: Rect) -> Option<(BufferId, usize)> {
        let window = self.workspace.focused_window().ok()?;
        let layout = self
            .workspace
            .resolved_layout(area)
            .into_iter()
            .find(|layout| layout.id == window.id)?;
        let body_height = layout.rect.height.saturating_sub(2) as usize;
        Some((window.buffer_id, body_height))
    }
}

struct BufferState {
    id: BufferId,
    buffer: TextBuffer,
    first_line: usize,
}

impl BufferState {
    fn new(id: BufferId, buffer: TextBuffer) -> Self {
        Self {
            id,
            buffer,
            first_line: 0,
        }
    }

    fn ensure_cursor_visible(&mut self, body_height: usize) {
        if body_height == 0 {
            self.first_line = self.buffer.cursor_position().line;
            return;
        }

        let cursor_line = self.buffer.cursor_position().line;
        if cursor_line < self.first_line {
            self.first_line = cursor_line;
        } else if cursor_line >= self.first_line.saturating_add(body_height) {
            self.first_line = cursor_line.saturating_sub(body_height - 1);
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
            app.sync_view_for_area(workspace_area);
            let buffer_views = app.buffer_views();
            let ui_frame =
                app.shell
                    .frame_for_workspace(&app.workspace, workspace_area, &buffer_views);
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
    if matches!(event.kind, CrosstermKeyEventKind::Release) {
        return;
    }

    let Some(stroke) = key_stroke_from_crossterm(event) else {
        return;
    };

    if app.handle_key_stroke(stroke) {
        return;
    }

    if let Some(ch) = text_input_from_crossterm(event) {
        app.handle_text_input(ch);
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

fn text_input_from_crossterm(event: CrosstermKeyEvent) -> Option<char> {
    let modifiers = key_modifiers_from_crossterm(event.modifiers);
    if modifiers.ctrl || modifiers.alt {
        return None;
    }

    match event.code {
        CrosstermKeyCode::Char(ch) if !ch.is_control() => Some(ch),
        _ => None,
    }
}

fn buffer_end_position(buffer: &TextBuffer) -> Position {
    let last_line = buffer.line_count().saturating_sub(1);
    let last_column = buffer.line(last_line).map(str::len).unwrap_or(0);
    Position::new(last_line, last_column)
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

    #[test]
    fn text_input_inserts_into_focused_buffer() {
        let mut app = AppState::new();

        app.handle_text_input('a');
        app.handle_text_input('é');

        let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
        assert_eq!(buffer.line(0), Some("aé"));
        assert_eq!(buffer.cursor_position(), Position::new(0, 3));
    }

    #[test]
    fn edit_commands_apply_to_focused_buffer() {
        let mut app = AppState::new();
        app.handle_text_input('a');
        app.handle_text_input('b');
        app.handle_command(&EditorCommand::Edit(EditCommand::MoveLeft));
        app.handle_text_input('x');
        app.handle_command(&EditorCommand::Edit(EditCommand::DeleteForward));
        app.handle_command(&EditorCommand::Edit(EditCommand::InsertNewline));
        app.handle_text_input('z');

        let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
        assert_eq!(buffer.line(0), Some("ax"));
        assert_eq!(buffer.line(1), Some("z"));
        assert_eq!(buffer.cursor_position(), Position::new(1, 1));
    }

    #[test]
    fn window_command_creates_focused_buffer_for_split() {
        let mut app = AppState::new();
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));

        app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));

        let focused = app.workspace.focused_window().unwrap();
        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(app.buffers.len(), 2);
        assert!(app.buffer_state(focused.buffer_id).is_some());
    }

    #[test]
    fn window_close_drops_unreferenced_buffer() {
        let mut app = AppState::new();
        app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
        let closed_buffer_id = app.workspace.focused_window().unwrap().buffer_id;

        app.handle_command(&EditorCommand::Window(WindowCommand::Close));

        assert_eq!(app.workspace.window_count(), 1);
        assert_eq!(app.buffers.len(), 1);
        assert!(app.buffer_state(closed_buffer_id).is_none());
    }

    #[test]
    fn crossterm_text_input_ignores_control_shortcuts() {
        let plain =
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('x'), CrosstermKeyModifiers::NONE);
        let control =
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('x'), CrosstermKeyModifiers::CONTROL);

        assert_eq!(text_input_from_crossterm(plain), Some('x'));
        assert_eq!(text_input_from_crossterm(control), None);
    }

    #[test]
    fn multi_stroke_key_sequence_applies_command() {
        let mut app = AppState::new();
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('w'), CrosstermKeyModifiers::CONTROL),
        );

        assert_eq!(app.workspace.window_count(), 1);

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('h'), CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.workspace.window_count(), 2);
    }

    #[test]
    fn invalid_pending_key_sequence_does_not_insert_text() {
        let mut app = AppState::new();

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('w'), CrosstermKeyModifiers::CONTROL),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('m'), CrosstermKeyModifiers::NONE),
        );

        let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
        assert_eq!(buffer.line(0), Some(""));
    }
}
