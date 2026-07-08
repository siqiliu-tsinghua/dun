use super::support::*;

#[test]
fn default_config_is_valid() {
    let config = Config::default();

    assert_eq!(config.theme, ThemeName::MsEdit);
    assert!(config.validate().is_ok());
    assert!(config.keybindings.bindings.len() > 10);
}

#[test]
fn terminal_overrides_apply_to_detected_profile() {
    let overrides = TerminalOverrides {
        encoding: Some(EncodingProfile::Ascii),
        colors: Some(ColorProfile::Color16),
    };

    assert_eq!(
        overrides.apply_to(TerminalProfile::utf8_256()),
        TerminalProfile::ascii_16()
    );
}

#[test]
fn config_resolves_theme_after_terminal_overrides() {
    let config = Config {
        terminal: TerminalOverrides {
            encoding: Some(EncodingProfile::Ascii),
            colors: Some(ColorProfile::Color16),
        },
        ..Config::default()
    };

    let theme = config.resolved_theme(TerminalProfile::utf8_256());

    assert_eq!(theme.colors, ColorProfile::Color16);
    assert_eq!(theme.name, "msedit");
}

#[test]
fn default_config_keeps_mouse_disabled() {
    assert!(!Config::default().mouse.enabled);
}

#[test]
fn default_config_text_lists_parseable_default_bindings() {
    let text = default_config_text();

    assert!(text.contains("# Appearance"));
    assert!(text.contains("# Terminal fallback overrides"));
    assert!(text.contains("# File and display limits"));
    assert!(text.contains("theme = msedit"));
    assert!(text.contains("mouse.enabled = false"));
    assert!(text.contains("key.app.help = F1"));
    assert!(text.contains("key.file_dialog.toggle_hidden = Ctrl+H"));
    parse_config(&text).unwrap().validate().unwrap();
}
