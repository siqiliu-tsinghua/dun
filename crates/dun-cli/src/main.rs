#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent,
    KeyEventKind as CrosstermKeyEventKind, KeyModifiers as CrosstermKeyModifiers,
    MouseButton as CrosstermMouseButton, MouseEvent as CrosstermMouseEvent,
    MouseEventKind as CrosstermMouseEventKind,
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

mod app;
mod command_output;
mod dialogs;
mod files;
mod help;
mod terminal;

use app::{
    AppState, BufferSearchState, BufferState, BufferViewContext, MouseDragState, SearchDirection,
    SearchSelection, SearchSpec, StatusEntry, StatusLevel, editor_body_width,
};
use command_output::{CapturedCommandStream, CommandOutputSection, CommandRunResult};
use dialogs::{
    BufferSwitcherEntry, BufferSwitcherState, ConfirmState, CopyTextError, FileDialogContext,
    FileDialogEntry, FileDialogKind, FileDialogListing, FileDialogState, FileDialogSubmit,
    LineInput, PendingAction, PromptCompletionState, PromptHistoryKind, PromptKind,
    PromptPreviewState, PromptState, ReplaceConfirmState,
};
use files::text::{
    advance_wrapped_column, buffer_end_position, byte_column_for_wrapped_row_column,
    byte_column_for_wrapped_row_start, clamp_to_char_boundary, clamp_to_display_column,
    display_width_for_editor_char,
};
use files::{
    AtomicTempReconcileReport, FileReadSnapshot, LoadedTextBuffer, atomic_write_text_file,
    common_entry_prefix, current_file_snapshot, file_dialog_context,
    file_dialog_recent_input_for_path, is_completable_file_dialog_entry, list_file_dialog_entries,
    load_text_buffer, opened_file_status, path_error_detail, path_io_error,
    reconcile_atomic_save_temp_files, reloaded_file_status, single_line_paste_text,
    status_with_atomic_temp_report, title_for_path, validate_save_snapshot,
};
#[cfg(test)]
use files::{atomic_temp_path, validate_stable_file_read};
use help::status::{
    axis_name, bracket, buffer_disk_state, buffer_error_text, color_status,
    default_config_path_text, env_config_path_text, file_encoding_status, line_ending_status,
    replacement_status_text, scroll_status, selection_status, terminal_profile_status,
    workspace_error_text,
};
use help::text::{
    command_output_index_line, command_output_relative_section_line,
    command_output_section_view_text, command_output_status_line, command_output_stderr_body_line,
    command_output_stderr_line, command_output_stdout_body_line, command_output_stdout_line,
    command_output_summary_line, command_output_truncated_line, line_with_exact_text,
    numbered_list_index_for_line, numbered_list_rows, outline_entries_for_buffer, outline_text,
    parse_config_diagnostics_section, parse_outline_target, search_results_text,
};
#[cfg(test)]
use terminal::rewrite_16_color_sgr;
use terminal::{
    RuntimeAction, TerminalColorRewrite, TerminalGuard, TerminalWriter, command_run_status,
    duration_status_text, exit_status_text, handle_key_event, handle_mouse_event,
    handle_runtime_action, key_stroke_from_crossterm, run_command_capture,
    text_input_from_crossterm,
};

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
        let loaded = load_text_buffer(&path, self.limits.editable_file_soft_limit_bytes)
            .map_err(|error| path_io_error(&path, error))?;
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
            validate_save_snapshot(buffer.file_snapshot, &path)?;
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

        let loaded = load_text_buffer(&path, self.limits.editable_file_soft_limit_bytes)
            .map_err(|error| path_io_error(&path, error))?;
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

    pub(crate) fn command_output_buffer_id(&self) -> Option<BufferId> {
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

    pub(crate) fn confirm_focused_dirty(&mut self, action: PendingAction) -> bool {
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

fn parse_command_output_section(input: &str) -> Option<CommandOutputSection> {
    match normalize_command_line_token(input).as_str() {
        "stdout" | "out" => Some(CommandOutputSection::Stdout),
        "stderr" | "err" => Some(CommandOutputSection::Stderr),
        _ => None,
    }
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

const STATUS_HISTORY_LIMIT: usize = 128;
const COMMAND_HISTORY_LIMIT: usize = 128;

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

fn wrapping_index(index: usize, len: usize, delta: isize) -> usize {
    debug_assert!(len > 0);
    let len = len as isize;
    (index as isize).saturating_add(delta).rem_euclid(len) as usize
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
mod tests;
