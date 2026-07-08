#![forbid(unsafe_code)]

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Read, Stdout, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, ExitStatus, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent,
    KeyEventKind as CrosstermKeyEventKind, KeyModifiers as CrosstermKeyModifiers,
    MouseButton as CrosstermMouseButton, MouseEvent as CrosstermMouseEvent,
    MouseEventKind as CrosstermMouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use dun_config::{
    ClipboardConfig, Config, FileDialogAction, FileDialogKeymap, Key, KeyModifiers, KeySequence,
    KeyStroke, Keymap, Limits, ThemeName, command_from_id, command_id, default_config_text,
    file_dialog_action_id, parse_config,
};
use dun_core::{
    AppCommand, Axis, BufferError, BufferId, BufferKind, Direction, EditCommand, EditorCommand,
    FileCommand, FileTextEncoding, LineEnding, Position, Rect, SearchMatch, SearchOptions,
    Selection, SplitDragHandle, TextBuffer, WindowCommand, WindowId, WindowKind, WindowState,
    Workspace, WorkspaceError, decode_file_text,
};
use dun_term::{ColorProfile, EncodingProfile, TerminalProfile, Theme};
use dun_ui::{BufferView, MenuSelection, UiMouseTarget, UiOverlay, UiShell};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const EXIT_RUNTIME_ERROR: u8 = 1;
const EXIT_USAGE_ERROR: u8 = 2;
const DUN_CONFIG_ENV: &str = "DUN_CONFIG";
const FILE_DIALOG_VISIBLE_ENTRIES: usize = 12;
const BUFFER_SWITCHER_VISIBLE_ENTRIES: usize = 12;
const EDITOR_MOUSE_WHEEL_LINES: usize = 3;
const MIN_BODY_COLUMNS_WITH_GUTTER: u16 = 4;
const EDITOR_INDENT: &str = "    ";
const COMMAND_OUTPUT_STREAM_SOFT_LIMIT_BYTES: usize = 512 * 1024;

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
        CliAction::DumpConfig => {
            print!("{}", default_config_text());
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
    let color_rewrite = TerminalColorRewrite::new(app.shell.profile);
    let backend = CrosstermBackend::new(TerminalWriter::new(io::stdout(), color_rewrite.clone()));
    let mut terminal = Terminal::new(backend)?;

    let result = run_event_loop(&mut terminal, &mut app, &mut guard, &color_rewrite);
    terminal.show_cursor()?;
    result
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CliAction {
    Help,
    Version,
    DumpConfig,
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
                "--dump-config" => {
                    set_cli_action(&mut action, CliAction::DumpConfig)?;
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
            "options --help, --version, and --dump-config cannot be combined with paths",
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
            "only one of --help, --version, or --dump-config may be used",
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
      --dump-config Print the built-in default configuration and exit.
      --no-config   Ignore DUN_CONFIG and default config paths.

Exit codes:
  0                 Success, --help, --version, or --dump-config.
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
    file_dialog_keys: FileDialogKeymap,
    clipboard: ClipboardConfig,
    mouse_enabled: bool,
    mouse_drag: Option<MouseDragState>,
    active_menu: Option<usize>,
    active_menu_entry: Option<usize>,
    should_quit: bool,
    workspace_area: Rect,
    pending_keys: Vec<KeyStroke>,
    status_message: Option<String>,
    prompt: Option<PromptState>,
    file_dialog: Option<FileDialogState>,
    buffer_switcher: Option<BufferSwitcherState>,
    confirm: Option<ConfirmState>,
    replace_confirm: Option<ReplaceConfirmState>,
    status_history: Vec<StatusEntry>,
    command_history: Vec<String>,
    run_command_history: Vec<String>,
    last_find_query: Option<String>,
    pending_replace_query: Option<String>,
    outline_source: Option<BufferId>,
    search_results_source: Option<BufferId>,
    kill_ring: Option<String>,
    recent_file_dialog_input: Option<String>,
    runtime_action: Option<RuntimeAction>,
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
        let file_dialog_keys = loaded_config.config.file_dialog_keys.clone();
        let clipboard = loaded_config.config.clipboard;
        let mouse_enabled = loaded_config.config.mouse.enabled;

        Self {
            workspace: Workspace::new_untitled(),
            buffers: vec![BufferState::new(BufferId(1), TextBuffer::new_untitled())],
            config_request,
            config_source: loaded_config.source,
            detected_profile,
            shell,
            limits,
            file_dialog_keys,
            clipboard,
            mouse_enabled,
            mouse_drag: None,
            active_menu: None,
            active_menu_entry: None,
            should_quit: false,
            workspace_area: Rect::default(),
            pending_keys: Vec::new(),
            status_message: None,
            prompt: None,
            file_dialog: None,
            buffer_switcher: None,
            confirm: None,
            replace_confirm: None,
            status_history: Vec::new(),
            command_history: Vec::new(),
            run_command_history: Vec::new(),
            last_find_query: None,
            pending_replace_query: None,
            outline_source: None,
            search_results_source: None,
            kill_ring: None,
            recent_file_dialog_input: None,
            runtime_action: None,
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
            .map(|buffer| {
                let search_matches = buffer
                    .search
                    .as_ref()
                    .map(|search| search.matches.as_slice())
                    .unwrap_or(&[]);
                let active_search_match = buffer
                    .search
                    .as_ref()
                    .and_then(|search| search.active_index);
                BufferView::scrolled_xy(
                    buffer.id,
                    &buffer.buffer,
                    buffer.first_line,
                    buffer.first_column,
                )
                .with_first_visual_row(buffer.first_visual_row)
                .with_search(search_matches, active_search_match)
                .with_view_options(
                    buffer.word_wrap,
                    buffer.visible_whitespace,
                    &buffer.bookmarks,
                )
            })
            .collect()
    }

    fn refresh_search_caches(&mut self) {
        for buffer in &mut self.buffers {
            buffer.refresh_search_cache();
        }
    }

    const fn mouse_enabled(&self) -> bool {
        self.mouse_enabled
    }

    fn menu_selection(&self) -> Option<MenuSelection> {
        self.active_menu.map(|menu_index| MenuSelection {
            menu_index,
            entry_index: self.active_menu_entry,
        })
    }

    fn clear_active_menu(&mut self) {
        self.active_menu = None;
        self.active_menu_entry = None;
    }

    fn first_menu_entry(&self, menu_index: usize) -> Option<usize> {
        self.shell
            .menu_entry_count(menu_index)
            .filter(|count| *count > 0)
            .map(|_| 0)
    }

    fn open_mouse_menu(&mut self, menu_index: usize) {
        if self.active_menu == Some(menu_index) {
            self.clear_active_menu();
        } else {
            self.active_menu = Some(menu_index);
            self.active_menu_entry = None;
        }
    }

    fn open_keyboard_menu(&mut self, menu_index: usize) {
        self.active_menu = Some(menu_index);
        self.active_menu_entry = self.first_menu_entry(menu_index);
    }

    fn move_active_menu(&mut self, delta: isize) -> bool {
        let Some(current) = self.active_menu else {
            return false;
        };
        let menu_count = self.shell.menu_count();
        if menu_count == 0 {
            self.clear_active_menu();
            return true;
        }

        let next = wrapping_index(current, menu_count, delta);
        let keep_entry_selected = self.active_menu_entry.is_some();
        self.active_menu = Some(next);
        self.active_menu_entry = if keep_entry_selected {
            self.first_menu_entry(next)
        } else {
            None
        };
        true
    }

    fn move_active_menu_entry(&mut self, delta: isize) -> bool {
        let Some(menu_index) = self.active_menu else {
            return false;
        };
        let Some(count) = self.shell.menu_entry_count(menu_index) else {
            return false;
        };
        if count == 0 {
            self.active_menu_entry = None;
            return true;
        }

        let current = self.active_menu_entry.unwrap_or(0);
        self.active_menu_entry = Some(wrapping_index(current, count, delta));
        true
    }

    fn dispatch_active_menu_entry(&mut self) -> bool {
        let Some(menu_index) = self.active_menu else {
            return false;
        };
        let entry_index = self.active_menu_entry.unwrap_or(0);
        let Some(command) = self.shell.menu_entry_command(menu_index, entry_index) else {
            return false;
        };

        self.clear_active_menu();
        self.pending_keys.clear();
        self.handle_command(&command);
        true
    }

    fn sync_view_for_area(&mut self, area: Rect) {
        self.workspace_area = area;
        self.refresh_search_caches();
        let Some(context) = self.focused_buffer_view_context(area) else {
            return;
        };
        let Some(buffer) = self.buffer_state_mut(context.buffer_id) else {
            return;
        };

        buffer.ensure_cursor_visible(context.body_height, context.body_width);
    }

    fn handle_mouse_down(&mut self, screen_x: u16, screen_y: u16) -> bool {
        self.mouse_drag = None;
        if screen_y == 0 {
            if let Some(menu_index) = self.shell.menu_index_at_column(screen_x) {
                self.pending_keys.clear();
                self.open_mouse_menu(menu_index);
                return true;
            }
            self.clear_active_menu();
            return false;
        }

        if let Some(selection) = self.menu_selection() {
            if let Some(command) = self.shell.menu_entry_command_at_in_area(
                selection,
                screen_x,
                screen_y,
                self.overlay_area(),
            ) {
                self.clear_active_menu();
                self.pending_keys.clear();
                self.handle_command(&command);
                return true;
            }
            self.clear_active_menu();
        }

        let Some((x, y)) = self.workspace_point_from_screen(screen_x, screen_y) else {
            return false;
        };
        if let Some(handle) = self.workspace.split_at(self.workspace_area, x, y) {
            let _ = self.workspace.focus_at(self.workspace_area, x, y);
            self.pending_keys.clear();
            self.mouse_drag = Some(MouseDragState::Split { handle });
            return true;
        }

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
        match hit.target {
            UiMouseTarget::Body(position) => {
                if let Some(buffer) = self.buffer_state_mut(hit.buffer_id) {
                    let _ = buffer.buffer.set_cursor(position);
                }
                self.mouse_drag = Some(MouseDragState::Selection {
                    buffer_id: hit.buffer_id,
                    anchor: position,
                });
                self.sync_view_for_area(self.workspace_area);
            }
            UiMouseTarget::Scrollbar {
                first_line,
                first_visual_row,
            } => {
                self.scroll_buffer_to_line(hit.buffer_id, first_line, first_visual_row);
                self.mouse_drag = Some(MouseDragState::Scrollbar {
                    buffer_id: hit.buffer_id,
                });
            }
            UiMouseTarget::Chrome | UiMouseTarget::Gutter => {}
        }

        true
    }

    fn handle_mouse_drag(&mut self, screen_x: u16, screen_y: u16) -> bool {
        if self.active_menu.is_some() {
            return false;
        }
        let Some(drag) = self.mouse_drag.clone() else {
            return false;
        };

        match drag {
            MouseDragState::Selection { buffer_id, anchor } => {
                self.update_mouse_selection(buffer_id, anchor, screen_x, screen_y)
            }
            MouseDragState::Split { handle } => {
                let Some((x, y)) = self.clamped_workspace_point_from_screen(screen_x, screen_y)
                else {
                    return false;
                };
                if self
                    .workspace
                    .resize_split_to(&handle, self.workspace_area, x, y)
                    .is_ok()
                {
                    self.sync_view_for_area(self.workspace_area);
                    true
                } else {
                    false
                }
            }
            MouseDragState::Scrollbar { buffer_id } => {
                self.update_scrollbar_drag(buffer_id, screen_x, screen_y)
            }
        }
    }

    fn handle_mouse_up(&mut self) {
        self.mouse_drag = None;
    }

    fn handle_file_dialog_mouse_down(&mut self, screen_x: u16, screen_y: u16) -> bool {
        self.mouse_drag = None;
        let Some(dialog) = &self.file_dialog else {
            return false;
        };
        let overlay = dialog.overlay(&self.file_dialog_keys);
        let Some(visible_index) =
            self.shell
                .hit_test_overlay_list(&overlay, self.overlay_area(), screen_x, screen_y)
        else {
            return false;
        };

        self.pending_keys.clear();
        self.click_file_dialog_visible_index(visible_index);
        true
    }

    fn overlay_area(&self) -> Rect {
        Rect::new(
            0,
            0,
            self.workspace_area.width,
            self.workspace_area.height.saturating_add(2),
        )
    }

    fn update_mouse_selection(
        &mut self,
        buffer_id: BufferId,
        anchor: Position,
        screen_x: u16,
        screen_y: u16,
    ) -> bool {
        let Some((x, y)) = self.clamped_workspace_point_from_screen(screen_x, screen_y) else {
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
        if hit.buffer_id != buffer_id {
            return false;
        }
        let position = match hit.target {
            UiMouseTarget::Body(position) => position,
            UiMouseTarget::Chrome | UiMouseTarget::Gutter | UiMouseTarget::Scrollbar { .. } => {
                let Some(position) = self.drag_scroll_selection_position(buffer_id, x, y) else {
                    return false;
                };
                position
            }
        };

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            let _ = buffer.buffer.select(anchor, position);
            self.sync_view_for_area(self.workspace_area);
            true
        } else {
            false
        }
    }

    fn drag_scroll_selection_position(
        &mut self,
        buffer_id: BufferId,
        workspace_x: u16,
        workspace_y: u16,
    ) -> Option<Position> {
        let layout = self
            .workspace
            .resolved_layout(self.workspace_area)
            .into_iter()
            .find(|layout| {
                self.workspace
                    .window(layout.id)
                    .ok()
                    .is_some_and(|window| window.buffer_id == buffer_id)
            })?;
        if layout.rect.width <= 2 || layout.rect.height <= 2 {
            return None;
        }

        let body_height = layout.rect.height.saturating_sub(2) as usize;
        let body_width = layout.rect.width.saturating_sub(2) as usize;
        let top = layout.rect.y.saturating_add(1);
        let bottom = layout
            .rect
            .y
            .saturating_add(layout.rect.height)
            .saturating_sub(2);
        let target_line = {
            let buffer = self.buffer_state_mut(buffer_id)?;
            if workspace_y <= top {
                buffer.scroll_view_lines(-1, body_height, body_width);
                buffer.first_line
            } else if workspace_y >= bottom {
                buffer.scroll_view_lines(1, body_height, body_width);
                buffer
                    .first_line
                    .saturating_add(body_height.saturating_sub(1))
                    .min(buffer.buffer.line_count().saturating_sub(1))
            } else {
                buffer
                    .first_line
                    .saturating_add(workspace_y.saturating_sub(top) as usize)
            }
        };

        let x = workspace_x
            .clamp(
                layout.rect.x.saturating_add(1),
                layout
                    .rect
                    .x
                    .saturating_add(layout.rect.width)
                    .saturating_sub(2),
            )
            .saturating_sub(layout.rect.x.saturating_add(1)) as usize;
        let buffer = self.buffer_state(buffer_id)?;
        let line = buffer.buffer.line(target_line)?;
        let display_column = buffer
            .first_column
            .saturating_add(x.min(body_width.saturating_sub(1)));
        let column = clamp_to_display_column(line, display_column);
        Some(Position::new(target_line, column))
    }

    fn handle_mouse_scroll(&mut self, screen_x: u16, screen_y: u16, delta: isize) -> bool {
        if self.active_menu.is_some() {
            self.clear_active_menu();
        }
        let Some((x, y)) = self.workspace_point_from_screen(screen_x, screen_y) else {
            return false;
        };
        let Some(window_id) = self.workspace.focus_at(self.workspace_area, x, y) else {
            return false;
        };
        let Some(buffer_id) = self
            .workspace
            .windows
            .iter()
            .find(|window| window.id == window_id)
            .map(|window| window.buffer_id)
        else {
            return false;
        };
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });

        self.pending_keys.clear();
        self.buffer_state_mut(buffer_id).is_some_and(|buffer| {
            let moved = buffer.scroll_view_lines(delta, context.body_height, context.body_width);
            buffer.ensure_cursor_column_visible(context.body_width);
            moved
        })
    }

    fn update_scrollbar_drag(&mut self, buffer_id: BufferId, screen_x: u16, screen_y: u16) -> bool {
        let Some((x, y)) = self.clamped_workspace_point_from_screen(screen_x, screen_y) else {
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
        if hit.buffer_id != buffer_id {
            return false;
        }
        let UiMouseTarget::Scrollbar {
            first_line,
            first_visual_row,
        } = hit.target
        else {
            return false;
        };

        self.scroll_buffer_to_line(buffer_id, first_line, first_visual_row)
    }

    fn scroll_buffer_to_line(
        &mut self,
        buffer_id: BufferId,
        first_line: usize,
        first_visual_row: usize,
    ) -> bool {
        let context = self
            .buffer_view_context(buffer_id, self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });
        self.pending_keys.clear();
        self.buffer_state_mut(buffer_id).is_some_and(|buffer| {
            let moved = buffer.scroll_view_to_line(
                first_line,
                first_visual_row,
                context.body_height,
                context.body_width,
            );
            buffer.ensure_cursor_column_visible(context.body_width);
            moved
        })
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

    fn clamped_workspace_point_from_screen(&self, column: u16, row: u16) -> Option<(u16, u16)> {
        if self.workspace_area.width == 0 || self.workspace_area.height == 0 {
            return None;
        }

        Some((
            column.min(self.workspace_area.width.saturating_sub(1)),
            row.saturating_sub(1)
                .min(self.workspace_area.height.saturating_sub(1)),
        ))
    }

    fn handle_command(&mut self, command: &EditorCommand) {
        self.clear_active_menu();
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

    fn handle_selection_key_stroke(&mut self, stroke: KeyStroke) -> bool {
        if stroke.modifiers != KeyModifiers::SHIFT {
            return false;
        }

        match stroke.key {
            Key::PageUp => return self.extend_focused_page(-1),
            Key::PageDown => return self.extend_focused_page(1),
            _ => {}
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        match stroke.key {
            Key::Left => buffer.buffer.extend_selection_left(),
            Key::Right => buffer.buffer.extend_selection_right(),
            Key::Up => buffer.buffer.extend_selection_up(),
            Key::Down => buffer.buffer.extend_selection_down(),
            Key::Home => buffer.buffer.extend_selection_to_line_start(),
            Key::End => buffer.buffer.extend_selection_to_line_end(),
            _ => false,
        }
    }

    fn handle_auxiliary_window_key_stroke(&mut self, stroke: KeyStroke) -> bool {
        if stroke.modifiers != KeyModifiers::NONE && stroke.modifiers != KeyModifiers::SHIFT {
            return false;
        }

        let Ok(window) = self.workspace.focused_window() else {
            return false;
        };

        match window.kind {
            WindowKind::Outline => match stroke.key {
                Key::Enter => {
                    self.jump_current_outline_target();
                    true
                }
                Key::Char('n') | Key::Char('N') => {
                    self.move_focused_numbered_aux_row(1, "Outline");
                    true
                }
                Key::Char('p') | Key::Char('P') => {
                    self.move_focused_numbered_aux_row(-1, "Outline");
                    true
                }
                _ => false,
            },
            WindowKind::SearchResults => match stroke.key {
                Key::Enter => {
                    self.jump_current_search_result();
                    true
                }
                Key::Char('n') | Key::Char('N') => {
                    self.move_focused_numbered_aux_row(1, "Search Results");
                    true
                }
                Key::Char('p') | Key::Char('P') => {
                    self.move_focused_numbered_aux_row(-1, "Search Results");
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn handle_auxiliary_enter_key_stroke(&mut self, stroke: KeyStroke) -> bool {
        if stroke.modifiers != KeyModifiers::NONE || stroke.key != Key::Enter {
            return false;
        }

        let Ok(window) = self.workspace.focused_window() else {
            return false;
        };

        match window.kind {
            WindowKind::Outline => {
                self.jump_current_outline_target();
                true
            }
            WindowKind::SearchResults => {
                self.jump_current_search_result();
                true
            }
            _ => false,
        }
    }

    fn handle_app_command(&mut self, command: &AppCommand) {
        match command {
            AppCommand::CommandOutputClear => self.clear_command_output(),
            AppCommand::CommandOutputCopy => self.copy_command_output(),
            AppCommand::CommandOutputIndex => self.jump_command_output_index(),
            AppCommand::CommandOutputNextMatch => {
                self.repeat_find_in_command_output(SearchDirection::Forward)
            }
            AppCommand::CommandOutputNextSection => {
                self.jump_command_output_relative_section(SearchDirection::Forward)
            }
            AppCommand::CommandOutputOnlyStderr => {
                self.open_command_output_section_view(CommandOutputSection::Stderr)
            }
            AppCommand::CommandOutputOnlyStdout => {
                self.open_command_output_section_view(CommandOutputSection::Stdout)
            }
            AppCommand::CommandOutputPreviousMatch => {
                self.repeat_find_in_command_output(SearchDirection::Backward)
            }
            AppCommand::CommandOutputPreviousSection => {
                self.jump_command_output_relative_section(SearchDirection::Backward)
            }
            AppCommand::CommandOutputStderr => self.jump_command_output_stderr(),
            AppCommand::CommandOutputStderrBody => self.jump_command_output_stderr_body(),
            AppCommand::CommandOutputStatus => self.jump_command_output_status(),
            AppCommand::CommandOutputStdout => self.jump_command_output_stdout(),
            AppCommand::CommandOutputStdoutBody => self.jump_command_output_stdout_body(),
            AppCommand::CommandOutputSummary => self.jump_command_output_summary(),
            AppCommand::CommandOutputSave => self.start_command_output_save_dialog(),
            AppCommand::CommandOutputTruncated => self.jump_command_output_truncated(),
            AppCommand::ConfigDiagnostics => self.open_config_diagnostics_screen(),
            AppCommand::ConfigDiagnosticsClipboard => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Clipboard)
            }
            AppCommand::ConfigDiagnosticsFileDialogKeymap => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::FileDialogKeymap)
            }
            AppCommand::ConfigDiagnosticsInput => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Input)
            }
            AppCommand::ConfigDiagnosticsKeymap => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Keymap)
            }
            AppCommand::ConfigDiagnosticsLimits => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Limits)
            }
            AppCommand::ConfigDiagnosticsPaths => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Paths)
            }
            AppCommand::ConfigDiagnosticsSource => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Source)
            }
            AppCommand::ConfigDiagnosticsSummary => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Summary)
            }
            AppCommand::ConfigDiagnosticsTerminal => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Terminal)
            }
            AppCommand::Help => self.open_help_screen(),
            AppCommand::Outline => self.open_outline_screen(),
            AppCommand::ReloadConfig => self.reload_config(),
            AppCommand::RunCommand => self.start_prompt(PromptKind::RunCommand, String::new()),
            AppCommand::SearchResults => self.open_search_results_screen(),
            AppCommand::ShellEscape => self.request_runtime_action(RuntimeAction::ShellEscape),
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

    fn request_runtime_action(&mut self, action: RuntimeAction) {
        self.clear_active_menu();
        self.pending_keys.clear();
        self.prompt = None;
        self.file_dialog = None;
        self.buffer_switcher = None;
        self.runtime_action = Some(action);
        self.set_status("Shell escape");
    }

    fn take_runtime_action(&mut self) -> Option<RuntimeAction> {
        self.runtime_action.take()
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
        self.mouse_drag = None;
        self.clear_active_menu();
        self.shell = UiShell::from_config(&loaded_config.config, self.detected_profile);
        self.limits = loaded_config.config.limits;
        self.file_dialog_keys = loaded_config.config.file_dialog_keys.clone();
        self.clipboard = loaded_config.config.clipboard;
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
                self.start_file_dialog(FileDialogKind::Open, self.default_open_dialog_input());
            }
            FileCommand::SwitchBuffer => {
                self.start_buffer_switcher();
            }
            FileCommand::SaveAs => {
                let focused_path = self.focused_path_text();
                let input = if focused_path.is_empty() {
                    self.recent_file_dialog_input.clone().unwrap_or_default()
                } else {
                    focused_path
                };
                self.start_file_dialog(FileDialogKind::SaveAs, input);
            }
            FileCommand::Reload => {
                if self.confirm_focused_dirty(PendingAction::ReloadBuffer) {
                    return;
                }
                if let Err(error) = self.reload_focused_buffer() {
                    self.set_status(format!("Reload failed: {error}"));
                }
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
            EditCommand::Cut => {
                self.cut_selection();
                return;
            }
            EditCommand::Copy => {
                self.copy_selection();
                return;
            }
            EditCommand::CopyExternal => {
                self.copy_selection_external();
                return;
            }
            EditCommand::CopyLine => {
                self.copy_current_line();
                return;
            }
            EditCommand::Paste => {
                self.paste_internal_clipboard();
                return;
            }
            EditCommand::DeleteLine => {
                self.delete_current_line();
                return;
            }
            EditCommand::MoveLineUp => {
                self.move_current_line(-1);
                return;
            }
            EditCommand::MoveLineDown => {
                self.move_current_line(1);
                return;
            }
            EditCommand::IndentLine => {
                self.indent_selected_lines();
                return;
            }
            EditCommand::OutdentLine => {
                self.outdent_selected_lines();
                return;
            }
            EditCommand::TrimTrailingWhitespace => {
                self.trim_trailing_whitespace();
                return;
            }
            EditCommand::ToggleWordWrap => {
                self.toggle_word_wrap();
                return;
            }
            EditCommand::ToggleVisibleWhitespace => {
                self.toggle_visible_whitespace();
                return;
            }
            EditCommand::ToggleBookmark => {
                self.toggle_bookmark();
                return;
            }
            EditCommand::NextBookmark => {
                self.goto_bookmark(SearchDirection::Forward);
                return;
            }
            EditCommand::PreviousBookmark => {
                self.goto_bookmark(SearchDirection::Backward);
                return;
            }
            EditCommand::Undo => {
                self.undo_focused_buffer();
                return;
            }
            EditCommand::Redo => {
                self.redo_focused_buffer();
                return;
            }
            EditCommand::MovePageUp => {
                self.move_focused_page(-1);
                return;
            }
            EditCommand::MovePageDown => {
                self.move_focused_page(1);
                return;
            }
            EditCommand::MoveDocumentStart => {
                self.move_focused_document_edge(false);
                return;
            }
            EditCommand::MoveDocumentEnd => {
                self.move_focused_document_edge(true);
                return;
            }
            EditCommand::ScrollLeft => {
                self.scroll_focused_columns(-1);
                return;
            }
            EditCommand::ScrollRight => {
                self.scroll_focused_columns(1);
                return;
            }
            EditCommand::ExtendSelectionPageUp => {
                self.extend_focused_page(-1);
                return;
            }
            EditCommand::ExtendSelectionPageDown => {
                self.extend_focused_page(1);
                return;
            }
            _ => {}
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            return;
        };

        match command {
            EditCommand::SelectAll => {
                let end = buffer_end_position(&buffer.buffer);
                let _ = buffer.buffer.select(Position::zero(), end);
            }
            EditCommand::SelectLine => {
                let _ = buffer.buffer.select_current_line();
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
            EditCommand::MoveWordLeft => {
                buffer.buffer.move_word_left();
            }
            EditCommand::MoveWordRight => {
                buffer.buffer.move_word_right();
            }
            EditCommand::MoveLineStart => {
                buffer.buffer.move_to_line_start();
            }
            EditCommand::MoveLineEnd => {
                buffer.buffer.move_to_line_end();
            }
            EditCommand::ExtendSelectionWordLeft => {
                buffer.buffer.extend_selection_word_left();
            }
            EditCommand::ExtendSelectionWordRight => {
                buffer.buffer.extend_selection_word_right();
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
            EditCommand::DeleteWordBackward => {
                let _ = buffer.buffer.delete_word_backward();
            }
            EditCommand::DeleteWordForward => {
                let _ = buffer.buffer.delete_word_forward();
            }
            EditCommand::Cut
            | EditCommand::Copy
            | EditCommand::CopyExternal
            | EditCommand::CopyLine
            | EditCommand::Paste
            | EditCommand::DeleteLine
            | EditCommand::MoveLineUp
            | EditCommand::MoveLineDown
            | EditCommand::IndentLine
            | EditCommand::OutdentLine
            | EditCommand::TrimTrailingWhitespace
            | EditCommand::ToggleWordWrap
            | EditCommand::ToggleVisibleWhitespace
            | EditCommand::ToggleBookmark
            | EditCommand::NextBookmark
            | EditCommand::PreviousBookmark
            | EditCommand::Undo
            | EditCommand::Redo
            | EditCommand::Find
            | EditCommand::FindNext
            | EditCommand::FindPrevious
            | EditCommand::Replace
            | EditCommand::GoToLine
            | EditCommand::MovePageUp
            | EditCommand::MovePageDown
            | EditCommand::MoveDocumentStart
            | EditCommand::MoveDocumentEnd
            | EditCommand::ScrollLeft
            | EditCommand::ScrollRight
            | EditCommand::ExtendSelectionPageUp
            | EditCommand::ExtendSelectionPageDown => {}
        }
    }

    fn copy_current_line(&mut self) {
        let Some(buffer_id) = self.focused_buffer_id() else {
            self.set_status("Copy line failed: focused buffer is missing");
            return;
        };

        let text = self.buffer_state(buffer_id).and_then(|buffer| {
            let range = buffer.buffer.current_line_range();
            buffer.buffer.text_in_range(range).ok()
        });
        match text {
            Some(text) => {
                self.kill_ring = Some(text);
                self.set_status("Copied line");
            }
            None => self.set_status("Copy line failed: focused buffer is missing"),
        }
    }

    fn delete_current_line(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Delete line failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.delete_current_line() {
            Ok(true) => {
                buffer.normalize_bookmarks();
                "Deleted line".to_string()
            }
            Ok(false) => "Delete line: nothing deleted".to_string(),
            Err(error) => format!("Delete line failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn move_current_line(&mut self, direction: isize) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Move line failed: focused buffer is missing");
            return;
        };

        let moved = if direction < 0 {
            buffer.buffer.move_current_line_up()
        } else {
            buffer.buffer.move_current_line_down()
        };
        let status = match moved {
            Ok(true) => {
                buffer.remap_bookmarks_after_line_move(direction);
                if direction < 0 {
                    "Moved line up".to_string()
                } else {
                    "Moved line down".to_string()
                }
            }
            Ok(false) if direction < 0 => "Move line: already at top".to_string(),
            Ok(false) => "Move line: already at bottom".to_string(),
            Err(error) => format!("Move line failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn indent_selected_lines(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Indent failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.indent_selected_lines(EDITOR_INDENT) {
            Ok(0) => "Indent: nothing changed".to_string(),
            Ok(count) => format!("Indented {count} line(s)"),
            Err(error) => format!("Indent failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn outdent_selected_lines(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Outdent failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.outdent_selected_lines(EDITOR_INDENT.len()) {
            Ok(0) => "Outdent: nothing changed".to_string(),
            Ok(count) => format!("Outdented {count} line(s)"),
            Err(error) => format!("Outdent failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn trim_trailing_whitespace(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Trim failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.trim_trailing_whitespace() {
            Ok(0) => "Trim: no trailing whitespace".to_string(),
            Ok(count) => format!("Trimmed trailing whitespace on {count} line(s)"),
            Err(error) => format!("Trim failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn toggle_word_wrap(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Wrap failed: focused buffer is missing");
            return;
        };

        buffer.word_wrap = !buffer.word_wrap;
        if buffer.word_wrap {
            buffer.first_column = 0;
            buffer.first_visual_row = 0;
            self.set_status("Word wrap on");
        } else {
            buffer.first_visual_row = 0;
            self.set_status("Word wrap off");
        }
    }

    fn toggle_visible_whitespace(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Whitespace failed: focused buffer is missing");
            return;
        };

        buffer.visible_whitespace = !buffer.visible_whitespace;
        if buffer.visible_whitespace {
            self.set_status("Visible whitespace on");
        } else {
            self.set_status("Visible whitespace off");
        }
    }

    fn toggle_bookmark(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Bookmark failed: focused buffer is missing");
            return;
        };

        let line = buffer.buffer.cursor_position().line;
        if let Some(index) = buffer
            .bookmarks
            .iter()
            .position(|bookmark| *bookmark == line)
        {
            buffer.bookmarks.remove(index);
            self.set_status(format!("Removed bookmark at line {}", line + 1));
        } else {
            buffer.bookmarks.push(line);
            buffer.bookmarks.sort_unstable();
            self.set_status(format!("Bookmarked line {}", line + 1));
        }
    }

    fn goto_bookmark(&mut self, direction: SearchDirection) {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Bookmark failed: focused buffer is missing");
            return;
        };

        buffer.normalize_bookmarks();
        if buffer.bookmarks.is_empty() {
            self.set_status("Bookmark: none set");
            return;
        }

        let cursor_line = buffer.buffer.cursor_position().line;
        let target_line = match direction {
            SearchDirection::Forward => buffer
                .bookmarks
                .iter()
                .copied()
                .find(|line| *line > cursor_line)
                .unwrap_or(buffer.bookmarks[0]),
            SearchDirection::Backward => buffer
                .bookmarks
                .iter()
                .rev()
                .copied()
                .find(|line| *line < cursor_line)
                .unwrap_or_else(|| *buffer.bookmarks.last().expect("non-empty bookmarks")),
        };
        let column =
            buffer.clamp_column_to_line(target_line, buffer.buffer.cursor_position().column);
        let _ = buffer.buffer.set_cursor(Position::new(target_line, column));
        buffer.ensure_cursor_visible(context.body_height, context.body_width);
        self.set_status(format!("Bookmark: line {}", target_line + 1));
    }

    fn undo_focused_buffer(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Undo failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.undo() {
            Ok(true) => "Undo".to_string(),
            Ok(false) => "Nothing to undo".to_string(),
            Err(error) => format!("Undo failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn redo_focused_buffer(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Redo failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.redo() {
            Ok(true) => "Redo".to_string(),
            Ok(false) => "Nothing to redo".to_string(),
            Err(error) => format!("Redo failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn move_focused_page(&mut self, direction: isize) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let body_height = context.body_height;
        let page_lines = body_height.saturating_sub(1).max(1);
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let moved = if buffer.word_wrap {
            buffer.move_wrapped_page(direction, page_lines, context.body_width)
        } else if direction < 0 {
            buffer.move_page_up(page_lines)
        } else {
            buffer.move_page_down(page_lines)
        };
        buffer.ensure_cursor_visible(body_height, context.body_width);
        moved
    }

    fn move_focused_document_edge(&mut self, end: bool) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let target = if end {
            buffer_end_position(&buffer.buffer)
        } else {
            Position::zero()
        };
        let moved =
            buffer.buffer.cursor_position() != target || buffer.buffer.selection().is_some();
        let _ = buffer.buffer.set_cursor(target);
        buffer.ensure_cursor_visible(context.body_height, context.body_width);
        moved
    }

    fn scroll_focused_columns(&mut self, direction: isize) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let step = context.body_width.saturating_div(2).max(1) as isize;
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let moved = buffer.scroll_view_columns(direction.saturating_mul(step), context.body_width);
        let first_column = buffer.first_column;
        let status = if moved {
            if direction < 0 {
                format!("Scrolled left to column {}", first_column + 1)
            } else {
                format!("Scrolled right to column {}", first_column + 1)
            }
        } else if direction < 0 {
            "Already at left edge".to_string()
        } else {
            "Already at right edge".to_string()
        };
        self.set_status(status);
        moved
    }

    fn extend_focused_page(&mut self, direction: isize) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let body_height = context.body_height;
        let page_lines = body_height.saturating_sub(1).max(1);
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let moved = if buffer.word_wrap {
            buffer.extend_wrapped_page(direction, page_lines, context.body_width)
        } else if direction < 0 {
            buffer.extend_page_up(page_lines)
        } else {
            buffer.extend_page_down(page_lines)
        };
        buffer.ensure_cursor_visible(body_height, context.body_width);
        moved
    }

    fn copy_selection(&mut self) {
        match self.focused_selection_text() {
            Ok(text) => {
                self.kill_ring = Some(text);
                self.set_status("Copied selection");
            }
            Err(CopyTextError::MissingBuffer) => {
                self.set_status("Copy failed: focused buffer is missing")
            }
            Err(CopyTextError::NoSelection) => self.set_status("Copy: no selection"),
            Err(CopyTextError::Buffer(error)) => {
                self.set_status(format!("Copy failed: {}", buffer_error_text(error)))
            }
        }
    }

    fn copy_selection_external(&mut self) {
        match self.focused_selection_text() {
            Ok(text) => self.copy_text_external(text, "selection"),
            Err(CopyTextError::MissingBuffer) => {
                self.set_status("External copy failed: focused buffer is missing")
            }
            Err(CopyTextError::NoSelection) => self.set_status("External copy: no selection"),
            Err(CopyTextError::Buffer(error)) => self.set_status(format!(
                "External copy failed: {}",
                buffer_error_text(error)
            )),
        }
    }

    fn focused_selection_text(&self) -> Result<String, CopyTextError> {
        let Some(buffer) = self.focused_buffer() else {
            return Err(CopyTextError::MissingBuffer);
        };
        let Some(range) = buffer
            .buffer
            .selection_range()
            .filter(|range| !range.is_empty())
        else {
            return Err(CopyTextError::NoSelection);
        };

        buffer
            .buffer
            .text_in_range(range)
            .map_err(CopyTextError::Buffer)
    }

    fn copy_text_external(&mut self, text: String, label: &str) {
        self.kill_ring = Some(text.clone());
        let byte_len = text.len();
        if !self.clipboard.osc52.enabled {
            self.set_status(format!("External copy disabled: copied {label} internally"));
            return;
        }
        if byte_len > self.clipboard.osc52.max_bytes {
            self.set_status(format!(
                "External copy failed: {label} is {byte_len} bytes; limit is {}",
                self.clipboard.osc52.max_bytes
            ));
            return;
        }

        self.runtime_action = Some(RuntimeAction::WriteTerminal(osc52_copy_sequence(&text)));
        self.set_status(format!("Copied {label} to external clipboard"));
    }

    fn cut_selection(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Cut failed: focused buffer is missing");
            return;
        };

        if buffer.buffer.is_read_only() {
            self.set_status("Cut failed: buffer is read-only");
            return;
        }

        let Some(range) = buffer
            .buffer
            .selection_range()
            .filter(|range| !range.is_empty())
        else {
            self.set_status("Cut: no selection");
            return;
        };

        let text = match buffer.buffer.text_in_range(range) {
            Ok(text) => text,
            Err(error) => {
                self.set_status(format!("Cut failed: {}", buffer_error_text(error)));
                return;
            }
        };

        match buffer.buffer.delete_range(range) {
            Ok(true) => {
                self.kill_ring = Some(text);
                self.set_status("Cut selection");
            }
            Ok(false) => self.set_status("Cut: no selection"),
            Err(error) => self.set_status(format!("Cut failed: {}", buffer_error_text(error))),
        }
    }

    fn paste_internal_clipboard(&mut self) {
        let Some(text) = self.kill_ring.clone() else {
            self.set_status("Paste: internal clipboard empty; use terminal paste");
            return;
        };
        if text.is_empty() {
            self.set_status("Paste: internal clipboard empty; use terminal paste");
            return;
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Paste failed: focused buffer is missing");
            return;
        };

        match buffer.buffer.insert_str(&text) {
            Ok(()) => self.set_status("Pasted selection"),
            Err(error) => self.set_status(format!("Paste failed: {}", buffer_error_text(error))),
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
            validate_save_snapshot(buffer, &path)?;
            (path, buffer.buffer.to_text())
        };

        let report =
            atomic_write_text_file(&path, &text).map_err(|error| path_io_error(&path, error))?;

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            buffer.buffer.mark_saved();
            buffer.file_snapshot = current_file_snapshot(&path).ok();
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
            buffer.file_snapshot = current_file_snapshot(&path).ok();
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

    fn reload_focused_buffer(&mut self) -> io::Result<()> {
        let window = self
            .workspace
            .focused_window()
            .map_err(|_| io::Error::other("focused window is missing"))?;
        let window_id = window.id;
        let buffer_id = window.buffer_id;
        let (
            path,
            cursor,
            first_line,
            first_visual_row,
            first_column,
            word_wrap,
            visible_whitespace,
            bookmarks,
        ) = {
            let buffer = self
                .buffer_state(buffer_id)
                .ok_or_else(|| io::Error::other("focused buffer is missing"))?;
            let path = buffer.path.clone().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer has no file path",
                )
            })?;
            (
                path,
                buffer.buffer.cursor_position(),
                buffer.first_line,
                buffer.first_visual_row,
                buffer.first_column,
                buffer.word_wrap,
                buffer.visible_whitespace,
                buffer.bookmarks.clone(),
            )
        };

        let loaded =
            load_text_buffer(&path, self.limits).map_err(|error| path_io_error(&path, error))?;
        let temp_report = reconcile_atomic_save_temp_files(&path);
        let title = title_for_path(&path);
        let kind = loaded.buffer.kind();
        let encoding = loaded.encoding;
        let mut reloaded = BufferState::from_file(buffer_id, path.clone(), loaded);
        reloaded.word_wrap = word_wrap;
        reloaded.visible_whitespace = visible_whitespace;
        reloaded.bookmarks = bookmarks;
        reloaded.normalize_bookmarks();
        let line = cursor
            .line
            .min(reloaded.buffer.line_count().saturating_sub(1));
        let column = reloaded.clamp_column_to_line(line, cursor.column);
        let _ = reloaded.buffer.set_cursor(Position::new(line, column));
        reloaded.first_line = first_line.min(reloaded.buffer.line_count().saturating_sub(1));
        reloaded.first_column = if reloaded.word_wrap { 0 } else { first_column };
        reloaded.first_visual_row = if reloaded.word_wrap {
            first_visual_row
        } else {
            0
        };

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = reloaded;
        } else {
            self.buffers.push(reloaded);
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = title;
            window.buffer_kind = kind;
        }

        let status = reloaded_file_status(&path, encoding);
        self.set_status(status_with_atomic_temp_report(status, &temp_report));
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

    fn open_read_only_aux_window(&mut self, kind: WindowKind, title: &str, buffer: TextBuffer) {
        if let Some(window_id) = self
            .workspace
            .windows
            .iter()
            .find(|window| window.kind == kind)
            .map(|window| window.id)
        {
            self.workspace.focused = window_id;
            let Ok(window) = self.workspace.window(window_id) else {
                return;
            };
            let buffer_id = window.buffer_id;
            if let Some(state) = self.buffer_state_mut(buffer_id) {
                *state = BufferState::new(buffer_id, buffer);
            }
            if let Ok(window) = self.workspace.window_mut(window_id) {
                window.title = title.to_string();
                window.kind = kind;
                window.buffer_kind = BufferKind::ReadOnly;
                window.collapsed = false;
            }
            return;
        }

        let Ok(window_id) = self.workspace.split_focused(Axis::Horizontal) else {
            self.set_status(format!("{title} failed: focused window is missing"));
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            self.set_status(format!("{title} failed: window is missing"));
            return;
        };
        let buffer_id = window.buffer_id;
        let state = BufferState::new(buffer_id, buffer);
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = state;
        } else {
            self.buffers.push(state);
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = title.to_string();
            window.kind = kind;
            window.buffer_kind = BufferKind::ReadOnly;
            window.collapsed = false;
        }
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
        let help = BufferState::new(
            buffer_id,
            help_buffer(&self.shell.keymap, &self.file_dialog_keys),
        );

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
        let help = BufferState::new(
            buffer_id,
            help_buffer(&self.shell.keymap, &self.file_dialog_keys),
        );

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = help;
        }
    }

    fn open_outline_screen(&mut self) {
        let Some(source_buffer_id) = self.outline_source_for_command() else {
            self.set_status("Outline failed: focused buffer is missing");
            return;
        };
        let source_title = self.buffer_display_name(source_buffer_id);
        let Some(source) = self.buffer_state(source_buffer_id) else {
            self.set_status("Outline failed: source buffer is missing");
            return;
        };
        let entries = outline_entries_for_buffer(&source.buffer);
        if entries.is_empty() {
            self.set_status("Outline: no sections");
            return;
        }

        self.outline_source = Some(source_buffer_id);
        let text = outline_text(&source_title, &entries);
        self.open_read_only_aux_window(WindowKind::Outline, "Outline", outline_buffer(&text));
        self.set_status(format!("Outline: {} section(s)", entries.len()));
    }

    fn outline_source_for_command(&self) -> Option<BufferId> {
        let focused = self.workspace.focused_window().ok()?;
        if focused.kind == WindowKind::Outline {
            return self.outline_source;
        }
        Some(focused.buffer_id)
    }

    fn jump_focused_outline_target(&mut self, target: &str) {
        let Some(source_buffer_id) = self.outline_source_for_command().or(self.outline_source)
        else {
            self.set_status("Outline failed: source buffer is missing");
            return;
        };
        let Some(source) = self.buffer_state(source_buffer_id) else {
            self.set_status("Outline failed: source buffer is missing");
            return;
        };
        let entries = outline_entries_for_buffer(&source.buffer);
        if entries.is_empty() {
            self.set_status("Outline: no sections");
            return;
        }

        let target_index = parse_outline_target(target, &entries);
        let Some(index) = target_index else {
            self.set_status(format!("Outline: no section {target}"));
            return;
        };
        let entry = entries[index].clone();
        if !self.focus_window_for_buffer(source_buffer_id) {
            self.set_status("Outline failed: source window is missing");
            return;
        }
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: source_buffer_id,
                body_height: 1,
                body_width: 1,
            });
        if let Some(buffer) = self.buffer_state_mut(source_buffer_id) {
            let _ = buffer.buffer.set_cursor(Position::new(entry.line, 0));
            buffer.ensure_cursor_visible(context.body_height, context.body_width);
        }
        self.set_status(format!("Outline: {}", entry.label));
    }

    fn jump_current_outline_target(&mut self) {
        let Some(index) = self.current_or_next_numbered_aux_index("Outline") else {
            return;
        };
        self.jump_focused_outline_target(&(index + 1).to_string());
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

    fn jump_config_diagnostics_section(&mut self, section: ConfigDiagnosticsSection) {
        self.open_config_diagnostics_screen();
        let Some(window_id) = self.config_diagnostics_window_id() else {
            self.set_status("Config diagnostics failed: diagnostics window is missing");
            return;
        };
        let Some(buffer_id) = self.config_diagnostics_buffer_id() else {
            self.set_status("Config diagnostics failed: diagnostics buffer is missing");
            return;
        };
        let Some(line_index) = self
            .buffer_state(buffer_id)
            .and_then(|buffer| line_with_exact_text(&buffer.buffer, section.heading()))
        else {
            self.set_status(format!(
                "Config diagnostics: {} section not found",
                section.label()
            ));
            return;
        };

        self.workspace.focused = window_id;
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            let _ = buffer.buffer.set_cursor(Position::new(line_index, 0));
            buffer.ensure_cursor_visible(context.body_height, context.body_width);
        }
        self.set_status(format!("Config diagnostics: {}", section.label()));
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

    fn open_command_output_screen(&mut self, result: &CommandRunResult) {
        let text = command_output_text(result);

        if let Some(window_id) = self.command_output_window_id() {
            self.workspace.focused = window_id;
            self.refresh_command_output_buffer(&text);
            if let Ok(window) = self.workspace.window_mut(window_id) {
                window.title = "Command Output".to_string();
                window.kind = WindowKind::CommandOutput;
                window.buffer_kind = BufferKind::ReadOnly;
                window.collapsed = false;
            }
            return;
        }

        let Ok(window_id) = self.workspace.split_focused(Axis::Horizontal) else {
            self.set_status("Run command failed: focused window is missing");
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            self.set_status("Run command failed: output window is missing");
            return;
        };
        let buffer_id = window.buffer_id;
        let output = BufferState::new(buffer_id, command_output_buffer(&text));

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = output;
        } else {
            self.buffers.push(output);
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = "Command Output".to_string();
            window.kind = WindowKind::CommandOutput;
            window.buffer_kind = BufferKind::ReadOnly;
            window.collapsed = false;
        }
    }

    fn command_output_window_id(&self) -> Option<WindowId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::CommandOutput)
            .map(|window| window.id)
    }

    fn command_output_buffer_id(&self) -> Option<BufferId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::CommandOutput)
            .map(|window| window.buffer_id)
    }

    fn refresh_command_output_buffer(&mut self, text: &str) {
        let Some(buffer_id) = self.command_output_buffer_id() else {
            return;
        };
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = BufferState::new(buffer_id, command_output_buffer(text));
        }
    }

    fn run_external_command_to_buffer(&mut self, input: &str) {
        self.set_status(format!("Running command: {input}"));
        match run_command_capture(input, COMMAND_OUTPUT_STREAM_SOFT_LIMIT_BYTES) {
            Ok(result) => {
                let status = command_run_status(&result);
                self.open_command_output_screen(&result);
                self.set_status(status);
            }
            Err(error) => {
                self.set_status(format!("Run command failed: {error}"));
            }
        }
    }

    fn clear_command_output(&mut self) {
        let Some(buffer_id) = self.command_output_buffer_id() else {
            self.set_status("Command Output: no output window");
            return;
        };
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = BufferState::new(buffer_id, command_output_empty_buffer());
        }
        if let Some(window_id) = self.command_output_window_id() {
            self.workspace.focused = window_id;
        }
        self.set_status("Command Output cleared");
    }

    fn copy_command_output(&mut self) {
        let Some(text) = self.command_output_text_current() else {
            self.set_status("Command Output: no output window");
            return;
        };
        self.kill_ring = Some(text);
        self.set_status("Copied Command Output");
    }

    fn jump_command_output_summary(&mut self) {
        self.jump_command_output_line(command_output_summary_line, "summary");
    }

    fn jump_command_output_index(&mut self) {
        self.jump_command_output_line(command_output_index_line, "index");
    }

    fn jump_command_output_stdout(&mut self) {
        self.jump_command_output_line(command_output_stdout_line, "stdout");
    }

    fn jump_command_output_stdout_body(&mut self) {
        self.jump_command_output_line(command_output_stdout_body_line, "stdout body");
    }

    fn jump_command_output_stderr(&mut self) {
        self.jump_command_output_line(command_output_stderr_line, "stderr");
    }

    fn jump_command_output_stderr_body(&mut self) {
        self.jump_command_output_line(command_output_stderr_body_line, "stderr body");
    }

    fn jump_command_output_status(&mut self) {
        self.jump_command_output_line(command_output_status_line, "status");
    }

    fn jump_command_output_truncated(&mut self) {
        self.jump_command_output_line(command_output_truncated_line, "truncated");
    }

    fn jump_command_output_relative_section(&mut self, direction: SearchDirection) {
        let Some(window_id) = self.command_output_window_id() else {
            self.set_status("Command Output: no output window");
            return;
        };
        let Some(buffer_id) = self.command_output_buffer_id() else {
            self.set_status("Command Output: no output buffer");
            return;
        };
        let Some((line_index, label)) = self.buffer_state(buffer_id).and_then(|buffer| {
            command_output_relative_section_line(
                &buffer.buffer,
                buffer.buffer.cursor_position().line,
                direction,
            )
        }) else {
            self.set_status("Command Output: no sections");
            return;
        };

        self.workspace.focused = window_id;
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            let _ = buffer.buffer.set_cursor(Position::new(line_index, 0));
            buffer.ensure_cursor_visible(context.body_height, context.body_width);
        }
        self.set_status(format!("Command Output: {label}"));
    }

    fn open_command_output_section_view(&mut self, section: CommandOutputSection) {
        let Some(buffer_id) = self.command_output_buffer_id() else {
            self.set_status("Command Output: no output window");
            return;
        };
        let Some(text) = self
            .buffer_state(buffer_id)
            .and_then(|buffer| command_output_section_view_text(&buffer.buffer, section))
        else {
            self.set_status(format!(
                "Command Output: {} section not found",
                section.label()
            ));
            return;
        };

        self.open_read_only_aux_window(
            WindowKind::CommandOutputView,
            section.view_title(),
            command_output_buffer(&text),
        );
        self.set_status(format!("Command Output: only {}", section.label()));
    }

    fn jump_command_output_line(
        &mut self,
        line_finder: fn(&TextBuffer) -> Option<usize>,
        label: &'static str,
    ) {
        let Some(window_id) = self.command_output_window_id() else {
            self.set_status("Command Output: no output window");
            return;
        };
        let Some(buffer_id) = self.command_output_buffer_id() else {
            self.set_status("Command Output: no output buffer");
            return;
        };
        let Some(line_index) = self
            .buffer_state(buffer_id)
            .and_then(|buffer| line_finder(&buffer.buffer))
        else {
            self.set_status(format!("Command Output: {label} section not found"));
            return;
        };

        self.workspace.focused = window_id;
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            let _ = buffer.buffer.set_cursor(Position::new(line_index, 0));
            buffer.ensure_cursor_visible(context.body_height, context.body_width);
        }
        self.set_status(format!("Command Output: {label}"));
    }

    fn repeat_find_in_command_output(&mut self, direction: SearchDirection) {
        let Some((window_id, buffer_id)) = self.command_output_search_target() else {
            self.set_status("Command Output: no output window");
            return;
        };
        let spec = self
            .buffer_state(buffer_id)
            .and_then(|buffer| buffer.search.as_ref().map(|search| search.spec.clone()))
            .or_else(|| {
                self.last_find_query
                    .as_ref()
                    .map(|query| SearchSpec::parse(query))
                    .filter(|spec| !spec.is_empty())
            });
        let Some(spec) = spec else {
            self.set_status("Command Output find: no query");
            return;
        };

        self.workspace.focused = window_id;
        self.find_in_focused_buffer(spec, direction);
    }

    fn find_in_command_output(&mut self, spec: SearchSpec) {
        if spec.is_empty() {
            self.set_status("Command Output find: no query");
            return;
        }
        let Some((window_id, _)) = self.command_output_search_target() else {
            self.set_status("Command Output: no output window");
            return;
        };
        self.workspace.focused = window_id;
        self.last_find_query = Some(spec.input.clone());
        self.find_in_focused_buffer(spec, SearchDirection::Forward);
    }

    fn start_command_output_save_dialog(&mut self) {
        if self.command_output_buffer_id().is_none() {
            self.set_status("Command Output: no output window");
            return;
        }
        let input = self
            .recent_file_dialog_input
            .clone()
            .unwrap_or_else(|| "command-output.txt".to_string());
        self.start_file_dialog(FileDialogKind::CommandOutputSave, input);
    }

    fn save_command_output_path(&mut self, path: PathBuf) {
        let Some(text) = self.command_output_text_current() else {
            self.set_status("Command Output: no output window");
            return;
        };
        match atomic_write_text_file(&path, &text).map_err(|error| path_io_error(&path, error)) {
            Ok(report) => {
                self.set_status(status_with_atomic_temp_report(
                    format!("Saved Command Output {}", path.display()),
                    &report.temp_reconcile,
                ));
            }
            Err(error) => self.set_status(format!("Command Output save failed: {error}")),
        }
    }

    fn command_output_text_current(&self) -> Option<String> {
        if let Ok(window) = self.workspace.focused_window() {
            if window.kind == WindowKind::CommandOutputView {
                return Some(self.buffer_state(window.buffer_id)?.buffer.to_text());
            }
        }

        let buffer_id = self.command_output_buffer_id()?;
        Some(self.buffer_state(buffer_id)?.buffer.to_text())
    }

    fn command_output_search_target(&self) -> Option<(WindowId, BufferId)> {
        if let Ok(window) = self.workspace.focused_window() {
            if matches!(
                window.kind,
                WindowKind::CommandOutput | WindowKind::CommandOutputView
            ) {
                return Some((window.id, window.buffer_id));
            }
        }

        let window_id = self.command_output_window_id()?;
        let buffer_id = self.command_output_buffer_id()?;
        Some((window_id, buffer_id))
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
        let important_unbound = important_config_diagnostic_commands()
            .iter()
            .filter(|command| self.shell.keymap.sequence_for_command(command).is_none())
            .map(command_id)
            .collect::<Vec<_>>();
        let important_unbound_text = if important_unbound.is_empty() {
            "none".to_string()
        } else {
            important_unbound.join(", ")
        };

        out.push_str("Summary\n");
        out.push_str(&format!(
            "  config: {}\n",
            self.config_source.diagnostics_text()
        ));
        out.push_str(&format!(
            "  request: {}\n",
            self.config_request.diagnostics_text()
        ));
        out.push_str(&format!(
            "  terminal: {}\n",
            terminal_profile_status(self.shell.profile)
        ));
        out.push_str(&format!(
            "  theme: {} ({})\n",
            self.shell.theme.name,
            color_status(self.shell.theme.colors)
        ));
        out.push_str(&format!(
            "  mouse: {}\n",
            if self.mouse_enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));
        out.push_str(&format!(
            "  osc52: {} (max {} bytes)\n",
            if self.clipboard.osc52.enabled {
                "enabled"
            } else {
                "disabled"
            },
            self.clipboard.osc52.max_bytes
        ));
        out.push_str(&format!(
            "  keymap: {} bindings, important_unbound: {}\n",
            self.shell.keymap.bindings.len(),
            important_unbound_text
        ));

        out.push_str("\nPaths\n");
        out.push_str(&format!("  {DUN_CONFIG_ENV}: {}\n", env_config_path_text()));
        out.push_str(&format!("  default path: {}\n", default_config_path_text()));
        out.push_str("  defaults: dun --dump-config\n");

        out.push_str("\nSource\n");
        out.push_str(&format!(
            "  active: {}\n",
            self.config_source.diagnostics_text()
        ));
        out.push_str(&format!(
            "  request: {}\n",
            self.config_request.diagnostics_text()
        ));

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

        out.push_str("\nClipboard\n");
        out.push_str(&format!(
            "  osc52: {}\n",
            if self.clipboard.osc52.enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));
        out.push_str(&format!(
            "  osc52_max_bytes: {}\n",
            self.clipboard.osc52.max_bytes
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
        out.push_str(&format!(
            "  bindings: {}\n",
            self.shell.keymap.bindings.len()
        ));
        out.push_str(&format!("  important_unbound: {important_unbound_text}\n"));
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

        out.push_str("\nFile Dialog Keymap\n");
        out.push_str(&format!(
            "  bindings: {}\n",
            self.file_dialog_keys.bindings.len()
        ));
        let mut bindings = self
            .file_dialog_keys
            .bindings
            .iter()
            .map(|binding| {
                (
                    file_dialog_action_id(binding.action),
                    binding.stroke.to_string(),
                )
            })
            .collect::<Vec<_>>();
        bindings.sort_by(|left, right| left.0.cmp(right.0));
        for (action, stroke) in bindings {
            out.push_str(&format!("  {action:<28} {stroke}\n"));
        }

        out
    }

    fn close_focused_window_unchecked(&mut self) {
        let focused = self.workspace.focused_window().ok().cloned();
        let closing_buffer_id = focused.as_ref().map(|window| window.buffer_id);
        let return_buffer_id = focused
            .as_ref()
            .and_then(|window| self.auxiliary_return_buffer_id(window));
        match self.workspace.close_focused() {
            Ok(_) => {
                if let Some(buffer_id) = closing_buffer_id {
                    self.drop_buffer_if_unreferenced(buffer_id);
                }
                if let Some(buffer_id) = return_buffer_id {
                    self.focus_window_for_buffer(buffer_id);
                }
                self.set_status("Closed window");
            }
            Err(error) => {
                self.set_status(format!("Close failed: {}", workspace_error_text(error)));
            }
        }
    }

    fn auxiliary_return_buffer_id(&self, window: &WindowState) -> Option<BufferId> {
        match window.kind {
            WindowKind::Outline => self.outline_source,
            WindowKind::SearchResults => self.search_results_source,
            WindowKind::CommandOutputView => self.command_output_buffer_id(),
            _ => None,
        }
    }

    fn focused_buffer_mut(&mut self) -> Option<&mut BufferState> {
        let buffer_id = self.focused_buffer_id()?;
        self.buffer_state_mut(buffer_id)
    }

    fn focused_buffer(&self) -> Option<&BufferState> {
        let buffer_id = self.focused_buffer_id()?;
        self.buffer_state(buffer_id)
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
        self.pending_keys.clear();
        self.status_message = None;
        self.confirm = None;
        self.file_dialog = None;
        self.replace_confirm = None;
        if !matches!(kind, PromptKind::ReplaceWith) {
            self.pending_replace_query = None;
        }
        let preview = self.prompt_preview_for(kind);
        self.prompt = Some(PromptState::new(kind, initial_input, preview));
        self.refresh_prompt_preview();
    }

    fn prompt_preview_for(&self, kind: PromptKind) -> Option<PromptPreviewState> {
        if !matches!(kind, PromptKind::Find | PromptKind::ReplaceFind) {
            return None;
        }

        let buffer_id = self.focused_buffer_id()?;
        let buffer = self.buffer_state(buffer_id)?;
        Some(PromptPreviewState {
            buffer_id,
            cursor: buffer.buffer.cursor_position(),
            selection: buffer.buffer.selection(),
            search: buffer.search.clone(),
        })
    }

    fn refresh_prompt_preview(&mut self) {
        let Some(prompt) = &self.prompt else {
            return;
        };
        if !matches!(prompt.kind, PromptKind::Find | PromptKind::ReplaceFind) {
            return;
        }

        let kind = prompt.kind;
        let input = prompt.input.as_str().trim().to_string();
        let preview = prompt.preview.clone();
        if input.is_empty() {
            self.restore_prompt_preview(preview.as_ref());
            self.status_message = Some(format!("{}type to search", kind.label()));
            return;
        }

        let spec = SearchSpec::parse(&input);
        self.preview_find_query(kind, spec, preview.as_ref());
    }

    fn preview_find_query(
        &mut self,
        kind: PromptKind,
        spec: SearchSpec,
        preview: Option<&PromptPreviewState>,
    ) {
        let buffer_id = preview
            .map(|preview| preview.buffer_id)
            .or_else(|| self.focused_buffer_id());
        let Some(buffer_id) = buffer_id else {
            self.status_message = Some(format!("{}focused buffer is missing", kind.name()));
            return;
        };
        self.focus_window_for_buffer(buffer_id);

        let Some(buffer) = self.buffer_state_mut(buffer_id) else {
            self.status_message = Some(format!("{}focused buffer is missing", kind.name()));
            return;
        };
        let matches = buffer
            .buffer
            .find_all_with_options(&spec.query, spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(spec.clone(), matches, None);
            self.status_message =
                Some(format!("{}no matches for {}", kind.label(), spec.display()));
            return;
        }

        let selection = preview
            .and_then(|preview| preview_selection_match(preview.selection, &matches))
            .or_else(|| current_match_selection(&buffer.buffer, &matches))
            .unwrap_or_else(|| {
                let origin = preview
                    .map(|preview| preview.cursor)
                    .unwrap_or_else(|| buffer.buffer.cursor_position());
                choose_search_match(&matches, origin, SearchDirection::Forward)
            });
        let selected = matches[selection.index].range;
        let _ = buffer.buffer.select(selected.start, selected.end);
        let match_count = matches.len();
        buffer.set_search(spec.clone(), matches, Some(selection.index));
        self.status_message = Some(format!(
            "{}{}/{} {}",
            kind.label(),
            selection.index + 1,
            match_count,
            spec.display()
        ));
    }

    fn restore_prompt_preview(&mut self, preview: Option<&PromptPreviewState>) {
        let Some(preview) = preview else {
            return;
        };
        let Some(buffer) = self.buffer_state_mut(preview.buffer_id) else {
            return;
        };
        if let Some(selection) = preview.selection {
            let _ = buffer.buffer.select(selection.anchor, selection.cursor);
        } else {
            let _ = buffer.buffer.set_cursor(preview.cursor);
        }
        buffer.search = preview.search.clone();
    }

    fn start_file_dialog(&mut self, kind: FileDialogKind, initial_input: String) {
        self.start_file_dialog_after(kind, initial_input, None);
    }

    fn default_open_dialog_input(&self) -> String {
        self.recent_file_dialog_input.clone().unwrap_or_default()
    }

    fn start_file_dialog_after(
        &mut self,
        kind: FileDialogKind,
        initial_input: String,
        after_success: Option<PendingAction>,
    ) {
        self.pending_keys.clear();
        self.status_message = None;
        self.confirm = None;
        self.prompt = None;
        self.replace_confirm = None;
        self.pending_replace_query = None;
        self.file_dialog = Some(FileDialogState::new(kind, initial_input, after_success));
    }

    fn start_confirm(&mut self, action: PendingAction, buffer_id: BufferId) {
        self.pending_keys.clear();
        self.status_message = None;
        self.prompt = None;
        self.file_dialog = None;
        self.replace_confirm = None;
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
            self.start_file_dialog_after(
                FileDialogKind::SaveAs,
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
            PendingAction::OpenPrompt => {
                self.start_file_dialog(FileDialogKind::Open, self.default_open_dialog_input())
            }
            PendingAction::ReloadBuffer => {
                if let Err(error) = self.reload_focused_buffer() {
                    self.set_status(format!("Reload failed: {error}"));
                }
            }
            PendingAction::CloseWindow => self.close_focused_window_unchecked(),
        }
    }

    fn handle_file_dialog_key_event(&mut self, event: CrosstermKeyEvent) -> bool {
        if self.file_dialog.is_none() {
            return false;
        }

        if let Some(action) = key_stroke_from_crossterm(event)
            .and_then(|stroke| self.file_dialog_keys.action_for_stroke(stroke))
        {
            self.handle_file_dialog_action(action);
            return true;
        }

        if let Some(ch) = text_input_from_crossterm(event) {
            if let Some(dialog) = &mut self.file_dialog {
                dialog.insert_char(ch);
            }
        }

        self.refresh_prompt_preview();
        true
    }

    fn handle_file_dialog_action(&mut self, action: FileDialogAction) {
        match action {
            FileDialogAction::Cancel => self.cancel_file_dialog(),
            FileDialogAction::Submit => self.submit_file_dialog(),
            FileDialogAction::CompleteForward => self.complete_file_dialog(true),
            FileDialogAction::CompleteBackward => self.complete_file_dialog(false),
            FileDialogAction::ToggleHidden => self.toggle_file_dialog_hidden(),
            FileDialogAction::MoveSelectionUp => self.move_file_dialog_selection(-1),
            FileDialogAction::MoveSelectionDown => self.move_file_dialog_selection(1),
            FileDialogAction::PageSelectionUp => self.page_file_dialog_selection(-1),
            FileDialogAction::PageSelectionDown => self.page_file_dialog_selection(1),
            FileDialogAction::MoveInputLeft => self.move_file_dialog_input_left(),
            FileDialogAction::MoveInputRight => self.move_file_dialog_input_right(),
            FileDialogAction::MoveInputStart => self.move_file_dialog_input_start(),
            FileDialogAction::MoveInputEnd => self.move_file_dialog_input_end(),
            FileDialogAction::DeleteBackward => self.delete_file_dialog_backward(),
            FileDialogAction::DeleteForward => self.delete_file_dialog_forward(),
        }
    }

    fn cancel_file_dialog(&mut self) {
        if let Some(dialog) = self.file_dialog.take() {
            self.set_status(format!("{} cancelled", dialog.kind.name()));
        }
    }

    fn move_file_dialog_selection(&mut self, delta: isize) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.move_selection(delta);
        }
    }

    fn page_file_dialog_selection(&mut self, delta: isize) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.page_selection(delta);
        }
    }

    fn scroll_file_dialog(&mut self, delta: isize) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.scroll(delta);
        }
    }

    fn move_file_dialog_input_left(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.move_input_left();
        }
    }

    fn move_file_dialog_input_right(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.move_input_right();
        }
    }

    fn move_file_dialog_input_start(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.move_input_start();
        }
    }

    fn move_file_dialog_input_end(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.move_input_end();
        }
    }

    fn delete_file_dialog_backward(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.delete_backward();
        }
    }

    fn delete_file_dialog_forward(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.delete_forward();
        }
    }

    fn toggle_file_dialog_hidden(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.toggle_hidden();
        }
    }

    fn complete_file_dialog(&mut self, forward: bool) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.complete(forward);
        }
    }

    fn submit_file_dialog(&mut self) {
        let Some(mut dialog) = self.file_dialog.take() else {
            return;
        };
        let submit = dialog.submit();
        self.finish_file_dialog_submit(dialog, submit);
    }

    fn click_file_dialog_visible_index(&mut self, visible_index: usize) {
        let Some(mut dialog) = self.file_dialog.take() else {
            return;
        };
        let submit = dialog.click_visible_entry(visible_index);
        self.finish_file_dialog_submit(dialog, submit);
    }

    fn finish_file_dialog_submit(&mut self, dialog: FileDialogState, submit: FileDialogSubmit) {
        match submit {
            FileDialogSubmit::Cancel => {
                self.set_status(format!("{} cancelled", dialog.kind.name()));
            }
            FileDialogSubmit::ContinueEditing => {
                self.file_dialog = Some(dialog);
            }
            FileDialogSubmit::Path(path) => match dialog.kind {
                FileDialogKind::Open => {
                    if let Err(error) = self.open_file_path(path.clone()) {
                        let status = format!("Open failed: {error}");
                        let mut dialog = dialog;
                        dialog.message = Some(status.clone());
                        self.file_dialog = Some(dialog);
                        self.set_status(status);
                    } else {
                        self.note_recent_file_dialog_path(&path);
                    }
                }
                FileDialogKind::SaveAs => {
                    if let Err(error) = self.save_focused_buffer_as(path.clone()) {
                        let status = format!("Save As failed: {error}");
                        let mut dialog = dialog;
                        dialog.message = Some(status.clone());
                        self.file_dialog = Some(dialog);
                        self.set_status(status);
                    } else if let Some(action) = dialog.after_success {
                        self.note_recent_file_dialog_path(&path);
                        self.continue_pending_action(action);
                    } else {
                        self.note_recent_file_dialog_path(&path);
                    }
                }
                FileDialogKind::CommandOutputSave => {
                    let Some(text) = self.command_output_text_current() else {
                        let status = "Command Output: no output window".to_string();
                        let mut dialog = dialog;
                        dialog.message = Some(status.clone());
                        self.file_dialog = Some(dialog);
                        self.set_status(status);
                        return;
                    };
                    match atomic_write_text_file(&path, &text)
                        .map_err(|error| path_io_error(&path, error))
                    {
                        Ok(report) => {
                            self.note_recent_file_dialog_path(&path);
                            self.set_status(status_with_atomic_temp_report(
                                format!("Saved Command Output {}", path.display()),
                                &report.temp_reconcile,
                            ));
                        }
                        Err(error) => {
                            let status = format!("Command Output save failed: {error}");
                            let mut dialog = dialog;
                            dialog.message = Some(status.clone());
                            self.file_dialog = Some(dialog);
                            self.set_status(status);
                        }
                    }
                }
            },
        }
    }

    fn note_recent_file_dialog_path(&mut self, path: &Path) {
        self.recent_file_dialog_input = Some(file_dialog_recent_input_for_path(path));
    }

    fn start_buffer_switcher(&mut self) {
        if self.buffers.len() <= 1 {
            self.set_status("Buffer switcher: only one buffer");
            return;
        }

        let selected = self
            .focused_buffer_id()
            .and_then(|id| self.buffers.iter().position(|buffer| buffer.id == id))
            .unwrap_or(0);
        self.clear_active_menu();
        self.pending_keys.clear();
        self.buffer_switcher = Some(BufferSwitcherState::new(selected, self.buffers.len()));
        self.set_status("Switch buffer");
    }

    fn handle_buffer_switcher_key_event(&mut self, event: CrosstermKeyEvent) -> bool {
        if self.buffer_switcher.is_none() {
            return false;
        }

        match event.code {
            CrosstermKeyCode::Esc => self.cancel_buffer_switcher(),
            CrosstermKeyCode::Enter => self.submit_buffer_switcher(),
            CrosstermKeyCode::Up => self.move_buffer_switcher_selection(-1),
            CrosstermKeyCode::Down => self.move_buffer_switcher_selection(1),
            CrosstermKeyCode::PageUp => self.page_buffer_switcher_selection(-1),
            CrosstermKeyCode::PageDown => self.page_buffer_switcher_selection(1),
            _ => {}
        }

        true
    }

    fn cancel_buffer_switcher(&mut self) {
        self.buffer_switcher = None;
        self.set_status("Switch buffer cancelled");
    }

    fn move_buffer_switcher_selection(&mut self, delta: isize) {
        if let Some(switcher) = &mut self.buffer_switcher {
            switcher.move_selection(delta, self.buffers.len());
        }
    }

    fn page_buffer_switcher_selection(&mut self, delta: isize) {
        if let Some(switcher) = &mut self.buffer_switcher {
            switcher.page_selection(delta, self.buffers.len());
        }
    }

    fn scroll_buffer_switcher(&mut self, delta: isize) {
        self.move_buffer_switcher_selection(delta);
    }

    fn submit_buffer_switcher(&mut self) {
        let Some(switcher) = self.buffer_switcher.take() else {
            return;
        };
        let Some(index) = switcher.selected_index(self.buffers.len()) else {
            self.set_status("Switch buffer failed: no buffers");
            return;
        };
        self.switch_to_buffer_index(index);
    }

    fn click_buffer_switcher_visible_index(&mut self, visible_index: usize) {
        let Some(mut switcher) = self.buffer_switcher.take() else {
            return;
        };
        let Some(index) = switcher.select_visible_index(visible_index, self.buffers.len()) else {
            self.buffer_switcher = Some(switcher);
            return;
        };
        self.switch_to_buffer_index(index);
    }

    fn switch_to_buffer_index(&mut self, index: usize) {
        let Some(buffer) = self.buffers.get(index) else {
            self.set_status("Switch buffer failed: buffer is missing");
            return;
        };
        let buffer_id = buffer.id;
        let display_name = self.buffer_display_name(buffer_id);
        if self.focus_window_for_buffer(buffer_id) {
            self.set_status(format!("Switched to {display_name}"));
        } else {
            self.set_status(format!(
                "Switch buffer failed: {display_name} has no window"
            ));
        }
    }

    fn handle_buffer_switcher_mouse_down(&mut self, screen_x: u16, screen_y: u16) -> bool {
        let Some(overlay) = self.active_overlay() else {
            return false;
        };
        let Some(visible_index) =
            self.shell
                .hit_test_overlay_list(&overlay, self.overlay_area(), screen_x, screen_y)
        else {
            return false;
        };

        self.pending_keys.clear();
        self.click_buffer_switcher_visible_index(visible_index);
        true
    }

    fn buffer_switcher_overlay(&self, switcher: &BufferSwitcherState) -> UiOverlay {
        let entries = self.buffer_switcher_entries();
        let (list, selected) = switcher.visible_entry_texts(&entries);
        let mut lines = vec![format!("Open buffers: {}", entries.len())];
        if entries.len() > BUFFER_SWITCHER_VISIBLE_ENTRIES {
            if let Some((start, end, _)) = switcher.visible_entry_range(entries.len()) {
                lines.push(format!(
                    "Showing {}-{} of {} buffers",
                    start + 1,
                    end,
                    entries.len()
                ));
            }
        }
        let mut overlay = UiOverlay::message(
            "Switch Buffer",
            lines,
            vec!["[Enter] Switch  [Esc] Cancel".to_string()],
        )
        .with_list(list, selected, 48);
        if let Some((start, end, _)) = switcher.visible_entry_range(entries.len()) {
            overlay = overlay.with_list_overflow(start > 0, end < entries.len());
        }
        overlay
    }

    fn buffer_switcher_entries(&self) -> Vec<BufferSwitcherEntry> {
        let focused = self.focused_buffer_id();
        self.buffers
            .iter()
            .map(|buffer| {
                let active = if Some(buffer.id) == focused { ">" } else { " " };
                let dirty = if buffer.buffer.is_dirty() { "*" } else { " " };
                let disk = buffer_disk_state(buffer)
                    .map(|state| format!(" {state}"))
                    .unwrap_or_default();
                let name = self.buffer_display_name(buffer.id);
                let path = buffer
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "(no path)".to_string());
                BufferSwitcherEntry {
                    buffer_id: buffer.id,
                    text: format!("{active} {dirty} {name}{disk}  {path}"),
                }
            })
            .collect()
    }

    fn handle_paste(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        self.pending_keys.clear();

        if self.confirm.is_some() {
            self.set_status("Paste ignored during confirmation");
            return;
        }
        if self.replace_confirm.is_some() {
            self.set_status("Paste ignored during replace confirmation");
            return;
        }
        if self.buffer_switcher.is_some() {
            self.set_status("Paste ignored during buffer switcher");
            return;
        }

        if let Some(dialog) = &mut self.file_dialog {
            let text = single_line_paste_text(text);
            dialog.insert_text(&text);
            return;
        }

        if let Some(prompt) = &mut self.prompt {
            prompt.detach_history();
            let text = single_line_paste_text(text);
            prompt.input.insert_str(&text);
            self.refresh_prompt_preview();
            return;
        }

        self.clear_active_menu();
        let Some(buffer) = self.focused_buffer_mut() else {
            return;
        };

        if let Err(error) = buffer.buffer.insert_str(text) {
            self.set_status(format!("Paste failed: {}", buffer_error_text(error)));
        }
    }

    fn note_right_click_paste(&mut self) {
        self.set_status("Paste: waiting for terminal bracketed paste data");
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
                self.recall_previous_prompt_history();
            }
            CrosstermKeyCode::Down => {
                self.recall_next_prompt_history();
            }
            CrosstermKeyCode::Tab => {
                self.complete_command_line_prompt(true);
            }
            CrosstermKeyCode::BackTab => {
                self.complete_command_line_prompt(false);
            }
            CrosstermKeyCode::Left => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.clear_completion();
                    prompt.input.move_left();
                }
            }
            CrosstermKeyCode::Right => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.clear_completion();
                    prompt.input.move_right();
                }
            }
            CrosstermKeyCode::Home => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.clear_completion();
                    prompt.input.move_start();
                }
            }
            CrosstermKeyCode::End => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.clear_completion();
                    prompt.input.move_end();
                }
            }
            CrosstermKeyCode::Delete => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.detach_history();
                    prompt.clear_completion();
                    prompt.input.delete_forward();
                }
            }
            CrosstermKeyCode::Backspace => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.detach_history();
                    prompt.clear_completion();
                    prompt.input.delete_backward();
                }
            }
            _ => {
                if let Some(ch) = text_input_from_crossterm(event) {
                    if let Some(prompt) = &mut self.prompt {
                        prompt.detach_history();
                        prompt.clear_completion();
                        prompt.input.insert_char(ch);
                    }
                }
            }
        }

        self.refresh_prompt_preview();
        true
    }

    fn complete_command_line_prompt(&mut self, forward: bool) {
        let Some(prompt) = &mut self.prompt else {
            return;
        };
        if prompt.kind != PromptKind::CommandLine {
            return;
        }
        let input = prompt.input.as_str().to_string();
        if prompt.input.cursor_index != input.len() {
            prompt.clear_completion();
            self.status_message = Some("Command completion: move cursor to end".to_string());
            return;
        }

        if let Some(replacement) = prompt.next_completion_replacement(&input, forward) {
            prompt.detach_history();
            prompt.input.set_text(replacement);
            self.status_message = prompt
                .completion
                .as_ref()
                .map(PromptCompletionState::status_text)
                .or_else(|| Some("Command completion".to_string()));
            return;
        }

        let completion = command_line_completion(&input);
        match completion {
            CommandCompletion::None => {
                prompt.clear_completion();
                self.status_message = Some("Command completion: no matches".to_string());
            }
            CommandCompletion::Unique(text) => {
                prompt.detach_history();
                prompt.clear_completion();
                prompt.input.set_text(text);
                self.status_message = Some("Command completion".to_string());
            }
            CommandCompletion::CommonPrefix(text, count) => {
                prompt.detach_history();
                prompt.clear_completion();
                prompt.input.set_text(text);
                self.status_message = Some(format!("Command completion: {count} matches"));
            }
            CommandCompletion::Candidates(candidates) => {
                prompt.completion = Some(PromptCompletionState::new(input, candidates));
                self.status_message = prompt
                    .completion
                    .as_ref()
                    .map(PromptCompletionState::status_text);
            }
        }
    }

    fn recall_previous_prompt_history(&mut self) {
        let Some(kind) = self
            .prompt
            .as_ref()
            .and_then(|prompt| prompt.kind.history_kind())
        else {
            return;
        };
        let history_len = self.prompt_history_len(kind);
        if history_len == 0 {
            return;
        }

        let next_index = {
            let Some(prompt) = self.prompt_history_prompt_mut(kind) else {
                return;
            };
            let next_index = match prompt.history_index {
                Some(0) => 0,
                Some(index) => index - 1,
                None => {
                    prompt.history_draft = prompt.input.as_str().to_string();
                    history_len - 1
                }
            };
            prompt.history_index = Some(next_index);
            prompt.clear_completion();
            next_index
        };

        let Some(input) = self.prompt_history_entry(kind, next_index) else {
            return;
        };
        if let Some(prompt) = self.prompt_history_prompt_mut(kind) {
            prompt.input.set_text(input);
        }
    }

    fn recall_next_prompt_history(&mut self) {
        let Some(kind) = self
            .prompt
            .as_ref()
            .and_then(|prompt| prompt.kind.history_kind())
        else {
            return;
        };
        let history_len = self.prompt_history_len(kind);
        let (entry_index, draft) = {
            let Some(prompt) = self.prompt_history_prompt_mut(kind) else {
                return;
            };
            let Some(index) = prompt.history_index else {
                return;
            };
            if index + 1 < history_len {
                let next_index = index + 1;
                prompt.history_index = Some(next_index);
                prompt.clear_completion();
                (Some(next_index), None)
            } else {
                prompt.history_index = None;
                prompt.clear_completion();
                (None, Some(std::mem::take(&mut prompt.history_draft)))
            }
        };

        let input = entry_index
            .and_then(|index| self.prompt_history_entry(kind, index))
            .or(draft)
            .unwrap_or_default();
        if let Some(prompt) = self.prompt_history_prompt_mut(kind) {
            prompt.input.set_text(input);
        }
    }

    fn prompt_history_prompt_mut(&mut self, kind: PromptHistoryKind) -> Option<&mut PromptState> {
        self.prompt
            .as_mut()
            .filter(|prompt| prompt.kind.history_kind() == Some(kind))
    }

    fn prompt_history_len(&self, kind: PromptHistoryKind) -> usize {
        self.prompt_history(kind).len()
    }

    fn prompt_history_entry(&self, kind: PromptHistoryKind, index: usize) -> Option<String> {
        self.prompt_history(kind).get(index).cloned()
    }

    fn prompt_history(&self, kind: PromptHistoryKind) -> &[String] {
        match kind {
            PromptHistoryKind::CommandLine => &self.command_history,
            PromptHistoryKind::RunCommand => &self.run_command_history,
        }
    }

    fn prompt_history_mut(&mut self, kind: PromptHistoryKind) -> &mut Vec<String> {
        match kind {
            PromptHistoryKind::CommandLine => &mut self.command_history,
            PromptHistoryKind::RunCommand => &mut self.run_command_history,
        }
    }

    fn cancel_prompt(&mut self) {
        if let Some(prompt) = self.prompt.take() {
            self.restore_prompt_preview(prompt.preview.as_ref());
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
            PromptKind::Find => {
                let input = prompt.input.as_str().trim().to_string();
                let spec = SearchSpec::parse(&input);
                if spec.is_empty() {
                    self.set_status(format!("{} cancelled", prompt.kind.name()));
                    return;
                }

                self.last_find_query = Some(input.clone());
                self.commit_find_preview(spec);
            }
            PromptKind::ReplaceFind => {
                let input = prompt.input.as_str().trim().to_string();
                let spec = SearchSpec::parse(&input);
                if spec.is_empty() {
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
                self.start_replace_confirmation(
                    SearchSpec::parse(&query),
                    prompt.input.as_str().to_string(),
                );
            }
            PromptKind::GoToLine => {
                let input = prompt.input.as_str().trim();
                if input.is_empty() {
                    self.set_status(format!("{} cancelled", prompt.kind.name()));
                    return;
                }

                self.go_to_line(input);
            }
            PromptKind::RunCommand => {
                let input = prompt.input.as_str().trim().to_string();
                if input.is_empty() {
                    self.set_status(format!("{} cancelled", prompt.kind.name()));
                    return;
                }

                self.record_prompt_history(PromptHistoryKind::RunCommand, input.clone());
                self.run_external_command_to_buffer(&input);
            }
            PromptKind::CommandLine => {
                let input = prompt.input.as_str().trim().to_string();
                if input.is_empty() {
                    self.set_status(format!("{} cancelled", prompt.kind.name()));
                    return;
                }

                self.record_command_history(input.clone());
                self.run_command_line(&input);
            }
        }
    }

    fn commit_find_preview(&mut self, spec: SearchSpec) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Find: focused buffer is missing");
            return;
        };
        let matches = buffer
            .buffer
            .find_all_with_options(&spec.query, spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(spec.clone(), matches, None);
            self.set_status(format!("Find: no matches for {}", spec.display()));
            return;
        }

        let selection = current_match_selection(&buffer.buffer, &matches).unwrap_or_else(|| {
            choose_search_match(
                &matches,
                buffer.buffer.cursor_position(),
                SearchDirection::Forward,
            )
        });
        let selected = matches[selection.index].range;
        let _ = buffer.buffer.select(selected.start, selected.end);
        let match_count = matches.len();
        buffer.set_search(spec.clone(), matches, Some(selection.index));
        self.set_status(format!(
            "Find: {}/{} {}",
            selection.index + 1,
            match_count,
            spec.display()
        ));
    }

    fn start_replace_confirmation(&mut self, spec: SearchSpec, replacement: String) {
        if spec.is_empty() {
            self.set_status("Replace: no query");
            return;
        }

        let Some(buffer_id) = self.focused_buffer_id() else {
            self.set_status("Replace: focused buffer is missing");
            return;
        };
        self.replace_confirm = Some(ReplaceConfirmState {
            buffer_id,
            spec,
            replacement,
            replaced: 0,
            skipped: 0,
            skipped_in_cycle: 0,
        });
        if !self.select_replace_confirm_match(SearchDirection::Forward) {
            self.replace_confirm = None;
        }
    }

    fn handle_replace_confirm_key_event(&mut self, event: CrosstermKeyEvent) -> bool {
        if self.replace_confirm.is_none() {
            return false;
        }

        match event.code {
            CrosstermKeyCode::Esc => self.cancel_replace_confirmation(),
            CrosstermKeyCode::Enter => self.replace_confirm_current(),
            CrosstermKeyCode::Char(ch) => match ch.to_ascii_lowercase() {
                'r' => self.replace_confirm_current(),
                's' => self.skip_replace_confirm_current(),
                'a' => self.replace_confirm_all(),
                'c' => self.cancel_replace_confirmation(),
                _ => {}
            },
            _ => {}
        }

        true
    }

    fn cancel_replace_confirmation(&mut self) {
        let Some(confirm) = self.replace_confirm.take() else {
            return;
        };
        self.set_status(format!(
            "Replace cancelled: {} replaced, {} skipped",
            confirm.replaced, confirm.skipped
        ));
    }

    fn select_replace_confirm_match(&mut self, direction: SearchDirection) -> bool {
        let Some(confirm) = self.replace_confirm.clone() else {
            return false;
        };
        self.focus_window_for_buffer(confirm.buffer_id);
        let Some(buffer) = self.buffer_state_mut(confirm.buffer_id) else {
            self.set_status("Replace: focused buffer is missing");
            return false;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&confirm.spec.query, confirm.spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(confirm.spec.clone(), matches, None);
            if confirm.replaced == 0 && confirm.skipped == 0 {
                self.set_status(format!(
                    "Replace: no matches for {}",
                    confirm.spec.display()
                ));
            } else {
                self.set_status(format!(
                    "Replace done: {} replaced, {} skipped",
                    confirm.replaced, confirm.skipped
                ));
            }
            return false;
        }

        let selection = current_match_selection(&buffer.buffer, &matches).unwrap_or_else(|| {
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
            choose_search_match(&matches, origin, direction)
        });
        let selected = matches[selection.index].range;
        let _ = buffer.buffer.select(selected.start, selected.end);
        let match_count = matches.len();
        buffer.set_search(confirm.spec.clone(), matches, Some(selection.index));
        self.status_message = Some(format!(
            "Replace confirm: {}/{} {} -> {}",
            selection.index + 1,
            match_count,
            confirm.spec.display(),
            replacement_status_text(&confirm.replacement)
        ));
        true
    }

    fn replace_confirm_current(&mut self) {
        let Some(mut confirm) = self.replace_confirm.clone() else {
            return;
        };
        self.focus_window_for_buffer(confirm.buffer_id);
        let Some(buffer) = self.buffer_state_mut(confirm.buffer_id) else {
            self.set_status("Replace: focused buffer is missing");
            self.replace_confirm = None;
            return;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&confirm.spec.query, confirm.spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(confirm.spec.clone(), matches, None);
            self.set_status(format!(
                "Replace done: {} replaced, {} skipped",
                confirm.replaced, confirm.skipped
            ));
            self.replace_confirm = None;
            return;
        }

        let selection = current_match_selection(&buffer.buffer, &matches).unwrap_or_else(|| {
            choose_search_match(
                &matches,
                buffer.buffer.cursor_position(),
                SearchDirection::Forward,
            )
        });
        let target = matches[selection.index].range;
        match buffer.buffer.replace_range(target, &confirm.replacement) {
            Ok(()) => {
                confirm.replaced += 1;
                confirm.skipped_in_cycle = 0;
                self.replace_confirm = Some(confirm);
                if !self.select_replace_confirm_match(SearchDirection::Forward) {
                    self.replace_confirm = None;
                }
            }
            Err(error) => {
                self.replace_confirm = None;
                self.set_status(format!("Replace failed: {}", buffer_error_text(error)));
            }
        }
    }

    fn skip_replace_confirm_current(&mut self) {
        let Some(mut confirm) = self.replace_confirm.clone() else {
            return;
        };
        self.focus_window_for_buffer(confirm.buffer_id);
        let Some(buffer) = self.buffer_state_mut(confirm.buffer_id) else {
            self.set_status("Replace: focused buffer is missing");
            self.replace_confirm = None;
            return;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&confirm.spec.query, confirm.spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(confirm.spec.clone(), matches, None);
            self.replace_confirm = None;
            self.set_status(format!(
                "Replace done: {} replaced, {} skipped",
                confirm.replaced, confirm.skipped
            ));
            return;
        }
        if matches.len() <= 1 || confirm.skipped_in_cycle + 1 >= matches.len() {
            confirm.skipped += 1;
            self.replace_confirm = None;
            self.set_status(format!(
                "Replace done: {} replaced, {} skipped",
                confirm.replaced, confirm.skipped
            ));
            return;
        }

        if let Some(selection) = current_match_selection(&buffer.buffer, &matches) {
            let _ = buffer.buffer.set_cursor(matches[selection.index].range.end);
        }
        confirm.skipped += 1;
        confirm.skipped_in_cycle += 1;
        self.replace_confirm = Some(confirm);
        let _ = self.select_replace_confirm_match(SearchDirection::Forward);
    }

    fn replace_confirm_all(&mut self) {
        let Some(confirm) = self.replace_confirm.take() else {
            return;
        };
        self.focus_window_for_buffer(confirm.buffer_id);
        let Some(buffer) = self.buffer_state_mut(confirm.buffer_id) else {
            self.set_status("Replace All: focused buffer is missing");
            return;
        };

        match buffer.buffer.replace_all_with_options(
            &confirm.spec.query,
            &confirm.replacement,
            confirm.spec.options,
        ) {
            Ok(count) => {
                let new_matches = buffer
                    .buffer
                    .find_all_with_options(&confirm.spec.query, confirm.spec.options);
                let remaining = new_matches.len();
                buffer.set_search(confirm.spec.clone(), new_matches, None);
                let total = confirm.replaced + count;
                let suffix = if remaining == 0 {
                    String::new()
                } else {
                    format!("; {remaining} matches remain")
                };
                self.set_status(format!(
                    "Replace All: {total} {} -> {}{suffix}",
                    confirm.spec.display(),
                    replacement_status_text(&confirm.replacement)
                ));
            }
            Err(error) => {
                self.set_status(format!("Replace All failed: {}", buffer_error_text(error)))
            }
        }
    }

    fn record_command_history(&mut self, input: String) {
        self.record_prompt_history(PromptHistoryKind::CommandLine, input);
    }

    fn record_prompt_history(&mut self, kind: PromptHistoryKind, input: String) {
        let history = self.prompt_history_mut(kind);
        if history.last().is_some_and(|previous| previous == &input) {
            return;
        }

        history.push(input);
        if history.len() > COMMAND_HISTORY_LIMIT {
            let overflow = history.len() - COMMAND_HISTORY_LIMIT;
            history.drain(0..overflow);
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
            "config" | "diagnostics" | "configdiagnostics" => {
                self.run_config_diagnostics_command(args)
            }
            "reload" | "reloadconfig" => self.reload_config(),
            "status" | "statushistory" => self.open_status_history_screen(),
            "theme" => self.run_theme_command(args),
            "quit" | "q" => self.handle_app_command(&AppCommand::Quit),
            "shell" | "sh" => {
                self.run_no_arg_command(args, EditorCommand::App(AppCommand::ShellEscape))
            }
            "run" | "command" => self.run_external_command_line(args),
            "output" | "commandoutput" => self.run_command_output_command(args),
            "outline" | "sections" => self.run_outline_command(args),
            "open" | "o" => self.run_open_command(args),
            "results" | "searchresults" | "matches" => self.run_search_results_command(args),
            "buffers" | "switch" | "switchbuffer" => {
                self.run_no_arg_command(args, EditorCommand::File(FileCommand::SwitchBuffer))
            }
            "save" | "write" | "w" => self.run_save_command(args),
            "saveas" | "writeas" => self.run_save_as_command(args),
            "new" => self.run_no_arg_command(args, EditorCommand::File(FileCommand::New)),
            "reloadfile" => self.run_no_arg_command(args, EditorCommand::File(FileCommand::Reload)),
            "close" => self.run_no_arg_command(args, EditorCommand::File(FileCommand::Close)),
            "wrap" => {
                self.run_no_arg_command(args, EditorCommand::Edit(EditCommand::ToggleWordWrap))
            }
            "whitespace" => self.run_no_arg_command(
                args,
                EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace),
            ),
            "mark" | "bookmark" => {
                self.run_no_arg_command(args, EditorCommand::Edit(EditCommand::ToggleBookmark))
            }
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

    fn run_external_command_line(&mut self, args: &[String]) {
        match args {
            [] => self.handle_app_command(&AppCommand::RunCommand),
            [command] => self.run_external_command_to_buffer(command),
            _ => self.set_status("Command failed: run expects zero args or one quoted command"),
        }
    }

    fn run_command_output_command(&mut self, args: &[String]) {
        match args {
            [action] if normalize_command_line_token(action) == "clear" => {
                self.handle_app_command(&AppCommand::CommandOutputClear)
            }
            [action] if normalize_command_line_token(action) == "copy" => {
                self.handle_app_command(&AppCommand::CommandOutputCopy)
            }
            [action] if normalize_command_line_token(action) == "index" => {
                self.handle_app_command(&AppCommand::CommandOutputIndex)
            }
            [action]
                if matches!(
                    normalize_command_line_token(action).as_str(),
                    "next" | "nextmatch"
                ) =>
            {
                self.handle_app_command(&AppCommand::CommandOutputNextMatch)
            }
            [action]
                if matches!(
                    normalize_command_line_token(action).as_str(),
                    "nextsection" | "sectionnext"
                ) =>
            {
                self.handle_app_command(&AppCommand::CommandOutputNextSection)
            }
            [action, section] if normalize_command_line_token(action) == "only" => {
                match parse_command_output_section(section) {
                    Some(CommandOutputSection::Stdout) => {
                        self.handle_app_command(&AppCommand::CommandOutputOnlyStdout)
                    }
                    Some(CommandOutputSection::Stderr) => {
                        self.handle_app_command(&AppCommand::CommandOutputOnlyStderr)
                    }
                    None => self.set_status("Command failed: output only expects stdout or stderr"),
                }
            }
            [action]
                if matches!(
                    normalize_command_line_token(action).as_str(),
                    "previous" | "prev" | "prevmatch" | "previousmatch"
                ) =>
            {
                self.handle_app_command(&AppCommand::CommandOutputPreviousMatch)
            }
            [action]
                if matches!(
                    normalize_command_line_token(action).as_str(),
                    "previoussection" | "prevsection" | "sectionprevious" | "sectionprev"
                ) =>
            {
                self.handle_app_command(&AppCommand::CommandOutputPreviousSection)
            }
            [action] if normalize_command_line_token(action) == "summary" => {
                self.handle_app_command(&AppCommand::CommandOutputSummary)
            }
            [action] if normalize_command_line_token(action) == "status" => {
                self.handle_app_command(&AppCommand::CommandOutputStatus)
            }
            [action] if normalize_command_line_token(action) == "stdout" => {
                self.handle_app_command(&AppCommand::CommandOutputStdout)
            }
            [action]
                if matches!(
                    normalize_command_line_token(action).as_str(),
                    "stdoutbody" | "outbody"
                ) =>
            {
                self.handle_app_command(&AppCommand::CommandOutputStdoutBody)
            }
            [action] if normalize_command_line_token(action) == "stderr" => {
                self.handle_app_command(&AppCommand::CommandOutputStderr)
            }
            [action]
                if matches!(
                    normalize_command_line_token(action).as_str(),
                    "stderrbody" | "errbody"
                ) =>
            {
                self.handle_app_command(&AppCommand::CommandOutputStderrBody)
            }
            [action]
                if matches!(
                    normalize_command_line_token(action).as_str(),
                    "truncated" | "truncate" | "trunc"
                ) =>
            {
                self.handle_app_command(&AppCommand::CommandOutputTruncated)
            }
            [action, query] if normalize_command_line_token(action) == "find" => {
                self.find_in_command_output(SearchSpec::parse(query))
            }
            [action] if normalize_command_line_token(action) == "save" => {
                self.handle_app_command(&AppCommand::CommandOutputSave)
            }
            [action, path] if normalize_command_line_token(action) == "save" => {
                self.save_command_output_path(PathBuf::from(path))
            }
            _ => self.set_status(
                "Command failed: output expects index, summary, status, stdout, stdout-body, stderr, stderr-body, truncated, only stdout|stderr, find QUERY, next, previous, next-section, previous-section, clear, copy, or save PATH",
            ),
        }
    }

    fn run_config_diagnostics_command(&mut self, args: &[String]) {
        match args {
            [] => self.open_config_diagnostics_screen(),
            [section] => match parse_config_diagnostics_section(section) {
                Some(section) => self.jump_config_diagnostics_section(section),
                None => self.set_status(format!(
                    "Command failed: config expects one of {}",
                    config_diagnostics_section_values()
                )),
            },
            _ => self.set_status(format!(
                "Command failed: config expects zero args or one of {}",
                config_diagnostics_section_values()
            )),
        }
    }

    fn run_outline_command(&mut self, args: &[String]) {
        match args {
            [] => self.open_outline_screen(),
            [target] => self.jump_focused_outline_target(target),
            _ => self
                .set_status("Command failed: outline expects zero args or one section number/name"),
        }
    }

    fn run_search_results_command(&mut self, args: &[String]) {
        match args {
            [] => self.open_search_results_screen(),
            [index] => self.jump_search_result(index),
            _ => self.set_status("Command failed: results expects zero args or one match number"),
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
                self.find_in_focused_buffer(SearchSpec::parse(query), SearchDirection::Forward);
            }
            _ => self.set_status("Command failed: find expects zero or one query"),
        }
    }

    fn run_replace_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_edit_command(&EditCommand::Replace),
            [mode, query, replacement] if normalize_command_line_token(mode) == "all" => {
                self.last_find_query = Some(query.clone());
                self.replace_all_in_focused_buffer(SearchSpec::parse(query), replacement);
            }
            [query, replacement] => {
                self.last_find_query = Some(query.clone());
                self.replace_in_focused_buffer(SearchSpec::parse(query), replacement);
            }
            _ => self.set_status(
                "Command failed: replace expects query and replacement, or all query replacement",
            ),
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

        let spec = SearchSpec::parse(&query);
        if spec.is_empty() {
            self.set_status("Find: no query");
            return;
        }

        self.find_in_focused_buffer(spec, direction);
    }

    fn open_search_results_screen(&mut self) {
        let Some(source_buffer_id) = self.search_results_source_for_command() else {
            self.set_status("Search Results: focused buffer is missing");
            return;
        };
        let Some((spec, matches)) = self.search_results_for_source(source_buffer_id) else {
            self.set_status("Search Results: no query");
            return;
        };
        if matches.is_empty() {
            self.set_status(format!("Search Results: no matches for {}", spec.display()));
            return;
        }

        let source_name = self.buffer_display_name(source_buffer_id);
        let text = search_results_text(
            &source_name,
            &spec,
            &matches,
            &self.buffer_state(source_buffer_id).unwrap().buffer,
        );
        self.search_results_source = Some(source_buffer_id);
        self.open_read_only_aux_window(
            WindowKind::SearchResults,
            "Search Results",
            search_results_buffer(&text),
        );
        self.set_status(format!("Search Results: {} match(es)", matches.len()));
    }

    fn search_results_source_for_command(&self) -> Option<BufferId> {
        let focused = self.workspace.focused_window().ok()?;
        if focused.kind == WindowKind::SearchResults {
            return self.search_results_source;
        }
        Some(focused.buffer_id)
    }

    fn search_results_for_source(
        &self,
        source_buffer_id: BufferId,
    ) -> Option<(SearchSpec, Vec<SearchMatch>)> {
        let buffer = self.buffer_state(source_buffer_id)?;
        if let Some(search) = &buffer.search {
            return Some((search.spec.clone(), search.matches.clone()));
        }
        let query = self.last_find_query.as_ref()?;
        let spec = SearchSpec::parse(query);
        if spec.is_empty() {
            return None;
        }
        Some((
            spec.clone(),
            buffer
                .buffer
                .find_all_with_options(&spec.query, spec.options),
        ))
    }

    fn jump_search_result(&mut self, target: &str) {
        let Some(source_buffer_id) = self
            .search_results_source_for_command()
            .or(self.search_results_source)
        else {
            self.set_status("Search Results: source buffer is missing");
            return;
        };
        let Some((spec, matches)) = self.search_results_for_source(source_buffer_id) else {
            self.set_status("Search Results: no query");
            return;
        };
        if matches.is_empty() {
            self.set_status(format!("Search Results: no matches for {}", spec.display()));
            return;
        }
        let Ok(number) = target.parse::<usize>() else {
            self.set_status("Search Results: match number expected");
            return;
        };
        let Some(index) = number.checked_sub(1).filter(|index| *index < matches.len()) else {
            self.set_status(format!("Search Results: match {number} out of range"));
            return;
        };

        if !self.focus_window_for_buffer(source_buffer_id) {
            self.set_status("Search Results: source window is missing");
            return;
        }
        let selected = matches[index].range;
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: source_buffer_id,
                body_height: 1,
                body_width: 1,
            });
        if let Some(buffer) = self.buffer_state_mut(source_buffer_id) {
            let _ = buffer.buffer.select(selected.start, selected.end);
            buffer.set_search(spec.clone(), matches.clone(), Some(index));
            buffer.ensure_cursor_visible(context.body_height, context.body_width);
        }
        self.set_status(format!(
            "Search Results: {}/{} {}",
            index + 1,
            matches.len(),
            spec.display()
        ));
    }

    fn jump_current_search_result(&mut self) {
        let Some(index) = self.current_or_next_numbered_aux_index("Search Results") else {
            return;
        };
        self.jump_search_result(&(index + 1).to_string());
    }

    fn find_in_focused_buffer(&mut self, spec: SearchSpec, direction: SearchDirection) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Find: focused buffer is missing");
            return;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&spec.query, spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(spec.clone(), matches, None);
            self.set_status(format!("Find: no matches for {}", spec.display()));
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
        let match_count = matches.len();
        buffer.set_search(spec.clone(), matches, Some(selection.index));

        let suffix = if selection.wrapped { " (wrapped)" } else { "" };
        self.set_status(format!(
            "Find: {}/{} {}{suffix}",
            selection.index + 1,
            match_count,
            spec.display()
        ));
    }

    fn current_or_next_numbered_aux_index(&mut self, label: &'static str) -> Option<usize> {
        let buffer_id = self.workspace.focused_window().ok()?.buffer_id;
        let current_line = self.buffer_state(buffer_id)?.buffer.cursor_position().line;
        if let Some(index) = self
            .buffer_state(buffer_id)
            .and_then(|buffer| buffer.buffer.line(current_line))
            .and_then(numbered_list_index_for_line)
        {
            return Some(index);
        }

        self.move_focused_numbered_aux_row(1, label)
    }

    fn move_focused_numbered_aux_row(
        &mut self,
        delta: isize,
        label: &'static str,
    ) -> Option<usize> {
        let buffer_id = self.workspace.focused_window().ok()?.buffer_id;
        let rows = self
            .buffer_state(buffer_id)
            .map(|buffer| numbered_list_rows(&buffer.buffer))
            .unwrap_or_default();
        if rows.is_empty() {
            self.set_status(format!("{label}: no entries"));
            return None;
        }

        let current_line = self.buffer_state(buffer_id)?.buffer.cursor_position().line;
        let current_row = rows.iter().position(|row| row.line == current_line);
        let next_row = if let Some(current_row) = current_row {
            wrapping_index(current_row, rows.len(), delta)
        } else {
            if delta < 0 {
                rows.iter()
                    .rposition(|row| row.line < current_line)
                    .unwrap_or(rows.len() - 1)
            } else {
                rows.iter()
                    .position(|row| row.line > current_line)
                    .unwrap_or(0)
            }
        };
        let row = rows[next_row];
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            let _ = buffer.buffer.set_cursor(Position::new(row.line, 0));
            buffer.ensure_cursor_visible(context.body_height, context.body_width);
        }
        self.set_status(format!(
            "{label}: selected {}/{}",
            row.index + 1,
            rows.len()
        ));
        Some(row.index)
    }

    fn replace_in_focused_buffer(&mut self, spec: SearchSpec, replacement: &str) {
        if spec.is_empty() {
            self.set_status("Replace: no query");
            return;
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Replace: focused buffer is missing");
            return;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&spec.query, spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(spec.clone(), matches, None);
            self.set_status(format!("Replace: no matches for {}", spec.display()));
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
        let old_total = matches.len();

        match buffer.buffer.replace_range(target, replacement) {
            Ok(()) => {
                let suffix = if selection.wrapped { " (wrapped)" } else { "" };
                let new_matches = buffer
                    .buffer
                    .find_all_with_options(&spec.query, spec.options);
                let next_selection = if new_matches.is_empty() {
                    None
                } else {
                    Some(choose_search_match(
                        &new_matches,
                        buffer.buffer.cursor_position(),
                        SearchDirection::Forward,
                    ))
                };
                if let Some(next) = next_selection {
                    let selected = new_matches[next.index].range;
                    let _ = buffer.buffer.select(selected.start, selected.end);
                }
                let next_status = match next_selection {
                    Some(next) => format!("; next {}/{}", next.index + 1, new_matches.len()),
                    None => "; no matches left".to_string(),
                };
                buffer.set_search(
                    spec.clone(),
                    new_matches,
                    next_selection.map(|selection| selection.index),
                );
                self.set_status(format!(
                    "Replace: {}/{} {} -> {}{suffix}{next_status}",
                    selection.index + 1,
                    old_total,
                    spec.display(),
                    replacement_status_text(replacement)
                ));
            }
            Err(error) => self.set_status(format!("Replace failed: {}", buffer_error_text(error))),
        }
    }

    fn replace_all_in_focused_buffer(&mut self, spec: SearchSpec, replacement: &str) {
        if spec.is_empty() {
            self.set_status("Replace All: no query");
            return;
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Replace All: focused buffer is missing");
            return;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&spec.query, spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(spec.clone(), matches, None);
            self.set_status(format!("Replace All: no matches for {}", spec.display()));
            return;
        }

        match buffer
            .buffer
            .replace_all_with_options(&spec.query, replacement, spec.options)
        {
            Ok(count) => {
                let new_matches = buffer
                    .buffer
                    .find_all_with_options(&spec.query, spec.options);
                let remaining = new_matches.len();
                buffer.set_search(spec.clone(), new_matches, None);
                let suffix = if remaining == 0 {
                    String::new()
                } else {
                    format!("; {remaining} matches remain")
                };
                self.set_status(format!(
                    "Replace All: {count} {} -> {}{suffix}",
                    spec.display(),
                    replacement_status_text(replacement)
                ));
            }
            Err(error) => {
                self.set_status(format!("Replace All failed: {}", buffer_error_text(error)))
            }
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

    #[cfg(test)]
    fn prompt_status_text(&self) -> Option<String> {
        self.prompt
            .as_ref()
            .map(PromptState::status_text)
            .or_else(|| self.file_dialog.as_ref().map(FileDialogState::status_text))
    }

    #[cfg(test)]
    fn confirm_status_text(&self) -> Option<String> {
        if let Some(confirm) = &self.confirm {
            let action = match confirm.action {
                PendingAction::Quit => "Save(s) Quit without saving(d) Cancel(c)",
                PendingAction::New
                | PendingAction::OpenPrompt
                | PendingAction::ReloadBuffer
                | PendingAction::CloseWindow => "Save(s) Discard(d) Cancel(c)",
            };
            return Some(format!(
                "Unsaved changes in {}: {action}",
                self.buffer_display_name(confirm.buffer_id)
            ));
        }

        self.replace_confirm
            .as_ref()
            .map(|confirm| self.replace_confirm_status_text(confirm))
    }

    fn active_overlay(&self) -> Option<UiOverlay> {
        if let Some(confirm) = &self.confirm {
            let action = match confirm.action {
                PendingAction::Quit => "[Save(s)] [Discard(d)] [Cancel(c)]",
                PendingAction::New
                | PendingAction::OpenPrompt
                | PendingAction::ReloadBuffer
                | PendingAction::CloseWindow => "[Save(s)] [Discard(d)] [Cancel(c)]",
            };
            return Some(UiOverlay::message(
                "Unsaved Changes",
                vec![format!(
                    "Unsaved changes in {}",
                    self.buffer_display_name(confirm.buffer_id)
                )],
                vec![action.to_string()],
            ));
        }

        if let Some(confirm) = &self.replace_confirm {
            return Some(self.replace_confirm_overlay(confirm));
        }

        if let Some(switcher) = &self.buffer_switcher {
            return Some(self.buffer_switcher_overlay(switcher));
        }

        if let Some(dialog) = &self.file_dialog {
            return Some(dialog.overlay(&self.file_dialog_keys));
        }

        let prompt = self.prompt.as_ref()?;
        let mut overlay = UiOverlay::prompt(
            prompt.kind.name(),
            prompt.input.as_str().to_string(),
            prompt.input.cursor_display_column(),
        );
        if let Some(completion) = &prompt.completion {
            overlay.lines.push(completion.status_text());
        }
        Some(overlay)
    }

    fn replace_confirm_overlay(&self, confirm: &ReplaceConfirmState) -> UiOverlay {
        UiOverlay::message(
            "Confirm Replace",
            vec![
                format!("Find: {}", confirm.spec.display()),
                format!(
                    "Replace with: {}",
                    replacement_status_text(&confirm.replacement)
                ),
                self.replace_confirm_status_text(confirm),
            ],
            vec!["[Replace(r)] [Skip(s)] [All(a)] [Cancel(c)]".to_string()],
        )
    }

    fn replace_confirm_status_text(&self, confirm: &ReplaceConfirmState) -> String {
        let match_status = self
            .buffer_state(confirm.buffer_id)
            .and_then(|buffer| buffer.search.as_ref())
            .filter(|search| search.spec == confirm.spec)
            .and_then(|search| match (search.matches.len(), search.active_index) {
                (0, _) => None,
                (total, Some(index)) => Some(format!("Match {}/{}", index + 1, total)),
                (total, None) => Some(format!("Match {total}")),
            })
            .unwrap_or_else(|| "Match -".to_string());

        format!(
            "{match_status}; replaced {}, skipped {}",
            confirm.replaced, confirm.skipped
        )
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
            return "[No Window]".to_string();
        };

        let Some(buffer) = self.buffer_state(window.buffer_id) else {
            return format!("[{}]", window.title);
        };

        let mode = if buffer.encoding == FileTextEncoding::EscapedBytes {
            "Escaped Bytes"
        } else if buffer.buffer.is_read_only() {
            "Read Only"
        } else {
            "Plain Text"
        };
        let dirty = if buffer.buffer.is_dirty() { "*" } else { "" };

        format!("[{mode}{dirty}]")
    }

    fn focused_detail_status(&self) -> String {
        let profile = terminal_profile_status(self.shell.profile);
        let window = self.focused_window_status();
        let Some(buffer_id) = self.focused_buffer_id() else {
            return format!("[Ln -] [{profile}] [{window}]");
        };
        let Some(buffer) = self.buffer_state(buffer_id) else {
            return format!("[Ln -] [{profile}] [{window}]");
        };

        let position = buffer.buffer.cursor_position();
        let column = buffer
            .buffer
            .line(position.line)
            .and_then(|line| line.get(..position.column))
            .map(|prefix| UnicodeWidthStr::width(prefix) + 1)
            .unwrap_or(1);

        let mut parts = vec![
            bracket(line_ending_status(buffer.buffer.line_ending())),
            bracket(file_encoding_status(buffer.encoding)),
            bracket("Spaces:4"),
            format!("{}:{}", position.line + 1, column),
            bracket(&scroll_status(
                buffer,
                self.focused_buffer_view_context(self.workspace_area),
            )),
            bracket(&profile),
            bracket(&window),
        ];
        if let Some(state) = buffer_disk_state(buffer) {
            parts.insert(4, bracket(state));
        }
        if buffer.word_wrap {
            parts.insert(4, bracket("Wrap"));
        }
        if buffer.visible_whitespace {
            parts.insert(4, bracket("Whitespace"));
        }
        if buffer.bookmarks.contains(&position.line) {
            parts.insert(4, bracket("Mark"));
        }
        if let Some(selection) = selection_status(&buffer.buffer) {
            parts.insert(4, bracket(&selection));
        }
        if let Some(search) = buffer.search_status() {
            parts.insert(4, bracket(&search));
        }

        parts.join(" ")
    }

    fn focused_file_status(&self) -> String {
        let Some(buffer_id) = self.focused_buffer_id() else {
            return "[No file]".to_string();
        };

        let name = self.buffer_display_name(buffer_id);
        bracket(&name)
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

    fn focused_buffer_view_context(&self, area: Rect) -> Option<BufferViewContext> {
        let window = self.workspace.focused_window().ok()?;
        self.buffer_view_context(window.buffer_id, area)
    }

    fn buffer_view_context(&self, buffer_id: BufferId, area: Rect) -> Option<BufferViewContext> {
        let window = self
            .workspace
            .windows
            .iter()
            .find(|window| window.buffer_id == buffer_id)?;
        let layout = self
            .workspace
            .resolved_layout(area)
            .into_iter()
            .find(|layout| layout.id == window.id)?;
        let buffer = self.buffer_state(buffer_id)?;
        let body_height = layout.rect.height.saturating_sub(2) as usize;
        let body_width = editor_body_width(buffer, layout.rect);
        Some(BufferViewContext {
            buffer_id,
            body_height,
            body_width,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BufferViewContext {
    buffer_id: BufferId,
    body_height: usize,
    body_width: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SearchSpec {
    input: String,
    query: String,
    options: SearchOptions,
}

impl SearchSpec {
    fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        let mut options = SearchOptions::default();

        if let Some(rest) = trimmed.strip_prefix('/') {
            let (flags, query) = rest
                .find(char::is_whitespace)
                .map(|index| (&rest[..index], rest[index..].trim_start()))
                .unwrap_or((rest, ""));
            if !flags.is_empty()
                && !query.is_empty()
                && flags
                    .chars()
                    .all(|ch| matches!(ch, 'i' | 'I' | 'c' | 'C' | 'w' | 'W'))
            {
                for flag in flags.chars() {
                    match flag.to_ascii_lowercase() {
                        'i' => options.case_sensitive = false,
                        'c' => options.case_sensitive = true,
                        'w' => options.whole_word = true,
                        _ => {}
                    }
                }
                return Self {
                    input: trimmed.to_string(),
                    query: query.to_string(),
                    options,
                };
            }
        }

        Self {
            input: trimmed.to_string(),
            query: trimmed.to_string(),
            options,
        }
    }

    fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    fn display(&self) -> String {
        let mut flags = Vec::new();
        if !self.options.case_sensitive {
            flags.push("ignore-case");
        }
        if self.options.whole_word {
            flags.push("whole-word");
        }
        if flags.is_empty() {
            self.query.clone()
        } else {
            format!("{} ({})", self.query, flags.join(", "))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PromptState {
    kind: PromptKind,
    input: LineInput,
    preview: Option<PromptPreviewState>,
    history_index: Option<usize>,
    history_draft: String,
    completion: Option<PromptCompletionState>,
}

impl PromptState {
    fn new(kind: PromptKind, input: String, preview: Option<PromptPreviewState>) -> Self {
        Self {
            kind,
            input: LineInput::new(input),
            preview,
            history_index: None,
            history_draft: String::new(),
            completion: None,
        }
    }

    #[cfg(test)]
    fn status_text(&self) -> String {
        format!("{}{}", self.kind.label(), self.input.as_str())
    }

    fn detach_history(&mut self) {
        if self.kind == PromptKind::CommandLine {
            self.history_index = None;
            self.history_draft.clear();
        }
    }

    fn clear_completion(&mut self) {
        self.completion = None;
    }

    fn next_completion_replacement(&mut self, input: &str, forward: bool) -> Option<String> {
        let completion = self.completion.as_mut()?;
        let next_index = if completion.active_index.is_none() && input == completion.base_input {
            if forward {
                0
            } else {
                completion.candidates.len().saturating_sub(1)
            }
        } else {
            let current_index = completion
                .active_index
                .or_else(|| {
                    completion
                        .candidates
                        .iter()
                        .position(|candidate| candidate.replacement == input)
                })
                .filter(|_| {
                    completion
                        .candidates
                        .iter()
                        .any(|candidate| candidate.replacement == input)
                })?;
            wrapping_index(
                current_index,
                completion.candidates.len(),
                if forward { 1 } else { -1 },
            )
        };
        completion.active_index = Some(next_index);
        Some(completion.candidates[next_index].replacement.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PromptCompletionState {
    base_input: String,
    candidates: Vec<CommandCompletionCandidate>,
    active_index: Option<usize>,
}

impl PromptCompletionState {
    fn new(base_input: String, candidates: Vec<CommandCompletionCandidate>) -> Self {
        Self {
            base_input,
            candidates,
            active_index: None,
        }
    }

    fn status_text(&self) -> String {
        let list = self
            .candidates
            .iter()
            .map(|candidate| candidate.display.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if let Some(index) = self.active_index {
            format!(
                "Command completion: {}/{} {}",
                index + 1,
                self.candidates.len(),
                list
            )
        } else {
            format!("Command completion: {list}")
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PromptPreviewState {
    buffer_id: BufferId,
    cursor: Position,
    selection: Option<Selection>,
    search: Option<BufferSearchState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromptKind {
    CommandLine,
    Find,
    ReplaceFind,
    ReplaceWith,
    GoToLine,
    RunCommand,
}

impl PromptKind {
    const fn history_kind(self) -> Option<PromptHistoryKind> {
        match self {
            Self::CommandLine => Some(PromptHistoryKind::CommandLine),
            Self::RunCommand => Some(PromptHistoryKind::RunCommand),
            Self::Find | Self::ReplaceFind | Self::ReplaceWith | Self::GoToLine => None,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::CommandLine => "Command: ",
            Self::Find => "Find: ",
            Self::ReplaceFind => "Replace Find: ",
            Self::ReplaceWith => "Replace With: ",
            Self::GoToLine => "Go To Line: ",
            Self::RunCommand => "Run Command: ",
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::CommandLine => "Command",
            Self::Find => "Find",
            Self::ReplaceFind | Self::ReplaceWith => "Replace",
            Self::GoToLine => "Go To Line",
            Self::RunCommand => "Run Command",
        }
    }

    const fn is_replace(self) -> bool {
        matches!(self, Self::ReplaceFind | Self::ReplaceWith)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromptHistoryKind {
    CommandLine,
    RunCommand,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileDialogState {
    kind: FileDialogKind,
    input: LineInput,
    entries: Vec<FileDialogEntry>,
    selected_index: Option<usize>,
    scroll_offset: usize,
    show_hidden: bool,
    selection_touched: bool,
    message: Option<String>,
    after_success: Option<PendingAction>,
    overwrite_path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BufferSwitcherState {
    selected_index: usize,
    scroll_offset: usize,
}

impl BufferSwitcherState {
    fn new(selected_index: usize, total: usize) -> Self {
        let mut state = Self {
            selected_index: selected_index.min(total.saturating_sub(1)),
            scroll_offset: 0,
        };
        state.ensure_selected_visible(total);
        state
    }

    fn selected_index(&self, total: usize) -> Option<usize> {
        (total > 0).then_some(self.selected_index.min(total - 1))
    }

    fn move_selection(&mut self, delta: isize, total: usize) {
        if total == 0 {
            self.selected_index = 0;
            self.scroll_offset = 0;
            return;
        }

        self.selected_index = if delta < 0 {
            self.selected_index.saturating_sub(delta.unsigned_abs())
        } else {
            self.selected_index
                .saturating_add(delta as usize)
                .min(total - 1)
        };
        self.ensure_selected_visible(total);
    }

    fn page_selection(&mut self, delta: isize, total: usize) {
        let step = BUFFER_SWITCHER_VISIBLE_ENTRIES.saturating_sub(1).max(1) as isize;
        self.move_selection(delta.saturating_mul(step), total);
    }

    fn select_visible_index(&mut self, visible_index: usize, total: usize) -> Option<usize> {
        let index = self.scroll_offset.saturating_add(visible_index);
        if index < total {
            self.selected_index = index;
            self.ensure_selected_visible(total);
            Some(index)
        } else {
            None
        }
    }

    fn visible_entry_texts(&self, entries: &[BufferSwitcherEntry]) -> (Vec<String>, Option<usize>) {
        let Some((start, end, selected)) = self.visible_entry_range(entries.len()) else {
            return (vec!["(no buffers)".to_string()], None);
        };
        let list = entries[start..end]
            .iter()
            .map(|entry| entry.text.clone())
            .collect::<Vec<_>>();
        let selected = if (start..end).contains(&selected) {
            Some(selected - start)
        } else {
            None
        };
        (list, selected)
    }

    fn visible_entry_range(&self, total: usize) -> Option<(usize, usize, usize)> {
        if total == 0 {
            return None;
        }

        let selected = self.selected_index.min(total - 1);
        let start = self.scroll_offset.min(total - 1);
        let end = start
            .saturating_add(BUFFER_SWITCHER_VISIBLE_ENTRIES)
            .min(total);
        Some((start, end, selected))
    }

    fn ensure_selected_visible(&mut self, total: usize) {
        if total == 0 {
            self.selected_index = 0;
            self.scroll_offset = 0;
            return;
        }

        self.selected_index = self.selected_index.min(total - 1);
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index
            >= self
                .scroll_offset
                .saturating_add(BUFFER_SWITCHER_VISIBLE_ENTRIES)
        {
            self.scroll_offset = self
                .selected_index
                .saturating_sub(BUFFER_SWITCHER_VISIBLE_ENTRIES.saturating_sub(1));
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BufferSwitcherEntry {
    buffer_id: BufferId,
    text: String,
}

impl FileDialogState {
    fn new(kind: FileDialogKind, input: String, after_success: Option<PendingAction>) -> Self {
        let mut state = Self {
            kind,
            input: LineInput::new(input),
            entries: Vec::new(),
            selected_index: None,
            scroll_offset: 0,
            show_hidden: false,
            selection_touched: false,
            message: None,
            after_success,
            overwrite_path: None,
        };
        state.refresh_entries();
        state
    }

    #[cfg(test)]
    fn status_text(&self) -> String {
        let label = match self.kind {
            FileDialogKind::Open => "Open: ",
            FileDialogKind::SaveAs => "Save As: ",
            FileDialogKind::CommandOutputSave => "Save Output: ",
        };
        format!("{label}{}", self.input.as_str())
    }

    fn overlay(&self, keymap: &FileDialogKeymap) -> UiOverlay {
        let context = file_dialog_context(self.input.as_str());
        let hidden_state = if self.show_hidden {
            "shown"
        } else if context.prefix.starts_with('.') {
            "shown by prefix"
        } else {
            "hidden"
        };
        let hidden_key = file_dialog_action_key_text(keymap, FileDialogAction::ToggleHidden);
        let entry_count = self.entries.iter().filter(|entry| !entry.is_parent).count();
        let mut lines = vec![
            format!("Look in: {}", context.directory.display()),
            format!("{}:", self.kind.input_label()),
            self.message
                .clone()
                .unwrap_or_else(|| self.kind.help_text(entry_count)),
            format!("Hidden: {hidden_state} ({hidden_key})"),
        ];
        if self.entries.len() > FILE_DIALOG_VISIBLE_ENTRIES {
            if let Some((start, end, _)) = self.visible_entry_range() {
                lines.push(format!(
                    "Showing {}-{} of {} matches",
                    start + 1,
                    end,
                    self.entries.len()
                ));
            }
        }

        let (list, selected) = self.visible_entry_texts();
        let mut overlay = UiOverlay::file_dialog(
            self.kind.name(),
            lines,
            self.input.as_str().to_string(),
            self.input.cursor_display_column(),
            list,
            selected,
            vec![file_dialog_shortcuts_text(keymap)],
        );
        if let Some((start, end, _)) = self.visible_entry_range() {
            overlay = overlay.with_list_overflow(start > 0, end < self.entries.len());
        }
        overlay
    }

    fn refresh_entries(&mut self) {
        let context = file_dialog_context(self.input.as_str());
        match list_file_dialog_entries(&context, self.show_hidden) {
            Ok(listing) => {
                self.message = file_dialog_list_message(
                    &context,
                    &listing.entries,
                    self.show_hidden,
                    listing.hidden_filtered,
                );
                self.entries = listing.entries;
                self.selected_index = if self.entries.is_empty() {
                    None
                } else {
                    Some(0)
                };
                self.scroll_offset = 0;
                self.selection_touched = false;
            }
            Err(error) => {
                self.entries.clear();
                self.selected_index = None;
                self.scroll_offset = 0;
                self.selection_touched = false;
                self.message = Some(format!(
                    "Cannot list {}: {}",
                    context.directory.display(),
                    path_error_detail(&error)
                ));
            }
        }
    }

    fn visible_entry_texts(&self) -> (Vec<String>, Option<usize>) {
        let Some((start, end, selected)) = self.visible_entry_range() else {
            return (vec!["(no matches)".to_string()], None);
        };
        let list = self.entries[start..end]
            .iter()
            .map(FileDialogEntry::display_text)
            .collect::<Vec<_>>();
        let selected = if (start..end).contains(&selected) {
            Some(selected - start)
        } else {
            None
        };
        (list, selected)
    }

    fn visible_entry_range(&self) -> Option<(usize, usize, usize)> {
        if self.entries.is_empty() {
            return None;
        }

        let selected = self
            .selected_index
            .filter(|index| *index < self.entries.len())
            .unwrap_or_else(|| self.scroll_offset.min(self.entries.len().saturating_sub(1)));
        let start = self.scroll_offset.min(self.max_scroll_offset());
        let end = start
            .saturating_add(FILE_DIALOG_VISIBLE_ENTRIES)
            .min(self.entries.len());
        Some((start, end, selected))
    }

    fn entry_index_for_visible_index(&self, visible_index: usize) -> Option<usize> {
        let (start, end, _) = self.visible_entry_range()?;
        let index = start.saturating_add(visible_index);
        (index < end).then_some(index)
    }

    fn max_scroll_offset(&self) -> usize {
        self.entries
            .len()
            .saturating_sub(FILE_DIALOG_VISIBLE_ENTRIES)
    }

    fn ensure_selection_visible(&mut self) {
        let Some(selected) = self.selected_index else {
            self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
            return;
        };

        if selected < self.scroll_offset {
            self.scroll_offset = selected;
        } else {
            let visible_end = self
                .scroll_offset
                .saturating_add(FILE_DIALOG_VISIBLE_ENTRIES);
            if selected >= visible_end {
                self.scroll_offset = selected
                    .saturating_add(1)
                    .saturating_sub(FILE_DIALOG_VISIBLE_ENTRIES);
            }
        }
        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
    }

    fn clamp_selection_to_visible_range(&mut self) {
        if self.entries.is_empty() {
            self.selected_index = None;
            self.scroll_offset = 0;
            return;
        }

        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
        let start = self.scroll_offset;
        let end = start
            .saturating_add(FILE_DIALOG_VISIBLE_ENTRIES)
            .min(self.entries.len());
        let selected = self.selected_index.unwrap_or(start);
        self.selected_index = Some(selected.clamp(start, end.saturating_sub(1)));
    }

    fn move_selection(&mut self, delta: isize) {
        if self.entries.is_empty() {
            self.selected_index = None;
            self.message = Some("No matches".to_string());
            return;
        }

        let current = self.selected_index.unwrap_or(0);
        self.selected_index = Some(wrapping_index(current, self.entries.len(), delta));
        self.ensure_selection_visible();
        self.selection_touched = true;
        self.message = None;
    }

    fn page_selection(&mut self, delta: isize) {
        if self.entries.is_empty() {
            self.selected_index = None;
            self.scroll_offset = 0;
            self.message = Some("No matches".to_string());
            return;
        }

        let current = self.selected_index.unwrap_or(0);
        let page = FILE_DIALOG_VISIBLE_ENTRIES.saturating_sub(1).max(1) as isize;
        let next = current
            .saturating_add_signed(delta.saturating_mul(page))
            .min(self.entries.len().saturating_sub(1));
        self.selected_index = Some(next);
        self.ensure_selection_visible();
        self.selection_touched = true;
        self.message = None;
    }

    fn scroll(&mut self, delta: isize) {
        if self.entries.is_empty() {
            self.selected_index = None;
            self.scroll_offset = 0;
            return;
        }

        self.scroll_offset = self
            .scroll_offset
            .saturating_add_signed(delta)
            .min(self.max_scroll_offset());
        self.clamp_selection_to_visible_range();
        self.selection_touched = true;
        self.message = None;
    }

    fn move_input_left(&mut self) {
        self.input.move_left();
        self.message = None;
    }

    fn move_input_right(&mut self) {
        self.input.move_right();
        self.message = None;
    }

    fn move_input_start(&mut self) {
        self.input.move_start();
        self.message = None;
    }

    fn move_input_end(&mut self) {
        self.input.move_end();
        self.message = None;
    }

    fn insert_char(&mut self, ch: char) {
        self.overwrite_path = None;
        self.input.insert_char(ch);
        self.refresh_entries();
    }

    fn insert_text(&mut self, text: &str) {
        self.overwrite_path = None;
        self.input.insert_str(text);
        self.refresh_entries();
    }

    fn delete_backward(&mut self) {
        self.overwrite_path = None;
        self.input.delete_backward();
        self.refresh_entries();
    }

    fn delete_forward(&mut self) {
        self.overwrite_path = None;
        self.input.delete_forward();
        self.refresh_entries();
    }

    fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.refresh_entries();
        self.message = Some(format!(
            "Hidden files {}",
            if self.show_hidden { "shown" } else { "hidden" }
        ));
    }

    fn complete(&mut self, forward: bool) {
        if self.entries.is_empty() {
            self.message = Some("No matches".to_string());
            return;
        }

        let context = file_dialog_context(self.input.as_str());
        let completion_indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| is_completable_file_dialog_entry(entry, &context.prefix))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        if completion_indices.is_empty() {
            self.message = Some("No matches".to_string());
            return;
        }

        if completion_indices.len() == 1 {
            self.apply_entry(completion_indices[0]);
            return;
        }

        if let Some(prefix) = common_entry_prefix(&self.entries, &context.prefix) {
            if prefix.len() > context.prefix.len() {
                self.input
                    .set_text(format!("{}{}", context.base_input, prefix));
                self.refresh_entries();
                return;
            }
        }

        let selected = self
            .selected_index
            .filter(|index| completion_indices.contains(index))
            .unwrap_or_else(|| {
                if forward {
                    completion_indices[0]
                } else {
                    *completion_indices
                        .last()
                        .expect("completion indices are non-empty")
                }
            });
        self.apply_entry(selected);
    }

    fn submit(&mut self) -> FileDialogSubmit {
        let input = self.input.as_str().trim().to_string();
        if input.is_empty() {
            return FileDialogSubmit::Cancel;
        }

        if self.kind == FileDialogKind::Open {
            if let Some(index) = self
                .selected_index
                .filter(|index| *index < self.entries.len())
            {
                let entry = self.entries[index].clone();
                let context = file_dialog_context(self.input.as_str());
                let should_use_selected = self.selection_touched
                    || entry.name == context.prefix
                    || entry.is_parent && context.prefix == "..";
                if should_use_selected {
                    if entry.is_dir {
                        self.apply_entry(index);
                        return FileDialogSubmit::ContinueEditing;
                    }
                    return FileDialogSubmit::Path(entry.path);
                }
            }

            let path = expand_user_path(&input);
            if path.is_dir() {
                self.input
                    .set_text(ensure_trailing_separator(input.to_string()));
                self.refresh_entries();
                return FileDialogSubmit::ContinueEditing;
            }
            return FileDialogSubmit::Path(path);
        }

        let path = expand_user_path(&input);
        if self.kind.confirms_overwrite()
            && path.exists()
            && !path.is_dir()
            && self.overwrite_path.as_ref() != Some(&path)
        {
            self.overwrite_path = Some(path.clone());
            self.message = Some(format!(
                "Replace existing file {}? Press Enter again.",
                path.display()
            ));
            return FileDialogSubmit::ContinueEditing;
        }

        FileDialogSubmit::Path(path)
    }

    fn click_visible_entry(&mut self, visible_index: usize) -> FileDialogSubmit {
        let Some(index) = self.entry_index_for_visible_index(visible_index) else {
            return FileDialogSubmit::ContinueEditing;
        };
        let Some(entry) = self.entries.get(index).cloned() else {
            return FileDialogSubmit::ContinueEditing;
        };

        self.selected_index = Some(index);
        self.selection_touched = true;
        match self.kind {
            FileDialogKind::Open => {
                if entry.is_dir {
                    self.apply_entry(index);
                    FileDialogSubmit::ContinueEditing
                } else {
                    FileDialogSubmit::Path(entry.path)
                }
            }
            FileDialogKind::SaveAs | FileDialogKind::CommandOutputSave => {
                self.apply_entry(index);
                FileDialogSubmit::ContinueEditing
            }
        }
    }

    fn apply_entry(&mut self, index: usize) {
        let Some(entry) = self.entries.get(index) else {
            return;
        };
        self.overwrite_path = None;
        self.input.set_text(entry.input.clone());
        self.refresh_entries();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileDialogKind {
    Open,
    SaveAs,
    CommandOutputSave,
}

impl FileDialogKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::SaveAs => "Save As",
            Self::CommandOutputSave => "Save Command Output",
        }
    }

    const fn input_label(self) -> &'static str {
        match self {
            Self::Open => "File name",
            Self::SaveAs => "Save as",
            Self::CommandOutputSave => "Save output as",
        }
    }

    const fn confirms_overwrite(self) -> bool {
        matches!(self, Self::SaveAs | Self::CommandOutputSave)
    }

    fn help_text(self, entry_count: usize) -> String {
        let noun = if entry_count == 1 { "entry" } else { "entries" };
        match self {
            Self::Open => format!("Select a file or type a path. {entry_count} {noun}."),
            Self::SaveAs => format!("Type the destination path. {entry_count} {noun}."),
            Self::CommandOutputSave => {
                format!("Type the output destination path. {entry_count} {noun}.")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileDialogEntry {
    name: String,
    input: String,
    path: PathBuf,
    is_dir: bool,
    is_parent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileDialogListing {
    entries: Vec<FileDialogEntry>,
    hidden_filtered: usize,
}

impl FileDialogEntry {
    fn display_text(&self) -> String {
        if self.is_parent {
            "[..] Parent directory".to_string()
        } else if self.is_dir {
            format!("[DIR]  {}/", self.name)
        } else {
            format!("       {}", self.name)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FileDialogSubmit {
    Cancel,
    ContinueEditing,
    Path(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileDialogContext {
    base_input: String,
    prefix: String,
    directory: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ConfirmState {
    action: PendingAction,
    buffer_id: BufferId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReplaceConfirmState {
    buffer_id: BufferId,
    spec: SearchSpec,
    replacement: String,
    replaced: usize,
    skipped: usize,
    skipped_in_cycle: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingAction {
    Quit,
    New,
    OpenPrompt,
    ReloadBuffer,
    CloseWindow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeAction {
    ShellEscape,
    WriteTerminal(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CopyTextError {
    MissingBuffer,
    NoSelection,
    Buffer(BufferError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MouseDragState {
    Selection {
        buffer_id: BufferId,
        anchor: Position,
    },
    Split {
        handle: SplitDragHandle,
    },
    Scrollbar {
        buffer_id: BufferId,
    },
}

const COMMAND_LINE_HELP: &str = "Commands: help, outline [section], results [N], config [section], status, reload-config, shell, run [\"command\"], output index|summary|status|stdout|stdout-body|stderr|stderr-body|truncated|only stdout|stderr|find QUERY|next|previous|next-section|previous-section|clear|copy|save PATH, theme [name], open [path], save [path], save-as [path], find [query], replace QUERY TEXT, replace all QUERY TEXT, goto LINE, or any command id such as edit.scroll_right";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandLineParseError {
    TrailingEscape,
    UnclosedQuote,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CommandCompletion {
    None,
    Unique(String),
    CommonPrefix(String, usize),
    Candidates(Vec<CommandCompletionCandidate>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandCompletionCandidate {
    display: String,
    replacement: String,
}

fn command_line_completion(input: &str) -> CommandCompletion {
    let trailing_space = input.chars().last().is_some_and(char::is_whitespace);
    let tokens = match parse_command_line(input) {
        Ok(tokens) => tokens,
        Err(_) => return CommandCompletion::None,
    };
    let mut tokens = tokens;
    if trailing_space {
        tokens.push(String::new());
    }

    match tokens.as_slice() {
        [] => complete_last_token(input, "", command_line_top_level_candidates(), true),
        [partial] => complete_last_token("", partial, command_line_top_level_candidates(), true),
        [command, partial] if command_accepts_path_argument(command) => {
            complete_path_token(&format!("{command} "), partial)
        }
        [command, partial] => {
            let candidates = match normalize_command_line_token(command).as_str() {
                "config" | "diagnostics" | "configdiagnostics" => {
                    config_diagnostics_section_candidates()
                }
                "output" | "commandoutput" => command_output_action_candidates(),
                "theme" => theme_command_candidates(),
                _ => return CommandCompletion::None,
            };
            let prefix = format!("{command} ");
            complete_last_token(&prefix, partial, candidates, false)
        }
        [command, subcommand, partial]
            if normalize_command_line_token(command) == "output"
                && normalize_command_line_token(subcommand) == "save" =>
        {
            complete_path_token(&format!("{command} {subcommand} "), partial)
        }
        [command, subcommand, partial]
            if normalize_command_line_token(command) == "output"
                && normalize_command_line_token(subcommand) == "only" =>
        {
            let prefix = format!("{command} {subcommand} ");
            complete_last_token(&prefix, partial, command_output_section_candidates(), false)
        }
        _ => CommandCompletion::None,
    }
}

fn complete_last_token(
    prefix: &str,
    partial: &str,
    candidates: &[&'static str],
    add_space_after_unique: bool,
) -> CommandCompletion {
    let normalized_partial = normalize_command_line_token(partial);
    let matches = candidates
        .iter()
        .copied()
        .filter(|candidate| {
            normalize_command_line_token(candidate).starts_with(&normalized_partial)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => CommandCompletion::None,
        [candidate] => {
            let mut text = format!("{prefix}{candidate}");
            if add_space_after_unique {
                text.push(' ');
            }
            CommandCompletion::Unique(text)
        }
        _ => {
            let common = common_candidate_prefix(&matches);
            if common.len() > partial.len() {
                CommandCompletion::CommonPrefix(format!("{prefix}{common}"), matches.len())
            } else {
                CommandCompletion::Candidates(
                    matches
                        .iter()
                        .map(|candidate| {
                            let mut replacement = format!("{prefix}{candidate}");
                            if add_space_after_unique {
                                replacement.push(' ');
                            }
                            CommandCompletionCandidate {
                                display: candidate.to_string(),
                                replacement,
                            }
                        })
                        .collect(),
                )
            }
        }
    }
}

fn command_accepts_path_argument(command: &str) -> bool {
    matches!(
        normalize_command_line_token(command).as_str(),
        "open" | "save" | "saveas" | "reloadfile"
    )
}

fn complete_path_token(prefix: &str, partial: &str) -> CommandCompletion {
    let context = file_dialog_context(partial);
    let Ok(listing) = list_file_dialog_entries(&context, false) else {
        return CommandCompletion::None;
    };
    let entries = listing
        .entries
        .into_iter()
        .filter(|entry| is_completable_file_dialog_entry(entry, &context.prefix))
        .collect::<Vec<_>>();
    match entries.as_slice() {
        [] => CommandCompletion::None,
        [entry] => CommandCompletion::Unique(format!(
            "{prefix}{}",
            quote_command_line_token(&entry.input)
        )),
        _ => {
            if let Some(common) = common_entry_prefix(&entries, &context.prefix) {
                if common.len() > context.prefix.len() {
                    return CommandCompletion::CommonPrefix(
                        format!(
                            "{prefix}{}",
                            quote_command_line_token(&format!("{}{}", context.base_input, common))
                        ),
                        entries.len(),
                    );
                }
            }

            CommandCompletion::Candidates(
                entries
                    .iter()
                    .map(|entry| CommandCompletionCandidate {
                        display: if entry.is_dir {
                            format!("{}/", entry.name)
                        } else {
                            entry.name.clone()
                        },
                        replacement: format!("{prefix}{}", quote_command_line_token(&entry.input)),
                    })
                    .collect(),
            )
        }
    }
}

fn quote_command_line_token(token: &str) -> String {
    if token.is_empty() {
        return "\"\"".to_string();
    }
    if !token
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\\'))
    {
        return token.to_string();
    }

    let mut quoted = String::from("\"");
    for ch in token.chars() {
        if matches!(ch, '"' | '\\') {
            quoted.push('\\');
        }
        quoted.push(ch);
    }
    quoted.push('"');
    quoted
}

fn common_candidate_prefix(candidates: &[&str]) -> String {
    let Some(first) = candidates.first() else {
        return String::new();
    };
    let mut prefix = (*first).to_string();
    for candidate in &candidates[1..] {
        while !candidate.starts_with(&prefix) {
            let Some((last, _)) = prefix.char_indices().last() else {
                return String::new();
            };
            prefix.truncate(last);
        }
    }
    prefix
}

const fn command_line_top_level_candidates() -> &'static [&'static str] {
    &[
        "buffers",
        "close",
        "commands",
        "config",
        "diagnostics",
        "find",
        "goto",
        "help",
        "mark",
        "matches",
        "new",
        "open",
        "outline",
        "output",
        "quit",
        "reload-config",
        "reloadfile",
        "replace",
        "results",
        "save",
        "save-as",
        "shell",
        "status",
        "theme",
        "whitespace",
        "wrap",
    ]
}

const fn command_output_action_candidates() -> &'static [&'static str] {
    &[
        "clear",
        "copy",
        "find",
        "index",
        "next",
        "next-section",
        "only",
        "previous",
        "previous-section",
        "save",
        "status",
        "stderr",
        "stderr-body",
        "stdout",
        "stdout-body",
        "summary",
        "truncated",
    ]
}

const fn command_output_section_candidates() -> &'static [&'static str] {
    &["stderr", "stdout"]
}

const fn config_diagnostics_section_candidates() -> &'static [&'static str] {
    &[
        "clipboard",
        "file-dialog-keymap",
        "input",
        "keymap",
        "limits",
        "paths",
        "source",
        "summary",
        "terminal",
    ]
}

const fn theme_command_candidates() -> &'static [&'static str] {
    &["dark", "dun", "msedit", "turbo"]
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

const fn config_diagnostics_section_values() -> &'static str {
    "summary|paths|source|terminal|input|clipboard|limits|keymap|file-dialog-keymap"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConfigDiagnosticsSection {
    Summary,
    Paths,
    Source,
    Terminal,
    Input,
    Clipboard,
    Limits,
    Keymap,
    FileDialogKeymap,
}

impl ConfigDiagnosticsSection {
    const fn heading(self) -> &'static str {
        match self {
            Self::Summary => "Summary",
            Self::Paths => "Paths",
            Self::Source => "Source",
            Self::Terminal => "Terminal",
            Self::Input => "Input",
            Self::Clipboard => "Clipboard",
            Self::Limits => "Limits",
            Self::Keymap => "Keymap",
            Self::FileDialogKeymap => "File Dialog Keymap",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Paths => "paths",
            Self::Source => "source",
            Self::Terminal => "terminal",
            Self::Input => "input",
            Self::Clipboard => "clipboard",
            Self::Limits => "limits",
            Self::Keymap => "keymap",
            Self::FileDialogKeymap => "file dialog keymap",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandOutputSection {
    Stdout,
    Stderr,
}

impl CommandOutputSection {
    const fn label(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }

    const fn view_title(self) -> &'static str {
        match self {
            Self::Stdout => "Command Output Stdout",
            Self::Stderr => "Command Output Stderr",
        }
    }
}

fn parse_command_output_section(input: &str) -> Option<CommandOutputSection> {
    match normalize_command_line_token(input).as_str() {
        "stdout" | "out" => Some(CommandOutputSection::Stdout),
        "stderr" | "err" => Some(CommandOutputSection::Stderr),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OutlineEntry {
    label: String,
    line: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NumberedListRow {
    line: usize,
    index: usize,
}

fn numbered_list_rows(buffer: &TextBuffer) -> Vec<NumberedListRow> {
    (0..buffer.line_count())
        .filter_map(|line| {
            let index = numbered_list_index_for_line(buffer.line(line)?)?;
            Some(NumberedListRow { line, index })
        })
        .collect()
}

fn numbered_list_index_for_line(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let digit_len = trimmed
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .map(char::len_utf8)
        .sum::<usize>();
    if digit_len == 0 || trimmed.get(digit_len..)?.chars().next()? != '.' {
        return None;
    }

    trimmed
        .get(..digit_len)?
        .parse::<usize>()
        .ok()?
        .checked_sub(1)
}

fn outline_entries_for_buffer(buffer: &TextBuffer) -> Vec<OutlineEntry> {
    (0..buffer.line_count())
        .filter_map(|line_index| {
            let line = buffer.line(line_index)?.trim();
            outline_label_for_line(line).map(|label| OutlineEntry {
                label,
                line: line_index,
            })
        })
        .collect()
}

fn outline_label_for_line(line: &str) -> Option<String> {
    const HEADINGS: &[&str] = &[
        "App",
        "File",
        "Edit",
        "Windows",
        "Prompts",
        "Selection",
        "Navigation",
        "File Dialogs",
        "Menus",
        "Notes",
        "Summary",
        "Paths",
        "Source",
        "Terminal",
        "Input",
        "Clipboard",
        "Limits",
        "Keymap",
        "File Dialog Keymap",
        "Index",
    ];
    if HEADINGS.contains(&line) || line.starts_with("--- stdout") || line.starts_with("--- stderr")
    {
        return Some(line.to_string());
    }

    markdown_outline_label(line)
        .or_else(|| bracket_section_outline_label(line))
        .or_else(|| rust_outline_label(line))
        .or_else(|| shell_outline_label(line))
}

fn markdown_outline_label(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = trimmed.get(level..)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let title = rest.trim();
    if title.is_empty() {
        None
    } else {
        Some(format!("{} {title}", "#".repeat(level)))
    }
}

fn bracket_section_outline_label(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("[[")
        .and_then(|rest| rest.strip_suffix("]]"))
        .or_else(|| {
            trimmed
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
        })?
        .trim();
    if inner.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn rust_outline_label(line: &str) -> Option<String> {
    let mut tokens = line.split_whitespace().peekable();
    while matches!(
        tokens.peek().copied(),
        Some("pub")
            | Some("async")
            | Some("unsafe")
            | Some("const")
            | Some("extern")
            | Some("default")
    ) || tokens.peek().is_some_and(|token| token.starts_with("pub("))
    {
        tokens.next();
    }

    match tokens.next()? {
        "fn" => {
            let name = outline_identifier(tokens.next()?)?;
            Some(format!("fn {name}"))
        }
        "struct" => {
            let name = outline_identifier(tokens.next()?)?;
            Some(format!("struct {name}"))
        }
        "enum" => {
            let name = outline_identifier(tokens.next()?)?;
            Some(format!("enum {name}"))
        }
        "trait" => {
            let name = outline_identifier(tokens.next()?)?;
            Some(format!("trait {name}"))
        }
        "mod" => {
            let name = outline_identifier(tokens.next()?)?;
            Some(format!("mod {name}"))
        }
        "impl" => {
            let rest = tokens.collect::<Vec<_>>().join(" ");
            let label = rest
                .split('{')
                .next()
                .unwrap_or_default()
                .split(" where ")
                .next()
                .unwrap_or_default()
                .trim();
            if label.is_empty() {
                Some("impl".to_string())
            } else {
                Some(format!("impl {label}"))
            }
        }
        _ => None,
    }
}

fn shell_outline_label(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("function ") {
        let name = outline_identifier(rest.split_whitespace().next()?)?;
        return Some(format!("function {name}"));
    }

    let Some(name) = trimmed.split("()").next() else {
        return None;
    };
    let name = name.trim();
    if name.is_empty()
        || !trimmed[name.len()..].trim_start().starts_with("()")
        || !name.chars().all(is_shell_identifier_char)
    {
        return None;
    }
    let after = trimmed[name.len() + 2..].trim_start();
    if after.is_empty() || after.starts_with('{') {
        Some(format!("{name}()"))
    } else {
        None
    }
}

fn outline_identifier(token: &str) -> Option<String> {
    let end = token
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'))
        .unwrap_or(token.len());
    let ident = token.get(..end)?.trim();
    if ident.is_empty() {
        None
    } else {
        Some(ident.to_string())
    }
}

fn is_shell_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn outline_text(source: &str, entries: &[OutlineEntry]) -> String {
    let mut out = String::from("Dun Outline\n\n");
    out.push_str(&format!("Source: {source}\n"));
    out.push_str(&format!("Sections: {}\n\n", entries.len()));
    for (index, entry) in entries.iter().enumerate() {
        out.push_str(&format!(
            "{:>3}. L{:<5} {}\n",
            index + 1,
            entry.line + 1,
            entry.label
        ));
    }
    out
}

fn parse_outline_target(target: &str, entries: &[OutlineEntry]) -> Option<usize> {
    if let Ok(number) = target.parse::<usize>() {
        return number.checked_sub(1).filter(|index| *index < entries.len());
    }
    let normalized = normalize_command_line_token(target);
    entries
        .iter()
        .position(|entry| normalize_command_line_token(&entry.label).contains(&normalized))
}

fn search_results_text(
    source: &str,
    spec: &SearchSpec,
    matches: &[SearchMatch],
    buffer: &TextBuffer,
) -> String {
    let mut out = String::from("Dun Search Results\n\n");
    out.push_str(&format!("Source: {source}\n"));
    out.push_str(&format!("Query: {}\n", spec.display()));
    out.push_str(&format!("Matches: {}\n\n", matches.len()));
    for (index, item) in matches.iter().enumerate() {
        let line = buffer.line(item.range.start.line).unwrap_or_default();
        out.push_str(&format!(
            "{:>3}. L{}:C{} {}\n",
            index + 1,
            item.range.start.line + 1,
            item.range.start.column + 1,
            clipped_result_line(line)
        ));
    }
    out
}

fn clipped_result_line(line: &str) -> String {
    const LIMIT: usize = 96;
    let mut out = String::new();
    for (index, ch) in line.chars().enumerate() {
        if index >= LIMIT {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

fn parse_config_diagnostics_section(input: &str) -> Option<ConfigDiagnosticsSection> {
    match normalize_command_line_token(input).as_str() {
        "summary" => Some(ConfigDiagnosticsSection::Summary),
        "paths" | "path" => Some(ConfigDiagnosticsSection::Paths),
        "source" => Some(ConfigDiagnosticsSection::Source),
        "terminal" | "term" => Some(ConfigDiagnosticsSection::Terminal),
        "input" => Some(ConfigDiagnosticsSection::Input),
        "clipboard" | "clip" => Some(ConfigDiagnosticsSection::Clipboard),
        "limits" | "limit" => Some(ConfigDiagnosticsSection::Limits),
        "keymap" | "keys" => Some(ConfigDiagnosticsSection::Keymap),
        "filedialogkeymap" | "filedialogkeys" | "dialogkeymap" | "dialogkeys" => {
            Some(ConfigDiagnosticsSection::FileDialogKeymap)
        }
        _ => None,
    }
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

fn preview_selection_match(
    selection: Option<Selection>,
    matches: &[SearchMatch],
) -> Option<SearchSelection> {
    let range = selection?.range();
    matches
        .iter()
        .position(|item| item.range == range)
        .map(|index| SearchSelection {
            index,
            wrapped: false,
        })
}

fn replacement_status_text(replacement: &str) -> &str {
    if replacement.is_empty() {
        "<empty>"
    } else {
        replacement
    }
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

fn scroll_status(buffer: &BufferState, context: Option<BufferViewContext>) -> String {
    let total = buffer.buffer.line_count().max(1);
    let context = context.unwrap_or(BufferViewContext {
        buffer_id: buffer.id,
        body_height: 1,
        body_width: 1,
    });
    if buffer.word_wrap {
        let total_rows = buffer.wrapped_total_visual_rows(context.body_width.max(1));
        let start_row = buffer
            .wrapped_top_visual_row(context.body_width.max(1))
            .min(total_rows.saturating_sub(1));
        let end_row = start_row
            .saturating_add(context.body_height.max(1))
            .min(total_rows);
        let line = buffer.first_line.min(total.saturating_sub(1)) + 1;
        return format!(
            "View V{}-{end_row}/{total_rows} L{line} wrap",
            start_row + 1
        );
    }

    let height = context.body_height.max(1);
    let start = buffer.first_line.min(total.saturating_sub(1));
    let end = start.saturating_add(height).min(total);
    let max_column = buffer.max_line_display_width();
    let body_width = context.body_width;
    if buffer.first_column == 0 && (body_width == 0 || max_column <= body_width) {
        format!("View {}-{end}/{total}", start + 1)
    } else {
        let column_start = buffer.first_column + 1;
        let column_end = buffer
            .first_column
            .saturating_add(body_width.max(1))
            .min(max_column.max(column_start));
        format!(
            "View {}-{end}/{total} X{}-{}/{}",
            start + 1,
            column_start,
            column_end,
            max_column.max(1)
        )
    }
}

fn buffer_disk_state(buffer: &BufferState) -> Option<&'static str> {
    let path = buffer.path.as_ref()?;
    let snapshot = buffer.file_snapshot?;
    match current_file_snapshot(path) {
        Ok(current) if current == snapshot => None,
        Ok(_) => Some("Disk Changed"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Some("Disk Missing"),
        Err(_) => Some("Disk ?"),
    }
}

fn bracket(text: &str) -> String {
    format!("[{text}]")
}

const fn file_encoding_status(encoding: FileTextEncoding) -> &'static str {
    match encoding {
        FileTextEncoding::Utf8 => "UTF-8",
        FileTextEncoding::EscapedBytes => "Escaped Bytes",
    }
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct BufferSearchState {
    spec: SearchSpec,
    matches: Vec<SearchMatch>,
    revision: u64,
    active_index: Option<usize>,
}

impl BufferSearchState {
    fn refresh(&mut self, buffer: &TextBuffer) {
        if self.revision == buffer.revision() {
            self.active_index = current_match_selection(buffer, &self.matches)
                .map(|selection| selection.index)
                .or_else(|| {
                    self.active_index
                        .filter(|index| *index < self.matches.len())
                });
            return;
        }

        let previous_active = self.active_index;
        self.matches = buffer.find_all_with_options(&self.spec.query, self.spec.options);
        self.revision = buffer.revision();
        self.active_index = current_match_selection(buffer, &self.matches)
            .map(|selection| selection.index)
            .or_else(|| previous_active.filter(|index| *index < self.matches.len()));
    }

    fn status_text(&self) -> String {
        match (self.matches.len(), self.active_index) {
            (0, _) => "Find 0".to_string(),
            (total, Some(index)) => format!("Find {}/{total}", index + 1),
            (total, None) => format!("Find {total}"),
        }
    }
}

struct BufferState {
    id: BufferId,
    buffer: TextBuffer,
    path: Option<PathBuf>,
    encoding: FileTextEncoding,
    file_snapshot: Option<FileReadSnapshot>,
    first_line: usize,
    first_visual_row: usize,
    first_column: usize,
    search: Option<BufferSearchState>,
    word_wrap: bool,
    visible_whitespace: bool,
    bookmarks: Vec<usize>,
}

impl BufferState {
    fn new(id: BufferId, buffer: TextBuffer) -> Self {
        Self {
            id,
            buffer,
            path: None,
            encoding: FileTextEncoding::Utf8,
            file_snapshot: None,
            first_line: 0,
            first_visual_row: 0,
            first_column: 0,
            search: None,
            word_wrap: false,
            visible_whitespace: false,
            bookmarks: Vec::new(),
        }
    }

    fn from_file(id: BufferId, path: PathBuf, loaded: LoadedTextBuffer) -> Self {
        Self {
            id,
            buffer: loaded.buffer,
            path: Some(path),
            encoding: loaded.encoding,
            file_snapshot: loaded.snapshot,
            first_line: 0,
            first_visual_row: 0,
            first_column: 0,
            search: None,
            word_wrap: false,
            visible_whitespace: false,
            bookmarks: Vec::new(),
        }
    }

    fn set_search(
        &mut self,
        spec: SearchSpec,
        matches: Vec<SearchMatch>,
        active_index: Option<usize>,
    ) {
        let active_index = active_index.filter(|index| *index < matches.len());
        self.search = Some(BufferSearchState {
            spec,
            matches,
            revision: self.buffer.revision(),
            active_index,
        });
    }

    fn refresh_search_cache(&mut self) {
        if let Some(search) = &mut self.search {
            search.refresh(&self.buffer);
        }
    }

    fn search_status(&self) -> Option<String> {
        let search = self.search.as_ref()?;
        (search.revision == self.buffer.revision()).then(|| search.status_text())
    }

    fn ensure_cursor_visible(&mut self, body_height: usize, body_width: usize) {
        if self.word_wrap {
            self.ensure_cursor_visible_wrapped(body_height, body_width);
            return;
        }
        self.first_visual_row = 0;
        if body_height == 0 {
            self.first_line = self.buffer.cursor_position().line;
        } else {
            let cursor_line = self.buffer.cursor_position().line;
            if cursor_line < self.first_line {
                self.first_line = cursor_line;
            } else if cursor_line >= self.first_line.saturating_add(body_height) {
                self.first_line = cursor_line.saturating_sub(body_height - 1);
            }
        }

        self.ensure_cursor_column_visible(body_width);
    }

    fn ensure_cursor_visible_wrapped(&mut self, body_height: usize, body_width: usize) {
        self.first_column = 0;
        let body_width = body_width.max(1);
        let cursor_row =
            self.wrapped_visual_row_for_position(self.buffer.cursor_position(), body_width);
        if body_height == 0 {
            self.set_wrapped_top_visual_row(cursor_row, body_width);
            return;
        }

        let top = self.wrapped_top_visual_row(body_width);
        let height = body_height.max(1);
        if cursor_row < top {
            self.set_wrapped_top_visual_row(cursor_row, body_width);
        } else if cursor_row >= top.saturating_add(height) {
            self.set_wrapped_top_visual_row(cursor_row.saturating_sub(height - 1), body_width);
        } else {
            self.normalize_wrapped_top(body_width);
        }
    }

    fn move_page_up(&mut self, lines: usize) -> bool {
        let mut moved = false;
        for _ in 0..lines.max(1) {
            moved |= self.buffer.move_up();
        }
        moved
    }

    fn move_page_down(&mut self, lines: usize) -> bool {
        let mut moved = false;
        for _ in 0..lines.max(1) {
            moved |= self.buffer.move_down();
        }
        moved
    }

    fn move_wrapped_page(&mut self, direction: isize, rows: usize, body_width: usize) -> bool {
        let body_width = body_width.max(1);
        let current = self.buffer.cursor_position();
        let current_row = self.wrapped_visual_row_for_position(current, body_width);
        let current_column = self.wrapped_visual_column_for_position(current, body_width);
        let max_row = self.wrapped_total_visual_rows(body_width).saturating_sub(1);
        let target_row = if direction < 0 {
            current_row.saturating_sub(rows.max(1))
        } else {
            current_row.saturating_add(rows.max(1)).min(max_row)
        };
        let target =
            self.position_for_wrapped_visual_row_column(target_row, current_column, body_width);
        let moved = target != current;
        let _ = self.buffer.set_cursor(target);
        moved
    }

    fn scroll_view_lines(&mut self, delta: isize, body_height: usize, body_width: usize) -> bool {
        if body_height == 0 || self.buffer.line_count() == 0 {
            return false;
        }
        if self.word_wrap {
            return self.scroll_wrapped_visual_rows(delta, body_height, body_width);
        }

        let old_first_line = self.first_line;
        self.first_visual_row = 0;
        let max_first_line = self.buffer.line_count().saturating_sub(body_height.max(1));
        self.first_line = if delta < 0 {
            self.first_line.saturating_sub(delta.unsigned_abs())
        } else {
            self.first_line
                .saturating_add(delta as usize)
                .min(max_first_line)
        };

        self.keep_cursor_inside_visible_lines(body_height);
        self.first_line != old_first_line
    }

    fn scroll_view_to_line(
        &mut self,
        first_line: usize,
        first_visual_row: usize,
        body_height: usize,
        body_width: usize,
    ) -> bool {
        if body_height == 0 || self.buffer.line_count() == 0 {
            return false;
        }
        if self.word_wrap {
            let old = (self.first_line, self.first_visual_row);
            let target = self
                .wrapped_visual_row_for_line(first_line, body_width.max(1))
                .saturating_add(first_visual_row);
            self.set_wrapped_top_visual_row(target, body_width.max(1));
            self.keep_cursor_inside_visible_wrapped_rows(body_height, body_width.max(1));
            return old != (self.first_line, self.first_visual_row);
        }

        let old_first_line = self.first_line;
        let max_first_line = self.buffer.line_count().saturating_sub(body_height.max(1));
        self.first_line = first_line.min(max_first_line);
        self.first_visual_row = 0;
        self.keep_cursor_inside_visible_lines(body_height);
        self.first_line != old_first_line
    }

    fn scroll_view_columns(&mut self, delta: isize, body_width: usize) -> bool {
        if self.word_wrap {
            self.first_column = 0;
            return false;
        }

        if body_width == 0 {
            return false;
        }

        let old_first_column = self.first_column;
        let max_first_column = self
            .max_line_display_width()
            .saturating_sub(body_width.max(1));
        self.first_column = if delta < 0 {
            self.first_column.saturating_sub(delta.unsigned_abs())
        } else {
            self.first_column
                .saturating_add(delta as usize)
                .min(max_first_column)
        };
        self.first_column != old_first_column
    }

    fn extend_page_up(&mut self, lines: usize) -> bool {
        let mut moved = false;
        for _ in 0..lines.max(1) {
            moved |= self.buffer.extend_selection_up();
        }
        moved
    }

    fn extend_page_down(&mut self, lines: usize) -> bool {
        let mut moved = false;
        for _ in 0..lines.max(1) {
            moved |= self.buffer.extend_selection_down();
        }
        moved
    }

    fn extend_wrapped_page(&mut self, direction: isize, rows: usize, body_width: usize) -> bool {
        let body_width = body_width.max(1);
        let current = self.buffer.cursor_position();
        let current_row = self.wrapped_visual_row_for_position(current, body_width);
        let current_column = self.wrapped_visual_column_for_position(current, body_width);
        let max_row = self.wrapped_total_visual_rows(body_width).saturating_sub(1);
        let target_row = if direction < 0 {
            current_row.saturating_sub(rows.max(1))
        } else {
            current_row.saturating_add(rows.max(1)).min(max_row)
        };
        let target =
            self.position_for_wrapped_visual_row_column(target_row, current_column, body_width);
        let anchor = self
            .buffer
            .selection()
            .map(|selection| selection.anchor)
            .unwrap_or(current);
        let moved = target != current;
        let _ = self.buffer.select(anchor, target);
        moved
    }

    fn ensure_cursor_column_visible(&mut self, body_width: usize) {
        if self.word_wrap {
            self.first_column = 0;
            self.normalize_wrapped_top(body_width.max(1));
            return;
        }

        let cursor_column = self.cursor_display_column();
        if body_width == 0 {
            self.first_column = cursor_column;
            return;
        }

        if cursor_column < self.first_column {
            self.first_column = cursor_column;
        } else if cursor_column >= self.first_column.saturating_add(body_width) {
            self.first_column = cursor_column.saturating_sub(body_width - 1);
        }
    }

    fn cursor_display_column(&self) -> usize {
        let position = self.buffer.cursor_position();
        self.buffer
            .line(position.line)
            .and_then(|line| line.get(..position.column))
            .map(UnicodeWidthStr::width)
            .unwrap_or(0)
    }

    fn max_line_display_width(&self) -> usize {
        self.buffer
            .lines()
            .iter()
            .map(|line| UnicodeWidthStr::width(line.as_str()))
            .max()
            .unwrap_or(0)
    }

    fn scroll_wrapped_visual_rows(
        &mut self,
        delta: isize,
        body_height: usize,
        body_width: usize,
    ) -> bool {
        let body_width = body_width.max(1);
        let old = (self.first_line, self.first_visual_row);
        let top = self.wrapped_top_visual_row(body_width);
        let max_top = self
            .wrapped_total_visual_rows(body_width)
            .saturating_sub(body_height.max(1));
        let next = if delta < 0 {
            top.saturating_sub(delta.unsigned_abs())
        } else {
            top.saturating_add(delta as usize).min(max_top)
        };
        self.set_wrapped_top_visual_row(next, body_width);
        self.keep_cursor_inside_visible_wrapped_rows(body_height, body_width);
        old != (self.first_line, self.first_visual_row)
    }

    fn normalize_wrapped_top(&mut self, body_width: usize) {
        if !self.word_wrap {
            self.first_visual_row = 0;
            return;
        }
        let body_width = body_width.max(1);
        let top = self.wrapped_top_visual_row(body_width);
        self.set_wrapped_top_visual_row(top, body_width);
    }

    fn wrapped_total_visual_rows(&self, body_width: usize) -> usize {
        (0..self.buffer.line_count())
            .map(|line_index| self.wrapped_line_visual_rows(line_index, body_width))
            .sum::<usize>()
            .max(1)
    }

    fn wrapped_top_visual_row(&self, body_width: usize) -> usize {
        self.wrapped_visual_row_for_line(self.first_line, body_width)
            .saturating_add(
                self.first_visual_row.min(
                    self.wrapped_line_visual_rows(self.first_line, body_width)
                        .saturating_sub(1),
                ),
            )
    }

    fn wrapped_visual_row_for_line(&self, line_index: usize, body_width: usize) -> usize {
        (0..line_index.min(self.buffer.line_count()))
            .map(|line| self.wrapped_line_visual_rows(line, body_width))
            .sum()
    }

    fn set_wrapped_top_visual_row(&mut self, target_row: usize, body_width: usize) {
        let body_width = body_width.max(1);
        let max_row = self.wrapped_total_visual_rows(body_width).saturating_sub(1);
        let mut remaining = target_row.min(max_row);
        for line_index in 0..self.buffer.line_count() {
            let rows = self.wrapped_line_visual_rows(line_index, body_width);
            if remaining < rows {
                self.first_line = line_index;
                self.first_visual_row = remaining;
                self.first_column = 0;
                return;
            }
            remaining = remaining.saturating_sub(rows);
        }

        self.first_line = self.buffer.line_count().saturating_sub(1);
        self.first_visual_row = 0;
        self.first_column = 0;
    }

    fn wrapped_visual_row_for_position(&self, position: Position, body_width: usize) -> usize {
        self.wrapped_visual_row_for_line(position.line, body_width)
            .saturating_add(self.wrapped_row_offset_for_position(position, body_width))
    }

    fn wrapped_row_offset_for_position(&self, position: Position, body_width: usize) -> usize {
        self.wrapped_row_column_for_position(position, body_width).0
    }

    fn wrapped_visual_column_for_position(&self, position: Position, body_width: usize) -> usize {
        self.wrapped_row_column_for_position(position, body_width).1
    }

    fn wrapped_row_column_for_position(
        &self,
        position: Position,
        body_width: usize,
    ) -> (usize, usize) {
        let body_width = body_width.max(1);
        let Some(line) = self.buffer.line(position.line) else {
            return (0, 0);
        };
        let prefix = line.get(..position.column).unwrap_or(line);
        let mut row = 0usize;
        let mut column = 0usize;
        for ch in prefix.chars() {
            advance_wrapped_column(
                &mut row,
                &mut column,
                display_width_for_editor_char(ch),
                body_width,
            );
        }
        if column >= body_width && position.column < line.len() {
            row = row.saturating_add(1);
            column = 0;
        }
        (row, column)
    }

    fn wrapped_line_visual_rows(&self, line_index: usize, body_width: usize) -> usize {
        let body_width = body_width.max(1);
        let Some(line) = self.buffer.line(line_index) else {
            return 1;
        };
        let mut row = 0usize;
        let mut column = 0usize;
        if line.is_empty() {
            return 1;
        }
        for ch in line.chars() {
            advance_wrapped_column(
                &mut row,
                &mut column,
                display_width_for_editor_char(ch),
                body_width,
            );
        }
        if self.visible_whitespace {
            advance_wrapped_column(&mut row, &mut column, 1, body_width);
        }
        row.saturating_add(1)
    }

    fn position_for_wrapped_visual_row(&self, target_row: usize, body_width: usize) -> Position {
        let body_width = body_width.max(1);
        let mut remaining = target_row;
        for line_index in 0..self.buffer.line_count() {
            let rows = self.wrapped_line_visual_rows(line_index, body_width);
            if remaining < rows {
                let line = self.buffer.line(line_index).unwrap_or_default();
                return Position::new(
                    line_index,
                    byte_column_for_wrapped_row_start(line, remaining, body_width),
                );
            }
            remaining = remaining.saturating_sub(rows);
        }
        buffer_end_position(&self.buffer)
    }

    fn position_for_wrapped_visual_row_column(
        &self,
        target_row: usize,
        target_column: usize,
        body_width: usize,
    ) -> Position {
        let body_width = body_width.max(1);
        let mut remaining = target_row;
        for line_index in 0..self.buffer.line_count() {
            let rows = self.wrapped_line_visual_rows(line_index, body_width);
            if remaining < rows {
                let line = self.buffer.line(line_index).unwrap_or_default();
                return Position::new(
                    line_index,
                    byte_column_for_wrapped_row_column(line, remaining, target_column, body_width),
                );
            }
            remaining = remaining.saturating_sub(rows);
        }
        buffer_end_position(&self.buffer)
    }

    fn normalize_bookmarks(&mut self) {
        let max_line = self.buffer.line_count().saturating_sub(1);
        for bookmark in &mut self.bookmarks {
            *bookmark = (*bookmark).min(max_line);
        }
        self.bookmarks.sort_unstable();
        self.bookmarks.dedup();
    }

    fn remap_bookmarks_after_line_move(&mut self, direction: isize) {
        let moved_to = self.buffer.cursor_position().line;
        let moved_from = if direction < 0 {
            moved_to.saturating_add(1)
        } else {
            moved_to.saturating_sub(1)
        };
        for bookmark in &mut self.bookmarks {
            if *bookmark == moved_from {
                *bookmark = moved_to;
            } else if *bookmark == moved_to {
                *bookmark = moved_from;
            }
        }
        self.normalize_bookmarks();
    }

    fn keep_cursor_inside_visible_lines(&mut self, body_height: usize) {
        if body_height == 0 {
            return;
        }

        let cursor = self.buffer.cursor_position();
        let last_visible = self
            .first_line
            .saturating_add(body_height.saturating_sub(1))
            .min(self.buffer.line_count().saturating_sub(1));
        let target_line = cursor.line.clamp(self.first_line, last_visible);
        if target_line == cursor.line {
            return;
        }

        let target_column = self.clamp_column_to_line(target_line, cursor.column);
        let _ = self
            .buffer
            .set_cursor(Position::new(target_line, target_column));
    }

    fn keep_cursor_inside_visible_wrapped_rows(&mut self, body_height: usize, body_width: usize) {
        if body_height == 0 {
            return;
        }

        let body_width = body_width.max(1);
        let top = self.wrapped_top_visual_row(body_width);
        let bottom = top.saturating_add(body_height.saturating_sub(1));
        let cursor_row =
            self.wrapped_visual_row_for_position(self.buffer.cursor_position(), body_width);
        let target_row = cursor_row.clamp(top, bottom);
        if target_row == cursor_row {
            return;
        }

        let _ = self
            .buffer
            .set_cursor(self.position_for_wrapped_visual_row(target_row, body_width));
    }

    fn clamp_column_to_line(&self, line_index: usize, target_column: usize) -> usize {
        let Some(line) = self.buffer.line(line_index) else {
            return 0;
        };
        let mut column = target_column.min(line.len());
        while !line.is_char_boundary(column) {
            column -= 1;
        }
        column
    }
}

fn editor_body_width(buffer: &BufferState, rect: Rect) -> usize {
    let inner_width = rect.width.saturating_sub(2);
    let gutter_width = editor_gutter_width(buffer, rect).min(inner_width);
    inner_width.saturating_sub(gutter_width) as usize
}

fn editor_gutter_width(buffer: &BufferState, rect: Rect) -> u16 {
    let inner_width = rect.width.saturating_sub(2);
    let digits = decimal_digits_for_editor(buffer.buffer.line_count().max(1));
    let width = (digits + 1) as u16;
    if inner_width < width.saturating_add(MIN_BODY_COLUMNS_WITH_GUTTER) {
        0
    } else {
        width
    }
}

fn decimal_digits_for_editor(mut value: usize) -> usize {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct LineInput {
    text: String,
    cursor_index: usize,
}

impl LineInput {
    fn new(text: String) -> Self {
        Self {
            cursor_index: text.len(),
            text,
        }
    }

    fn as_str(&self) -> &str {
        &self.text
    }

    fn set_text(&mut self, text: String) {
        self.cursor_index = text.len();
        self.text = text;
    }

    fn cursor_display_column(&self) -> usize {
        self.text
            .get(..self.cursor_index)
            .map(UnicodeWidthStr::width)
            .unwrap_or_else(|| UnicodeWidthStr::width(self.text.as_str()))
    }

    fn move_left(&mut self) {
        if self.cursor_index > 0 {
            self.cursor_index = previous_char_boundary(&self.text, self.cursor_index);
        }
    }

    fn move_right(&mut self) {
        if self.cursor_index < self.text.len() {
            self.cursor_index = next_char_boundary(&self.text, self.cursor_index);
        }
    }

    fn move_start(&mut self) {
        self.cursor_index = 0;
    }

    fn move_end(&mut self) {
        self.cursor_index = self.text.len();
    }

    fn insert_char(&mut self, ch: char) {
        self.text.insert(self.cursor_index, ch);
        self.cursor_index += ch.len_utf8();
    }

    fn insert_str(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.text.insert_str(self.cursor_index, text);
        self.cursor_index += text.len();
    }

    fn delete_backward(&mut self) {
        if self.cursor_index == 0 {
            return;
        }

        let start = previous_char_boundary(&self.text, self.cursor_index);
        self.text.drain(start..self.cursor_index);
        self.cursor_index = start;
    }

    fn delete_forward(&mut self) {
        if self.cursor_index >= self.text.len() {
            return;
        }

        let end = next_char_boundary(&self.text, self.cursor_index);
        self.text.drain(self.cursor_index..end);
    }
}

const STATUS_HISTORY_LIMIT: usize = 128;
const COMMAND_HISTORY_LIMIT: usize = 128;

struct TerminalGuard {
    mouse_enabled: bool,
    bracketed_paste_enabled: bool,
    active: bool,
}

impl TerminalGuard {
    fn enter(mouse_enabled: bool) -> io::Result<Self> {
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

    fn set_mouse_enabled(&mut self, enabled: bool) -> io::Result<()> {
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

    fn suspend(&mut self) -> io::Result<()> {
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

    fn resume(&mut self, mouse_enabled: bool) -> io::Result<()> {
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

#[derive(Clone)]
struct TerminalColorRewrite {
    rewrite_16_color_sgr: Arc<AtomicBool>,
}

impl TerminalColorRewrite {
    fn new(profile: TerminalProfile) -> Self {
        Self {
            rewrite_16_color_sgr: Arc::new(AtomicBool::new(should_rewrite_16_color_sgr(profile))),
        }
    }

    fn set_profile(&self, profile: TerminalProfile) {
        self.rewrite_16_color_sgr
            .store(should_rewrite_16_color_sgr(profile), Ordering::Relaxed);
    }

    fn is_enabled(&self) -> bool {
        self.rewrite_16_color_sgr.load(Ordering::Relaxed)
    }
}

fn should_rewrite_16_color_sgr(profile: TerminalProfile) -> bool {
    matches!(profile.colors, ColorProfile::Color16)
}

struct TerminalWriter {
    inner: Stdout,
    color_rewrite: TerminalColorRewrite,
    pending_escape: Vec<u8>,
}

impl TerminalWriter {
    fn new(inner: Stdout, color_rewrite: TerminalColorRewrite) -> Self {
        Self {
            inner,
            color_rewrite,
            pending_escape: Vec::new(),
        }
    }
}

impl Write for TerminalWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if !self.color_rewrite.is_enabled() {
            return self.inner.write(buffer);
        }

        let rewritten = rewrite_16_color_sgr(buffer, &mut self.pending_escape);
        self.inner.write_all(&rewritten)?;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.color_rewrite.is_enabled() && !self.pending_escape.is_empty() {
            self.inner.write_all(&self.pending_escape)?;
            self.pending_escape.clear();
        }
        self.inner.flush()
    }
}

const MAX_PENDING_ESCAPE_BYTES: usize = 1024;

fn rewrite_16_color_sgr(buffer: &[u8], pending_escape: &mut Vec<u8>) -> Vec<u8> {
    let mut input = Vec::with_capacity(pending_escape.len().saturating_add(buffer.len()));
    if !pending_escape.is_empty() {
        input.extend_from_slice(pending_escape);
        pending_escape.clear();
    }
    input.extend_from_slice(buffer);

    let mut output = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        if input[index] != 0x1b {
            output.push(input[index]);
            index += 1;
            continue;
        }

        if index + 1 >= input.len() {
            pending_escape.extend_from_slice(&input[index..]);
            break;
        }

        if input[index + 1] != b'[' {
            output.push(input[index]);
            output.push(input[index + 1]);
            index += 2;
            continue;
        }

        let mut end = index + 2;
        while end < input.len() && !is_csi_final_byte(input[end]) {
            end += 1;
        }

        if end >= input.len() {
            let pending = &input[index..];
            if pending.len() <= MAX_PENDING_ESCAPE_BYTES {
                pending_escape.extend_from_slice(pending);
            } else {
                output.extend_from_slice(pending);
            }
            break;
        }

        if input[end] == b'm' {
            output.extend_from_slice(&rewrite_16_color_sgr_sequence(&input[index + 2..end]));
        } else {
            output.extend_from_slice(&input[index..=end]);
        }
        index = end + 1;
    }

    output
}

fn is_csi_final_byte(byte: u8) -> bool {
    (0x40..=0x7e).contains(&byte)
}

fn rewrite_16_color_sgr_sequence(params: &[u8]) -> Vec<u8> {
    let Some(values) = parse_sgr_params(params) else {
        return original_sgr_sequence(params);
    };

    let mut rewritten = Vec::with_capacity(values.len());
    let mut index = 0;
    while index < values.len() {
        let value = values[index];
        if matches!(value, 38 | 48)
            && index + 2 < values.len()
            && values[index + 1] == 5
            && values[index + 2] <= 15
        {
            rewritten.push(legacy_16_color_sgr_code(value == 48, values[index + 2]));
            index += 3;
            continue;
        }

        rewritten.push(value);
        index += 1;
    }

    let mut output = Vec::new();
    output.extend_from_slice(b"\x1b[");
    for (index, value) in rewritten.iter().enumerate() {
        if index > 0 {
            output.push(b';');
        }
        output.extend_from_slice(value.to_string().as_bytes());
    }
    output.push(b'm');
    output
}

fn parse_sgr_params(params: &[u8]) -> Option<Vec<u16>> {
    if params.is_empty() {
        return Some(vec![0]);
    }

    let mut values = Vec::new();
    for param in params.split(|byte| *byte == b';') {
        if param.is_empty() {
            values.push(0);
            continue;
        }
        if !param.iter().all(u8::is_ascii_digit) {
            return None;
        }
        let text = std::str::from_utf8(param).ok()?;
        values.push(text.parse().ok()?);
    }
    Some(values)
}

fn legacy_16_color_sgr_code(background: bool, color: u16) -> u16 {
    let base = match (background, color < 8) {
        (false, true) => 30,
        (false, false) => 90,
        (true, true) => 40,
        (true, false) => 100,
    };
    base + (color % 8)
}

fn original_sgr_sequence(params: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(params.len().saturating_add(3));
    output.extend_from_slice(b"\x1b[");
    output.extend_from_slice(params);
    output.push(b'm');
    output
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<TerminalWriter>>,
    app: &mut AppState,
    guard: &mut TerminalGuard,
    color_rewrite: &TerminalColorRewrite,
) -> io::Result<()> {
    while !app.should_quit {
        guard.set_mouse_enabled(app.mouse_enabled())?;
        color_rewrite.set_profile(app.shell.profile);
        terminal.draw(|frame| {
            let area = frame.area();
            let workspace_area = Rect::new(0, 0, area.width, area.height.saturating_sub(2));
            app.sync_view_for_area(workspace_area);
            let buffer_views = app.buffer_views();
            let mut ui_frame = app.shell.frame_for_workspace_with_menu_selection(
                &app.workspace,
                workspace_area,
                &buffer_views,
                app.menu_selection(),
            );
            if let Some(message) = &app.status_message {
                ui_frame.status.left = message.clone();
            } else {
                ui_frame.status.left = app.focused_buffer_status();
            }
            if app.prompt.is_none()
                && app.file_dialog.is_none()
                && app.buffer_switcher.is_none()
                && app.confirm.is_none()
                && app.replace_confirm.is_none()
            {
                ui_frame.status.left = format!(
                    "{} {}",
                    app.focused_buffer_status(),
                    app.focused_detail_status()
                );
            }
            ui_frame.status.right = app.focused_file_status();
            ui_frame.overlay = app.active_overlay();
            app.shell.render(frame, &ui_frame);
        })?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(event) => handle_key_event(app, event),
                Event::Paste(text) => app.handle_paste(&text),
                Event::Mouse(event) => handle_mouse_event(app, event),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if let Some(action) = app.take_runtime_action() {
            handle_runtime_action(action, terminal, app, guard)?;
        }
    }

    Ok(())
}

fn handle_runtime_action(
    action: RuntimeAction,
    terminal: &mut Terminal<CrosstermBackend<TerminalWriter>>,
    app: &mut AppState,
    guard: &mut TerminalGuard,
) -> io::Result<()> {
    match action {
        RuntimeAction::ShellEscape => run_shell_escape(terminal, app, guard),
        RuntimeAction::WriteTerminal(payload) => {
            let mut stdout = io::stdout();
            stdout.write_all(payload.as_bytes())?;
            stdout.flush()?;
            Ok(())
        }
    }
}

fn run_shell_escape(
    terminal: &mut Terminal<CrosstermBackend<TerminalWriter>>,
    app: &mut AppState,
    guard: &mut TerminalGuard,
) -> io::Result<()> {
    terminal.show_cursor()?;
    guard.suspend()?;
    let status = run_interactive_shell();
    let resume_result = guard.resume(app.mouse_enabled());
    if resume_result.is_ok() {
        terminal.clear()?;
    }

    match (status, resume_result) {
        (Ok(status), Ok(())) => {
            app.set_status(format!("Shell returned {}", exit_status_text(status)));
            Ok(())
        }
        (Err(error), Ok(())) => {
            app.set_status(format!("Shell failed: {error}"));
            Ok(())
        }
        (_, Err(error)) => Err(error),
    }
}

fn handle_mouse_event(app: &mut AppState, event: CrosstermMouseEvent) {
    if !app.mouse_enabled()
        || app.prompt.is_some()
        || app.confirm.is_some()
        || app.replace_confirm.is_some()
    {
        app.handle_mouse_up();
        return;
    }

    if app.buffer_switcher.is_some() {
        match event.kind {
            CrosstermMouseEventKind::Down(CrosstermMouseButton::Left) => {
                app.handle_buffer_switcher_mouse_down(event.column, event.row);
            }
            CrosstermMouseEventKind::Down(CrosstermMouseButton::Right) => {
                app.note_right_click_paste();
            }
            CrosstermMouseEventKind::ScrollUp => {
                app.scroll_buffer_switcher(-1);
            }
            CrosstermMouseEventKind::ScrollDown => {
                app.scroll_buffer_switcher(1);
            }
            CrosstermMouseEventKind::ScrollLeft | CrosstermMouseEventKind::ScrollRight => {}
            CrosstermMouseEventKind::Up(CrosstermMouseButton::Left) => {
                app.handle_mouse_up();
            }
            _ => {}
        }
        return;
    }

    if app.file_dialog.is_some() {
        match event.kind {
            CrosstermMouseEventKind::Down(CrosstermMouseButton::Left) => {
                app.handle_file_dialog_mouse_down(event.column, event.row);
            }
            CrosstermMouseEventKind::Down(CrosstermMouseButton::Right) => {
                app.note_right_click_paste();
            }
            CrosstermMouseEventKind::ScrollUp => {
                app.scroll_file_dialog(-1);
            }
            CrosstermMouseEventKind::ScrollDown => {
                app.scroll_file_dialog(1);
            }
            CrosstermMouseEventKind::ScrollLeft | CrosstermMouseEventKind::ScrollRight => {}
            CrosstermMouseEventKind::Up(CrosstermMouseButton::Left) => {
                app.handle_mouse_up();
            }
            _ => {}
        }
        return;
    }

    match event.kind {
        CrosstermMouseEventKind::Down(CrosstermMouseButton::Left) => {
            app.handle_mouse_down(event.column, event.row);
        }
        CrosstermMouseEventKind::Down(CrosstermMouseButton::Right) => {
            app.note_right_click_paste();
        }
        CrosstermMouseEventKind::Drag(CrosstermMouseButton::Left) => {
            app.handle_mouse_drag(event.column, event.row);
        }
        CrosstermMouseEventKind::ScrollUp => {
            app.handle_mouse_scroll(
                event.column,
                event.row,
                -(EDITOR_MOUSE_WHEEL_LINES as isize),
            );
        }
        CrosstermMouseEventKind::ScrollDown => {
            app.handle_mouse_scroll(event.column, event.row, EDITOR_MOUSE_WHEEL_LINES as isize);
        }
        CrosstermMouseEventKind::ScrollLeft => {
            app.scroll_focused_columns(-1);
        }
        CrosstermMouseEventKind::ScrollRight => {
            app.scroll_focused_columns(1);
        }
        CrosstermMouseEventKind::Up(CrosstermMouseButton::Left) => {
            app.handle_mouse_up();
        }
        _ => {}
    }
}

fn handle_key_event(app: &mut AppState, event: CrosstermKeyEvent) {
    if matches!(event.kind, CrosstermKeyEventKind::Release) {
        return;
    }

    if app.active_menu.is_some() {
        handle_active_menu_key_event(app, event);
        return;
    }

    if app.handle_confirm_key_event(event) {
        return;
    }

    if app.handle_replace_confirm_key_event(event) {
        return;
    }

    if app.handle_buffer_switcher_key_event(event) {
        return;
    }

    if app.handle_file_dialog_key_event(event) {
        return;
    }

    if app.handle_prompt_key_event(event) {
        return;
    }

    let Some(stroke) = key_stroke_from_crossterm(event) else {
        return;
    };

    if app.handle_auxiliary_enter_key_stroke(stroke) {
        return;
    }

    if app.handle_key_stroke(stroke) {
        return;
    }

    if app.handle_selection_key_stroke(stroke) {
        return;
    }

    if app.handle_auxiliary_window_key_stroke(stroke) {
        return;
    }

    if handle_menu_mnemonic_key_event(app, event) {
        return;
    }

    if let Some(ch) = text_input_from_crossterm(event) {
        app.handle_text_input(ch);
    }
}

fn handle_active_menu_key_event(app: &mut AppState, event: CrosstermKeyEvent) {
    match event.code {
        CrosstermKeyCode::Esc => app.clear_active_menu(),
        CrosstermKeyCode::Left => {
            app.move_active_menu(-1);
        }
        CrosstermKeyCode::Right => {
            app.move_active_menu(1);
        }
        CrosstermKeyCode::Up => {
            app.move_active_menu_entry(-1);
        }
        CrosstermKeyCode::Down => {
            app.move_active_menu_entry(1);
        }
        CrosstermKeyCode::Enter => {
            app.dispatch_active_menu_entry();
        }
        CrosstermKeyCode::Char(ch) if event.modifiers.contains(CrosstermKeyModifiers::ALT) => {
            if let Some(menu_index) = app.shell.menu_index_for_mnemonic(ch) {
                app.open_keyboard_menu(menu_index);
            }
        }
        _ => {}
    }
}

fn handle_menu_mnemonic_key_event(app: &mut AppState, event: CrosstermKeyEvent) -> bool {
    if !event.modifiers.contains(CrosstermKeyModifiers::ALT)
        || event.modifiers.contains(CrosstermKeyModifiers::CONTROL)
    {
        return false;
    }
    let CrosstermKeyCode::Char(ch) = event.code else {
        return false;
    };
    let Some(menu_index) = app.shell.menu_index_for_mnemonic(ch) else {
        return false;
    };

    app.pending_keys.clear();
    app.open_keyboard_menu(menu_index);
    true
}

fn wrapping_index(index: usize, len: usize, delta: isize) -> usize {
    debug_assert!(len > 0);
    let len = len as isize;
    (index as isize).saturating_add(delta).rem_euclid(len) as usize
}

fn file_dialog_context(input: &str) -> FileDialogContext {
    let input = input.trim();
    if input.is_empty() {
        return FileDialogContext {
            base_input: String::new(),
            prefix: String::new(),
            directory: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        };
    }

    if let Some(index) = input.rfind('/') {
        let base_input = input[..=index].to_string();
        let prefix = input[index + 1..].to_string();
        let directory = directory_path_from_dialog_base(&base_input);
        return FileDialogContext {
            base_input,
            prefix,
            directory,
        };
    }

    FileDialogContext {
        base_input: String::new(),
        prefix: input.to_string(),
        directory: PathBuf::from("."),
    }
}

fn directory_path_from_dialog_base(base: &str) -> PathBuf {
    if base == "/" {
        return PathBuf::from("/");
    }

    let without_trailing = base.trim_end_matches('/');
    if without_trailing.is_empty() {
        PathBuf::from(".")
    } else {
        expand_user_path(without_trailing)
    }
}

fn parent_input_for_dialog_base(base: &str) -> String {
    if base.is_empty() {
        return "../".to_string();
    }
    if base == "/" {
        return "/".to_string();
    }

    let trimmed = base.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }

    if let Some(index) = trimmed.rfind('/') {
        return trimmed[..=index].to_string();
    }

    String::new()
}

fn list_file_dialog_entries(
    context: &FileDialogContext,
    show_hidden: bool,
) -> io::Result<FileDialogListing> {
    let mut entries = Vec::new();
    let mut hidden_filtered = 0;
    let parent_input = parent_input_for_dialog_base(&context.base_input);
    entries.push(FileDialogEntry {
        name: "..".to_string(),
        input: parent_input.clone(),
        path: expand_user_path(&parent_input),
        is_dir: true,
        is_parent: true,
    });

    let include_hidden = show_hidden || context.prefix.starts_with('.');
    for entry in fs::read_dir(&context.directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') && !include_hidden {
            hidden_filtered += 1;
            continue;
        }
        if !name.starts_with(&context.prefix) {
            continue;
        }

        let path = entry.path();
        let is_dir = path.is_dir();
        let input = format!(
            "{}{}{}",
            context.base_input,
            name,
            if is_dir { "/" } else { "" }
        );
        entries.push(FileDialogEntry {
            name,
            path: expand_user_path(&input),
            input,
            is_dir,
            is_parent: false,
        });
    }

    entries.sort_by(|left, right| {
        right
            .is_parent
            .cmp(&left.is_parent)
            .then_with(|| right.is_dir.cmp(&left.is_dir))
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(FileDialogListing {
        entries,
        hidden_filtered,
    })
}

fn file_dialog_list_message(
    context: &FileDialogContext,
    entries: &[FileDialogEntry],
    show_hidden: bool,
    hidden_filtered: usize,
) -> Option<String> {
    let visible_entries = entries.iter().filter(|entry| !entry.is_parent).count();
    if visible_entries > 0 {
        return None;
    }

    if !context.prefix.is_empty() {
        if hidden_filtered > 0 && !show_hidden && !context.prefix.starts_with('.') {
            return Some("No visible matches; type . or toggle hidden files".to_string());
        }
        return Some(format!("No matches for `{}`; `..` goes up", context.prefix));
    }

    if hidden_filtered > 0 && !show_hidden {
        Some("Only hidden entries are filtered; type . or toggle hidden files".to_string())
    } else {
        Some("Directory is empty; `..` goes up".to_string())
    }
}

fn common_entry_prefix(entries: &[FileDialogEntry], current_prefix: &str) -> Option<String> {
    let mut iter = entries
        .iter()
        .filter(|entry| is_completable_file_dialog_entry(entry, current_prefix));
    let first = iter.next()?.name.clone();
    let mut prefix = first;

    for entry in iter {
        prefix = common_prefix(&prefix, &entry.name);
        if prefix.is_empty() {
            break;
        }
    }

    Some(prefix)
}

fn is_completable_file_dialog_entry(entry: &FileDialogEntry, current_prefix: &str) -> bool {
    !entry.is_parent || current_prefix.starts_with("..")
}

fn common_prefix(left: &str, right: &str) -> String {
    let mut end = 0;
    for ((left_index, left_char), (_, right_char)) in left.char_indices().zip(right.char_indices())
    {
        if left_char != right_char {
            break;
        }
        end = left_index + left_char.len_utf8();
    }

    left[..end].to_string()
}

fn previous_char_boundary(input: &str, index: usize) -> usize {
    input[..index]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_char_boundary(input: &str, index: usize) -> usize {
    input[index..]
        .chars()
        .next()
        .map(|ch| index + ch.len_utf8())
        .unwrap_or(input.len())
}

fn single_line_paste_text(input: &str) -> String {
    let mut output = String::new();
    let mut in_line_break = false;
    for ch in input.chars() {
        if matches!(ch, '\r' | '\n') {
            if !in_line_break {
                output.push(' ');
                in_line_break = true;
            }
        } else {
            output.push(ch);
            in_line_break = false;
        }
    }
    output
}

fn expand_user_path(input: &str) -> PathBuf {
    if input == "~" {
        return env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(input));
    }

    if let Some(rest) = input.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }

    PathBuf::from(input)
}

fn ensure_trailing_separator(mut input: String) -> String {
    if !input.ends_with('/') {
        input.push('/');
    }
    input
}

fn file_dialog_recent_input_for_path(path: &Path) -> String {
    path.parent()
        .map(|parent| ensure_trailing_separator(parent.to_string_lossy().into_owned()))
        .unwrap_or_default()
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

fn help_buffer(keymap: &Keymap, file_dialog_keys: &FileDialogKeymap) -> TextBuffer {
    let text = help_text(keymap, file_dialog_keys);
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, &text)
}

fn help_text(keymap: &Keymap, file_dialog_keys: &FileDialogKeymap) -> String {
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
        "Selection\n  Shift+Arrow       Extend selection by character or line\n  Shift+Home/End    Extend selection to line edge\n  Shift+PageUp/Down Extend selection by visible page\n  Ctrl+Shift+Arrow  Extend selection by word when delivered\n\n",
    );
    out.push_str(
        "Navigation\n  PageUp/PageDown   Move by visible page\n  Ctrl+Home/End     Move to document start/end\n  F3/Shift+F3       Repeat find forward/backward\n\n",
    );
    out.push_str("File Dialogs\n");
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::Submit,
        "Open/save selected path",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::Cancel,
        "Cancel dialog",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::CompleteForward,
        "Complete path",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::CompleteBackward,
        "Complete path backward",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::ToggleHidden,
        "Toggle hidden files",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveSelectionUp,
        "Move file selection up",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveSelectionDown,
        "Move file selection down",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::PageSelectionUp,
        "Page file selection up",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::PageSelectionDown,
        "Page file selection down",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveInputLeft,
        "Move path cursor left",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveInputRight,
        "Move path cursor right",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveInputStart,
        "Move path cursor to start",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveInputEnd,
        "Move path cursor to end",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::DeleteBackward,
        "Delete previous path character",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::DeleteForward,
        "Delete path character",
    );
    out.push_str(
        "  Mouse click     Select list entry when mouse is enabled\n  Mouse wheel     Scroll list when mouse is enabled\n\n",
    );
    out.push_str(
        "Menus\n  Alt+F/E/V/H     Open File/Edit/View/Help menu\n  Left/Right      Switch open menu\n  Up/Down         Move menu selection\n  Enter           Run selected menu command\n  Esc             Close open menu\n\n",
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

fn push_file_dialog_help(
    out: &mut String,
    keymap: &FileDialogKeymap,
    action: FileDialogAction,
    description: &str,
) {
    let sequence = file_dialog_action_key_text(keymap, action);
    out.push_str(&format!(
        "  {sequence:<15} {description} [{}]\n",
        file_dialog_action_id(action)
    ));
}

fn file_dialog_action_key_text(keymap: &FileDialogKeymap, action: FileDialogAction) -> String {
    keymap
        .stroke_for_action(action)
        .map(|stroke| stroke.to_string())
        .unwrap_or_else(|| "(unbound)".to_string())
}

fn file_dialog_shortcuts_text(keymap: &FileDialogKeymap) -> String {
    format!(
        "[{}] OK  [{}] Complete  [{}] Hidden  [{}] Cancel",
        file_dialog_action_key_text(keymap, FileDialogAction::Submit),
        file_dialog_action_key_text(keymap, FileDialogAction::CompleteForward),
        file_dialog_action_key_text(keymap, FileDialogAction::ToggleHidden),
        file_dialog_action_key_text(keymap, FileDialogAction::Cancel),
    )
}

fn important_config_diagnostic_commands() -> &'static [EditorCommand] {
    &[
        EditorCommand::App(AppCommand::CommandLine),
        EditorCommand::App(AppCommand::Help),
        EditorCommand::App(AppCommand::ReloadConfig),
        EditorCommand::App(AppCommand::ConfigDiagnostics),
        EditorCommand::App(AppCommand::RunCommand),
        EditorCommand::File(FileCommand::Open),
        EditorCommand::File(FileCommand::Save),
        EditorCommand::Edit(EditCommand::Find),
        EditorCommand::Edit(EditCommand::CopyExternal),
        EditorCommand::Window(WindowCommand::SplitHorizontal),
        EditorCommand::Window(WindowCommand::FocusLeft),
    ]
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
                command: EditorCommand::App(AppCommand::Outline),
                description: "Outline sections for focused read-only pane",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::SearchResults),
                description: "List current search results",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::ReloadConfig),
                description: "Reload config",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::RunCommand),
                description: "Run command to read-only output",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputSummary),
                description: "Jump Command Output to summary",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputIndex),
                description: "Jump Command Output to index",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputStatus),
                description: "Jump Command Output to status",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputStdout),
                description: "Jump Command Output to stdout",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputStdoutBody),
                description: "Jump Command Output to first stdout line",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputStderr),
                description: "Jump Command Output to stderr",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputStderrBody),
                description: "Jump Command Output to first stderr line",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputTruncated),
                description: "Jump Command Output to truncation flag",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputNextMatch),
                description: "Find next match in Command Output",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputPreviousMatch),
                description: "Find previous match in Command Output",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputNextSection),
                description: "Jump Command Output to next section",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputPreviousSection),
                description: "Jump Command Output to previous section",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputOnlyStdout),
                description: "Open stdout-only Command Output view",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputOnlyStderr),
                description: "Open stderr-only Command Output view",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputCopy),
                description: "Copy Command Output internally",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputSave),
                description: "Save Command Output with dialog",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandOutputClear),
                description: "Clear Command Output",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::ShellEscape),
                description: "Open interactive shell and return",
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
                command: EditorCommand::File(FileCommand::SwitchBuffer),
                description: "Switch open buffer",
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
                command: EditorCommand::File(FileCommand::Reload),
                description: "Reload from disk",
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
                command: EditorCommand::Edit(EditCommand::MovePageUp),
                description: "Move page up",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MovePageDown),
                description: "Move page down",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveDocumentStart),
                description: "Move to document start",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveDocumentEnd),
                description: "Move to document end",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ScrollLeft),
                description: "Scroll view left",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ScrollRight),
                description: "Scroll view right",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveWordLeft),
                description: "Move word left",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveWordRight),
                description: "Move word right",
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
                command: EditorCommand::Edit(EditCommand::ExtendSelectionPageUp),
                description: "Extend selection page up",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ExtendSelectionPageDown),
                description: "Extend selection page down",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ExtendSelectionWordLeft),
                description: "Extend selection word left",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ExtendSelectionWordRight),
                description: "Extend selection word right",
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
                command: EditorCommand::Edit(EditCommand::DeleteWordBackward),
                description: "Delete word backward",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::DeleteWordForward),
                description: "Delete word forward",
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
                command: EditorCommand::Edit(EditCommand::Cut),
                description: "Cut selection to internal clipboard",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Copy),
                description: "Copy selection to internal clipboard",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::CopyExternal),
                description: "Copy selection through OSC 52 when enabled",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Paste),
                description: "Paste internal clipboard",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::SelectAll),
                description: "Select all",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::SelectLine),
                description: "Select current line",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::CopyLine),
                description: "Copy current line",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::DeleteLine),
                description: "Delete current line",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveLineUp),
                description: "Move current line up",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveLineDown),
                description: "Move current line down",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::IndentLine),
                description: "Indent selected/current lines",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::OutdentLine),
                description: "Outdent selected/current lines",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::TrimTrailingWhitespace),
                description: "Trim trailing whitespace",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ToggleWordWrap),
                description: "Toggle word wrap",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace),
                description: "Toggle visible whitespace",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ToggleBookmark),
                description: "Toggle bookmark on current line",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::NextBookmark),
                description: "Go to next bookmark",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::PreviousBookmark),
                description: "Go to previous bookmark",
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

fn outline_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}

fn search_results_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}

fn command_output_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}

fn command_output_empty_buffer() -> TextBuffer {
    command_output_buffer("Dun Command Output\n\n(empty)\n")
}

fn config_diagnostics_buffer(text: &str) -> TextBuffer {
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, text)
}

fn command_output_text(result: &CommandRunResult) -> String {
    let mut out = String::from("Dun Command Output\n\n");
    out.push_str(&format!("Command: {}\n", result.command));
    out.push_str(&format!("Shell: {}\n", result.shell.to_string_lossy()));
    out.push_str(&format!("Status: {}\n", exit_status_text(result.status)));
    out.push_str(&format!(
        "Elapsed: {}\n",
        duration_status_text(result.elapsed)
    ));
    out.push_str(&format!(
        "Limit: {} bytes per stream\n",
        COMMAND_OUTPUT_STREAM_SOFT_LIMIT_BYTES
    ));
    out.push_str(&format!(
        "Stdout: {}\n",
        command_stream_summary(&result.stdout)
    ));
    out.push_str(&format!(
        "Stdout Lines: {}\n",
        command_stream_line_count(&result.stdout)
    ));
    out.push_str(&format!(
        "Stderr: {}\n",
        command_stream_summary(&result.stderr)
    ));
    out.push_str(&format!(
        "Stderr Lines: {}\n",
        command_stream_line_count(&result.stderr)
    ));
    out.push_str("Sections: 4\n");
    out.push_str(&format!(
        "Truncated: {}\n",
        if result.stdout.truncated || result.stderr.truncated {
            "yes"
        } else {
            "no"
        }
    ));
    out.push_str(
        "\nIndex\n  output summary       metadata summary\n  output status        exit status line\n  output stdout        stdout section header\n  output stdout-body   first non-empty stdout line\n  output stderr        stderr section header\n  output stderr-body   first non-empty stderr line\n  output truncated     truncation flag\n",
    );

    out.push_str(&format!(
        "\n--- stdout ({}) ---\n",
        command_stream_summary(&result.stdout)
    ));
    push_decoded_command_stream(&mut out, &result.stdout);
    out.push_str(&format!(
        "\n--- stderr ({}) ---\n",
        command_stream_summary(&result.stderr)
    ));
    push_decoded_command_stream(&mut out, &result.stderr);
    out
}

fn command_stream_summary(stream: &CapturedCommandStream) -> String {
    format!(
        "{} bytes, {}",
        stream.bytes.len(),
        if stream.truncated {
            "truncated"
        } else {
            "complete"
        }
    )
}

fn command_stream_line_count(stream: &CapturedCommandStream) -> usize {
    if stream.bytes.is_empty() {
        return 0;
    }
    decode_file_text(stream.bytes.clone())
        .text
        .lines()
        .count()
        .max(1)
}

fn osc52_copy_sequence(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64_encode(text.as_bytes()))
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn push_decoded_command_stream(out: &mut String, stream: &CapturedCommandStream) {
    if stream.bytes.is_empty() {
        out.push_str("(empty)\n");
    } else {
        let decoded = decode_file_text(stream.bytes.clone());
        out.push_str(&decoded.text);
        if !decoded.text.ends_with('\n') {
            out.push('\n');
        }
    }
    if stream.truncated {
        out.push_str("[truncated]\n");
    }
}

fn command_output_summary_line(buffer: &TextBuffer) -> Option<usize> {
    (0..buffer.line_count()).find(|line_index| {
        buffer
            .line(*line_index)
            .is_some_and(|line| line.starts_with("Command: "))
    })
}

fn command_output_index_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_section_line(buffer, "Index")
}

fn command_output_status_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_section_line(buffer, "Status: ")
}

fn command_output_truncated_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_section_line(buffer, "Truncated: ")
}

fn command_output_stdout_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_section_line(buffer, "--- stdout")
}

fn command_output_stdout_body_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_body_line(buffer, command_output_stdout_line)
}

fn command_output_stderr_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_section_line(buffer, "--- stderr")
}

fn command_output_stderr_body_line(buffer: &TextBuffer) -> Option<usize> {
    command_output_body_line(buffer, command_output_stderr_line)
}

fn command_output_section_line(buffer: &TextBuffer, prefix: &str) -> Option<usize> {
    (0..buffer.line_count()).find(|line_index| {
        buffer
            .line(*line_index)
            .is_some_and(|line| line.starts_with(prefix))
    })
}

fn command_output_section_view_text(
    buffer: &TextBuffer,
    section: CommandOutputSection,
) -> Option<String> {
    let header = match section {
        CommandOutputSection::Stdout => command_output_stdout_line(buffer)?,
        CommandOutputSection::Stderr => command_output_stderr_line(buffer)?,
    };
    let end = ((header + 1)..buffer.line_count())
        .find(|line_index| {
            buffer
                .line(*line_index)
                .is_some_and(|line| line.starts_with("--- "))
        })
        .unwrap_or(buffer.line_count());
    let body_line_count = end.saturating_sub(header + 1);
    let mut out = format!("Dun Command Output {}\n\n", section.label());
    out.push_str(&format!("Section: {}\n", section.label()));
    out.push_str(&format!("Lines: {body_line_count}\n\n"));
    for line_index in header..buffer.line_count() {
        let line = buffer.line(line_index).unwrap_or_default();
        if line_index > header && line.starts_with("--- ") {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    Some(out)
}

fn command_output_relative_section_line(
    buffer: &TextBuffer,
    current_line: usize,
    direction: SearchDirection,
) -> Option<(usize, &'static str)> {
    let mut sections = Vec::new();
    if let Some(line) = command_output_summary_line(buffer) {
        sections.push((line, "summary"));
    }
    if let Some(line) = command_output_index_line(buffer) {
        sections.push((line, "index"));
    }
    if let Some(line) = command_output_stdout_line(buffer) {
        sections.push((line, "stdout"));
    }
    if let Some(line) = command_output_stderr_line(buffer) {
        sections.push((line, "stderr"));
    }
    sections.sort_by_key(|(line, _)| *line);
    sections.dedup_by_key(|(line, _)| *line);
    match direction {
        SearchDirection::Forward => sections
            .iter()
            .find(|(line, _)| *line > current_line)
            .or_else(|| sections.first())
            .copied(),
        SearchDirection::Backward => sections
            .iter()
            .rev()
            .find(|(line, _)| *line < current_line)
            .or_else(|| sections.last())
            .copied(),
    }
}

fn line_with_exact_text(buffer: &TextBuffer, text: &str) -> Option<usize> {
    (0..buffer.line_count()).find(|line_index| buffer.line(*line_index) == Some(text))
}

fn command_output_body_line(
    buffer: &TextBuffer,
    header_finder: fn(&TextBuffer) -> Option<usize>,
) -> Option<usize> {
    let header = header_finder(buffer)?;
    for line_index in header.saturating_add(1)..buffer.line_count() {
        let Some(line) = buffer.line(line_index) else {
            continue;
        };
        if line.starts_with("--- ") {
            return None;
        }
        let trimmed = line.trim();
        if !trimmed.is_empty() && trimmed != "(empty)" && trimmed != "[truncated]" {
            return Some(line_index);
        }
    }
    None
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

fn clamp_to_display_column(line: &str, target: usize) -> usize {
    let mut display = 0usize;
    for (index, ch) in line.char_indices() {
        let width = ch.width().unwrap_or(0);
        if display.saturating_add(width) > target {
            return index;
        }
        display = display.saturating_add(width);
    }
    line.len()
}

fn display_width_for_editor_char(ch: char) -> usize {
    ch.width().unwrap_or(0).max(1)
}

fn advance_wrapped_column(row: &mut usize, column: &mut usize, width: usize, body_width: usize) {
    let width = width.max(1);
    let body_width = body_width.max(1);
    if *column > 0 && (*column).saturating_add(width) > body_width {
        *row = (*row).saturating_add(1);
        *column = 0;
    }
    *column = (*column).saturating_add(width);
}

fn byte_column_for_wrapped_row_start(line: &str, target_row: usize, body_width: usize) -> usize {
    if target_row == 0 {
        return 0;
    }

    let mut row = 0usize;
    let mut column = 0usize;
    for (index, ch) in line.char_indices() {
        let width = display_width_for_editor_char(ch);
        if column > 0 && column.saturating_add(width) > body_width.max(1) {
            row = row.saturating_add(1);
            column = 0;
            if row == target_row {
                return index;
            }
        }
        column = column.saturating_add(width);
    }

    line.len()
}

fn byte_column_for_wrapped_row_column(
    line: &str,
    target_row: usize,
    target_column: usize,
    body_width: usize,
) -> usize {
    let body_width = body_width.max(1);
    let row_start = byte_column_for_wrapped_row_start(line, target_row, body_width);
    if target_column == 0 {
        return row_start;
    }

    let mut visual_column = 0usize;
    for (offset, ch) in line[row_start..].char_indices() {
        let index = row_start.saturating_add(offset);
        let width = display_width_for_editor_char(ch);
        if visual_column > 0 && visual_column.saturating_add(width) > body_width {
            return index;
        }
        if visual_column.saturating_add(width) > target_column {
            return index;
        }
        visual_column = visual_column.saturating_add(width);
    }

    line.len()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LoadedTextBuffer {
    buffer: TextBuffer,
    encoding: FileTextEncoding,
    snapshot: Option<FileReadSnapshot>,
}

fn load_text_buffer(path: &Path, limits: Limits) -> io::Result<LoadedTextBuffer> {
    let read = read_editable_file_with_snapshot(path, limits.editable_file_soft_limit_bytes)?;
    let decoded = decode_file_text(read.bytes);
    Ok(LoadedTextBuffer {
        buffer: TextBuffer::from_text_with_kind(decoded.encoding.buffer_kind(), &decoded.text),
        encoding: decoded.encoding,
        snapshot: Some(read.snapshot),
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

fn validate_save_snapshot(buffer: &BufferState, path: &Path) -> io::Result<()> {
    let Some(snapshot) = buffer.file_snapshot else {
        return Ok(());
    };

    match current_file_snapshot(path) {
        Ok(current) if current == snapshot => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file changed on disk; reload before saving or use Save As",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "file no longer exists; use Save As",
        )),
        Err(error) => Err(error),
    }
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct EditableFileRead {
    bytes: Vec<u8>,
    snapshot: FileReadSnapshot,
}

fn read_editable_file_with_snapshot(path: &Path, soft_limit: u64) -> io::Result<EditableFileRead> {
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

    Ok(EditableFileRead { bytes, snapshot })
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

fn current_file_snapshot(path: &Path) -> io::Result<FileReadSnapshot> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path is not a regular file",
        ));
    }

    Ok(FileReadSnapshot::from_metadata(&metadata))
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

fn reloaded_file_status(path: &Path, encoding: FileTextEncoding) -> String {
    match encoding {
        FileTextEncoding::Utf8 => format!("Reloaded {}", path.display()),
        FileTextEncoding::EscapedBytes => format!(
            "Reloaded {} read-only: non-UTF-8 bytes shown as escapes",
            path.display()
        ),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandRunResult {
    command: String,
    shell: OsString,
    status: ExitStatus,
    elapsed: Duration,
    stdout: CapturedCommandStream,
    stderr: CapturedCommandStream,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CapturedCommandStream {
    bytes: Vec<u8>,
    truncated: bool,
}

fn run_interactive_shell() -> io::Result<ExitStatus> {
    Command::new(shell_program()).status()
}

fn run_command_capture(command: &str, stream_limit: usize) -> io::Result<CommandRunResult> {
    let shell = shell_program();
    let started = Instant::now();
    let mut child = Command::new(&shell)
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("failed to capture command stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("failed to capture command stderr"))?;
    let stdout_reader = std::thread::spawn(move || read_capped_stream(stdout, stream_limit));
    let stderr_reader = std::thread::spawn(move || read_capped_stream(stderr, stream_limit));
    let status = child.wait()?;
    let elapsed = started.elapsed();
    let stdout = join_captured_stream(stdout_reader)?;
    let stderr = join_captured_stream(stderr_reader)?;

    Ok(CommandRunResult {
        command: command.to_string(),
        shell,
        status,
        elapsed,
        stdout,
        stderr,
    })
}

fn shell_program() -> OsString {
    env::var_os("SHELL")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| OsString::from("/bin/sh"))
}

fn read_capped_stream<R: Read>(mut reader: R, limit: usize) -> io::Result<CapturedCommandStream> {
    let mut bytes = Vec::new();
    let mut truncated = false;
    let mut chunk = [0u8; 8192];
    loop {
        let read = reader.read(&mut chunk)?;
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

fn command_run_status(result: &CommandRunResult) -> String {
    let mut status = format!(
        "Command returned {} in {}",
        exit_status_text(result.status),
        duration_status_text(result.elapsed)
    );
    if result.stdout.truncated || result.stderr.truncated {
        status.push_str("; output truncated");
    }
    status
}

fn exit_status_text(status: ExitStatus) -> String {
    status
        .code()
        .map(|code| format!("exit {code}"))
        .unwrap_or_else(|| "terminated".to_string())
}

fn duration_status_text(duration: Duration) -> String {
    if duration.as_secs() >= 1 {
        format!("{:.2}s", duration.as_secs_f64())
    } else {
        format!("{}ms", duration.as_millis())
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

    #[test]
    fn sgr_rewriter_converts_crossterm_ansi_palette_codes_to_legacy_codes() {
        let mut pending = Vec::new();
        let output =
            rewrite_16_color_sgr(b"\x1b[38;5;7;48;5;4mX\x1b[38;5;15;48;5;8mY", &mut pending);

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "\x1b[37;44mX\x1b[97;100mY"
        );
        assert!(pending.is_empty());
    }

    #[test]
    fn sgr_rewriter_preserves_split_sequences_until_complete() {
        let mut pending = Vec::new();

        assert_eq!(rewrite_16_color_sgr(b"\x1b[38;5", &mut pending), b"");
        assert_eq!(
            String::from_utf8(rewrite_16_color_sgr(b";11m!", &mut pending)).unwrap(),
            "\x1b[93m!"
        );
        assert!(pending.is_empty());
    }

    #[test]
    fn sgr_rewriter_leaves_non_sgr_csi_sequences_unchanged() {
        let mut pending = Vec::new();
        let output = rewrite_16_color_sgr(b"\x1b[?25l\x1b[2;3H", &mut pending);

        assert_eq!(String::from_utf8(output).unwrap(), "\x1b[?25l\x1b[2;3H");
        assert!(pending.is_empty());
    }

    fn left_click(column: u16, row: u16) -> CrosstermMouseEvent {
        CrosstermMouseEvent {
            kind: CrosstermMouseEventKind::Down(CrosstermMouseButton::Left),
            column,
            row,
            modifiers: CrosstermKeyModifiers::NONE,
        }
    }

    fn right_click(column: u16, row: u16) -> CrosstermMouseEvent {
        CrosstermMouseEvent {
            kind: CrosstermMouseEventKind::Down(CrosstermMouseButton::Right),
            column,
            row,
            modifiers: CrosstermKeyModifiers::NONE,
        }
    }

    fn left_drag(column: u16, row: u16) -> CrosstermMouseEvent {
        CrosstermMouseEvent {
            kind: CrosstermMouseEventKind::Drag(CrosstermMouseButton::Left),
            column,
            row,
            modifiers: CrosstermKeyModifiers::NONE,
        }
    }

    fn left_up(column: u16, row: u16) -> CrosstermMouseEvent {
        CrosstermMouseEvent {
            kind: CrosstermMouseEventKind::Up(CrosstermMouseButton::Left),
            column,
            row,
            modifiers: CrosstermKeyModifiers::NONE,
        }
    }

    fn scroll_down(column: u16, row: u16) -> CrosstermMouseEvent {
        CrosstermMouseEvent {
            kind: CrosstermMouseEventKind::ScrollDown,
            column,
            row,
            modifiers: CrosstermKeyModifiers::NONE,
        }
    }

    fn scroll_up(column: u16, row: u16) -> CrosstermMouseEvent {
        CrosstermMouseEvent {
            kind: CrosstermMouseEventKind::ScrollUp,
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
            parse_cli_args(["--dump-config"]).unwrap(),
            CliAction::DumpConfig
        );
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
            "options --help, --version, and --dump-config cannot be combined with paths"
        );
        assert_eq!(
            parse_cli_args(["--help", "--version"])
                .unwrap_err()
                .to_string(),
            "only one of --help, --version, or --dump-config may be used"
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
        assert!(cli_help_text().contains("--dump-config"));
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
    fn translates_common_modified_terminal_keys() {
        assert_eq!(
            key_stroke_from_crossterm(CrosstermKeyEvent::new(
                CrosstermKeyCode::Home,
                CrosstermKeyModifiers::CONTROL,
            )),
            Some(KeyStroke::new(Key::Home, KeyModifiers::CTRL))
        );
        assert_eq!(
            key_stroke_from_crossterm(CrosstermKeyEvent::new(
                CrosstermKeyCode::End,
                CrosstermKeyModifiers::CONTROL,
            )),
            Some(KeyStroke::new(Key::End, KeyModifiers::CTRL))
        );
        assert_eq!(
            key_stroke_from_crossterm(CrosstermKeyEvent::new(
                CrosstermKeyCode::F(3),
                CrosstermKeyModifiers::SHIFT,
            )),
            Some(KeyStroke::new(Key::F(3), KeyModifiers::SHIFT))
        );
        assert_eq!(
            key_stroke_from_crossterm(CrosstermKeyEvent::new(
                CrosstermKeyCode::Left,
                CrosstermKeyModifiers::SHIFT | CrosstermKeyModifiers::CONTROL,
            )),
            Some(KeyStroke::new(
                Key::Left,
                KeyModifiers {
                    shift: true,
                    ctrl: true,
                    alt: false,
                },
            ))
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
    fn mouse_wheel_scrolls_editor_body_when_enabled() {
        let text = (0..20)
            .map(|index| format!("line{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.buffer_state_mut(BufferId(1)).unwrap().buffer =
            TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
        app.sync_view_for_area(Rect::new(0, 0, 80, 8));

        handle_mouse_event(&mut app, scroll_down(10, 3));

        let buffer = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(buffer.first_line, EDITOR_MOUSE_WHEEL_LINES);
        assert_eq!(
            buffer.buffer.cursor_position(),
            Position::new(EDITOR_MOUSE_WHEEL_LINES, 0)
        );

        handle_mouse_event(&mut app, scroll_up(10, 3));
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().first_line, 0);
    }

    #[test]
    fn mouse_wheel_scrolls_wrapped_visual_rows() {
        let mut app = AppState::new();
        app.mouse_enabled = true;
        let state = app.buffer_state_mut(BufferId(1)).unwrap();
        state.buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdefghijklmnop");
        state.word_wrap = true;
        app.sync_view_for_area(Rect::new(0, 0, 12, 3));

        handle_mouse_event(&mut app, scroll_down(5, 2));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.first_line, 0);
        assert_eq!(state.first_visual_row, 1);
        assert_eq!(state.buffer.cursor_position(), Position::new(0, 8));
        assert!(
            scroll_status(state, app.focused_buffer_view_context(app.workspace_area))
                .contains("View V2-2/2 L1 wrap")
        );
    }

    #[test]
    fn mouse_scrollbar_click_and_drag_scrolls_editor_body_when_enabled() {
        let text = (0..20)
            .map(|index| format!("line{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.buffer_state_mut(BufferId(1)).unwrap().buffer =
            TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
        app.sync_view_for_area(Rect::new(0, 0, 80, 8));

        handle_mouse_event(&mut app, left_click(79, 5));

        assert_eq!(app.buffer_state(BufferId(1)).unwrap().first_line, 8);

        handle_mouse_event(&mut app, left_drag(79, 7));
        handle_mouse_event(&mut app, left_up(79, 7));

        assert_eq!(app.buffer_state(BufferId(1)).unwrap().first_line, 14);
        assert_eq!(app.mouse_drag, None);
    }

    #[test]
    fn mouse_menu_click_dispatches_command_when_enabled() {
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));
        assert_eq!(app.shell.menu_index_at_column(20), Some(3));

        handle_mouse_event(&mut app, left_click(20, 0));
        assert_eq!(app.active_menu, Some(3));
        handle_mouse_event(&mut app, left_click(20, 2));

        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::Help
        );
    }

    #[test]
    fn mouse_click_outside_open_menu_closes_it() {
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));

        handle_mouse_event(&mut app, left_click(20, 0));
        assert_eq!(app.active_menu, Some(3));
        handle_mouse_event(&mut app, left_click(0, 2));

        assert_eq!(app.active_menu, None);
        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::Edit
        );
    }

    #[test]
    fn escape_closes_active_menu_before_keymap_dispatch() {
        let mut app = AppState::new();
        app.active_menu = Some(0);
        app.active_menu_entry = Some(0);

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Esc, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.active_menu, None);
        assert_eq!(app.active_menu_entry, None);
        assert!(!app.should_quit);
    }

    #[test]
    fn alt_mnemonic_opens_menu_without_mouse_enabled() {
        let mut app = AppState::new();

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('h'), CrosstermKeyModifiers::ALT),
        );

        assert_eq!(app.active_menu, Some(3));
        assert_eq!(app.active_menu_entry, Some(0));
    }

    #[test]
    fn keyboard_menu_enter_dispatches_selected_entry() {
        let mut app = AppState::new();

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('h'), CrosstermKeyModifiers::ALT),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.active_menu, None);
        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::Help
        );
    }

    #[test]
    fn keyboard_menu_arrows_switch_menu_and_entry() {
        let mut app = AppState::new();
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('f'), CrosstermKeyModifiers::ALT),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Right, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(app.active_menu, Some(1));
        assert_eq!(app.active_menu_entry, Some(0));

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Right, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.status_message, Some("Split vertically".to_string()));
        assert_eq!(app.workspace.window_count(), 2);
    }

    #[test]
    fn mouse_drag_selects_text_in_editor_body() {
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));
        app.handle_text_input('a');
        app.handle_text_input('b');
        app.handle_text_input('c');
        app.handle_text_input('d');

        handle_mouse_event(&mut app, left_click(4, 2));
        handle_mouse_event(&mut app, left_drag(6, 2));
        handle_mouse_event(&mut app, left_up(6, 2));

        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .selection_range(),
            Some(TextRange::new(Position::new(0, 1), Position::new(0, 3)))
        );
        assert_eq!(app.mouse_drag, None);
    }

    #[test]
    fn mouse_drag_selection_scrolls_when_dragged_to_window_edge() {
        let text = (0..20)
            .map(|index| format!("line{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.buffer_state_mut(BufferId(1)).unwrap().buffer =
            TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
        app.sync_view_for_area(Rect::new(0, 0, 80, 8));

        handle_mouse_event(&mut app, left_click(4, 6));
        handle_mouse_event(&mut app, left_drag(4, 8));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert!(state.first_line > 0);
        assert!(
            state
                .buffer
                .selection_range()
                .is_some_and(|range| range.end.line >= state.first_line)
        );
    }

    #[test]
    fn mouse_drag_resizes_split_boundary() {
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));
        let left = app.workspace.focused;
        let right = app.workspace.split_focused(Axis::Horizontal).unwrap();

        handle_mouse_event(&mut app, left_click(40, 2));
        handle_mouse_event(&mut app, left_drag(60, 2));
        handle_mouse_event(&mut app, left_up(60, 2));

        let layouts = app.workspace.resolved_layout(Rect::new(0, 0, 80, 20));
        assert_eq!(
            layouts
                .iter()
                .find(|layout| layout.id == left)
                .unwrap()
                .rect,
            Rect::new(0, 0, 60, 20)
        );
        assert_eq!(
            layouts
                .iter()
                .find(|layout| layout.id == right)
                .unwrap()
                .rect,
            Rect::new(60, 0, 20, 20)
        );
        assert_eq!(app.mouse_drag, None);
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
    fn line_edit_commands_apply_to_focused_buffer() {
        let mut app = app_with_text("one  \ntwo\nthree   ");
        app.handle_command(&EditorCommand::Edit(EditCommand::MoveDown));

        app.handle_command(&EditorCommand::Edit(EditCommand::CopyLine));
        assert_eq!(app.kill_ring.as_deref(), Some("two\n"));

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineUp));
        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "two\none  \nthree   "
        );

        app.handle_command(&EditorCommand::Edit(EditCommand::IndentLine));
        app.handle_command(&EditorCommand::Edit(EditCommand::OutdentLine));
        app.handle_command(&EditorCommand::Edit(EditCommand::TrimTrailingWhitespace));
        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "two\none\nthree"
        );

        app.handle_command(&EditorCommand::Edit(EditCommand::DeleteLine));
        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "one\nthree"
        );
    }

    #[test]
    fn view_toggles_and_bookmarks_update_buffer_state() {
        let mut app = app_with_text("one\ntwo");

        app.handle_command(&EditorCommand::Edit(EditCommand::ToggleWordWrap));
        app.handle_command(&EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace));
        app.handle_command(&EditorCommand::Edit(EditCommand::ToggleBookmark));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert!(state.word_wrap);
        assert!(state.visible_whitespace);
        assert_eq!(state.bookmarks, vec![0]);
        assert!(app.focused_detail_status().contains("[Mark]"));

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveDown));
        app.handle_command(&EditorCommand::Edit(EditCommand::PreviousBookmark));
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .cursor_position(),
            Position::new(0, 0)
        );
    }

    #[test]
    fn editor_page_commands_move_cursor_by_visible_page() {
        let text = (0..20)
            .map(|index| format!("line{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut app = AppState::new();
        app.buffer_state_mut(BufferId(1)).unwrap().buffer =
            TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
        app.sync_view_for_area(Rect::new(0, 0, 80, 6));

        app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .cursor_position(),
            Position::new(3, 0)
        );
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().first_line, 0);

        app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .cursor_position(),
            Position::new(6, 0)
        );
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().first_line, 3);

        app.handle_command(&EditorCommand::Edit(EditCommand::MovePageUp));
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .cursor_position(),
            Position::new(3, 0)
        );
    }

    #[test]
    fn shift_page_commands_extend_selection_by_visible_page() {
        let text = (0..20)
            .map(|index| format!("line{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut app = AppState::new();
        app.buffer_state_mut(BufferId(1)).unwrap().buffer =
            TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
        app.sync_view_for_area(Rect::new(0, 0, 80, 6));

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::PageDown, CrosstermKeyModifiers::SHIFT),
        );

        let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
        assert_eq!(buffer.cursor_position(), Position::new(3, 0));
        assert_eq!(
            buffer.selection(),
            Some(dun_core::Selection::new(
                Position::zero(),
                Position::new(3, 0)
            ))
        );
    }

    #[test]
    fn wrapped_page_commands_move_cursor_by_visual_rows() {
        let mut app = app_with_text("abcdefghijklmnop");
        app.buffer_state_mut(BufferId(1)).unwrap().word_wrap = true;
        app.sync_view_for_area(Rect::new(0, 0, 12, 3));

        app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.cursor_position(), Position::new(0, 8));
        assert_eq!(state.first_line, 0);

        app.handle_command(&EditorCommand::Edit(EditCommand::MovePageUp));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.cursor_position(), Position::zero());
        assert_eq!(state.first_visual_row, 0);
    }

    #[test]
    fn wrapped_shift_page_commands_extend_selection_by_visual_rows() {
        let mut app = app_with_text("abcdefghijklmnop");
        app.buffer_state_mut(BufferId(1)).unwrap().word_wrap = true;
        app.sync_view_for_area(Rect::new(0, 0, 12, 3));

        app.handle_command(&EditorCommand::Edit(EditCommand::ExtendSelectionPageDown));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.cursor_position(), Position::new(0, 8));
        assert_eq!(
            state.buffer.selection(),
            Some(dun_core::Selection::new(
                Position::zero(),
                Position::new(0, 8)
            ))
        );
        assert_eq!(
            state
                .buffer
                .text_in_range(state.buffer.selection_range().unwrap())
                .unwrap(),
            "abcdefgh"
        );
    }

    #[test]
    fn wrapped_page_commands_preserve_visual_column_across_wide_chars() {
        let mut app = app_with_text("界abcdefghi");
        let state = app.buffer_state_mut(BufferId(1)).unwrap();
        state.word_wrap = true;
        state
            .buffer
            .set_cursor(Position::new(0, "界a".len()))
            .unwrap();
        app.sync_view_for_area(Rect::new(0, 0, 10, 3));

        app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));

        let state = app.buffer_state(BufferId(1)).unwrap();
        let cursor = state.buffer.cursor_position();
        assert_eq!(
            state.buffer.line(0).unwrap().get(..cursor.column),
            Some("界abcdefg")
        );
    }

    #[test]
    fn wrapped_page_commands_preserve_visual_column_across_tab_and_control() {
        let mut app = app_with_text("a\tbcdefgh\na\u{1}bcdefgh");
        let state = app.buffer_state_mut(BufferId(1)).unwrap();
        state.word_wrap = true;
        state
            .buffer
            .set_cursor(Position::new(0, "a\t".len()))
            .unwrap();
        app.sync_view_for_area(Rect::new(0, 0, 8, 3));

        app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));

        {
            let state = app.buffer_state(BufferId(1)).unwrap();
            let cursor = state.buffer.cursor_position();
            assert_eq!(
                state.buffer.line(0).unwrap().get(..cursor.column),
                Some("a\tbcde")
            );
        }

        app.buffer_state_mut(BufferId(1))
            .unwrap()
            .buffer
            .set_cursor(Position::new(1, "a\u{1}".len()))
            .unwrap();
        app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));

        let state = app.buffer_state(BufferId(1)).unwrap();
        let cursor = state.buffer.cursor_position();
        assert_eq!(
            state.buffer.line(1).unwrap().get(..cursor.column),
            Some("a\u{1}bcde")
        );
    }

    #[test]
    fn horizontal_scroll_keeps_cursor_visible_and_reports_offset() {
        let mut app = app_with_text("0123456789abcdef");
        app.sync_view_for_area(Rect::new(0, 0, 10, 4));

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
        app.sync_view_for_area(Rect::new(0, 0, 10, 4));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert!(state.first_column > 0);
        assert!(app.focused_detail_status().contains(" X"));

        let buffer_views = app.buffer_views();
        let frame =
            app.shell
                .frame_for_workspace(&app.workspace, app.workspace_area, &buffer_views);
        assert!(
            frame.windows[0]
                .body
                .first()
                .is_some_and(|line| line.as_plain_text().ends_with("bcdef"))
        );
    }

    #[test]
    fn horizontal_scroll_commands_move_viewport_without_moving_cursor() {
        let mut app = app_with_text("0123456789abcdef");
        app.sync_view_for_area(Rect::new(0, 0, 10, 4));

        app.handle_command(&EditorCommand::Edit(EditCommand::ScrollRight));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.first_column, 3);
        assert_eq!(state.buffer.cursor_position(), Position::zero());
        assert_eq!(
            app.status_message,
            Some("Scrolled right to column 4".to_string())
        );

        app.handle_command(&EditorCommand::Edit(EditCommand::ScrollLeft));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.first_column, 0);
        assert_eq!(
            app.status_message,
            Some("Scrolled left to column 1".to_string())
        );
    }

    #[test]
    fn undo_redo_commands_report_status() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::Edit(EditCommand::Undo));
        assert_eq!(app.status_message, Some("Nothing to undo".to_string()));

        app.handle_text_input('x');
        app.handle_command(&EditorCommand::Edit(EditCommand::Undo));
        assert_eq!(app.status_message, Some("Undo".to_string()));

        app.handle_command(&EditorCommand::Edit(EditCommand::Redo));
        assert_eq!(app.status_message, Some("Redo".to_string()));

        app.handle_command(&EditorCommand::Edit(EditCommand::Redo));
        assert_eq!(app.status_message, Some("Nothing to redo".to_string()));
    }

    #[test]
    fn word_edit_commands_apply_to_focused_buffer() {
        let mut app = AppState::new();
        app.buffer_state_mut(BufferId(1)).unwrap().buffer =
            TextBuffer::from_text_with_kind(BufferKind::Untitled, "alpha beta");

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveWordRight));
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .cursor_position(),
            Position::new(0, 6)
        );

        app.handle_command(&EditorCommand::Edit(EditCommand::ExtendSelectionWordRight));
        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.selection(),
            Some(dun_core::Selection::new(
                Position::new(0, 6),
                Position::new(0, 10)
            ))
        );

        app.handle_command(&EditorCommand::Edit(EditCommand::DeleteWordBackward));
        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "alpha "
        );

        app.handle_command(&EditorCommand::Edit(EditCommand::DeleteWordBackward));
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");
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
    fn buffer_switcher_focuses_selected_buffer() {
        let mut app = AppState::new();
        let first_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
        app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
        let second_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
        assert_ne!(first_buffer_id, second_buffer_id);

        app.handle_command(&EditorCommand::File(FileCommand::SwitchBuffer));
        assert!(
            app.active_overlay()
                .unwrap()
                .title
                .contains("Switch Buffer")
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(
            app.workspace.focused_window().unwrap().buffer_id,
            first_buffer_id
        );
        assert_eq!(app.status_message, Some("Switched to Untitled".to_string()));
    }

    #[test]
    fn buffer_switcher_overlay_reports_scroll_overflow() {
        let mut app = AppState::new();
        for _ in 0..13 {
            app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
        }

        app.handle_command(&EditorCommand::File(FileCommand::SwitchBuffer));
        let overlay = app.active_overlay().expect("buffer switcher overlay");
        assert_eq!(overlay.title, "Switch Buffer");
        assert!(overlay.list_has_more_above);
        assert!(!overlay.list_has_more_below);

        app.move_buffer_switcher_selection(-20);
        let overlay = app.active_overlay().expect("buffer switcher overlay");
        assert!(!overlay.list_has_more_above);
        assert!(overlay.list_has_more_below);
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
    fn outline_window_lists_and_jumps_read_only_sections() {
        let mut app = AppState::new();
        app.handle_command(&EditorCommand::App(AppCommand::Help));
        let help_buffer_id = app.workspace.focused_window().unwrap().buffer_id;

        submit_command_line(&mut app, "outline");

        let outline_window = app.workspace.focused_window().unwrap();
        assert_eq!(outline_window.kind, WindowKind::Outline);
        assert_eq!(outline_window.buffer_kind, BufferKind::ReadOnly);
        let text = app
            .buffer_state(outline_window.buffer_id)
            .unwrap()
            .buffer
            .to_text();
        assert!(text.contains("Dun Outline"));
        assert!(text.contains("App"));
        assert!(text.contains("Navigation"));

        submit_command_line(&mut app, "outline Navigation");

        let window = app.workspace.focused_window().unwrap();
        assert_eq!(window.buffer_id, help_buffer_id);
        let buffer = app.buffer_state(help_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("Navigation")
        );
    }

    #[test]
    fn outline_recognizes_common_text_config_and_source_sections() {
        let buffer = TextBuffer::from_text_with_kind(
            BufferKind::Untitled,
            "\
# Markdown Title
body
## Nested Heading
[service]
[[servers]]
pub struct Worker {
impl Worker {
pub async fn run_task() {
function deploy {
cleanup() {
",
        );

        let labels = outline_entries_for_buffer(&buffer)
            .into_iter()
            .map(|entry| entry.label)
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec![
                "# Markdown Title",
                "## Nested Heading",
                "[service]",
                "[[servers]]",
                "struct Worker",
                "impl Worker",
                "fn run_task",
                "function deploy",
                "cleanup()",
            ]
        );
    }

    #[test]
    fn outline_window_keyboard_selection_enters_section_and_close_returns_source() {
        let mut app = app_with_text("# First\nbody\n# Second\n");

        submit_command_line(&mut app, "outline");
        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::Outline
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('n'), CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('n'), CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let source = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(
            app.workspace.focused_window().unwrap().buffer_id,
            BufferId(1)
        );
        assert_eq!(source.buffer.cursor_position(), Position::new(2, 0));

        submit_command_line(&mut app, "outline");
        assert_eq!(
            app.workspace.focused_window().unwrap().kind,
            WindowKind::Outline
        );
        app.handle_command(&EditorCommand::Window(WindowCommand::Close));

        assert_eq!(
            app.workspace.focused_window().unwrap().buffer_id,
            BufferId(1)
        );
    }

    #[test]
    fn document_edge_commands_work_in_read_only_windows() {
        let mut app = AppState::new();
        app.sync_view_for_area(Rect::new(0, 0, 80, 6));
        app.handle_command(&EditorCommand::App(AppCommand::Help));
        let help_buffer_id = app.workspace.focused_window().unwrap().buffer_id;

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveDocumentEnd));

        let buffer = app.buffer_state(help_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.cursor_position(),
            buffer_end_position(&buffer.buffer)
        );
        assert!(buffer.first_line > 0);

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveDocumentStart));

        let buffer = app.buffer_state(help_buffer_id).unwrap();
        assert_eq!(buffer.buffer.cursor_position(), Position::zero());
        assert_eq!(buffer.first_line, 0);
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
        assert!(text.contains("Jump Command Output to status [app.command_output_status]"));
        assert!(
            text.contains("Jump Command Output to truncation flag [app.command_output_truncated]")
        );
        assert!(text.contains("(unbound)"));
        assert!(text.contains("Close focused window [window.close]"));
        assert!(text.contains("Toggle hidden files [file_dialog.toggle_hidden]"));
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
        assert!(text.contains("Summary\n"));
        assert!(text.contains("Paths\n"));
        assert!(text.contains("keymap:"));
        assert!(text.contains("active: disabled (--no-config)"));
        assert!(text.contains("theme:"));
        assert!(text.contains("mouse: disabled"));
        assert!(text.contains("defaults: dun --dump-config"));
        assert!(text.contains("osc52_max_bytes: 16384"));
        assert!(text.contains("bindings:"));
        assert!(text.contains("important_unbound: none"));
        assert!(text.contains("app.config_diagnostics"));
        assert!(text.contains("F6"));
        assert!(text.contains("File Dialog Keymap"));
        assert!(text.contains("file_dialog.toggle_hidden"));
        assert!(text.contains("Ctrl+H"));
        assert_eq!(app.status_message, Some("Config diagnostics".to_string()));

        app.handle_command(&EditorCommand::App(AppCommand::ConfigDiagnostics));

        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(app.workspace.focused, config_window_id);

        app.handle_command(&EditorCommand::Window(WindowCommand::Close));
        assert_eq!(app.workspace.window_count(), 1);
        assert!(app.buffer_state(config_buffer_id).is_none());
    }

    #[test]
    fn config_diagnostics_command_jumps_to_named_sections() {
        let mut app = AppState::new();

        submit_command_line(&mut app, "config keymap");

        let window = app.workspace.focused_window().unwrap();
        assert_eq!(window.kind, WindowKind::ConfigDiagnostics);
        let buffer = app.buffer_state(window.buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("Keymap")
        );
        assert_eq!(
            app.status_message,
            Some("Config diagnostics: keymap".to_string())
        );

        submit_command_line(&mut app, "diagnostics file-dialog-keymap");

        let window = app.workspace.focused_window().unwrap();
        let buffer = app.buffer_state(window.buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("File Dialog Keymap")
        );
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
    fn command_line_prompt_completes_output_commands() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        send_text(&mut app, "outp");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Command: output ".to_string())
        );

        send_text(&mut app, "stdout-b");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Command: output stdout-body".to_string())
        );
    }

    #[test]
    fn command_line_prompt_completes_config_sections_and_themes() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        send_text(&mut app, "config file-d");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Command: config file-dialog-keymap".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Esc, CrosstermKeyModifiers::NONE),
        );
        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        send_text(&mut app, "theme ms");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Command: theme msedit".to_string())
        );
    }

    #[test]
    fn command_line_prompt_lists_and_cycles_ambiguous_completions() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        send_text(&mut app, "re");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(app.prompt_status_text(), Some("Command: re".to_string()));
        assert!(app.status_message.as_deref().is_some_and(
            |status| status.contains("reload-config")
                && status.contains("replace")
                && status.contains("results")
        ));

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Command: reload-config ".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Command: reloadfile ".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::BackTab, CrosstermKeyModifiers::SHIFT),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Command: reload-config ".to_string())
        );
    }

    #[test]
    fn command_line_prompt_overlay_shows_ambiguous_completion_candidates() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        send_text(&mut app, "re");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );

        let overlay = app.active_overlay().expect("command prompt overlay");
        assert_eq!(overlay.title, "Command");
        assert!(overlay.lines.iter().any(|line| {
            line.contains("Command completion")
                && line.contains("reload-config")
                && line.contains("replace")
                && line.contains("results")
        }));
    }

    #[test]
    fn command_line_prompt_completes_path_arguments() {
        let directory = temp_file_path("command-line-complete");
        let nested = directory.join("nested");
        let file = nested.join("alpha file.txt");
        std::fs::create_dir(&directory).unwrap();
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(&file, "alpha").unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        send_text(&mut app, &format!("open {}/n", directory.display()));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Command: open {}/nested/", directory.display()))
        );
        send_text(&mut app, "a");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some(format!(
                "Command: open \"{}/nested/alpha file.txt\"",
                directory.display()
            ))
        );

        let _ = std::fs::remove_file(file);
        let _ = std::fs::remove_dir(nested);
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn shell_escape_command_requests_runtime_action() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::ShellEscape));

        assert_eq!(app.take_runtime_action(), Some(RuntimeAction::ShellEscape));
        assert_eq!(app.status_message, Some("Shell escape".to_string()));
    }

    #[test]
    fn run_command_prompt_opens_read_only_output_window() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
        assert_eq!(app.prompt_status_text(), Some("Run Command: ".to_string()));
        send_text(&mut app, "printf dun-run");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let window = app.workspace.focused_window().unwrap();
        assert_eq!(window.kind, WindowKind::CommandOutput);
        let buffer = app.buffer_state(window.buffer_id).unwrap();
        assert!(buffer.buffer.is_read_only());
        let text = buffer.buffer.to_text();
        assert!(text.contains("Command: printf dun-run"));
        assert!(text.contains("Stdout: 7 bytes, complete"));
        assert!(text.contains("Truncated: no"));
        assert!(text.contains("--- stdout (7 bytes, complete) ---\ndun-run\n"));
        assert!(
            app.status_message
                .as_deref()
                .is_some_and(|status| status.contains("Command returned exit 0"))
        );
    }

    #[test]
    fn run_command_history_navigates_separately_from_command_line_history() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
        send_text(&mut app, "printf first");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
        send_text(&mut app, "printf second");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
        send_text(&mut app, "draft");

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Run Command: printf second".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Run Command: printf first".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Run Command: printf second".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some("Run Command: draft".to_string())
        );
        assert!(app.command_history.is_empty());
        assert_eq!(
            app.run_command_history,
            vec!["printf first".to_string(), "printf second".to_string()]
        );
    }

    #[test]
    fn run_command_reuses_output_window_for_new_results() {
        let mut app = AppState::new();

        app.run_external_command_to_buffer("printf one");
        let first_window = app.workspace.focused_window().unwrap().clone();
        let window_count = app.workspace.windows.len();

        app.run_external_command_to_buffer("printf two");

        let second_window = app.workspace.focused_window().unwrap();
        assert_eq!(app.workspace.windows.len(), window_count);
        assert_eq!(second_window.id, first_window.id);
        assert_eq!(second_window.kind, WindowKind::CommandOutput);
        assert_eq!(second_window.buffer_kind, BufferKind::ReadOnly);
        assert!(!second_window.collapsed);
        let text = app
            .buffer_state(second_window.buffer_id)
            .unwrap()
            .buffer
            .to_text();
        assert!(text.contains("Command: printf two"));
        assert!(text.contains("two"));
        assert!(!text.contains("one"));
    }

    #[test]
    fn command_output_actions_copy_clear_jump_and_save_output() {
        let mut app = AppState::new();
        app.run_external_command_to_buffer("printf stdout; printf stderr >&2");

        app.handle_command(&EditorCommand::App(AppCommand::CommandOutputCopy));
        assert!(
            app.kill_ring
                .as_deref()
                .is_some_and(|text| text.contains("stdout") && text.contains("stderr"))
        );
        assert_eq!(
            app.status_message,
            Some("Copied Command Output".to_string())
        );

        app.handle_command(&EditorCommand::App(AppCommand::CommandOutputStderr));
        let window = app.workspace.focused_window().unwrap();
        let output_buffer_id = window.buffer_id;
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("--- stderr (6 bytes, complete) ---")
        );

        app.handle_command(&EditorCommand::App(AppCommand::CommandOutputStdout));
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("--- stdout (6 bytes, complete) ---")
        );

        app.handle_command(&EditorCommand::App(AppCommand::CommandOutputSummary));
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("Command: printf stdout; printf stderr >&2")
        );

        submit_command_line(&mut app, "output find stderr");
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert!(
            buffer
                .search_status()
                .is_some_and(|status| status == "Find 1/7")
        );

        submit_command_line(&mut app, "output index");
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("Index")
        );

        submit_command_line(&mut app, "output stdout-body");
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("stdout")
        );

        submit_command_line(&mut app, "output stderr-body");
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("stderr")
        );

        app.handle_command(&EditorCommand::App(AppCommand::CommandOutputStatus));
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert!(
            buffer
                .buffer
                .line(buffer.buffer.cursor_position().line)
                .is_some_and(|line| line.starts_with("Status: "))
        );

        submit_command_line(&mut app, "output truncated");
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("Truncated: no")
        );
        assert_eq!(
            app.status_message,
            Some("Command Output: truncated".to_string())
        );

        submit_command_line(&mut app, "output next-section");
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("Index")
        );
        assert_eq!(
            app.status_message,
            Some("Command Output: index".to_string())
        );

        submit_command_line(&mut app, "output next-section");
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("--- stdout (6 bytes, complete) ---")
        );
        assert_eq!(
            app.status_message,
            Some("Command Output: stdout".to_string())
        );

        submit_command_line(&mut app, "output previous-section");
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("Index")
        );
        assert_eq!(
            app.status_message,
            Some("Command Output: index".to_string())
        );

        submit_command_line(&mut app, "output only stdout");
        let view_window = app.workspace.focused_window().unwrap();
        assert_eq!(view_window.kind, WindowKind::CommandOutputView);
        let view_text = app
            .buffer_state(view_window.buffer_id)
            .unwrap()
            .buffer
            .to_text();
        assert!(view_text.contains("Dun Command Output stdout"));
        assert!(view_text.contains("stdout"));
        assert!(!view_text.contains("stderr"));

        submit_command_line(&mut app, "output only stderr");
        let view_window = app.workspace.focused_window().unwrap();
        assert_eq!(view_window.kind, WindowKind::CommandOutputView);
        let view_buffer_id = view_window.buffer_id;
        let view_text = app.buffer_state(view_buffer_id).unwrap().buffer.to_text();
        assert!(view_text.contains("Dun Command Output stderr"));
        assert!(view_text.contains("Section: stderr"));
        assert!(view_text.contains("Lines: "));
        assert!(view_text.contains("stderr"));
        assert!(!view_text.contains("stdout"));

        submit_command_line(&mut app, "output find stderr");
        let view_buffer = app.buffer_state(view_buffer_id).unwrap();
        assert!(view_buffer.search_status().is_some());

        let only_path = temp_file_path("command-output-only-save.txt");
        submit_command_line(
            &mut app,
            &format!("output save {}", only_path.to_string_lossy()),
        );
        let saved_only = std::fs::read_to_string(&only_path).unwrap();
        let _ = std::fs::remove_file(&only_path);
        assert!(saved_only.contains("Dun Command Output stderr"));
        assert!(saved_only.contains("stderr"));
        assert!(!saved_only.contains("stdout"));

        app.handle_command(&EditorCommand::Window(WindowCommand::Close));
        assert_eq!(
            app.workspace.focused_window().unwrap().buffer_id,
            output_buffer_id
        );

        let path = temp_file_path("command-output-save.txt");
        submit_command_line(&mut app, &format!("output save {}", path.to_string_lossy()));
        let saved = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(saved.contains("Command: printf stdout; printf stderr >&2"));
        assert!(saved.contains("stdout"));
        assert!(saved.contains("stderr"));

        app.handle_command(&EditorCommand::App(AppCommand::CommandOutputClear));
        let buffer = app
            .buffer_state(app.workspace.focused_window().unwrap().buffer_id)
            .unwrap();
        assert_eq!(buffer.buffer.to_text(), "Dun Command Output\n\n(empty)\n");
    }

    #[test]
    fn command_output_find_next_previous_use_output_search_cache() {
        let mut app = AppState::new();
        app.run_external_command_to_buffer("printf seed");
        let output_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
        app.buffer_state_mut(output_buffer_id).unwrap().buffer = command_output_buffer(
            "Dun Command Output\n\nCommand: generated\nShell: sh\nStatus: exit 0\nElapsed: 1ms\nLimit: 1 bytes per stream\nStdout: 2 bytes, complete\nStderr: 0 bytes, complete\nTruncated: no\n\nIndex\n  output next         next match\n\n--- stdout (2 bytes, complete) ---\nneedle\nother\nneedle\n--- stderr (0 bytes, complete) ---\n(empty)\n",
        );

        submit_command_line(&mut app, "output find needle");
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("needle")
        );
        assert!(
            buffer
                .search_status()
                .is_some_and(|status| status == "Find 1/2")
        );

        submit_command_line(&mut app, "output next");
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert_eq!(
            buffer.buffer.line(buffer.buffer.cursor_position().line),
            Some("needle")
        );
        assert!(
            buffer
                .search_status()
                .is_some_and(|status| status == "Find 2/2")
        );

        submit_command_line(&mut app, "output previous");
        let buffer = app.buffer_state(output_buffer_id).unwrap();
        assert!(
            buffer
                .search_status()
                .is_some_and(|status| status == "Find 1/2")
        );
    }

    #[test]
    fn command_output_save_dialog_writes_output() {
        let mut app = AppState::new();
        app.run_external_command_to_buffer("printf dialog-save");
        let path = temp_file_path("command-output-dialog-save.txt");
        let _ = std::fs::remove_file(&path);

        app.handle_command(&EditorCommand::App(AppCommand::CommandOutputSave));
        assert_eq!(
            app.file_dialog.as_ref().map(FileDialogState::status_text),
            Some("Save Output: command-output.txt".to_string())
        );
        app.file_dialog
            .as_mut()
            .unwrap()
            .input
            .set_text(path.to_string_lossy().to_string());
        app.submit_file_dialog();

        let saved = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(saved.contains("dialog-save"));
        assert!(app.file_dialog.is_none());
        assert!(
            app.status_message
                .as_deref()
                .is_some_and(|status| status.contains("Saved Command Output"))
        );
    }

    #[test]
    fn command_output_save_dialog_requires_second_enter_before_overwrite() {
        let mut app = AppState::new();
        app.run_external_command_to_buffer("printf replacement-output");
        let path = temp_file_path("command-output-overwrite.txt");
        std::fs::write(&path, "old output").unwrap();

        app.handle_command(&EditorCommand::App(AppCommand::CommandOutputSave));
        app.file_dialog
            .as_mut()
            .unwrap()
            .input
            .set_text(path.to_string_lossy().to_string());
        app.submit_file_dialog();

        assert!(app.file_dialog.is_some());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old output");
        assert!(
            app.file_dialog
                .as_ref()
                .and_then(|dialog| dialog.message.as_deref())
                .is_some_and(|message| message.contains("Replace existing file"))
        );

        app.submit_file_dialog();

        let saved = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(saved.contains("replacement-output"));
        assert!(app.file_dialog.is_none());
        assert!(
            app.status_message
                .as_deref()
                .is_some_and(|status| status.contains("Saved Command Output"))
        );
    }

    #[test]
    fn command_output_save_dialog_keeps_dialog_on_write_error() {
        let mut app = AppState::new();
        app.run_external_command_to_buffer("printf cannot-save");
        let path = temp_file_path("missing-command-output-parent").join("output.txt");

        app.handle_command(&EditorCommand::App(AppCommand::CommandOutputSave));
        app.file_dialog
            .as_mut()
            .unwrap()
            .input
            .set_text(path.to_string_lossy().to_string());
        app.submit_file_dialog();

        assert!(app.file_dialog.is_some());
        assert!(!path.exists());
        assert!(
            app.status_message
                .as_deref()
                .is_some_and(|status| status.contains("Command Output save failed"))
        );
        assert!(
            app.file_dialog
                .as_ref()
                .and_then(|dialog| dialog.message.as_deref())
                .is_some_and(|message| message.contains("Command Output save failed"))
        );
    }

    #[test]
    fn command_line_run_executes_quoted_command() {
        let mut app = AppState::new();

        submit_command_line(&mut app, "run \"printf quoted-run\"");

        let window = app.workspace.focused_window().unwrap();
        assert_eq!(window.kind, WindowKind::CommandOutput);
        assert!(
            app.buffer_state(window.buffer_id)
                .unwrap()
                .buffer
                .to_text()
                .contains("quoted-run")
        );
    }

    #[test]
    fn search_results_window_lists_and_jumps_matches() {
        let mut app = app_with_text("alpha\nbeta alpha\ngamma\n");

        submit_command_line(&mut app, "find alpha");
        submit_command_line(&mut app, "results");

        let results_window = app.workspace.focused_window().unwrap();
        assert_eq!(results_window.kind, WindowKind::SearchResults);
        assert_eq!(results_window.buffer_kind, BufferKind::ReadOnly);
        let text = app
            .buffer_state(results_window.buffer_id)
            .unwrap()
            .buffer
            .to_text();
        assert!(text.contains("Dun Search Results"));
        assert!(text.contains("Matches: 2"));
        assert!(text.contains("  2. L2:C6 beta alpha"));

        submit_command_line(&mut app, "results 2");

        let source = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(source.buffer.cursor_position(), Position::new(1, 10));
        assert_eq!(
            source.buffer.selection_range(),
            Some(TextRange::new(Position::new(1, 5), Position::new(1, 10)))
        );
        assert!(
            source
                .search_status()
                .is_some_and(|status| status == "Find 2/2")
        );
    }

    #[test]
    fn search_results_window_keyboard_selection_enters_match() {
        let mut app = app_with_text("alpha\nbeta alpha\ngamma\n");

        submit_command_line(&mut app, "find alpha");
        submit_command_line(&mut app, "results");

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('n'), CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.status_message,
            Some("Search Results: selected 1/2".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('n'), CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.status_message,
            Some("Search Results: selected 2/2".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let window = app.workspace.focused_window().unwrap();
        assert_eq!(window.buffer_id, BufferId(1));
        let source = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(source.buffer.cursor_position(), Position::new(1, 10));
        assert_eq!(
            source.buffer.selection_range(),
            Some(TextRange::new(Position::new(1, 5), Position::new(1, 10)))
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
    fn command_line_prompt_cursor_edits_middle_of_input() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        send_text(&mut app, "ac");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::NONE),
        );
        send_text(&mut app, "b");
        assert_eq!(app.prompt_status_text(), Some("Command: abc".to_string()));

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Home, CrosstermKeyModifiers::NONE),
        );
        send_text(&mut app, ">");
        assert_eq!(app.prompt_status_text(), Some("Command: >abc".to_string()));

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::End, CrosstermKeyModifiers::NONE),
        );
        send_text(&mut app, "<");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Backspace, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(app.prompt_status_text(), Some("Command: >ab<".to_string()));
    }

    #[test]
    fn command_line_prompt_cursor_respects_utf8_boundaries() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        send_text(&mut app, "中b");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Backspace, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(app.prompt_status_text(), Some("Command: b".to_string()));

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Delete, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(app.prompt_status_text(), Some("Command: ".to_string()));
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
    fn shift_arrow_keys_extend_selection_in_editor_buffer() {
        let mut app = app_with_text("abcd");
        app.buffers[0]
            .buffer
            .set_cursor(Position::new(0, 1))
            .unwrap();

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Right, CrosstermKeyModifiers::SHIFT),
        );
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .selection_range(),
            Some(TextRange::new(Position::new(0, 1), Position::new(0, 2)))
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Right, CrosstermKeyModifiers::SHIFT),
        );
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .selection_range(),
            Some(TextRange::new(Position::new(0, 1), Position::new(0, 3)))
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::SHIFT),
        );
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .selection_range(),
            Some(TextRange::new(Position::new(0, 1), Position::new(0, 2)))
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::NONE),
        );
        let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
        assert_eq!(buffer.selection_range(), None);
        assert_eq!(buffer.cursor_position(), Position::new(0, 1));
    }

    #[test]
    fn shift_home_end_extend_selection_to_line_edges() {
        let mut app = app_with_text("abc\ndef");
        app.buffers[0]
            .buffer
            .set_cursor(Position::new(1, 1))
            .unwrap();

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::End, CrosstermKeyModifiers::SHIFT),
        );
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .selection_range(),
            Some(TextRange::new(Position::new(1, 1), Position::new(1, 3)))
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Home, CrosstermKeyModifiers::SHIFT),
        );
        let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
        assert_eq!(
            buffer.selection_range(),
            Some(TextRange::new(Position::new(1, 0), Position::new(1, 1)))
        );
        assert_eq!(buffer.cursor_position(), Position::new(1, 0));
    }

    #[test]
    fn select_line_command_selects_current_line() {
        let mut app = app_with_text("first\nsecond\nthird");
        app.buffers[0]
            .buffer
            .set_cursor(Position::new(1, 2))
            .unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::SelectLine));

        let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
        assert_eq!(
            buffer.selection_range(),
            Some(TextRange::new(Position::new(1, 0), Position::new(2, 0)))
        );
        assert_eq!(
            buffer
                .text_in_range(buffer.selection_range().unwrap())
                .unwrap(),
            "second\n"
        );
    }

    #[test]
    fn configured_shift_arrow_binding_wins_before_selection_fallback() {
        let config = parse_config("key.window.split_horizontal = Shift+Right").unwrap();
        let mut app = AppState::from_config(config);
        app.sync_view_for_area(Rect::new(0, 0, 80, 20));
        app.handle_text_input('a');

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Right, CrosstermKeyModifiers::SHIFT),
        );

        assert_eq!(app.workspace.window_count(), 2);
        assert_eq!(app.status_message, Some("Split horizontally".to_string()));
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .selection_range(),
            None
        );
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
        assert_eq!(app.focused_buffer_status(), "[Escaped Bytes]");
        assert_eq!(app.focused_file_status(), bracket(&title_for_path(&path)));
        assert!(app.focused_detail_status().contains("[Escaped Bytes]"));
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
        assert!(app.file_dialog.is_some());
        assert!(
            app.active_overlay()
                .unwrap()
                .lines
                .iter()
                .any(|line| line.contains("Open failed:"))
        );
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");
    }

    #[test]
    fn open_dialog_reuses_recent_successful_directory() {
        let directory = temp_file_path("open-dialog-recent");
        let path = directory.join("recent.txt");
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(&path, "opened").unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );
        app.handle_command(&EditorCommand::File(FileCommand::Open));

        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Open: {}/", directory.display()))
        );

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn open_command_reports_directory_path() {
        let path = temp_file_path("open-dir");
        std::fs::create_dir(&path).unwrap();
        let mut app = AppState::new();

        app.run_open_command(&[path.to_string_lossy().into_owned()]);

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
    fn open_dialog_enters_directory_path() {
        let path = temp_file_path("open-dialog-dir");
        std::fs::create_dir(&path).unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Open: {}/", path.display()))
        );
        assert_eq!(app.status_message, None);
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");

        let _ = std::fs::remove_dir(path);
    }

    #[test]
    fn open_dialog_tab_completes_unique_file_path() {
        let directory = temp_file_path("open-dialog-tab");
        let path = directory.join("alpha.log");
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(&path, "opened").unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/al", directory.display()));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Open: {}", path.display()))
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "opened");
        assert_eq!(state.path.as_ref(), Some(&path));

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn file_dialog_path_input_cursor_edits_middle_of_path() {
        let directory = temp_file_path("file-dialog-cursor");
        std::fs::create_dir(&directory).unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/ab", directory.display()));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::NONE),
        );
        send_text(&mut app, "X");
        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Open: {}/aXb", directory.display()))
        );
        assert_eq!(
            app.file_dialog
                .as_ref()
                .map(|dialog| dialog.input.cursor_index),
            Some(format!("{}/aX", directory.display()).len())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Backspace, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Open: {}/ab", directory.display()))
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Delete, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Open: {}/a", directory.display()))
        );

        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn file_dialog_path_input_home_end_and_utf8_cursor_are_safe() {
        let directory = temp_file_path("file-dialog-home-end");
        std::fs::create_dir(&directory).unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
        send_text(&mut app, &format!("{}/中b", directory.display()));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Backspace, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Save As: {}/b", directory.display()))
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Home, CrosstermKeyModifiers::NONE),
        );
        send_text(&mut app, "~");
        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Save As: ~{}/b", directory.display()))
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::End, CrosstermKeyModifiers::NONE),
        );
        send_text(&mut app, "!");
        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Save As: ~{}/b!", directory.display()))
        );

        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn open_dialog_down_enter_opens_selected_file() {
        let directory = temp_file_path("open-dialog-select");
        let first = directory.join("a.txt");
        let second = directory.join("b.txt");
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(&first, "first").unwrap();
        std::fs::write(&second, "second").unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/", directory.display()));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "second");
        assert_eq!(state.path.as_ref(), Some(&second));

        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn open_dialog_page_keys_move_selection_and_scroll() {
        let directory = temp_file_path("open-dialog-page");
        std::fs::create_dir(&directory).unwrap();
        for index in 0..20 {
            std::fs::write(
                directory.join(format!("item{index:02}.txt")),
                format!("item{index:02}"),
            )
            .unwrap();
        }
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/", directory.display()));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::PageDown, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.file_dialog
                .as_ref()
                .and_then(|dialog| dialog.selected_index),
            Some(11)
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::PageDown, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.file_dialog
                .as_ref()
                .and_then(|dialog| dialog.selected_index),
            Some(20)
        );
        assert_eq!(
            app.file_dialog.as_ref().map(|dialog| dialog.scroll_offset),
            Some(9)
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::PageUp, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.file_dialog
                .as_ref()
                .and_then(|dialog| dialog.selected_index),
            Some(9)
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::PageUp, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.file_dialog
                .as_ref()
                .and_then(|dialog| dialog.selected_index),
            Some(0)
        );
        assert_eq!(
            app.file_dialog.as_ref().map(|dialog| dialog.scroll_offset),
            Some(0)
        );

        for index in 0..20 {
            let _ = std::fs::remove_file(directory.join(format!("item{index:02}.txt")));
        }
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn file_dialog_parent_entry_is_first_and_enters_parent_directory() {
        let directory = temp_file_path("file-dialog-parent");
        let child = directory.join("child");
        std::fs::create_dir(&directory).unwrap();
        std::fs::create_dir(&child).unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/", child.display()));

        let dialog = app.file_dialog.as_ref().unwrap();
        assert_eq!(
            dialog.entries.first().map(|entry| entry.name.as_str()),
            Some("..")
        );
        assert_eq!(
            dialog.entries.first().map(|entry| entry.is_parent),
            Some(true)
        );

        app.click_file_dialog_visible_index(0);

        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Open: {}/", directory.display()))
        );
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");

        let _ = std::fs::remove_dir(child);
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn file_dialog_hides_dotfiles_until_prefix_or_toggle() {
        let directory = temp_file_path("file-dialog-hidden");
        let hidden = directory.join(".secret");
        let visible = directory.join("visible.txt");
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(&hidden, "hidden").unwrap();
        std::fs::write(&visible, "visible").unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/", directory.display()));

        let entry_names = app
            .file_dialog
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        assert!(entry_names.contains(&".."));
        assert!(entry_names.contains(&"visible.txt"));
        assert!(!entry_names.contains(&".secret"));

        send_text(&mut app, ".");
        let entry_names = app
            .file_dialog
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        assert!(entry_names.contains(&".secret"));

        let mut app = AppState::new();
        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/", directory.display()));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('h'), CrosstermKeyModifiers::CONTROL),
        );

        let dialog = app.file_dialog.as_ref().unwrap();
        let entry_names = dialog
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        assert!(dialog.show_hidden);
        assert!(entry_names.contains(&".secret"));
        assert_eq!(dialog.message.as_deref(), Some("Hidden files shown"));

        let _ = std::fs::remove_file(hidden);
        let _ = std::fs::remove_file(visible);
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn file_dialog_overlay_exposes_msedit_like_dialog_fields() {
        let directory = temp_file_path("file-dialog-overlay");
        let file = directory.join("alpha.log");
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(&file, "alpha").unwrap();
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/", directory.display()));

        let overlay = app.active_overlay().expect("file dialog overlay");
        assert_eq!(overlay.title, "Open");
        assert_eq!(
            overlay.input.as_deref(),
            Some(format!("{}/", directory.display()).as_str())
        );
        assert!(
            overlay
                .lines
                .iter()
                .any(|line| line.starts_with("Look in: "))
        );
        assert!(overlay.lines.iter().any(|line| line == "File name:"));
        assert!(
            overlay
                .lines
                .iter()
                .any(|line| line.starts_with("Hidden: "))
        );
        assert!(
            overlay
                .list
                .iter()
                .any(|line| line == "[..] Parent directory")
        );
        assert!(overlay.list.iter().any(|line| line.contains("alpha.log")));
        assert!(
            overlay
                .buttons
                .iter()
                .any(|line| line.contains("[Enter] OK"))
        );
        assert_eq!(overlay.min_width, 60);

        let _ = std::fs::remove_file(file);
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn file_dialog_uses_configured_modal_keybindings() {
        let mut config = Config::default();
        config.file_dialog_keys.set_action_binding(
            FileDialogAction::ToggleHidden,
            Some(KeyStroke::plain(Key::F(8))),
        );
        let mut app = AppState::from_config(config);

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('h'), CrosstermKeyModifiers::CONTROL),
        );
        assert!(!app.file_dialog.as_ref().unwrap().show_hidden);

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::F(8), CrosstermKeyModifiers::NONE),
        );
        assert!(app.file_dialog.as_ref().unwrap().show_hidden);

        let help = help_text(&app.shell.keymap, &app.file_dialog_keys);
        assert!(help.contains("F8"));
        assert!(help.contains("file_dialog.toggle_hidden"));
    }

    #[test]
    fn mouse_wheel_scroll_changes_file_dialog_click_target() {
        let directory = temp_file_path("open-dialog-wheel");
        std::fs::create_dir(&directory).unwrap();
        for index in 0..14 {
            std::fs::write(
                directory.join(format!("item{index:02}.txt")),
                format!("item{index:02}"),
            )
            .unwrap();
        }
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.sync_view_for_area(Rect::new(0, 0, 90, 14));

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/", directory.display()));
        handle_mouse_event(&mut app, scroll_down(20, 8));
        assert_eq!(
            app.file_dialog.as_ref().map(|dialog| dialog.scroll_offset),
            Some(1)
        );
        let (x, y) = file_dialog_list_point(&app, 0);
        handle_mouse_event(&mut app, left_click(x, y));

        let path = directory.join("item00.txt");
        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "item00");
        assert_eq!(state.path.as_ref(), Some(&path));

        for index in 0..14 {
            let _ = std::fs::remove_file(directory.join(format!("item{index:02}.txt")));
        }
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn file_dialog_overlay_reports_scroll_overflow() {
        let directory = temp_file_path("open-dialog-overflow");
        std::fs::create_dir(&directory).unwrap();
        for index in 0..14 {
            std::fs::write(directory.join(format!("item{index:02}.txt")), "x").unwrap();
        }
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/", directory.display()));
        let overlay = app.active_overlay().expect("file dialog overlay");
        assert!(!overlay.list_has_more_above);
        assert!(overlay.list_has_more_below);

        app.scroll_file_dialog(2);
        let overlay = app.active_overlay().expect("file dialog overlay");
        assert!(overlay.list_has_more_above);
        assert!(overlay.list_has_more_below);

        for index in 0..14 {
            let _ = std::fs::remove_file(directory.join(format!("item{index:02}.txt")));
        }
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn mouse_click_open_dialog_file_opens_selected_file() {
        let directory = temp_file_path("open-dialog-mouse-file");
        let first = directory.join("a.txt");
        let second = directory.join("b.txt");
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(&first, "first").unwrap();
        std::fs::write(&second, "second").unwrap();
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.sync_view_for_area(Rect::new(0, 0, 90, 14));

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/", directory.display()));
        let (x, y) = file_dialog_list_point(&app, 2);
        handle_mouse_event(&mut app, left_click(x, y));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "second");
        assert_eq!(state.path.as_ref(), Some(&second));
        assert_eq!(app.prompt_status_text(), None);

        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn mouse_click_open_dialog_directory_enters_directory() {
        let directory = temp_file_path("open-dialog-mouse-dir");
        let child = directory.join("child");
        std::fs::create_dir(&directory).unwrap();
        std::fs::create_dir(&child).unwrap();
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.sync_view_for_area(Rect::new(0, 0, 90, 14));

        app.handle_command(&EditorCommand::File(FileCommand::Open));
        send_text(&mut app, &format!("{}/", directory.display()));
        let (x, y) = file_dialog_list_point(&app, 1);
        handle_mouse_event(&mut app, left_click(x, y));

        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Open: {}/", child.display()))
        );
        assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");

        let _ = std::fs::remove_dir(child);
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn mouse_click_save_as_dialog_directory_updates_input_without_saving() {
        let directory = temp_file_path("save-as-dialog-mouse");
        let child = directory.join("child");
        std::fs::create_dir(&directory).unwrap();
        std::fs::create_dir(&child).unwrap();
        let mut app = AppState::new();
        app.mouse_enabled = true;
        app.sync_view_for_area(Rect::new(0, 0, 90, 14));
        app.handle_text_input('x');

        app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
        send_text(&mut app, &format!("{}/", directory.display()));
        let (x, y) = file_dialog_list_point(&app, 1);
        handle_mouse_event(&mut app, left_click(x, y));

        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Save As: {}/", child.display()))
        );
        let state = app.buffer_state(BufferId(1)).unwrap();
        assert!(state.buffer.is_dirty());
        assert_eq!(state.path, None);

        let _ = std::fs::remove_dir(child);
        let _ = std::fs::remove_dir(directory);
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
    fn save_refuses_external_file_change() {
        let path = temp_file_path("save-external-change.txt");
        std::fs::write(&path, "old").unwrap();
        let mut app = AppState::from_path(Some(path.clone())).unwrap();
        std::fs::write(&path, "external change").unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
        app.handle_text_input('!');
        app.handle_command(&EditorCommand::File(FileCommand::Save));

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "external change");
        assert!(
            app.status_message
                .as_deref()
                .is_some_and(|status| status.contains("file changed on disk"))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reload_command_refreshes_focused_file_buffer() {
        let path = temp_file_path("reload-file.txt");
        std::fs::write(&path, "old").unwrap();
        let mut app = AppState::from_path(Some(path.clone())).unwrap();
        std::fs::write(&path, "new").unwrap();

        app.handle_command(&EditorCommand::File(FileCommand::Reload));

        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "new"
        );
        assert_eq!(
            app.status_message,
            Some(format!("Reloaded {}", path.display()))
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
        make_destination_at_least_as_new_as(&path, &stale_temp, "old");
        app.buffer_state_mut(BufferId(1)).unwrap().file_snapshot =
            current_file_snapshot(&path).ok();

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
    fn save_as_dialog_requires_second_enter_before_overwrite() {
        let path = temp_file_path("save-as-overwrite.txt");
        std::fs::write(&path, "old").unwrap();
        let mut app = AppState::new();
        app.handle_text_input('x');

        app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
        send_text(&mut app, &path.to_string_lossy());
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old");
        assert!(app.file_dialog.is_some());
        assert!(
            app.active_overlay()
                .unwrap()
                .lines
                .iter()
                .any(|line| line.contains("Replace existing file"))
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().path.as_ref(),
            Some(&path)
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_as_dialog_tab_completes_directory_before_save() {
        let parent = temp_file_path("save-as-dialog-tab");
        let directory = parent.join("nested");
        let path = directory.join("out.txt");
        std::fs::create_dir(&parent).unwrap();
        std::fs::create_dir(&directory).unwrap();
        let mut app = AppState::new();
        app.handle_text_input('x');

        app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
        send_text(&mut app, &format!("{}/nes", parent.display()));
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.prompt_status_text(),
            Some(format!("Save As: {}/", directory.display()))
        );

        send_text(&mut app, "out.txt");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
        assert_eq!(state.path.as_ref(), Some(&path));
        assert!(!state.buffer.is_dirty());

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(directory);
        let _ = std::fs::remove_dir(parent);
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
    fn find_prompt_previews_matches_and_cancel_restores_cursor() {
        let mut app = app_with_text("zero one two one");
        app.buffers[0]
            .buffer
            .set_cursor(Position::new(0, 2))
            .unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::Find));
        send_text(&mut app, "one");

        assert_eq!(app.status_message, Some("Find: 1/2 one".to_string()));
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .selection_range(),
            Some(TextRange::new(Position::new(0, 5), Position::new(0, 8)))
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Esc, CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.cursor_position(), Position::new(0, 2));
        assert_eq!(state.buffer.selection_range(), None);
        assert_eq!(app.status_message, Some("Find cancelled".to_string()));
    }

    #[test]
    fn find_prompt_supports_ignore_case_and_whole_word_flags() {
        let mut app = app_with_text("ERROR errors error_error error");

        app.handle_command(&EditorCommand::Edit(EditCommand::Find));
        send_text(&mut app, "/iw error");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(
            app.status_message,
            Some("Find: 1/2 error (ignore-case, whole-word)".to_string())
        );
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .selection_range(),
            Some(TextRange::new(Position::new(0, 0), Position::new(0, 5)))
        );

        app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));

        assert_eq!(
            app.status_message,
            Some("Find: 2/2 error (ignore-case, whole-word)".to_string())
        );
        assert_eq!(
            app.buffer_state(BufferId(1))
                .unwrap()
                .buffer
                .selection_range(),
            Some(TextRange::new(Position::new(0, 25), Position::new(0, 30)))
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
        assert_eq!(
            app.confirm_status_text(),
            Some("Match 1/2; replaced 0, skipped 0".to_string())
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('r'), CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "uno two one");
        assert!(state.buffer.is_dirty());
        assert_eq!(app.last_find_query, Some("one".to_string()));
        assert_eq!(
            app.status_message,
            Some("Replace confirm: 1/1 one -> uno".to_string())
        );
        assert_eq!(
            app.confirm_status_text(),
            Some("Match 1/1; replaced 1, skipped 0".to_string())
        );
        assert_eq!(
            state.buffer.selection_range(),
            Some(TextRange::new(Position::new(0, 8), Position::new(0, 11)))
        );
    }

    #[test]
    fn replace_confirmation_can_skip_and_replace_next_match() {
        let mut app = app_with_text("one two one");

        app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
        send_text(&mut app, "one");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );
        send_text(&mut app, "uno");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        assert_eq!(
            app.confirm_status_text(),
            Some("Match 1/2; replaced 0, skipped 0".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('s'), CrosstermKeyModifiers::NONE),
        );
        assert_eq!(
            app.confirm_status_text(),
            Some("Match 2/2; replaced 0, skipped 1".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('r'), CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "one two uno");
        assert_eq!(
            app.status_message,
            Some("Replace confirm: 1/1 one -> uno".to_string())
        );
        assert_eq!(
            app.confirm_status_text(),
            Some("Match 1/1; replaced 1, skipped 1".to_string())
        );
        assert_eq!(
            state.buffer.selection_range(),
            Some(TextRange::new(Position::new(0, 0), Position::new(0, 3)))
        );
    }

    #[test]
    fn replace_confirmation_all_replaces_remaining_matches() {
        let mut app = app_with_text("one two one");

        app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
        send_text(&mut app, "one");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );
        send_text(&mut app, "uno");
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('a'), CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "uno two uno");
        assert_eq!(app.replace_confirm, None);
        assert_eq!(
            app.status_message,
            Some("Replace All: 2 one -> uno".to_string())
        );
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
        assert_eq!(
            app.confirm_status_text(),
            Some("Match 2/2; replaced 0, skipped 0".to_string())
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('r'), CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "one two uno");
        assert_eq!(
            app.status_message,
            Some("Replace confirm: 1/1 one -> uno".to_string())
        );
        assert_eq!(
            app.confirm_status_text(),
            Some("Match 1/1; replaced 1, skipped 0".to_string())
        );
        assert_eq!(
            state.buffer.selection_range(),
            Some(TextRange::new(Position::new(0, 0), Position::new(0, 3)))
        );
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
        assert_eq!(
            app.confirm_status_text(),
            Some("Match 1/1; replaced 0, skipped 0".to_string())
        );
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Char('r'), CrosstermKeyModifiers::NONE),
        );

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), " two");
        assert_eq!(
            app.status_message,
            Some("Replace done: 1 replaced, 0 skipped".to_string())
        );
    }

    #[test]
    fn command_line_replace_all_is_single_undo_step() {
        let mut app = app_with_text("one two one");

        app.run_command_line("replace all one uno");

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "uno two uno");
        assert_eq!(
            app.status_message,
            Some("Replace All: 2 one -> uno".to_string())
        );

        app.handle_command(&EditorCommand::Edit(EditCommand::Undo));
        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "one two one"
        );
    }

    #[test]
    fn command_line_replace_all_honors_search_flags() {
        let mut app = app_with_text("ERROR errors error_error error");

        app.run_command_line("replace all \"/iw error\" ok");

        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "ok errors error_error ok"
        );
        assert_eq!(
            app.status_message,
            Some("Replace All: 2 error (ignore-case, whole-word) -> ok".to_string())
        );
    }

    #[test]
    fn find_populates_status_field_and_view_highlights() {
        let mut app = app_with_text("one two one");
        app.workspace_area = Rect::new(0, 0, 80, 8);

        app.last_find_query = Some("one".to_string());
        app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));
        app.sync_view_for_area(app.workspace_area);

        assert!(app.focused_detail_status().contains("[Find 1/2]"));
        let buffer_views = app.buffer_views();
        assert_eq!(buffer_views[0].search_matches.len(), 2);
        assert_eq!(buffer_views[0].active_search_match, Some(0));
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

        assert_eq!(app.focused_buffer_status(), "[Plain Text*]");
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
            "[LF] [UTF-8] [Spaces:4] 2:3 [View 1-1/2] [ASCII/mono] [Win 1/1]"
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
            "[LF] [UTF-8] [Spaces:4] 1:1 [View 1-1/1] [UTF-8/16c] [Win 2/2]"
        );

        app.workspace.focused = WindowId(1);

        assert_eq!(
            app.focused_detail_status(),
            "[CRLF] [UTF-8] [Spaces:4] 1:1 [View 1-1/2] [UTF-8/16c] [Win 1/2]"
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
            "[LF] [UTF-8] [Spaces:4] 1:4 [Sel 3c] [View 1-1/1] [UTF-8/256c] [Win 1/1]"
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
    fn copy_selection_pastes_internal_clipboard_without_mutating_source() {
        let mut app = app_with_text("abc def");
        app.buffers[0]
            .buffer
            .select(Position::new(0, 0), Position::new(0, 3))
            .unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::Copy));

        assert_eq!(app.kill_ring.as_deref(), Some("abc"));
        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "abc def"
        );
        assert_eq!(app.status_message, Some("Copied selection".to_string()));

        app.buffers[0]
            .buffer
            .set_cursor(Position::new(0, "abc def".len()))
            .unwrap();
        app.handle_command(&EditorCommand::Edit(EditCommand::Paste));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "abc defabc");
        assert_eq!(state.buffer.selection_range(), None);
        assert_eq!(app.status_message, Some("Pasted selection".to_string()));
    }

    #[test]
    fn copy_external_requires_opt_in_and_preserves_internal_clipboard() {
        let mut app = app_with_text("abc def");
        app.buffers[0]
            .buffer
            .select(Position::new(0, 0), Position::new(0, 3))
            .unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::CopyExternal));

        assert_eq!(app.kill_ring.as_deref(), Some("abc"));
        assert_eq!(app.take_runtime_action(), None);
        assert_eq!(
            app.status_message,
            Some("External copy disabled: copied selection internally".to_string())
        );
    }

    #[test]
    fn copy_external_emits_osc52_when_enabled_and_under_limit() {
        let mut config = Config::default();
        config.clipboard.osc52.enabled = true;
        config.clipboard.osc52.max_bytes = 8;
        let mut app = AppState::from_config(config);
        app.buffers[0].buffer.insert_str("abc").unwrap();
        app.buffers[0]
            .buffer
            .select(Position::new(0, 0), Position::new(0, 3))
            .unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::CopyExternal));

        assert_eq!(app.kill_ring.as_deref(), Some("abc"));
        assert_eq!(
            app.take_runtime_action(),
            Some(RuntimeAction::WriteTerminal(
                "\x1b]52;c;YWJj\x07".to_string()
            ))
        );
        assert_eq!(
            app.status_message,
            Some("Copied selection to external clipboard".to_string())
        );
    }

    #[test]
    fn copy_external_honors_osc52_byte_limit() {
        let mut config = Config::default();
        config.clipboard.osc52.enabled = true;
        config.clipboard.osc52.max_bytes = 2;
        let mut app = AppState::from_config(config);
        app.buffers[0].buffer.insert_str("abc").unwrap();
        app.buffers[0]
            .buffer
            .select(Position::new(0, 0), Position::new(0, 3))
            .unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::CopyExternal));

        assert_eq!(app.kill_ring.as_deref(), Some("abc"));
        assert_eq!(app.take_runtime_action(), None);
        assert_eq!(
            app.status_message,
            Some("External copy failed: selection is 3 bytes; limit is 2".to_string())
        );
    }

    #[test]
    fn cut_selection_removes_text_and_preserves_internal_clipboard() {
        let mut app = app_with_text("one two");
        app.buffers[0]
            .buffer
            .select(Position::new(0, 4), Position::new(0, 7))
            .unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::Cut));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(app.kill_ring.as_deref(), Some("two"));
        assert_eq!(state.buffer.to_text(), "one ");
        assert_eq!(state.buffer.cursor_position(), Position::new(0, 4));
        assert_eq!(state.buffer.selection_range(), None);
        assert_eq!(app.status_message, Some("Cut selection".to_string()));

        app.handle_command(&EditorCommand::Edit(EditCommand::Undo));

        assert_eq!(
            app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
            "one two"
        );
        assert_eq!(app.kill_ring.as_deref(), Some("two"));
    }

    #[test]
    fn internal_paste_replaces_active_selection() {
        let mut app = app_with_text("abc");
        app.kill_ring = Some("X".to_string());
        app.buffers[0]
            .buffer
            .select(Position::new(0, 1), Position::new(0, 2))
            .unwrap();

        app.handle_command(&EditorCommand::Edit(EditCommand::Paste));

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "aXc");
        assert_eq!(state.buffer.cursor_position(), Position::new(0, 2));
        assert_eq!(state.buffer.selection_range(), None);
    }

    #[test]
    fn cut_copy_and_internal_paste_report_empty_or_read_only_states() {
        let mut app = app_with_text("abc");

        app.handle_command(&EditorCommand::Edit(EditCommand::Copy));
        assert_eq!(app.kill_ring, None);
        assert_eq!(app.status_message, Some("Copy: no selection".to_string()));

        app.handle_command(&EditorCommand::Edit(EditCommand::Cut));
        assert_eq!(app.kill_ring, None);
        assert_eq!(app.status_message, Some("Cut: no selection".to_string()));

        app.handle_command(&EditorCommand::Edit(EditCommand::Paste));
        assert_eq!(
            app.status_message,
            Some("Paste: internal clipboard empty; use terminal paste".to_string())
        );

        app.handle_command(&EditorCommand::App(AppCommand::Help));
        let help_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
        app.buffer_state_mut(help_buffer_id)
            .unwrap()
            .buffer
            .select(Position::new(0, 0), Position::new(0, 3))
            .unwrap();
        app.kill_ring = Some("old".to_string());

        app.handle_command(&EditorCommand::Edit(EditCommand::Cut));
        assert_eq!(app.kill_ring.as_deref(), Some("old"));
        assert_eq!(
            app.status_message,
            Some("Cut failed: buffer is read-only".to_string())
        );

        app.handle_command(&EditorCommand::Edit(EditCommand::Paste));
        assert_eq!(
            app.status_message,
            Some("Paste failed: buffer is read-only".to_string())
        );
    }

    #[test]
    fn bracketed_paste_inserts_text_into_editor_buffer() {
        let mut app = AppState::new();

        app.handle_paste("a\r\nb\x1b[31m");

        let state = app.buffer_state(BufferId(1)).unwrap();
        assert_eq!(state.buffer.to_text(), "a\nb\x1b[31m");
        assert!(state.buffer.is_dirty());
    }

    #[test]
    fn paste_command_reports_empty_internal_clipboard_hint() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::Edit(EditCommand::Paste));

        assert_eq!(
            app.status_message,
            Some("Paste: internal clipboard empty; use terminal paste".to_string())
        );
    }

    #[test]
    fn bracketed_paste_rejects_read_only_focused_buffer() {
        let mut app = AppState::new();
        app.handle_command(&EditorCommand::App(AppCommand::Help));
        let buffer_id = app.workspace.focused_window().unwrap().buffer_id;
        let before = app.buffer_state(buffer_id).unwrap().buffer.to_text();

        app.handle_paste("x");

        assert_eq!(
            app.buffer_state(buffer_id).unwrap().buffer.to_text(),
            before
        );
        assert_eq!(
            app.status_message,
            Some("Paste failed: buffer is read-only".to_string())
        );
    }

    #[test]
    fn bracketed_paste_targets_prompt_and_file_dialog_as_single_line() {
        let mut app = AppState::new();

        app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
        app.handle_paste("theme\r\nmsedit");
        assert_eq!(
            app.prompt_status_text(),
            Some("Command: theme msedit".to_string())
        );

        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Esc, CrosstermKeyModifiers::NONE),
        );
        app.handle_command(&EditorCommand::File(FileCommand::Open));
        app.handle_paste("a\nb");
        assert_eq!(app.prompt_status_text(), Some("Open: a b".to_string()));
    }

    #[test]
    fn bracketed_paste_is_ignored_during_unsaved_confirmation() {
        let mut app = AppState::new();
        app.handle_text_input('x');
        app.handle_command(&EditorCommand::App(AppCommand::Quit));

        app.handle_paste("y");

        assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "x");
        assert!(app.confirm.is_some());
        assert_eq!(
            app.status_message,
            Some("Paste ignored during confirmation".to_string())
        );
    }

    #[test]
    fn right_click_reports_paste_wait_status_when_mouse_is_enabled() {
        let mut app = AppState::new();
        app.mouse_enabled = true;

        handle_mouse_event(&mut app, right_click(3, 2));

        assert_eq!(
            app.status_message,
            Some("Paste: waiting for terminal bracketed paste data".to_string())
        );
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

    fn make_destination_at_least_as_new_as(path: &Path, other: &Path, contents: &str) {
        for _ in 0..100 {
            std::thread::sleep(Duration::from_millis(10));
            std::fs::write(path, contents).unwrap();
            if file_modified(path) >= file_modified(other) {
                return;
            }
        }

        panic!("could not make destination at least as new as comparison file");
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

    fn file_dialog_list_point(app: &AppState, visible_index: usize) -> (u16, u16) {
        let overlay = app.active_overlay().expect("file dialog overlay");
        let area = app.overlay_area();
        for y in 0..area.height {
            if app.shell.hit_test_overlay_list(&overlay, area, 20, y) == Some(visible_index) {
                return (20, y);
            }
        }

        panic!("visible file dialog row {visible_index} was not hittable");
    }

    fn app_with_text(text: &str) -> AppState {
        let mut app = AppState::new();
        app.buffers[0].buffer =
            TextBuffer::from_text_with_kind(dun_core::BufferKind::Untitled, text);
        app
    }
}
