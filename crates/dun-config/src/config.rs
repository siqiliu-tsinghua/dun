use dun_term::{ColorProfile, EncodingProfile, TerminalProfile, Theme, ThemeName};

use crate::plugins::validate_plugin_entries;
use crate::{FileDialogKeymap, FileDialogKeymapError, Keymap, KeymapError, Limits, LimitsError};
use crate::{PluginConfigError, PluginEntry};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub theme: ThemeName,
    pub terminal: TerminalOverrides,
    pub mouse: MouseConfig,
    pub clipboard: ClipboardConfig,
    pub keybindings: Keymap,
    pub file_dialog_keys: FileDialogKeymap,
    pub limits: Limits,
    pub plugins: Vec<PluginEntry>,
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.keybindings.validate()?;
        self.file_dialog_keys.validate()?;
        self.limits.validate()?;
        validate_plugin_entries(&self.plugins)?;
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
            theme: ThemeName::Dun,
            terminal: TerminalOverrides::default(),
            mouse: MouseConfig::default(),
            clipboard: ClipboardConfig::default(),
            keybindings: Keymap::default(),
            file_dialog_keys: FileDialogKeymap::default(),
            limits: Limits::default(),
            plugins: Vec::new(),
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseConfig {
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClipboardConfig {
    pub osc52: Osc52Config,
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
    Plugins(PluginConfigError),
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

impl From<PluginConfigError> for ConfigError {
    fn from(error: PluginConfigError) -> Self {
        Self::Plugins(error)
    }
}
