use super::support::*;

#[test]
fn parses_line_based_config_overlay() {
    let config = parse_config(
        "\
# Dun config
theme = dark
terminal.encoding = ascii
terminal.colors = mono
mouse.enabled = true
clipboard.osc52.enabled = true
clipboard.osc52.max_bytes = 2 KiB
limits.editable_file_soft_limit_bytes = 2 MiB
limits.line_display_soft_limit_bytes = 4 KiB
key.app.quit = Esc
key.edit.find = none
key.file_dialog.toggle_hidden = F8
file_dialog.key.delete_forward = none
",
    )
    .unwrap();

    assert_eq!(config.theme, ThemeName::Dark);
    assert_eq!(config.terminal.encoding, Some(EncodingProfile::Ascii));
    assert_eq!(config.terminal.colors, Some(ColorProfile::Mono));
    assert!(config.mouse.enabled);
    assert!(config.clipboard.osc52.enabled);
    assert_eq!(config.clipboard.osc52.max_bytes, 2 * 1024);
    assert_eq!(
        config.limits.editable_file_soft_limit_bytes,
        2 * 1024 * 1024
    );
    assert_eq!(config.limits.line_display_soft_limit_bytes, 4 * 1024);
    assert_eq!(
        config
            .keybindings
            .command_for_sequence(&KeySequence::from_str("Esc").unwrap()),
        Some(&EditorCommand::App(AppCommand::Quit))
    );
    assert!(
        !config
            .keybindings
            .bindings
            .iter()
            .any(|binding| binding.command == EditorCommand::Edit(EditCommand::Find))
    );
    assert_eq!(
        config
            .file_dialog_keys
            .action_for_stroke(KeyStroke::from_str("F8").unwrap()),
        Some(FileDialogAction::ToggleHidden)
    );
    assert_eq!(
        config
            .file_dialog_keys
            .stroke_for_action(FileDialogAction::DeleteForward),
        None
    );
}

#[test]
fn config_parser_reports_line_errors() {
    let error = parse_config("bad = value").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("unknown config key"));

    let error = parse_config("key.app.nope = Ctrl+X").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("unknown command id"));

    let error = parse_config("key.file_dialog.nope = Ctrl+X").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("unknown file dialog action"));

    let error = parse_config("key.file_dialog.cancel = Ctrl+W,Q").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("single key stroke"));

    let error = parse_config("mouse.enabled = maybe").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("expected true or false"));

    let error = parse_config("missing separator").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("expected `key = value` entry"));

    let error = parse_config(" = value").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("empty config key"));

    let error = parse_config("theme = unknown").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("unknown theme name"));

    let error = parse_config("terminal.encoding = latin1").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("unknown terminal encoding"));

    let error = parse_config("terminal.colors = truecolor").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("unknown terminal colors"));

    let error = parse_config("key.edit.find = Ctrl+").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("invalid key sequence"));

    let error = parse_config("key.file_dialog.cancel = Ctrl+").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("invalid key sequence"));
}

#[test]
fn config_parser_rejects_duplicate_resulting_keybindings() {
    let error = parse_config("key.app.quit = Ctrl+S").unwrap_err();

    assert_eq!(error.line, None);
    assert!(error.to_string().contains("duplicate key sequence"));

    let error = parse_config("key.file_dialog.cancel = Enter").unwrap_err();

    assert_eq!(error.line, None);
    assert!(error.to_string().contains("duplicate key stroke"));
}

#[test]
fn config_parser_replaces_all_default_aliases_for_command() {
    let config = parse_config("key.window.focus_left = Ctrl+W,A").unwrap();

    assert_eq!(
        config
            .keybindings
            .command_for_sequence(&KeySequence::from_str("Ctrl+W,A").unwrap()),
        Some(&EditorCommand::Window(WindowCommand::FocusLeft))
    );
    assert_eq!(
        config
            .keybindings
            .command_for_sequence(&KeySequence::from_str("Ctrl+W,Left").unwrap()),
        None
    );
    assert_eq!(
        config
            .keybindings
            .command_for_sequence(&KeySequence::from_str("Alt+Left").unwrap()),
        None
    );
}

#[test]
fn config_parser_accepts_aliases_quotes_and_byte_suffixes() {
    let config = parse_config(
        "\
theme = \"turbo-vision\"
terminal.encoding = 'utf8'
terminal.color = 16-color
mouse.enabled = yes
clipboard.osc52 = disabled
clipboard.osc52.max_bytes = 3 kb
limits.editable_file_soft_limit_bytes = 1_024 bytes
limits.line_display_soft_limit_bytes = 5KiB
",
    )
    .unwrap();

    assert_eq!(config.theme, ThemeName::Turbo);
    assert_eq!(config.terminal.encoding, Some(EncodingProfile::Utf8));
    assert_eq!(config.terminal.colors, Some(ColorProfile::Color16));
    assert!(config.mouse.enabled);
    assert!(!config.clipboard.osc52.enabled);
    assert_eq!(config.clipboard.osc52.max_bytes, 3 * 1024);
    assert_eq!(config.limits.editable_file_soft_limit_bytes, 1024);
    assert_eq!(config.limits.line_display_soft_limit_bytes, 5 * 1024);

    assert_eq!(
        parse_config("theme = microsoft edit").unwrap().theme,
        ThemeName::MsEdit
    );
    assert_eq!(parse_config("theme = dun").unwrap().theme, ThemeName::Dun);
    assert_eq!(
        parse_config("terminal.colors = 256-color")
            .unwrap()
            .terminal
            .colors,
        Some(ColorProfile::Color256)
    );
    assert_eq!(
        parse_config("terminal.colors = off")
            .unwrap()
            .terminal
            .colors,
        Some(ColorProfile::Mono)
    );
    assert_eq!(
        parse_config("terminal.encoding = ascii")
            .unwrap()
            .terminal
            .encoding,
        Some(EncodingProfile::Ascii)
    );
    assert_eq!(
        parse_config("limits.editable_file_soft_limit_bytes = 1 mib")
            .unwrap()
            .limits
            .editable_file_soft_limit_bytes,
        1024 * 1024
    );
    assert_eq!(
        parse_config("limits.editable_file_soft_limit_bytes = 1gb")
            .unwrap()
            .limits
            .editable_file_soft_limit_bytes,
        1024 * 1024 * 1024
    );
}

#[test]
fn config_parser_reports_byte_count_errors() {
    let error = parse_config("clipboard.osc52.max_bytes = bytes").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("expected byte count"));

    let error = parse_config("clipboard.osc52.max_bytes = 1 tb").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("unknown byte-count suffix"));

    let error = parse_config("clipboard.osc52.max_bytes = 18446744073709551615 gb").unwrap_err();

    assert_eq!(error.line, Some(1));
    assert!(error.to_string().contains("outside the supported range"));
}
