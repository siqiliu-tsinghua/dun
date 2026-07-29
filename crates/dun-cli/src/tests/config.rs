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
    assert!(
        error
            .to_string()
            .contains("`Ctrl+S` is bound to both `file.save` and `app.quit`")
    );
    assert!(error.to_string().contains("`key.file.save = none`"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn configured_help_binding_replaces_default_runtime_binding() {
    let config = parse_config("key.app.help = F10").unwrap();
    let mut app = AppState::from_config(config);

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::F(1), TerminalKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 1);

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::F(10), TerminalKeyModifiers::NONE),
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
        TerminalKeyEvent::new(TerminalKeyCode::F(1), TerminalKeyModifiers::NONE),
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
plugins.status_bar = true
plugins.idle_after_ms = 60000
key.app.help = F10
",
    )
    .unwrap();

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::F(5), TerminalKeyModifiers::NONE),
    );

    assert_eq!(
        app.status_message,
        Some(format!("Config reloaded from {}", path.display()))
    );
    assert_eq!(app.limits.editable_file_soft_limit_bytes, 8 * 1024);
    assert!(app.mouse_enabled());
    assert!(app.plugin_status.status_bar);
    assert_eq!(app.plugin_status.idle_after_ms, 60_000);
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "x");

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::F(1), TerminalKeyModifiers::NONE),
    );
    assert_eq!(app.workspace.window_count(), 1);

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::F(10), TerminalKeyModifiers::NONE),
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
        TerminalKeyEvent::new(TerminalKeyCode::F(5), TerminalKeyModifiers::NONE),
    );

    assert!(
        app.status_message
            .as_ref()
            .is_some_and(|message| message.starts_with("Config reload failed:"))
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::F(1), TerminalKeyModifiers::NONE),
    );
    assert_eq!(app.workspace.window_count(), 1);

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::F(10), TerminalKeyModifiers::NONE),
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
        TerminalKeyEvent::new(TerminalKeyCode::Right, TerminalKeyModifiers::SHIFT),
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

/// The installed layer (`<bin>/../share/dun/config`) is a base the user's
/// file is applied *on top of*, not an alternative to it. A system-wide
/// install would otherwise be all-or-nothing: any personal setting would
/// throw away every machine-wide one.
#[test]
fn installed_config_is_a_base_layer_the_user_file_overlays() {
    let installed = temp_file_path("installed-base-config");
    let user = temp_file_path("user-over-base-config");
    std::fs::write(
        &installed,
        "theme = turbo\nkey.app.help = F9\nmouse.enabled = false\n",
    )
    .unwrap();
    // The user changes the theme and nothing else.
    std::fs::write(&user, "theme = dark\n").unwrap();

    let request = ConfigLoadRequest::explicit(user.clone());
    let loaded = load_config_from(&request, Some(installed.clone())).unwrap();

    assert_eq!(loaded.config.theme, ThemeName::Dark, "user file wins");
    assert!(
        !loaded.config.mouse.enabled,
        "an installed setting the user did not mention must survive"
    );
    assert_eq!(
        loaded
            .config
            .keybindings
            .command_for_sequence(&KeySequence::from_str("F9").unwrap()),
        Some(&EditorCommand::App(AppCommand::Help)),
        "an installed keybinding the user did not mention must survive"
    );
    assert_eq!(loaded.base, Some(installed.clone()));
    assert!(loaded.base_diagnostic.is_none());

    let _ = std::fs::remove_file(&installed);
    let _ = std::fs::remove_file(&user);
}

/// With no user file at all, the installed layer is still what is running,
/// and saying "built-in defaults" would be untrue.
#[test]
fn installed_config_applies_without_a_user_file_and_is_named_as_the_source() {
    let installed = temp_file_path("installed-only-config");
    std::fs::write(&installed, "theme = turbo\n").unwrap();

    let request = ConfigLoadRequest::new(None, false);
    let loaded = load_config_from(&request, Some(installed.clone())).unwrap();

    // The user layer is whatever discovery finds on this machine, so only
    // the base assertions are safe here.
    assert_eq!(loaded.base, Some(installed.clone()));
    if matches!(loaded.source, ConfigSource::BuiltInDefaults) {
        assert_eq!(loaded.config.theme, ThemeName::Turbo);
        assert_eq!(
            loaded.status_source(),
            ConfigSource::DefaultFile(installed.clone())
        );
    }

    let _ = std::fs::remove_file(&installed);
}

/// A machine-wide file is not the user's to fix. A broken one reports and
/// steps aside; it must never be the reason somebody cannot start dun.
#[test]
fn a_broken_installed_config_reports_and_does_not_stop_startup() {
    let installed = temp_file_path("broken-installed-config");
    let user = temp_file_path("user-over-broken-config");
    std::fs::write(&installed, "theme = no-such-theme\n").unwrap();
    std::fs::write(&user, "mouse.enabled = false\n").unwrap();

    let request = ConfigLoadRequest::explicit(user.clone());
    let loaded = load_config_from(&request, Some(installed.clone())).unwrap();

    assert!(loaded.base.is_none(), "a rejected base is not applied");
    let diagnostic = loaded.base_diagnostic.expect("reports the broken file");
    assert!(
        diagnostic.contains("broken-installed-config"),
        "{diagnostic}"
    );
    assert!(
        !loaded.config.mouse.enabled,
        "the user's own file still applies"
    );
    assert_eq!(
        loaded.config.theme,
        Config::default().theme,
        "and the rest falls back to built-in defaults"
    );

    let _ = std::fs::remove_file(&installed);
    let _ = std::fs::remove_file(&user);
}

/// The user's own file is a different matter: they can fix it, and quietly
/// ignoring what they wrote would be worse than refusing to start.
#[test]
fn a_broken_user_config_is_still_a_startup_error() {
    let installed = temp_file_path("good-installed-config");
    let user = temp_file_path("broken-user-config");
    std::fs::write(&installed, "theme = turbo\n").unwrap();
    std::fs::write(&user, "theme = no-such-theme\n").unwrap();

    let request = ConfigLoadRequest::explicit(user.clone());
    let error = load_config_from(&request, Some(installed.clone())).unwrap_err();
    assert!(error.to_string().contains("broken-user-config"), "{error}");

    let _ = std::fs::remove_file(&installed);
    let _ = std::fs::remove_file(&user);
}

#[test]
fn no_config_disables_the_installed_layer_too() {
    let installed = temp_file_path("ignored-installed-config");
    std::fs::write(&installed, "theme = turbo\n").unwrap();

    let request = ConfigLoadRequest::new(None, true);
    let loaded = load_config_from(&request, Some(installed.clone())).unwrap();

    assert_eq!(loaded.config.theme, Config::default().theme);
    assert!(loaded.base.is_none());
    assert_eq!(loaded.source, ConfigSource::Disabled);

    let _ = std::fs::remove_file(&installed);
}

/// The installed layer and its catalogs are two names in one directory, so
/// they can never drift apart. Hardcoded prefixes, not the joins the
/// implementation uses.
#[test]
fn installed_share_directory_is_a_sibling_of_bin() {
    for (exe, want) in [
        ("/opt/dun/bin/dun", Some("/opt/dun/share/dun")),
        ("/usr/bin/dun", Some("/usr/share/dun")),
        ("/home/u/.local/bin/dun", Some("/home/u/.local/share/dun")),
        ("/dun", None),
    ] {
        assert_eq!(
            installed_share_dir_for_exe(std::path::Path::new(exe)),
            want.map(std::path::PathBuf::from),
            "executable {exe}"
        );
    }
}
