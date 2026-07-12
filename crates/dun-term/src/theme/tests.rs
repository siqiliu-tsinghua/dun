use super::*;
use crate::profile::{ColorProfile, EncodingProfile, TerminalProfile};

#[test]
fn default_theme_is_dun_256() {
    let theme = Theme::default();

    assert_eq!(theme.name, "dun");
    assert_eq!(theme.theme, ThemeName::Dun);
    assert_eq!(theme.colors, ColorProfile::Color256);
    // dun's accent is buckskin (Indexed 180), the horse-coat tan that drives
    // the focused border, the active menu, and the status bar.
    assert_eq!(
        theme.palette.window_border_focused.fg,
        TerminalColor::Indexed(180)
    );
    assert_eq!(theme.palette.menu_active.bg, TerminalColor::Indexed(180));
    assert_eq!(theme.palette.status_bar.bg, TerminalColor::Indexed(180));
    // Warm ink on a deep neutral ground: xterm-256 has no dark brown, so the
    // ground cannot carry the hue (see dun_256's doc comment).
    assert_eq!(theme.palette.editor.bg, TerminalColor::Indexed(234));
    assert_eq!(theme.palette.editor.fg, TerminalColor::Indexed(187));
}

/// dun and dark used to be the same theme with a one-step-darker background
/// and a marginally bluer accent, which made them indistinguishable in use.
#[test]
fn dun_and_dark_are_visibly_different_themes() {
    let dun = Theme::dun_256().palette;
    let dark = Theme::dark_256().palette;

    // Warm accent versus cool accent is the whole point of the split.
    assert_eq!(dun.window_border_focused.fg, TerminalColor::Indexed(180));
    assert_eq!(dark.window_border_focused.fg, TerminalColor::Indexed(38));

    // And they no longer agree role-for-role on the syntax palette.
    assert_ne!(dun.syntax_keyword.fg, dark.syntax_keyword.fg);
    assert_ne!(dun.syntax_string.fg, dark.syntax_string.fg);
    assert_ne!(dun.editor.fg, dark.editor.fg);
}

/// A dun terminal that only does 16 colors must not fall back into msedit's
/// blue desktop; it gets dun's own black-and-sand fallback.
#[test]
fn dun_falls_back_to_its_own_16_color_variant() {
    let theme = Theme::for_profile(
        ThemeName::Dun,
        TerminalProfile::new(EncodingProfile::Utf8, ColorProfile::Color16),
    );

    assert_eq!(theme.name, "dun");
    assert_eq!(theme.colors, ColorProfile::Color16);
    assert_eq!(
        theme.palette.editor.bg,
        TerminalColor::Ansi(AnsiColor::Black)
    );
    assert_eq!(
        theme.palette.status_bar.bg,
        TerminalColor::Ansi(AnsiColor::Yellow)
    );
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

#[test]
fn palette_role_ids_all_resolve() {
    let palette = Theme::default().palette;

    assert_eq!(PALETTE_ROLE_IDS.len(), 41);
    for id in PALETTE_ROLE_IDS {
        assert!(
            palette.role(id).is_some(),
            "palette role {id:?} must resolve"
        );
    }
    assert!(palette.role("not_a_palette_role").is_none());
}

#[test]
fn role_mut_overrides_a_single_field() {
    let mut palette = Theme::default().palette;
    let original = palette;
    let replacement = Style::new(
        TerminalColor::Ansi(AnsiColor::Red),
        TerminalColor::Ansi(AnsiColor::Cyan),
        StyleAttrs::UNDERLINE,
    );

    *palette.role_mut("warning").expect("warning role exists") = replacement;

    for id in PALETTE_ROLE_IDS {
        let expected = if *id == "warning" {
            replacement
        } else {
            original.role(id).expect("listed role exists")
        };
        assert_eq!(
            palette.role(id),
            Some(expected),
            "unexpected change to {id}"
        );
    }
}

#[test]
fn themes_define_a_warning_color() {
    assert_eq!(
        Theme::dun_256().palette.warning.fg,
        TerminalColor::Indexed(203)
    );
    assert_eq!(
        Theme::dark_256().palette.warning.fg,
        TerminalColor::Indexed(214)
    );
}
