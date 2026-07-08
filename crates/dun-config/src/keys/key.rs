use std::fmt;
use std::str::FromStr;

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

pub(super) fn key_stroke_text(stroke: &KeyStroke) -> String {
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

pub(crate) fn normalize_token(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|ch| *ch != '-' && *ch != '_' && !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}
