//! Host lifecycle: launch policy, load/unload, activity, config wiring.
//!
//! Split out of `tests/plugins.rs` when it passed the 45k test-file debt
//! threshold in docs/code-organization-guidelines.md. Tests moved verbatim;
//! shared helpers stay in the parent module.

use super::*;

#[test]
fn plugin_activity_tracks_jobs_failures_unload_and_disabled_idle_threshold() {
    let (mut host, _messages, events) = PluginHost::for_tests();

    assert!(host.schedule(job(0, 0, 3)));
    let active_now = Instant::now();
    assert_eq!(
        host.activity_at(active_now, Some(Duration::from_secs(10))),
        PluginActivity::Active
    );
    assert_eq!(
        host.activity_at(
            active_now + Duration::from_secs(11),
            Some(Duration::from_secs(10))
        ),
        PluginActivity::Idle
    );

    events
        .send(HostEvent::Highlight(HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            result: Err("failed".to_string()),
        }))
        .unwrap();
    assert_eq!(host.poll().len(), 1);
    assert_eq!(
        host.activity_at(Instant::now(), Some(Duration::from_secs(10))),
        PluginActivity::Error
    );

    host.unload();
    assert_eq!(
        host.activity_at(Instant::now(), Some(Duration::from_secs(10))),
        PluginActivity::Off
    );

    host.load();
    assert_eq!(
        host.activity_at(Instant::now() + Duration::from_secs(60), None),
        PluginActivity::Active
    );
}

#[test]
fn unload_then_load_resets_dedupe_so_next_snapshot_resends() {
    let (mut host, messages, _events) = PluginHost::for_tests();
    let snapshot = job(7, 2, 4);

    assert!(host.schedule(snapshot.clone()));
    assert_eq!(
        messages.try_recv().ok(),
        Some(WorkerMessage::Job(snapshot.clone()))
    );
    assert!(!host.schedule(snapshot.clone()));

    host.unload();
    assert!(!host.is_loaded());
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Unload));

    host.load();
    assert!(host.is_loaded());
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Load));
    assert!(
        messages.try_recv().is_err(),
        "a host without UI grants must not request an eager launch"
    );

    assert!(host.schedule(snapshot.clone()));
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Job(snapshot)));
}

#[test]
fn worker_unload_drops_jobs_until_load_reenables_them() {
    let (mut host, messages, _events) = PluginHost::for_tests();
    let snapshot = job(3, 0, 2);
    let mut worker_unloaded = false;

    host.unload();
    assert!(host.schedule(snapshot.clone()));
    assert_eq!(
        next_worker_action_for_tests(&messages, &mut worker_unloaded).unwrap(),
        (false, None),
        "an unloaded worker must not return a job to the launch/request path"
    );
    assert!(worker_unloaded);

    host.load();
    assert!(host.schedule(snapshot.clone()));
    assert_eq!(
        next_worker_action_for_tests(&messages, &mut worker_unloaded).unwrap(),
        (false, Some(snapshot))
    );
    assert!(!worker_unloaded);
}

#[test]
fn eager_host_load_requests_an_immediate_launch() {
    let (mut host, messages, _events) = PluginHost::for_tests_granted("menu-host", eager_grant());
    assert!(host.launches_eagerly());

    host.load();
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Load));
    assert_eq!(
        messages.try_recv().ok(),
        Some(WorkerMessage::Launch),
        "a host with UI grants must relaunch on load, not wait for an edit"
    );
}

#[test]
fn start_failed_event_reports_status_and_error_activity() {
    let mut app = AppState::new();
    let (host, _messages, events) = PluginHost::for_tests_granted("menu-host", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);

    events
        .send(HostEvent::StartFailed {
            error: "spawn failed".to_string(),
        })
        .unwrap();
    app.pump_plugins();

    assert_eq!(
        app.status_message,
        Some("Plugin menu-host failed: spawn failed".to_string())
    );
    let host = app.plugin_hosts.iter().next().unwrap();
    assert_eq!(
        host.activity_at(Instant::now(), None),
        PluginActivity::Error
    );
}

#[test]
fn from_entries_launches_an_eager_host_without_any_edit() {
    // A trusted log-filter entry holds `menu`/`window`, so construction alone
    // must attempt the launch; the nonexistent command turns that attempt
    // into an observable StartFailed event. No job or edit is ever issued.
    let entry = PluginEntry {
        id: "logf".to_string(),
        command: PathBuf::from("/nonexistent/dun-eager-launch-fixture"),
        trust: PluginTrust::UserTrustedExternal,
        roles: vec![PluginRole::LogFilter],
        timeout_ms: 500,
        max_frame_bytes: 64 * 1024,
    };
    let mut hosts = PluginHosts::from_entries(&[entry]);

    let deadline = Instant::now() + Duration::from_secs(5);
    let error = loop {
        let failure = hosts
            .iter_mut()
            .next()
            .unwrap()
            .poll()
            .into_iter()
            .find_map(|event| match event {
                HostEvent::StartFailed { error } => Some(error),
                _ => None,
            });
        if let Some(error) = failure {
            break error;
        }
        assert!(
            Instant::now() < deadline,
            "an eager host must surface its launch failure as StartFailed"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        error.contains("failed to launch plugin host"),
        "unexpected launch error: {error}"
    );
    assert_eq!(
        hosts
            .iter()
            .next()
            .unwrap()
            .activity_at(Instant::now(), None),
        PluginActivity::Error
    );
}

#[test]
fn plugin_command_reports_and_controls_a_single_host() {
    let mut app = AppState::new();
    let (host, messages, _events) = PluginHost::for_tests();
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);

    app.run_command_line("plugin");
    assert_eq!(
        app.status_message,
        Some("Plugin test-plugin is loaded".to_string())
    );

    app.run_command_line("plugin unload");
    assert_eq!(
        app.status_message,
        Some("Plugin test-plugin unloaded".to_string())
    );
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Unload));

    app.run_command_line("plugin");
    assert_eq!(
        app.status_message,
        Some("Plugin test-plugin is unloaded".to_string())
    );

    app.run_command_line("plugin load");
    assert_eq!(
        app.status_message,
        Some("Plugin test-plugin loaded (starts on the next edit)".to_string())
    );
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Load));

    app.run_command_line("plugin restart");
    assert_eq!(
        app.status_message,
        Some("Usage: plugin [load|unload] [plugin-id]".to_string())
    );
}

#[test]
fn plugin_command_addresses_hosts_by_id() {
    let mut app = AppState::new();
    let (alpha, alpha_messages, _alpha_events) =
        PluginHost::for_tests_granted("alpha", GrantedCapabilities::default());
    let (beta, beta_messages, _beta_events) =
        PluginHost::for_tests_granted("beta", GrantedCapabilities::default());
    app.plugin_hosts = PluginHosts::for_tests(vec![alpha, beta]);

    app.run_command_line("plugin");
    assert_eq!(
        app.status_message,
        Some("Plugin alpha is loaded; Plugin beta is loaded".to_string())
    );

    app.run_command_line("plugin unload beta");
    assert_eq!(app.status_message, Some("Plugin beta unloaded".to_string()));
    assert_eq!(beta_messages.try_recv().ok(), Some(WorkerMessage::Unload));
    assert!(
        alpha_messages.try_recv().is_err(),
        "addressing beta must not touch alpha"
    );

    app.run_command_line("plugin");
    assert_eq!(
        app.status_message,
        Some("Plugin alpha is loaded; Plugin beta is unloaded".to_string())
    );

    // With several hosts a bare load/unload is ambiguous.
    app.run_command_line("plugin load");
    assert_eq!(
        app.status_message,
        Some("Usage: plugin [load|unload] [plugin-id]".to_string())
    );

    app.run_command_line("plugin load nosuch");
    assert_eq!(
        app.status_message,
        Some("No plugin named nosuch".to_string())
    );

    app.run_command_line("plugin load beta");
    assert_eq!(
        app.status_message,
        Some("Plugin beta loaded (starts on the next edit)".to_string())
    );
    assert_eq!(beta_messages.try_recv().ok(), Some(WorkerMessage::Load));
}

#[test]
fn plugin_command_reports_eager_load_without_the_edit_hint() {
    let mut app = AppState::new();
    let (host, messages, _events) = PluginHost::for_tests_granted("menu-host", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);

    app.run_command_line("plugin load");
    assert_eq!(
        app.status_message,
        Some("Plugin menu-host loaded".to_string()),
        "an eager host launches now; '(starts on the next edit)' would be wrong"
    );
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Load));
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Launch));
}

#[test]
fn plugin_indicator_is_hidden_when_the_toggle_is_off() {
    let mut app = AppState::new();
    let (host, _messages, _events) = PluginHost::for_tests();
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);

    assert_eq!(app.plugin_indicator(), None);
}

#[test]
fn plugin_indicator_flags_an_idle_host_when_the_toggle_is_on() {
    let config = Config {
        plugin_status: dun_config::PluginStatusConfig {
            status_bar: true,
            idle_after_ms: 1_000,
        },
        ..Config::default()
    };
    let mut app = AppState::from_config(config);
    let (mut host, _messages, _events) = PluginHost::for_tests();
    host.set_last_activity_for_tests(Instant::now().checked_sub(Duration::from_secs(2)).unwrap());
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);

    let indicator = app.plugin_indicator().unwrap();
    assert!(indicator.alert);
    assert!(indicator.text.ends_with("idle]"));
}

#[test]
fn plugin_indicator_concatenates_every_host_in_config_order() {
    let config = Config {
        plugin_status: dun_config::PluginStatusConfig {
            status_bar: true,
            idle_after_ms: 1_000,
        },
        ..Config::default()
    };
    let mut app = AppState::from_config(config);
    let (alpha, _alpha_messages, _alpha_events) =
        PluginHost::for_tests_granted("alpha", GrantedCapabilities::default());
    let (mut beta, _beta_messages, _beta_events) =
        PluginHost::for_tests_granted("beta", GrantedCapabilities::default());
    beta.set_last_activity_for_tests(Instant::now().checked_sub(Duration::from_secs(2)).unwrap());
    app.plugin_hosts = PluginHosts::for_tests(vec![alpha, beta]);

    let indicator = app.plugin_indicator().unwrap();
    assert_eq!(indicator.text, "[alpha][beta idle]");
    assert!(indicator.alert, "any alerting host alerts the indicator");
}

#[test]
fn plugin_command_reports_when_no_host_is_configured() {
    let mut app = AppState::new();

    for command in ["plugin", "plugin load", "plugin unload"] {
        app.run_command_line(command);
        assert_eq!(
            app.status_message,
            Some("No plugin host configured".to_string())
        );
    }
}
