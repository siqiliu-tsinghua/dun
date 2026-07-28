//! Menu contribution: resolution, mnemonics, collisions, rejections.
//!
//! Split out of `tests/plugins.rs` when it passed the 45k test-file debt
//! threshold in docs/dev/code-organization-guidelines.md. Tests moved verbatim;
//! shared helpers stay in the parent module.

use super::*;

/// Entry mnemonics are the host's to choose; dun derives nothing. No general
/// rule exists — an IDE host's `Find References` and `Format Document` both
/// start with `F` and only its author knows which should own the key.
#[test]
fn declared_entry_mnemonics_are_composed_and_undeclared_ones_are_left_alone() {
    let menu = menu_with_declared_mnemonics(
        "Tools",
        None,
        &[
            ("Find References", "find", Some("R")),
            ("Format Document", "format", Some("F")),
            ("Plain", "plain", None),
        ],
    );
    let mut app = AppState::new();
    app.plugin_hosts = PluginHosts::for_tests(vec![started_menu_host("tools", menu)]);
    app.refresh_plugin_contributions();

    assert_eq!(
        rendered_plugin_entry_labels(&app),
        vec![
            "Find References (R)".to_string(),
            "Format Document (F)".to_string(),
            "Plain".to_string(),
        ],
        "declared mnemonics compose; an undeclared one stays bare"
    );
}

/// The entry matcher reads ONLY a trailing `(M)` — unlike the top-level one it
/// has no first-character fallback. So an entry suffix must be composed even
/// when the label already starts with that letter, or the key silently does
/// nothing. This asymmetry is exactly what a well-meaning refactor would
/// "simplify" away.
#[test]
fn an_entry_mnemonic_matching_its_own_first_letter_is_still_composed() {
    let menu = menu_with_declared_mnemonics("Tools", None, &[("Edit Pattern", "edit", Some("E"))]);
    let mut app = AppState::new();
    app.plugin_hosts = PluginHosts::for_tests(vec![started_menu_host("tools", menu)]);
    app.refresh_plugin_contributions();

    assert_eq!(
        rendered_plugin_entry_labels(&app),
        vec!["Edit Pattern (E)".to_string()],
        "without the suffix `entry_mnemonic` finds nothing and the key is dead"
    );
}

/// A duplicate drops only the later entry's *shortcut*. The entry itself stays
/// — arrows, Enter and the mouse still reach it — and its siblings are
/// untouched. A top-level collision is the case that rejects a whole subtree,
/// because there the menu becomes unreachable entirely.
#[test]
fn a_duplicate_entry_mnemonic_drops_only_the_later_shortcut() {
    let menu = menu_with_declared_mnemonics(
        "Tools",
        None,
        &[
            ("Edit Pattern", "edit", Some("E")),
            ("Export", "export", Some("E")),
            ("Apply", "apply", Some("A")),
        ],
    );
    let mut app = AppState::new();
    app.plugin_hosts = PluginHosts::for_tests(vec![started_menu_host("tools", menu)]);
    app.refresh_plugin_contributions();

    assert_eq!(
        rendered_plugin_entry_labels(&app),
        vec![
            "Edit Pattern (E)".to_string(),
            "Export".to_string(),
            "Apply (A)".to_string(),
        ],
        "the second E loses its key; it and its siblings survive"
    );
}

/// A host may pick a letter that is not the label's first: then the suffix is
/// mandatory even in plain English, because the top-level matcher would
/// otherwise derive the first character and the declared key would not work.
#[test]
fn a_declared_top_mnemonic_that_is_not_the_first_letter_is_shown_in_english() {
    let menu = menu_with_declared_mnemonics("Log Filter", Some("G"), &[("Run", "run", None)]);
    let mut app = AppState::new();
    app.plugin_hosts = PluginHosts::for_tests(vec![started_menu_host("logf", menu)]);
    app.refresh_plugin_contributions();

    let buffer_views = app.buffer_views();
    let menu = app
        .shell
        .frame_for_workspace(&app.workspace, Rect::new(0, 0, 120, 20), &buffer_views)
        .menu;
    assert_eq!(menu.items.last().unwrap().label, "Log Filter (G)");
    assert_eq!(
        app.shell.menu_index_for_mnemonic('g'),
        Some(4),
        "the declared key must actually open it"
    );
}

#[test]
fn translated_plugin_menu_opens_on_same_alt_chord_as_english() {
    let menu = menu_with_top_labels("Log Filter", &[("zh-CN", "日志过滤")]);

    let mut english = AppState::new();
    english.plugin_hosts =
        PluginHosts::for_tests(vec![started_menu_host("logfilter", menu.clone())]);
    english.pump_plugins();

    let mut translated = AppState::new();
    translated.plugin_menu_tags = vec!["zh-CN".to_string()];
    translated.plugin_hosts = PluginHosts::for_tests(vec![started_menu_host("logfilter", menu)]);
    translated.pump_plugins();

    assert_eq!(english.shell.menu_index_for_mnemonic('l'), Some(4));
    assert_eq!(
        translated.shell.menu_index_for_mnemonic('l'),
        english.shell.menu_index_for_mnemonic('l'),
        "the active translation must not change the plugin menu's Alt chord"
    );

    handle_key_event(
        &mut english,
        TerminalKeyEvent::new(TerminalKeyCode::Char('l'), TerminalKeyModifiers::ALT),
    );
    handle_key_event(
        &mut translated,
        TerminalKeyEvent::new(TerminalKeyCode::Char('l'), TerminalKeyModifiers::ALT),
    );
    assert_eq!(english.active_menu, Some(4));
    assert_eq!(translated.active_menu, Some(4));
}

#[test]
fn translated_plugin_menu_composes_the_english_mnemonic() {
    let menu = menu_with_top_labels("Log Filter", &[("zh-CN", "日志过滤"), ("fr", "Log Filter")]);
    let english = app_with_menu_contributions(vec![("logfilter", menu.clone())], &[]);
    let translated = app_with_menu_contributions(vec![("logfilter", menu.clone())], &["zh-CN"]);
    let equal_text_translation = app_with_menu_contributions(vec![("logfilter", menu)], &["fr"]);

    assert_eq!(english.shell.plugin_menu_items[0].label, "Log Filter");
    assert_eq!(translated.shell.plugin_menu_items[0].label, "日志过滤 (L)");
    assert_eq!(
        equal_text_translation.shell.plugin_menu_items[0].label, "Log Filter (L)",
        "translation selection is distinct from fallback even when text is equal"
    );
    assert_eq!(
        translated.shell.plugin_menu_items[0].entries[0].label, "Run",
        "dropdown entries remain unchanged and carry no new mnemonic"
    );
}

#[test]
fn plugin_menu_rejects_non_ascii_english_mnemonic() {
    let app = app_with_menu_contributions(
        vec![("non-ascii", menu_with_top_labels("日志过滤", &[]))],
        &[],
    );

    assert!(app.shell.plugin_menu_items.is_empty());
    assert_eq!(
        app.plugin_menu_rejections(),
        &[PluginMenuRejection {
            plugin_id: "non-ascii".to_string(),
            reason: PluginMenuRejectionReason::InvalidEnglishMnemonic,
        }]
    );
    assert_eq!(
        app.status_message.as_deref(),
        Some("Plugin non-ascii menu ignored: en_US label has no valid mnemonic")
    );
}

#[test]
fn plugin_menu_rejects_digit_english_mnemonic() {
    let app = app_with_menu_contributions(
        vec![("digit", menu_with_top_labels("1 Log Filter", &[]))],
        &[],
    );

    assert!(app.shell.plugin_menu_items.is_empty());
    assert_eq!(
        app.plugin_menu_rejections()[0].reason,
        PluginMenuRejectionReason::InvalidEnglishMnemonic
    );
    assert_eq!(
        app.status_message.as_deref(),
        Some("Plugin digit menu ignored: en_US label has no valid mnemonic")
    );
}

#[test]
fn plugin_menu_rejects_conflicting_embedded_english_mnemonic() {
    let app = app_with_menu_contributions(
        vec![("embedded", menu_with_top_labels("Log Filter (X) ", &[]))],
        &[],
    );

    assert!(app.shell.plugin_menu_items.is_empty());
    assert_eq!(
        app.plugin_menu_rejections()[0].reason,
        PluginMenuRejectionReason::InvalidEnglishMnemonic
    );
    assert_eq!(
        app.status_message.as_deref(),
        Some("Plugin embedded menu ignored: en_US label has no valid mnemonic")
    );
}

#[test]
fn matching_embedded_english_mnemonic_is_accepted() {
    let app = app_with_menu_contributions(
        vec![("matching", menu_with_top_labels("Log Filter (L)", &[]))],
        &[],
    );

    assert_eq!(app.shell.plugin_menu_items[0].label, "Log Filter (L)");
    assert!(app.plugin_menu_rejections().is_empty());
    assert!(app.status_message.is_none());
}

#[test]
fn plugin_menu_colliding_with_builtin_is_rejected_and_reported() {
    let app = app_with_menu_contributions(
        vec![("files-extra", menu_with_top_labels("Files Extra", &[]))],
        &[],
    );

    assert_eq!(
        app.shell.menu_index_for_mnemonic('f'),
        Some(0),
        "File must remain the first menu"
    );
    assert_eq!(app.shell.menu_count(), 4);
    assert!(app.shell.plugin_menu_items.is_empty());
    assert_eq!(
        app.plugin_menu_rejections(),
        &[PluginMenuRejection {
            plugin_id: "files-extra".to_string(),
            reason: PluginMenuRejectionReason::MnemonicConflict('F'),
        }]
    );
    assert_eq!(
        app.status_message.as_deref(),
        Some("Plugin files-extra menu ignored: mnemonic F conflicts")
    );
}

#[test]
fn later_plugin_with_duplicate_mnemonic_is_rejected_and_reported() {
    let mut app = app_with_menu_contributions(
        vec![
            ("alpha", menu_with_top_labels("Log Filter", &[])),
            ("beta", menu_with_top_labels("Language", &[])),
        ],
        &[],
    );

    assert_eq!(app.shell.plugin_menu_items.len(), 1);
    assert_eq!(app.shell.plugin_menu_items[0].label, "Log Filter");
    assert_eq!(
        app.plugin_menu_rejections(),
        &[PluginMenuRejection {
            plugin_id: "beta".to_string(),
            reason: PluginMenuRejectionReason::MnemonicConflict('L'),
        }]
    );
    assert_eq!(
        app.status_message.as_deref(),
        Some("Plugin beta menu ignored: mnemonic L conflicts")
    );
    assert_eq!(app.status_history.len(), 1);

    app.refresh_plugin_contributions();
    assert_eq!(
        app.status_history.len(),
        1,
        "an unchanged rejection must be reported only once"
    );
}

#[test]
fn unloading_first_claimant_promotes_later_plugin_menu() {
    let alpha_menu = menu_with_top_labels("Log Filter", &[]);
    let beta_menu = menu_with_top_labels("Language", &[]);
    let (mut alpha, _alpha_messages, alpha_events) =
        PluginHost::for_tests_granted("alpha", eager_grant());
    alpha_events
        .send(HostEvent::Started {
            menu: Some(alpha_menu.clone()),
            keybinding: None,
        })
        .unwrap();
    assert!(alpha.poll().is_empty());
    let beta = started_menu_host("beta", beta_menu);

    let mut app = AppState::new();
    app.plugin_hosts = PluginHosts::for_tests(vec![alpha, beta]);
    app.refresh_plugin_contributions();
    assert_eq!(app.shell.plugin_menu_items[0].label, "Log Filter");
    assert_eq!(app.plugin_menu_rejections()[0].plugin_id, "beta");

    app.plugin_hosts.get_mut("alpha").unwrap().unload();
    app.refresh_plugin_contributions();
    assert_eq!(app.shell.plugin_menu_items.len(), 1);
    assert_eq!(app.shell.plugin_menu_items[0].label, "Language");
    assert!(app.plugin_menu_rejections().is_empty());

    app.plugin_hosts.get_mut("alpha").unwrap().load();
    alpha_events
        .send(HostEvent::Started {
            menu: Some(alpha_menu),
            keybinding: None,
        })
        .unwrap();
    app.pump_plugins();
    assert_eq!(app.shell.plugin_menu_items[0].label, "Log Filter");
    assert_eq!(app.plugin_menu_rejections()[0].plugin_id, "beta");
    assert_eq!(
        app.status_history
            .iter()
            .filter(|entry| entry.message == "Plugin beta menu ignored: mnemonic L conflicts")
            .count(),
        2,
        "a rejection may report again after clearing"
    );
}

#[test]
fn accepted_plugin_menu_reports_no_rejection() {
    let app = app_with_menu_contributions(
        vec![
            ("tools", menu_with_top_labels("Tools", &[])),
            ("logs", menu_with_top_labels("Logs", &[])),
        ],
        &[],
    );

    assert_eq!(
        app.shell
            .plugin_menu_items
            .iter()
            .map(|item| item.label.as_ref())
            .collect::<Vec<_>>(),
        ["Tools", "Logs"]
    );
    assert!(app.plugin_menu_rejections().is_empty());
    assert!(app.status_message.is_none());
    assert!(app.status_history.is_empty());
}

#[test]
fn new_plugin_menu_rejections_are_each_reported_once() {
    let mut app = app_with_menu_contributions(
        vec![
            ("invalid", menu_with_top_labels("1 Invalid", &[])),
            ("conflict", menu_with_top_labels("File Extra", &[])),
        ],
        &[],
    );

    assert_eq!(app.plugin_menu_rejections().len(), 2);
    assert_eq!(
        app.status_history
            .iter()
            .map(|entry| entry.message.as_str())
            .collect::<Vec<_>>(),
        [
            "Plugin invalid menu ignored: en_US label has no valid mnemonic",
            "Plugin conflict menu ignored: mnemonic F conflicts",
        ]
    );

    app.refresh_plugin_contributions();
    assert_eq!(app.status_history.len(), 2);
}

#[test]
fn translated_plugin_menu_is_mouse_hittable_at_its_rendered_columns() {
    let app = app_with_menu_contributions(
        vec![(
            "logfilter",
            menu_with_top_labels("Log Filter", &[("zh-CN", "日志过滤")]),
        )],
        &["zh-CN"],
    );
    let buffer_views = app.buffer_views();
    let frame =
        app.shell
            .frame_for_workspace(&app.workspace, Rect::new(0, 0, 80, 20), &buffer_views);
    assert_eq!(
        frame
            .menu
            .items
            .iter()
            .map(|item| item.label.as_ref())
            .collect::<Vec<_>>(),
        ["File", "Edit", "View", "Help", "日志过滤 (L)"]
    );

    assert_eq!(app.shell.menu_index_at_column(24), Some(3));
    for column in [25, 26, 32, 38] {
        assert_eq!(
            app.shell.menu_index_at_column(column),
            Some(4),
            "rendered plugin-menu column {column} must be clickable"
        );
    }
    assert_eq!(app.shell.menu_index_at_column(39), None);
}

#[test]
fn started_event_installs_the_menu_and_unload_clears_it() {
    let (mut host, _messages, events) = PluginHost::for_tests_granted("menu-host", eager_grant());
    let menu = sample_menu();
    events
        .send(HostEvent::Started {
            menu: Some(menu.clone()),
            keybinding: None,
        })
        .unwrap();
    assert!(host.poll().is_empty(), "handshake events are absorbed");

    let mut hosts = PluginHosts::for_tests(vec![host]);
    assert_eq!(
        hosts.menus().collect::<Vec<_>>(),
        vec![("menu-host", &menu)]
    );

    hosts.get_mut("menu-host").unwrap().unload();
    assert_eq!(
        hosts.menus().count(),
        0,
        "an unloaded host must contribute no menu"
    );
}

#[test]
fn started_event_injects_the_resolved_menu_into_the_menu_bar() {
    let mut app = AppState::new();
    let (mut host, _messages, events) = PluginHost::for_tests_granted("menu-host", eager_grant());
    events
        .send(HostEvent::Started {
            menu: Some(sample_menu()),
            keybinding: None,
        })
        .unwrap();
    assert!(host.poll().is_empty(), "handshake events are absorbed");
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);

    app.pump_plugins();

    // The handshake's menu is resolved onto the shell (dun-ui's own tests cover
    // that these items trail the built-in menus in the rendered bar). The entry
    // carries a PluginAction tagged by plugin and action id.
    assert_eq!(app.shell.plugin_menu_items.len(), 1);
    let injected = &app.shell.plugin_menu_items[0];
    assert_eq!(injected.label, "Tools");
    assert_eq!(injected.entries[0].label, "Run");
    assert_eq!(
        injected.entries[0].command,
        EditorCommand::PluginAction {
            plugin_id: "menu-host".into(),
            action_id: "run".into(),
            kind: PluginActionKind::Surface,
        }
    );
}

#[test]
fn unloading_a_host_removes_its_injected_menu() {
    let mut app = AppState::new();
    let (mut host, _messages, events) = PluginHost::for_tests_granted("menu-host", eager_grant());
    events
        .send(HostEvent::Started {
            menu: Some(sample_menu()),
            keybinding: None,
        })
        .unwrap();
    assert!(host.poll().is_empty());
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);
    app.pump_plugins();
    assert_eq!(app.shell.plugin_menu_items.len(), 1);

    app.run_command_line("plugin unload");

    assert!(
        app.shell.plugin_menu_items.is_empty(),
        "an unloaded host's menu must disappear from the bar at once"
    );
}
