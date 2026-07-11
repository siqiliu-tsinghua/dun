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
fn solarized_themes_use_the_solarized_256_palette() {
    let dark = Theme::solarized_dark();
    assert_eq!(dark.name, "solarized-dark");
    assert_eq!(dark.theme, ThemeName::SolarizedDark);
    assert_eq!(dark.colors, ColorProfile::Color256);
    assert_eq!(dark.palette.editor.bg, TerminalColor::Indexed(234)); // base03
    assert_eq!(dark.palette.editor.fg, TerminalColor::Indexed(244)); // base0
    assert_eq!(dark.palette.syntax_string.fg, TerminalColor::Indexed(37)); // cyan
    assert_eq!(dark.palette.syntax_keyword.fg, TerminalColor::Indexed(64)); // green

    let light = Theme::solarized_light();
    assert_eq!(light.name, "solarized-light");
    assert_eq!(light.theme, ThemeName::SolarizedLight);
    assert_eq!(light.palette.editor.bg, TerminalColor::Indexed(230)); // base3
    assert_eq!(light.palette.editor.fg, TerminalColor::Indexed(241)); // base00
    // Accents are shared across both variants.
    assert_eq!(light.palette.syntax_string.fg, TerminalColor::Indexed(37)); // cyan
}

#[test]
fn for_profile_selects_solarized_variants() {
    let profile = TerminalProfile::default();
    assert_eq!(
        Theme::for_profile(ThemeName::SolarizedDark, profile).name,
        "solarized-dark"
    );
    assert_eq!(
        Theme::for_profile(ThemeName::SolarizedLight, profile).name,
        "solarized-light"
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
