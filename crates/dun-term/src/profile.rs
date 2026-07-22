#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodingProfile {
    Utf8,
    Ascii,
}

impl EncodingProfile {
    pub const fn supports_utf8(self) -> bool {
        matches!(self, Self::Utf8)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorProfile {
    Color256,
    Color16,
    Mono,
}

impl ColorProfile {
    pub const fn is_color(self) -> bool {
        !matches!(self, Self::Mono)
    }
}

/// How the terminal renders Unicode *East Asian Ambiguous*-width characters
/// (the box-drawing block, `◆`, etc.). `Narrow` is the Western/default reading
/// (1 column); `Wide` is the East-Asian reading (2 columns) that Solaris tmux
/// and CJK-configured terminals use. `dun` lays out and renders to match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AmbiguousWidth {
    #[default]
    Narrow,
    Wide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalProfile {
    pub encoding: EncodingProfile,
    pub colors: ColorProfile,
    pub ambiguous_width: AmbiguousWidth,
}

impl TerminalProfile {
    pub const fn new(encoding: EncodingProfile, colors: ColorProfile) -> Self {
        Self {
            encoding,
            colors,
            ambiguous_width: AmbiguousWidth::Narrow,
        }
    }

    /// Return a copy with the ambiguous-width reading set. Kept separate from
    /// `new` so the many `new(encoding, colors)` call sites stay unchanged.
    pub const fn with_ambiguous_width(self, ambiguous_width: AmbiguousWidth) -> Self {
        Self {
            encoding: self.encoding,
            colors: self.colors,
            ambiguous_width,
        }
    }

    pub const fn utf8_256() -> Self {
        Self::new(EncodingProfile::Utf8, ColorProfile::Color256)
    }

    pub const fn utf8_16() -> Self {
        Self::new(EncodingProfile::Utf8, ColorProfile::Color16)
    }

    pub const fn ascii_16() -> Self {
        Self::new(EncodingProfile::Ascii, ColorProfile::Color16)
    }

    pub const fn ascii_mono() -> Self {
        Self::new(EncodingProfile::Ascii, ColorProfile::Mono)
    }

    pub const fn vt100() -> Self {
        Self::ascii_16()
    }

    pub const fn supports_unicode_glyphs(self) -> bool {
        self.encoding.supports_utf8()
    }

    pub fn from_capabilities(
        term: Option<&str>,
        colorterm: Option<&str>,
        lang: Option<&str>,
        lc_ctype: Option<&str>,
        no_color: bool,
    ) -> Self {
        let encoding = detect_encoding(term, lang, lc_ctype);
        let colors = detect_colors(term, colorterm, no_color);
        // Stage A ships Narrow by default; runtime auto-detection is a later
        // step. The config option `terminal.ambiguous-width = wide` opts in.
        Self {
            encoding,
            colors,
            ambiguous_width: AmbiguousWidth::Narrow,
        }
    }
}

impl Default for TerminalProfile {
    fn default() -> Self {
        Self::utf8_256()
    }
}

fn detect_encoding(
    term: Option<&str>,
    lang: Option<&str>,
    lc_ctype: Option<&str>,
) -> EncodingProfile {
    if is_low_capability_term(term) {
        return EncodingProfile::Ascii;
    }

    match lc_ctype.or(lang).map(str::to_ascii_lowercase) {
        Some(locale) if locale.contains("utf-8") || locale.contains("utf8") => {
            EncodingProfile::Utf8
        }
        Some(locale) if locale == "c" || locale == "posix" => EncodingProfile::Ascii,
        Some(locale) if locale.contains('.') => EncodingProfile::Ascii,
        _ => EncodingProfile::Utf8,
    }
}

fn detect_colors(term: Option<&str>, colorterm: Option<&str>, no_color: bool) -> ColorProfile {
    if no_color {
        return ColorProfile::Mono;
    }

    let term = term.unwrap_or_default().to_ascii_lowercase();
    let colorterm = colorterm.unwrap_or_default().to_ascii_lowercase();

    if term == "dumb" {
        return ColorProfile::Mono;
    }

    if term.contains("256color")
        || colorterm.contains("truecolor")
        || colorterm.contains("24bit")
        || colorterm.contains("256")
    {
        return ColorProfile::Color256;
    }

    if term.contains("color")
        || term.contains("ansi")
        || term.contains("xterm")
        || term.contains("screen")
        || term.contains("tmux")
        || term.contains("vt100")
    {
        return ColorProfile::Color16;
    }

    ColorProfile::Color256
}

fn is_low_capability_term(term: Option<&str>) -> bool {
    let term = term.unwrap_or_default().to_ascii_lowercase();
    matches!(term.as_str(), "dumb" | "vt100" | "vt102" | "ansi")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_is_utf8_256_color() {
        assert_eq!(TerminalProfile::default(), TerminalProfile::utf8_256());
    }

    #[test]
    fn vt100_profile_uses_ascii_and_16_colors() {
        assert_eq!(TerminalProfile::vt100(), TerminalProfile::ascii_16());
        assert!(!TerminalProfile::vt100().supports_unicode_glyphs());
    }

    #[test]
    fn detects_utf8_256_color_terminal() {
        let profile = TerminalProfile::from_capabilities(
            Some("xterm-256color"),
            None,
            Some("en_US.UTF-8"),
            None,
            false,
        );

        assert_eq!(profile, TerminalProfile::utf8_256());
    }

    #[test]
    fn detects_ascii_locale_and_16_color_terminal() {
        let profile =
            TerminalProfile::from_capabilities(Some("xterm-color"), None, Some("C"), None, false);

        assert_eq!(profile, TerminalProfile::ascii_16());
    }

    #[test]
    fn no_color_forces_mono() {
        let profile = TerminalProfile::from_capabilities(
            Some("xterm-256color"),
            None,
            Some("en_US.UTF-8"),
            None,
            true,
        );

        assert_eq!(
            profile,
            TerminalProfile::new(EncodingProfile::Utf8, ColorProfile::Mono)
        );
    }
}
