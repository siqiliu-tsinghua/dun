mod builtins;
mod color;
mod palette;
mod style;

use crate::profile::ColorProfile;

pub use color::{AnsiColor, TerminalColor};
pub use palette::Palette;
pub use style::{Style, StyleAttrs};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeName {
    MsEdit,
    Turbo,
    Dark,
    Dun,
}

impl ThemeName {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MsEdit => "msedit",
            Self::Turbo => "turbo",
            Self::Dark => "dark",
            Self::Dun => "dun",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub theme: ThemeName,
    pub colors: ColorProfile,
    pub palette: Palette,
}

#[cfg(test)]
mod tests;
