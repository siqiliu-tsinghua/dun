use crate::profile::{ColorProfile, TerminalProfile};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnsiColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalColor {
    Default,
    Ansi(AnsiColor),
    Indexed(u8),
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub editor: Style,
    pub editor_text: Style,
    pub menu_bar: Style,
    pub menu_text: Style,
    pub menu_hotkey: Style,
    pub menu_active: Style,
    pub menu_active_hotkey: Style,
    pub menu_panel: Style,
    pub menu_panel_text: Style,
    pub menu_panel_hotkey: Style,
    pub menu_panel_border: Style,
    pub status_bar: Style,
    pub status_text: Style,
    pub window_border: Style,
    pub window_border_focused: Style,
    pub title: Style,
    pub title_focused: Style,
    pub gutter: Style,
    pub gutter_separator: Style,
    pub current_line: Style,
    pub selection: Style,
    pub selection_text: Style,
    pub modal_scrim: Style,
    pub modal: Style,
    pub modal_text: Style,
    pub modal_border: Style,
    pub modal_input: Style,
    pub dirty: Style,
    pub read_only: Style,
    pub control: Style,
    pub escape: Style,
    pub truncation: Style,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub theme: ThemeName,
    pub colors: ColorProfile,
    pub palette: Palette,
}

impl Theme {
    pub const fn msedit() -> Self {
        Self::msedit_256()
    }

    pub const fn msedit_256() -> Self {
        let editor_bg = TerminalColor::Indexed(235);
        let editor_fg = TerminalColor::Indexed(252);
        let chrome_bg = TerminalColor::Indexed(67);
        let chrome_fg = TerminalColor::Indexed(255);
        let active_menu_bg = TerminalColor::Indexed(108);
        let active_menu_fg = TerminalColor::Indexed(235);
        let panel_bg = TerminalColor::Indexed(242);
        let panel_fg = TerminalColor::Indexed(255);
        let panel_border = TerminalColor::Indexed(253);
        let input_bg = TerminalColor::Indexed(235);
        let current_line_bg = TerminalColor::Indexed(240);
        let accent = TerminalColor::Indexed(255);

        Self {
            name: "msedit",
            theme: ThemeName::MsEdit,
            colors: ColorProfile::Color256,
            palette: Palette {
                editor: Style::plain(editor_fg, editor_bg),
                editor_text: Style::plain(editor_fg, editor_bg),
                menu_bar: Style::plain(chrome_fg, chrome_bg),
                menu_text: Style::plain(chrome_fg, chrome_bg),
                menu_hotkey: Style::new(chrome_fg, chrome_bg, StyleAttrs::BOLD),
                menu_active: Style::plain(active_menu_fg, active_menu_bg),
                menu_active_hotkey: Style::new(active_menu_fg, active_menu_bg, StyleAttrs::BOLD),
                menu_panel: Style::plain(panel_fg, panel_bg),
                menu_panel_text: Style::plain(panel_fg, panel_bg),
                menu_panel_hotkey: Style::new(panel_fg, panel_bg, StyleAttrs::UNDERLINE),
                menu_panel_border: Style::plain(panel_border, panel_bg),
                status_bar: Style::plain(chrome_fg, chrome_bg),
                status_text: Style::plain(chrome_fg, chrome_bg),
                window_border: Style::plain(panel_border, editor_bg),
                window_border_focused: Style::new(panel_border, editor_bg, StyleAttrs::BOLD),
                title: Style::plain(TerminalColor::Indexed(153), editor_bg),
                title_focused: Style::new(accent, editor_bg, StyleAttrs::BOLD),
                gutter: Style::plain(TerminalColor::Indexed(110), editor_bg),
                gutter_separator: Style::plain(panel_border, editor_bg),
                current_line: Style::plain(editor_fg, current_line_bg),
                selection: Style::plain(editor_fg, TerminalColor::Indexed(24)),
                selection_text: Style::plain(editor_fg, TerminalColor::Indexed(24)),
                modal_scrim: Style::plain(TerminalColor::Indexed(245), editor_bg),
                modal: Style::plain(panel_fg, panel_bg),
                modal_text: Style::plain(panel_fg, panel_bg),
                modal_border: Style::plain(panel_border, panel_bg),
                modal_input: Style::plain(panel_fg, input_bg),
                dirty: Style::new(accent, editor_bg, StyleAttrs::BOLD),
                read_only: Style::plain(TerminalColor::Indexed(203), editor_bg),
                control: Style::plain(TerminalColor::Indexed(214), editor_bg),
                escape: Style::new(TerminalColor::Indexed(203), editor_bg, StyleAttrs::BOLD),
                truncation: Style::new(TerminalColor::Indexed(245), editor_bg, StyleAttrs::BOLD),
            },
        }
    }

    pub const fn msedit_16() -> Self {
        let editor_bg = TerminalColor::Ansi(AnsiColor::Blue);
        let editor_fg = TerminalColor::Ansi(AnsiColor::BrightWhite);

        Self {
            name: "msedit",
            theme: ThemeName::MsEdit,
            colors: ColorProfile::Color16,
            palette: Palette {
                editor: Style::plain(editor_fg, editor_bg),
                editor_text: Style::plain(editor_fg, editor_bg),
                menu_bar: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::Blue),
                ),
                menu_text: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::Blue),
                ),
                menu_hotkey: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
                    TerminalColor::Ansi(AnsiColor::Blue),
                    StyleAttrs::BOLD,
                ),
                menu_active: Style::plain(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::Green),
                ),
                menu_active_hotkey: Style::new(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::Green),
                    StyleAttrs::BOLD,
                ),
                menu_panel: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                menu_panel_text: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                menu_panel_hotkey: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                    StyleAttrs::UNDERLINE,
                ),
                menu_panel_border: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                status_bar: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::Blue),
                ),
                status_text: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::Blue),
                ),
                window_border: Style::plain(TerminalColor::Ansi(AnsiColor::Cyan), editor_bg),
                window_border_focused: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
                    editor_bg,
                    StyleAttrs::BOLD,
                ),
                title: Style::plain(TerminalColor::Ansi(AnsiColor::BrightCyan), editor_bg),
                title_focused: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
                    editor_bg,
                    StyleAttrs::BOLD,
                ),
                gutter: Style::plain(TerminalColor::Ansi(AnsiColor::BrightBlack), editor_bg),
                gutter_separator: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    editor_bg,
                ),
                current_line: Style::plain(editor_fg, TerminalColor::Ansi(AnsiColor::BrightBlack)),
                selection: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::Magenta),
                ),
                selection_text: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::Magenta),
                ),
                modal_scrim: Style::plain(TerminalColor::Ansi(AnsiColor::BrightBlack), editor_bg),
                modal: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                modal_text: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                modal_border: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                modal_input: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::Black),
                ),
                dirty: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
                    editor_bg,
                    StyleAttrs::BOLD,
                ),
                read_only: Style::plain(TerminalColor::Ansi(AnsiColor::BrightRed), editor_bg),
                control: Style::plain(TerminalColor::Ansi(AnsiColor::Yellow), editor_bg),
                escape: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightRed),
                    editor_bg,
                    StyleAttrs::BOLD,
                ),
                truncation: Style::new(
                    TerminalColor::Ansi(AnsiColor::White),
                    editor_bg,
                    StyleAttrs::BOLD,
                ),
            },
        }
    }

    pub const fn mono() -> Self {
        let plain = Style::plain(TerminalColor::Default, TerminalColor::Default);
        let reverse = Style::new(
            TerminalColor::Default,
            TerminalColor::Default,
            StyleAttrs::REVERSE,
        );
        let bold = Style::new(
            TerminalColor::Default,
            TerminalColor::Default,
            StyleAttrs::BOLD,
        );

        Self {
            name: "mono",
            theme: ThemeName::MsEdit,
            colors: ColorProfile::Mono,
            palette: Palette {
                editor: plain,
                editor_text: plain,
                menu_bar: reverse,
                menu_text: reverse,
                menu_hotkey: Style::new(
                    TerminalColor::Default,
                    TerminalColor::Default,
                    StyleAttrs::BOLD_REVERSE,
                ),
                menu_active: Style::new(
                    TerminalColor::Default,
                    TerminalColor::Default,
                    StyleAttrs::BOLD_REVERSE,
                ),
                menu_active_hotkey: Style::new(
                    TerminalColor::Default,
                    TerminalColor::Default,
                    StyleAttrs::BOLD_REVERSE,
                ),
                menu_panel: plain,
                menu_panel_text: plain,
                menu_panel_hotkey: Style::new(
                    TerminalColor::Default,
                    TerminalColor::Default,
                    StyleAttrs::UNDERLINE,
                ),
                menu_panel_border: bold,
                status_bar: reverse,
                status_text: reverse,
                window_border: plain,
                window_border_focused: bold,
                title: plain,
                title_focused: bold,
                gutter: plain,
                gutter_separator: bold,
                current_line: reverse,
                selection: reverse,
                selection_text: reverse,
                modal_scrim: plain,
                modal: plain,
                modal_text: plain,
                modal_border: bold,
                modal_input: reverse,
                dirty: bold,
                read_only: bold,
                control: bold,
                escape: bold,
                truncation: bold,
            },
        }
    }

    pub const fn turbo_16() -> Self {
        let bg = TerminalColor::Ansi(AnsiColor::Blue);

        Self {
            name: "turbo",
            theme: ThemeName::Turbo,
            colors: ColorProfile::Color16,
            palette: Palette {
                editor: Style::plain(TerminalColor::Ansi(AnsiColor::BrightWhite), bg),
                editor_text: Style::plain(TerminalColor::Ansi(AnsiColor::BrightWhite), bg),
                menu_bar: Style::plain(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::Cyan),
                ),
                menu_text: Style::plain(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::Cyan),
                ),
                menu_hotkey: Style::new(
                    TerminalColor::Ansi(AnsiColor::Red),
                    TerminalColor::Ansi(AnsiColor::Cyan),
                    StyleAttrs::BOLD,
                ),
                menu_active: Style::plain(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::Green),
                ),
                menu_active_hotkey: Style::new(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::Green),
                    StyleAttrs::BOLD,
                ),
                menu_panel: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                menu_panel_text: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                menu_panel_hotkey: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                    StyleAttrs::UNDERLINE,
                ),
                menu_panel_border: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                status_bar: Style::plain(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::BrightCyan),
                ),
                status_text: Style::plain(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::BrightCyan),
                ),
                window_border: Style::plain(TerminalColor::Ansi(AnsiColor::White), bg),
                window_border_focused: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
                    bg,
                    StyleAttrs::BOLD,
                ),
                title: Style::plain(TerminalColor::Ansi(AnsiColor::BrightWhite), bg),
                title_focused: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
                    bg,
                    StyleAttrs::BOLD,
                ),
                gutter: Style::plain(TerminalColor::Ansi(AnsiColor::BrightBlack), bg),
                gutter_separator: Style::plain(TerminalColor::Ansi(AnsiColor::White), bg),
                current_line: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                selection: Style::plain(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                ),
                selection_text: Style::plain(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                ),
                modal_scrim: Style::plain(TerminalColor::Ansi(AnsiColor::BrightBlack), bg),
                modal: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                modal_text: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                modal_border: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                ),
                modal_input: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    TerminalColor::Ansi(AnsiColor::Black),
                ),
                dirty: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
                    bg,
                    StyleAttrs::BOLD,
                ),
                read_only: Style::plain(TerminalColor::Ansi(AnsiColor::BrightRed), bg),
                control: Style::plain(TerminalColor::Ansi(AnsiColor::Yellow), bg),
                escape: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightRed),
                    bg,
                    StyleAttrs::BOLD,
                ),
                truncation: Style::new(TerminalColor::Ansi(AnsiColor::White), bg, StyleAttrs::BOLD),
            },
        }
    }

    pub const fn dark_256() -> Self {
        Self::with_256_darkish("dark", ThemeName::Dark, 234, 252, 38, 214)
    }

    pub const fn dun_256() -> Self {
        Self::with_256_darkish("dun", ThemeName::Dun, 235, 252, 44, 203)
    }

    pub const fn for_profile(theme: ThemeName, profile: TerminalProfile) -> Self {
        match profile.colors {
            ColorProfile::Color256 => match theme {
                ThemeName::MsEdit => Self::msedit_256(),
                ThemeName::Turbo => Self::turbo_16(),
                ThemeName::Dark => Self::dark_256(),
                ThemeName::Dun => Self::dun_256(),
            },
            ColorProfile::Color16 => match theme {
                ThemeName::Turbo => Self::turbo_16(),
                _ => Self::msedit_16(),
            },
            ColorProfile::Mono => Self::mono(),
        }
    }

    const fn with_256_darkish(
        name: &'static str,
        theme: ThemeName,
        bg: u8,
        fg: u8,
        accent: u8,
        warning: u8,
    ) -> Self {
        let editor_bg = TerminalColor::Indexed(bg);
        let editor_fg = TerminalColor::Indexed(fg);
        let accent = TerminalColor::Indexed(accent);
        let warning = TerminalColor::Indexed(warning);

        Self {
            name,
            theme,
            colors: ColorProfile::Color256,
            palette: Palette {
                editor: Style::plain(editor_fg, editor_bg),
                editor_text: Style::plain(editor_fg, editor_bg),
                menu_bar: Style::plain(editor_fg, TerminalColor::Indexed(bg + 1)),
                menu_text: Style::plain(editor_fg, TerminalColor::Indexed(bg + 1)),
                menu_hotkey: Style::new(accent, TerminalColor::Indexed(bg + 1), StyleAttrs::BOLD),
                menu_active: Style::plain(editor_bg, accent),
                menu_active_hotkey: Style::new(editor_bg, accent, StyleAttrs::BOLD),
                menu_panel: Style::plain(editor_fg, TerminalColor::Indexed(bg + 7)),
                menu_panel_text: Style::plain(editor_fg, TerminalColor::Indexed(bg + 7)),
                menu_panel_hotkey: Style::new(
                    editor_fg,
                    TerminalColor::Indexed(bg + 7),
                    StyleAttrs::UNDERLINE,
                ),
                menu_panel_border: Style::plain(
                    TerminalColor::Indexed(250),
                    TerminalColor::Indexed(bg + 7),
                ),
                status_bar: Style::plain(editor_bg, accent),
                status_text: Style::plain(editor_bg, accent),
                window_border: Style::plain(TerminalColor::Indexed(244), editor_bg),
                window_border_focused: Style::new(accent, editor_bg, StyleAttrs::BOLD),
                title: Style::plain(TerminalColor::Indexed(250), editor_bg),
                title_focused: Style::new(accent, editor_bg, StyleAttrs::BOLD),
                gutter: Style::plain(TerminalColor::Indexed(242), editor_bg),
                gutter_separator: Style::plain(TerminalColor::Indexed(248), editor_bg),
                current_line: Style::plain(editor_fg, TerminalColor::Indexed(bg + 4)),
                selection: Style::plain(editor_fg, TerminalColor::Indexed(bg + 5)),
                selection_text: Style::plain(editor_fg, TerminalColor::Indexed(bg + 5)),
                modal_scrim: Style::plain(TerminalColor::Indexed(244), editor_bg),
                modal: Style::plain(editor_fg, TerminalColor::Indexed(bg + 7)),
                modal_text: Style::plain(editor_fg, TerminalColor::Indexed(bg + 7)),
                modal_border: Style::plain(
                    TerminalColor::Indexed(250),
                    TerminalColor::Indexed(bg + 7),
                ),
                modal_input: Style::plain(editor_fg, editor_bg),
                dirty: Style::new(warning, editor_bg, StyleAttrs::BOLD),
                read_only: Style::plain(warning, editor_bg),
                control: Style::plain(TerminalColor::Indexed(215), editor_bg),
                escape: Style::new(warning, editor_bg, StyleAttrs::BOLD),
                truncation: Style::new(TerminalColor::Indexed(245), editor_bg, StyleAttrs::BOLD),
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::msedit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{ColorProfile, EncodingProfile};

    #[test]
    fn default_theme_is_msedit_256() {
        let theme = Theme::default();

        assert_eq!(theme.name, "msedit");
        assert_eq!(theme.theme, ThemeName::MsEdit);
        assert_eq!(theme.colors, ColorProfile::Color256);
        assert_eq!(
            theme.palette.window_border_focused.fg,
            TerminalColor::Indexed(253)
        );
        assert_eq!(theme.palette.menu_active.bg, TerminalColor::Indexed(108));
    }

    #[test]
    fn msedit_16_uses_only_ansi_colors() {
        let theme = Theme::msedit_16();

        assert_eq!(theme.colors, ColorProfile::Color16);
        assert_eq!(
            theme.palette.editor.bg,
            TerminalColor::Ansi(AnsiColor::Blue)
        );
        assert_eq!(
            theme.palette.menu_hotkey.fg,
            TerminalColor::Ansi(AnsiColor::BrightYellow)
        );
    }

    #[test]
    fn mono_theme_uses_reverse_for_chrome() {
        let theme = Theme::mono();

        assert_eq!(theme.colors, ColorProfile::Mono);
        assert!(theme.palette.menu_bar.attrs.reverse);
        assert!(theme.palette.status_bar.attrs.reverse);
        assert!(theme.palette.window_border_focused.attrs.bold);
    }

    #[test]
    fn profile_selects_expected_fallback_theme() {
        let profile = TerminalProfile::new(EncodingProfile::Ascii, ColorProfile::Color16);
        let theme = Theme::for_profile(ThemeName::MsEdit, profile);

        assert_eq!(theme.colors, ColorProfile::Color16);
        assert_eq!(theme.name, "msedit");
    }

    #[test]
    fn color256_profile_allows_optional_dun_theme() {
        let theme = Theme::for_profile(ThemeName::Dun, TerminalProfile::default());

        assert_eq!(theme.name, "dun");
        assert_eq!(theme.theme, ThemeName::Dun);
        assert_eq!(theme.colors, ColorProfile::Color256);
    }
}
