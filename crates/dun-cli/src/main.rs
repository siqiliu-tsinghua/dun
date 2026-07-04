#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};
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
    SearchMatch, TextBuffer, TextRange, WindowCommand, Workspace,
};
use dun_ui::{BufferView, UiShell};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Position as TuiPosition;
use unicode_width::UnicodeWidthStr;

fn main() -> io::Result<()> {
    let mut app = AppState::from_args(env::args_os().skip(1))?;
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
    status_message: Option<String>,
    prompt: Option<PromptState>,
    confirm: Option<ConfirmState>,
    last_find_query: Option<String>,
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
            status_message: None,
            prompt: None,
            confirm: None,
            last_find_query: None,
        }
    }

    fn from_args<I>(args: I) -> io::Result<Self>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut args = args.into_iter();
        let Some(path) = args.next() else {
            return Ok(Self::new());
        };

        if args.next().is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "dun currently accepts at most one file path",
            ));
        }

        let mut app = Self::new();
        app.open_file_path(PathBuf::from(path))?;
        Ok(app)
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
            if self.confirm_any_dirty(PendingAction::Quit) {
                return;
            }
            self.should_quit = true;
        }
    }

    fn handle_file_command(&mut self, command: &FileCommand) {
        match command {
            FileCommand::New => {
                if self.confirm_focused_dirty(PendingAction::New) {
                    return;
                }
                self.reset_focused_to_untitled();
            }
            FileCommand::Save => {
                if let Err(error) = self.save_focused_buffer() {
                    self.set_status(format!("Save failed: {error}"));
                }
            }
            FileCommand::Open => {
                if self.confirm_focused_dirty(PendingAction::OpenPrompt) {
                    return;
                }
                self.start_prompt(PromptKind::Open, String::new());
            }
            FileCommand::SaveAs => {
                self.start_prompt(PromptKind::SaveAs, self.focused_path_text());
            }
            FileCommand::Close => {
                self.handle_window_command(&WindowCommand::Close);
            }
        }
    }

    fn handle_edit_command(&mut self, command: &EditCommand) {
        match command {
            EditCommand::Find => {
                self.start_prompt(
                    PromptKind::Find,
                    self.last_find_query.clone().unwrap_or_default(),
                );
                return;
            }
            EditCommand::FindNext => {
                self.repeat_find(SearchDirection::Forward);
                return;
            }
            EditCommand::FindPrevious => {
                self.repeat_find(SearchDirection::Backward);
                return;
            }
            EditCommand::Replace => {
                self.set_status("Replace is not implemented yet");
                return;
            }
            _ => {}
        }

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
            | EditCommand::FindNext
            | EditCommand::FindPrevious
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
                if self.workspace.window_count() > 1
                    && self.confirm_focused_dirty(PendingAction::CloseWindow)
                {
                    return;
                }
                self.close_focused_window_unchecked();
            }
            WindowCommand::Only => {}
        }
    }

    fn handle_text_input(&mut self, ch: char) {
        if let Some(buffer) = self.focused_buffer_mut() {
            let _ = buffer.buffer.insert_char(ch);
        }
    }

    fn reset_focused_to_untitled(&mut self) {
        let Ok(window) = self.workspace.focused_window() else {
            return;
        };
        let window_id = window.id;
        let buffer_id = window.buffer_id;

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = BufferState::new(buffer_id, TextBuffer::new_untitled());
        }
        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = "Untitled".to_string();
            window.buffer_kind = dun_core::BufferKind::Untitled;
        }
        self.set_status("New untitled buffer");
    }

    fn open_file_path(&mut self, path: PathBuf) -> io::Result<()> {
        let buffer = load_text_buffer(&path)?;
        self.replace_focused_buffer_with_file(path, buffer);
        Ok(())
    }

    fn replace_focused_buffer_with_file(&mut self, path: PathBuf, buffer: TextBuffer) {
        let Ok(window) = self.workspace.focused_window() else {
            return;
        };
        let window_id = window.id;
        let buffer_id = window.buffer_id;
        let title = title_for_path(&path);
        let kind = buffer.kind();

        if let Some(state) = self.buffer_state_mut(buffer_id) {
            *state = BufferState::from_file(buffer_id, path.clone(), buffer);
        } else {
            self.buffers
                .push(BufferState::from_file(buffer_id, path.clone(), buffer));
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = title;
            window.buffer_kind = kind;
        }

        self.set_status(format!("Opened {}", path.display()));
    }

    fn save_focused_buffer(&mut self) -> io::Result<()> {
        let buffer_id = self
            .focused_buffer_id()
            .ok_or_else(|| io::Error::other("focused window is missing"))?;
        self.save_buffer(buffer_id).map(|_| ())
    }

    fn save_buffer(&mut self, buffer_id: BufferId) -> io::Result<PathBuf> {
        let (path, text) = {
            let buffer = self
                .buffer_state(buffer_id)
                .ok_or_else(|| io::Error::other("buffer is missing"))?;
            let path = buffer.path.clone().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer has no file path",
                )
            })?;
            (path, buffer.buffer.to_text())
        };

        fs::write(&path, text.as_bytes())?;

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            buffer.buffer.mark_saved();
        }
        self.set_status(format!("Saved {}", path.display()));
        Ok(path)
    }

    fn save_focused_buffer_as(&mut self, path: PathBuf) -> io::Result<()> {
        let window = self
            .workspace
            .focused_window()
            .map_err(|_| io::Error::other("focused window is missing"))?;
        let window_id = window.id;
        let buffer_id = window.buffer_id;
        let text = self
            .buffer_state(buffer_id)
            .map(|buffer| buffer.buffer.to_text())
            .ok_or_else(|| io::Error::other("focused buffer is missing"))?;

        fs::write(&path, text.as_bytes())?;

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            buffer.path = Some(path.clone());
            buffer.buffer.set_kind(dun_core::BufferKind::File);
            buffer.buffer.mark_saved();
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = title_for_path(&path);
            window.buffer_kind = dun_core::BufferKind::File;
        }

        self.set_status(format!("Saved {}", path.display()));
        Ok(())
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

    fn close_focused_window_unchecked(&mut self) {
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

    fn focused_buffer_mut(&mut self) -> Option<&mut BufferState> {
        let buffer_id = self.focused_buffer_id()?;
        self.buffer_state_mut(buffer_id)
    }

    fn focused_buffer_id(&self) -> Option<BufferId> {
        self.workspace
            .focused_window()
            .ok()
            .map(|window| window.buffer_id)
    }

    fn buffer_state(&self, id: BufferId) -> Option<&BufferState> {
        self.buffers.iter().find(|buffer| buffer.id == id)
    }

    fn buffer_state_mut(&mut self, id: BufferId) -> Option<&mut BufferState> {
        self.buffers.iter_mut().find(|buffer| buffer.id == id)
    }

    fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    fn start_prompt(&mut self, kind: PromptKind, initial_input: String) {
        self.start_prompt_after(kind, initial_input, None);
    }

    fn start_prompt_after(
        &mut self,
        kind: PromptKind,
        initial_input: String,
        after_success: Option<PendingAction>,
    ) {
        self.pending_keys.clear();
        self.status_message = None;
        self.confirm = None;
        self.prompt = Some(PromptState::new(kind, initial_input, after_success));
    }

    fn start_confirm(&mut self, action: PendingAction, buffer_id: BufferId) {
        self.pending_keys.clear();
        self.status_message = None;
        self.prompt = None;
        self.confirm = Some(ConfirmState { action, buffer_id });
    }

    fn confirm_focused_dirty(&mut self, action: PendingAction) -> bool {
        let Some(buffer_id) = self.focused_buffer_id() else {
            return false;
        };

        if self
            .buffer_state(buffer_id)
            .is_some_and(|buffer| buffer.buffer.is_dirty())
        {
            self.start_confirm(action, buffer_id);
            true
        } else {
            false
        }
    }

    fn confirm_any_dirty(&mut self, action: PendingAction) -> bool {
        let Some(buffer_id) = self
            .buffers
            .iter()
            .find(|buffer| buffer.buffer.is_dirty())
            .map(|buffer| buffer.id)
        else {
            return false;
        };

        self.focus_window_for_buffer(buffer_id);
        self.start_confirm(action, buffer_id);
        true
    }

    fn handle_confirm_key_event(&mut self, event: CrosstermKeyEvent) -> bool {
        if self.confirm.is_none() {
            return false;
        }

        match event.code {
            CrosstermKeyCode::Esc => self.cancel_confirm(),
            CrosstermKeyCode::Char(ch) => match ch.to_ascii_lowercase() {
                's' => self.save_confirmed_action(),
                'd' => self.discard_confirmed_action(),
                'c' => self.cancel_confirm(),
                _ => {}
            },
            _ => {}
        }

        true
    }

    fn cancel_confirm(&mut self) {
        self.confirm = None;
        self.set_status("Unsaved changes cancelled");
    }

    fn save_confirmed_action(&mut self) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };

        self.focus_window_for_buffer(confirm.buffer_id);
        if self
            .buffer_state(confirm.buffer_id)
            .and_then(|buffer| buffer.path.as_ref())
            .is_none()
        {
            self.start_prompt_after(
                PromptKind::SaveAs,
                self.path_text_for_buffer(confirm.buffer_id),
                Some(confirm.action),
            );
            return;
        }

        match self.save_buffer(confirm.buffer_id) {
            Ok(_) => self.continue_pending_action(confirm.action),
            Err(error) => self.set_status(format!("Save failed: {error}")),
        }
    }

    fn discard_confirmed_action(&mut self) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };

        match confirm.action {
            PendingAction::Quit => self.should_quit = true,
            action => {
                self.focus_window_for_buffer(confirm.buffer_id);
                self.continue_pending_action(action);
            }
        }
    }

    fn continue_pending_action(&mut self, action: PendingAction) {
        match action {
            PendingAction::Quit => {
                if !self.confirm_any_dirty(PendingAction::Quit) {
                    self.should_quit = true;
                }
            }
            PendingAction::New => self.reset_focused_to_untitled(),
            PendingAction::OpenPrompt => self.start_prompt(PromptKind::Open, String::new()),
            PendingAction::CloseWindow => self.close_focused_window_unchecked(),
        }
    }

    fn handle_prompt_key_event(&mut self, event: CrosstermKeyEvent) -> bool {
        if self.prompt.is_none() {
            return false;
        }

        match event.code {
            CrosstermKeyCode::Esc => {
                self.cancel_prompt();
            }
            CrosstermKeyCode::Enter => {
                self.submit_prompt();
            }
            CrosstermKeyCode::Backspace => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.input.pop();
                }
            }
            _ => {
                if let Some(ch) = text_input_from_crossterm(event) {
                    if let Some(prompt) = &mut self.prompt {
                        prompt.input.push(ch);
                    }
                }
            }
        }

        true
    }

    fn cancel_prompt(&mut self) {
        if let Some(prompt) = self.prompt.take() {
            self.set_status(format!("{} cancelled", prompt.kind.name()));
        }
    }

    fn submit_prompt(&mut self) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };

        let input = prompt.input.trim().to_string();
        if input.is_empty() {
            self.set_status(format!("{} cancelled", prompt.kind.name()));
            return;
        }

        match prompt.kind {
            PromptKind::Open => {
                if let Err(error) = self.open_file_path(PathBuf::from(&input)) {
                    self.set_status(format!("Open failed: {error}"));
                }
            }
            PromptKind::SaveAs => {
                if let Err(error) = self.save_focused_buffer_as(PathBuf::from(&input)) {
                    self.set_status(format!("Save As failed: {error}"));
                } else if let Some(action) = prompt.after_success {
                    self.continue_pending_action(action);
                }
            }
            PromptKind::Find => {
                self.last_find_query = Some(input.clone());
                self.find_in_focused_buffer(&input, SearchDirection::Forward);
            }
        }
    }

    fn repeat_find(&mut self, direction: SearchDirection) {
        let Some(query) = self.last_find_query.clone() else {
            self.set_status("Find: no query");
            return;
        };

        if query.is_empty() {
            self.set_status("Find: no query");
            return;
        }

        self.find_in_focused_buffer(&query, direction);
    }

    fn find_in_focused_buffer(&mut self, query: &str, direction: SearchDirection) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Find: focused buffer is missing");
            return;
        };

        let matches = buffer.buffer.find_all(query);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            self.set_status(format!("Find: no matches for {query}"));
            return;
        }

        let origin = match direction {
            SearchDirection::Forward => buffer
                .buffer
                .selection_range()
                .map(|range| range.end)
                .unwrap_or_else(|| buffer.buffer.cursor_position()),
            SearchDirection::Backward => buffer
                .buffer
                .selection_range()
                .map(|range| range.start)
                .unwrap_or_else(|| buffer.buffer.cursor_position()),
        };
        let selection = choose_search_match(&matches, origin, direction);
        let selected = matches[selection.index].range;
        let _ = buffer.buffer.select(selected.start, selected.end);

        let suffix = if selection.wrapped { " (wrapped)" } else { "" };
        self.set_status(format!(
            "Find: {}/{} {query}{suffix}",
            selection.index + 1,
            matches.len()
        ));
    }

    fn prompt_status_text(&self) -> Option<String> {
        self.prompt.as_ref().map(PromptState::status_text)
    }

    fn confirm_status_text(&self) -> Option<String> {
        self.confirm.as_ref().map(|confirm| {
            let action = match confirm.action {
                PendingAction::Quit => "Save(s) Quit without saving(d) Cancel(c)",
                PendingAction::New | PendingAction::OpenPrompt | PendingAction::CloseWindow => {
                    "Save(s) Discard(d) Cancel(c)"
                }
            };
            format!(
                "Unsaved changes in {}: {action}",
                self.buffer_display_name(confirm.buffer_id)
            )
        })
    }

    fn prompt_cursor_column(&self) -> Option<usize> {
        self.prompt_status_text()
            .map(|text| UnicodeWidthStr::width(text.as_str()))
    }

    fn focused_path_text(&self) -> String {
        let Some(buffer_id) = self.focused_buffer_id() else {
            return String::new();
        };

        self.path_text_for_buffer(buffer_id)
    }

    fn path_text_for_buffer(&self, buffer_id: BufferId) -> String {
        self.buffer_state(buffer_id)
            .and_then(|buffer| buffer.path.as_ref())
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn buffer_display_name(&self, buffer_id: BufferId) -> String {
        if let Some(name) = self
            .buffer_state(buffer_id)
            .and_then(|buffer| buffer.path.as_ref())
            .map(|path| title_for_path(path))
        {
            return name;
        }

        self.workspace
            .windows
            .iter()
            .find(|window| window.buffer_id == buffer_id)
            .map(|window| window.title.clone())
            .unwrap_or_else(|| format!("Buffer {}", buffer_id.0))
    }

    fn focus_window_for_buffer(&mut self, buffer_id: BufferId) -> bool {
        let Some(window) = self
            .workspace
            .windows
            .iter()
            .find(|window| window.buffer_id == buffer_id)
        else {
            return false;
        };

        self.workspace.focused = window.id;
        true
    }

    fn focused_buffer_status(&self) -> String {
        let Ok(window) = self.workspace.focused_window() else {
            return "No window".to_string();
        };

        let Some(buffer) = self.buffer_state(window.buffer_id) else {
            return window.title.clone();
        };

        let name = buffer
            .path
            .as_ref()
            .map(|path| title_for_path(path))
            .unwrap_or_else(|| window.title.clone());
        let dirty = if buffer.buffer.is_dirty() { "*" } else { "" };
        let read_only = if buffer.buffer.is_read_only() {
            " [readonly]"
        } else {
            ""
        };

        format!("{name}{dirty}{read_only}")
    }

    fn focused_position_status(&self) -> String {
        let Some(buffer_id) = self
            .workspace
            .focused_window()
            .ok()
            .map(|window| window.buffer_id)
        else {
            return "Ln -, Col -".to_string();
        };
        let Some(buffer) = self.buffer_state(buffer_id) else {
            return "Ln -, Col -".to_string();
        };

        let position = buffer.buffer.cursor_position();
        let column = buffer
            .buffer
            .line(position.line)
            .and_then(|line| line.get(..position.column))
            .map(|prefix| UnicodeWidthStr::width(prefix) + 1)
            .unwrap_or(1);

        format!("Ln {}, Col {}", position.line + 1, column)
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct PromptState {
    kind: PromptKind,
    input: String,
    after_success: Option<PendingAction>,
}

impl PromptState {
    fn new(kind: PromptKind, input: String, after_success: Option<PendingAction>) -> Self {
        Self {
            kind,
            input,
            after_success,
        }
    }

    fn status_text(&self) -> String {
        format!("{}{}", self.kind.label(), self.input)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromptKind {
    Open,
    SaveAs,
    Find,
}

impl PromptKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Open => "Open: ",
            Self::SaveAs => "Save As: ",
            Self::Find => "Find: ",
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::SaveAs => "Save As",
            Self::Find => "Find",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ConfirmState {
    action: PendingAction,
    buffer_id: BufferId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingAction {
    Quit,
    New,
    OpenPrompt,
    CloseWindow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SearchSelection {
    index: usize,
    wrapped: bool,
}

fn choose_search_match(
    matches: &[SearchMatch],
    origin: Position,
    direction: SearchDirection,
) -> SearchSelection {
    match direction {
        SearchDirection::Forward => matches
            .iter()
            .position(|item| item.range.start >= origin)
            .map(|index| SearchSelection {
                index,
                wrapped: false,
            })
            .unwrap_or(SearchSelection {
                index: 0,
                wrapped: true,
            }),
        SearchDirection::Backward => matches
            .iter()
            .rposition(|item| item.range.start < origin)
            .map(|index| SearchSelection {
                index,
                wrapped: false,
            })
            .unwrap_or(SearchSelection {
                index: matches.len().saturating_sub(1),
                wrapped: true,
            }),
    }
}

struct BufferState {
    id: BufferId,
    buffer: TextBuffer,
    path: Option<PathBuf>,
    first_line: usize,
}

impl BufferState {
    fn new(id: BufferId, buffer: TextBuffer) -> Self {
        Self {
            id,
            buffer,
            path: None,
            first_line: 0,
        }
    }

    fn from_file(id: BufferId, path: PathBuf, buffer: TextBuffer) -> Self {
        Self {
            id,
            buffer,
            path: Some(path),
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
            let mut ui_frame =
                app.shell
                    .frame_for_workspace(&app.workspace, workspace_area, &buffer_views);
            if let Some(confirm) = app.confirm_status_text() {
                ui_frame.status.left = confirm;
            } else if let Some(prompt) = app.prompt_status_text() {
                ui_frame.status.left = prompt;
            } else if let Some(message) = &app.status_message {
                ui_frame.status.left = message.clone();
            } else {
                ui_frame.status.left = app.focused_buffer_status();
            }
            ui_frame.status.right = app.focused_position_status();
            app.shell.render(frame, &ui_frame);
            if let Some(column) = app.prompt_cursor_column() {
                let area = frame.area();
                if area.width > 0 && area.height > 0 {
                    let x = column.min(area.width.saturating_sub(1) as usize) as u16;
                    frame.set_cursor_position(TuiPosition::new(
                        area.x + x,
                        area.y + area.height - 1,
                    ));
                }
            }
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

    if app.handle_confirm_key_event(event) {
        return;
    }

    if app.handle_prompt_key_event(event) {
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

fn load_text_buffer(path: &Path) -> io::Result<TextBuffer> {
    let bytes = fs::read(path)?;
    let text = String::from_utf8(bytes).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not valid UTF-8: {error}", path.display()),
        )
    })?;
    Ok(TextBuffer::from_text(&text))
}

fn title_for_path(path: &Path) -> String {
    path.file_name()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
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

    #[test]
    fn from_args_opens_utf8_file_path() {
        let path = temp_file_path("open.txt");
        std::fs::write(&path, "one\r\ntwo").unwrap();

        let app = AppState::from_args([path.clone().into_os_string()]).unwrap();
        let state = app.buffer_state(BufferId(1)).unwrap();

        assert_eq!(state.path.as_ref(), Some(&path));
        assert_eq!(state.buffer.line(0), Some("one"));
        assert_eq!(state.buffer.line(1), Some("two"));
        assert_eq!(state.buffer.line_ending(), dun_core::LineEnding::CrLf);
        assert!(!state.buffer.is_dirty());
        assert_eq!(
            app.workspace.focused_window().unwrap().title,
            title_for_path(&path)
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn invalid_utf8_file_path_is_rejected() {
        let path = temp_file_path("invalid.txt");
        std::fs::write(&path, [0xff, b'a']).unwrap();

        let error = match AppState::from_args([path.clone().into_os_string()]) {
            Ok(_) => panic!("invalid UTF-8 input should be rejected"),
            Err(error) => error,
        };

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_command_writes_focused_file_buffer() {
        let path = temp_file_path("save.txt");
        std::fs::write(&path, "old").unwrap();
        let mut app = AppState::from_args([path.clone().into_os_string()]).unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
        app.handle_text_input('!');
        app.handle_command(&EditorCommand::File(FileCommand::Save));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old!");
        assert!(!state.buffer.is_dirty());
        assert_eq!(
            app.status_message,
            Some(format!("Saved {}", path.display()))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_without_path_reports_status_message() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Save));

        assert_eq!(
            app.status_message,
            Some("Save failed: focused buffer has no file path".to_string())
        );
    }

    #[test]
    fn new_command_clears_loaded_file_metadata() {
        let path = temp_file_path("new.txt");
        std::fs::write(&path, "loaded").unwrap();
        let mut app = AppState::from_args([path.clone().into_os_string()]).unwrap();

        app.handle_command(&EditorCommand::File(FileCommand::New));

        let state = app.buffer_state(BufferId(1)).unwrap();
        let window = app.workspace.focused_window().unwrap();
        assert_eq!(state.path, None);
        assert_eq!(state.buffer.kind(), dun_core::BufferKind::Untitled);
        assert_eq!(state.buffer.to_text(), "");
        assert_eq!(window.title, "Untitled");
        assert_eq!(window.buffer_kind, dun_core::BufferKind::Untitled);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn open_command_uses_prompt_to_load_file() {
        let path = temp_file_path("prompt-open.txt");
        std::fs::write(&path, "opened").unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        assert_eq!(app.prompt_status_text(), Some("Open: ".to_string()));

        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "opened");
        assert_eq!(state.path.as_ref(), Some(&path));
        assert_eq!(
            app.status_message,
            Some(format!("Opened {}", path.display()))
        );
        assert_eq!(app.prompt_status_text(), None);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_as_prompt_writes_and_attaches_path() {
        let path = temp_file_path("prompt-save-as.txt");
        let mut app = AppState::new();
        app.handle_text_input('x');

        app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
        assert_eq!(app.prompt_status_text(), Some("Save As: ".to_string()));

        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        let window = app.workspace.focused_window().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
        assert_eq!(state.path.as_ref(), Some(&path));
        assert_eq!(state.buffer.kind(), dun_core::BufferKind::File);
        assert!(!state.buffer.is_dirty());
        assert_eq!(window.buffer_kind, dun_core::BufferKind::File);
        assert_eq!(window.title, title_for_path(&path));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn find_command_selects_first_match_from_prompt() {
        let mut app = app_with_text("one two one");

        app.handle_command(&EditorCommand::Edit(EditCommand::Find));
        assert_eq!(app.prompt_status_text(), Some("Find: ".to_string()));

        send_text(&mut app, "one");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(app.last_find_query, Some("one".to_string()));
        assert_eq!(app.status_message, Some("Find: 1/2 one".to_string()));
        assert_eq!(
            state.buffer.selection_range(),
            Some(TextRange::new(Position::new(0, 0), Position::new(0, 3)))
        );
    }

    #[test]
    fn find_next_repeats_query_and_wraps() {
        let mut app = app_with_text("one two one");
        app.last_find_query = Some("one".to_string());

        app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));
        app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(app.status_message, Some("Find: 2/2 one".to_string()));
        assert_eq!(
            state.buffer.selection_range(),
            Some(TextRange::new(Position::new(0, 8), Position::new(0, 11)))
        );

        app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(
            app.status_message,
            Some("Find: 1/2 one (wrapped)".to_string())
        );
        assert_eq!(
            state.buffer.selection_range(),
            Some(TextRange::new(Position::new(0, 0), Position::new(0, 3)))
        );
    }

    #[test]
    fn find_previous_repeats_query_and_wraps() {
        let mut app = app_with_text("one two one");
        app.last_find_query = Some("one".to_string());

        app.handle_command(&EditorCommand::Edit(EditCommand::FindPrevious));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(
            app.status_message,
            Some("Find: 2/2 one (wrapped)".to_string())
        );
        assert_eq!(
            state.buffer.selection_range(),
            Some(TextRange::new(Position::new(0, 8), Position::new(0, 11)))
        );
    }

    #[test]
    fn find_reports_missing_query_and_missing_match() {
        let mut app = app_with_text("abc");

        app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));
        assert_eq!(app.status_message, Some("Find: no query".to_string()));

        app.last_find_query = Some("z".to_string());
        app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(
            app.status_message,
            Some("Find: no matches for z".to_string())
        );
        assert_eq!(state.buffer.selection_range(), None);
    }

    #[test]
    fn focused_status_reports_dirty_buffer_name() {
        let mut app = AppState::new();

        app.handle_text_input('x');

        assert_eq!(app.focused_buffer_status(), "Untitled*");
    }

    #[test]
    fn focused_position_status_uses_display_column() {
        let mut app = app_with_text("a\n中x");
        app.buffers[0]
            .buffer
            .set_cursor(Position::new(1, "中".len()))
            .unwrap();

        assert_eq!(app.focused_position_status(), "Ln 2, Col 3");
    }

    #[test]
    fn prompt_cancel_restores_editor_input() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, "abc");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Esc, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('x'), CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(app.status_message, Some("Open cancelled".to_string()));
        assert_eq!(state.buffer.to_text(), "x");
    }

    #[test]
    fn prompt_backspace_edits_prompt_not_buffer() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::Edit(EditCommand::Find));
        send_text(&mut app, "abc");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Backspace, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(app.prompt_status_text(), Some("Find: ab".to_string()));
        assert_eq!(state.buffer.to_text(), "");
    }

    #[test]
    fn new_command_confirms_dirty_buffer_before_clearing() {
        let mut app = AppState::new();
        app.handle_text_input('x');

        app.handle_command(&EditorCommand::File(FileCommand::New));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "x");
        assert_eq!(
            app.confirm_status_text(),
            Some("Unsaved changes in Untitled: Save(s) Discard(d) Cancel(c)".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('c'), CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "x");
        assert_eq!(
            app.status_message,
            Some("Unsaved changes cancelled".to_string())
        );

        app.handle_command(&EditorCommand::File(FileCommand::New));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('d'), CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "");
        assert_eq!(app.confirm_status_text(), None);
    }

    #[test]
    fn open_command_can_discard_dirty_buffer_before_prompt() {
        let path = temp_file_path("confirm-open.txt");
        std::fs::write(&path, "opened").unwrap();
        let mut app = AppState::new();
        app.handle_text_input('x');

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        assert_eq!(
            app.confirm_status_text(),
            Some("Unsaved changes in Untitled: Save(s) Discard(d) Cancel(c)".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('d'), CrosstermKeyModifiers::NONE),
        );
        assert_eq!(app.prompt_status_text(), Some("Open: ".to_string()));

        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "opened");
        assert_eq!(state.path.as_ref(), Some(&path));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn quit_confirms_dirty_file_and_saves_before_exit() {
        let path = temp_file_path("confirm-quit-save.txt");
        std::fs::write(&path, "old").unwrap();
        let mut app = AppState::from_args([path.clone().into_os_string()]).unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
        app.handle_text_input('!');
        app.handle_command(&EditorCommand::App(AppCommand::Quit));

        assert!(!app.should_quit);
        assert_eq!(
            app.confirm_status_text(),
            Some(format!(
                "Unsaved changes in {}: Save(s) Quit without saving(d) Cancel(c)",
                title_for_path(&path)
            ))
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('s'), CrosstermKeyModifiers::NONE),
        );

        assert!(app.should_quit);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old!");
        assert!(!app.buffer_state(BufferId(1)).unwrap().buffer.is_dirty());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn quit_dirty_untitled_save_prompts_for_save_as_then_exits() {
        let path = temp_file_path("confirm-quit-save-as.txt");
        let mut app = AppState::new();
        app.handle_text_input('x');

        app.handle_command(&EditorCommand::App(AppCommand::Quit));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('s'), CrosstermKeyModifiers::NONE),
        );

        assert!(!app.should_quit);
        assert_eq!(app.prompt_status_text(), Some("Save As: ".to_string()));

        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert!(app.should_quit);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn close_dirty_window_can_be_cancelled() {
        let mut app = AppState::new();
        app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
        let focused = app.workspace.focused;
        app.handle_text_input('x');

        app.handle_command(&EditorCommand::Window(WindowCommand::Close));

        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(app.workspace.focused, focused);
        assert_eq!(
            app.confirm_status_text(),
            Some("Unsaved changes in Untitled-2: Save(s) Discard(d) Cancel(c)".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Esc, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(app.workspace.focused, focused);
        assert_eq!(
            app.status_message,
            Some("Unsaved changes cancelled".to_string())
        );
    }

    fn temp_file_path(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "dun-cli-test-{}-{unique}-{name}",
            std::process::id()
        ))
    }

    fn send_text(app: &mut AppState, text: &str) {
        for ch in text.chars() {
            handle_key_event(
                app,
                CrosstermKeyEvent::new(CrosstermKeyCode::Char(ch), CrosstermKeyModifiers::NONE),
            );
        }
    }

    fn app_with_text(text: &str) -> AppState {
        let mut app = AppState::new();
        app.buffers[0].buffer =
            TextBuffer::from_text_with_kind(dun_core::BufferKind::Untitled, text);
        app
    }
}
