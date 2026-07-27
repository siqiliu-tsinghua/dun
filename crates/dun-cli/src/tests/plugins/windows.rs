//! Plugin-owned windows: surfaces, the scratch buffer, execute, streams.
//!
//! Split out of `tests/plugins.rs` when it passed the 45k test-file debt
//! threshold in docs/code-organization-guidelines.md. Tests moved verbatim;
//! shared helpers stay in the parent module.

use super::*;

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
