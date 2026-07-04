#![forbid(unsafe_code)]

use dun_term::Theme;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub theme: Theme,
    pub keybindings: Keymap,
    pub limits: Limits,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            keybindings: Keymap::default(),
            limits: Limits::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keymap {
    pub bindings: Vec<KeyBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    pub key: String,
    pub command: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub editable_file_soft_limit_bytes: u64,
    pub line_display_soft_limit_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            editable_file_soft_limit_bytes: 16 * 1024 * 1024,
            line_display_soft_limit_bytes: 16 * 1024,
        }
    }
}
