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

mod highlight;
mod keybinding;
mod lifecycle;
mod menu;
mod windows;

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

fn scratch_window_count(app: &AppState) -> usize {
    app.workspace
        .windows
        .iter()
        .filter(|window| window.kind == WindowKind::PluginScratch)
        .count()
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
