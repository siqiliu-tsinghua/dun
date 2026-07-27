use std::thread;
use std::time::{Duration, Instant};

use dun_config::{PluginEntry, PluginRole, PluginTrust};
use dun_plugin::json::{self, Json};
use dun_plugin::{
    Capability, GrantedCapabilities, PluginKeybinding, PluginMenu, Role, StyleId, StyleSpan,
    TrustClass,
};

use super::support::app_with_text;
use crate::plugins::{
    PluginActivity, PluginHost, PluginMenuRejection, PluginMenuRejectionReason, WorkerMessage,
    next_worker_action_for_tests,
};
use crate::*;

fn span(line: u32, start_col: u32, end_col: u32) -> StyleSpan {
    StyleSpan {
        line,
        start_col,
        end_col,
        style: StyleId::Keyword,
    }
}

fn job(revision: u64, first_line: usize, line_count: usize) -> HighlightJob {
    HighlightJob {
        buffer_id: BufferId(1),
        revision,
        language: "rust".to_string(),
        first_line,
        lines: vec!["fn main() {}".to_string(); line_count],
    }
}

/// The grant of a trusted log-filter host: holds `menu` and `window`, so the
/// host launches eagerly.
fn eager_grant() -> GrantedCapabilities {
    GrantedCapabilities::for_roles(&[Role::LogFilter], TrustClass::UserTrustedExternal)
}

fn sample_menu() -> PluginMenu {
    let payload = json::obj([
        ("top_label", json::obj([("en_US", json::str("Tools"))])),
        (
            "items",
            Json::Arr(vec![json::obj([
                ("label", json::obj([("en_US", json::str("Run"))])),
                ("action_id", json::str("run")),
            ])]),
        ),
    ]);
    PluginMenu::from_payload(&payload).expect("valid menu payload")
}

fn menu_with_top_labels(english: &str, translations: &[(&str, &str)]) -> PluginMenu {
    let mut top_labels = vec![("en_US".to_string(), json::str(english))];
    top_labels.extend(
        translations
            .iter()
            .map(|(tag, label)| ((*tag).to_string(), json::str(label))),
    );
    let payload = json::obj([
        ("top_label", Json::Obj(top_labels)),
        (
            "items",
            Json::Arr(vec![json::obj([
                ("label", json::obj([("en_US", json::str("Run"))])),
                ("action_id", json::str("run")),
            ])]),
        ),
    ]);
    PluginMenu::from_payload(&payload).expect("valid menu payload")
}

/// A menu whose host declares its own mnemonics: `top_mnemonic` plus one per
/// item. `items` is `(en_US label, action_id, mnemonic)`.
fn menu_with_declared_mnemonics(
    english: &str,
    top_mnemonic: Option<&str>,
    items: &[(&str, &str, Option<&str>)],
) -> PluginMenu {
    let mut fields = vec![
        (
            "top_label".to_string(),
            json::obj([("en_US", json::str(english))]),
        ),
        (
            "items".to_string(),
            Json::Arr(
                items
                    .iter()
                    .map(|(label, action_id, mnemonic)| {
                        let mut item = vec![
                            (
                                "label".to_string(),
                                json::obj([("en_US", json::str(label))]),
                            ),
                            ("action_id".to_string(), json::str(action_id)),
                        ];
                        if let Some(mnemonic) = mnemonic {
                            item.push(("mnemonic".to_string(), json::str(mnemonic)));
                        }
                        Json::Obj(item)
                    })
                    .collect(),
            ),
        ),
    ];
    if let Some(top_mnemonic) = top_mnemonic {
        fields.push(("top_mnemonic".to_string(), json::str(top_mnemonic)));
    }
    PluginMenu::from_payload(&Json::Obj(fields)).expect("valid menu payload")
}

fn rendered_plugin_entry_labels(app: &AppState) -> Vec<String> {
    let buffer_views = app.buffer_views();
    app.shell
        .frame_for_workspace(&app.workspace, Rect::new(0, 0, 120, 20), &buffer_views)
        .menu
        .items
        .last()
        .expect("a menu bar item")
        .entries
        .iter()
        .map(|entry| entry.label.clone().into_owned())
        .collect()
}

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

fn started_menu_host(plugin_id: &str, menu: PluginMenu) -> PluginHost {
    let (mut host, _messages, events) = PluginHost::for_tests_granted(plugin_id, eager_grant());
    events
        .send(HostEvent::Started {
            menu: Some(menu),
            keybinding: None,
        })
        .unwrap();
    assert!(host.poll().is_empty(), "handshake events are absorbed");
    host
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

fn app_with_menu_contributions(contributions: Vec<(&str, PluginMenu)>, tags: &[&str]) -> AppState {
    let mut app = AppState::new();
    app.plugin_menu_tags = tags.iter().map(|tag| (*tag).to_string()).collect();
    app.plugin_hosts = PluginHosts::for_tests(
        contributions
            .into_iter()
            .map(|(plugin_id, menu)| started_menu_host(plugin_id, menu))
            .collect(),
    );
    app.refresh_plugin_contributions();
    app
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
fn language_hint_uses_lowercased_extension() {
    assert_eq!(language_hint(Some(&PathBuf::from("/tmp/Main.RS"))), "rs");
    assert_eq!(language_hint(Some(&PathBuf::from("/tmp/README"))), "");
    assert_eq!(language_hint(None), "");
}

#[test]
fn highlight_outcome_applies_only_for_the_current_revision() {
    let mut app = app_with_text("fn main");

    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            result: Ok(vec![span(0, 0, 2)]),
        },
    );
    let stored = app.buffer_state(BufferId(1)).unwrap().highlight.clone();
    assert_eq!(
        stored,
        Some(BufferHighlight {
            revision: 0,
            spans: vec![BufferHighlightSpan {
                line: 0,
                start_column: 0,
                end_column: 2,
                class: HighlightClass::Keyword,
            }],
        })
    );

    app.buffer_state_mut(BufferId(1))
        .unwrap()
        .buffer
        .insert_char('x')
        .unwrap();
    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            result: Ok(vec![span(0, 0, 1)]),
        },
    );
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().highlight,
        stored,
        "stale outcome must not overwrite the cache"
    );
}

#[test]
fn highlight_conversion_maps_char_columns_to_byte_columns() {
    // "a中b": char columns 1..2 cover 中, which is bytes 1..4.
    let mut app = app_with_text("a\u{4e2d}b");

    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            result: Ok(vec![span(0, 1, 2), span(0, 9, 10)]),
        },
    );

    let highlight = app
        .buffer_state(BufferId(1))
        .unwrap()
        .highlight
        .clone()
        .unwrap();
    assert_eq!(
        highlight.spans,
        vec![BufferHighlightSpan {
            line: 0,
            start_column: 1,
            end_column: 4,
            class: HighlightClass::Keyword,
        }],
        "wide char widens to its byte range; the out-of-range span is dropped"
    );
}

#[test]
fn highlight_error_outcome_reports_plugin_status() {
    let mut app = AppState::new();

    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            result: Err("plugin host timed out".to_string()),
        },
    );

    assert_eq!(
        app.status_message,
        Some("Plugin demo failed: plugin host timed out".to_string())
    );
}

#[test]
fn highlight_failure_leaves_buffer_and_prior_highlight_untouched() {
    let mut app = app_with_text("fn main");
    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            result: Ok(vec![span(0, 0, 2)]),
        },
    );
    let highlight_before = app.buffer_state(BufferId(1)).unwrap().highlight.clone();
    let revision_before = app.buffer_state(BufferId(1)).unwrap().buffer.revision();
    assert!(highlight_before.is_some());

    // A later failure for the same buffer must be inert beyond the status
    // line: it never touches buffer text, revision, or the valid highlight.
    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            result: Err("plugin host crashed".to_string()),
        },
    );

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().highlight,
        highlight_before,
        "a plugin failure must not clear the existing highlight"
    );
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.revision(),
        revision_before,
        "a plugin failure must not mutate the buffer"
    );
    assert_eq!(
        app.status_message,
        Some("Plugin demo failed: plugin host crashed".to_string())
    );
}

#[test]
fn schedule_dedupes_identical_snapshots_and_sends_changed_ones() {
    let (mut host, jobs, _events) = PluginHost::for_tests();

    assert!(host.schedule(job(0, 0, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(WorkerMessage::Job(job(0, 0, 3))));

    assert!(!host.schedule(job(0, 0, 3)));
    assert!(jobs.try_recv().is_err());

    assert!(host.schedule(job(1, 0, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(WorkerMessage::Job(job(1, 0, 3))));

    assert!(host.schedule(job(1, 5, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(WorkerMessage::Job(job(1, 5, 3))));
}

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

fn plugin_action(plugin_id: &str, action_id: &str) -> EditorCommand {
    plugin_action_kind(plugin_id, action_id, PluginActionKind::Surface)
}

fn plugin_action_kind(plugin_id: &str, action_id: &str, kind: PluginActionKind) -> EditorCommand {
    EditorCommand::PluginAction {
        plugin_id: plugin_id.into(),
        action_id: action_id.into(),
        kind,
    }
}

fn surface_window_count(app: &AppState) -> usize {
    app.workspace
        .windows
        .iter()
        .filter(|window| window.kind == WindowKind::PluginSurface)
        .count()
}

#[test]
fn plugin_action_opens_a_surface_window_only_when_window_is_granted() {
    // A host without the `window` capability opens nothing when its action is
    // invoked; the grant is the gate.
    let mut app = AppState::new();
    let (ungranted, _m, _e) =
        PluginHost::for_tests_granted("no-window", GrantedCapabilities::default());
    app.plugin_hosts = PluginHosts::for_tests(vec![ungranted]);
    app.handle_command(&plugin_action("no-window", "open"));
    assert_eq!(surface_window_count(&app), 0);
    assert_eq!(app.plugin_windows.count("no-window"), 0);

    // A window-granted host opens a read-only PluginSurface it owns.
    let mut app = AppState::new();
    let (granted, _m, _e) = PluginHost::for_tests_granted("winhost", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![granted]);
    app.handle_command(&plugin_action("winhost", "open"));
    assert_eq!(surface_window_count(&app), 1);
    assert_eq!(app.plugin_windows.count("winhost"), 1);
    let surface = app
        .workspace
        .windows
        .iter()
        .find(|window| window.kind == WindowKind::PluginSurface)
        .unwrap();
    assert_eq!(surface.title, "winhost: open");
    assert!(
        app.buffer_state(surface.buffer_id)
            .unwrap()
            .buffer
            .is_read_only(),
        "a surface is read-only to the user until surface-write lands"
    );
}

#[test]
fn invoking_an_action_reuses_the_plugins_surface_window() {
    let mut app = AppState::new();
    let (granted, _m, _e) = PluginHost::for_tests_granted("winhost", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![granted]);

    // Repeated invokes reuse the one surface rather than spawning a window each
    // time; the per-plugin cap itself is unit-tested on the registry.
    app.handle_command(&plugin_action("winhost", "open"));
    app.handle_command(&plugin_action("winhost", "filter"));
    app.handle_command(&plugin_action("winhost", "open"));
    assert_eq!(surface_window_count(&app), 1);
    assert_eq!(app.plugin_windows.count("winhost"), 1);
    // The reused window's title tracks the latest action.
    let surface = app
        .workspace
        .windows
        .iter()
        .find(|window| window.kind == WindowKind::PluginSurface)
        .unwrap();
    assert_eq!(surface.title, "winhost: open");
}

#[test]
fn surface_write_response_fills_the_plugins_window() {
    let mut app = AppState::new();
    let (host, messages, events) = PluginHost::for_tests_granted("winhost", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);

    // Invoking sends a surface request to the worker and opens the surface.
    app.handle_command(&plugin_action("winhost", "show"));
    assert_eq!(surface_window_count(&app), 1);
    assert_eq!(
        messages.try_recv().ok(),
        Some(WorkerMessage::Surface("show".to_string()))
    );

    // The host's response fills the surface on the next pump.
    events
        .send(HostEvent::Surface {
            action_id: "show".to_string(),
            result: Ok(vec!["alpha".to_string(), "beta".to_string()]),
        })
        .unwrap();
    app.pump_plugins();

    let surface = app
        .workspace
        .windows
        .iter()
        .find(|window| window.kind == WindowKind::PluginSurface)
        .unwrap();
    let buffer = &app.buffer_state(surface.buffer_id).unwrap().buffer;
    assert_eq!(buffer.line(0), Some("alpha"));
    assert_eq!(buffer.line(1), Some("beta"));
    assert!(buffer.is_read_only());
}

#[test]
fn window_only_host_gets_an_empty_surface_and_sends_no_request() {
    let mut app = AppState::new();
    // Granted `window` but not `surface-write`: the surface opens but stays empty
    // and no surface request is issued.
    let grant = GrantedCapabilities::grant([Capability::Window], TrustClass::UserTrustedExternal);
    let (host, messages, _events) = PluginHost::for_tests_granted("winhost", grant);
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);

    app.handle_command(&plugin_action("winhost", "open"));
    assert_eq!(surface_window_count(&app), 1);
    assert!(
        messages.try_recv().is_err(),
        "a host without surface-write must not be asked for content"
    );
}

#[test]
fn unloading_a_host_reaps_its_surface_windows() {
    let mut app = AppState::new();
    let (granted, _m, _e) = PluginHost::for_tests_granted("winhost", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![granted]);
    app.handle_command(&plugin_action("winhost", "open"));
    assert_eq!(surface_window_count(&app), 1);

    app.run_command_line("plugin unload");

    assert_eq!(surface_window_count(&app), 0, "unload reaps the surface");
    assert_eq!(app.plugin_windows.count("winhost"), 0);
}

#[test]
fn closing_a_plugin_surface_frees_the_slot() {
    let mut app = AppState::new();
    let (granted, _m, _e) = PluginHost::for_tests_granted("winhost", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![granted]);
    app.handle_command(&plugin_action("winhost", "open"));
    assert_eq!(app.plugin_windows.count("winhost"), 1);

    // Closing the surface releases the slot from the registry.
    app.handle_command(&EditorCommand::Window(WindowCommand::Close));
    assert_eq!(app.plugin_windows.count("winhost"), 0);
    assert_eq!(surface_window_count(&app), 0);

    // The plugin can open a fresh surface again afterward.
    app.handle_command(&plugin_action("winhost", "open"));
    assert_eq!(app.plugin_windows.count("winhost"), 1);
    assert_eq!(surface_window_count(&app), 1);
}

fn surface_buffer_text(app: &AppState, plugin_id: &str) -> String {
    let window = app
        .workspace
        .windows
        .iter()
        .find(|window| {
            window.kind == WindowKind::PluginSurface
                && app.plugin_windows.owns(plugin_id, window.id)
        })
        .expect("plugin surface window");
    let buffer = &app.buffer_state(window.buffer_id).unwrap().buffer;
    (0..buffer.line_count())
        .map(|i| buffer.line(i).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn stream_read_feed_sends_a_chunk_only_to_stream_read_hosts() {
    let mut app = AppState::new();
    let (filter, filter_msgs, _fe) = PluginHost::for_tests_granted("logf", eager_grant());
    let (plain, plain_msgs, _pe) =
        PluginHost::for_tests_granted("plain", GrantedCapabilities::default());
    app.plugin_hosts = PluginHosts::for_tests(vec![filter, plain]);

    app.feed_stream_to_filters("command-output", &["a".into(), "b".into()]);

    // The stream-read host gets a chunk; the ungranted host gets nothing.
    match filter_msgs.try_recv() {
        Ok(WorkerMessage::Stream(chunk)) => {
            assert_eq!(chunk.stream_id, "command-output");
            assert_eq!(chunk.lines, vec!["a".to_string(), "b".to_string()]);
            assert!(chunk.final_chunk);
        }
        other => panic!("expected a stream chunk, got {other:?}"),
    }
    assert!(
        plain_msgs.try_recv().is_err(),
        "a host without stream-read must not be fed"
    );
}

#[test]
fn stream_verdict_shows_kept_lines_in_the_hosts_surface() {
    let mut app = AppState::new();
    let (filter, _m, events) = PluginHost::for_tests_granted("logf", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![filter]);

    // Feed three lines, then the host keeps lines 0 and 2.
    app.feed_stream_to_filters(
        "command-output",
        &["alpha".into(), "beta".into(), "gamma".into()],
    );
    events
        .send(HostEvent::StreamVerdict {
            result: Ok(vec![true, false, true]),
        })
        .unwrap();
    app.pump_plugins();

    assert_eq!(surface_window_count(&app), 1);
    assert_eq!(surface_buffer_text(&app, "logf"), "alpha\ngamma");
}

#[test]
fn stream_verdict_with_mismatched_length_is_dropped() {
    let mut app = AppState::new();
    let (filter, _m, events) = PluginHost::for_tests_granted("logf", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![filter]);

    app.feed_stream_to_filters("command-output", &["a".into(), "b".into()]);
    // A verdict whose length no longer matches the fed lines is ignored: no
    // window is opened.
    events
        .send(HostEvent::StreamVerdict {
            result: Ok(vec![true]),
        })
        .unwrap();
    app.pump_plugins();

    assert_eq!(surface_window_count(&app), 0);
}

#[test]
fn a_large_stream_is_split_into_bounded_chunks() {
    // Command output larger than the 512-line budget must be fed as several
    // chunks (index rising, final on the last), not one oversized chunk the
    // client would reject.
    let mut app = AppState::new();
    let (filter, msgs, _e) = PluginHost::for_tests_granted("logf", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![filter]);

    let lines: Vec<String> = (0..1200).map(|i| format!("line {i}")).collect();
    app.feed_stream_to_filters("command-output", &lines);

    let chunks: Vec<_> = std::iter::from_fn(|| match msgs.try_recv() {
        Ok(WorkerMessage::Stream(chunk)) => Some(chunk),
        _ => None,
    })
    .collect();
    // 1200 lines / 512 = three chunks: 512, 512, 176.
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].lines.len(), 512);
    assert_eq!(chunks[1].lines.len(), 512);
    assert_eq!(chunks[2].lines.len(), 176);
    assert_eq!(
        chunks.iter().map(|c| c.chunk_index).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        chunks.iter().map(|c| c.final_chunk).collect::<Vec<_>>(),
        vec![false, false, true]
    );
    // No line is lost or duplicated across the chunks.
    let flat: Vec<&String> = chunks.iter().flat_map(|c| c.lines.iter()).collect();
    assert_eq!(flat.len(), 1200);
    assert_eq!(flat[0], "line 0");
    assert_eq!(flat[1199], "line 1199");
}

#[test]
fn stream_verdicts_accumulate_across_chunks_in_the_surface() {
    // Each chunk's kept lines append to the surface; the window shows the whole
    // filtered stream, not just the last chunk.
    let mut app = AppState::new();
    let (filter, _m, events) = PluginHost::for_tests_granted("logf", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![filter]);

    // Two chunks worth of lines (600 > 512).
    let lines: Vec<String> = (0..600).map(|i| format!("line {i}")).collect();
    app.feed_stream_to_filters("command-output", &lines);

    // Chunk 0 (512 lines): keep only the first. Chunk 1 (88 lines): keep only
    // the first. The surface should end up with one line from each.
    let mut keep0 = vec![false; 512];
    keep0[0] = true;
    let mut keep1 = vec![false; 88];
    keep1[0] = true;
    events
        .send(HostEvent::StreamVerdict { result: Ok(keep0) })
        .unwrap();
    events
        .send(HostEvent::StreamVerdict { result: Ok(keep1) })
        .unwrap();
    app.pump_plugins();

    assert_eq!(surface_window_count(&app), 1);
    // "line 0" from chunk 0 and "line 512" from chunk 1, accumulated.
    assert_eq!(surface_buffer_text(&app, "logf"), "line 0\nline 512");
}

#[test]
fn a_new_stream_resets_the_accumulated_surface() {
    // The first chunk of a fresh stream (chunk_index 0) resets the accumulator,
    // so a second command's filtered output replaces the first, not appends.
    let mut app = AppState::new();
    let (filter, _m, events) = PluginHost::for_tests_granted("logf", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![filter]);

    app.feed_stream_to_filters("command-output", &["a".into(), "b".into(), "c".into()]);
    events
        .send(HostEvent::StreamVerdict {
            result: Ok(vec![true, true, true]),
        })
        .unwrap();
    app.pump_plugins();
    assert_eq!(surface_buffer_text(&app, "logf"), "a\nb\nc");

    app.feed_stream_to_filters("command-output", &["x".into(), "y".into()]);
    events
        .send(HostEvent::StreamVerdict {
            result: Ok(vec![true, true]),
        })
        .unwrap();
    app.pump_plugins();
    assert_eq!(surface_buffer_text(&app, "logf"), "x\ny");
}

fn scratch_window_count(app: &AppState) -> usize {
    app.workspace
        .windows
        .iter()
        .filter(|window| window.kind == WindowKind::PluginScratch)
        .count()
}

/// A plugin may own two windows. The second must stack under the first, not
/// split whatever is focused: the latter produced three side-by-side columns
/// and at 80 columns left every pane too narrow to read.
#[test]
fn a_plugins_second_window_stacks_under_its_first_instead_of_taking_a_third_column() {
    use dun_core::{Axis, LayoutNode};

    let mut app = AppState::new();
    let (host, _m, _e) = PluginHost::for_tests_granted("logf", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);
    let main = app.workspace.focused;

    app.handle_command(&plugin_action_kind(
        "logf",
        "input",
        PluginActionKind::Scratch,
    ));
    let scratch = app.workspace.focused;
    assert_ne!(scratch, main);

    app.handle_command(&plugin_action("logf", "open"));
    let surface = app.workspace.focused;
    assert_eq!(app.plugin_windows.count("logf"), 2);

    // main | scratch
    //        surface
    let LayoutNode::Split {
        axis: Axis::Horizontal,
        first,
        second,
        ..
    } = &app.workspace.root
    else {
        panic!(
            "editor should keep its own column: {:?}",
            app.workspace.root
        );
    };
    assert_eq!(
        **first,
        LayoutNode::Leaf(main),
        "the editor must stay on the left, not be pushed into the plugin column"
    );
    let LayoutNode::Split {
        axis: Axis::Vertical,
        first: upper,
        second: lower,
        ..
    } = &**second
    else {
        panic!("the plugin column should be stacked, got {second:?}");
    };
    assert_eq!(**upper, LayoutNode::Leaf(scratch));
    assert_eq!(**lower, LayoutNode::Leaf(surface));
}

#[test]
fn scratch_action_opens_an_editable_window_only_with_the_grant() {
    // A window-only host (no scratch-input): a Scratch action opens nothing.
    let mut app = AppState::new();
    let grant = GrantedCapabilities::grant([Capability::Window], TrustClass::UserTrustedExternal);
    let (host, _m, _e) = PluginHost::for_tests_granted("winhost", grant);
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);
    app.handle_command(&plugin_action_kind(
        "winhost",
        "input",
        PluginActionKind::Scratch,
    ));
    assert_eq!(scratch_window_count(&app), 0);

    // A scratch-input host (log-filter grant): the editable scratch window opens.
    let mut app = AppState::new();
    let (host, _m, _e) = PluginHost::for_tests_granted("logf", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);
    app.handle_command(&plugin_action_kind(
        "logf",
        "input",
        PluginActionKind::Scratch,
    ));
    assert_eq!(scratch_window_count(&app), 1);
    assert_eq!(app.plugin_windows.count("logf"), 1);
    let scratch = app
        .workspace
        .windows
        .iter()
        .find(|window| window.kind == WindowKind::PluginScratch)
        .unwrap();
    assert_eq!(scratch.title, "logf: input");
    assert!(
        !app.buffer_state(scratch.buffer_id)
            .unwrap()
            .buffer
            .is_read_only(),
        "a scratch window is editable"
    );
}

#[test]
fn execute_action_submits_scratch_text_and_shows_the_result() {
    let mut app = AppState::new();
    let (host, messages, events) = PluginHost::for_tests_granted("logf", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);

    // Open the scratch window and type into its buffer.
    app.handle_command(&plugin_action_kind(
        "logf",
        "input",
        PluginActionKind::Scratch,
    ));
    let scratch_buffer = app
        .workspace
        .windows
        .iter()
        .find(|window| window.kind == WindowKind::PluginScratch)
        .unwrap()
        .buffer_id;
    app.buffer_state_mut(scratch_buffer)
        .unwrap()
        .buffer
        .insert_str("level:error")
        .unwrap();

    // Execute submits the whole scratch text as one blob to the worker.
    app.handle_command(&plugin_action_kind(
        "logf",
        "run",
        PluginActionKind::Execute,
    ));
    assert_eq!(
        messages.try_recv().ok(),
        Some(WorkerMessage::Execute("level:error".to_string()))
    );

    // The host's result fills the surface window on the next pump.
    events
        .send(HostEvent::Surface {
            action_id: "execute".to_string(),
            result: Ok(vec!["ran ok".to_string()]),
        })
        .unwrap();
    app.pump_plugins();
    assert_eq!(surface_buffer_text(&app, "logf"), "ran ok");
}

#[test]
fn execute_without_a_scratch_window_sends_nothing() {
    let mut app = AppState::new();
    let (host, messages, _e) = PluginHost::for_tests_granted("logf", eager_grant());
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);

    // No scratch window open yet: execute submits nothing.
    app.handle_command(&plugin_action_kind(
        "logf",
        "run",
        PluginActionKind::Execute,
    ));
    assert!(
        messages.try_recv().is_err(),
        "execute with no scratch window must not submit"
    );
}

fn keybinding(leader: &str, key: &str, action_id: &str) -> PluginKeybinding {
    let payload = json::obj([
        ("leader", json::str(leader)),
        (
            "chords",
            Json::Arr(vec![json::obj([
                ("key", json::str(key)),
                ("action_id", json::str(action_id)),
            ])]),
        ),
    ]);
    PluginKeybinding::from_payload(&payload).expect("valid keybinding")
}

/// A window+keybinding-granted host with its leader contribution installed on
/// the shell (as a launch handshake would deliver it).
fn app_with_keybinding_host(plugin_id: &str, keybinding: PluginKeybinding) -> AppState {
    let mut app = AppState::new();
    let (mut host, _messages, events) = PluginHost::for_tests_granted(plugin_id, eager_grant());
    events
        .send(HostEvent::Started {
            menu: None,
            keybinding: Some(keybinding),
        })
        .unwrap();
    assert!(host.poll().is_empty(), "handshake events are absorbed");
    app.plugin_hosts = PluginHosts::for_tests(vec![host]);
    app.pump_plugins();
    app
}

fn stroke(spec: &str) -> KeyStroke {
    spec.parse().expect("valid keystroke spec")
}

#[test]
fn keybinding_leader_chord_dispatches_a_plugin_action() {
    let mut app = app_with_keybinding_host("logf", keybinding("Ctrl+J", "o", "open"));
    assert_eq!(app.shell.plugin_keymap.bindings.len(), 1);

    // The leader alone is a pending prefix: consumed, nothing dispatched.
    assert!(app.handle_key_stroke(stroke("Ctrl+J")));
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

    assert!(app.handle_key_stroke(stroke("Ctrl+J")));
    // A key that is not a chord under the leader cancels the pending prefix.
    app.handle_key_stroke(stroke("z"));
    assert_eq!(surface_window_count(&app), 0);
    assert_eq!(app.plugin_windows.count("logf"), 0);
}

#[test]
fn keybinding_leader_colliding_with_a_built_in_prefix_is_rejected() {
    // `Ctrl+X` is the built-in leader for many bindings, so a plugin claiming it
    // would shadow them; its whole contribution is dropped.
    let app = app_with_keybinding_host("logf", keybinding("Ctrl+X", "o", "open"));
    assert!(
        app.shell.plugin_keymap.bindings.is_empty(),
        "a leader that collides with a built-in prefix must not install"
    );
}

#[test]
fn a_rejected_keybinding_reports_a_status_message() {
    // A colliding leader is dropped from the keymap — a silent no-op before —
    // so the user now gets a status message naming the plugin instead.
    let app = app_with_keybinding_host("logf", keybinding("Ctrl+X", "o", "open"));
    assert!(app.shell.plugin_keymap.bindings.is_empty());
    assert_eq!(
        app.status_message,
        Some("Plugin logf keybinding ignored: leader conflicts".to_string()),
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
fn two_plugins_cannot_claim_the_same_leader() {
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
            keybinding: Some(keybinding("Ctrl+J", "b", "beta-open")),
        })
        .unwrap();
    assert!(alpha.poll().is_empty());
    assert!(beta.poll().is_empty());
    app.plugin_hosts = PluginHosts::for_tests(vec![alpha, beta]);
    app.pump_plugins();

    // Only the first claimant (config order) keeps the leader.
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
