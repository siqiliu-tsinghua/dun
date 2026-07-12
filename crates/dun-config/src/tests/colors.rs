use super::support::*;
use crate::colors::{format_attrs, format_color, parse_attrs, parse_color};
use dun_term::{AnsiColor, StyleAttrs, TerminalColor};

#[test]
fn parse_color_accepts_index_name_and_default() {
    assert_eq!(parse_color("17"), Some(TerminalColor::Indexed(17)));
    assert_eq!(
        parse_color("red"),
        Some(TerminalColor::Ansi(AnsiColor::Red))
    );
    assert_eq!(
        parse_color("bright_blue"),
        Some(TerminalColor::Ansi(AnsiColor::BrightBlue))
    );
    assert_eq!(
        parse_color("bright-blue"),
        Some(TerminalColor::Ansi(AnsiColor::BrightBlue))
    );
    assert_eq!(parse_color("default"), Some(TerminalColor::Default));
    assert_eq!(parse_color("256"), None);
    assert_eq!(parse_color("-1"), None);
    assert_eq!(parse_color("mauve"), None);
}

#[test]
fn parse_attrs_handles_lists_and_none() {
    assert_eq!(parse_attrs("bold"), Some(StyleAttrs::BOLD));
    assert_eq!(
        parse_attrs("bold, underline"),
        Some(StyleAttrs {
            bold: true,
            underline: true,
            reverse: false,
        })
    );
    assert_eq!(
        parse_attrs("bold underline reverse"),
        Some(StyleAttrs {
            bold: true,
            underline: true,
            reverse: true,
        })
    );
    assert_eq!(parse_attrs("none"), Some(StyleAttrs::NONE));
    assert_eq!(parse_attrs("none, bold"), None);
    assert_eq!(parse_attrs("sparkle"), None);
}

#[test]
fn color_specs_round_trip() {
    let ansi_colors = [
        AnsiColor::Black,
        AnsiColor::Red,
        AnsiColor::Green,
        AnsiColor::Yellow,
        AnsiColor::Blue,
        AnsiColor::Magenta,
        AnsiColor::Cyan,
        AnsiColor::White,
        AnsiColor::BrightBlack,
        AnsiColor::BrightRed,
        AnsiColor::BrightGreen,
        AnsiColor::BrightYellow,
        AnsiColor::BrightBlue,
        AnsiColor::BrightMagenta,
        AnsiColor::BrightCyan,
        AnsiColor::BrightWhite,
    ];
    for ansi in ansi_colors {
        let color = TerminalColor::Ansi(ansi);
        assert_eq!(parse_color(&format_color(color)), Some(color));
    }
    for color in [
        TerminalColor::Indexed(0),
        TerminalColor::Indexed(231),
        TerminalColor::Default,
    ] {
        assert_eq!(parse_color(&format_color(color)), Some(color));
    }

    for attrs in [
        StyleAttrs::NONE,
        StyleAttrs::BOLD,
        StyleAttrs {
            bold: true,
            underline: true,
            reverse: true,
        },
    ] {
        assert_eq!(parse_attrs(&format_attrs(attrs)), Some(attrs));
    }
}

#[test]
fn granular_override_changes_only_one_component() {
    let profile = TerminalProfile::utf8_256();
    let original = Config::default().resolved_theme(profile).palette;
    let config = parse_config("color.editor.bg = 17").unwrap();
    let overridden = config.resolved_theme(profile).palette;

    assert_eq!(overridden.editor.bg, TerminalColor::Indexed(17));
    assert_eq!(overridden.editor.fg, original.editor.fg);
    assert_eq!(overridden.editor.attrs, original.editor.attrs);
    for id in dun_term::PALETTE_ROLE_IDS {
        if *id != "editor" {
            assert_eq!(
                overridden.role(id),
                original.role(id),
                "unexpected change to {id}"
            );
        }
    }
}

#[test]
fn shorthand_sets_fg_and_optional_bg() {
    let profile = TerminalProfile::utf8_256();
    let original = Config::default().resolved_theme(profile).palette;
    let config = parse_config(
        "\
color.warning = 196 / 0
color.dirty = 208
",
    )
    .unwrap();
    let palette = config.resolved_theme(profile).palette;

    assert_eq!(palette.warning.fg, TerminalColor::Indexed(196));
    assert_eq!(palette.warning.bg, TerminalColor::Indexed(0));
    assert_eq!(palette.dirty.fg, TerminalColor::Indexed(208));
    assert_eq!(palette.dirty.bg, original.dirty.bg);
}

#[test]
fn attrs_override_applies() {
    let config = parse_config("color.title.attrs = bold, underline").unwrap();
    let attrs = config
        .resolved_theme(TerminalProfile::utf8_256())
        .palette
        .title
        .attrs;

    assert!(attrs.bold);
    assert!(attrs.underline);
}

#[test]
fn overrides_layer_on_selected_theme() {
    let profile = TerminalProfile::utf8_256();
    let original = parse_config("theme = dark")
        .unwrap()
        .resolved_theme(profile)
        .palette;
    let theme = parse_config(
        "\
theme = dark
color.warning.fg = 200
",
    )
    .unwrap()
    .resolved_theme(profile);

    assert_eq!(theme.name, "dark");
    assert_eq!(theme.palette.warning.fg, TerminalColor::Indexed(200));
    assert_eq!(theme.palette.warning.bg, original.warning.bg);
}

#[test]
fn unknown_role_and_component_are_line_errors() {
    let role_error = parse_config("color.nope.fg = 1").unwrap_err();
    assert_eq!(role_error.line, Some(1));
    assert!(role_error.message.contains("unknown color role `nope`"));

    let component_error = parse_config("color.editor.glow = 1").unwrap_err();
    assert_eq!(component_error.line, Some(1));
    assert!(
        component_error
            .message
            .contains("unknown color component `glow`")
    );
}

/// A palette role missing from the config dump is the same declaration drift
/// that left the advertised `mono` theme unparseable; both sides must agree.
#[test]
fn palette_roles_and_dumped_config_agree() {
    let text = default_config_text();
    let dumped_roles = text
        .lines()
        .filter_map(|line| {
            let declaration = line.strip_prefix("# color.")?;
            declaration.split_once(" = ").map(|(role, _)| role)
        })
        .collect::<Vec<_>>();

    assert!(text.contains("# Color overrides"));
    assert_eq!(dumped_roles.len(), dun_term::PALETTE_ROLE_IDS.len());
    for &role in dun_term::PALETTE_ROLE_IDS {
        assert_eq!(
            dumped_roles
                .iter()
                .filter(|dumped| **dumped == role)
                .count(),
            1,
            "palette role `{role}` must appear exactly once in the config dump"
        );
    }
    for &role in &dumped_roles {
        assert!(
            dun_term::PALETTE_ROLE_IDS.contains(&role),
            "config dump contains unknown palette role `{role}`"
        );
    }
    parse_config(&text).unwrap().validate().unwrap();
}
