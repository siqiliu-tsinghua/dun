//! Keybinding contribution: the reserved leader and chord claims.
//!
//! Split out of `tests/plugins.rs` when it passed the 45k test-file debt
//! threshold in docs/code-organization-guidelines.md. Tests moved verbatim;
//! shared helpers stay in the parent module.

use super::*;

#[test]
fn keybinding_leader_chord_dispatches_a_plugin_action() {
    let mut app = app_with_keybinding_host("logf", keybinding("Ctrl+J", "o", "open"));
    assert_eq!(app.shell.plugin_keymap.bindings.len(), 1);

    // The host's own "Ctrl+J" is ignored: dun binds every plugin under its
    // reserved leader, so the sequence is Ctrl+T then the chord.
    assert!(app.handle_key_stroke(stroke("Ctrl+T")));
    assert_eq!(surface_window_count(&app), 0);

    // The chord completes the sequence and dispatches the plugin action, which
    // opens the plugin's surface window (the host holds `window`).
    assert!(app.handle_key_stroke(stroke("o")));
    assert_eq!(surface_window_count(&app), 1);
    assert_eq!(app.plugin_windows.count("logf"), 1);
}

#[test]
fn keybinding_leader_then_unbound_key_cancels_without_dispatch() {
    let mut app = app_with_keybinding_host("logf", keybinding("Ctrl+J", "o", "open"));

    assert!(app.handle_key_stroke(stroke("Ctrl+T")));
    // A key that is not a chord under the leader cancels the pending prefix.
    app.handle_key_stroke(stroke("z"));
    assert_eq!(surface_window_count(&app), 0);
    assert_eq!(app.plugin_windows.count("logf"), 0);
}

#[test]
fn plugin_keybindings_are_dropped_when_the_reserved_leader_is_taken() {
    // A plugin can no longer collide with an editor key by choosing a bad
    // leader -- it does not choose one. The remaining way to lose the prefix
    // is the user binding it themselves, and then every plugin loses at once.
    let mut app = AppState::new();
    app.shell.keymap.bindings.push(dun_config::KeyBinding {
        sequence: KeySequence::single(stroke(crate::plugins::PLUGIN_LEADER)),
        command: EditorCommand::App(dun_core::AppCommand::Help),
    });
    let (mut host, _messages, events) = PluginHost::for_tests_granted("logf", eager_grant());
    events
        .send(HostEvent::Started {
            menu: None,
            keybinding: Some(keybinding("Ctrl+J", "o", "open")),
        })
        .unwrap();
    assert!(host.poll().is_empty());
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);
    app.pump_plugins();

    assert!(
        app.shell.plugin_keymap.bindings.is_empty(),
        "no plugin may bind when the reserved leader is not free"
    );
    assert_eq!(
        app.status_message,
        Some("Plugin keybindings disabled: Ctrl+T is bound in your keymap".to_string()),
        "the reported cause must be the leader, not a chord clash"
    );
}

#[test]
fn a_rejected_keybinding_reports_a_status_message() {
    // Two plugins want the same chord under the shared leader. The later one
    // is dropped -- a silent no-op before -- so the user gets a status message
    // naming it and can fix it in that plugin's own config.
    let mut app = AppState::new();
    let (mut alpha, _am, alpha_events) = PluginHost::for_tests_granted("alpha", eager_grant());
    let (mut logf, _lm, logf_events) = PluginHost::for_tests_granted("logf", eager_grant());
    alpha_events
        .send(HostEvent::Started {
            menu: None,
            keybinding: Some(keybinding("Ctrl+J", "o", "alpha-open")),
        })
        .unwrap();
    logf_events
        .send(HostEvent::Started {
            menu: None,
            keybinding: Some(keybinding("Ctrl+J", "o", "open")),
        })
        .unwrap();
    assert!(alpha.poll().is_empty());
    assert!(logf.poll().is_empty());
    app.plugin_hosts = PluginHosts::for_tests(vec![alpha, logf]);
    app.pump_plugins();

    assert_eq!(
        app.shell.plugin_keymap.bindings.len(),
        1,
        "the first claimant keeps the chord"
    );
    assert_eq!(
        app.status_message,
        Some("Plugin logf keybinding ignored: chord already claimed".to_string()),
        "a rejected keybinding must be reported, not silent"
    );
    assert_eq!(
        app.shell.plugin_keybinding_rejections,
        vec!["logf".to_string()]
    );
}

#[test]
fn an_accepted_keybinding_reports_nothing() {
    // The good-path counterpart: a free leader installs and stays silent.
    let app = app_with_keybinding_host("logf", keybinding("Ctrl+J", "o", "open"));
    assert_eq!(app.shell.plugin_keymap.bindings.len(), 1);
    assert!(
        app.shell.plugin_keybinding_rejections.is_empty(),
        "an accepted keybinding must not be reported as rejected"
    );
}

#[test]
fn two_plugins_cannot_claim_the_same_chord() {
    let mut app = AppState::new();
    let (mut alpha, _am, alpha_events) = PluginHost::for_tests_granted("alpha", eager_grant());
    let (mut beta, _bm, beta_events) = PluginHost::for_tests_granted("beta", eager_grant());
    alpha_events
        .send(HostEvent::Started {
            menu: None,
            keybinding: Some(keybinding("Ctrl+J", "a", "alpha-open")),
        })
        .unwrap();
    beta_events
        .send(HostEvent::Started {
            menu: None,
            keybinding: Some(keybinding("Ctrl+J", "a", "beta-open")),
        })
        .unwrap();
    assert!(alpha.poll().is_empty());
    assert!(beta.poll().is_empty());
    app.plugin_hosts = PluginHosts::for_tests(vec![alpha, beta]);
    app.pump_plugins();

    // Both bind under the same reserved leader, so the clash is the chord
    // itself; only the first claimant in config order keeps it.
    assert_eq!(app.shell.plugin_keymap.bindings.len(), 1);
    assert_eq!(
        app.shell.plugin_keymap.bindings[0].command,
        plugin_action("alpha", "alpha-open")
    );
}

#[test]
fn unloading_a_host_removes_its_keybindings() {
    let mut app = app_with_keybinding_host("logf", keybinding("Ctrl+J", "o", "open"));
    assert_eq!(app.shell.plugin_keymap.bindings.len(), 1);

    app.run_command_line("plugin unload");

    assert!(
        app.shell.plugin_keymap.bindings.is_empty(),
        "an unloaded host contributes no keybindings"
    );
}
