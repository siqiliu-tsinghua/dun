#![allow(unused_imports)]

use super::support::*;

#[test]
fn run_command_prompt_opens_read_only_output_window() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
    assert_eq!(app.prompt_status_text(), Some("Run Command: ".to_string()));
    send_text(&mut app, "printf dun-run");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let window = app.workspace.focused_window().unwrap();
    assert_eq!(window.kind, WindowKind::CommandOutput);
    let buffer = app.buffer_state(window.buffer_id).unwrap();
    assert!(buffer.buffer.is_read_only());
    let text = buffer.buffer.to_text();
    assert!(text.contains("Command: printf dun-run"));
    assert!(text.contains("Stdout: 7 bytes, complete"));
    assert!(text.contains("Truncated: no"));
    assert!(text.contains("--- stdout (7 bytes, complete) ---\ndun-run\n"));
    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|status| status.contains("Command returned exit 0"))
    );
}

#[test]
fn run_command_history_navigates_separately_from_command_line_history() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
    send_text(&mut app, "printf first");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
    send_text(&mut app, "printf second");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    app.handle_command(&EditorCommand::App(AppCommand::RunCommand));
    send_text(&mut app, "draft");

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Run Command: printf second".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Run Command: printf first".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Run Command: printf second".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some("Run Command: draft".to_string())
    );
    assert!(app.command_history.is_empty());
    assert_eq!(
        app.run_command_history,
        vec!["printf first".to_string(), "printf second".to_string()]
    );
}

#[test]
fn run_command_reuses_output_window_for_new_results() {
    let mut app = AppState::new();

    app.run_external_command_to_buffer("printf one");
    let first_window = app.workspace.focused_window().unwrap().clone();
    let window_count = app.workspace.windows.len();

    app.run_external_command_to_buffer("printf two");

    let second_window = app.workspace.focused_window().unwrap();
    assert_eq!(app.workspace.windows.len(), window_count);
    assert_eq!(second_window.id, first_window.id);
    assert_eq!(second_window.kind, WindowKind::CommandOutput);
    assert_eq!(second_window.buffer_kind, BufferKind::ReadOnly);
    assert!(!second_window.collapsed);
    let text = app
        .buffer_state(second_window.buffer_id)
        .unwrap()
        .buffer
        .to_text();
    assert!(text.contains("Command: printf two"));
    assert!(text.contains("two"));
    assert!(!text.contains("one"));
}
