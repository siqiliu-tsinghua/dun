use super::support::*;

#[test]
fn default_config_is_valid() {
    let config = Config::default();

    assert_eq!(config.theme, ThemeName::Dun);
    assert!(!config.clipboard.osc52.enabled);
    assert!(!config.clipboard.osc52.allow_read);
    assert!(config.validate().is_ok());
    assert!(config.keybindings.bindings.len() > 10);
}

#[test]
fn terminal_overrides_apply_to_detected_profile() {
    let overrides = TerminalOverrides {
        encoding: Some(EncodingProfile::Ascii),
        colors: Some(ColorProfile::Color16),
        ambiguous_width: None,
    };

    assert_eq!(
        overrides.apply_to(TerminalProfile::utf8_256()),
        TerminalProfile::ascii_16()
    );
}

#[test]
fn terminal_ambiguous_width_parses_and_applies() {
    use crate::AmbiguousWidth;

    let wide = parse_config("terminal.ambiguous-width = wide").unwrap();
    assert_eq!(wide.terminal.ambiguous_width, Some(AmbiguousWidth::Wide));
    assert_eq!(
        wide.terminal_profile(TerminalProfile::utf8_256())
            .ambiguous_width,
        AmbiguousWidth::Wide
    );

    let narrow = parse_config("terminal.ambiguous-width = narrow").unwrap();
    assert_eq!(
        narrow.terminal.ambiguous_width,
        Some(AmbiguousWidth::Narrow)
    );

    // Absent override leaves the detected profile's mode (Narrow) untouched.
    let empty = parse_config("theme = dun").unwrap();
    assert_eq!(empty.terminal.ambiguous_width, None);

    assert!(parse_config("terminal.ambiguous-width = huge").is_err());
}

#[test]
fn config_resolves_theme_after_terminal_overrides() {
    let config = Config {
        terminal: TerminalOverrides {
            encoding: Some(EncodingProfile::Ascii),
            colors: Some(ColorProfile::Color16),
            ambiguous_width: None,
        },
        ..Config::default()
    };

    let theme = config.resolved_theme(TerminalProfile::utf8_256());

    // The default theme is dun, and forcing 16 colors keeps it dun: the theme
    // carries its own 16-color variant rather than degrading into msedit.
    assert_eq!(theme.colors, ColorProfile::Color16);
    assert_eq!(theme.name, "dun");
}

#[test]
fn default_config_keeps_mouse_disabled() {
    assert!(!Config::default().mouse.enabled);
}

#[test]
fn default_plugin_status_is_opt_in_with_a_five_minute_idle_threshold() {
    let plugin_status = Config::default().plugin_status;

    assert!(!plugin_status.status_bar);
    assert_eq!(plugin_status.idle_after_ms, 300_000);
}

#[test]
fn default_config_text_lists_parseable_default_bindings() {
    let text = default_config_text();

    assert!(text.contains("# Appearance"));
    assert!(text.contains("# Terminal fallback overrides"));
    assert!(text.contains("# File and display limits"));
    assert!(text.contains("theme = dun"));
    assert!(text.contains("mouse.enabled = false"));
    assert!(text.contains("plugins.status_bar = false"));
    assert!(text.contains("plugins.idle_after_ms = 300000"));
    assert!(text.contains("clipboard.osc52.enabled = false"));
    assert!(text.contains("clipboard.osc52.allow_read = false"));
    assert!(text.contains("key.app.help = F1"));
    assert!(text.contains("key.file_dialog.toggle_hidden = Ctrl+H"));
    let defaults = parse_config(&text).unwrap();
    assert!(!defaults.clipboard.osc52.enabled);
    assert!(!defaults.clipboard.osc52.allow_read);

    let opted_in = text
        .replace(
            "clipboard.osc52.enabled = false",
            "clipboard.osc52.enabled = true",
        )
        .replace(
            "clipboard.osc52.allow_read = false",
            "clipboard.osc52.allow_read = true",
        );
    let opted_in = parse_config(&opted_in).unwrap();
    assert!(opted_in.clipboard.osc52.enabled);
    assert!(opted_in.clipboard.osc52.allow_read);
    opted_in.validate().unwrap();
}
