#![forbid(unsafe_code)]

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Read, Stdout, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode as CrosstermKeyCode,
    KeyEvent as CrosstermKeyEvent, KeyEventKind as CrosstermKeyEventKind,
    KeyModifiers as CrosstermKeyModifiers, MouseButton as CrosstermMouseButton,
    MouseEvent as CrosstermMouseEvent, MouseEventKind as CrosstermMouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use dun_config::{
    Config, Key, KeyModifiers, KeySequence, KeyStroke, Keymap, Limits, ThemeName, command_from_id,
    command_id, parse_config,
};
use dun_core::{
    AppCommand, Axis, BufferError, BufferId, BufferKind, Direction, EditCommand, EditorCommand,
    FileCommand, FileTextEncoding, LineEnding, Position, Rect, SearchMatch, TextBuffer,
    WindowCommand, WindowId, WindowKind, Workspace, WorkspaceError, decode_file_text,
};
use dun_term::{ColorProfile, EncodingProfile, TerminalProfile, Theme};
use dun_ui::{BufferView, UiMouseTarget, UiShell};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Position as TuiPosition;
use unicode_width::UnicodeWidthStr;

const EXIT_RUNTIME_ERROR: u8 = 1;
const EXIT_USAGE_ERROR: u8 = 2;
const DUN_CONFIG_ENV: &str = "DUN_CONFIG";

fn main() -> ExitCode {
    match run_cli(env::args_os().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn run_cli<I>(args: I) -> Result<(), CliError>
where
    I: IntoIterator<Item = OsString>,
{
    match parse_cli_args(args)? {
        CliAction::Help => {
            print!("{}", cli_help_text());
            Ok(())
        }
        CliAction::Version => {
            println!("{}", cli_version_text());
            Ok(())
        }
        CliAction::Run {
            config_path,
            no_config,
            path,
        } => run_tui(config_path, no_config, path).map_err(CliError::Io),
    }
}

fn run_tui(config_path: Option<PathBuf>, no_config: bool, path: Option<PathBuf>) -> io::Result<()> {
    let config_request = ConfigLoadRequest::new(config_path, no_config);
    let loaded_config = load_config(&config_request)?;
    let mut app = AppState::from_loaded_config_path(config_request, loaded_config, path)?;
    let mut guard = TerminalGuard::enter(app.mouse_enabled())?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run_event_loop(&mut terminal, &mut app, &mut guard);
    terminal.show_cursor()?;
    result
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CliAction {
    Help,
    Version,
    Run {
        config_path: Option<PathBuf>,
        no_config: bool,
        path: Option<PathBuf>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct UsageError {
    message: String,
}

impl UsageError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for UsageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

#[derive(Debug)]
enum CliError {
    Usage(UsageError),
    Io(io::Error),
}

impl CliError {
    const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => EXIT_USAGE_ERROR,
            Self::Io(_) => EXIT_RUNTIME_ERROR,
        }
    }
}

impl From<UsageError> for CliError {
    fn from(error: UsageError) -> Self {
        Self::Usage(error)
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(error) => write!(formatter, "dun: {error}\n\n{}", cli_usage_text()),
            Self::Io(error) => write!(formatter, "dun: {error}"),
        }
    }
}

fn parse_cli_args<I>(args: I) -> Result<CliAction, UsageError>
where
    I: IntoIterator,
    I::Item: Into<OsString>,
{
    let mut action = None;
    let mut paths = Vec::new();
    let mut config_path = None;
    let mut no_config = false;
    let mut parse_options = true;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        let arg = arg.into();
        if parse_options && arg == "--" {
            parse_options = false;
            continue;
        }

        if parse_options {
            match arg.to_string_lossy().as_ref() {
                "-h" | "--help" => {
                    set_cli_action(&mut action, CliAction::Help)?;
                    continue;
                }
                "-V" | "--version" => {
                    set_cli_action(&mut action, CliAction::Version)?;
                    continue;
                }
                "--no-config" => {
                    if config_path.is_some() {
                        return Err(UsageError::new(
                            "--config and --no-config cannot be used together",
                        ));
                    }
                    no_config = true;
                    continue;
                }
                "--config" => {
                    if no_config {
                        return Err(UsageError::new(
                            "--config and --no-config cannot be used together",
                        ));
                    }
                    if config_path.is_some() {
                        return Err(UsageError::new("--config may only be used once"));
                    }
                    let Some(path) = args.next() else {
                        return Err(UsageError::new("missing path after --config"));
                    };
                    config_path = Some(PathBuf::from(path.into()));
                    continue;
                }
                option if option.starts_with("--config=") => {
                    if no_config {
                        return Err(UsageError::new(
                            "--config and --no-config cannot be used together",
                        ));
                    }
                    if config_path.is_some() {
                        return Err(UsageError::new("--config may only be used once"));
                    }
                    let path = option.trim_start_matches("--config=");
                    if path.is_empty() {
                        return Err(UsageError::new("missing path after --config"));
                    }
                    config_path = Some(PathBuf::from(path));
                    continue;
                }
                option if option.starts_with('-') && option != "-" => {
                    return Err(UsageError::new(format!("unknown option {option}")));
                }
                _ => {}
            }
        }

        paths.push(PathBuf::from(arg));
    }

    if let Some(action) = action {
        if paths.is_empty() {
            return Ok(action);
        }
        return Err(UsageError::new(
            "options --help and --version cannot be combined with paths",
        ));
    }

    match paths.len() {
        0 => Ok(CliAction::Run {
            config_path,
            no_config,
            path: None,
        }),
        1 => Ok(CliAction::Run {
            config_path,
            no_config,
            path: paths.into_iter().next(),
        }),
        count => Err(UsageError::new(format!(
            "expected at most one path, got {count}"
        ))),
    }
}

fn set_cli_action(action: &mut Option<CliAction>, new_action: CliAction) -> Result<(), UsageError> {
    if action.is_some() {
        return Err(UsageError::new(
            "only one of --help or --version may be used",
        ));
    }

    *action = Some(new_action);
    Ok(())
}

fn cli_version_text() -> String {
    format!("dun {}", env!("CARGO_PKG_VERSION"))
}

fn cli_usage_text() -> &'static str {
    "Usage: dun [OPTIONS] [--] [PATH]\nTry 'dun --help' for more information."
}

fn cli_help_text() -> &'static str {
    "\
dun - Microsoft Edit-like terminal editor

Usage:
  dun [OPTIONS] [--] [PATH]

Arguments:
  PATH              Open one UTF-8 text file at startup.

Options:
  -h, --help        Show this help text and exit.
  -V, --version     Show version information and exit.
      --config PATH Load configuration from PATH.
      --no-config   Ignore DUN_CONFIG and default config paths.

Exit codes:
  0                 Success, --help, or --version.
  1                 Runtime, terminal, or file I/O error.
  2                 Command-line usage error.
"
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfigLoadRequest {
    explicit_path: Option<PathBuf>,
    no_config: bool,
}

impl ConfigLoadRequest {
    const fn new(explicit_path: Option<PathBuf>, no_config: bool) -> Self {
        Self {
            explicit_path,
            no_config,
        }
    }

    #[cfg(test)]
    fn explicit(path: PathBuf) -> Self {
        Self::new(Some(path), false)
    }

    fn diagnostics_text(&self) -> String {
        if self.no_config {
            return "--no-config".to_string();
        }

        match &self.explicit_path {
            Some(path) => format!("--config {}", path.display()),
            None => "discovery (DUN_CONFIG, XDG_CONFIG_HOME, HOME)".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LoadedConfig {
    config: Config,
    source: ConfigSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ConfigSource {
    Disabled,
    Explicit(PathBuf),
    Environment(PathBuf),
    DefaultFile(PathBuf),
    BuiltInDefaults,
}

impl ConfigSource {
    fn status_text(&self) -> String {
        match self {
            Self::Disabled => "Config reloaded from built-in defaults (--no-config)".to_string(),
            Self::Explicit(path) => format!("Config reloaded from {}", path.display()),
            Self::Environment(path) => {
                format!("Config reloaded from {DUN_CONFIG_ENV}={}", path.display())
            }
            Self::DefaultFile(path) => format!("Config reloaded from {}", path.display()),
            Self::BuiltInDefaults => "Config reloaded from built-in defaults".to_string(),
        }
    }

    fn diagnostics_text(&self) -> String {
        match self {
            Self::Disabled => "disabled (--no-config)".to_string(),
            Self::Explicit(path) => format!("explicit file ({})", path.display()),
            Self::Environment(path) => format!("{DUN_CONFIG_ENV} ({})", path.display()),
            Self::DefaultFile(path) => format!("default file ({})", path.display()),
            Self::BuiltInDefaults => "built-in defaults".to_string(),
        }
    }
}

#[cfg(test)]
fn load_startup_config(explicit_path: Option<&Path>, no_config: bool) -> io::Result<Config> {
    let request = ConfigLoadRequest::new(explicit_path.map(Path::to_path_buf), no_config);
    load_config(&request).map(|loaded| loaded.config)
}

fn load_config(request: &ConfigLoadRequest) -> io::Result<LoadedConfig> {
    if request.no_config {
        return Ok(LoadedConfig {
            config: Config::default(),
            source: ConfigSource::Disabled,
        });
    }

    if let Some(path) = &request.explicit_path {
        return Ok(LoadedConfig {
            config: read_config_file(path)?,
            source: ConfigSource::Explicit(path.clone()),
        });
    }

    if let Some(path) = env_config_path() {
        return Ok(LoadedConfig {
            config: read_config_file(&path)?,
            source: ConfigSource::Environment(path),
        });
    }

    if let Some(path) = default_config_path() {
        if path.exists() {
            return Ok(LoadedConfig {
                config: read_config_file(&path)?,
                source: ConfigSource::DefaultFile(path),
            });
        }
    }

    Ok(LoadedConfig {
        config: Config::default(),
        source: ConfigSource::BuiltInDefaults,
    })
}

fn read_config_file(path: &Path) -> io::Result<Config> {
    let text = fs::read_to_string(path)
        .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))?;
    parse_config(&text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {error}", path.display()),
        )
    })
}

fn env_config_path() -> Option<PathBuf> {
    env::var_os(DUN_CONFIG_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn default_config_path() -> Option<PathBuf> {
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(config_home).join("dun").join("config"));
    }

    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("dun")
                .join("config")
        })
}

struct AppState {
    workspace: Workspace,
    buffers: Vec<BufferState>,
    config_request: ConfigLoadRequest,
    config_source: ConfigSource,
    detected_profile: TerminalProfile,
    shell: UiShell,
    limits: Limits,
    mouse_enabled: bool,
    should_quit: bool,
    workspace_area: Rect,
    pending_keys: Vec<KeyStroke>,
    status_message: Option<String>,
    prompt: Option<PromptState>,
    confirm: Option<ConfirmState>,
    status_history: Vec<StatusEntry>,
    command_history: Vec<String>,
    last_find_query: Option<String>,
    pending_replace_query: Option<String>,
}

impl AppState {
    #[cfg(test)]
    fn new() -> Self {
        Self::from_config(Config::default())
    }

    #[cfg(test)]
    fn from_config(config: Config) -> Self {
        Self::from_loaded_config(
            ConfigLoadRequest::new(None, true),
            LoadedConfig {
                config,
                source: ConfigSource::Disabled,
            },
        )
    }

    fn from_loaded_config(config_request: ConfigLoadRequest, loaded_config: LoadedConfig) -> Self {
        let detected_profile = detect_terminal_profile();
        let shell = UiShell::from_config(&loaded_config.config, detected_profile);
        let limits = loaded_config.config.limits;
        let mouse_enabled = loaded_config.config.mouse.enabled;

        Self {
            workspace: Workspace::new_untitled(),
            buffers: vec![BufferState::new(BufferId(1), TextBuffer::new_untitled())],
            config_request,
            config_source: loaded_config.source,
            detected_profile,
            shell,
            limits,
            mouse_enabled,
            should_quit: false,
            workspace_area: Rect::default(),
            pending_keys: Vec::new(),
            status_message: None,
            prompt: None,
            confirm: None,
            status_history: Vec::new(),
            command_history: Vec::new(),
            last_find_query: None,
            pending_replace_query: None,
        }
    }

    #[cfg(test)]
    fn from_path(path: Option<PathBuf>) -> io::Result<Self> {
        let mut app = Self::new();
        if let Some(path) = path {
            app.open_file_path(path)?;
        }
        Ok(app)
    }

    #[cfg(test)]
    fn from_config_path(config: Config, path: Option<PathBuf>) -> io::Result<Self> {
        let mut app = Self::from_config(config);
        if let Some(path) = path {
            app.open_file_path(path)?;
        }
        Ok(app)
    }

    fn from_loaded_config_path(
        config_request: ConfigLoadRequest,
        loaded_config: LoadedConfig,
        path: Option<PathBuf>,
    ) -> io::Result<Self> {
        let mut app = Self::from_loaded_config(config_request, loaded_config);
        if let Some(path) = path {
            app.open_file_path(path)?;
        }
        Ok(app)
    }

    fn buffer_views(&self) -> Vec<BufferView<'_>> {
        self.buffers
            .iter()
            .map(|buffer| BufferView::scrolled(buffer.id, &buffer.buffer, buffer.first_line))
            .collect()
    }

    const fn mouse_enabled(&self) -> bool {
        self.mouse_enabled
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

    fn handle_left_click(&mut self, screen_x: u16, screen_y: u16) -> bool {
        let Some((x, y)) = self.workspace_point_from_screen(screen_x, screen_y) else {
            return false;
        };
        let hit = {
            let buffer_views = self.buffer_views();
            self.shell
                .hit_test_workspace(&self.workspace, self.workspace_area, &buffer_views, x, y)
        };
        let Some(hit) = hit else {
            return false;
        };

        if self.workspace.focus_at(self.workspace_area, x, y).is_none() {
            return false;
        }

        self.pending_keys.clear();
        if let UiMouseTarget::Body(position) = hit.target {
            if let Some(buffer) = self.buffer_state_mut(hit.buffer_id) {
                let _ = buffer.buffer.set_cursor(position);
            }
            self.sync_view_for_area(self.workspace_area);
        }

        true
    }

    fn workspace_point_from_screen(&self, column: u16, row: u16) -> Option<(u16, u16)> {
        if self.workspace_area.width == 0 || self.workspace_area.height == 0 || row == 0 {
            return None;
        }

        let y = row - 1;
        if column >= self.workspace_area.width || y >= self.workspace_area.height {
            return None;
        }

        Some((column, y))
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
        match command {
            AppCommand::ConfigDiagnostics => self.open_config_diagnostics_screen(),
            AppCommand::Help => self.open_help_screen(),
            AppCommand::ReloadConfig => self.reload_config(),
            AppCommand::StatusHistory => self.open_status_history_screen(),
            AppCommand::Quit => {
                if self.confirm_any_dirty(PendingAction::Quit) {
                    return;
                }
                self.should_quit = true;
            }
            AppCommand::CommandLine => {
                self.start_prompt(PromptKind::CommandLine, String::new());
            }
        }
    }

    fn reload_config(&mut self) {
        match load_config(&self.config_request) {
            Ok(loaded_config) => {
                let status = loaded_config.source.status_text();
                self.apply_loaded_config(loaded_config);
                self.set_status(status);
            }
            Err(error) => self.set_status(format!("Config reload failed: {error}")),
        }
    }

    fn apply_loaded_config(&mut self, loaded_config: LoadedConfig) {
        self.pending_keys.clear();
        self.shell = UiShell::from_config(&loaded_config.config, self.detected_profile);
        self.limits = loaded_config.config.limits;
        self.mouse_enabled = loaded_config.config.mouse.enabled;
        self.config_source = loaded_config.source;
        self.refresh_help_buffer();
        self.refresh_config_diagnostics_buffer();
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
                self.start_prompt(
                    PromptKind::ReplaceFind,
                    self.last_find_query.clone().unwrap_or_default(),
                );
                return;
            }
            EditCommand::GoToLine => {
                self.start_prompt(PromptKind::GoToLine, String::new());
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
            | EditCommand::Replace
            | EditCommand::GoToLine => {}
        }
    }

    fn handle_window_command(&mut self, command: &WindowCommand) {
        match command {
            WindowCommand::SplitHorizontal => {
                self.split_focused(Axis::Horizontal, "Split horizontally")
            }
            WindowCommand::SplitVertical => self.split_focused(Axis::Vertical, "Split vertically"),
            WindowCommand::FocusLeft => self.focus_window_direction(Direction::Left, "left"),
            WindowCommand::FocusRight => self.focus_window_direction(Direction::Right, "right"),
            WindowCommand::FocusUp => self.focus_window_direction(Direction::Up, "up"),
            WindowCommand::FocusDown => self.focus_window_direction(Direction::Down, "down"),
            WindowCommand::ResizeLeft => self.resize_window_direction(Direction::Left, "left"),
            WindowCommand::ResizeRight => self.resize_window_direction(Direction::Right, "right"),
            WindowCommand::ResizeUp => self.resize_window_direction(Direction::Up, "up"),
            WindowCommand::ResizeDown => self.resize_window_direction(Direction::Down, "down"),
            WindowCommand::Equalize => {
                self.workspace.equalize();
                self.set_status("Equalized splits");
            }
            WindowCommand::RotateSplit => match self.workspace.rotate_focused_split() {
                Ok(axis) => {
                    self.set_status(format!("Rotated focused split to {}", axis_name(axis)))
                }
                Err(error) => self.set_status(format!(
                    "Rotate split failed: {}",
                    workspace_error_text(error)
                )),
            },
            WindowCommand::Collapse => match self.workspace.collapse_focused() {
                Ok(()) => self.set_status("Collapsed pane"),
                Err(error) => {
                    self.set_status(format!("Collapse failed: {}", workspace_error_text(error)))
                }
            },
            WindowCommand::Expand => match self.workspace.expand_focused() {
                Ok(()) => self.set_status("Expanded pane"),
                Err(error) => {
                    self.set_status(format!("Expand failed: {}", workspace_error_text(error)))
                }
            },
            WindowCommand::ToggleCollapse => match self.workspace.toggle_focused_collapse() {
                Ok(true) => self.set_status("Collapsed pane"),
                Ok(false) => self.set_status("Expanded pane"),
                Err(error) => self.set_status(format!(
                    "Toggle collapse failed: {}",
                    workspace_error_text(error)
                )),
            },
            WindowCommand::Close => {
                if self.workspace.window_count() > 1
                    && self.confirm_focused_dirty(PendingAction::CloseWindow)
                {
                    return;
                }
                self.close_focused_window_unchecked();
            }
            WindowCommand::Only => self.set_status("Only window is not implemented yet"),
        }
    }

    fn focus_window_direction(&mut self, direction: Direction, label: &str) {
        match self
            .workspace
            .focus_direction(direction, self.workspace_area)
        {
            Ok(_) => self.set_status(format!("Focused {label}")),
            Err(error) => self.set_status(format!(
                "Focus {label} failed: {}",
                workspace_error_text(error)
            )),
        }
    }

    fn resize_window_direction(&mut self, direction: Direction, label: &str) {
        match self.workspace.resize_focused(direction) {
            Ok(_) => self.set_status(format!("Resized {label}")),
            Err(error) => self.set_status(format!(
                "Resize {label} failed: {}",
                workspace_error_text(error)
            )),
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
        let loaded =
            load_text_buffer(&path, self.limits).map_err(|error| path_io_error(&path, error))?;
        let temp_report = reconcile_atomic_save_temp_files(&path);
        self.replace_focused_buffer_with_file(path, loaded, temp_report);
        Ok(())
    }

    fn replace_focused_buffer_with_file(
        &mut self,
        path: PathBuf,
        loaded: LoadedTextBuffer,
        temp_report: AtomicTempReconcileReport,
    ) {
        let Ok(window) = self.workspace.focused_window() else {
            return;
        };
        let window_id = window.id;
        let buffer_id = window.buffer_id;
        let title = title_for_path(&path);
        let kind = loaded.buffer.kind();
        let encoding = loaded.encoding;

        if let Some(state) = self.buffer_state_mut(buffer_id) {
            *state = BufferState::from_file(buffer_id, path.clone(), loaded);
        } else {
            self.buffers
                .push(BufferState::from_file(buffer_id, path.clone(), loaded));
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = title;
            window.buffer_kind = kind;
        }

        let status = opened_file_status(&path, encoding);
        self.set_status(status_with_atomic_temp_report(status, &temp_report));
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
            if buffer.buffer.is_read_only() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer is read-only",
                ));
            }
            if !buffer.encoding.is_save_safe() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer encoding is not save-safe",
                ));
            }
            let path = buffer.path.clone().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer has no file path",
                )
            })?;
            (path, buffer.buffer.to_text())
        };

        let report =
            atomic_write_text_file(&path, &text).map_err(|error| path_io_error(&path, error))?;

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            buffer.buffer.mark_saved();
        }
        self.set_status(status_with_atomic_temp_report(
            format!("Saved {}", path.display()),
            &report.temp_reconcile,
        ));
        Ok(path)
    }

    fn save_focused_buffer_as(&mut self, path: PathBuf) -> io::Result<()> {
        let window = self
            .workspace
            .focused_window()
            .map_err(|_| io::Error::other("focused window is missing"))?;
        let window_id = window.id;
        let buffer_id = window.buffer_id;
        let text = {
            let buffer = self
                .buffer_state(buffer_id)
                .ok_or_else(|| io::Error::other("focused buffer is missing"))?;
            if buffer.buffer.is_read_only() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer is read-only",
                ));
            }
            if !buffer.encoding.is_save_safe() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer encoding is not save-safe",
                ));
            }
            buffer.buffer.to_text()
        };

        let report =
            atomic_write_text_file(&path, &text).map_err(|error| path_io_error(&path, error))?;

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            buffer.path = Some(path.clone());
            buffer.encoding = FileTextEncoding::Utf8;
            buffer.buffer.set_kind(dun_core::BufferKind::File);
            buffer.buffer.mark_saved();
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = title_for_path(&path);
            window.buffer_kind = dun_core::BufferKind::File;
        }

        self.set_status(status_with_atomic_temp_report(
            format!("Saved {}", path.display()),
            &report.temp_reconcile,
        ));
        Ok(())
    }

    fn split_focused(&mut self, axis: Axis, success_status: &'static str) {
        let window_id = match self.workspace.split_focused(axis) {
            Ok(window_id) => window_id,
            Err(error) => {
                self.set_status(format!("Split failed: {}", workspace_error_text(error)));
                return;
            }
        };
        let window = match self.workspace.window(window_id) {
            Ok(window) => window,
            Err(error) => {
                self.set_status(format!("Split failed: {}", workspace_error_text(error)));
                return;
            }
        };

        if self.buffer_state(window.buffer_id).is_none() {
            self.buffers.push(BufferState::new(
                window.buffer_id,
                TextBuffer::new_untitled(),
            ));
        }

        self.set_status(success_status);
    }

    fn open_help_screen(&mut self) {
        if let Some(window_id) = self.help_window_id() {
            self.workspace.focused = window_id;
            self.set_status("Help");
            return;
        }

        let Ok(window_id) = self.workspace.split_focused(Axis::Horizontal) else {
            self.set_status("Help failed: focused window is missing");
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            self.set_status("Help failed: help window is missing");
            return;
        };
        let buffer_id = window.buffer_id;
        let help = BufferState::new(buffer_id, help_buffer(&self.shell.keymap));

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = help;
        } else {
            self.buffers.push(help);
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = "Help".to_string();
            window.kind = WindowKind::Help;
            window.buffer_kind = BufferKind::ReadOnly;
            window.collapsed = false;
        }

        self.set_status("Help");
    }

    fn help_window_id(&self) -> Option<WindowId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::Help)
            .map(|window| window.id)
    }

    fn help_buffer_id(&self) -> Option<BufferId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::Help)
            .map(|window| window.buffer_id)
    }

    fn refresh_help_buffer(&mut self) {
        let Some(buffer_id) = self.help_buffer_id() else {
            return;
        };
        let help = BufferState::new(buffer_id, help_buffer(&self.shell.keymap));

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = help;
        }
    }

    fn open_config_diagnostics_screen(&mut self) {
        self.set_status("Config diagnostics");

        if let Some(window_id) = self.config_diagnostics_window_id() {
            self.workspace.focused = window_id;
            self.refresh_config_diagnostics_buffer();
            return;
        }

        let Ok(window_id) = self.workspace.split_focused(Axis::Horizontal) else {
            self.set_status("Config diagnostics failed: focused window is missing");
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            self.set_status("Config diagnostics failed: diagnostics window is missing");
            return;
        };
        let buffer_id = window.buffer_id;
        let text = self.config_diagnostics_text();
        let diagnostics = BufferState::new(buffer_id, config_diagnostics_buffer(&text));

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = diagnostics;
        } else {
            self.buffers.push(diagnostics);
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = "Config Diagnostics".to_string();
            window.kind = WindowKind::ConfigDiagnostics;
            window.buffer_kind = BufferKind::ReadOnly;
            window.collapsed = false;
        }
    }

    fn config_diagnostics_window_id(&self) -> Option<WindowId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::ConfigDiagnostics)
            .map(|window| window.id)
    }

    fn config_diagnostics_buffer_id(&self) -> Option<BufferId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::ConfigDiagnostics)
            .map(|window| window.buffer_id)
    }

    fn refresh_config_diagnostics_buffer(&mut self) {
        let Some(buffer_id) = self.config_diagnostics_buffer_id() else {
            return;
        };
        let text = self.config_diagnostics_text();
        let diagnostics = BufferState::new(buffer_id, config_diagnostics_buffer(&text));

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = diagnostics;
        }
    }

    fn open_status_history_screen(&mut self) {
        self.set_status("Status history");

        if let Some(window_id) = self.status_history_window_id() {
            self.workspace.focused = window_id;
            return;
        }

        let Ok(window_id) = self.workspace.split_focused(Axis::Horizontal) else {
            self.set_status("Status history failed: focused window is missing");
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            self.set_status("Status history failed: status window is missing");
            return;
        };
        let buffer_id = window.buffer_id;
        let text = self.status_history_text();

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = BufferState::new(buffer_id, status_history_buffer(&text));
        } else {
            self.buffers
                .push(BufferState::new(buffer_id, status_history_buffer(&text)));
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = "Status History".to_string();
            window.kind = WindowKind::StatusHistory;
            window.buffer_kind = BufferKind::ReadOnly;
            window.collapsed = false;
        }
    }

    fn status_history_window_id(&self) -> Option<WindowId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::StatusHistory)
            .map(|window| window.id)
    }

    fn status_history_buffer_id(&self) -> Option<BufferId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::StatusHistory)
            .map(|window| window.buffer_id)
    }

    fn status_history_text(&self) -> String {
        let mut out = String::from("Dun Status History\n\n");
        if self.status_history.is_empty() {
            out.push_str("No status messages yet.\n");
            return out;
        }

        for (index, entry) in self.status_history.iter().enumerate() {
            out.push_str(&format!(
                "{:>3}. [{}] {}\n",
                index + 1,
                entry.level.label(),
                entry.message
            ));
        }

        out
    }

    fn config_diagnostics_text(&self) -> String {
        let mut out = String::from("Dun Config Diagnostics\n\n");

        out.push_str("Source\n");
        out.push_str(&format!(
            "  active: {}\n",
            self.config_source.diagnostics_text()
        ));
        out.push_str(&format!(
            "  request: {}\n",
            self.config_request.diagnostics_text()
        ));
        out.push_str(&format!("  {DUN_CONFIG_ENV}: {}\n", env_config_path_text()));
        out.push_str(&format!("  default path: {}\n", default_config_path_text()));

        out.push_str("\nTerminal\n");
        out.push_str(&format!(
            "  detected: {}\n",
            terminal_profile_status(self.detected_profile)
        ));
        out.push_str(&format!(
            "  effective: {}\n",
            terminal_profile_status(self.shell.profile)
        ));
        out.push_str(&format!(
            "  theme: {} ({})\n",
            self.shell.theme.name,
            color_status(self.shell.theme.colors)
        ));
        out.push_str(&format!(
            "  glyphs: {}\n",
            if self.shell.profile.supports_unicode_glyphs() {
                "unicode"
            } else {
                "ascii"
            }
        ));

        out.push_str("\nInput\n");
        out.push_str(&format!(
            "  mouse: {}\n",
            if self.mouse_enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));

        out.push_str("\nLimits\n");
        out.push_str(&format!(
            "  editable_file_soft_limit_bytes: {}\n",
            self.limits.editable_file_soft_limit_bytes
        ));
        out.push_str(&format!(
            "  line_display_soft_limit_bytes: {}\n",
            self.limits.line_display_soft_limit_bytes
        ));

        out.push_str("\nKeymap\n");
        let mut bindings = self
            .shell
            .keymap
            .bindings
            .iter()
            .map(|binding| (command_id(&binding.command), binding.sequence.to_string()))
            .collect::<Vec<_>>();
        bindings.sort_by(|left, right| left.0.cmp(right.0));
        for (command, sequence) in bindings {
            out.push_str(&format!("  {command:<28} {sequence}\n"));
        }

        out
    }

    fn close_focused_window_unchecked(&mut self) {
        let closing_buffer_id = self
            .workspace
            .focused_window()
            .ok()
            .map(|window| window.buffer_id);
        match self.workspace.close_focused() {
            Ok(_) => {
                if let Some(buffer_id) = closing_buffer_id {
                    self.drop_buffer_if_unreferenced(buffer_id);
                }
                self.set_status("Closed window");
            }
            Err(error) => {
                self.set_status(format!("Close failed: {}", workspace_error_text(error)));
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

    fn focused_buffer_is_dirty(&self) -> bool {
        self.focused_buffer_id()
            .and_then(|buffer_id| self.buffer_state(buffer_id))
            .is_some_and(|buffer| buffer.buffer.is_dirty())
    }

    fn buffer_state(&self, id: BufferId) -> Option<&BufferState> {
        self.buffers.iter().find(|buffer| buffer.id == id)
    }

    fn buffer_state_mut(&mut self, id: BufferId) -> Option<&mut BufferState> {
        self.buffers.iter_mut().find(|buffer| buffer.id == id)
    }

    fn set_status(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.status_message = Some(message.clone());
        self.record_status(message);
    }

    fn record_status(&mut self, message: String) {
        self.status_history.push(StatusEntry {
            level: StatusLevel::for_message(&message),
            message,
        });
        if self.status_history.len() > STATUS_HISTORY_LIMIT {
            let overflow = self.status_history.len() - STATUS_HISTORY_LIMIT;
            self.status_history.drain(0..overflow);
        }
        self.refresh_status_history_buffer();
    }

    fn refresh_status_history_buffer(&mut self) {
        let Some(buffer_id) = self.status_history_buffer_id() else {
            return;
        };
        let text = self.status_history_text();

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = BufferState::new(buffer_id, status_history_buffer(&text));
        }
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
        if !matches!(kind, PromptKind::ReplaceWith) {
            self.pending_replace_query = None;
        }
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
            CrosstermKeyCode::Up => {
                self.recall_previous_command();
            }
            CrosstermKeyCode::Down => {
                self.recall_next_command();
            }
            CrosstermKeyCode::Backspace => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.detach_history();
                    prompt.input.pop();
                }
            }
            _ => {
                if let Some(ch) = text_input_from_crossterm(event) {
                    if let Some(prompt) = &mut self.prompt {
                        prompt.detach_history();
                        prompt.input.push(ch);
                    }
                }
            }
        }

        true
    }

    fn recall_previous_command(&mut self) {
        let history_len = self.command_history.len();
        if history_len == 0 {
            return;
        }

        let next_index = {
            let Some(prompt) = self.command_line_prompt_mut() else {
                return;
            };
            let next_index = match prompt.history_index {
                Some(0) => 0,
                Some(index) => index - 1,
                None => {
                    prompt.history_draft = prompt.input.clone();
                    history_len - 1
                }
            };
            prompt.history_index = Some(next_index);
            next_index
        };

        let input = self.command_history[next_index].clone();
        if let Some(prompt) = self.command_line_prompt_mut() {
            prompt.input = input;
        }
    }

    fn recall_next_command(&mut self) {
        let history_len = self.command_history.len();
        let (entry_index, draft) = {
            let Some(prompt) = self.command_line_prompt_mut() else {
                return;
            };
            let Some(index) = prompt.history_index else {
                return;
            };
            if index + 1 < history_len {
                let next_index = index + 1;
                prompt.history_index = Some(next_index);
                (Some(next_index), None)
            } else {
                prompt.history_index = None;
                (None, Some(std::mem::take(&mut prompt.history_draft)))
            }
        };

        let input = entry_index
            .map(|index| self.command_history[index].clone())
            .or(draft)
            .unwrap_or_default();
        if let Some(prompt) = self.command_line_prompt_mut() {
            prompt.input = input;
        }
    }

    fn command_line_prompt_mut(&mut self) -> Option<&mut PromptState> {
        self.prompt
            .as_mut()
            .filter(|prompt| prompt.kind == PromptKind::CommandLine)
    }

    fn cancel_prompt(&mut self) {
        if let Some(prompt) = self.prompt.take() {
            if prompt.kind.is_replace() {
                self.pending_replace_query = None;
            }
            self.set_status(format!("{} cancelled", prompt.kind.name()));
        }
    }

    fn submit_prompt(&mut self) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };

        match prompt.kind {
            PromptKind::Open => {
                let input = prompt.input.trim().to_string();
                if input.is_empty() {
                    self.set_status(format!("{} cancelled", prompt.kind.name()));
                    return;
                }

                if let Err(error) = self.open_file_path(PathBuf::from(&input)) {
                    self.set_status(format!("Open failed: {error}"));
                }
            }
            PromptKind::SaveAs => {
                let input = prompt.input.trim().to_string();
                if input.is_empty() {
                    self.set_status(format!("{} cancelled", prompt.kind.name()));
                    return;
                }

                if let Err(error) = self.save_focused_buffer_as(PathBuf::from(&input)) {
                    self.set_status(format!("Save As failed: {error}"));
                } else if let Some(action) = prompt.after_success {
                    self.continue_pending_action(action);
                }
            }
            PromptKind::Find => {
                let input = prompt.input.trim().to_string();
                if input.is_empty() {
                    self.set_status(format!("{} cancelled", prompt.kind.name()));
                    return;
                }

                self.last_find_query = Some(input.clone());
                self.find_in_focused_buffer(&input, SearchDirection::Forward);
            }
            PromptKind::ReplaceFind => {
                let input = prompt.input.trim().to_string();
                if input.is_empty() {
                    self.pending_replace_query = None;
                    self.set_status(format!("{} cancelled", prompt.kind.name()));
                    return;
                }

                self.pending_replace_query = Some(input);
                self.start_prompt(PromptKind::ReplaceWith, String::new());
            }
            PromptKind::ReplaceWith => {
                let Some(query) = self.pending_replace_query.take() else {
                    self.set_status("Replace: no query");
                    return;
                };

                self.last_find_query = Some(query.clone());
                self.replace_in_focused_buffer(&query, &prompt.input);
            }
            PromptKind::GoToLine => {
                let input = prompt.input.trim();
                if input.is_empty() {
                    self.set_status(format!("{} cancelled", prompt.kind.name()));
                    return;
                }

                self.go_to_line(input);
            }
            PromptKind::CommandLine => {
                let input = prompt.input.trim().to_string();
                if input.is_empty() {
                    self.set_status(format!("{} cancelled", prompt.kind.name()));
                    return;
                }

                self.record_command_history(input.clone());
                self.run_command_line(&input);
            }
        }
    }

    fn record_command_history(&mut self, input: String) {
        if self
            .command_history
            .last()
            .is_some_and(|previous| previous == &input)
        {
            return;
        }

        self.command_history.push(input);
        if self.command_history.len() > COMMAND_HISTORY_LIMIT {
            let overflow = self.command_history.len() - COMMAND_HISTORY_LIMIT;
            self.command_history.drain(0..overflow);
        }
    }

    fn run_command_line(&mut self, input: &str) {
        let tokens = match parse_command_line(input) {
            Ok(tokens) => tokens,
            Err(error) => {
                self.set_status(format!(
                    "Command failed: {}",
                    command_line_parse_error_text(error)
                ));
                return;
            }
        };
        let Some((command, args)) = tokens.split_first() else {
            self.set_status("Command cancelled");
            return;
        };

        match normalize_command_line_token(command).as_str() {
            "help" | "h" | "?" => self.open_help_screen(),
            "config" | "diagnostics" | "configdiagnostics" => self.open_config_diagnostics_screen(),
            "reload" | "reloadconfig" => self.reload_config(),
            "status" | "statushistory" => self.open_status_history_screen(),
            "theme" => self.run_theme_command(args),
            "quit" | "q" => self.handle_app_command(&AppCommand::Quit),
            "open" | "o" => self.run_open_command(args),
            "save" | "write" | "w" => self.run_save_command(args),
            "saveas" | "writeas" => self.run_save_as_command(args),
            "new" => self.run_no_arg_command(args, EditorCommand::File(FileCommand::New)),
            "close" => self.run_no_arg_command(args, EditorCommand::File(FileCommand::Close)),
            "find" => self.run_find_command(args),
            "replace" => self.run_replace_command(args),
            "goto" | "gotoline" | "line" => self.run_go_to_line_command(args),
            "commands" => self.set_status(COMMAND_LINE_HELP),
            _ => self.run_command_id_command(command, args),
        }
    }

    fn run_command_id_command(&mut self, command: &str, args: &[String]) {
        match command_from_id(command) {
            Ok(command) => self.run_no_arg_command(args, command),
            Err(_) => self.set_status(format!("Unknown command: {command}")),
        }
    }

    fn run_theme_command(&mut self, args: &[String]) {
        match args {
            [] => self.set_status(format!(
                "Theme: {} ({})",
                self.shell.theme.name,
                theme_command_values()
            )),
            [theme] => match parse_theme_command_value(theme) {
                Some(theme) => self.set_runtime_theme(theme),
                None => self.set_status(format!(
                    "Theme failed: unknown theme {theme}; expected {}",
                    theme_command_values()
                )),
            },
            _ => self.set_status("Command failed: theme expects zero or one theme name"),
        }
    }

    fn set_runtime_theme(&mut self, theme: ThemeName) {
        self.shell.theme = Theme::for_profile(theme, self.shell.profile);
        self.refresh_config_diagnostics_buffer();
        self.set_status(format!("Theme: {}", theme.as_str()));
    }

    fn run_open_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_file_command(&FileCommand::Open),
            [path] => {
                if self.focused_buffer_is_dirty() {
                    self.set_status("Open failed: focused buffer has unsaved changes");
                    return;
                }
                if let Err(error) = self.open_file_path(PathBuf::from(path)) {
                    self.set_status(format!("Open failed: {error}"));
                }
            }
            _ => self.set_status("Command failed: open expects zero or one path"),
        }
    }

    fn run_save_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_file_command(&FileCommand::Save),
            [path] => {
                if let Err(error) = self.save_focused_buffer_as(PathBuf::from(path)) {
                    self.set_status(format!("Save As failed: {error}"));
                }
            }
            _ => self.set_status("Command failed: save expects zero or one path"),
        }
    }

    fn run_save_as_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_file_command(&FileCommand::SaveAs),
            [path] => {
                if let Err(error) = self.save_focused_buffer_as(PathBuf::from(path)) {
                    self.set_status(format!("Save As failed: {error}"));
                }
            }
            _ => self.set_status("Command failed: save-as expects zero or one path"),
        }
    }

    fn run_no_arg_command(&mut self, args: &[String], command: EditorCommand) {
        if !args.is_empty() {
            self.set_status(format!(
                "Command failed: {} expects no arguments",
                command_id(&command)
            ));
            return;
        }

        self.handle_command(&command);
    }

    fn run_find_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_edit_command(&EditCommand::Find),
            [query] => {
                self.last_find_query = Some(query.clone());
                self.find_in_focused_buffer(query, SearchDirection::Forward);
            }
            _ => self.set_status("Command failed: find expects zero or one query"),
        }
    }

    fn run_replace_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_edit_command(&EditCommand::Replace),
            [query, replacement] => {
                self.last_find_query = Some(query.clone());
                self.replace_in_focused_buffer(query, replacement);
            }
            _ => self.set_status("Command failed: replace expects query and replacement"),
        }
    }

    fn run_go_to_line_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_edit_command(&EditCommand::GoToLine),
            [line] => self.go_to_line(line),
            _ => self.set_status("Command failed: go-to-line expects one line number"),
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

    fn replace_in_focused_buffer(&mut self, query: &str, replacement: &str) {
        if query.is_empty() {
            self.set_status("Replace: no query");
            return;
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Replace: focused buffer is missing");
            return;
        };

        let matches = buffer.buffer.find_all(query);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            self.set_status(format!("Replace: no matches for {query}"));
            return;
        }

        let selection = current_match_selection(&buffer.buffer, &matches).unwrap_or_else(|| {
            let origin = buffer
                .buffer
                .selection_range()
                .map(|range| range.end)
                .unwrap_or_else(|| buffer.buffer.cursor_position());
            choose_search_match(&matches, origin, SearchDirection::Forward)
        });
        let target = matches[selection.index].range;

        match buffer.buffer.replace_range(target, replacement) {
            Ok(()) => {
                let suffix = if selection.wrapped { " (wrapped)" } else { "" };
                self.set_status(format!(
                    "Replace: {}/{} {query}{suffix}",
                    selection.index + 1,
                    matches.len()
                ));
            }
            Err(error) => self.set_status(format!("Replace failed: {}", buffer_error_text(error))),
        }
    }

    fn go_to_line(&mut self, input: &str) {
        let Ok(line_number) = input.parse::<usize>() else {
            self.set_status(format!("Go to line failed: invalid line number {input}"));
            return;
        };
        if line_number == 0 {
            self.set_status("Go to line failed: line numbers start at 1");
            return;
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Go to line failed: focused buffer is missing");
            return;
        };

        let line_count = buffer.buffer.line_count();
        if line_number > line_count {
            self.set_status(format!(
                "Go to line failed: line {line_number} is past end ({line_count} lines)"
            ));
            return;
        }

        let target_line = line_number - 1;
        let current_column = buffer.buffer.cursor_position().column;
        let target_column = buffer
            .buffer
            .line(target_line)
            .map(|line| clamp_to_char_boundary(line, current_column))
            .unwrap_or(0);

        match buffer
            .buffer
            .set_cursor(Position::new(target_line, target_column))
        {
            Ok(()) => self.set_status(format!("Go to line: {line_number}")),
            Err(error) => {
                self.set_status(format!("Go to line failed: {}", buffer_error_text(error)))
            }
        }
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
        let escaped = if buffer.encoding == FileTextEncoding::EscapedBytes {
            " [escaped]"
        } else {
            ""
        };

        format!("{name}{dirty}{read_only}{escaped}")
    }

    fn focused_detail_status(&self) -> String {
        let profile = terminal_profile_status(self.shell.profile);
        let window = self.focused_window_status();
        let Some(buffer_id) = self.focused_buffer_id() else {
            return format!("Ln -, Col - | {profile} | {window}");
        };
        let Some(buffer) = self.buffer_state(buffer_id) else {
            return format!("Ln -, Col - | {profile} | {window}");
        };

        let position = buffer.buffer.cursor_position();
        let column = buffer
            .buffer
            .line(position.line)
            .and_then(|line| line.get(..position.column))
            .map(|prefix| UnicodeWidthStr::width(prefix) + 1)
            .unwrap_or(1);

        let mut parts = vec![
            format!(
                "Ln {}/{}, Col {}",
                position.line + 1,
                buffer.buffer.line_count(),
                column
            ),
            line_ending_status(buffer.buffer.line_ending()).to_string(),
            buffer.encoding.status_label().to_string(),
            profile,
            window,
        ];
        if let Some(selection) = selection_status(&buffer.buffer) {
            parts.insert(1, selection);
        }

        parts.join(" | ")
    }

    fn focused_window_status(&self) -> String {
        let total = self.workspace.window_count();
        let Some(index) = self
            .workspace
            .windows
            .iter()
            .position(|window| window.id == self.workspace.focused)
            .map(|index| index + 1)
        else {
            return format!("Win -/{total}");
        };

        format!("Win {index}/{total}")
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
    history_index: Option<usize>,
    history_draft: String,
}

impl PromptState {
    fn new(kind: PromptKind, input: String, after_success: Option<PendingAction>) -> Self {
        Self {
            kind,
            input,
            after_success,
            history_index: None,
            history_draft: String::new(),
        }
    }

    fn status_text(&self) -> String {
        format!("{}{}", self.kind.label(), self.input)
    }

    fn detach_history(&mut self) {
        if self.kind == PromptKind::CommandLine {
            self.history_index = None;
            self.history_draft.clear();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromptKind {
    CommandLine,
    Open,
    SaveAs,
    Find,
    ReplaceFind,
    ReplaceWith,
    GoToLine,
}

impl PromptKind {
    const fn label(self) -> &'static str {
        match self {
            Self::CommandLine => "Command: ",
            Self::Open => "Open: ",
            Self::SaveAs => "Save As: ",
            Self::Find => "Find: ",
            Self::ReplaceFind => "Replace Find: ",
            Self::ReplaceWith => "Replace With: ",
            Self::GoToLine => "Go To Line: ",
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::CommandLine => "Command",
            Self::Open => "Open",
            Self::SaveAs => "Save As",
            Self::Find => "Find",
            Self::ReplaceFind | Self::ReplaceWith => "Replace",
            Self::GoToLine => "Go To Line",
        }
    }

    const fn is_replace(self) -> bool {
        matches!(self, Self::ReplaceFind | Self::ReplaceWith)
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

const COMMAND_LINE_HELP: &str = "Commands: help, config, status, reload-config, theme [name], open [path], save [path], save-as [path], find [query], replace QUERY TEXT, goto LINE, or any command id such as window.split_horizontal";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandLineParseError {
    TrailingEscape,
    UnclosedQuote,
}

fn parse_command_line(input: &str) -> Result<Vec<String>, CommandLineParseError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut token_started = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            token_started = true;
            continue;
        }

        if let Some(quote_char) = quote {
            if ch == quote_char {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '\'' | '"' => {
                quote = Some(ch);
                token_started = true;
            }
            ch if ch.is_whitespace() => {
                if token_started {
                    tokens.push(std::mem::take(&mut current));
                    token_started = false;
                }
            }
            _ => {
                current.push(ch);
                token_started = true;
            }
        }
    }

    if escaped {
        return Err(CommandLineParseError::TrailingEscape);
    }
    if quote.is_some() {
        return Err(CommandLineParseError::UnclosedQuote);
    }
    if token_started {
        tokens.push(current);
    }

    Ok(tokens)
}

fn command_line_parse_error_text(error: CommandLineParseError) -> &'static str {
    match error {
        CommandLineParseError::TrailingEscape => "trailing escape",
        CommandLineParseError::UnclosedQuote => "unclosed quote",
    }
}

fn normalize_command_line_token(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|ch| *ch != '-' && *ch != '_' && !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn parse_theme_command_value(input: &str) -> Option<ThemeName> {
    match normalize_command_line_token(input).as_str() {
        "msedit" | "microsoftedit" => Some(ThemeName::MsEdit),
        "turbo" | "turbovision" => Some(ThemeName::Turbo),
        "dark" => Some(ThemeName::Dark),
        "dun" => Some(ThemeName::Dun),
        _ => None,
    }
}

const fn theme_command_values() -> &'static str {
    "msedit|turbo|dark|dun"
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

fn current_match_selection(
    buffer: &TextBuffer,
    matches: &[SearchMatch],
) -> Option<SearchSelection> {
    let range = buffer.selection_range()?;
    matches
        .iter()
        .position(|item| item.range == range)
        .map(|index| SearchSelection {
            index,
            wrapped: false,
        })
}

fn selection_status(buffer: &TextBuffer) -> Option<String> {
    let range = buffer.selection_range()?;
    if range.is_empty() {
        return None;
    }

    if range.start.line == range.end.line {
        let line = buffer.line(range.start.line)?;
        let selected = &line[range.start.column..range.end.column];
        return Some(format!("Sel {}c", UnicodeWidthStr::width(selected)));
    }

    Some(format!("Sel {}L", range.end.line - range.start.line + 1))
}

const fn line_ending_status(line_ending: LineEnding) -> &'static str {
    match line_ending {
        LineEnding::Lf => "LF",
        LineEnding::CrLf => "CRLF",
    }
}

fn terminal_profile_status(profile: TerminalProfile) -> String {
    format!(
        "{}/{}",
        encoding_status(profile.encoding),
        color_status(profile.colors)
    )
}

fn env_config_path_text() -> String {
    env_config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(unset)".to_string())
}

fn default_config_path_text() -> String {
    default_config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(unavailable)".to_string())
}

const fn encoding_status(encoding: EncodingProfile) -> &'static str {
    match encoding {
        EncodingProfile::Utf8 => "UTF-8",
        EncodingProfile::Ascii => "ASCII",
    }
}

const fn color_status(colors: ColorProfile) -> &'static str {
    match colors {
        ColorProfile::Color256 => "256c",
        ColorProfile::Color16 => "16c",
        ColorProfile::Mono => "mono",
    }
}

const fn buffer_error_text(error: BufferError) -> &'static str {
    match error {
        BufferError::InvalidPosition(_) => "invalid position",
        BufferError::InvalidRange(_) => "invalid range",
        BufferError::ReadOnly => "buffer is read-only",
    }
}

const fn workspace_error_text(error: WorkspaceError) -> &'static str {
    match error {
        WorkspaceError::CannotCloseLastWindow => "cannot close the last window",
        WorkspaceError::FocusMissing => "focused window is missing",
        WorkspaceError::NoNeighbor => "no neighboring pane",
        WorkspaceError::NoResizableSplit => "no matching split",
        WorkspaceError::WindowMissing => "window is missing",
    }
}

const fn axis_name(axis: Axis) -> &'static str {
    match axis {
        Axis::Horizontal => "horizontal",
        Axis::Vertical => "vertical",
    }
}

struct BufferState {
    id: BufferId,
    buffer: TextBuffer,
    path: Option<PathBuf>,
    encoding: FileTextEncoding,
    first_line: usize,
}

impl BufferState {
    fn new(id: BufferId, buffer: TextBuffer) -> Self {
        Self {
            id,
            buffer,
            path: None,
            encoding: FileTextEncoding::Utf8,
            first_line: 0,
        }
    }

    fn from_file(id: BufferId, path: PathBuf, loaded: LoadedTextBuffer) -> Self {
        Self {
            id,
            buffer: loaded.buffer,
            path: Some(path),
            encoding: loaded.encoding,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct StatusEntry {
    level: StatusLevel,
    message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusLevel {
    Info,
    Error,
}

impl StatusLevel {
    fn for_message(message: &str) -> Self {
        let message = message.to_ascii_lowercase();
        if message.contains("failed") || message.contains("error") || message.contains("invalid") {
            Self::Error
        } else {
            Self::Info
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Error => "error",
        }
    }
}

const STATUS_HISTORY_LIMIT: usize = 128;
const COMMAND_HISTORY_LIMIT: usize = 128;

struct TerminalGuard {
    mouse_enabled: bool,
}

impl TerminalGuard {
    fn enter(mouse_enabled: bool) -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        if mouse_enabled {
            if let Err(error) = execute!(stdout, EnableMouseCapture) {
                let _ = execute!(stdout, LeaveAlternateScreen);
                let _ = disable_raw_mode();
                return Err(error);
            }
        }
        Ok(Self { mouse_enabled })
    }

    fn set_mouse_enabled(&mut self, enabled: bool) -> io::Result<()> {
        if self.mouse_enabled == enabled {
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
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        if self.mouse_enabled {
            let _ = execute!(stdout, DisableMouseCapture);
        }
        let _ = execute!(stdout, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut AppState,
    guard: &mut TerminalGuard,
) -> io::Result<()> {
    while !app.should_quit {
        guard.set_mouse_enabled(app.mouse_enabled())?;
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
            ui_frame.status.right = app.focused_detail_status();
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
                Event::Mouse(event) => handle_mouse_event(app, event),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_mouse_event(app: &mut AppState, event: CrosstermMouseEvent) {
    if !app.mouse_enabled() || app.prompt.is_some() || app.confirm.is_some() {
        return;
    }

    if matches!(
        event.kind,
        CrosstermMouseEventKind::Down(CrosstermMouseButton::Left)
    ) {
        app.handle_left_click(event.column, event.row);
    }
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

fn help_buffer(keymap: &Keymap) -> TextBuffer {
    let text = help_text(keymap);
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, &text)
}

fn help_text(keymap: &Keymap) -> String {
    let mut out = String::from("Dun Help\n\n");

    for (index, section) in HELP_SECTIONS.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(section.title);
        out.push('\n');

        for command in section.commands {
            push_help_command(&mut out, keymap, &command.command, command.description);
        }
    }

    out.push_str(
        "\nPrompts\n  Enter           Submit prompt\n  Esc             Cancel prompt\n  Backspace       Edit prompt input\n  Up/Down         Command history\n\n",
    );
    out.push_str(
        "Notes\n  Type commands in the command prompt to list command-line actions.\n  Help opens as a read-only tiled window.\n  Dirty buffers ask for confirmation before changes are discarded.\n",
    );

    out
}

fn push_help_command(
    out: &mut String,
    keymap: &Keymap,
    command: &EditorCommand,
    description: &str,
) {
    let sequence = keymap
        .sequence_for_command(command)
        .map(ToString::to_string)
        .unwrap_or_else(|| "(unbound)".to_string());
    out.push_str(&format!(
        "  {sequence:<15} {description} [{}]\n",
        command_id(command)
    ));
}

struct HelpSection {
    title: &'static str,
    commands: &'static [HelpCommand],
}

struct HelpCommand {
    command: EditorCommand,
    description: &'static str,
}

const HELP_SECTIONS: &[HelpSection] = &[
    HelpSection {
        title: "App",
        commands: &[
            HelpCommand {
                command: EditorCommand::App(AppCommand::Help),
                description: "Help",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::StatusHistory),
                description: "Status history",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandLine),
                description: "Command line",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::ConfigDiagnostics),
                description: "Config diagnostics",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::ReloadConfig),
                description: "Reload config",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::Quit),
                description: "Quit",
            },
        ],
    },
    HelpSection {
        title: "File",
        commands: &[
            HelpCommand {
                command: EditorCommand::File(FileCommand::New),
                description: "New untitled buffer",
            },
            HelpCommand {
                command: EditorCommand::File(FileCommand::Open),
                description: "Open file",
            },
            HelpCommand {
                command: EditorCommand::File(FileCommand::Save),
                description: "Save",
            },
            HelpCommand {
                command: EditorCommand::File(FileCommand::SaveAs),
                description: "Save as",
            },
            HelpCommand {
                command: EditorCommand::File(FileCommand::Close),
                description: "Close focused file/window",
            },
        ],
    },
    HelpSection {
        title: "Edit",
        commands: &[
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveLeft),
                description: "Move left",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveRight),
                description: "Move right",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveUp),
                description: "Move up",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveDown),
                description: "Move down",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveLineStart),
                description: "Move to line start",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveLineEnd),
                description: "Move to line end",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::InsertNewline),
                description: "Insert newline",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::DeleteBackward),
                description: "Delete backward",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::DeleteForward),
                description: "Delete forward",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Undo),
                description: "Undo",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Redo),
                description: "Redo",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::SelectAll),
                description: "Select all",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Find),
                description: "Find",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::FindNext),
                description: "Find next",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::FindPrevious),
                description: "Find previous",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Replace),
                description: "Replace current or next match",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::GoToLine),
                description: "Go to line",
            },
        ],
    },
    HelpSection {
        title: "Windows",
        commands: &[
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::SplitHorizontal),
                description: "Split horizontally",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::SplitVertical),
                description: "Split vertically",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::FocusLeft),
                description: "Focus left",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::FocusRight),
                description: "Focus right",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::FocusUp),
                description: "Focus up",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::FocusDown),
                description: "Focus down",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::ResizeLeft),
                description: "Resize left",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::ResizeRight),
                description: "Resize right",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::ResizeUp),
                description: "Resize up",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::ResizeDown),
                description: "Resize down",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::Equalize),
                description: "Equalize splits",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::RotateSplit),
                description: "Rotate focused split",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::ToggleCollapse),
                description: "Collapse or expand focused pane",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::Close),
                description: "Close focused window",
            },
        ],
    },
];

fn status_history_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}

fn config_diagnostics_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}

fn buffer_end_position(buffer: &TextBuffer) -> Position {
    let last_line = buffer.line_count().saturating_sub(1);
    let last_column = buffer.line(last_line).map(str::len).unwrap_or(0);
    Position::new(last_line, last_column)
}

fn clamp_to_char_boundary(line: &str, column: usize) -> usize {
    let mut column = column.min(line.len());
    while !line.is_char_boundary(column) {
        column -= 1;
    }
    column
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LoadedTextBuffer {
    buffer: TextBuffer,
    encoding: FileTextEncoding,
}

fn load_text_buffer(path: &Path, limits: Limits) -> io::Result<LoadedTextBuffer> {
    let bytes = read_editable_file(path, limits.editable_file_soft_limit_bytes)?;
    let decoded = decode_file_text(bytes);
    Ok(LoadedTextBuffer {
        buffer: TextBuffer::from_text_with_kind(decoded.encoding.buffer_kind(), &decoded.text),
        encoding: decoded.encoding,
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AtomicTempReconcileReport {
    cleaned: usize,
    cleanup_failures: usize,
    recovery_candidates: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AtomicWriteReport {
    temp_reconcile: AtomicTempReconcileReport,
}

fn atomic_write_text_file(path: &Path, text: &str) -> io::Result<AtomicWriteReport> {
    let destination = atomic_write_destination(path)?;
    let preexisting_temp_report = reconcile_atomic_save_temp_files(&destination);
    let recovery_candidates_to_preserve = preexisting_temp_report.recovery_candidates.clone();
    let existing_permissions = existing_atomic_write_permissions(&destination)?;
    let (temp_path, mut temp_file) = create_atomic_temp_file(&destination)?;

    let write_result = (|| {
        if let Some(permissions) = existing_permissions {
            temp_file.set_permissions(permissions)?;
        }
        temp_file.write_all(text.as_bytes())?;
        temp_file.sync_all()?;
        Ok(())
    })();

    if let Err(error) = write_result {
        drop(temp_file);
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    drop(temp_file);
    if let Err(error) = fs::rename(&temp_path, &destination) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    let post_save_temp_report =
        reconcile_atomic_save_temp_files_preserving(&destination, &recovery_candidates_to_preserve);
    Ok(AtomicWriteReport {
        temp_reconcile: merged_atomic_temp_reports(preexisting_temp_report, post_save_temp_report),
    })
}

fn atomic_write_destination(path: &Path) -> io::Result<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "save path is empty",
        ));
    }

    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::canonicalize(path),
        Ok(_) => Ok(path.to_path_buf()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(error) => Err(error),
    }
}

fn existing_atomic_write_permissions(path: &Path) -> io::Result<Option<fs::Permissions>> {
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "destination is a directory",
                ));
            }
            if !metadata.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "destination is not a regular file",
                ));
            }

            let permissions = metadata.permissions();
            if permissions.readonly() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "destination is read-only",
                ));
            }

            Ok(Some(permissions))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn create_atomic_temp_file(path: &Path) -> io::Result<(PathBuf, fs::File)> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    validate_save_parent_directory(directory)?;
    let file_name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path has no file name"))?;

    for attempt in 0..1000 {
        let temp_path = atomic_temp_path(directory, file_name, attempt);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate atomic save temp file",
    ))
}

fn atomic_temp_path(directory: &Path, file_name: &OsStr, attempt: u32) -> PathBuf {
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(".dun-save-{}-{attempt}.tmp", std::process::id()));
    directory.join(temp_name)
}

fn reconcile_atomic_save_temp_files(path: &Path) -> AtomicTempReconcileReport {
    reconcile_atomic_save_temp_files_preserving(path, &[])
}

fn reconcile_atomic_save_temp_files_preserving(
    path: &Path,
    preserve: &[PathBuf],
) -> AtomicTempReconcileReport {
    let Ok(destination) = atomic_write_destination(path) else {
        return AtomicTempReconcileReport::default();
    };
    let Some(file_name) = destination.file_name().filter(|name| !name.is_empty()) else {
        return AtomicTempReconcileReport::default();
    };
    let directory = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let destination_modified = fs::metadata(&destination)
        .and_then(|metadata| metadata.modified())
        .ok();
    let Ok(entries) = fs::read_dir(directory) else {
        return AtomicTempReconcileReport::default();
    };

    let mut report = AtomicTempReconcileReport::default();
    for entry in entries.filter_map(Result::ok) {
        let entry_file_name = entry.file_name();
        if !is_atomic_temp_file_name_for(file_name, &entry_file_name) {
            continue;
        }

        let path = entry.path();
        if preserve.iter().any(|preserved| preserved == &path) {
            report.recovery_candidates.push(path);
            continue;
        }

        if atomic_temp_file_is_obsolete(&path, destination_modified) {
            match fs::remove_file(&path) {
                Ok(()) => report.cleaned += 1,
                Err(_) => report.cleanup_failures += 1,
            }
        } else {
            report.recovery_candidates.push(path);
        }
    }

    report
}

fn merged_atomic_temp_reports(
    first: AtomicTempReconcileReport,
    second: AtomicTempReconcileReport,
) -> AtomicTempReconcileReport {
    let mut recovery_candidates = first.recovery_candidates;
    for candidate in second.recovery_candidates {
        if !recovery_candidates.contains(&candidate) {
            recovery_candidates.push(candidate);
        }
    }

    AtomicTempReconcileReport {
        cleaned: first.cleaned + second.cleaned,
        cleanup_failures: first.cleanup_failures + second.cleanup_failures,
        recovery_candidates,
    }
}

fn is_atomic_temp_file_name_for(destination_file_name: &OsStr, candidate: &OsStr) -> bool {
    let mut prefix = OsString::from(".");
    prefix.push(destination_file_name);
    prefix.push(".dun-save-");
    let prefix = prefix.to_string_lossy();
    let candidate = candidate.to_string_lossy();

    let Some(suffix) = candidate
        .strip_prefix(&*prefix)
        .and_then(|suffix| suffix.strip_suffix(".tmp"))
    else {
        return false;
    };
    let Some((pid, attempt)) = suffix.split_once('-') else {
        return false;
    };

    !pid.is_empty()
        && !attempt.is_empty()
        && pid.bytes().all(|byte| byte.is_ascii_digit())
        && attempt.bytes().all(|byte| byte.is_ascii_digit())
}

fn atomic_temp_file_is_obsolete(
    path: &Path,
    destination_modified: Option<std::time::SystemTime>,
) -> bool {
    let Some(destination_modified) = destination_modified else {
        return false;
    };
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    metadata
        .modified()
        .is_ok_and(|modified| modified <= destination_modified)
}

fn status_with_atomic_temp_report(
    status: impl Into<String>,
    report: &AtomicTempReconcileReport,
) -> String {
    let mut status = status.into();
    let mut suffixes = Vec::new();

    if report.cleaned > 0 {
        suffixes.push(format!(
            "cleaned {} stale save temp file(s)",
            report.cleaned
        ));
    }
    if report.cleanup_failures > 0 {
        suffixes.push(format!(
            "failed to clean {} save temp file(s)",
            report.cleanup_failures
        ));
    }
    if let Some(first) = report.recovery_candidates.first() {
        if report.recovery_candidates.len() == 1 {
            suffixes.push(format!("recovery temp file found: {}", first.display()));
        } else {
            suffixes.push(format!(
                "{} recovery temp file(s) found; first: {}",
                report.recovery_candidates.len(),
                first.display()
            ));
        }
    }

    if !suffixes.is_empty() {
        status.push_str("; ");
        status.push_str(&suffixes.join("; "));
    }

    status
}

fn path_io_error(path: &Path, error: io::Error) -> io::Error {
    let kind = error.kind();
    io::Error::new(
        kind,
        format!("{}: {}", path_error_label(path), path_error_detail(&error)),
    )
}

fn path_error_label(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        "(empty path)".to_string()
    } else {
        path.display().to_string()
    }
}

fn path_error_detail(error: &io::Error) -> String {
    let message = error.to_string();
    match error.kind() {
        io::ErrorKind::NotFound if message == "parent directory does not exist" => message,
        io::ErrorKind::NotFound => "not found".to_string(),
        io::ErrorKind::PermissionDenied if message == "destination is read-only" => message,
        io::ErrorKind::PermissionDenied => "permission denied".to_string(),
        _ => message,
    }
}

fn validate_save_parent_directory(directory: &Path) -> io::Result<()> {
    match fs::metadata(directory) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "parent path is not a directory",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "parent directory does not exist",
        )),
        Err(error) => Err(error),
    }
}

fn read_editable_file(path: &Path, soft_limit: u64) -> io::Result<Vec<u8>> {
    let metadata = fs::metadata(path)?;
    if metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path is a directory",
        ));
    }
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path is not a regular file",
        ));
    }
    if metadata.len() > soft_limit {
        return Err(editable_file_soft_limit_error(metadata.len(), soft_limit));
    }
    let snapshot = FileReadSnapshot::from_metadata(&metadata);

    let file = fs::File::open(path)?;
    let mut reader = file.take(soft_limit.saturating_add(1));
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    let bytes_read = bytes.len() as u64;
    if bytes_read > soft_limit {
        return Err(editable_file_soft_limit_error(bytes_read, soft_limit));
    }
    validate_stable_file_read(path, snapshot, bytes_read)?;

    Ok(bytes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileReadSnapshot {
    len: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl FileReadSnapshot {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;

        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        }
    }
}

fn validate_stable_file_read(
    path: &Path,
    before: FileReadSnapshot,
    bytes_read: u64,
) -> io::Result<()> {
    if bytes_read != before.len {
        return Err(file_changed_while_reading_error());
    }

    let after = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(file_changed_while_reading_error());
        }
        Err(error) => return Err(error),
    };
    if !after.is_file() || FileReadSnapshot::from_metadata(&after) != before {
        return Err(file_changed_while_reading_error());
    }

    Ok(())
}

fn file_changed_while_reading_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "file changed while reading; retry open",
    )
}

fn editable_file_soft_limit_error(size: u64, soft_limit: u64) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "too large for editable mode: {size} bytes exceeds the {soft_limit} byte soft limit",
        ),
    )
}

fn opened_file_status(path: &Path, encoding: FileTextEncoding) -> String {
    match encoding {
        FileTextEncoding::Utf8 => format!("Opened {}", path.display()),
        FileTextEncoding::EscapedBytes => format!(
            "Opened {} read-only: non-UTF-8 bytes shown as escapes",
            path.display()
        ),
    }
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
    use dun_core::TextRange;
    use ratatui::backend::TestBackend;
    use std::str::FromStr;
    use std::time::Instant;

    fn left_click(column: u16, row: u16) -> CrosstermMouseEvent {
        CrosstermMouseEvent {
            kind: CrosstermMouseEventKind::Down(CrosstermMouseButton::Left),
            column,
            row,
            modifiers: CrosstermKeyModifiers::NONE,
        }
    }

    #[test]
    fn parse_cli_args_accepts_no_path_or_single_path() {
        assert_eq!(
            parse_cli_args(Vec::<&str>::new()).unwrap(),
            CliAction::Run {
                config_path: None,
                no_config: false,
                path: None,
            }
        );

        assert_eq!(
            parse_cli_args(["sample.txt"]).unwrap(),
            CliAction::Run {
                config_path: None,
                no_config: false,
                path: Some(PathBuf::from("sample.txt"))
            }
        );
    }

    #[test]
    fn parse_cli_args_accepts_help_version_and_separator() {
        assert_eq!(parse_cli_args(["--help"]).unwrap(), CliAction::Help);
        assert_eq!(parse_cli_args(["-h"]).unwrap(), CliAction::Help);
        assert_eq!(parse_cli_args(["--version"]).unwrap(), CliAction::Version);
        assert_eq!(parse_cli_args(["-V"]).unwrap(), CliAction::Version);
        assert_eq!(
            parse_cli_args(["--", "--literal-path"]).unwrap(),
            CliAction::Run {
                config_path: None,
                no_config: false,
                path: Some(PathBuf::from("--literal-path"))
            }
        );
    }

    #[test]
    fn parse_cli_args_accepts_config_options() {
        assert_eq!(
            parse_cli_args(["--config", "dun.conf", "sample.txt"]).unwrap(),
            CliAction::Run {
                config_path: Some(PathBuf::from("dun.conf")),
                no_config: false,
                path: Some(PathBuf::from("sample.txt")),
            }
        );
        assert_eq!(
            parse_cli_args(["--config=dun.conf"]).unwrap(),
            CliAction::Run {
                config_path: Some(PathBuf::from("dun.conf")),
                no_config: false,
                path: None,
            }
        );
        assert_eq!(
            parse_cli_args(["--no-config", "sample.txt"]).unwrap(),
            CliAction::Run {
                config_path: None,
                no_config: true,
                path: Some(PathBuf::from("sample.txt")),
            }
        );
    }

    #[test]
    fn parse_cli_args_reports_usage_errors() {
        assert_eq!(
            parse_cli_args(["--bad"]).unwrap_err().to_string(),
            "unknown option --bad"
        );
        assert_eq!(
            parse_cli_args(["one", "two"]).unwrap_err().to_string(),
            "expected at most one path, got 2"
        );
        assert_eq!(
            parse_cli_args(["--help", "file.txt"])
                .unwrap_err()
                .to_string(),
            "options --help and --version cannot be combined with paths"
        );
        assert_eq!(
            parse_cli_args(["--help", "--version"])
                .unwrap_err()
                .to_string(),
            "only one of --help or --version may be used"
        );
        assert_eq!(
            parse_cli_args(["--config"]).unwrap_err().to_string(),
            "missing path after --config"
        );
        assert_eq!(
            parse_cli_args(["--config", "one", "--config", "two"])
                .unwrap_err()
                .to_string(),
            "--config may only be used once"
        );
        assert_eq!(
            parse_cli_args(["--config", "one", "--no-config"])
                .unwrap_err()
                .to_string(),
            "--config and --no-config cannot be used together"
        );
    }

    #[test]
    fn cli_error_exit_codes_are_stable() {
        assert_eq!(CliError::Usage(UsageError::new("bad")).exit_code(), 2);
        assert_eq!(CliError::Io(io::Error::other("boom")).exit_code(), 1);
        assert!(cli_help_text().contains("Exit codes:"));
        assert!(cli_help_text().contains("--config PATH"));
        assert_eq!(
            cli_version_text(),
            format!("dun {}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn load_startup_config_reads_explicit_config_path() {
        let path = temp_file_path("dun-config");
        std::fs::write(
            &path,
            "\
theme = dark
limits.editable_file_soft_limit_bytes = 3 KiB
key.app.quit = Esc
",
        )
        .unwrap();

        let config = load_startup_config(Some(&path), false).unwrap();

        assert_eq!(config.theme, dun_config::ThemeName::Dark);
        assert_eq!(config.limits.editable_file_soft_limit_bytes, 3 * 1024);
        assert_eq!(
            config
                .keybindings
                .command_for_sequence(&KeySequence::from_str("Esc").unwrap()),
            Some(&EditorCommand::App(AppCommand::Quit))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_startup_config_reports_parse_errors_with_path() {
        let path = temp_file_path("bad-dun-config");
        std::fs::write(&path, "bad = value").unwrap();

        let error = load_startup_config(Some(&path), false).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains(path.to_string_lossy().as_ref()));
        assert!(error.to_string().contains("line 1"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_startup_config_reports_keybinding_conflicts_with_path() {
        let path = temp_file_path("conflicting-dun-config");
        std::fs::write(&path, "key.app.quit = Ctrl+S").unwrap();

        let error = load_startup_config(Some(&path), false).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains(path.to_string_lossy().as_ref()));
        assert!(error.to_string().contains("duplicate key sequence"));

        let _ = std::fs::remove_file(path);
    }

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
    fn mouse_click_is_ignored_when_mouse_is_disabled() {
        let mut app = AppState::new();
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));
        let left = app.workspace.focused;
        let right = app.workspace.split_focused(Axis::Horizontal).unwrap();
        assert_eq!(app.workspace.focused, right);

        handle_mouse_event(&mut app, left_click(3, 2));

        assert_eq!(app.workspace.focused, right);
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .cursor_position(),
            Position::zero()
        );
        assert_eq!(left, WindowId(1));
    }

    #[test]
    fn mouse_click_focuses_window_when_enabled() {
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));
        let left = app.workspace.focused;
        let right = app.workspace.split_focused(Axis::Horizontal).unwrap();
        assert_eq!(app.workspace.focused, right);

        handle_mouse_event(&mut app, left_click(3, 2));

        assert_eq!(app.workspace.focused, left);
    }

    #[test]
    fn mouse_body_click_moves_cursor_when_enabled() {
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));
        app.handle_text_input('a');
        app.handle_text_input('b');
        app.handle_text_input('c');
        app.handle_text_input('d');

        handle_mouse_event(&mut app, left_click(5, 2));

        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .cursor_position(),
            Position::new(0, 2)
        );
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
    fn window_focus_and_resize_commands_report_status() {
        let mut app = AppState::new();
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));

        app.handle_command(&EditorCommand::Window(WindowCommand::FocusLeft));
        assert_eq!(
            app.status_message,
            Some("Focus left failed: no neighboring pane".to_string())
        );

        app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
        assert_eq!(app.status_message, Some("Split horizontally".to_string()));

        let right = app.workspace.focused;
        app.handle_command(&EditorCommand::Window(WindowCommand::FocusLeft));
        assert_ne!(app.workspace.focused, right);
        assert_eq!(app.status_message, Some("Focused left".to_string()));

        app.handle_command(&EditorCommand::Window(WindowCommand::ResizeDown));
        assert_eq!(
            app.status_message,
            Some("Resize down failed: no matching split".to_string())
        );

        app.handle_command(&EditorCommand::Window(WindowCommand::ResizeRight));
        assert_eq!(app.status_message, Some("Resized right".to_string()));
    }

    #[test]
    fn window_layout_commands_report_status() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::Window(WindowCommand::RotateSplit));
        assert_eq!(
            app.status_message,
            Some("Rotate split failed: no matching split".to_string())
        );

        app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
        app.handle_command(&EditorCommand::Window(WindowCommand::RotateSplit));
        assert_eq!(
            app.status_message,
            Some("Rotated focused split to vertical".to_string())
        );

        app.handle_command(&EditorCommand::Window(WindowCommand::ToggleCollapse));
        assert!(app.workspace.focused_window().unwrap().collapsed);
        assert_eq!(app.status_message, Some("Collapsed pane".to_string()));

        app.handle_command(&EditorCommand::Window(WindowCommand::Expand));
        assert!(!app.workspace.focused_window().unwrap().collapsed);
        assert_eq!(app.status_message, Some("Expanded pane".to_string()));

        app.handle_command(&EditorCommand::Window(WindowCommand::Equalize));
        assert_eq!(app.status_message, Some("Equalized splits".to_string()));

        app.handle_command(&EditorCommand::Window(WindowCommand::Only));
        assert_eq!(
            app.status_message,
            Some("Only window is not implemented yet".to_string())
        );
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
        assert_eq!(app.status_message, Some("Closed window".to_string()));
    }

    #[test]
    fn window_close_reports_last_window_failure() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::Window(WindowCommand::Close));

        assert_eq!(app.workspace.window_count(), 1);
        assert_eq!(
            app.status_message,
            Some("Close failed: cannot close the last window".to_string())
        );
    }

    #[test]
    fn command_line_runs_window_command_ids() {
        let mut app = AppState::new();
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));

        submit_command_line(&mut app, "window.split_horizontal");
        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(app.status_message, Some("Split horizontally".to_string()));

        let right = app.workspace.focused;
        submit_command_line(&mut app, "window.focus_left");
        assert_ne!(app.workspace.focused, right);
        assert_eq!(app.status_message, Some("Focused left".to_string()));

        submit_command_line(&mut app, "window.resize_down extra");
        assert_eq!(
            app.status_message,
            Some("Command failed: window.resize_down expects no arguments".to_string())
        );
    }

    #[test]
    fn help_command_opens_read_only_help_window_once() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::Help));

        let help_window = app.workspace.focused_window().unwrap();
        let help_window_id = help_window.id;
        let help_buffer_id = help_window.buffer_id;
        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(help_window.title, "Help");
        assert_eq!(help_window.kind, WindowKind::Help);
        assert_eq!(help_window.buffer_kind, BufferKind::ReadOnly);

        let help_buffer = app.buffer_state(help_buffer_id).unwrap();
        assert!(help_buffer.buffer.is_read_only());
        assert!(help_buffer.buffer.to_text().contains("Ctrl+G"));
        assert_eq!(app.status_message, Some("Help".to_string()));

        app.handle_command(&EditorCommand::App(AppCommand::Help));

        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(app.workspace.focused, help_window_id);

        app.handle_command(&EditorCommand::Window(WindowCommand::Close));
        assert_eq!(app.workspace.window_count(), 1);
        assert!(app.buffer_state(help_buffer_id).is_none());
    }

    #[test]
    fn f1_key_opens_help_screen() {
        let mut app = AppState::new();

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(1), CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::Help
        );
    }

    #[test]
    fn configured_help_binding_replaces_default_runtime_binding() {
        let config = parse_config("key.app.help = F10").unwrap();
        let mut app = AppState::from_config(config);

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(1), CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.workspace.window_count(), 1);

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(10), CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::Help
        );
    }

    #[test]
    fn configured_disabled_keybinding_is_not_dispatched() {
        let config = parse_config("key.app.help = none").unwrap();
        let mut app = AppState::from_config(config);

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(1), CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.workspace.window_count(), 1);
    }

    #[test]
    fn help_screen_lists_configured_keybindings() {
        let config = parse_config(
            "\
key.app.help = F10
key.edit.go_to_line = F9
key.window.close = none
",
        )
        .unwrap();
        let mut app = AppState::from_config(config);

        app.handle_command(&EditorCommand::App(AppCommand::Help));

        let help_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
        let text = app.buffer_state(help_buffer_id).unwrap().buffer.to_text();
        assert!(text.contains("F10"));
        assert!(text.contains("Help [app.help]"));
        assert!(text.contains("F9"));
        assert!(text.contains("Go to line [edit.go_to_line]"));
        assert!(text.contains("(unbound)"));
        assert!(text.contains("Close focused window [window.close]"));
        assert!(!text.contains("Ctrl+G"));
    }

    #[test]
    fn config_diagnostics_command_opens_read_only_window_once() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::ConfigDiagnostics));

        let config_window = app.workspace.focused_window().unwrap();
        let config_window_id = config_window.id;
        let config_buffer_id = config_window.buffer_id;
        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(config_window.title, "Config Diagnostics");
        assert_eq!(config_window.kind, WindowKind::ConfigDiagnostics);
        assert_eq!(config_window.buffer_kind, BufferKind::ReadOnly);

        let config_buffer = app.buffer_state(config_buffer_id).unwrap();
        let text = config_buffer.buffer.to_text();
        assert!(config_buffer.buffer.is_read_only());
        assert!(text.contains("Dun Config Diagnostics"));
        assert!(text.contains("active: disabled (--no-config)"));
        assert!(text.contains("theme:"));
        assert!(text.contains("mouse: disabled"));
        assert!(text.contains("app.config_diagnostics"));
        assert!(text.contains("F6"));
        assert_eq!(app.status_message, Some("Config diagnostics".to_string()));

        app.handle_command(&EditorCommand::App(AppCommand::ConfigDiagnostics));

        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(app.workspace.focused, config_window_id);

        app.handle_command(&EditorCommand::Window(WindowCommand::Close));
        assert_eq!(app.workspace.window_count(), 1);
        assert!(app.buffer_state(config_buffer_id).is_none());
    }

    #[test]
    fn f6_key_opens_config_diagnostics_screen() {
        let mut app = AppState::new();

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(6), CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::ConfigDiagnostics
        );
    }

    #[test]
    fn command_line_prompt_dispatches_app_commands() {
        let mut app = AppState::new();

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('p'), CrosstermKeyModifiers::CONTROL),
        );
        assert_eq!(app.prompt_status_text(), Some("Command: ".to_string()));

        send_text(&mut app, "help");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::Help
        );

        submit_command_line(&mut app, "config");
        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::ConfigDiagnostics
        );
    }

    #[test]
    fn command_line_history_navigates_recent_commands_and_restores_draft() {
        let mut app = AppState::new();

        submit_command_line(&mut app, "commands");
        submit_command_line(&mut app, "theme");

        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        send_text(&mut app, "draft");

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(app.prompt_status_text(), Some("Command: theme".to_string()));

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Command: commands".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Command: commands".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(app.prompt_status_text(), Some("Command: theme".to_string()));

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(app.prompt_status_text(), Some("Command: draft".to_string()));
    }

    #[test]
    fn command_line_history_repeats_previous_command() {
        let mut app = AppState::new();

        submit_command_line(&mut app, "commands");
        app.set_status("cleared");

        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.status_message, Some(COMMAND_LINE_HELP.to_string()));
        assert_eq!(app.command_history, vec!["commands".to_string()]);
    }

    #[test]
    fn command_line_history_is_capped_and_skips_consecutive_duplicates() {
        let mut app = AppState::new();

        for index in 0..(COMMAND_HISTORY_LIMIT + 2) {
            app.record_command_history(format!("cmd-{index}"));
        }

        assert_eq!(app.command_history.len(), COMMAND_HISTORY_LIMIT);
        assert_eq!(app.command_history.first(), Some(&"cmd-2".to_string()));
        assert_eq!(
            app.command_history.last(),
            Some(&format!("cmd-{}", COMMAND_HISTORY_LIMIT + 1))
        );

        let last = app.command_history.last().cloned().unwrap();
        app.record_command_history(last);
        assert_eq!(app.command_history.len(), COMMAND_HISTORY_LIMIT);
    }

    #[test]
    fn command_line_history_does_not_affect_other_prompts() {
        let mut app = AppState::new();
        app.record_command_history("theme".to_string());

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, "path");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.prompt_status_text(), Some("Open: path".to_string()));
    }

    #[test]
    fn command_line_theme_switches_runtime_theme_and_refreshes_diagnostics() {
        let mut app = AppState::new();
        app.shell.profile = TerminalProfile::utf8_256();

        app.handle_command(&EditorCommand::App(AppCommand::ConfigDiagnostics));
        let diagnostics_buffer_id = app.workspace.focused_window().unwrap().buffer_id;

        submit_command_line(&mut app, "theme dark");

        assert_eq!(app.shell.theme.theme, ThemeName::Dark);
        assert_eq!(app.status_message, Some("Theme: dark".to_string()));
        assert!(
            app.buffer_state(diagnostics_buffer_id)
                .unwrap()
                .buffer
                .to_text()
                .contains("theme: dark")
        );

        submit_command_line(&mut app, "theme");

        assert!(
            app.status_message
                .as_ref()
                .is_some_and(|message| message.starts_with("Theme: dark"))
        );
    }

    #[test]
    fn command_line_theme_reports_unknown_theme() {
        let mut app = AppState::new();
        let original_theme = app.shell.theme.theme;

        submit_command_line(&mut app, "theme unknown");

        assert_eq!(app.shell.theme.theme, original_theme);
        assert_eq!(
            app.status_message,
            Some("Theme failed: unknown theme unknown; expected msedit|turbo|dark|dun".to_string())
        );
    }

    #[test]
    fn reload_config_restores_configured_theme_after_runtime_theme_switch() {
        let path = temp_file_path("theme-reload-config");
        std::fs::write(&path, "theme = turbo\n").unwrap();
        let mut app = app_from_config_path(path.clone());
        app.detected_profile = TerminalProfile::utf8_256();

        submit_command_line(&mut app, "reload-config");
        assert_eq!(app.shell.theme.theme, ThemeName::Turbo);

        submit_command_line(&mut app, "theme dark");
        assert_eq!(app.shell.theme.theme, ThemeName::Dark);

        submit_command_line(&mut app, "reload-config");
        assert_eq!(app.shell.theme.theme, ThemeName::Turbo);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn command_line_runs_file_commands_with_quoted_paths() {
        let save_path = temp_file_path("command save.txt");
        let open_path = temp_file_path("command open.txt");
        std::fs::write(&open_path, "opened").unwrap();
        let mut app = AppState::new();

        app.handle_text_input('x');
        submit_command_line(&mut app, &format!("save-as \"{}\"", save_path.display()));

        assert_eq!(std::fs::read_to_string(&save_path).unwrap(), "x");
        assert_eq!(
            app.status_message,
            Some(format!("Saved {}", save_path.display()))
        );

        submit_command_line(&mut app, "new");
        submit_command_line(&mut app, &format!("open \"{}\"", open_path.display()));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "opened");
        assert_eq!(state.path.as_ref(), Some(&open_path));

        let _ = std::fs::remove_file(save_path);
        let _ = std::fs::remove_file(open_path);
    }

    #[test]
    fn command_line_open_path_refuses_dirty_focused_buffer() {
        let path = temp_file_path("command-open-dirty.txt");
        std::fs::write(&path, "opened").unwrap();
        let mut app = AppState::new();
        app.handle_text_input('x');

        submit_command_line(&mut app, &format!("open {}", path.display()));

        assert_eq!(
            app.status_message,
            Some("Open failed: focused buffer has unsaved changes".to_string())
        );
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "x");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn command_line_reports_unknown_and_parse_errors() {
        let mut app = AppState::new();

        submit_command_line(&mut app, "wat");
        assert_eq!(app.status_message, Some("Unknown command: wat".to_string()));

        submit_command_line(&mut app, "open \"unterminated");
        assert_eq!(
            app.status_message,
            Some("Command failed: unclosed quote".to_string())
        );
    }

    #[test]
    fn command_line_parser_handles_quotes_and_escapes() {
        assert_eq!(
            parse_command_line("open \"a b\" save\\ path").unwrap(),
            vec![
                "open".to_string(),
                "a b".to_string(),
                "save path".to_string()
            ]
        );
        assert_eq!(
            parse_command_line("replace one \"\"").unwrap(),
            vec!["replace".to_string(), "one".to_string(), String::new()]
        );
        assert_eq!(
            parse_command_line("open \"unterminated"),
            Err(CommandLineParseError::UnclosedQuote)
        );
        assert_eq!(
            parse_command_line("open path\\"),
            Err(CommandLineParseError::TrailingEscape)
        );
    }

    #[test]
    fn reload_config_applies_updated_keymap_and_limits_without_resetting_buffers() {
        let path = temp_file_path("reload-config");
        std::fs::write(&path, "limits.editable_file_soft_limit_bytes = 4 KiB\n").unwrap();
        let mut app = app_from_config_path(path.clone());
        app.handle_text_input('x');

        std::fs::write(
            &path,
            "\
limits.editable_file_soft_limit_bytes = 8 KiB
mouse.enabled = true
key.app.help = F10
",
        )
        .unwrap();

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(5), CrosstermKeyModifiers::NONE),
        );

        assert_eq!(
            app.status_message,
            Some(format!("Config reloaded from {}", path.display()))
        );
        assert_eq!(app.limits.editable_file_soft_limit_bytes, 8 * 1024);
        assert!(app.mouse_enabled());
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "x");

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(1), CrosstermKeyModifiers::NONE),
        );
        assert_eq!(app.workspace.window_count(), 1);

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(10), CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::Help
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reload_config_refreshes_open_config_diagnostics_screen() {
        let path = temp_file_path("reload-config-diagnostics");
        std::fs::write(&path, "\n").unwrap();
        let mut app = app_from_config_path(path.clone());

        app.handle_command(&EditorCommand::App(AppCommand::ConfigDiagnostics));
        let config_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
        let text = app.buffer_state(config_buffer_id).unwrap().buffer.to_text();
        assert!(keymap_command_line(&text, "app.help").contains("F1"));

        std::fs::write(&path, "key.app.help = F10\n").unwrap();
        app.handle_command(&EditorCommand::App(AppCommand::ReloadConfig));

        let text = app.buffer_state(config_buffer_id).unwrap().buffer.to_text();
        let line = keymap_command_line(&text, "app.help");
        assert!(line.contains("F10"));
        assert!(!line.contains("F1 "));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reload_config_failure_keeps_previous_keymap() {
        let path = temp_file_path("bad-reload-config");
        std::fs::write(&path, "key.app.help = F10\n").unwrap();
        let mut app = app_from_config_path(path.clone());

        std::fs::write(&path, "bad = value\n").unwrap();
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(5), CrosstermKeyModifiers::NONE),
        );

        assert!(
            app.status_message
                .as_ref()
                .is_some_and(|message| message.starts_with("Config reload failed:"))
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(1), CrosstermKeyModifiers::NONE),
        );
        assert_eq!(app.workspace.window_count(), 1);

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(10), CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::Help
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reload_config_refreshes_open_help_screen() {
        let path = temp_file_path("reload-help-config");
        std::fs::write(&path, "\n").unwrap();
        let mut app = app_from_config_path(path.clone());

        app.handle_command(&EditorCommand::App(AppCommand::Help));
        let help_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
        let text = app.buffer_state(help_buffer_id).unwrap().buffer.to_text();
        assert!(help_command_line(&text).contains("F1"));

        std::fs::write(&path, "key.app.help = F10\n").unwrap();
        app.handle_command(&EditorCommand::App(AppCommand::ReloadConfig));

        let text = app.buffer_state(help_buffer_id).unwrap().buffer.to_text();
        let line = help_command_line(&text);
        assert!(line.contains("F10"));
        assert!(!line.contains("F1 "));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn status_history_command_opens_read_only_status_window_once() {
        let mut app = AppState::new();
        app.set_status("Opened sample.txt");
        app.set_status("Save failed: disk full");

        app.handle_command(&EditorCommand::App(AppCommand::StatusHistory));

        let status_window = app.workspace.focused_window().unwrap();
        let status_window_id = status_window.id;
        let status_buffer_id = status_window.buffer_id;
        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(status_window.title, "Status History");
        assert_eq!(status_window.kind, WindowKind::StatusHistory);
        assert_eq!(status_window.buffer_kind, BufferKind::ReadOnly);

        let status_buffer = app.buffer_state(status_buffer_id).unwrap();
        let text = status_buffer.buffer.to_text();
        assert!(status_buffer.buffer.is_read_only());
        assert!(text.contains("[info] Opened sample.txt"));
        assert!(text.contains("[error] Save failed: disk full"));
        assert!(text.contains("[info] Status history"));

        app.handle_command(&EditorCommand::App(AppCommand::StatusHistory));

        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(app.workspace.focused, status_window_id);

        app.handle_command(&EditorCommand::Window(WindowCommand::Close));
        assert_eq!(app.workspace.window_count(), 1);
        assert!(app.buffer_state(status_buffer_id).is_none());
    }

    #[test]
    fn status_history_window_refreshes_when_status_changes() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::StatusHistory));
        let status_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
        assert!(
            !app.buffer_state(status_buffer_id)
                .unwrap()
                .buffer
                .to_text()
                .contains("Later")
        );

        app.set_status("Later");

        assert!(
            app.buffer_state(status_buffer_id)
                .unwrap()
                .buffer
                .to_text()
                .contains("[info] Later")
        );
    }

    #[test]
    fn status_history_is_capped_to_recent_entries() {
        let mut app = AppState::new();

        for index in 0..(STATUS_HISTORY_LIMIT + 2) {
            app.set_status(format!("message {index}"));
        }

        assert_eq!(app.status_history.len(), STATUS_HISTORY_LIMIT);
        assert_eq!(app.status_history[0].message, "message 2");
        assert_eq!(
            app.status_history[STATUS_HISTORY_LIMIT - 1].message,
            format!("message {}", STATUS_HISTORY_LIMIT + 1)
        );
    }

    #[test]
    fn f2_key_opens_status_history() {
        let mut app = AppState::new();

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(2), CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::StatusHistory
        );
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
    fn from_path_opens_utf8_file_path() {
        let path = temp_file_path("open.txt");
        std::fs::write(&path, "one\r\ntwo").unwrap();

        let app = AppState::from_path(Some(path.clone())).unwrap();
        let state = app.buffer_state(BufferId(1)).unwrap();

        assert_eq!(state.path.as_ref(), Some(&path));
        assert_eq!(state.encoding, FileTextEncoding::Utf8);
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
    fn stable_file_read_validation_accepts_unchanged_file() {
        let path = temp_file_path("stable-read.txt");
        std::fs::write(&path, "stable").unwrap();
        let metadata = std::fs::metadata(&path).unwrap();
        let snapshot = FileReadSnapshot::from_metadata(&metadata);

        validate_stable_file_read(&path, snapshot, metadata.len()).unwrap();

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn stable_file_read_validation_rejects_truncated_file() {
        let path = temp_file_path("truncated-read.txt");
        std::fs::write(&path, "stable").unwrap();
        let metadata = std::fs::metadata(&path).unwrap();
        let snapshot = FileReadSnapshot::from_metadata(&metadata);
        std::fs::write(&path, "x").unwrap();

        let error = validate_stable_file_read(&path, snapshot, metadata.len()).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "file changed while reading; retry open");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn stable_file_read_validation_rejects_deleted_file() {
        let path = temp_file_path("deleted-read.txt");
        std::fs::write(&path, "stable").unwrap();
        let metadata = std::fs::metadata(&path).unwrap();
        let snapshot = FileReadSnapshot::from_metadata(&metadata);
        std::fs::remove_file(&path).unwrap();

        let error = validate_stable_file_read(&path, snapshot, metadata.len()).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "file changed while reading; retry open");
    }

    #[cfg(unix)]
    #[test]
    fn stable_file_read_validation_rejects_same_size_replacement() {
        let path = temp_file_path("replaced-read.txt");
        let replacement = temp_file_path("replaced-read-next.txt");
        std::fs::write(&path, "aaaa").unwrap();
        let metadata = std::fs::metadata(&path).unwrap();
        let snapshot = FileReadSnapshot::from_metadata(&metadata);
        std::fs::write(&replacement, "bbbb").unwrap();
        std::fs::rename(&replacement, &path).unwrap();

        let error = validate_stable_file_read(&path, snapshot, metadata.len()).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "file changed while reading; retry open");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn invalid_utf8_file_path_opens_read_only_fallback() {
        let path = temp_file_path("invalid.txt");
        std::fs::write(&path, [b'o', b'k', 0xff, b'\n', b'\\', b'\t', 0xe4]).unwrap();

        let app = AppState::from_path(Some(path.clone())).unwrap();
        let state = app.buffer_state(BufferId(1)).unwrap();
        let window = app.workspace.focused_window().unwrap();

        assert!(state.buffer.is_read_only());
        assert_eq!(state.buffer.kind(), BufferKind::ReadOnly);
        assert_eq!(state.encoding, FileTextEncoding::EscapedBytes);
        assert_eq!(state.buffer.to_text(), "ok\\xFF\n\\\\\\x09\\xE4");
        assert_eq!(state.path.as_ref(), Some(&path));
        assert_eq!(window.buffer_kind, BufferKind::ReadOnly);
        assert_eq!(
            app.focused_buffer_status(),
            format!("{} [readonly] [escaped]", title_for_path(&path))
        );
        assert!(app.focused_detail_status().contains("Escaped bytes"));
        assert_eq!(
            app.status_message,
            Some(format!(
                "Opened {} read-only: non-UTF-8 bytes shown as escapes",
                path.display()
            ))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn invalid_utf8_fallback_preserves_valid_unicode_segments() {
        let path = temp_file_path("invalid-with-unicode.txt");
        std::fs::write(&path, [b'a', 0xe4, 0xb8, 0xad, 0xff, b'b']).unwrap();

        let app = AppState::from_path(Some(path.clone())).unwrap();
        let state = app.buffer_state(BufferId(1)).unwrap();

        assert!(state.buffer.is_read_only());
        assert_eq!(state.encoding, FileTextEncoding::EscapedBytes);
        assert_eq!(state.buffer.to_text(), "a中\\xFFb");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_rejects_read_only_invalid_utf8_fallback() {
        let path = temp_file_path("invalid-save.txt");
        std::fs::write(&path, [0xff, b'a']).unwrap();
        let mut app = AppState::from_path(Some(path.clone())).unwrap();

        app.handle_command(&EditorCommand::File(FileCommand::Save));

        assert_eq!(
            app.status_message,
            Some("Save failed: focused buffer is read-only".to_string())
        );
        assert_eq!(std::fs::read(&path).unwrap(), vec![0xff, b'a']);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_as_rejects_read_only_invalid_utf8_fallback() {
        let path = temp_file_path("invalid-save-as.txt");
        let target = temp_file_path("invalid-save-as-target.txt");
        std::fs::write(&path, [0xff, b'a']).unwrap();
        let mut app = AppState::from_path(Some(path.clone())).unwrap();

        app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
        send_text(&mut app, &target.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(
            app.status_message,
            Some("Save As failed: focused buffer is read-only".to_string())
        );
        assert!(!target.exists());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn file_at_editable_soft_limit_is_accepted() {
        let path = temp_file_path("soft-limit-ok.txt");
        std::fs::write(&path, "abcd").unwrap();
        let config = config_with_editable_file_soft_limit(4);

        let app = AppState::from_config_path(config, Some(path.clone())).unwrap();

        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "abcd"
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn file_over_editable_soft_limit_is_rejected_before_editing() {
        let path = temp_file_path("soft-limit-large.txt");
        std::fs::write(&path, "abcd").unwrap();
        let config = config_with_editable_file_soft_limit(3);

        let error = match AppState::from_config_path(config, Some(path.clone())) {
            Ok(_) => panic!("file above editable soft limit should be rejected"),
            Err(error) => error,
        };

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("too large for editable mode"));
        assert!(error.to_string().contains("3 byte soft limit"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn open_prompt_reports_file_over_editable_soft_limit() {
        let path = temp_file_path("prompt-soft-limit-large.txt");
        std::fs::write(&path, "abcd").unwrap();
        let mut app = AppState::from_config(config_with_editable_file_soft_limit(3));

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "");
        assert_eq!(state.path, None);
        let status = app.status_message.as_deref().unwrap_or_default();
        assert!(status.starts_with("Open failed: "));
        assert!(status.contains("too large for editable mode"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn open_prompt_reports_missing_file_with_path() {
        let path = temp_file_path("missing-open.txt");
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(
            app.status_message,
            Some(format!("Open failed: {}: not found", path.display()))
        );
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");
    }

    #[test]
    fn open_prompt_reports_directory_path() {
        let path = temp_file_path("open-dir");
        std::fs::create_dir(&path).unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(
            app.status_message,
            Some(format!(
                "Open failed: {}: path is a directory",
                path.display()
            ))
        );
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");

        let _ = std::fs::remove_dir(path);
    }

    #[test]
    fn save_command_writes_focused_file_buffer() {
        let path = temp_file_path("save.txt");
        std::fs::write(&path, "old").unwrap();
        let mut app = AppState::from_path(Some(path.clone())).unwrap();

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
    fn save_command_cleans_atomic_temp_file() {
        let path = temp_file_path("atomic-save.txt");
        std::fs::write(&path, "old").unwrap();
        let mut app = AppState::from_path(Some(path.clone())).unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
        app.handle_text_input('!');
        app.handle_command(&EditorCommand::File(FileCommand::Save));

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old!");
        assert!(atomic_temp_files_for(&path).is_empty());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn open_cleans_stale_atomic_save_temp_file() {
        let path = temp_file_path("stale-open-cleanup.txt");
        let stale_temp = write_atomic_temp_file_for(&path, 0, "stale");
        std::fs::write(&path, "current").unwrap();

        let app = AppState::from_path(Some(path.clone())).unwrap();

        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "current"
        );
        assert!(!stale_temp.exists());
        assert_eq!(
            app.status_message,
            Some(format!(
                "Opened {}; cleaned 1 stale save temp file(s)",
                path.display()
            ))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn open_reports_newer_atomic_save_recovery_temp_file() {
        let path = temp_file_path("recovery-open-warning.txt");
        std::fs::write(&path, "current").unwrap();
        let recovery_temp = write_newer_atomic_temp_file_for(&path, "recovered");

        let app = AppState::from_path(Some(path.clone())).unwrap();

        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "current"
        );
        assert!(recovery_temp.exists());
        let status = app.status_message.as_deref().unwrap_or_default();
        assert!(status.starts_with(&format!(
            "Opened {}; recovery temp file found: ",
            path.display()
        )));
        assert!(status.contains(&recovery_temp.display().to_string()));

        let _ = std::fs::remove_file(recovery_temp);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_cleans_stale_atomic_save_temp_file() {
        let path = temp_file_path("stale-save-cleanup.txt");
        std::fs::write(&path, "old").unwrap();
        let mut app = AppState::from_path(Some(path.clone())).unwrap();
        let stale_temp = write_atomic_temp_file_for(&path, 0, "stale");
        std::fs::write(&path, "external change").unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
        app.handle_text_input('!');
        app.handle_command(&EditorCommand::File(FileCommand::Save));

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old!");
        assert!(!stale_temp.exists());
        assert_eq!(
            app.status_message,
            Some(format!(
                "Saved {}; cleaned 1 stale save temp file(s)",
                path.display()
            ))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_preserves_newer_atomic_save_recovery_temp_file() {
        let path = temp_file_path("recovery-save-warning.txt");
        std::fs::write(&path, "old").unwrap();
        let mut app = AppState::from_path(Some(path.clone())).unwrap();
        let recovery_temp = write_newer_atomic_temp_file_for(&path, "recovered");

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
        app.handle_text_input('!');
        app.handle_command(&EditorCommand::File(FileCommand::Save));

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old!");
        assert!(recovery_temp.exists());
        let status = app.status_message.as_deref().unwrap_or_default();
        assert!(status.starts_with(&format!(
            "Saved {}; recovery temp file found: ",
            path.display()
        )));
        assert!(status.contains(&recovery_temp.display().to_string()));

        let _ = std::fs::remove_file(recovery_temp);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_rejects_read_only_target_without_replacing_it() {
        let path = temp_file_path("readonly-save.txt");
        std::fs::write(&path, "old").unwrap();
        let mut app = AppState::from_path(Some(path.clone())).unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
        app.handle_text_input('!');
        set_path_readonly(&path, true);

        app.handle_command(&EditorCommand::File(FileCommand::Save));

        set_path_readonly(&path, false);
        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old");
        assert!(state.buffer.is_dirty());
        assert_eq!(
            app.status_message,
            Some(format!(
                "Save failed: {}: destination is read-only",
                path.display()
            ))
        );
        assert!(atomic_temp_files_for(&path).is_empty());

        let _ = std::fs::remove_file(path);
    }

    #[cfg(unix)]
    #[test]
    fn save_through_symlink_preserves_link_and_updates_target() {
        let target = temp_file_path("atomic-symlink-target.txt");
        let link = temp_file_path("atomic-symlink-link.txt");
        std::fs::write(&target, "old").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let mut app = AppState::from_path(Some(link.clone())).unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
        app.handle_text_input('!');
        app.handle_command(&EditorCommand::File(FileCommand::Save));

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "old!");
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            app.status_message,
            Some(format!("Saved {}", link.display()))
        );

        let _ = std::fs::remove_file(link);
        let _ = std::fs::remove_file(target);
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
        let mut app = AppState::from_path(Some(path.clone())).unwrap();

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
    fn save_as_reports_missing_parent_directory() {
        let parent = temp_file_path("missing-save-parent");
        let path = parent.join("out.txt");
        let mut app = AppState::new();
        app.handle_text_input('x');

        app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(
            app.status_message,
            Some(format!(
                "Save As failed: {}: parent directory does not exist",
                path.display()
            ))
        );
        assert!(state.buffer.is_dirty());
        assert_eq!(state.path, None);
        assert!(!path.exists());
    }

    #[test]
    fn save_as_reports_directory_destination() {
        let path = temp_file_path("save-as-dir");
        std::fs::create_dir(&path).unwrap();
        let mut app = AppState::new();
        app.handle_text_input('x');

        app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(
            app.status_message,
            Some(format!(
                "Save As failed: {}: destination is a directory",
                path.display()
            ))
        );
        assert!(state.buffer.is_dirty());
        assert_eq!(state.path, None);

        let _ = std::fs::remove_dir(path);
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
    fn replace_command_prompts_and_replaces_next_match() {
        let mut app = app_with_text("one two one");

        app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
        assert_eq!(app.prompt_status_text(), Some("Replace Find: ".to_string()));

        send_text(&mut app, "one");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(app.prompt_status_text(), Some("Replace With: ".to_string()));

        send_text(&mut app, "uno");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "uno two one");
        assert!(state.buffer.is_dirty());
        assert_eq!(app.last_find_query, Some("one".to_string()));
        assert_eq!(app.status_message, Some("Replace: 1/2 one".to_string()));
    }

    #[test]
    fn replace_prefers_current_selected_match() {
        let mut app = app_with_text("one two one");
        app.last_find_query = Some("one".to_string());
        app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));
        app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));

        app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
        assert_eq!(
            app.prompt_status_text(),
            Some("Replace Find: one".to_string())
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );
        send_text(&mut app, "uno");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "one two uno");
        assert_eq!(app.status_message, Some("Replace: 2/2 one".to_string()));
    }

    #[test]
    fn replace_accepts_empty_replacement_as_delete() {
        let mut app = app_with_text("one two");

        app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
        send_text(&mut app, "one");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), " two");
        assert_eq!(app.status_message, Some("Replace: 1/1 one".to_string()));
    }

    #[test]
    fn replace_reports_missing_match() {
        let mut app = app_with_text("abc");

        app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
        send_text(&mut app, "z");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );
        send_text(&mut app, "x");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "abc");
        assert_eq!(
            app.status_message,
            Some("Replace: no matches for z".to_string())
        );
        assert_eq!(state.buffer.selection_range(), None);
    }

    #[test]
    fn go_to_line_prompt_moves_cursor_to_requested_line() {
        let mut app = app_with_text("ab\ncd\nef");
        app.buffers[0]
            .buffer
            .set_cursor(Position::new(0, 1))
            .unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::GoToLine));
        assert_eq!(app.prompt_status_text(), Some("Go To Line: ".to_string()));

        send_text(&mut app, "3");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.cursor_position(), Position::new(2, 1));
        assert_eq!(state.buffer.selection_range(), None);
        assert_eq!(app.status_message, Some("Go to line: 3".to_string()));
    }

    #[test]
    fn go_to_line_rejects_invalid_or_out_of_range_input() {
        let mut app = app_with_text("ab\ncd");
        app.buffers[0]
            .buffer
            .set_cursor(Position::new(1, 1))
            .unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::GoToLine));
        send_text(&mut app, "abc");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(
            app.status_message,
            Some("Go to line failed: invalid line number abc".to_string())
        );
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .cursor_position(),
            Position::new(1, 1)
        );

        app.handle_command(&EditorCommand::Edit(EditCommand::GoToLine));
        send_text(&mut app, "9");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(
            app.status_message,
            Some("Go to line failed: line 9 is past end (2 lines)".to_string())
        );
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .cursor_position(),
            Position::new(1, 1)
        );
    }

    #[test]
    fn focused_status_reports_dirty_buffer_name() {
        let mut app = AppState::new();

        app.handle_text_input('x');

        assert_eq!(app.focused_buffer_status(), "Untitled*");
    }

    #[test]
    fn focused_detail_status_reports_position_and_buffer_metadata() {
        let mut app = app_with_text("a\n中x");
        app.shell.profile = TerminalProfile::new(EncodingProfile::Ascii, ColorProfile::Mono);
        app.buffers[0]
            .buffer
            .set_cursor(Position::new(1, "中".len()))
            .unwrap();

        assert_eq!(
            app.focused_detail_status(),
            "Ln 2/2, Col 3 | LF | Text UTF-8 | ASCII/mono | Win 1/1"
        );
    }

    #[test]
    fn focused_detail_status_reports_crlf_and_focused_window_index() {
        let mut app = AppState::new();
        app.shell.profile = TerminalProfile::new(EncodingProfile::Utf8, ColorProfile::Color16);
        app.buffers[0].buffer = TextBuffer::from_text("one\r\ntwo");
        app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));

        assert_eq!(
            app.focused_detail_status(),
            "Ln 1/1, Col 1 | LF | Text UTF-8 | UTF-8/16c | Win 2/2"
        );

        app.workspace.focused = WindowId(1);

        assert_eq!(
            app.focused_detail_status(),
            "Ln 1/2, Col 1 | CRLF | Text UTF-8 | UTF-8/16c | Win 1/2"
        );
    }

    #[test]
    fn focused_detail_status_reports_selection_summary() {
        let mut app = app_with_text("abc def");
        app.shell.profile = TerminalProfile::utf8_256();
        app.buffers[0]
            .buffer
            .select(Position::new(0, 0), Position::new(0, 3))
            .unwrap();

        assert_eq!(
            app.focused_detail_status(),
            "Ln 1/1, Col 4 | Sel 3c | LF | Text UTF-8 | UTF-8/256c | Win 1/1"
        );
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
        let mut app = AppState::from_path(Some(path.clone())).unwrap();

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

    #[test]
    #[ignore]
    fn large_file_perf_baseline_open_search_scroll_and_render() {
        let path = temp_file_path("large-file-perf.log");
        let fixture = write_large_file_perf_fixture(&path, large_file_perf_target_bytes());
        eprintln!(
            "large_file_perf fixture: bytes={} lines={} error_lines={}",
            fixture.bytes, fixture.lines, fixture.error_lines
        );

        let config = config_with_editable_file_soft_limit(fixture.bytes as u64);
        let mut app = measure_large_file_perf("startup_open", || {
            AppState::from_config_path(config, Some(path.clone())).unwrap()
        });
        let buffer_id = app.focused_buffer_id().unwrap();
        let line_count = app.buffer_state(buffer_id).unwrap().buffer.line_count();
        assert_eq!(line_count, fixture.lines);

        let sparse_matches = measure_large_file_perf("find_all_sparse_match", || {
            app.buffer_state(buffer_id)
                .unwrap()
                .buffer
                .find_all("ERROR service=api")
        });
        assert_eq!(sparse_matches.len(), fixture.error_lines);

        let missing_matches = measure_large_file_perf("find_all_missing_match", || {
            app.buffer_state(buffer_id)
                .unwrap()
                .buffer
                .find_all("needle-that-does-not-exist")
        });
        assert!(missing_matches.is_empty());

        let last_line = fixture.lines.saturating_sub(1);
        app.focused_buffer_mut()
            .unwrap()
            .buffer
            .set_cursor(Position::new(last_line, 0))
            .unwrap();
        measure_large_file_perf("sync_view_to_eof", || {
            app.sync_view_for_area(Rect::new(0, 0, 120, 40));
        });
        assert!(app.buffer_state(buffer_id).unwrap().first_line > 0);

        let buffer_views = app.buffer_views();
        let ui_frame = measure_large_file_perf("ui_frame_visible_window", || {
            app.shell
                .frame_for_workspace(&app.workspace, app.workspace_area, &buffer_views)
        });
        assert!(!ui_frame.windows[0].body.is_empty());

        let backend = TestBackend::new(120, 42);
        let mut terminal = Terminal::new(backend).unwrap();
        measure_large_file_perf("ratatui_draw_visible_window", || {
            terminal
                .draw(|frame| app.shell.render(frame, &ui_frame))
                .unwrap();
        });

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    #[ignore]
    fn large_file_perf_long_line_display_cap() {
        let line_bytes = large_line_perf_target_bytes();
        let long_line = "x".repeat(line_bytes);
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, &long_line);
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let workspace = Workspace::new_untitled();
        let shell = UiShell::default();

        let ui_frame = measure_large_file_perf("ui_frame_long_line_display_cap", || {
            shell.frame_for_workspace(&workspace, Rect::new(0, 0, 120, 8), &[buffer_view])
        });

        let line = &ui_frame.windows[0].body[0];
        assert!(line.truncated);
        assert_eq!(
            line.bytes_consumed,
            Limits::default().line_display_soft_limit_bytes
        );

        let backend = TestBackend::new(120, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        measure_large_file_perf("ratatui_draw_long_line_display_cap", || {
            terminal
                .draw(|frame| shell.render(frame, &ui_frame))
                .unwrap();
        });
    }

    fn config_with_editable_file_soft_limit(limit: u64) -> Config {
        Config {
            limits: Limits {
                editable_file_soft_limit_bytes: limit,
                ..Limits::default()
            },
            ..Config::default()
        }
    }

    fn app_from_config_path(path: PathBuf) -> AppState {
        let request = ConfigLoadRequest::explicit(path);
        let loaded_config = load_config(&request).unwrap();
        AppState::from_loaded_config(request, loaded_config)
    }

    fn help_command_line(text: &str) -> &str {
        text.lines()
            .find(|line| line.contains("Help [app.help]"))
            .expect("help command line should be present")
    }

    fn keymap_command_line<'a>(text: &'a str, command: &str) -> &'a str {
        text.lines()
            .find(|line| line.contains(command))
            .expect("keymap command line should be present")
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

    #[derive(Clone, Copy, Debug)]
    struct LargeFilePerfFixture {
        bytes: usize,
        lines: usize,
        error_lines: usize,
    }

    fn large_file_perf_target_bytes() -> usize {
        perf_env_usize("DUN_PERF_LARGE_FILE_BYTES").unwrap_or(8 * 1024 * 1024)
    }

    fn large_line_perf_target_bytes() -> usize {
        perf_env_usize("DUN_PERF_LONG_LINE_BYTES").unwrap_or(512 * 1024)
    }

    fn perf_env_usize(name: &str) -> Option<usize> {
        let value = std::env::var(name).ok()?.parse().ok()?;
        (value > 0).then_some(value)
    }

    fn write_large_file_perf_fixture(path: &Path, target_bytes: usize) -> LargeFilePerfFixture {
        let mut file = std::fs::File::create(path).unwrap();
        let mut bytes = 0;
        let mut lines = 0;
        let mut error_lines = 0;

        while bytes < target_bytes {
            if lines > 0 {
                file.write_all(b"\n").unwrap();
                bytes += 1;
            }

            let line = if lines % 257 == 0 {
                error_lines += 1;
                format!(
                    "ERROR service=api shard={:04} request_id={:08x} message=slow backend response",
                    lines % 4096,
                    lines
                )
            } else {
                format!(
                    "INFO service=api shard={:04} request_id={:08x} message=heartbeat ok",
                    lines % 4096,
                    lines
                )
            };
            file.write_all(line.as_bytes()).unwrap();
            bytes += line.len();
            lines += 1;
        }

        LargeFilePerfFixture {
            bytes,
            lines,
            error_lines,
        }
    }

    fn measure_large_file_perf<T>(label: &str, action: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let output = action();
        let elapsed = started.elapsed();
        eprintln!("large_file_perf {label}: {} ms", elapsed.as_millis());
        output
    }

    fn atomic_temp_files_for(path: &Path) -> Vec<PathBuf> {
        let directory = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let file_name = path.file_name().unwrap_or_default();
        let mut prefix = OsString::from(".");
        prefix.push(file_name);
        prefix.push(".dun-save-");
        let prefix = prefix.to_string_lossy();

        std::fs::read_dir(directory)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(&*prefix))
            .map(|entry| entry.path())
            .collect()
    }

    fn write_atomic_temp_file_for(path: &Path, attempt: u32, contents: &str) -> PathBuf {
        let directory = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let file_name = path.file_name().unwrap_or_default();
        let temp_path = atomic_temp_path(directory, file_name, attempt);
        std::fs::write(&temp_path, contents).unwrap();
        temp_path
    }

    fn write_newer_atomic_temp_file_for(path: &Path, contents: &str) -> PathBuf {
        for attempt in 0..100 {
            std::thread::sleep(Duration::from_millis(10));
            let temp_path = write_atomic_temp_file_for(path, attempt, contents);
            if file_modified(&temp_path) > file_modified(path) {
                return temp_path;
            }
            let _ = std::fs::remove_file(&temp_path);
        }

        panic!("could not create atomic temp file newer than destination");
    }

    fn file_modified(path: &Path) -> std::time::SystemTime {
        std::fs::metadata(path).unwrap().modified().unwrap()
    }

    fn set_path_readonly(path: &Path, readonly: bool) {
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_readonly(readonly);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    fn send_text(app: &mut AppState, text: &str) {
        for ch in text.chars() {
            handle_key_event(
                app,
                CrosstermKeyEvent::new(CrosstermKeyCode::Char(ch), CrosstermKeyModifiers::NONE),
            );
        }
    }

    fn submit_command_line(app: &mut AppState, text: &str) {
        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        send_text(app, text);
        handle_key_event(
            app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );
    }

    fn app_with_text(text: &str) -> AppState {
        let mut app = AppState::new();
        app.buffers[0].buffer =
            TextBuffer::from_text_with_kind(dun_core::BufferKind::Untitled, text);
        app
    }
}
