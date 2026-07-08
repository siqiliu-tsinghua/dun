use dun_core::{AppCommand, EditCommand, EditorCommand, FileCommand, WindowCommand};

pub fn command_id(command: &EditorCommand) -> &'static str {
    match command {
        EditorCommand::File(FileCommand::New) => "file.new",
        EditorCommand::File(FileCommand::Open) => "file.open",
        EditorCommand::File(FileCommand::SwitchBuffer) => "file.switch_buffer",
        EditorCommand::File(FileCommand::Save) => "file.save",
        EditorCommand::File(FileCommand::SaveAs) => "file.save_as",
        EditorCommand::File(FileCommand::Reload) => "file.reload",
        EditorCommand::File(FileCommand::Close) => "file.close",
        EditorCommand::Edit(EditCommand::Undo) => "edit.undo",
        EditorCommand::Edit(EditCommand::Redo) => "edit.redo",
        EditorCommand::Edit(EditCommand::Cut) => "edit.cut",
        EditorCommand::Edit(EditCommand::Copy) => "edit.copy",
        EditorCommand::Edit(EditCommand::CopyExternal) => "edit.copy_external",
        EditorCommand::Edit(EditCommand::Paste) => "edit.paste",
        EditorCommand::Edit(EditCommand::SelectAll) => "edit.select_all",
        EditorCommand::Edit(EditCommand::SelectLine) => "edit.select_line",
        EditorCommand::Edit(EditCommand::CopyLine) => "edit.copy_line",
        EditorCommand::Edit(EditCommand::DeleteLine) => "edit.delete_line",
        EditorCommand::Edit(EditCommand::MoveLineUp) => "edit.move_line_up",
        EditorCommand::Edit(EditCommand::MoveLineDown) => "edit.move_line_down",
        EditorCommand::Edit(EditCommand::IndentLine) => "edit.indent_line",
        EditorCommand::Edit(EditCommand::OutdentLine) => "edit.outdent_line",
        EditorCommand::Edit(EditCommand::TrimTrailingWhitespace) => "edit.trim_trailing_whitespace",
        EditorCommand::Edit(EditCommand::ToggleWordWrap) => "edit.toggle_word_wrap",
        EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace) => {
            "edit.toggle_visible_whitespace"
        }
        EditorCommand::Edit(EditCommand::ToggleBookmark) => "edit.toggle_bookmark",
        EditorCommand::Edit(EditCommand::NextBookmark) => "edit.next_bookmark",
        EditorCommand::Edit(EditCommand::PreviousBookmark) => "edit.previous_bookmark",
        EditorCommand::Edit(EditCommand::MoveLeft) => "edit.move_left",
        EditorCommand::Edit(EditCommand::MoveRight) => "edit.move_right",
        EditorCommand::Edit(EditCommand::MoveUp) => "edit.move_up",
        EditorCommand::Edit(EditCommand::MoveDown) => "edit.move_down",
        EditorCommand::Edit(EditCommand::MovePageUp) => "edit.move_page_up",
        EditorCommand::Edit(EditCommand::MovePageDown) => "edit.move_page_down",
        EditorCommand::Edit(EditCommand::MoveDocumentStart) => "edit.move_document_start",
        EditorCommand::Edit(EditCommand::MoveDocumentEnd) => "edit.move_document_end",
        EditorCommand::Edit(EditCommand::ScrollLeft) => "edit.scroll_left",
        EditorCommand::Edit(EditCommand::ScrollRight) => "edit.scroll_right",
        EditorCommand::Edit(EditCommand::MoveWordLeft) => "edit.move_word_left",
        EditorCommand::Edit(EditCommand::MoveWordRight) => "edit.move_word_right",
        EditorCommand::Edit(EditCommand::MoveLineStart) => "edit.move_line_start",
        EditorCommand::Edit(EditCommand::MoveLineEnd) => "edit.move_line_end",
        EditorCommand::Edit(EditCommand::ExtendSelectionPageUp) => "edit.extend_selection_page_up",
        EditorCommand::Edit(EditCommand::ExtendSelectionPageDown) => {
            "edit.extend_selection_page_down"
        }
        EditorCommand::Edit(EditCommand::ExtendSelectionWordLeft) => {
            "edit.extend_selection_word_left"
        }
        EditorCommand::Edit(EditCommand::ExtendSelectionWordRight) => {
            "edit.extend_selection_word_right"
        }
        EditorCommand::Edit(EditCommand::InsertNewline) => "edit.insert_newline",
        EditorCommand::Edit(EditCommand::DeleteBackward) => "edit.delete_backward",
        EditorCommand::Edit(EditCommand::DeleteForward) => "edit.delete_forward",
        EditorCommand::Edit(EditCommand::DeleteWordBackward) => "edit.delete_word_backward",
        EditorCommand::Edit(EditCommand::DeleteWordForward) => "edit.delete_word_forward",
        EditorCommand::Edit(EditCommand::Find) => "edit.find",
        EditorCommand::Edit(EditCommand::FindNext) => "edit.find_next",
        EditorCommand::Edit(EditCommand::FindPrevious) => "edit.find_previous",
        EditorCommand::Edit(EditCommand::Replace) => "edit.replace",
        EditorCommand::Edit(EditCommand::GoToLine) => "edit.go_to_line",
        EditorCommand::Window(WindowCommand::SplitHorizontal) => "window.split_horizontal",
        EditorCommand::Window(WindowCommand::SplitVertical) => "window.split_vertical",
        EditorCommand::Window(WindowCommand::FocusLeft) => "window.focus_left",
        EditorCommand::Window(WindowCommand::FocusRight) => "window.focus_right",
        EditorCommand::Window(WindowCommand::FocusUp) => "window.focus_up",
        EditorCommand::Window(WindowCommand::FocusDown) => "window.focus_down",
        EditorCommand::Window(WindowCommand::ResizeLeft) => "window.resize_left",
        EditorCommand::Window(WindowCommand::ResizeRight) => "window.resize_right",
        EditorCommand::Window(WindowCommand::ResizeUp) => "window.resize_up",
        EditorCommand::Window(WindowCommand::ResizeDown) => "window.resize_down",
        EditorCommand::Window(WindowCommand::Equalize) => "window.equalize",
        EditorCommand::Window(WindowCommand::RotateSplit) => "window.rotate_split",
        EditorCommand::Window(WindowCommand::Collapse) => "window.collapse",
        EditorCommand::Window(WindowCommand::Expand) => "window.expand",
        EditorCommand::Window(WindowCommand::ToggleCollapse) => "window.toggle_collapse",
        EditorCommand::Window(WindowCommand::Close) => "window.close",
        EditorCommand::Window(WindowCommand::Only) => "window.only",
        EditorCommand::App(AppCommand::CommandLine) => "app.command_line",
        EditorCommand::App(AppCommand::ConfigDiagnostics) => "app.config_diagnostics",
        EditorCommand::App(AppCommand::Help) => "app.help",
        EditorCommand::App(AppCommand::ReloadConfig) => "app.reload_config",
        EditorCommand::App(AppCommand::RunCommand) => "app.run_command",
        EditorCommand::App(AppCommand::CommandOutputClear) => "app.command_output_clear",
        EditorCommand::App(AppCommand::CommandOutputCopy) => "app.command_output_copy",
        EditorCommand::App(AppCommand::CommandOutputIndex) => "app.command_output_index",
        EditorCommand::App(AppCommand::CommandOutputNextMatch) => "app.command_output_next_match",
        EditorCommand::App(AppCommand::CommandOutputNextSection) => {
            "app.command_output_next_section"
        }
        EditorCommand::App(AppCommand::CommandOutputOnlyStderr) => "app.command_output_only_stderr",
        EditorCommand::App(AppCommand::CommandOutputOnlyStdout) => "app.command_output_only_stdout",
        EditorCommand::App(AppCommand::CommandOutputPreviousMatch) => {
            "app.command_output_previous_match"
        }
        EditorCommand::App(AppCommand::CommandOutputPreviousSection) => {
            "app.command_output_previous_section"
        }
        EditorCommand::App(AppCommand::CommandOutputStderr) => "app.command_output_stderr",
        EditorCommand::App(AppCommand::CommandOutputStderrBody) => "app.command_output_stderr_body",
        EditorCommand::App(AppCommand::CommandOutputStatus) => "app.command_output_status",
        EditorCommand::App(AppCommand::CommandOutputStdout) => "app.command_output_stdout",
        EditorCommand::App(AppCommand::CommandOutputStdoutBody) => "app.command_output_stdout_body",
        EditorCommand::App(AppCommand::CommandOutputSummary) => "app.command_output_summary",
        EditorCommand::App(AppCommand::CommandOutputSave) => "app.command_output_save",
        EditorCommand::App(AppCommand::CommandOutputTruncated) => "app.command_output_truncated",
        EditorCommand::App(AppCommand::ConfigDiagnosticsClipboard) => {
            "app.config_diagnostics_clipboard"
        }
        EditorCommand::App(AppCommand::ConfigDiagnosticsFileDialogKeymap) => {
            "app.config_diagnostics_file_dialog_keymap"
        }
        EditorCommand::App(AppCommand::ConfigDiagnosticsInput) => "app.config_diagnostics_input",
        EditorCommand::App(AppCommand::ConfigDiagnosticsKeymap) => "app.config_diagnostics_keymap",
        EditorCommand::App(AppCommand::ConfigDiagnosticsLimits) => "app.config_diagnostics_limits",
        EditorCommand::App(AppCommand::ConfigDiagnosticsPaths) => "app.config_diagnostics_paths",
        EditorCommand::App(AppCommand::ConfigDiagnosticsSource) => "app.config_diagnostics_source",
        EditorCommand::App(AppCommand::ConfigDiagnosticsSummary) => {
            "app.config_diagnostics_summary"
        }
        EditorCommand::App(AppCommand::ConfigDiagnosticsTerminal) => {
            "app.config_diagnostics_terminal"
        }
        EditorCommand::App(AppCommand::Outline) => "app.outline",
        EditorCommand::App(AppCommand::SearchResults) => "app.search_results",
        EditorCommand::App(AppCommand::ShellEscape) => "app.shell_escape",
        EditorCommand::App(AppCommand::StatusHistory) => "app.status_history",
        EditorCommand::App(AppCommand::Quit) => "app.quit",
    }
}

pub fn command_from_id(input: &str) -> Result<EditorCommand, CommandParseError> {
    match normalize_command_id(input).as_str() {
        "file.new" => Ok(EditorCommand::File(FileCommand::New)),
        "file.open" => Ok(EditorCommand::File(FileCommand::Open)),
        "file.switch_buffer" => Ok(EditorCommand::File(FileCommand::SwitchBuffer)),
        "file.save" => Ok(EditorCommand::File(FileCommand::Save)),
        "file.save_as" => Ok(EditorCommand::File(FileCommand::SaveAs)),
        "file.reload" => Ok(EditorCommand::File(FileCommand::Reload)),
        "file.close" => Ok(EditorCommand::File(FileCommand::Close)),
        "edit.undo" => Ok(EditorCommand::Edit(EditCommand::Undo)),
        "edit.redo" => Ok(EditorCommand::Edit(EditCommand::Redo)),
        "edit.cut" => Ok(EditorCommand::Edit(EditCommand::Cut)),
        "edit.copy" => Ok(EditorCommand::Edit(EditCommand::Copy)),
        "edit.copy_external" => Ok(EditorCommand::Edit(EditCommand::CopyExternal)),
        "edit.paste" => Ok(EditorCommand::Edit(EditCommand::Paste)),
        "edit.select_all" => Ok(EditorCommand::Edit(EditCommand::SelectAll)),
        "edit.select_line" => Ok(EditorCommand::Edit(EditCommand::SelectLine)),
        "edit.copy_line" => Ok(EditorCommand::Edit(EditCommand::CopyLine)),
        "edit.delete_line" => Ok(EditorCommand::Edit(EditCommand::DeleteLine)),
        "edit.move_line_up" => Ok(EditorCommand::Edit(EditCommand::MoveLineUp)),
        "edit.move_line_down" => Ok(EditorCommand::Edit(EditCommand::MoveLineDown)),
        "edit.indent_line" => Ok(EditorCommand::Edit(EditCommand::IndentLine)),
        "edit.outdent_line" => Ok(EditorCommand::Edit(EditCommand::OutdentLine)),
        "edit.trim_trailing_whitespace" => {
            Ok(EditorCommand::Edit(EditCommand::TrimTrailingWhitespace))
        }
        "edit.toggle_word_wrap" => Ok(EditorCommand::Edit(EditCommand::ToggleWordWrap)),
        "edit.toggle_visible_whitespace" => {
            Ok(EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace))
        }
        "edit.toggle_bookmark" => Ok(EditorCommand::Edit(EditCommand::ToggleBookmark)),
        "edit.next_bookmark" => Ok(EditorCommand::Edit(EditCommand::NextBookmark)),
        "edit.previous_bookmark" => Ok(EditorCommand::Edit(EditCommand::PreviousBookmark)),
        "edit.move_left" => Ok(EditorCommand::Edit(EditCommand::MoveLeft)),
        "edit.move_right" => Ok(EditorCommand::Edit(EditCommand::MoveRight)),
        "edit.move_up" => Ok(EditorCommand::Edit(EditCommand::MoveUp)),
        "edit.move_down" => Ok(EditorCommand::Edit(EditCommand::MoveDown)),
        "edit.move_page_up" => Ok(EditorCommand::Edit(EditCommand::MovePageUp)),
        "edit.move_page_down" => Ok(EditorCommand::Edit(EditCommand::MovePageDown)),
        "edit.move_document_start" => Ok(EditorCommand::Edit(EditCommand::MoveDocumentStart)),
        "edit.move_document_end" => Ok(EditorCommand::Edit(EditCommand::MoveDocumentEnd)),
        "edit.scroll_left" => Ok(EditorCommand::Edit(EditCommand::ScrollLeft)),
        "edit.scroll_right" => Ok(EditorCommand::Edit(EditCommand::ScrollRight)),
        "edit.move_word_left" => Ok(EditorCommand::Edit(EditCommand::MoveWordLeft)),
        "edit.move_word_right" => Ok(EditorCommand::Edit(EditCommand::MoveWordRight)),
        "edit.move_line_start" => Ok(EditorCommand::Edit(EditCommand::MoveLineStart)),
        "edit.move_line_end" => Ok(EditorCommand::Edit(EditCommand::MoveLineEnd)),
        "edit.extend_selection_page_up" => {
            Ok(EditorCommand::Edit(EditCommand::ExtendSelectionPageUp))
        }
        "edit.extend_selection_page_down" => {
            Ok(EditorCommand::Edit(EditCommand::ExtendSelectionPageDown))
        }
        "edit.extend_selection_word_left" => {
            Ok(EditorCommand::Edit(EditCommand::ExtendSelectionWordLeft))
        }
        "edit.extend_selection_word_right" => {
            Ok(EditorCommand::Edit(EditCommand::ExtendSelectionWordRight))
        }
        "edit.insert_newline" => Ok(EditorCommand::Edit(EditCommand::InsertNewline)),
        "edit.delete_backward" => Ok(EditorCommand::Edit(EditCommand::DeleteBackward)),
        "edit.delete_forward" => Ok(EditorCommand::Edit(EditCommand::DeleteForward)),
        "edit.delete_word_backward" => Ok(EditorCommand::Edit(EditCommand::DeleteWordBackward)),
        "edit.delete_word_forward" => Ok(EditorCommand::Edit(EditCommand::DeleteWordForward)),
        "edit.find" => Ok(EditorCommand::Edit(EditCommand::Find)),
        "edit.find_next" => Ok(EditorCommand::Edit(EditCommand::FindNext)),
        "edit.find_previous" => Ok(EditorCommand::Edit(EditCommand::FindPrevious)),
        "edit.replace" => Ok(EditorCommand::Edit(EditCommand::Replace)),
        "edit.go_to_line" => Ok(EditorCommand::Edit(EditCommand::GoToLine)),
        "window.split_horizontal" => Ok(EditorCommand::Window(WindowCommand::SplitHorizontal)),
        "window.split_vertical" => Ok(EditorCommand::Window(WindowCommand::SplitVertical)),
        "window.focus_left" => Ok(EditorCommand::Window(WindowCommand::FocusLeft)),
        "window.focus_right" => Ok(EditorCommand::Window(WindowCommand::FocusRight)),
        "window.focus_up" => Ok(EditorCommand::Window(WindowCommand::FocusUp)),
        "window.focus_down" => Ok(EditorCommand::Window(WindowCommand::FocusDown)),
        "window.resize_left" => Ok(EditorCommand::Window(WindowCommand::ResizeLeft)),
        "window.resize_right" => Ok(EditorCommand::Window(WindowCommand::ResizeRight)),
        "window.resize_up" => Ok(EditorCommand::Window(WindowCommand::ResizeUp)),
        "window.resize_down" => Ok(EditorCommand::Window(WindowCommand::ResizeDown)),
        "window.equalize" => Ok(EditorCommand::Window(WindowCommand::Equalize)),
        "window.rotate_split" => Ok(EditorCommand::Window(WindowCommand::RotateSplit)),
        "window.collapse" => Ok(EditorCommand::Window(WindowCommand::Collapse)),
        "window.expand" => Ok(EditorCommand::Window(WindowCommand::Expand)),
        "window.toggle_collapse" => Ok(EditorCommand::Window(WindowCommand::ToggleCollapse)),
        "window.close" => Ok(EditorCommand::Window(WindowCommand::Close)),
        "window.only" => Ok(EditorCommand::Window(WindowCommand::Only)),
        "app.command_line" => Ok(EditorCommand::App(AppCommand::CommandLine)),
        "app.config_diagnostics" => Ok(EditorCommand::App(AppCommand::ConfigDiagnostics)),
        "app.help" => Ok(EditorCommand::App(AppCommand::Help)),
        "app.reload_config" => Ok(EditorCommand::App(AppCommand::ReloadConfig)),
        "app.run_command" => Ok(EditorCommand::App(AppCommand::RunCommand)),
        "app.command_output_clear" => Ok(EditorCommand::App(AppCommand::CommandOutputClear)),
        "app.command_output_copy" => Ok(EditorCommand::App(AppCommand::CommandOutputCopy)),
        "app.command_output_index" => Ok(EditorCommand::App(AppCommand::CommandOutputIndex)),
        "app.command_output_next_match" => {
            Ok(EditorCommand::App(AppCommand::CommandOutputNextMatch))
        }
        "app.command_output_next_section" => {
            Ok(EditorCommand::App(AppCommand::CommandOutputNextSection))
        }
        "app.command_output_only_stderr" => {
            Ok(EditorCommand::App(AppCommand::CommandOutputOnlyStderr))
        }
        "app.command_output_only_stdout" => {
            Ok(EditorCommand::App(AppCommand::CommandOutputOnlyStdout))
        }
        "app.command_output_previous_match" => {
            Ok(EditorCommand::App(AppCommand::CommandOutputPreviousMatch))
        }
        "app.command_output_previous_section" => {
            Ok(EditorCommand::App(AppCommand::CommandOutputPreviousSection))
        }
        "app.command_output_stderr" => Ok(EditorCommand::App(AppCommand::CommandOutputStderr)),
        "app.command_output_stderr_body" => {
            Ok(EditorCommand::App(AppCommand::CommandOutputStderrBody))
        }
        "app.command_output_status" => Ok(EditorCommand::App(AppCommand::CommandOutputStatus)),
        "app.command_output_stdout" => Ok(EditorCommand::App(AppCommand::CommandOutputStdout)),
        "app.command_output_stdout_body" => {
            Ok(EditorCommand::App(AppCommand::CommandOutputStdoutBody))
        }
        "app.command_output_summary" => Ok(EditorCommand::App(AppCommand::CommandOutputSummary)),
        "app.command_output_save" => Ok(EditorCommand::App(AppCommand::CommandOutputSave)),
        "app.command_output_truncated" => {
            Ok(EditorCommand::App(AppCommand::CommandOutputTruncated))
        }
        "app.config_diagnostics_clipboard" => {
            Ok(EditorCommand::App(AppCommand::ConfigDiagnosticsClipboard))
        }
        "app.config_diagnostics_file_dialog_keymap" => Ok(EditorCommand::App(
            AppCommand::ConfigDiagnosticsFileDialogKeymap,
        )),
        "app.config_diagnostics_input" => {
            Ok(EditorCommand::App(AppCommand::ConfigDiagnosticsInput))
        }
        "app.config_diagnostics_keymap" => {
            Ok(EditorCommand::App(AppCommand::ConfigDiagnosticsKeymap))
        }
        "app.config_diagnostics_limits" => {
            Ok(EditorCommand::App(AppCommand::ConfigDiagnosticsLimits))
        }
        "app.config_diagnostics_paths" => {
            Ok(EditorCommand::App(AppCommand::ConfigDiagnosticsPaths))
        }
        "app.config_diagnostics_source" => {
            Ok(EditorCommand::App(AppCommand::ConfigDiagnosticsSource))
        }
        "app.config_diagnostics_summary" => {
            Ok(EditorCommand::App(AppCommand::ConfigDiagnosticsSummary))
        }
        "app.config_diagnostics_terminal" => {
            Ok(EditorCommand::App(AppCommand::ConfigDiagnosticsTerminal))
        }
        "app.outline" => Ok(EditorCommand::App(AppCommand::Outline)),
        "app.search_results" => Ok(EditorCommand::App(AppCommand::SearchResults)),
        "app.shell_escape" => Ok(EditorCommand::App(AppCommand::ShellEscape)),
        "app.status_history" => Ok(EditorCommand::App(AppCommand::StatusHistory)),
        "app.quit" => Ok(EditorCommand::App(AppCommand::Quit)),
        _ => Err(CommandParseError::UnknownCommand(input.to_string())),
    }
}

pub(crate) fn normalize_command_id(input: &str) -> String {
    input
        .trim()
        .chars()
        .map(|ch| match ch {
            '-' | ' ' => '_',
            _ => ch.to_ascii_lowercase(),
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandParseError {
    UnknownCommand(String),
}
