#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use dun_core::{AppCommand, EditCommand, EditorCommand, FileCommand, WindowCommand};
use dun_term::Theme;
pub use dun_term::{ColorProfile, EncodingProfile, TerminalProfile, ThemeName};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub theme: ThemeName,
    pub terminal: TerminalOverrides,
    pub mouse: MouseConfig,
    pub clipboard: ClipboardConfig,
    pub keybindings: Keymap,
    pub file_dialog_keys: FileDialogKeymap,
    pub limits: Limits,
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.keybindings.validate()?;
        self.file_dialog_keys.validate()?;
        self.limits.validate()?;
        Ok(())
    }

    pub fn terminal_profile(&self, detected: TerminalProfile) -> TerminalProfile {
        self.terminal.apply_to(detected)
    }

    pub fn resolved_theme(&self, detected: TerminalProfile) -> Theme {
        Theme::for_profile(self.theme, self.terminal_profile(detected))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeName::MsEdit,
            terminal: TerminalOverrides::default(),
            mouse: MouseConfig::default(),
            clipboard: ClipboardConfig::default(),
            keybindings: Keymap::default(),
            file_dialog_keys: FileDialogKeymap::default(),
            limits: Limits::default(),
        }
    }
}

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
        KeymapError::DuplicateBinding(sequence) => {
            format!("duplicate key sequence `{}`", key_sequence_text(sequence))
        }
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

fn key_sequence_text(sequence: &KeySequence) -> String {
    sequence
        .strokes
        .iter()
        .map(key_stroke_text)
        .collect::<Vec<_>>()
        .join(",")
}

fn key_stroke_text(stroke: &KeyStroke) -> String {
    let mut parts = Vec::new();
    if stroke.modifiers.ctrl {
        parts.push("Ctrl".to_string());
    }
    if stroke.modifiers.alt {
        parts.push("Alt".to_string());
    }
    if stroke.modifiers.shift {
        parts.push("Shift".to_string());
    }
    parts.push(key_text(stroke.key));
    parts.join("+")
}

fn key_text(key: Key) -> String {
    match key {
        Key::Char(ch) if ch.is_ascii_alphabetic() => ch.to_ascii_uppercase().to_string(),
        Key::Char(ch) => ch.to_string(),
        Key::F(number) => format!("F{number}"),
        Key::Enter => "Enter".to_string(),
        Key::Esc => "Esc".to_string(),
        Key::Backspace => "Backspace".to_string(),
        Key::Delete => "Delete".to_string(),
        Key::Insert => "Insert".to_string(),
        Key::Tab => "Tab".to_string(),
        Key::BackTab => "BackTab".to_string(),
        Key::Left => "Left".to_string(),
        Key::Right => "Right".to_string(),
        Key::Up => "Up".to_string(),
        Key::Down => "Down".to_string(),
        Key::Home => "Home".to_string(),
        Key::End => "End".to_string(),
        Key::PageUp => "PageUp".to_string(),
        Key::PageDown => "PageDown".to_string(),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerminalOverrides {
    pub encoding: Option<EncodingProfile>,
    pub colors: Option<ColorProfile>,
}

impl TerminalOverrides {
    pub const fn apply_to(self, detected: TerminalProfile) -> TerminalProfile {
        TerminalProfile {
            encoding: match self.encoding {
                Some(encoding) => encoding,
                None => detected.encoding,
            },
            colors: match self.colors {
                Some(colors) => colors,
                None => detected.colors,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseConfig {
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipboardConfig {
    pub osc52: Osc52Config,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            osc52: Osc52Config::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Osc52Config {
    pub enabled: bool,
    pub max_bytes: usize,
}

impl Default for Osc52Config {
    fn default() -> Self {
        Self {
            enabled: false,
            max_bytes: 16 * 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    Keymap(KeymapError),
    FileDialogKeymap(FileDialogKeymapError),
    Limits(LimitsError),
}

impl From<KeymapError> for ConfigError {
    fn from(error: KeymapError) -> Self {
        Self::Keymap(error)
    }
}

impl From<FileDialogKeymapError> for ConfigError {
    fn from(error: FileDialogKeymapError) -> Self {
        Self::FileDialogKeymap(error)
    }
}

impl From<LimitsError> for ConfigError {
    fn from(error: LimitsError) -> Self {
        Self::Limits(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keymap {
    pub bindings: Vec<KeyBinding>,
}

impl Keymap {
    pub fn new(bindings: Vec<KeyBinding>) -> Result<Self, KeymapError> {
        let keymap = Self { bindings };
        keymap.validate()?;
        Ok(keymap)
    }

    pub fn empty() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn default_editor() -> Self {
        Self {
            bindings: vec![
                KeyBinding::new("F1", EditorCommand::App(AppCommand::Help)),
                KeyBinding::new("F2", EditorCommand::App(AppCommand::StatusHistory)),
                KeyBinding::new("F5", EditorCommand::App(AppCommand::ReloadConfig)),
                KeyBinding::new("F6", EditorCommand::App(AppCommand::ConfigDiagnostics)),
                KeyBinding::new("Ctrl+Q", EditorCommand::App(AppCommand::Quit)),
                KeyBinding::new("Ctrl+P", EditorCommand::App(AppCommand::CommandLine)),
                KeyBinding::new("Ctrl+W,O", EditorCommand::App(AppCommand::RunCommand)),
                KeyBinding::new("Ctrl+W,S", EditorCommand::App(AppCommand::ShellEscape)),
                KeyBinding::new("Ctrl+N", EditorCommand::File(FileCommand::New)),
                KeyBinding::new("Ctrl+O", EditorCommand::File(FileCommand::Open)),
                KeyBinding::new("Ctrl+W,B", EditorCommand::File(FileCommand::SwitchBuffer)),
                KeyBinding::new("Ctrl+S", EditorCommand::File(FileCommand::Save)),
                KeyBinding::new("Ctrl+Shift+S", EditorCommand::File(FileCommand::SaveAs)),
                KeyBinding::new("Ctrl+W,E", EditorCommand::File(FileCommand::Reload)),
                KeyBinding::new("Ctrl+W,Q", EditorCommand::File(FileCommand::Close)),
                KeyBinding::new("Ctrl+Z", EditorCommand::Edit(EditCommand::Undo)),
                KeyBinding::new("Ctrl+Y", EditorCommand::Edit(EditCommand::Redo)),
                KeyBinding::new(
                    "Ctrl+W,Ctrl+C",
                    EditorCommand::Edit(EditCommand::CopyExternal),
                ),
                KeyBinding::new("Ctrl+F", EditorCommand::Edit(EditCommand::Find)),
                KeyBinding::new("F3", EditorCommand::Edit(EditCommand::FindNext)),
                KeyBinding::new("Shift+F3", EditorCommand::Edit(EditCommand::FindPrevious)),
                KeyBinding::new("Ctrl+R", EditorCommand::Edit(EditCommand::Replace)),
                KeyBinding::new("Ctrl+G", EditorCommand::Edit(EditCommand::GoToLine)),
                KeyBinding::new("Ctrl+A", EditorCommand::Edit(EditCommand::SelectAll)),
                KeyBinding::new("Ctrl+L", EditorCommand::Edit(EditCommand::SelectLine)),
                KeyBinding::new("Ctrl+W,Y", EditorCommand::Edit(EditCommand::CopyLine)),
                KeyBinding::new("Ctrl+K", EditorCommand::Edit(EditCommand::DeleteLine)),
                KeyBinding::new("Ctrl+W,U", EditorCommand::Edit(EditCommand::MoveLineUp)),
                KeyBinding::new("Ctrl+W,J", EditorCommand::Edit(EditCommand::MoveLineDown)),
                KeyBinding::new("Tab", EditorCommand::Edit(EditCommand::IndentLine)),
                KeyBinding::new("BackTab", EditorCommand::Edit(EditCommand::OutdentLine)),
                KeyBinding::new(
                    "Ctrl+W,T",
                    EditorCommand::Edit(EditCommand::TrimTrailingWhitespace),
                ),
                KeyBinding::new("Ctrl+W,Z", EditorCommand::Edit(EditCommand::ToggleWordWrap)),
                KeyBinding::new(
                    "Ctrl+W,.",
                    EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace),
                ),
                KeyBinding::new("Ctrl+W,M", EditorCommand::Edit(EditCommand::ToggleBookmark)),
                KeyBinding::new("Ctrl+W,N", EditorCommand::Edit(EditCommand::NextBookmark)),
                KeyBinding::new(
                    "Ctrl+W,P",
                    EditorCommand::Edit(EditCommand::PreviousBookmark),
                ),
                KeyBinding::new("Left", EditorCommand::Edit(EditCommand::MoveLeft)),
                KeyBinding::new("Right", EditorCommand::Edit(EditCommand::MoveRight)),
                KeyBinding::new("Up", EditorCommand::Edit(EditCommand::MoveUp)),
                KeyBinding::new("Down", EditorCommand::Edit(EditCommand::MoveDown)),
                KeyBinding::new("PageUp", EditorCommand::Edit(EditCommand::MovePageUp)),
                KeyBinding::new("PageDown", EditorCommand::Edit(EditCommand::MovePageDown)),
                KeyBinding::new("Ctrl+W,[", EditorCommand::Edit(EditCommand::ScrollLeft)),
                KeyBinding::new("Ctrl+W,]", EditorCommand::Edit(EditCommand::ScrollRight)),
                KeyBinding::new(
                    "Shift+PageUp",
                    EditorCommand::Edit(EditCommand::ExtendSelectionPageUp),
                ),
                KeyBinding::new(
                    "Shift+PageDown",
                    EditorCommand::Edit(EditCommand::ExtendSelectionPageDown),
                ),
                KeyBinding::new("Ctrl+Left", EditorCommand::Edit(EditCommand::MoveWordLeft)),
                KeyBinding::new(
                    "Ctrl+Right",
                    EditorCommand::Edit(EditCommand::MoveWordRight),
                ),
                KeyBinding::new(
                    "Ctrl+Shift+Left",
                    EditorCommand::Edit(EditCommand::ExtendSelectionWordLeft),
                ),
                KeyBinding::new(
                    "Ctrl+Shift+Right",
                    EditorCommand::Edit(EditCommand::ExtendSelectionWordRight),
                ),
                KeyBinding::new("Home", EditorCommand::Edit(EditCommand::MoveLineStart)),
                KeyBinding::new("End", EditorCommand::Edit(EditCommand::MoveLineEnd)),
                KeyBinding::new("Enter", EditorCommand::Edit(EditCommand::InsertNewline)),
                KeyBinding::new(
                    "Backspace",
                    EditorCommand::Edit(EditCommand::DeleteBackward),
                ),
                KeyBinding::new("Delete", EditorCommand::Edit(EditCommand::DeleteForward)),
                KeyBinding::new(
                    "Ctrl+Backspace",
                    EditorCommand::Edit(EditCommand::DeleteWordBackward),
                ),
                KeyBinding::new(
                    "Ctrl+Delete",
                    EditorCommand::Edit(EditCommand::DeleteWordForward),
                ),
                KeyBinding::new(
                    "Ctrl+W,H",
                    EditorCommand::Window(WindowCommand::SplitHorizontal),
                ),
                KeyBinding::new(
                    "Ctrl+W,V",
                    EditorCommand::Window(WindowCommand::SplitVertical),
                ),
                KeyBinding::new(
                    "Ctrl+W,Left",
                    EditorCommand::Window(WindowCommand::FocusLeft),
                ),
                KeyBinding::new(
                    "Ctrl+W,Right",
                    EditorCommand::Window(WindowCommand::FocusRight),
                ),
                KeyBinding::new("Ctrl+W,Up", EditorCommand::Window(WindowCommand::FocusUp)),
                KeyBinding::new(
                    "Ctrl+W,Down",
                    EditorCommand::Window(WindowCommand::FocusDown),
                ),
                KeyBinding::new("Alt+Left", EditorCommand::Window(WindowCommand::FocusLeft)),
                KeyBinding::new(
                    "Alt+Right",
                    EditorCommand::Window(WindowCommand::FocusRight),
                ),
                KeyBinding::new("Alt+Up", EditorCommand::Window(WindowCommand::FocusUp)),
                KeyBinding::new("Alt+Down", EditorCommand::Window(WindowCommand::FocusDown)),
                KeyBinding::new(
                    "Ctrl+W,Shift+Left",
                    EditorCommand::Window(WindowCommand::ResizeLeft),
                ),
                KeyBinding::new(
                    "Ctrl+W,Shift+Right",
                    EditorCommand::Window(WindowCommand::ResizeRight),
                ),
                KeyBinding::new(
                    "Ctrl+W,Shift+Up",
                    EditorCommand::Window(WindowCommand::ResizeUp),
                ),
                KeyBinding::new(
                    "Ctrl+W,Shift+Down",
                    EditorCommand::Window(WindowCommand::ResizeDown),
                ),
                KeyBinding::new(
                    "Alt+Shift+Left",
                    EditorCommand::Window(WindowCommand::ResizeLeft),
                ),
                KeyBinding::new(
                    "Alt+Shift+Right",
                    EditorCommand::Window(WindowCommand::ResizeRight),
                ),
                KeyBinding::new(
                    "Alt+Shift+Up",
                    EditorCommand::Window(WindowCommand::ResizeUp),
                ),
                KeyBinding::new(
                    "Alt+Shift+Down",
                    EditorCommand::Window(WindowCommand::ResizeDown),
                ),
                KeyBinding::new("Ctrl+W,=", EditorCommand::Window(WindowCommand::Equalize)),
                KeyBinding::new(
                    "Ctrl+W,R",
                    EditorCommand::Window(WindowCommand::RotateSplit),
                ),
                KeyBinding::new(
                    "Ctrl+W,C",
                    EditorCommand::Window(WindowCommand::ToggleCollapse),
                ),
                KeyBinding::new("Ctrl+W,X", EditorCommand::Window(WindowCommand::Close)),
            ],
        }
    }

    pub fn validate(&self) -> Result<(), KeymapError> {
        let mut seen = HashSet::new();

        for binding in &self.bindings {
            if binding.sequence.strokes.is_empty() {
                return Err(KeymapError::EmptySequence);
            }

            if !seen.insert(binding.sequence.clone()) {
                return Err(KeymapError::DuplicateBinding(binding.sequence.clone()));
            }
        }

        Ok(())
    }

    pub fn command_for_sequence(&self, sequence: &KeySequence) -> Option<&EditorCommand> {
        self.bindings
            .iter()
            .find(|binding| &binding.sequence == sequence)
            .map(|binding| &binding.command)
    }

    pub fn command_for_stroke(&self, stroke: KeyStroke) -> Option<&EditorCommand> {
        self.command_for_sequence(&KeySequence::single(stroke))
    }

    pub fn sequence_for_command(&self, command: &EditorCommand) -> Option<&KeySequence> {
        self.bindings
            .iter()
            .find(|binding| &binding.command == command)
            .map(|binding| &binding.sequence)
    }

    pub fn has_sequence_prefix(&self, sequence: &KeySequence) -> bool {
        if sequence.strokes.is_empty() {
            return false;
        }

        self.bindings.iter().any(|binding| {
            binding.sequence.strokes.len() > sequence.strokes.len()
                && binding.sequence.strokes.starts_with(&sequence.strokes)
        })
    }

    pub fn set_command_binding(&mut self, command: EditorCommand, sequence: Option<KeySequence>) {
        self.bindings.retain(|binding| binding.command != command);
        if let Some(sequence) = sequence {
            self.bindings.push(KeyBinding { sequence, command });
        }
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::default_editor()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDialogKeymap {
    pub bindings: Vec<FileDialogKeyBinding>,
}

impl FileDialogKeymap {
    pub fn new(bindings: Vec<FileDialogKeyBinding>) -> Result<Self, FileDialogKeymapError> {
        let keymap = Self { bindings };
        keymap.validate()?;
        Ok(keymap)
    }

    pub fn default_file_dialog() -> Self {
        Self {
            bindings: vec![
                FileDialogKeyBinding::new("Esc", FileDialogAction::Cancel),
                FileDialogKeyBinding::new("Enter", FileDialogAction::Submit),
                FileDialogKeyBinding::new("Tab", FileDialogAction::CompleteForward),
                FileDialogKeyBinding::new("BackTab", FileDialogAction::CompleteBackward),
                FileDialogKeyBinding::new("Ctrl+H", FileDialogAction::ToggleHidden),
                FileDialogKeyBinding::new("Up", FileDialogAction::MoveSelectionUp),
                FileDialogKeyBinding::new("Down", FileDialogAction::MoveSelectionDown),
                FileDialogKeyBinding::new("PageUp", FileDialogAction::PageSelectionUp),
                FileDialogKeyBinding::new("PageDown", FileDialogAction::PageSelectionDown),
                FileDialogKeyBinding::new("Left", FileDialogAction::MoveInputLeft),
                FileDialogKeyBinding::new("Right", FileDialogAction::MoveInputRight),
                FileDialogKeyBinding::new("Home", FileDialogAction::MoveInputStart),
                FileDialogKeyBinding::new("End", FileDialogAction::MoveInputEnd),
                FileDialogKeyBinding::new("Backspace", FileDialogAction::DeleteBackward),
                FileDialogKeyBinding::new("Delete", FileDialogAction::DeleteForward),
            ],
        }
    }

    pub fn validate(&self) -> Result<(), FileDialogKeymapError> {
        let mut seen = HashSet::new();

        for binding in &self.bindings {
            if !seen.insert(binding.stroke) {
                return Err(FileDialogKeymapError::DuplicateBinding(binding.stroke));
            }
        }

        Ok(())
    }

    pub fn action_for_stroke(&self, stroke: KeyStroke) -> Option<FileDialogAction> {
        self.bindings
            .iter()
            .find(|binding| binding.stroke == stroke)
            .map(|binding| binding.action)
    }

    pub fn stroke_for_action(&self, action: FileDialogAction) -> Option<KeyStroke> {
        self.bindings
            .iter()
            .find(|binding| binding.action == action)
            .map(|binding| binding.stroke)
    }

    pub fn set_action_binding(&mut self, action: FileDialogAction, stroke: Option<KeyStroke>) {
        self.bindings.retain(|binding| binding.action != action);
        if let Some(stroke) = stroke {
            self.bindings.push(FileDialogKeyBinding { stroke, action });
        }
    }
}

impl Default for FileDialogKeymap {
    fn default() -> Self {
        Self::default_file_dialog()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileDialogKeyBinding {
    pub stroke: KeyStroke,
    pub action: FileDialogAction,
}

impl FileDialogKeyBinding {
    pub fn new(stroke: &str, action: FileDialogAction) -> Self {
        Self {
            stroke: KeyStroke::from_str(stroke).expect("default file dialog key should be valid"),
            action,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileDialogAction {
    Cancel,
    Submit,
    CompleteForward,
    CompleteBackward,
    ToggleHidden,
    MoveSelectionUp,
    MoveSelectionDown,
    PageSelectionUp,
    PageSelectionDown,
    MoveInputLeft,
    MoveInputRight,
    MoveInputStart,
    MoveInputEnd,
    DeleteBackward,
    DeleteForward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileDialogKeymapError {
    DuplicateBinding(KeyStroke),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    pub sequence: KeySequence,
    pub command: EditorCommand,
}

impl KeyBinding {
    pub fn new(sequence: &str, command: EditorCommand) -> Self {
        Self {
            sequence: KeySequence::parse_lossy(sequence),
            command,
        }
    }

    pub fn try_new(sequence: &str, command: EditorCommand) -> Result<Self, KeyParseError> {
        Ok(Self {
            sequence: KeySequence::from_str(sequence)?,
            command,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeymapError {
    DuplicateBinding(KeySequence),
    EmptySequence,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeySequence {
    pub strokes: Vec<KeyStroke>,
}

impl KeySequence {
    pub fn single(stroke: KeyStroke) -> Self {
        Self {
            strokes: vec![stroke],
        }
    }

    pub fn parse_lossy(input: &str) -> Self {
        Self::from_str(input).expect("default key sequence should be valid")
    }
}

impl FromStr for KeySequence {
    type Err = KeyParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(KeyParseError::EmptySequence);
        }

        let mut strokes = Vec::new();
        for raw_stroke in trimmed.split(',') {
            let raw_stroke = raw_stroke.trim();
            if raw_stroke.is_empty() {
                return Err(KeyParseError::EmptyStroke);
            }
            strokes.push(KeyStroke::from_str(raw_stroke)?);
        }

        Ok(Self { strokes })
    }
}

impl fmt::Display for KeySequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", key_sequence_text(self))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyStroke {
    pub key: Key,
    pub modifiers: KeyModifiers,
}

impl KeyStroke {
    pub const fn new(key: Key, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    pub const fn plain(key: Key) -> Self {
        Self::new(key, KeyModifiers::NONE)
    }
}

impl FromStr for KeyStroke {
    type Err = KeyParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        parse_key_stroke(input)
    }
}

impl fmt::Display for KeyStroke {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", key_stroke_text(self))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    F(u8),
    Enter,
    Esc,
    Backspace,
    Delete,
    Insert,
    Tab,
    BackTab,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
}

impl fmt::Display for Key {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", key_text(*self))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyModifiers {
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
    };

    pub const SHIFT: Self = Self {
        shift: true,
        ctrl: false,
        alt: false,
    };

    pub const CTRL: Self = Self {
        shift: false,
        ctrl: true,
        alt: false,
    };

    pub const ALT: Self = Self {
        shift: false,
        ctrl: false,
        alt: true,
    };

    pub const CTRL_SHIFT: Self = Self {
        shift: true,
        ctrl: true,
        alt: false,
    };

    pub const ALT_SHIFT: Self = Self {
        shift: true,
        ctrl: false,
        alt: true,
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyParseError {
    EmptySequence,
    EmptyStroke,
    MissingKey,
    DuplicateModifier(String),
    UnknownModifier(String),
    UnknownKey(String),
    InvalidFunctionKey(String),
}

fn parse_key_stroke(input: &str) -> Result<KeyStroke, KeyParseError> {
    let mut modifiers = KeyModifiers::NONE;
    let mut key = None;

    for raw_part in input.split('+') {
        let part = raw_part.trim();
        if part.is_empty() {
            return Err(KeyParseError::MissingKey);
        }

        let normalized = normalize_token(part);
        match normalized.as_str() {
            "shift" => set_modifier(&mut modifiers.shift, part)?,
            "ctrl" | "control" => set_modifier(&mut modifiers.ctrl, part)?,
            "alt" | "meta" => set_modifier(&mut modifiers.alt, part)?,
            _ if key.is_none() => key = Some(parse_key(part, modifiers)?),
            _ => return Err(KeyParseError::UnknownModifier(part.to_string())),
        }
    }

    let key = key.ok_or(KeyParseError::MissingKey)?;
    Ok(KeyStroke { key, modifiers })
}

fn set_modifier(slot: &mut bool, raw: &str) -> Result<(), KeyParseError> {
    if *slot {
        Err(KeyParseError::DuplicateModifier(raw.to_string()))
    } else {
        *slot = true;
        Ok(())
    }
}

fn parse_key(input: &str, modifiers: KeyModifiers) -> Result<Key, KeyParseError> {
    let normalized = normalize_token(input);

    match normalized.as_str() {
        "enter" | "return" => Ok(Key::Enter),
        "esc" | "escape" => Ok(Key::Esc),
        "backspace" | "bs" => Ok(Key::Backspace),
        "delete" | "del" => Ok(Key::Delete),
        "insert" | "ins" => Ok(Key::Insert),
        "tab" => Ok(Key::Tab),
        "backtab" | "shift-tab" => Ok(Key::BackTab),
        "left" => Ok(Key::Left),
        "right" => Ok(Key::Right),
        "up" => Ok(Key::Up),
        "down" => Ok(Key::Down),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" | "pgup" => Ok(Key::PageUp),
        "pagedown" | "pgdn" => Ok(Key::PageDown),
        _ if is_function_key_name(&normalized) => parse_function_key(input, &normalized),
        _ => parse_char_key(input, modifiers),
    }
}

fn is_function_key_name(normalized: &str) -> bool {
    normalized.len() > 1
        && normalized.starts_with('f')
        && normalized[1..].chars().all(|ch| ch.is_ascii_digit())
}

fn parse_function_key(raw: &str, normalized: &str) -> Result<Key, KeyParseError> {
    let number = normalized[1..]
        .parse::<u8>()
        .map_err(|_| KeyParseError::InvalidFunctionKey(raw.to_string()))?;

    if (1..=24).contains(&number) {
        Ok(Key::F(number))
    } else {
        Err(KeyParseError::InvalidFunctionKey(raw.to_string()))
    }
}

fn parse_char_key(input: &str, modifiers: KeyModifiers) -> Result<Key, KeyParseError> {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return Err(KeyParseError::MissingKey);
    };

    if chars.next().is_some() {
        return Err(KeyParseError::UnknownKey(input.to_string()));
    }

    let ch = if first.is_ascii_alphabetic() && modifiers.shift {
        first.to_ascii_uppercase()
    } else if first.is_ascii_alphabetic() {
        first.to_ascii_lowercase()
    } else {
        first
    };

    Ok(Key::Char(ch))
}

fn normalize_token(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|ch| *ch != '-' && *ch != '_' && !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
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
        EditorCommand::App(AppCommand::CommandOutputStderr) => "app.command_output_stderr",
        EditorCommand::App(AppCommand::CommandOutputStdout) => "app.command_output_stdout",
        EditorCommand::App(AppCommand::CommandOutputSummary) => "app.command_output_summary",
        EditorCommand::App(AppCommand::CommandOutputSave) => "app.command_output_save",
        EditorCommand::App(AppCommand::ShellEscape) => "app.shell_escape",
        EditorCommand::App(AppCommand::StatusHistory) => "app.status_history",
        EditorCommand::App(AppCommand::Quit) => "app.quit",
    }
}

pub fn file_dialog_action_id(action: FileDialogAction) -> &'static str {
    match action {
        FileDialogAction::Cancel => "file_dialog.cancel",
        FileDialogAction::Submit => "file_dialog.submit",
        FileDialogAction::CompleteForward => "file_dialog.complete_forward",
        FileDialogAction::CompleteBackward => "file_dialog.complete_backward",
        FileDialogAction::ToggleHidden => "file_dialog.toggle_hidden",
        FileDialogAction::MoveSelectionUp => "file_dialog.move_selection_up",
        FileDialogAction::MoveSelectionDown => "file_dialog.move_selection_down",
        FileDialogAction::PageSelectionUp => "file_dialog.page_selection_up",
        FileDialogAction::PageSelectionDown => "file_dialog.page_selection_down",
        FileDialogAction::MoveInputLeft => "file_dialog.move_input_left",
        FileDialogAction::MoveInputRight => "file_dialog.move_input_right",
        FileDialogAction::MoveInputStart => "file_dialog.move_input_start",
        FileDialogAction::MoveInputEnd => "file_dialog.move_input_end",
        FileDialogAction::DeleteBackward => "file_dialog.delete_backward",
        FileDialogAction::DeleteForward => "file_dialog.delete_forward",
    }
}

pub fn file_dialog_action_from_id(input: &str) -> Result<FileDialogAction, CommandParseError> {
    match normalize_command_id(input).as_str() {
        "cancel" | "file_dialog.cancel" => Ok(FileDialogAction::Cancel),
        "submit" | "file_dialog.submit" => Ok(FileDialogAction::Submit),
        "complete_forward" | "file_dialog.complete_forward" => {
            Ok(FileDialogAction::CompleteForward)
        }
        "complete_backward" | "file_dialog.complete_backward" => {
            Ok(FileDialogAction::CompleteBackward)
        }
        "toggle_hidden" | "file_dialog.toggle_hidden" => Ok(FileDialogAction::ToggleHidden),
        "move_selection_up" | "file_dialog.move_selection_up" => {
            Ok(FileDialogAction::MoveSelectionUp)
        }
        "move_selection_down" | "file_dialog.move_selection_down" => {
            Ok(FileDialogAction::MoveSelectionDown)
        }
        "page_selection_up" | "file_dialog.page_selection_up" => {
            Ok(FileDialogAction::PageSelectionUp)
        }
        "page_selection_down" | "file_dialog.page_selection_down" => {
            Ok(FileDialogAction::PageSelectionDown)
        }
        "move_input_left" | "file_dialog.move_input_left" => Ok(FileDialogAction::MoveInputLeft),
        "move_input_right" | "file_dialog.move_input_right" => Ok(FileDialogAction::MoveInputRight),
        "move_input_start" | "file_dialog.move_input_start" => Ok(FileDialogAction::MoveInputStart),
        "move_input_end" | "file_dialog.move_input_end" => Ok(FileDialogAction::MoveInputEnd),
        "delete_backward" | "file_dialog.delete_backward" => Ok(FileDialogAction::DeleteBackward),
        "delete_forward" | "file_dialog.delete_forward" => Ok(FileDialogAction::DeleteForward),
        _ => Err(CommandParseError::UnknownCommand(input.to_string())),
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
        "app.command_output_stderr" => Ok(EditorCommand::App(AppCommand::CommandOutputStderr)),
        "app.command_output_stdout" => Ok(EditorCommand::App(AppCommand::CommandOutputStdout)),
        "app.command_output_summary" => Ok(EditorCommand::App(AppCommand::CommandOutputSummary)),
        "app.command_output_save" => Ok(EditorCommand::App(AppCommand::CommandOutputSave)),
        "app.shell_escape" => Ok(EditorCommand::App(AppCommand::ShellEscape)),
        "app.status_history" => Ok(EditorCommand::App(AppCommand::StatusHistory)),
        "app.quit" => Ok(EditorCommand::App(AppCommand::Quit)),
        _ => Err(CommandParseError::UnknownCommand(input.to_string())),
    }
}

fn normalize_command_id(input: &str) -> String {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub editable_file_soft_limit_bytes: u64,
    pub line_display_soft_limit_bytes: usize,
}

impl Limits {
    pub fn validate(self) -> Result<(), LimitsError> {
        if self.editable_file_soft_limit_bytes == 0 {
            return Err(LimitsError::EditableFileSoftLimitZero);
        }

        if self.line_display_soft_limit_bytes == 0 {
            return Err(LimitsError::LineDisplaySoftLimitZero);
        }

        Ok(())
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            editable_file_soft_limit_bytes: 16 * 1024 * 1024,
            line_display_soft_limit_bytes: 16 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitsError {
    EditableFileSoftLimitZero,
    LineDisplaySoftLimitZero,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();

        assert_eq!(config.theme, ThemeName::MsEdit);
        assert!(config.validate().is_ok());
        assert!(config.keybindings.bindings.len() > 10);
    }

    #[test]
    fn terminal_overrides_apply_to_detected_profile() {
        let overrides = TerminalOverrides {
            encoding: Some(EncodingProfile::Ascii),
            colors: Some(ColorProfile::Color16),
        };

        assert_eq!(
            overrides.apply_to(TerminalProfile::utf8_256()),
            TerminalProfile::ascii_16()
        );
    }

    #[test]
    fn config_resolves_theme_after_terminal_overrides() {
        let config = Config {
            terminal: TerminalOverrides {
                encoding: Some(EncodingProfile::Ascii),
                colors: Some(ColorProfile::Color16),
            },
            ..Config::default()
        };

        let theme = config.resolved_theme(TerminalProfile::utf8_256());

        assert_eq!(theme.colors, ColorProfile::Color16);
        assert_eq!(theme.name, "msedit");
    }

    #[test]
    fn parses_single_key_stroke_with_modifiers() {
        let stroke = KeyStroke::from_str("Ctrl+Alt+Q").unwrap();

        assert_eq!(
            stroke,
            KeyStroke::new(
                Key::Char('q'),
                KeyModifiers {
                    ctrl: true,
                    alt: true,
                    shift: false,
                },
            )
        );
    }

    #[test]
    fn parses_key_sequence() {
        let sequence = KeySequence::from_str("Ctrl+W, V").unwrap();

        assert_eq!(sequence.strokes.len(), 2);
        assert_eq!(sequence.strokes[0].key, Key::Char('w'));
        assert_eq!(sequence.strokes[1].key, Key::Char('v'));
    }

    #[test]
    fn parses_special_keys() {
        assert_eq!(
            KeyStroke::from_str("Alt+Shift+Left").unwrap(),
            KeyStroke::new(Key::Left, KeyModifiers::ALT_SHIFT)
        );
        assert_eq!(
            KeyStroke::from_str("F12").unwrap(),
            KeyStroke::plain(Key::F(12))
        );
        assert_eq!(
            KeyStroke::from_str("Esc").unwrap(),
            KeyStroke::plain(Key::Esc)
        );
    }

    #[test]
    fn rejects_unknown_key_names() {
        assert_eq!(
            KeyStroke::from_str("Ctrl+Hyper"),
            Err(KeyParseError::UnknownKey("Hyper".to_string()))
        );
    }

    #[test]
    fn keymap_finds_bound_command() {
        let keymap = Keymap::default_editor();
        let sequence = KeySequence::from_str("Ctrl+S").unwrap();

        assert_eq!(
            keymap.command_for_sequence(&sequence),
            Some(&EditorCommand::File(FileCommand::Save))
        );

        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Ctrl+G").unwrap()),
            Some(&EditorCommand::Edit(EditCommand::GoToLine))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("PageDown").unwrap()),
            Some(&EditorCommand::Edit(EditCommand::MovePageDown))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Shift+PageDown").unwrap()),
            Some(&EditorCommand::Edit(EditCommand::ExtendSelectionPageDown))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Ctrl+W,]").unwrap()),
            Some(&EditorCommand::Edit(EditCommand::ScrollRight))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Ctrl+Right").unwrap()),
            Some(&EditorCommand::Edit(EditCommand::MoveWordRight))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Ctrl+Delete").unwrap()),
            Some(&EditorCommand::Edit(EditCommand::DeleteWordForward))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Ctrl+Shift+Left").unwrap()),
            Some(&EditorCommand::Edit(EditCommand::ExtendSelectionWordLeft))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Ctrl+W,Ctrl+C").unwrap()),
            Some(&EditorCommand::Edit(EditCommand::CopyExternal))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("F2").unwrap()),
            Some(&EditorCommand::App(AppCommand::StatusHistory))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("F5").unwrap()),
            Some(&EditorCommand::App(AppCommand::ReloadConfig))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("F6").unwrap()),
            Some(&EditorCommand::App(AppCommand::ConfigDiagnostics))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Ctrl+W,O").unwrap()),
            Some(&EditorCommand::App(AppCommand::RunCommand))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Ctrl+W,S").unwrap()),
            Some(&EditorCommand::App(AppCommand::ShellEscape))
        );
        assert_eq!(
            keymap.sequence_for_command(&EditorCommand::File(FileCommand::Save)),
            Some(&sequence)
        );
    }

    #[test]
    fn default_keymap_has_mac_friendly_window_aliases() {
        let keymap = Keymap::default_editor();

        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Ctrl+W,Left").unwrap()),
            Some(&EditorCommand::Window(WindowCommand::FocusLeft))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Alt+Left").unwrap()),
            Some(&EditorCommand::Window(WindowCommand::FocusLeft))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Ctrl+W,Shift+Right").unwrap()),
            Some(&EditorCommand::Window(WindowCommand::ResizeRight))
        );
        assert_eq!(
            keymap.command_for_sequence(&KeySequence::from_str("Alt+Shift+Right").unwrap()),
            Some(&EditorCommand::Window(WindowCommand::ResizeRight))
        );
        assert_eq!(
            keymap.sequence_for_command(&EditorCommand::Window(WindowCommand::FocusLeft)),
            Some(&KeySequence::from_str("Ctrl+W,Left").unwrap())
        );
    }

    #[test]
    fn keymap_reports_sequence_prefixes() {
        let keymap = Keymap::default_editor();

        assert!(keymap.has_sequence_prefix(&KeySequence::from_str("Ctrl+W").unwrap()));
        assert!(!keymap.has_sequence_prefix(&KeySequence::from_str("Ctrl+W,H").unwrap()));
        assert!(!keymap.has_sequence_prefix(&KeySequence::from_str("Alt+Left").unwrap()));
    }

    #[test]
    fn keymap_rejects_duplicate_bindings() {
        let keymap = Keymap {
            bindings: vec![
                KeyBinding::new("Ctrl+S", EditorCommand::File(FileCommand::Save)),
                KeyBinding::new("Ctrl+S", EditorCommand::File(FileCommand::SaveAs)),
            ],
        };

        assert_eq!(
            keymap.validate(),
            Err(KeymapError::DuplicateBinding(
                KeySequence::from_str("Ctrl+S").unwrap()
            ))
        );
    }

    #[test]
    fn file_dialog_keymap_finds_bound_actions() {
        let keymap = FileDialogKeymap::default_file_dialog();

        assert_eq!(
            keymap.action_for_stroke(KeyStroke::from_str("Ctrl+H").unwrap()),
            Some(FileDialogAction::ToggleHidden)
        );
        assert_eq!(
            keymap.stroke_for_action(FileDialogAction::MoveInputStart),
            Some(KeyStroke::from_str("Home").unwrap())
        );
        assert_eq!(
            file_dialog_action_id(FileDialogAction::DeleteForward),
            "file_dialog.delete_forward"
        );
        assert_eq!(
            file_dialog_action_from_id("delete-forward"),
            Ok(FileDialogAction::DeleteForward)
        );
    }

    #[test]
    fn file_dialog_keymap_rejects_duplicate_bindings() {
        let keymap = FileDialogKeymap {
            bindings: vec![
                FileDialogKeyBinding::new("Esc", FileDialogAction::Cancel),
                FileDialogKeyBinding::new("Esc", FileDialogAction::Submit),
            ],
        };

        assert_eq!(
            keymap.validate(),
            Err(FileDialogKeymapError::DuplicateBinding(
                KeyStroke::from_str("Esc").unwrap()
            ))
        );
    }

    #[test]
    fn key_sequences_have_stable_display_text() {
        assert_eq!(
            KeySequence::from_str("Ctrl+W,H").unwrap().to_string(),
            "Ctrl+W,H"
        );
        assert_eq!(
            KeySequence::from_str("Alt+Shift+Left").unwrap().to_string(),
            "Alt+Shift+Left"
        );
    }

    #[test]
    fn command_ids_round_trip() {
        let command = EditorCommand::Window(WindowCommand::ToggleCollapse);
        let id = command_id(&command);

        assert_eq!(id, "window.toggle_collapse");
        assert_eq!(command_from_id(id), Ok(command));
        assert_eq!(
            command_from_id("app.reload_config"),
            Ok(EditorCommand::App(AppCommand::ReloadConfig))
        );
        assert_eq!(
            command_from_id("app.config_diagnostics"),
            Ok(EditorCommand::App(AppCommand::ConfigDiagnostics))
        );
        assert_eq!(
            command_from_id("edit.move-word-right"),
            Ok(EditorCommand::Edit(EditCommand::MoveWordRight))
        );
        assert_eq!(
            command_from_id("edit.extend_selection_page_down"),
            Ok(EditorCommand::Edit(EditCommand::ExtendSelectionPageDown))
        );
        assert_eq!(
            command_from_id("edit.scroll_right"),
            Ok(EditorCommand::Edit(EditCommand::ScrollRight))
        );
        assert_eq!(
            command_from_id("edit.delete_word_backward"),
            Ok(EditorCommand::Edit(EditCommand::DeleteWordBackward))
        );
        assert_eq!(
            command_from_id("edit.copy_external"),
            Ok(EditorCommand::Edit(EditCommand::CopyExternal))
        );
        assert_eq!(
            command_from_id("app.command_output_clear"),
            Ok(EditorCommand::App(AppCommand::CommandOutputClear))
        );
        assert_eq!(
            command_from_id("app.command_output_stdout"),
            Ok(EditorCommand::App(AppCommand::CommandOutputStdout))
        );
        assert_eq!(
            command_from_id("app.command_output_summary"),
            Ok(EditorCommand::App(AppCommand::CommandOutputSummary))
        );
        assert_eq!(
            command_from_id("app.command_output_save"),
            Ok(EditorCommand::App(AppCommand::CommandOutputSave))
        );
        assert_eq!(
            command_from_id("app.nope"),
            Err(CommandParseError::UnknownCommand("app.nope".to_string()))
        );
    }

    #[test]
    fn parses_line_based_config_overlay() {
        let config = parse_config(
            "\
# Dun config
theme = dark
terminal.encoding = ascii
terminal.colors = mono
mouse.enabled = true
clipboard.osc52.enabled = true
clipboard.osc52.max_bytes = 2 KiB
limits.editable_file_soft_limit_bytes = 2 MiB
limits.line_display_soft_limit_bytes = 4 KiB
key.app.quit = Esc
key.edit.find = none
key.file_dialog.toggle_hidden = F8
file_dialog.key.delete_forward = none
",
        )
        .unwrap();

        assert_eq!(config.theme, ThemeName::Dark);
        assert_eq!(config.terminal.encoding, Some(EncodingProfile::Ascii));
        assert_eq!(config.terminal.colors, Some(ColorProfile::Mono));
        assert!(config.mouse.enabled);
        assert!(config.clipboard.osc52.enabled);
        assert_eq!(config.clipboard.osc52.max_bytes, 2 * 1024);
        assert_eq!(
            config.limits.editable_file_soft_limit_bytes,
            2 * 1024 * 1024
        );
        assert_eq!(config.limits.line_display_soft_limit_bytes, 4 * 1024);
        assert_eq!(
            config
                .keybindings
                .command_for_sequence(&KeySequence::from_str("Esc").unwrap()),
            Some(&EditorCommand::App(AppCommand::Quit))
        );
        assert!(
            !config
                .keybindings
                .bindings
                .iter()
                .any(|binding| binding.command == EditorCommand::Edit(EditCommand::Find))
        );
        assert_eq!(
            config
                .file_dialog_keys
                .action_for_stroke(KeyStroke::from_str("F8").unwrap()),
            Some(FileDialogAction::ToggleHidden)
        );
        assert_eq!(
            config
                .file_dialog_keys
                .stroke_for_action(FileDialogAction::DeleteForward),
            None
        );
    }

    #[test]
    fn default_config_keeps_mouse_disabled() {
        assert!(!Config::default().mouse.enabled);
    }

    #[test]
    fn default_config_text_lists_parseable_default_bindings() {
        let text = default_config_text();

        assert!(text.contains("theme = msedit"));
        assert!(text.contains("mouse.enabled = false"));
        assert!(text.contains("key.app.help = F1"));
        assert!(text.contains("key.file_dialog.toggle_hidden = Ctrl+H"));
        parse_config(&text).unwrap().validate().unwrap();
    }

    #[test]
    fn config_parser_reports_line_errors() {
        let error = parse_config("bad = value").unwrap_err();

        assert_eq!(error.line, Some(1));
        assert!(error.to_string().contains("unknown config key"));

        let error = parse_config("key.app.nope = Ctrl+X").unwrap_err();

        assert_eq!(error.line, Some(1));
        assert!(error.to_string().contains("unknown command id"));

        let error = parse_config("key.file_dialog.nope = Ctrl+X").unwrap_err();

        assert_eq!(error.line, Some(1));
        assert!(error.to_string().contains("unknown file dialog action"));

        let error = parse_config("key.file_dialog.cancel = Ctrl+W,Q").unwrap_err();

        assert_eq!(error.line, Some(1));
        assert!(error.to_string().contains("single key stroke"));

        let error = parse_config("mouse.enabled = maybe").unwrap_err();

        assert_eq!(error.line, Some(1));
        assert!(error.to_string().contains("expected true or false"));
    }

    #[test]
    fn config_parser_rejects_duplicate_resulting_keybindings() {
        let error = parse_config("key.app.quit = Ctrl+S").unwrap_err();

        assert_eq!(error.line, None);
        assert!(error.to_string().contains("duplicate key sequence"));

        let error = parse_config("key.file_dialog.cancel = Enter").unwrap_err();

        assert_eq!(error.line, None);
        assert!(error.to_string().contains("duplicate key stroke"));
    }

    #[test]
    fn config_parser_replaces_all_default_aliases_for_command() {
        let config = parse_config("key.window.focus_left = Ctrl+W,A").unwrap();

        assert_eq!(
            config
                .keybindings
                .command_for_sequence(&KeySequence::from_str("Ctrl+W,A").unwrap()),
            Some(&EditorCommand::Window(WindowCommand::FocusLeft))
        );
        assert_eq!(
            config
                .keybindings
                .command_for_sequence(&KeySequence::from_str("Ctrl+W,Left").unwrap()),
            None
        );
        assert_eq!(
            config
                .keybindings
                .command_for_sequence(&KeySequence::from_str("Alt+Left").unwrap()),
            None
        );
    }

    #[test]
    fn limits_reject_zero_values() {
        assert_eq!(
            Limits {
                line_display_soft_limit_bytes: 0,
                ..Limits::default()
            }
            .validate(),
            Err(LimitsError::LineDisplaySoftLimitZero)
        );
    }
}
