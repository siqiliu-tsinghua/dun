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
