#![forbid(unsafe_code)]

use std::fmt;
use std::str::FromStr;

use dun_core::{AppCommand, EditCommand, EditorCommand, FileCommand, WindowCommand};
pub use dun_term::{ColorProfile, EncodingProfile, TerminalProfile, ThemeName};

mod config;
mod keys;
mod limits;

pub use config::{
    ClipboardConfig, Config, ConfigError, MouseConfig, Osc52Config, TerminalOverrides,
};
use keys::normalize_token;
pub use keys::{
    FileDialogAction, FileDialogKeyBinding, FileDialogKeymap, FileDialogKeymapError, Key,
    KeyBinding, KeyModifiers, KeyParseError, KeySequence, KeyStroke, Keymap, KeymapError,
    file_dialog_action_from_id, file_dialog_action_id,
};
pub use limits::{Limits, LimitsError};

pub fn parse_config(input: &str) -> Result<Config, ConfigParseError> {
    parse_config_overlay(Config::default(), input)
}

pub fn default_config_text() -> String {
    let config = Config::default();
    let mut out = String::from(
        "\
# Dun default configuration
# Copy to ~/.config/dun/config and edit as needed.

",
    );

    out.push_str(&format!("theme = {}\n", config.theme.as_str()));
    out.push_str("# terminal.encoding = utf8\n");
    out.push_str("# terminal.colors = 256\n");
    out.push_str(&format!("mouse.enabled = {}\n", config.mouse.enabled));
    out.push_str(&format!(
        "clipboard.osc52.enabled = {}\n",
        config.clipboard.osc52.enabled
    ));
    out.push_str(&format!(
        "clipboard.osc52.max_bytes = {}\n",
        config.clipboard.osc52.max_bytes
    ));
    out.push_str(&format!(
        "limits.editable_file_soft_limit_bytes = {}\n",
        config.limits.editable_file_soft_limit_bytes
    ));
    out.push_str(&format!(
        "limits.line_display_soft_limit_bytes = {}\n",
        config.limits.line_display_soft_limit_bytes
    ));

    out.push_str("\n# Global editor command keybindings\n");
    let mut keybindings = config
        .keybindings
        .bindings
        .iter()
        .map(|binding| (command_id(&binding.command), binding.sequence.to_string()))
        .collect::<Vec<_>>();
    keybindings.sort_by(|left, right| left.0.cmp(right.0));
    for (command, sequence) in keybindings {
        out.push_str(&format!("key.{command} = {sequence}\n"));
    }

    out.push_str("\n# Open/Save As modal keybindings\n");
    let mut file_dialog_bindings = config
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
    file_dialog_bindings.sort_by(|left, right| left.0.cmp(right.0));
    for (action, stroke) in file_dialog_bindings {
        out.push_str(&format!("key.{action} = {stroke}\n"));
    }

    out
}

pub fn parse_config_overlay(mut config: Config, input: &str) -> Result<Config, ConfigParseError> {
    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(ConfigParseError::line(
                line_number,
                "expected `key = value` entry",
            ));
        };
        apply_config_entry(&mut config, raw_key.trim(), raw_value.trim(), line_number)?;
    }

    config
        .validate()
        .map_err(|error| ConfigParseError::global(config_error_text(&error)))?;
    Ok(config)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigParseError {
    pub line: Option<usize>,
    pub message: String,
}

impl ConfigParseError {
    fn line(line: usize, message: impl Into<String>) -> Self {
        Self {
            line: Some(line),
            message: message.into(),
        }
    }

    fn global(message: impl Into<String>) -> Self {
        Self {
            line: None,
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "line {line}: {}", self.message),
            None => write!(formatter, "{}", self.message),
        }
    }
}

impl std::error::Error for ConfigParseError {}

fn strip_comment(line: &str) -> &str {
    line.split_once('#')
        .map(|(before_comment, _)| before_comment)
        .unwrap_or(line)
}

fn apply_config_entry(
    config: &mut Config,
    raw_key: &str,
    raw_value: &str,
    line_number: usize,
) -> Result<(), ConfigParseError> {
    if raw_key.is_empty() {
        return Err(ConfigParseError::line(line_number, "empty config key"));
    }

    let key = normalize_config_key(raw_key);
    let value = unquote_value(raw_value);

    match key.as_str() {
        "theme" => {
            config.theme = parse_theme_name(value)
                .ok_or_else(|| ConfigParseError::line(line_number, "unknown theme name"))?;
        }
        "terminal.encoding" => {
            config.terminal.encoding = parse_encoding_profile(value)
                .map(Some)
                .ok_or_else(|| ConfigParseError::line(line_number, "unknown terminal encoding"))?;
        }
        "terminal.colors" | "terminal.color" => {
            config.terminal.colors = parse_color_profile(value)
                .map(Some)
                .ok_or_else(|| ConfigParseError::line(line_number, "unknown terminal colors"))?;
        }
        "mouse.enabled" | "input.mouse" => {
            config.mouse.enabled = parse_bool(value)
                .ok_or_else(|| ConfigParseError::line(line_number, "expected true or false"))?;
        }
        "clipboard.osc52.enabled" | "clipboard.osc52" => {
            config.clipboard.osc52.enabled = parse_bool(value)
                .ok_or_else(|| ConfigParseError::line(line_number, "expected true or false"))?;
        }
        "clipboard.osc52.max_bytes" => {
            let value = parse_byte_count(value, line_number)?;
            config.clipboard.osc52.max_bytes = usize::try_from(value).map_err(|_| {
                ConfigParseError::line(line_number, "OSC 52 byte limit does not fit this platform")
            })?;
        }
        "limits.editable_file_soft_limit_bytes" => {
            config.limits.editable_file_soft_limit_bytes = parse_byte_count(value, line_number)?;
        }
        "limits.line_display_soft_limit_bytes" => {
            let value = parse_byte_count(value, line_number)?;
            config.limits.line_display_soft_limit_bytes = usize::try_from(value).map_err(|_| {
                ConfigParseError::line(
                    line_number,
                    "line display soft limit does not fit this platform",
                )
            })?;
        }
        _ if key.starts_with("key.file_dialog.") => {
            apply_file_dialog_key_binding(
                config,
                &key["key.file_dialog.".len()..],
                value,
                line_number,
            )?;
        }
        _ if key.starts_with("file_dialog.key.") => {
            apply_file_dialog_key_binding(
                config,
                &key["file_dialog.key.".len()..],
                value,
                line_number,
            )?;
        }
        _ if key.starts_with("key.") => {
            apply_key_binding(config, &key["key.".len()..], value, line_number)?;
        }
        _ if key.starts_with("keybinding.") => {
            apply_key_binding(config, &key["keybinding.".len()..], value, line_number)?;
        }
        _ => {
            return Err(ConfigParseError::line(
                line_number,
                format!("unknown config key `{raw_key}`"),
            ));
        }
    }

    Ok(())
}

fn apply_key_binding(
    config: &mut Config,
    command_id: &str,
    value: &str,
    line_number: usize,
) -> Result<(), ConfigParseError> {
    let command = command_from_id(command_id).map_err(|_| {
        ConfigParseError::line(line_number, format!("unknown command id `{command_id}`"))
    })?;

    let sequence = match normalize_token(value).as_str() {
        "none" | "disabled" | "unbind" => None,
        _ => Some(KeySequence::from_str(value).map_err(|error| {
            ConfigParseError::line(
                line_number,
                format!("invalid key sequence: {}", key_parse_error_text(&error)),
            )
        })?),
    };

    config.keybindings.set_command_binding(command, sequence);
    Ok(())
}

fn apply_file_dialog_key_binding(
    config: &mut Config,
    action_id: &str,
    value: &str,
    line_number: usize,
) -> Result<(), ConfigParseError> {
    let action = file_dialog_action_from_id(action_id).map_err(|_| {
        ConfigParseError::line(
            line_number,
            format!("unknown file dialog action `{action_id}`"),
        )
    })?;

    let stroke = match normalize_token(value).as_str() {
        "none" | "disabled" | "unbind" => None,
        _ => {
            let sequence = KeySequence::from_str(value).map_err(|error| {
                ConfigParseError::line(
                    line_number,
                    format!("invalid key sequence: {}", key_parse_error_text(&error)),
                )
            })?;
            if sequence.strokes.len() != 1 {
                return Err(ConfigParseError::line(
                    line_number,
                    "file dialog keybindings must use a single key stroke",
                ));
            }
            Some(sequence.strokes[0])
        }
    };

    config.file_dialog_keys.set_action_binding(action, stroke);
    Ok(())
}

fn normalize_config_key(input: &str) -> String {
    input
        .trim()
        .chars()
        .map(|ch| match ch {
            '-' | ' ' => '_',
            _ => ch.to_ascii_lowercase(),
        })
        .collect()
}

fn unquote_value(input: &str) -> &str {
    let trimmed = input.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
        {
            return &trimmed[1..trimmed.len() - 1];
        }
    }

    trimmed
}

fn parse_theme_name(input: &str) -> Option<ThemeName> {
    match normalize_token(input).as_str() {
        "msedit" | "microsoftedit" => Some(ThemeName::MsEdit),
        "turbo" | "turbovision" => Some(ThemeName::Turbo),
        "dark" => Some(ThemeName::Dark),
        "dun" => Some(ThemeName::Dun),
        _ => None,
    }
}

fn parse_encoding_profile(input: &str) -> Option<EncodingProfile> {
    match normalize_token(input).as_str() {
        "utf8" => Some(EncodingProfile::Utf8),
        "ascii" => Some(EncodingProfile::Ascii),
        _ => None,
    }
}

fn parse_color_profile(input: &str) -> Option<ColorProfile> {
    match normalize_token(input).as_str() {
        "256" | "256color" | "color256" => Some(ColorProfile::Color256),
        "16" | "16color" | "color16" | "ansi" => Some(ColorProfile::Color16),
        "mono" | "monochrome" | "none" | "off" => Some(ColorProfile::Mono),
        _ => None,
    }
}

fn parse_bool(input: &str) -> Option<bool> {
    match normalize_token(input).as_str() {
        "true" | "yes" | "on" | "1" | "enabled" => Some(true),
        "false" | "no" | "off" | "0" | "disabled" => Some(false),
        _ => None,
    }
}

fn parse_byte_count(input: &str, line_number: usize) -> Result<u64, ConfigParseError> {
    let normalized: String = input
        .trim()
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_')
        .flat_map(char::to_lowercase)
        .collect();

    let digit_count = normalized
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .count();
    if digit_count == 0 {
        return Err(ConfigParseError::line(
            line_number,
            "expected byte count such as `1048576` or `16 MiB`",
        ));
    }

    let number = normalized[..digit_count].parse::<u64>().map_err(|_| {
        ConfigParseError::line(line_number, "byte count is outside the supported range")
    })?;
    let suffix = &normalized[digit_count..];
    let multiplier = match suffix {
        "" | "b" | "byte" | "bytes" => 1,
        "k" | "kb" | "kib" => 1024,
        "m" | "mb" | "mib" => 1024 * 1024,
        "g" | "gb" | "gib" => 1024 * 1024 * 1024,
        _ => {
            return Err(ConfigParseError::line(
                line_number,
                format!("unknown byte-count suffix `{suffix}`"),
            ));
        }
    };

    number.checked_mul(multiplier).ok_or_else(|| {
        ConfigParseError::line(line_number, "byte count is outside the supported range")
    })
}

fn config_error_text(error: &ConfigError) -> String {
    match error {
        ConfigError::Keymap(error) => format!("invalid keymap: {}", keymap_error_text(error)),
        ConfigError::FileDialogKeymap(error) => {
            format!(
                "invalid file dialog keymap: {}",
                file_dialog_keymap_error_text(error)
            )
        }
        ConfigError::Limits(error) => format!("invalid limits: {}", limits_error_text(*error)),
    }
}

fn keymap_error_text(error: &KeymapError) -> String {
    match error {
        KeymapError::DuplicateBinding(sequence) => format!("duplicate key sequence `{sequence}`"),
        KeymapError::EmptySequence => "empty key sequence".to_string(),
    }
}

fn file_dialog_keymap_error_text(error: &FileDialogKeymapError) -> String {
    match error {
        FileDialogKeymapError::DuplicateBinding(stroke) => {
            format!("duplicate key stroke `{stroke}`")
        }
    }
}

fn limits_error_text(error: LimitsError) -> &'static str {
    match error {
        LimitsError::EditableFileSoftLimitZero => {
            "editable file soft limit must be greater than zero"
        }
        LimitsError::LineDisplaySoftLimitZero => {
            "line display soft limit must be greater than zero"
        }
    }
}

fn key_parse_error_text(error: &KeyParseError) -> String {
    match error {
        KeyParseError::EmptySequence => "empty sequence".to_string(),
        KeyParseError::EmptyStroke => "empty stroke".to_string(),
        KeyParseError::MissingKey => "missing key".to_string(),
        KeyParseError::DuplicateModifier(modifier) => {
            format!("duplicate modifier `{modifier}`")
        }
        KeyParseError::UnknownModifier(modifier) => format!("unknown modifier `{modifier}`"),
        KeyParseError::UnknownKey(key) => format!("unknown key `{key}`"),
        KeyParseError::InvalidFunctionKey(key) => format!("invalid function key `{key}`"),
    }
}

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

#[cfg(test)]
mod tests;
