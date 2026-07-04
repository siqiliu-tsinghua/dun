#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::str::FromStr;

use dun_core::{AppCommand, EditCommand, EditorCommand, FileCommand, WindowCommand};
use dun_term::Theme;
pub use dun_term::{ColorProfile, EncodingProfile, TerminalProfile, ThemeName};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub theme: ThemeName,
    pub terminal: TerminalOverrides,
    pub keybindings: Keymap,
    pub limits: Limits,
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.keybindings.validate()?;
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
            keybindings: Keymap::default(),
            limits: Limits::default(),
        }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    Keymap(KeymapError),
    Limits(LimitsError),
}

impl From<KeymapError> for ConfigError {
    fn from(error: KeymapError) -> Self {
        Self::Keymap(error)
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
                KeyBinding::new("Ctrl+Q", EditorCommand::App(AppCommand::Quit)),
                KeyBinding::new("Ctrl+P", EditorCommand::App(AppCommand::CommandLine)),
                KeyBinding::new("Ctrl+N", EditorCommand::File(FileCommand::New)),
                KeyBinding::new("Ctrl+O", EditorCommand::File(FileCommand::Open)),
                KeyBinding::new("Ctrl+S", EditorCommand::File(FileCommand::Save)),
                KeyBinding::new("Ctrl+Shift+S", EditorCommand::File(FileCommand::SaveAs)),
                KeyBinding::new("Ctrl+W,Q", EditorCommand::File(FileCommand::Close)),
                KeyBinding::new("Ctrl+Z", EditorCommand::Edit(EditCommand::Undo)),
                KeyBinding::new("Ctrl+Y", EditorCommand::Edit(EditCommand::Redo)),
                KeyBinding::new("Ctrl+F", EditorCommand::Edit(EditCommand::Find)),
                KeyBinding::new("F3", EditorCommand::Edit(EditCommand::FindNext)),
                KeyBinding::new("Shift+F3", EditorCommand::Edit(EditCommand::FindPrevious)),
                KeyBinding::new("Ctrl+R", EditorCommand::Edit(EditCommand::Replace)),
                KeyBinding::new("Ctrl+A", EditorCommand::Edit(EditCommand::SelectAll)),
                KeyBinding::new("Left", EditorCommand::Edit(EditCommand::MoveLeft)),
                KeyBinding::new("Right", EditorCommand::Edit(EditCommand::MoveRight)),
                KeyBinding::new("Up", EditorCommand::Edit(EditCommand::MoveUp)),
                KeyBinding::new("Down", EditorCommand::Edit(EditCommand::MoveDown)),
                KeyBinding::new("Home", EditorCommand::Edit(EditCommand::MoveLineStart)),
                KeyBinding::new("End", EditorCommand::Edit(EditCommand::MoveLineEnd)),
                KeyBinding::new("Enter", EditorCommand::Edit(EditCommand::InsertNewline)),
                KeyBinding::new(
                    "Backspace",
                    EditorCommand::Edit(EditCommand::DeleteBackward),
                ),
                KeyBinding::new("Delete", EditorCommand::Edit(EditCommand::DeleteForward)),
                KeyBinding::new(
                    "Ctrl+W,H",
                    EditorCommand::Window(WindowCommand::SplitHorizontal),
                ),
                KeyBinding::new(
                    "Ctrl+W,V",
                    EditorCommand::Window(WindowCommand::SplitVertical),
                ),
                KeyBinding::new("Alt+Left", EditorCommand::Window(WindowCommand::FocusLeft)),
                KeyBinding::new(
                    "Alt+Right",
                    EditorCommand::Window(WindowCommand::FocusRight),
                ),
                KeyBinding::new("Alt+Up", EditorCommand::Window(WindowCommand::FocusUp)),
                KeyBinding::new("Alt+Down", EditorCommand::Window(WindowCommand::FocusDown)),
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

    pub fn has_sequence_prefix(&self, sequence: &KeySequence) -> bool {
        if sequence.strokes.is_empty() {
            return false;
        }

        self.bindings.iter().any(|binding| {
            binding.sequence.strokes.len() > sequence.strokes.len()
                && binding.sequence.strokes.starts_with(&sequence.strokes)
        })
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::default_editor()
    }
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
        EditorCommand::File(FileCommand::Save) => "file.save",
        EditorCommand::File(FileCommand::SaveAs) => "file.save_as",
        EditorCommand::File(FileCommand::Close) => "file.close",
        EditorCommand::Edit(EditCommand::Undo) => "edit.undo",
        EditorCommand::Edit(EditCommand::Redo) => "edit.redo",
        EditorCommand::Edit(EditCommand::Cut) => "edit.cut",
        EditorCommand::Edit(EditCommand::Copy) => "edit.copy",
        EditorCommand::Edit(EditCommand::Paste) => "edit.paste",
        EditorCommand::Edit(EditCommand::SelectAll) => "edit.select_all",
        EditorCommand::Edit(EditCommand::MoveLeft) => "edit.move_left",
        EditorCommand::Edit(EditCommand::MoveRight) => "edit.move_right",
        EditorCommand::Edit(EditCommand::MoveUp) => "edit.move_up",
        EditorCommand::Edit(EditCommand::MoveDown) => "edit.move_down",
        EditorCommand::Edit(EditCommand::MoveLineStart) => "edit.move_line_start",
        EditorCommand::Edit(EditCommand::MoveLineEnd) => "edit.move_line_end",
        EditorCommand::Edit(EditCommand::InsertNewline) => "edit.insert_newline",
        EditorCommand::Edit(EditCommand::DeleteBackward) => "edit.delete_backward",
        EditorCommand::Edit(EditCommand::DeleteForward) => "edit.delete_forward",
        EditorCommand::Edit(EditCommand::Find) => "edit.find",
        EditorCommand::Edit(EditCommand::FindNext) => "edit.find_next",
        EditorCommand::Edit(EditCommand::FindPrevious) => "edit.find_previous",
        EditorCommand::Edit(EditCommand::Replace) => "edit.replace",
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
        EditorCommand::App(AppCommand::Help) => "app.help",
        EditorCommand::App(AppCommand::Quit) => "app.quit",
    }
}

pub fn command_from_id(input: &str) -> Result<EditorCommand, CommandParseError> {
    match normalize_command_id(input).as_str() {
        "file.new" => Ok(EditorCommand::File(FileCommand::New)),
        "file.open" => Ok(EditorCommand::File(FileCommand::Open)),
        "file.save" => Ok(EditorCommand::File(FileCommand::Save)),
        "file.save_as" => Ok(EditorCommand::File(FileCommand::SaveAs)),
        "file.close" => Ok(EditorCommand::File(FileCommand::Close)),
        "edit.undo" => Ok(EditorCommand::Edit(EditCommand::Undo)),
        "edit.redo" => Ok(EditorCommand::Edit(EditCommand::Redo)),
        "edit.cut" => Ok(EditorCommand::Edit(EditCommand::Cut)),
        "edit.copy" => Ok(EditorCommand::Edit(EditCommand::Copy)),
        "edit.paste" => Ok(EditorCommand::Edit(EditCommand::Paste)),
        "edit.select_all" => Ok(EditorCommand::Edit(EditCommand::SelectAll)),
        "edit.move_left" => Ok(EditorCommand::Edit(EditCommand::MoveLeft)),
        "edit.move_right" => Ok(EditorCommand::Edit(EditCommand::MoveRight)),
        "edit.move_up" => Ok(EditorCommand::Edit(EditCommand::MoveUp)),
        "edit.move_down" => Ok(EditorCommand::Edit(EditCommand::MoveDown)),
        "edit.move_line_start" => Ok(EditorCommand::Edit(EditCommand::MoveLineStart)),
        "edit.move_line_end" => Ok(EditorCommand::Edit(EditCommand::MoveLineEnd)),
        "edit.insert_newline" => Ok(EditorCommand::Edit(EditCommand::InsertNewline)),
        "edit.delete_backward" => Ok(EditorCommand::Edit(EditCommand::DeleteBackward)),
        "edit.delete_forward" => Ok(EditorCommand::Edit(EditCommand::DeleteForward)),
        "edit.find" => Ok(EditorCommand::Edit(EditCommand::Find)),
        "edit.find_next" => Ok(EditorCommand::Edit(EditCommand::FindNext)),
        "edit.find_previous" => Ok(EditorCommand::Edit(EditCommand::FindPrevious)),
        "edit.replace" => Ok(EditorCommand::Edit(EditCommand::Replace)),
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
        "app.help" => Ok(EditorCommand::App(AppCommand::Help)),
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
    fn command_ids_round_trip() {
        let command = EditorCommand::Window(WindowCommand::ToggleCollapse);
        let id = command_id(&command);

        assert_eq!(id, "window.toggle_collapse");
        assert_eq!(command_from_id(id), Ok(command));
        assert_eq!(
            command_from_id("app.nope"),
            Err(CommandParseError::UnknownCommand("app.nope".to_string()))
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
