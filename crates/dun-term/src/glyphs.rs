use crate::profile::TerminalProfile;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderGlyphs {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,
    pub left_tee: char,
    pub right_tee: char,
    pub top_tee: char,
    pub bottom_tee: char,
    pub cross: char,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndicatorGlyphs {
    pub focused: char,
    pub dirty: char,
    pub read_only: char,
    pub collapsed: char,
    pub ellipsis: char,
    pub truncation: char,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphSet {
    pub border: BorderGlyphs,
    pub indicators: IndicatorGlyphs,
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
                left_tee: '├',
                right_tee: '┤',
                top_tee: '┬',
                bottom_tee: '┴',
                cross: '┼',
            },
            indicators: IndicatorGlyphs {
                focused: '◆',
                dirty: '●',
                read_only: '■',
                collapsed: '▶',
                ellipsis: '…',
                truncation: '…',
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
                left_tee: '+',
                right_tee: '+',
                top_tee: '+',
                bottom_tee: '+',
                cross: '+',
            },
            indicators: IndicatorGlyphs {
                focused: '*',
                dirty: '*',
                read_only: '#',
                collapsed: '>',
                ellipsis: '.',
                truncation: '~',
            },
        }
    }

    pub const fn for_profile(profile: TerminalProfile) -> Self {
        if profile.supports_unicode_glyphs() {
            Self::unicode_single_line()
        } else {
            Self::ascii()
        }
    }
}

impl Default for GlyphSet {
    fn default() -> Self {
        Self::unicode_single_line()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{ColorProfile, EncodingProfile};

    #[test]
    fn default_glyphs_are_unicode_single_line() {
        let glyphs = GlyphSet::default();

        assert_eq!(glyphs.border.top_left, '┌');
        assert_eq!(glyphs.border.cross, '┼');
        assert_eq!(glyphs.indicators.dirty, '●');
    }

    #[test]
    fn ascii_glyphs_are_plain_terminal_safe() {
        let glyphs = GlyphSet::ascii();

        assert_eq!(glyphs.border.top_left, '+');
        assert_eq!(glyphs.border.horizontal, '-');
        assert_eq!(glyphs.border.vertical, '|');
        assert_eq!(glyphs.border.cross, '+');
        assert_eq!(glyphs.indicators.collapsed, '>');
    }

    #[test]
    fn profile_selects_ascii_glyphs_when_utf8_is_unavailable() {
        let profile = TerminalProfile::new(EncodingProfile::Ascii, ColorProfile::Color16);

        assert_eq!(GlyphSet::for_profile(profile), GlyphSet::ascii());
    }
}
