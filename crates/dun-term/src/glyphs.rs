#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderGlyphs {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphSet {
    pub border: BorderGlyphs,
}

impl GlyphSet {
    pub const fn unicode_single_line() -> Self {
        Self {
            border: BorderGlyphs {
                top_left: '┌',
                top_right: '┐',
                bottom_left: '└',
                bottom_right: '┘',
                horizontal: '─',
                vertical: '│',
            },
        }
    }

    pub const fn ascii() -> Self {
        Self {
            border: BorderGlyphs {
                top_left: '+',
                top_right: '+',
                bottom_left: '+',
                bottom_right: '+',
                horizontal: '-',
                vertical: '|',
            },
        }
    }
}

impl Default for GlyphSet {
    fn default() -> Self {
        Self::unicode_single_line()
    }
}
