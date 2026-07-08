#![allow(unused_imports)]

use super::support::*;

#[test]
fn load_startup_config_reads_explicit_config_path() {
    let path = temp_file_path("dun-config");
    std::fs::write(
        &path,
        "\
theme = dark
limits.editable_file_soft_limit_bytes = 3 KiB
key.app.quit = Esc
",
    )
    .unwrap();

    let config = load_startup_config(Some(&path), false).unwrap();

    assert_eq!(config.theme, dun_config::ThemeName::Dark);
    assert_eq!(config.limits.editable_file_soft_limit_bytes, 3 * 1024);
    assert_eq!(
        config
            .keybindings
            .command_for_sequence(&KeySequence::from_str("Esc").unwrap()),
        Some(&EditorCommand::App(AppCommand::Quit))
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn load_startup_config_reports_parse_errors_with_path() {
    let path = temp_file_path("bad-dun-config");
    std::fs::write(&path, "bad = value").unwrap();

    let error = load_startup_config(Some(&path), false).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains(path.to_string_lossy().as_ref()));
    assert!(error.to_string().contains("line 1"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn load_startup_config_reports_keybinding_conflicts_with_path() {
    let path = temp_file_path("conflicting-dun-config");
    std::fs::write(&path, "key.app.quit = Ctrl+S").unwrap();

    let error = load_startup_config(Some(&path), false).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains(path.to_string_lossy().as_ref()));
    assert!(error.to_string().contains("duplicate key sequence"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn configured_help_binding_replaces_default_runtime_binding() {
    let config = parse_config("key.app.help = F10").unwrap();
    let mut app = AppState::from_config(config);

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(1), CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 1);

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(10), CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::Help
    );
}

#[test]
fn configured_disabled_keybinding_is_not_dispatched() {
    let config = parse_config("key.app.help = none").unwrap();
    let mut app = AppState::from_config(config);

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(1), CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 1);
}

#[test]
fn command_line_theme_switches_runtime_theme_and_refreshes_diagnostics() {
    let mut app = AppState::new();
    app.shell.profile = TerminalProfile::utf8_256();

    app.handle_command(&EditorCommand::App(AppCommand::ConfigDiagnostics));
    let diagnostics_buffer_id = app.workspace.focused_window().unwrap().buffer_id;

    submit_command_line(&mut app, "theme dark");

    assert_eq!(app.shell.theme.theme, ThemeName::Dark);
    assert_eq!(app.status_message, Some("Theme: dark".to_string()));
    assert!(
        app.buffer_state(diagnostics_buffer_id)
            .unwrap()
            .buffer
            .to_text()
            .contains("theme: dark")
    );

    submit_command_line(&mut app, "theme");

    assert!(
        app.status_message
            .as_ref()
            .is_some_and(|message| message.starts_with("Theme: dark"))
    );
}

#[test]
fn command_line_theme_reports_unknown_theme() {
    let mut app = AppState::new();
    let original_theme = app.shell.theme.theme;

    submit_command_line(&mut app, "theme unknown");

    assert_eq!(app.shell.theme.theme, original_theme);
    assert_eq!(
        app.status_message,
        Some("Theme failed: unknown theme unknown; expected msedit|turbo|dark|dun".to_string())
    );
}

#[test]
fn reload_config_restores_configured_theme_after_runtime_theme_switch() {
    let path = temp_file_path("theme-reload-config");
    std::fs::write(&path, "theme = turbo\n").unwrap();
    let mut app = app_from_config_path(path.clone());
    app.detected_profile = TerminalProfile::utf8_256();

    submit_command_line(&mut app, "reload-config");
    assert_eq!(app.shell.theme.theme, ThemeName::Turbo);

    submit_command_line(&mut app, "theme dark");
    assert_eq!(app.shell.theme.theme, ThemeName::Dark);

    submit_command_line(&mut app, "reload-config");
    assert_eq!(app.shell.theme.theme, ThemeName::Turbo);

    let _ = std::fs::remove_file(path);
}

#[test]
fn reload_config_applies_updated_keymap_and_limits_without_resetting_buffers() {
    let path = temp_file_path("reload-config");
    std::fs::write(&path, "limits.editable_file_soft_limit_bytes = 4 KiB\n").unwrap();
    let mut app = app_from_config_path(path.clone());
    app.handle_text_input('x');

    std::fs::write(
        &path,
        "\
limits.editable_file_soft_limit_bytes = 8 KiB
mouse.enabled = true
key.app.help = F10
",
    )
    .unwrap();

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(5), CrosstermKeyModifiers::NONE),
    );

    assert_eq!(
        app.status_message,
        Some(format!("Config reloaded from {}", path.display()))
    );
    assert_eq!(app.limits.editable_file_soft_limit_bytes, 8 * 1024);
    assert!(app.mouse_enabled());
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "x");

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(1), CrosstermKeyModifiers::NONE),
    );
    assert_eq!(app.workspace.window_count(), 1);

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(10), CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::Help
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn reload_config_refreshes_open_config_diagnostics_screen() {
    let path = temp_file_path("reload-config-diagnostics");
    std::fs::write(&path, "\n").unwrap();
    let mut app = app_from_config_path(path.clone());

    app.handle_command(&EditorCommand::App(AppCommand::ConfigDiagnostics));
    let config_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    let text = app.buffer_state(config_buffer_id).unwrap().buffer.to_text();
    assert!(keymap_command_line(&text, "app.help").contains("F1"));

    std::fs::write(&path, "key.app.help = F10\n").unwrap();
    app.handle_command(&EditorCommand::App(AppCommand::ReloadConfig));

    let text = app.buffer_state(config_buffer_id).unwrap().buffer.to_text();
    let line = keymap_command_line(&text, "app.help");
    assert!(line.contains("F10"));
    assert!(!line.contains("F1 "));

    let _ = std::fs::remove_file(path);
}

#[test]
fn reload_config_failure_keeps_previous_keymap() {
    let path = temp_file_path("bad-reload-config");
    std::fs::write(&path, "key.app.help = F10\n").unwrap();
    let mut app = app_from_config_path(path.clone());

    std::fs::write(&path, "bad = value\n").unwrap();
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(5), CrosstermKeyModifiers::NONE),
    );

    assert!(
        app.status_message
            .as_ref()
            .is_some_and(|message| message.starts_with("Config reload failed:"))
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(1), CrosstermKeyModifiers::NONE),
    );
    assert_eq!(app.workspace.window_count(), 1);

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(10), CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::Help
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn reload_config_refreshes_open_help_screen() {
    let path = temp_file_path("reload-help-config");
    std::fs::write(&path, "\n").unwrap();
    let mut app = app_from_config_path(path.clone());

    app.handle_command(&EditorCommand::App(AppCommand::Help));
    let help_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    let text = app.buffer_state(help_buffer_id).unwrap().buffer.to_text();
    assert!(help_command_line(&text).contains("F1"));

    std::fs::write(&path, "key.app.help = F10\n").unwrap();
    app.handle_command(&EditorCommand::App(AppCommand::ReloadConfig));

    let text = app.buffer_state(help_buffer_id).unwrap().buffer.to_text();
    let line = help_command_line(&text);
    assert!(line.contains("F10"));
    assert!(!line.contains("F1 "));

    let _ = std::fs::remove_file(path);
}

#[test]
fn configured_shift_arrow_binding_wins_before_selection_fallback() {
    let config = parse_config("key.window.split_horizontal = Shift+Right").unwrap();
    let mut app = AppState::from_config(config);
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));
    app.handle_text_input('a');

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Right, CrosstermKeyModifiers::SHIFT),
    );

    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(app.status_message, Some("Split horizontally".to_string()));
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .selection_range(),
        None
    );
}
