use super::TerminalColor;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StyleAttrs {
    pub bold: bool,
    pub underline: bool,
    pub reverse: bool,
}

impl StyleAttrs {
    pub const NONE: Self = Self {
        bold: false,
        underline: false,
        reverse: false,
    };

    pub const BOLD: Self = Self {
        bold: true,
        underline: false,
        reverse: false,
    };

    pub const UNDERLINE: Self = Self {
        bold: false,
        underline: true,
        reverse: false,
    };

    pub const REVERSE: Self = Self {
        bold: false,
        underline: false,
        reverse: true,
    };

    pub const BOLD_REVERSE: Self = Self {
        bold: true,
        underline: false,
        reverse: true,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub fg: TerminalColor,
    pub bg: TerminalColor,
    pub attrs: StyleAttrs,
}

impl Style {
    pub const fn new(fg: TerminalColor, bg: TerminalColor, attrs: StyleAttrs) -> Self {
        Self { fg, bg, attrs }
    }

    pub const fn plain(fg: TerminalColor, bg: TerminalColor) -> Self {
        Self::new(fg, bg, StyleAttrs::NONE)
    }
}
