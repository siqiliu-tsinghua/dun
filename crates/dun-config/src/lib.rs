#![forbid(unsafe_code)]

pub use dun_term::{ColorProfile, EncodingProfile, TerminalProfile, ThemeName};

mod commands;
mod config;
mod defaults;
mod keys;
mod limits;
mod parser;
mod validation;

pub(crate) use commands::normalize_command_id;
pub use commands::{CommandParseError, command_from_id, command_id};
pub use config::{
    ClipboardConfig, Config, ConfigError, MouseConfig, Osc52Config, TerminalOverrides,
};
pub use defaults::default_config_text;
pub use keys::{
    FileDialogAction, FileDialogKeyBinding, FileDialogKeymap, FileDialogKeymapError, Key,
    KeyBinding, KeyModifiers, KeyParseError, KeySequence, KeyStroke, Keymap, KeymapError,
    file_dialog_action_from_id, file_dialog_action_id,
};
pub use limits::{Limits, LimitsError};
pub use parser::{ConfigParseError, parse_config, parse_config_overlay};

#[cfg(test)]
mod tests;
