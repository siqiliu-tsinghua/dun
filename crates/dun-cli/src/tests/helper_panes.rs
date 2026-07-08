#![allow(unused_imports)]

use super::support::*;

#[test]
fn help_command_opens_read_only_help_window_once() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::Help));

    let help_window = app.workspace.focused_window().unwrap();
    let help_window_id = help_window.id;
    let help_buffer_id = help_window.buffer_id;
    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(help_window.title, "Help");
    assert_eq!(help_window.kind, WindowKind::Help);
    assert_eq!(help_window.buffer_kind, BufferKind::ReadOnly);

    let help_buffer = app.buffer_state(help_buffer_id).unwrap();
    assert!(help_buffer.buffer.is_read_only());
    assert!(help_buffer.buffer.to_text().contains("Ctrl+G"));
    assert_eq!(app.status_message, Some("Help".to_string()));

    app.handle_command(&EditorCommand::App(AppCommand::Help));

    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(app.workspace.focused, help_window_id);

    app.handle_command(&EditorCommand::Window(WindowCommand::Close));
    assert_eq!(app.workspace.window_count(), 1);
    assert!(app.buffer_state(help_buffer_id).is_none());
}

#[test]
fn f1_key_opens_help_screen() {
    let mut app = AppState::new();

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(1), CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::Help
    );
}

#[test]
fn outline_window_lists_and_jumps_read_only_sections() {
    let mut app = AppState::new();
    app.handle_command(&EditorCommand::App(AppCommand::Help));
    let help_buffer_id = app.workspace.focused_window().unwrap().buffer_id;

    submit_command_line(&mut app, "outline");

    let outline_window = app.workspace.focused_window().unwrap();
    assert_eq!(outline_window.kind, WindowKind::Outline);
    assert_eq!(outline_window.buffer_kind, BufferKind::ReadOnly);
    let text = app
        .buffer_state(outline_window.buffer_id)
        .unwrap()
        .buffer
        .to_text();
    assert!(text.contains("Dun Outline"));
    assert!(text.contains("App"));
    assert!(text.contains("Navigation"));

    submit_command_line(&mut app, "outline Navigation");

    let window = app.workspace.focused_window().unwrap();
    assert_eq!(window.buffer_id, help_buffer_id);
    let buffer = app.buffer_state(help_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("Navigation")
    );
}

#[test]
fn outline_recognizes_common_text_config_and_source_sections() {
    let buffer = TextBuffer::from_text_with_kind(
        BufferKind::Untitled,
        "\
# Markdown Title
body
## Nested Heading
[service]
[[servers]]
pub struct Worker {
impl Worker {
pub async fn run_task() {
function deploy {
cleanup() {
",
    );

    let labels = outline_entries_for_buffer(&buffer)
        .into_iter()
        .map(|entry| entry.label)
        .collect::<Vec<_>>();

    assert_eq!(
        labels,
        vec![
            "# Markdown Title",
            "## Nested Heading",
            "[service]",
            "[[servers]]",
            "struct Worker",
            "impl Worker",
            "fn run_task",
            "function deploy",
            "cleanup()",
        ]
    );
}

#[test]
fn outline_window_keyboard_selection_enters_section_and_close_returns_source() {
    let mut app = app_with_text("# First\nbody\n# Second\n");

    submit_command_line(&mut app, "outline");
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::Outline
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('n'), CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('n'), CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let source = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(
        app.workspace.focused_window().unwrap().buffer_id,
        BufferId(1)
    );
    assert_eq!(source.buffer.cursor_position(), Position::new(2, 0));

    submit_command_line(&mut app, "outline");
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::Outline
    );
    app.handle_command(&EditorCommand::Window(WindowCommand::Close));

    assert_eq!(
        app.workspace.focused_window().unwrap().buffer_id,
        BufferId(1)
    );
}

#[test]
fn document_edge_commands_work_in_read_only_windows() {
    let mut app = AppState::new();
    app.sync_view_for_area(Rect::new(0, 0, 80, 6));
    app.handle_command(&EditorCommand::App(AppCommand::Help));
    let help_buffer_id = app.workspace.focused_window().unwrap().buffer_id;

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveDocumentEnd));

    let buffer = app.buffer_state(help_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.cursor_position(),
        buffer_end_position(&buffer.buffer)
    );
    assert!(buffer.first_line > 0);

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveDocumentStart));

    let buffer = app.buffer_state(help_buffer_id).unwrap();
    assert_eq!(buffer.buffer.cursor_position(), Position::zero());
    assert_eq!(buffer.first_line, 0);
}

#[test]
fn help_screen_lists_configured_keybindings() {
    let config = parse_config(
        "\
key.app.help = F10
key.edit.go_to_line = F9
key.window.close = none
",
    )
    .unwrap();
    let mut app = AppState::from_config(config);

    app.handle_command(&EditorCommand::App(AppCommand::Help));

    let help_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    let text = app.buffer_state(help_buffer_id).unwrap().buffer.to_text();
    assert!(text.contains("F10"));
    assert!(text.contains("Help [app.help]"));
    assert!(text.contains("F9"));
    assert!(text.contains("Go to line [edit.go_to_line]"));
    assert!(text.contains("Jump Command Output to status [app.command_output_status]"));
    assert!(text.contains("Jump Command Output to truncation flag [app.command_output_truncated]"));
    assert!(text.contains("(unbound)"));
    assert!(text.contains("Close focused window [window.close]"));
    assert!(text.contains("Toggle hidden files [file_dialog.toggle_hidden]"));
    assert!(!text.contains("Ctrl+G"));
}

#[test]
fn config_diagnostics_command_opens_read_only_window_once() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::ConfigDiagnostics));

    let config_window = app.workspace.focused_window().unwrap();
    let config_window_id = config_window.id;
    let config_buffer_id = config_window.buffer_id;
    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(config_window.title, "Config Diagnostics");
    assert_eq!(config_window.kind, WindowKind::ConfigDiagnostics);
    assert_eq!(config_window.buffer_kind, BufferKind::ReadOnly);

    let config_buffer = app.buffer_state(config_buffer_id).unwrap();
    let text = config_buffer.buffer.to_text();
    assert!(config_buffer.buffer.is_read_only());
    assert!(text.contains("Dun Config Diagnostics"));
    assert!(text.contains("Summary\n"));
    assert!(text.contains("Paths\n"));
    assert!(text.contains("keymap:"));
    assert!(text.contains("active: disabled (--no-config)"));
    assert!(text.contains("theme:"));
    assert!(text.contains("mouse: disabled"));
    assert!(text.contains("defaults: dun --dump-config"));
    assert!(text.contains("osc52_max_bytes: 16384"));
    assert!(text.contains("bindings:"));
    assert!(text.contains("important_unbound: none"));
    assert!(text.contains("app.config_diagnostics"));
    assert!(text.contains("F6"));
    assert!(text.contains("File Dialog Keymap"));
    assert!(text.contains("file_dialog.toggle_hidden"));
    assert!(text.contains("Ctrl+H"));
    assert_eq!(app.status_message, Some("Config diagnostics".to_string()));

    app.handle_command(&EditorCommand::App(AppCommand::ConfigDiagnostics));

    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(app.workspace.focused, config_window_id);

    app.handle_command(&EditorCommand::Window(WindowCommand::Close));
    assert_eq!(app.workspace.window_count(), 1);
    assert!(app.buffer_state(config_buffer_id).is_none());
}

#[test]
fn config_diagnostics_command_jumps_to_named_sections() {
    let mut app = AppState::new();

    submit_command_line(&mut app, "config keymap");

    let window = app.workspace.focused_window().unwrap();
    assert_eq!(window.kind, WindowKind::ConfigDiagnostics);
    let buffer = app.buffer_state(window.buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("Keymap")
    );
    assert_eq!(
        app.status_message,
        Some("Config diagnostics: keymap".to_string())
    );

    submit_command_line(&mut app, "diagnostics file-dialog-keymap");

    let window = app.workspace.focused_window().unwrap();
    let buffer = app.buffer_state(window.buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("File Dialog Keymap")
    );
}

#[test]
fn f6_key_opens_config_diagnostics_screen() {
    let mut app = AppState::new();

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(6), CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::ConfigDiagnostics
    );
}

#[test]
fn search_results_window_lists_and_jumps_matches() {
    let mut app = app_with_text("alpha\nbeta alpha\ngamma\n");

    submit_command_line(&mut app, "find alpha");
    submit_command_line(&mut app, "results");

    let results_window = app.workspace.focused_window().unwrap();
    assert_eq!(results_window.kind, WindowKind::SearchResults);
    assert_eq!(results_window.buffer_kind, BufferKind::ReadOnly);
    let text = app
        .buffer_state(results_window.buffer_id)
        .unwrap()
        .buffer
        .to_text();
    assert!(text.contains("Dun Search Results"));
    assert!(text.contains("Matches: 2"));
    assert!(text.contains("  2. L2:C6 beta alpha"));

    submit_command_line(&mut app, "results 2");

    let source = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(source.buffer.cursor_position(), Position::new(1, 10));
    assert_eq!(
        source.buffer.selection_range(),
        Some(TextRange::new(Position::new(1, 5), Position::new(1, 10)))
    );
    assert!(
        source
            .search_status()
            .is_some_and(|status| status == "Find 2/2")
    );
}

#[test]
fn search_results_window_keyboard_selection_enters_match() {
    let mut app = app_with_text("alpha\nbeta alpha\ngamma\n");

    submit_command_line(&mut app, "find alpha");
    submit_command_line(&mut app, "results");

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('n'), CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.status_message,
        Some("Search Results: selected 1/2".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('n'), CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.status_message,
        Some("Search Results: selected 2/2".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let window = app.workspace.focused_window().unwrap();
    assert_eq!(window.buffer_id, BufferId(1));
    let source = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(source.buffer.cursor_position(), Position::new(1, 10));
    assert_eq!(
        source.buffer.selection_range(),
        Some(TextRange::new(Position::new(1, 5), Position::new(1, 10)))
    );
}

#[test]
fn status_history_command_opens_read_only_status_window_once() {
    let mut app = AppState::new();
    app.set_status("Opened sample.txt");
    app.set_status("Save failed: disk full");

    app.handle_command(&EditorCommand::App(AppCommand::StatusHistory));

    let status_window = app.workspace.focused_window().unwrap();
    let status_window_id = status_window.id;
    let status_buffer_id = status_window.buffer_id;
    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(status_window.title, "Status History");
    assert_eq!(status_window.kind, WindowKind::StatusHistory);
    assert_eq!(status_window.buffer_kind, BufferKind::ReadOnly);

    let status_buffer = app.buffer_state(status_buffer_id).unwrap();
    let text = status_buffer.buffer.to_text();
    assert!(status_buffer.buffer.is_read_only());
    assert!(text.contains("[info] Opened sample.txt"));
    assert!(text.contains("[error] Save failed: disk full"));
    assert!(text.contains("[info] Status history"));

    app.handle_command(&EditorCommand::App(AppCommand::StatusHistory));

    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(app.workspace.focused, status_window_id);

    app.handle_command(&EditorCommand::Window(WindowCommand::Close));
    assert_eq!(app.workspace.window_count(), 1);
    assert!(app.buffer_state(status_buffer_id).is_none());
}

#[test]
fn status_history_window_refreshes_when_status_changes() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::StatusHistory));
    let status_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    assert!(
        !app.buffer_state(status_buffer_id)
            .unwrap()
            .buffer
            .to_text()
            .contains("Later")
    );

    app.set_status("Later");

    assert!(
        app.buffer_state(status_buffer_id)
            .unwrap()
            .buffer
            .to_text()
            .contains("[info] Later")
    );
}

#[test]
fn status_history_is_capped_to_recent_entries() {
    let mut app = AppState::new();

    for index in 0..(STATUS_HISTORY_LIMIT + 2) {
        app.set_status(format!("message {index}"));
    }

    assert_eq!(app.status_history.len(), STATUS_HISTORY_LIMIT);
    assert_eq!(app.status_history[0].message, "message 2");
    assert_eq!(
        app.status_history[STATUS_HISTORY_LIMIT - 1].message,
        format!("message {}", STATUS_HISTORY_LIMIT + 1)
    );
}

#[test]
fn f2_key_opens_status_history() {
    let mut app = AppState::new();

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(2), CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::StatusHistory
    );
}

#[test]
fn focused_status_reports_dirty_buffer_name() {
    let mut app = AppState::new();

    app.handle_text_input('x');

    assert_eq!(app.focused_buffer_status(), "[Plain Text*]");
}

#[test]
fn focused_detail_status_reports_position_and_buffer_metadata() {
    let mut app = app_with_text("a\n中x");
    app.shell.profile = TerminalProfile::new(EncodingProfile::Ascii, ColorProfile::Mono);
    app.buffers[0]
        .buffer
        .set_cursor(Position::new(1, "中".len()))
        .unwrap();

    assert_eq!(
        app.focused_detail_status(),
        "[LF] [UTF-8] [Spaces:4] 2:3 [View 1-1/2] [ASCII/mono] [Win 1/1]"
    );
}

#[test]
fn focused_detail_status_reports_crlf_and_focused_window_index() {
    let mut app = AppState::new();
    app.shell.profile = TerminalProfile::new(EncodingProfile::Utf8, ColorProfile::Color16);
    app.buffers[0].buffer = TextBuffer::from_text("one\r\ntwo");
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));

    assert_eq!(
        app.focused_detail_status(),
        "[LF] [UTF-8] [Spaces:4] 1:1 [View 1-1/1] [UTF-8/16c] [Win 2/2]"
    );

    app.workspace.focused = WindowId(1);

    assert_eq!(
        app.focused_detail_status(),
        "[CRLF] [UTF-8] [Spaces:4] 1:1 [View 1-1/2] [UTF-8/16c] [Win 1/2]"
    );
}

#[test]
fn focused_detail_status_reports_selection_summary() {
    let mut app = app_with_text("abc def");
    app.shell.profile = TerminalProfile::utf8_256();
    app.buffers[0]
        .buffer
        .select(Position::new(0, 0), Position::new(0, 3))
        .unwrap();

    assert_eq!(
        app.focused_detail_status(),
        "[LF] [UTF-8] [Spaces:4] 1:4 [Sel 3c] [View 1-1/1] [UTF-8/256c] [Win 1/1]"
    );
}
