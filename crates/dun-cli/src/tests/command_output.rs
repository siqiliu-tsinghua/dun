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

#[test]
fn command_output_actions_copy_clear_jump_and_save_output() {
    let mut app = AppState::new();
    app.run_external_command_to_buffer("printf stdout; printf stderr >&2");

    app.handle_command(&EditorCommand::App(AppCommand::CommandOutputCopy));
    assert!(
        app.kill_ring
            .as_deref()
            .is_some_and(|text| text.contains("stdout") && text.contains("stderr"))
    );
    assert_eq!(
        app.status_message,
        Some("Copied Command Output".to_string())
    );

    app.handle_command(&EditorCommand::App(AppCommand::CommandOutputStderr));
    let window = app.workspace.focused_window().unwrap();
    let output_buffer_id = window.buffer_id;
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("--- stderr (6 bytes, complete) ---")
    );

    app.handle_command(&EditorCommand::App(AppCommand::CommandOutputStdout));
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("--- stdout (6 bytes, complete) ---")
    );

    app.handle_command(&EditorCommand::App(AppCommand::CommandOutputSummary));
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("Command: printf stdout; printf stderr >&2")
    );

    submit_command_line(&mut app, "output find stderr");
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert!(
        buffer
            .search_status()
            .is_some_and(|status| status == "Find 1/7")
    );

    submit_command_line(&mut app, "output index");
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("Index")
    );

    submit_command_line(&mut app, "output stdout-body");
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("stdout")
    );

    submit_command_line(&mut app, "output stderr-body");
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("stderr")
    );

    app.handle_command(&EditorCommand::App(AppCommand::CommandOutputStatus));
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert!(
        buffer
            .buffer
            .line(buffer.buffer.cursor_position().line)
            .is_some_and(|line| line.starts_with("Status: "))
    );

    submit_command_line(&mut app, "output truncated");
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("Truncated: no")
    );
    assert_eq!(
        app.status_message,
        Some("Command Output: truncated".to_string())
    );

    submit_command_line(&mut app, "output next-section");
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("Index")
    );
    assert_eq!(
        app.status_message,
        Some("Command Output: index".to_string())
    );

    submit_command_line(&mut app, "output next-section");
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("--- stdout (6 bytes, complete) ---")
    );
    assert_eq!(
        app.status_message,
        Some("Command Output: stdout".to_string())
    );

    submit_command_line(&mut app, "output previous-section");
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("Index")
    );
    assert_eq!(
        app.status_message,
        Some("Command Output: index".to_string())
    );

    submit_command_line(&mut app, "output only stdout");
    let view_window = app.workspace.focused_window().unwrap();
    assert_eq!(view_window.kind, WindowKind::CommandOutputView);
    let view_text = app
        .buffer_state(view_window.buffer_id)
        .unwrap()
        .buffer
        .to_text();
    assert!(view_text.contains("Dun Command Output stdout"));
    assert!(view_text.contains("stdout"));
    assert!(!view_text.contains("stderr"));

    submit_command_line(&mut app, "output only stderr");
    let view_window = app.workspace.focused_window().unwrap();
    assert_eq!(view_window.kind, WindowKind::CommandOutputView);
    let view_buffer_id = view_window.buffer_id;
    let view_text = app.buffer_state(view_buffer_id).unwrap().buffer.to_text();
    assert!(view_text.contains("Dun Command Output stderr"));
    assert!(view_text.contains("Section: stderr"));
    assert!(view_text.contains("Lines: "));
    assert!(view_text.contains("stderr"));
    assert!(!view_text.contains("stdout"));

    submit_command_line(&mut app, "output find stderr");
    let view_buffer = app.buffer_state(view_buffer_id).unwrap();
    assert!(view_buffer.search_status().is_some());

    let only_path = temp_file_path("command-output-only-save.txt");
    submit_command_line(
        &mut app,
        &format!("output save {}", only_path.to_string_lossy()),
    );
    let saved_only = std::fs::read_to_string(&only_path).unwrap();
    let _ = std::fs::remove_file(&only_path);
    assert!(saved_only.contains("Dun Command Output stderr"));
    assert!(saved_only.contains("stderr"));
    assert!(!saved_only.contains("stdout"));

    app.handle_command(&EditorCommand::Window(WindowCommand::Close));
    assert_eq!(
        app.workspace.focused_window().unwrap().buffer_id,
        output_buffer_id
    );

    let path = temp_file_path("command-output-save.txt");
    submit_command_line(&mut app, &format!("output save {}", path.to_string_lossy()));
    let saved = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(saved.contains("Command: printf stdout; printf stderr >&2"));
    assert!(saved.contains("stdout"));
    assert!(saved.contains("stderr"));

    app.handle_command(&EditorCommand::App(AppCommand::CommandOutputClear));
    let buffer = app
        .buffer_state(app.workspace.focused_window().unwrap().buffer_id)
        .unwrap();
    assert_eq!(buffer.buffer.to_text(), "Dun Command Output\n\n(empty)\n");
}

#[test]
fn command_output_find_next_previous_use_output_search_cache() {
    let mut app = AppState::new();
    app.run_external_command_to_buffer("printf seed");
    let output_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    app.buffer_state_mut(output_buffer_id).unwrap().buffer = command_output_buffer(
        "Dun Command Output\n\nCommand: generated\nShell: sh\nStatus: exit 0\nElapsed: 1ms\nLimit: 1 bytes per stream\nStdout: 2 bytes, complete\nStderr: 0 bytes, complete\nTruncated: no\n\nIndex\n  output next         next match\n\n--- stdout (2 bytes, complete) ---\nneedle\nother\nneedle\n--- stderr (0 bytes, complete) ---\n(empty)\n",
    );

    submit_command_line(&mut app, "output find needle");
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("needle")
    );
    assert!(
        buffer
            .search_status()
            .is_some_and(|status| status == "Find 1/2")
    );

    submit_command_line(&mut app, "output next");
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("needle")
    );
    assert!(
        buffer
            .search_status()
            .is_some_and(|status| status == "Find 2/2")
    );

    submit_command_line(&mut app, "output previous");
    let buffer = app.buffer_state(output_buffer_id).unwrap();
    assert!(
        buffer
            .search_status()
            .is_some_and(|status| status == "Find 1/2")
    );
}

#[test]
fn command_output_save_dialog_writes_output() {
    let mut app = AppState::new();
    app.run_external_command_to_buffer("printf dialog-save");
    let path = temp_file_path("command-output-dialog-save.txt");
    let _ = std::fs::remove_file(&path);

    app.handle_command(&EditorCommand::App(AppCommand::CommandOutputSave));
    assert_eq!(
        app.file_dialog.as_ref().map(FileDialogState::status_text),
        Some("Save Output: command-output.txt".to_string())
    );
    app.file_dialog
        .as_mut()
        .unwrap()
        .input
        .set_text(path.to_string_lossy().to_string());
    app.submit_file_dialog();

    let saved = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(saved.contains("dialog-save"));
    assert!(app.file_dialog.is_none());
    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|status| status.contains("Saved Command Output"))
    );
}

#[test]
fn command_output_save_dialog_requires_second_enter_before_overwrite() {
    let mut app = AppState::new();
    app.run_external_command_to_buffer("printf replacement-output");
    let path = temp_file_path("command-output-overwrite.txt");
    std::fs::write(&path, "old output").unwrap();

    app.handle_command(&EditorCommand::App(AppCommand::CommandOutputSave));
    app.file_dialog
        .as_mut()
        .unwrap()
        .input
        .set_text(path.to_string_lossy().to_string());
    app.submit_file_dialog();

    assert!(app.file_dialog.is_some());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old output");
    assert!(
        app.file_dialog
            .as_ref()
            .and_then(|dialog| dialog.message.as_deref())
            .is_some_and(|message| message.contains("Replace existing file"))
    );

    app.submit_file_dialog();

    let saved = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(saved.contains("replacement-output"));
    assert!(app.file_dialog.is_none());
    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|status| status.contains("Saved Command Output"))
    );
}

#[test]
fn command_output_save_dialog_keeps_dialog_on_write_error() {
    let mut app = AppState::new();
    app.run_external_command_to_buffer("printf cannot-save");
    let path = temp_file_path("missing-command-output-parent").join("output.txt");

    app.handle_command(&EditorCommand::App(AppCommand::CommandOutputSave));
    app.file_dialog
        .as_mut()
        .unwrap()
        .input
        .set_text(path.to_string_lossy().to_string());
    app.submit_file_dialog();

    assert!(app.file_dialog.is_some());
    assert!(!path.exists());
    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|status| status.contains("Command Output save failed"))
    );
    assert!(
        app.file_dialog
            .as_ref()
            .and_then(|dialog| dialog.message.as_deref())
            .is_some_and(|message| message.contains("Command Output save failed"))
    );
}
