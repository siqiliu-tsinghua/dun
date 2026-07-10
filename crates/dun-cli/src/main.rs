#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use crossterm::event::{
    KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent,
    KeyEventKind as CrosstermKeyEventKind, KeyModifiers as CrosstermKeyModifiers,
    MouseButton as CrosstermMouseButton, MouseEvent as CrosstermMouseEvent,
    MouseEventKind as CrosstermMouseEventKind,
};
use dun_config::{
    ClipboardConfig, FileDialogAction, FileDialogKeymap, Key, KeyModifiers, KeySequence, KeyStroke,
    Keymap, Limits, ThemeName, command_from_id, command_id, default_config_text,
    file_dialog_action_id,
};
#[cfg(test)]
use dun_config::{Config, parse_config};
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
mod cli;
mod command_line;
mod command_output;
mod config_loading;
mod dialogs;
mod files;
mod help;
mod terminal;
mod util;

use app::{
    AppState, BufferSearchState, BufferState, BufferViewContext, MouseDragState, SearchDirection,
    SearchSpec, StatusEntry, StatusLevel, choose_search_match, current_match_selection,
    editor_body_width, preview_selection_match,
};
#[cfg(test)]
pub(crate) use cli::UsageError;
pub(crate) use cli::{CliAction, CliError, cli_help_text, cli_version_text, parse_cli_args};
#[cfg(test)]
pub(crate) use command_line::CommandLineParseError;
pub(crate) use command_line::{
    COMMAND_LINE_HELP, CommandCompletion, CommandCompletionCandidate, command_line_completion,
    command_line_parse_error_text, config_diagnostics_section_values, normalize_command_line_token,
    parse_command_line, parse_theme_command_value, theme_command_values,
};
use command_output::{
    COMMAND_OUTPUT_STREAM_SOFT_LIMIT_BYTES, CapturedCommandStream, CommandRunResult,
    command_output_buffer, command_output_text,
};
#[cfg(test)]
pub(crate) use config_loading::load_startup_config;
pub(crate) use config_loading::{
    ConfigLoadRequest, ConfigSource, DUN_CONFIG_ENV, LoadedConfig, default_config_path,
    env_config_path, load_config,
};
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
use help::buffers::{
    config_diagnostics_buffer, outline_buffer, search_results_buffer, status_history_buffer,
};
#[cfg(test)]
use help::content::help_text;
use help::content::{
    file_dialog_action_key_text, file_dialog_shortcuts_text, help_buffer,
    important_config_diagnostic_commands,
};
use help::status::{
    axis_name, bracket, buffer_disk_state, buffer_error_text, color_status,
    default_config_path_text, env_config_path_text, file_encoding_status, line_ending_status,
    replacement_status_text, scroll_status, selection_status, terminal_profile_status,
    workspace_error_text,
};
use help::text::{
    ConfigDiagnosticsSection, line_with_exact_text, numbered_list_index_for_line,
    numbered_list_rows, outline_entries_for_buffer, outline_text, parse_config_diagnostics_section,
    parse_outline_target, search_results_text,
};
#[cfg(test)]
use terminal::rewrite_16_color_sgr;
use terminal::{
    RuntimeAction, TerminalColorRewrite, TerminalGuard, TerminalWriter, command_run_status,
    detect_terminal_profile, key_stroke_from_crossterm, osc52_copy_sequence, run_command_capture,
    run_event_loop, text_input_from_crossterm,
};
#[cfg(test)]
use terminal::{handle_key_event, handle_mouse_event};
use util::{
    BUFFER_SWITCHER_VISIBLE_ENTRIES, COMMAND_HISTORY_LIMIT, EDITOR_INDENT,
    EDITOR_MOUSE_WHEEL_LINES, FILE_DIALOG_VISIBLE_ENTRIES, MIN_BODY_COLUMNS_WITH_GUTTER,
    STATUS_HISTORY_LIMIT, wrapping_index,
};

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

#[cfg(test)]
mod tests;
