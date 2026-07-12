use crate::profile::{ColorProfile, TerminalProfile};

use super::{AnsiColor, Palette, Style, StyleAttrs, TerminalColor, Theme, ThemeName};

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
                search_match: Style::plain(editor_bg, TerminalColor::Indexed(220)),
                active_search_match: Style::new(
                    editor_bg,
                    TerminalColor::Indexed(226),
                    StyleAttrs::BOLD,
                ),
                scrollbar_thumb: Style::new(chrome_fg, chrome_bg, StyleAttrs::BOLD),
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
                syntax_keyword: Style::plain(TerminalColor::Indexed(117), editor_bg),
                syntax_comment: Style::plain(TerminalColor::Indexed(245), editor_bg),
                syntax_string: Style::plain(TerminalColor::Indexed(150), editor_bg),
                syntax_number: Style::plain(TerminalColor::Indexed(176), editor_bg),
                syntax_emphasis: Style::new(
                    TerminalColor::Indexed(223),
                    editor_bg,
                    StyleAttrs::BOLD,
                ),
                warning: Style::new(TerminalColor::Indexed(214), editor_bg, StyleAttrs::BOLD),
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
                search_match: Style::plain(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
                ),
                active_search_match: Style::new(
                    TerminalColor::Ansi(AnsiColor::Black),
                    TerminalColor::Ansi(AnsiColor::Yellow),
                    StyleAttrs::BOLD,
                ),
                scrollbar_thumb: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
                    editor_bg,
                    StyleAttrs::BOLD,
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
                syntax_keyword: Style::plain(TerminalColor::Ansi(AnsiColor::BrightCyan), editor_bg),
                syntax_comment: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightBlack),
                    editor_bg,
                ),
                syntax_string: Style::plain(TerminalColor::Ansi(AnsiColor::BrightGreen), editor_bg),
                syntax_number: Style::plain(
                    TerminalColor::Ansi(AnsiColor::BrightMagenta),
                    editor_bg,
                ),
                syntax_emphasis: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightWhite),
                    editor_bg,
                    StyleAttrs::BOLD,
                ),
                warning: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
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
                search_match: reverse,
                active_search_match: Style::new(
                    TerminalColor::Default,
                    TerminalColor::Default,
                    StyleAttrs::BOLD_REVERSE,
                ),
                scrollbar_thumb: Style::new(
                    TerminalColor::Default,
                    TerminalColor::Default,
                    StyleAttrs::BOLD_REVERSE,
                ),
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
                syntax_keyword: bold,
                syntax_comment: plain,
                syntax_string: Style::new(
                    TerminalColor::Default,
                    TerminalColor::Default,
                    StyleAttrs::UNDERLINE,
                ),
                syntax_number: plain,
                syntax_emphasis: bold,
                warning: bold,
            },
        }
    }

    pub const fn turbo_16() -> Self {
        // The classic Borland Turbo Vision look: a blue editor desktop with
        // light-gray body text, white reserved words, green types, magenta
        // numbers; gray (light-gray) menu/status bars and dialogs with black
        // text and red hotkeys. CGA names: LightGray = White, White =
        // BrightWhite, DarkGray = BrightBlack, Light* = Bright*.
        let bg = TerminalColor::Ansi(AnsiColor::Blue);
        let text = TerminalColor::Ansi(AnsiColor::White); // CGA light gray
        let bright = TerminalColor::Ansi(AnsiColor::BrightWhite); // CGA white
        let gray = TerminalColor::Ansi(AnsiColor::BrightBlack); // CGA dark gray
        let bar = TerminalColor::Ansi(AnsiColor::White); // light-gray chrome
        let black = TerminalColor::Ansi(AnsiColor::Black);
        let red = TerminalColor::Ansi(AnsiColor::Red);
        let green = TerminalColor::Ansi(AnsiColor::Green);
        let cyan = TerminalColor::Ansi(AnsiColor::Cyan);

        Self {
            name: "turbo",
            theme: ThemeName::Turbo,
            colors: ColorProfile::Color16,
            palette: Palette {
                editor: Style::plain(text, bg),
                editor_text: Style::plain(text, bg),
                menu_bar: Style::plain(black, bar),
                menu_text: Style::plain(black, bar),
                menu_hotkey: Style::new(red, bar, StyleAttrs::BOLD),
                menu_active: Style::plain(black, green),
                menu_active_hotkey: Style::new(red, green, StyleAttrs::BOLD),
                menu_panel: Style::plain(black, bar),
                menu_panel_text: Style::plain(black, bar),
                menu_panel_hotkey: Style::new(red, bar, StyleAttrs::UNDERLINE),
                menu_panel_border: Style::plain(gray, bar),
                status_bar: Style::plain(black, bar),
                status_text: Style::plain(black, bar),
                window_border: Style::plain(text, bg),
                window_border_focused: Style::new(bright, bg, StyleAttrs::BOLD),
                title: Style::plain(text, bg),
                title_focused: Style::new(bright, bg, StyleAttrs::BOLD),
                gutter: Style::plain(gray, bg),
                gutter_separator: Style::plain(text, bg),
                current_line: Style::plain(bright, bg),
                selection: Style::plain(black, cyan),
                selection_text: Style::plain(black, cyan),
                search_match: Style::plain(black, TerminalColor::Ansi(AnsiColor::BrightYellow)),
                active_search_match: Style::new(
                    black,
                    TerminalColor::Ansi(AnsiColor::BrightRed),
                    StyleAttrs::BOLD,
                ),
                scrollbar_thumb: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightCyan),
                    bg,
                    StyleAttrs::BOLD,
                ),
                modal_scrim: Style::plain(gray, bg),
                modal: Style::plain(black, bar),
                modal_text: Style::plain(black, bar),
                modal_border: Style::plain(gray, bar),
                modal_input: Style::plain(black, cyan),
                dirty: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
                    bg,
                    StyleAttrs::BOLD,
                ),
                read_only: Style::plain(TerminalColor::Ansi(AnsiColor::BrightRed), bg),
                control: Style::plain(TerminalColor::Ansi(AnsiColor::BrightYellow), bg),
                escape: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightRed),
                    bg,
                    StyleAttrs::BOLD,
                ),
                truncation: Style::new(gray, bg, StyleAttrs::BOLD),
                syntax_keyword: Style::new(bright, bg, StyleAttrs::BOLD),
                syntax_comment: Style::plain(gray, bg),
                syntax_string: Style::plain(TerminalColor::Ansi(AnsiColor::BrightRed), bg),
                syntax_number: Style::plain(TerminalColor::Ansi(AnsiColor::BrightMagenta), bg),
                syntax_emphasis: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightGreen),
                    bg,
                    StyleAttrs::BOLD,
                ),
                warning: Style::new(
                    TerminalColor::Ansi(AnsiColor::BrightYellow),
                    bg,
                    StyleAttrs::BOLD,
                ),
            },
        }
    }

    pub const fn dark_256() -> Self {
        Self::with_256_darkish("dark", ThemeName::Dark, 234, 252, 38, 214)
    }

    /// "Dun": the dull grayish-brown of a horse's coat — buckskin body, dark
    /// points, dark dorsal stripe. xterm-256 cannot express a dark brown at
    /// all (the color cube's channels step 0 -> 95, so the only dark entries
    /// are black, dark red, dark olive, dark blue/green/purple and gray), so
    /// the warmth lives in the ink rather than the ground: sand and buckskin
    /// on deep shadow. `dun_16` is the 16-color fallback.
    pub const fn dun_256() -> Self {
        let shadow = TerminalColor::Indexed(234); // #1c1c1c — the ground
        let band = TerminalColor::Indexed(236); // #303030 — current line
        let scrub = TerminalColor::Indexed(58); // #5f5f00 — the one warm dark
        let panel = TerminalColor::Indexed(237); // #3a3a3a — panels and modals
        let sand = TerminalColor::Indexed(187); // #d7d7af — body text
        let buckskin = TerminalColor::Indexed(180); // #d7af87 — the coat: accent
        let hide = TerminalColor::Indexed(137); // #af875f — muted tan
        let dust = TerminalColor::Indexed(245); // #8a8a8a — dim gray
        let cream = TerminalColor::Indexed(223); // #ffd7af — emphasis
        let rust = TerminalColor::Indexed(173); // #d7875f — keywords
        let sage = TerminalColor::Indexed(107); // #87af5f — strings
        let mauve = TerminalColor::Indexed(139); // #af87af — numbers
        let gold = TerminalColor::Indexed(179); // #d7af5f — search
        let amber = TerminalColor::Indexed(214); // #ffaf00 — control glyphs
        let coral = TerminalColor::Indexed(203); // #ff5f5f — alarm

        Self {
            name: "dun",
            theme: ThemeName::Dun,
            colors: ColorProfile::Color256,
            palette: Palette {
                editor: Style::plain(sand, shadow),
                editor_text: Style::plain(sand, shadow),
                menu_bar: Style::plain(sand, band),
                menu_text: Style::plain(sand, band),
                menu_hotkey: Style::new(buckskin, band, StyleAttrs::BOLD),
                menu_active: Style::plain(shadow, buckskin),
                menu_active_hotkey: Style::new(shadow, buckskin, StyleAttrs::BOLD),
                menu_panel: Style::plain(sand, panel),
                menu_panel_text: Style::plain(sand, panel),
                menu_panel_hotkey: Style::new(cream, panel, StyleAttrs::UNDERLINE),
                menu_panel_border: Style::plain(hide, panel),
                status_bar: Style::plain(shadow, buckskin),
                status_text: Style::plain(shadow, buckskin),
                window_border: Style::plain(hide, shadow),
                window_border_focused: Style::new(buckskin, shadow, StyleAttrs::BOLD),
                title: Style::plain(hide, shadow),
                title_focused: Style::new(cream, shadow, StyleAttrs::BOLD),
                gutter: Style::plain(hide, shadow),
                gutter_separator: Style::plain(panel, shadow),
                current_line: Style::plain(sand, band),
                selection: Style::plain(cream, scrub),
                selection_text: Style::plain(cream, scrub),
                search_match: Style::plain(shadow, gold),
                active_search_match: Style::new(shadow, amber, StyleAttrs::BOLD),
                scrollbar_thumb: Style::new(buckskin, shadow, StyleAttrs::BOLD),
                modal_scrim: Style::plain(dust, shadow),
                modal: Style::plain(sand, panel),
                modal_text: Style::plain(sand, panel),
                modal_border: Style::plain(hide, panel),
                modal_input: Style::plain(cream, shadow),
                dirty: Style::new(amber, shadow, StyleAttrs::BOLD),
                read_only: Style::plain(coral, shadow),
                control: Style::plain(amber, shadow),
                escape: Style::new(coral, shadow, StyleAttrs::BOLD),
                truncation: Style::new(dust, shadow, StyleAttrs::BOLD),
                syntax_keyword: Style::plain(rust, shadow),
                syntax_comment: Style::plain(dust, shadow),
                syntax_string: Style::plain(sage, shadow),
                syntax_number: Style::plain(mauve, shadow),
                syntax_emphasis: Style::new(cream, shadow, StyleAttrs::BOLD),
                warning: Style::new(coral, shadow, StyleAttrs::BOLD),
            },
        }
    }

    /// 16-color fallback for `dun`: black ground, yellow/white ink. Without
    /// this, `dun` degraded into `msedit_16`'s blue desktop, which reads as a
    /// different theme entirely.
    pub const fn dun_16() -> Self {
        let shadow = TerminalColor::Ansi(AnsiColor::Black);
        let sand = TerminalColor::Ansi(AnsiColor::White);
        let bright = TerminalColor::Ansi(AnsiColor::BrightWhite);
        let buckskin = TerminalColor::Ansi(AnsiColor::Yellow);
        let cream = TerminalColor::Ansi(AnsiColor::BrightYellow);
        let dust = TerminalColor::Ansi(AnsiColor::BrightBlack);
        let sage = TerminalColor::Ansi(AnsiColor::Green);
        let mauve = TerminalColor::Ansi(AnsiColor::Magenta);
        let coral = TerminalColor::Ansi(AnsiColor::BrightRed);

        Self {
            name: "dun",
            theme: ThemeName::Dun,
            colors: ColorProfile::Color16,
            palette: Palette {
                editor: Style::plain(sand, shadow),
                editor_text: Style::plain(sand, shadow),
                menu_bar: Style::plain(shadow, sand),
                menu_text: Style::plain(shadow, sand),
                menu_hotkey: Style::new(shadow, sand, StyleAttrs::BOLD),
                menu_active: Style::plain(shadow, buckskin),
                menu_active_hotkey: Style::new(shadow, buckskin, StyleAttrs::BOLD),
                menu_panel: Style::plain(bright, dust),
                menu_panel_text: Style::plain(bright, dust),
                menu_panel_hotkey: Style::new(cream, dust, StyleAttrs::UNDERLINE),
                menu_panel_border: Style::plain(sand, dust),
                status_bar: Style::plain(shadow, buckskin),
                status_text: Style::plain(shadow, buckskin),
                window_border: Style::plain(dust, shadow),
                window_border_focused: Style::new(cream, shadow, StyleAttrs::BOLD),
                title: Style::plain(sand, shadow),
                title_focused: Style::new(cream, shadow, StyleAttrs::BOLD),
                gutter: Style::plain(dust, shadow),
                gutter_separator: Style::plain(dust, shadow),
                current_line: Style::new(bright, shadow, StyleAttrs::BOLD),
                selection: Style::plain(shadow, sand),
                selection_text: Style::plain(shadow, sand),
                search_match: Style::plain(shadow, cream),
                active_search_match: Style::new(shadow, buckskin, StyleAttrs::BOLD),
                scrollbar_thumb: Style::new(cream, shadow, StyleAttrs::BOLD),
                modal_scrim: Style::plain(dust, shadow),
                modal: Style::plain(bright, dust),
                modal_text: Style::plain(bright, dust),
                modal_border: Style::plain(sand, dust),
                modal_input: Style::plain(cream, shadow),
                dirty: Style::new(cream, shadow, StyleAttrs::BOLD),
                read_only: Style::plain(coral, shadow),
                control: Style::plain(cream, shadow),
                escape: Style::new(coral, shadow, StyleAttrs::BOLD),
                truncation: Style::new(dust, shadow, StyleAttrs::BOLD),
                syntax_keyword: Style::new(cream, shadow, StyleAttrs::BOLD),
                syntax_comment: Style::plain(dust, shadow),
                syntax_string: Style::plain(sage, shadow),
                syntax_number: Style::plain(mauve, shadow),
                syntax_emphasis: Style::new(bright, shadow, StyleAttrs::BOLD),
                warning: Style::new(coral, shadow, StyleAttrs::BOLD),
            },
        }
    }

    /// The Borland Turbo Vision look pinned to fixed xterm-256 indices that
    /// track the CGA palette, so the deep-blue desktop renders consistently
    /// instead of inheriting whatever the terminal maps ANSI "blue" to. Used
    /// on 256-color terminals; `turbo_16` is the 16-color fallback.
    pub const fn turbo_256() -> Self {
        let bg = TerminalColor::Indexed(19); // CGA blue (#0000af)
        let text = TerminalColor::Indexed(250); // light gray
        let white = TerminalColor::Indexed(231);
        let gray = TerminalColor::Indexed(240); // dark gray
        let bar = TerminalColor::Indexed(250); // light-gray chrome
        let black = TerminalColor::Indexed(16);
        let red = TerminalColor::Indexed(196);
        let green = TerminalColor::Indexed(40);
        let cyan = TerminalColor::Indexed(37);
        let ltcyan = TerminalColor::Indexed(87);
        let ltred = TerminalColor::Indexed(203);
        let ltmagenta = TerminalColor::Indexed(207);
        let ltgreen = TerminalColor::Indexed(83);
        let yellow = TerminalColor::Indexed(226);

        Self {
            name: "turbo",
            theme: ThemeName::Turbo,
            colors: ColorProfile::Color256,
            palette: Palette {
                editor: Style::plain(text, bg),
                editor_text: Style::plain(text, bg),
                menu_bar: Style::plain(black, bar),
                menu_text: Style::plain(black, bar),
                menu_hotkey: Style::new(red, bar, StyleAttrs::BOLD),
                menu_active: Style::plain(black, green),
                menu_active_hotkey: Style::new(red, green, StyleAttrs::BOLD),
                menu_panel: Style::plain(black, bar),
                menu_panel_text: Style::plain(black, bar),
                menu_panel_hotkey: Style::new(red, bar, StyleAttrs::UNDERLINE),
                menu_panel_border: Style::plain(gray, bar),
                status_bar: Style::plain(black, bar),
                status_text: Style::plain(black, bar),
                window_border: Style::plain(text, bg),
                window_border_focused: Style::new(white, bg, StyleAttrs::BOLD),
                title: Style::plain(text, bg),
                title_focused: Style::new(white, bg, StyleAttrs::BOLD),
                gutter: Style::plain(gray, bg),
                gutter_separator: Style::plain(text, bg),
                current_line: Style::plain(white, bg),
                selection: Style::plain(black, cyan),
                selection_text: Style::plain(black, cyan),
                search_match: Style::plain(black, yellow),
                active_search_match: Style::new(black, ltred, StyleAttrs::BOLD),
                scrollbar_thumb: Style::new(ltcyan, bg, StyleAttrs::BOLD),
                modal_scrim: Style::plain(gray, bg),
                modal: Style::plain(black, bar),
                modal_text: Style::plain(black, bar),
                modal_border: Style::plain(gray, bar),
                modal_input: Style::plain(black, cyan),
                dirty: Style::new(yellow, bg, StyleAttrs::BOLD),
                read_only: Style::plain(ltred, bg),
                control: Style::plain(yellow, bg),
                escape: Style::new(ltred, bg, StyleAttrs::BOLD),
                truncation: Style::new(gray, bg, StyleAttrs::BOLD),
                syntax_keyword: Style::new(white, bg, StyleAttrs::BOLD),
                syntax_comment: Style::plain(gray, bg),
                syntax_string: Style::plain(ltred, bg),
                syntax_number: Style::plain(ltmagenta, bg),
                syntax_emphasis: Style::new(ltgreen, bg, StyleAttrs::BOLD),
                warning: Style::new(TerminalColor::Indexed(226), bg, StyleAttrs::BOLD),
            },
        }
    }

    pub const fn for_profile(theme: ThemeName, profile: TerminalProfile) -> Self {
        match profile.colors {
            ColorProfile::Color256 => match theme {
                ThemeName::MsEdit => Self::msedit_256(),
                ThemeName::Turbo => Self::turbo_256(),
                ThemeName::Dark => Self::dark_256(),
                ThemeName::Dun => Self::dun_256(),
            },
            ColorProfile::Color16 => match theme {
                ThemeName::Turbo => Self::turbo_16(),
                ThemeName::Dun => Self::dun_16(),
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
                search_match: Style::plain(editor_bg, warning),
                active_search_match: Style::new(editor_bg, accent, StyleAttrs::BOLD),
                scrollbar_thumb: Style::new(accent, editor_bg, StyleAttrs::BOLD),
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
                syntax_keyword: Style::plain(TerminalColor::Indexed(111), editor_bg),
                syntax_comment: Style::plain(TerminalColor::Indexed(244), editor_bg),
                syntax_string: Style::plain(TerminalColor::Indexed(150), editor_bg),
                syntax_number: Style::plain(TerminalColor::Indexed(176), editor_bg),
                syntax_emphasis: Style::new(
                    TerminalColor::Indexed(222),
                    editor_bg,
                    StyleAttrs::BOLD,
                ),
                warning: Style::new(warning, editor_bg, StyleAttrs::BOLD),
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dun_256()
    }
}
