#![allow(unused_imports)]

use super::support::*;

#[test]
fn prompt_cancel_restores_editor_input() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, "abc");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Esc, CrosstermKeyModifiers::NONE),
    );

    // Cancelling reports it, and the message stands until the next keypress.
    assert_eq!(app.status_message, Some("Open cancelled".to_string()));

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('x'), CrosstermKeyModifiers::NONE),
    );

    // That keypress both hands the status line back and reaches the buffer.
    assert_eq!(app.status_message, None);
    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "x");
}

#[test]
fn prompt_backspace_edits_prompt_not_buffer() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::Edit(EditCommand::Find));
    send_text(&mut app, "abc");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Backspace, CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(app.prompt_status_text(), Some("Find: ab".to_string()));
    assert_eq!(state.buffer.to_text(), "");
}

#[test]
fn new_command_confirms_dirty_buffer_before_clearing() {
    let mut app = AppState::new();
    app.handle_text_input('x');

    app.handle_command(&EditorCommand::File(FileCommand::New));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "x");
    assert_eq!(
        app.confirm_status_text(),
        Some("Unsaved changes in Untitled: Save(s) Discard(d) Cancel(c)".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('c'), CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "x");
    assert_eq!(
        app.status_message,
        Some("Unsaved changes cancelled".to_string())
    );

    app.handle_command(&EditorCommand::File(FileCommand::New));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('d'), CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "");
    assert_eq!(app.confirm_status_text(), None);
}

#[test]
fn quit_confirms_dirty_file_and_saves_before_exit() {
    let path = temp_file_path("confirm-quit-save.txt");
    std::fs::write(&path, "old").unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
    app.handle_text_input('!');
    app.handle_command(&EditorCommand::App(AppCommand::Quit));

    assert!(!app.should_quit);
    assert_eq!(
        app.confirm_status_text(),
        Some(format!(
            "Unsaved changes in {}: Save(s) Quit without saving(d) Cancel(c)",
            title_for_path(&path)
        ))
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('s'), CrosstermKeyModifiers::NONE),
    );

    assert!(app.should_quit);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old!");
    assert!(!app.buffer_state(BufferId(1)).unwrap().buffer.is_dirty());

    let _ = std::fs::remove_file(path);
}

#[test]
fn quit_dirty_untitled_save_prompts_for_save_as_then_exits() {
    let path = temp_file_path("confirm-quit-save-as.txt");
    let mut app = AppState::new();
    app.handle_text_input('x');

    app.handle_command(&EditorCommand::App(AppCommand::Quit));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('s'), CrosstermKeyModifiers::NONE),
    );

    assert!(!app.should_quit);
    assert_eq!(app.prompt_status_text(), Some("Save As: ".to_string()));

    send_text(&mut app, &path.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert!(app.should_quit);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");

    let _ = std::fs::remove_file(path);
}
