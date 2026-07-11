use super::*;
use crate::profile::{ColorProfile, EncodingProfile, TerminalProfile};

#[test]
fn default_theme_is_dun_256() {
    let theme = Theme::default();

    assert_eq!(theme.name, "dun");
    assert_eq!(theme.theme, ThemeName::Dun);
    assert_eq!(theme.colors, ColorProfile::Color256);
    // dun's accent (Indexed 44) drives the focused border and active menu.
    assert_eq!(
        theme.palette.window_border_focused.fg,
        TerminalColor::Indexed(44)
    );
    assert_eq!(theme.palette.menu_active.bg, TerminalColor::Indexed(44));
}

#[test]
fn turbo_256_pins_the_cga_blue_desktop() {
    // 256-color terminals get fixed indices so the deep-blue desktop does not
    // inherit the terminal's ANSI-blue mapping; 16-color falls back to ANSI.
    let theme = Theme::for_profile(ThemeName::Turbo, TerminalProfile::default());
    assert_eq!(theme.name, "turbo");
    assert_eq!(theme.colors, ColorProfile::Color256);
    assert_eq!(theme.palette.editor.bg, TerminalColor::Indexed(19)); // CGA blue
    assert_eq!(theme.palette.editor.fg, TerminalColor::Indexed(250)); // light gray

    let fallback = Theme::for_profile(
        ThemeName::Turbo,
        TerminalProfile::new(EncodingProfile::Utf8, ColorProfile::Color16),
    );
    assert_eq!(fallback.colors, ColorProfile::Color16);
    assert_eq!(
        fallback.palette.editor.bg,
        TerminalColor::Ansi(AnsiColor::Blue)
    );
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
