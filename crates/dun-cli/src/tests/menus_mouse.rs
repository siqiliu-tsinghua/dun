#![allow(unused_imports)]

use super::support::*;

#[test]
fn mouse_click_is_ignored_when_mouse_is_disabled() {
    let mut app = AppState::new();
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));
    let left = app.workspace.focused;
    let right = app.workspace.split_focused(Axis::Horizontal).unwrap();
    assert_eq!(app.workspace.focused, right);

    handle_mouse_event(&mut app, left_click(3, 2));

    assert_eq!(app.workspace.focused, right);
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .cursor_position(),
        Position::zero()
    );
    assert_eq!(left, WindowId(1));
}

#[test]
fn mouse_click_focuses_window_when_enabled() {
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));
    let left = app.workspace.focused;
    let right = app.workspace.split_focused(Axis::Horizontal).unwrap();
    assert_eq!(app.workspace.focused, right);

    handle_mouse_event(&mut app, left_click(3, 2));

    assert_eq!(app.workspace.focused, left);
}

#[test]
fn mouse_body_click_moves_cursor_when_enabled() {
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));
    app.handle_text_input('a');
    app.handle_text_input('b');
    app.handle_text_input('c');
    app.handle_text_input('d');

    handle_mouse_event(&mut app, left_click(5, 2));

    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .cursor_position(),
        Position::new(0, 2)
    );
}

#[test]
fn mouse_wheel_scrolls_editor_body_when_enabled() {
    let text = (0..20)
        .map(|index| format!("line{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.buffer_state_mut(BufferId(1)).unwrap().buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
    app.sync_view_for_area(Rect::new(0, 0, 80, 8));

    handle_mouse_event(&mut app, scroll_down(10, 3));

    let buffer = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(buffer.first_line, EDITOR_MOUSE_WHEEL_LINES);
    assert_eq!(
        buffer.buffer.cursor_position(),
        Position::new(EDITOR_MOUSE_WHEEL_LINES, 0)
    );

    handle_mouse_event(&mut app, scroll_up(10, 3));
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().first_line, 0);
}

#[test]
fn mouse_wheel_scrolls_wrapped_visual_rows() {
    let mut app = AppState::new();
    app.mouse_enabled = true;
    let state = app.buffer_state_mut(BufferId(1)).unwrap();
    state.buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdefghijklmnop");
    state.word_wrap = true;
    app.sync_view_for_area(Rect::new(0, 0, 12, 3));

    handle_mouse_event(&mut app, scroll_down(5, 2));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.first_line, 0);
    assert_eq!(state.first_visual_row, 1);
    assert_eq!(state.buffer.cursor_position(), Position::new(0, 8));
    assert!(
        scroll_status(
            state,
            app.focused_buffer_view_context(app.workspace_area),
            app.shell.profile.ambiguous_width,
        )
        .contains("View V2-2/2 L1 wrap")
    );
}

#[test]
fn mouse_scrollbar_click_and_drag_scrolls_editor_body_when_enabled() {
    let text = (0..20)
        .map(|index| format!("line{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.buffer_state_mut(BufferId(1)).unwrap().buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
    app.sync_view_for_area(Rect::new(0, 0, 80, 8));

    handle_mouse_event(&mut app, left_click(79, 5));

    assert_eq!(app.buffer_state(BufferId(1)).unwrap().first_line, 8);

    handle_mouse_event(&mut app, left_drag(79, 7));
    handle_mouse_event(&mut app, left_up(79, 7));

    assert_eq!(app.buffer_state(BufferId(1)).unwrap().first_line, 14);
    assert_eq!(app.mouse_drag, None);
}

#[test]
fn mouse_menu_click_dispatches_command_when_enabled() {
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));
    assert_eq!(app.shell.menu_index_at_column(20), Some(3));

    handle_mouse_event(&mut app, left_click(20, 0));
    assert_eq!(app.active_menu, Some(3));
    handle_mouse_event(&mut app, left_click(20, 2));

    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::Help
    );
}

#[test]
fn mouse_click_outside_open_menu_closes_it() {
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));

    handle_mouse_event(&mut app, left_click(20, 0));
    assert_eq!(app.active_menu, Some(3));
    handle_mouse_event(&mut app, left_click(0, 2));

    assert_eq!(app.active_menu, None);
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::Edit
    );
}

#[test]
fn escape_closes_active_menu_before_keymap_dispatch() {
    let mut app = AppState::new();
    app.active_menu = Some(0);
    app.active_menu_entry = Some(0);

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Esc, TerminalKeyModifiers::NONE),
    );

    assert_eq!(app.active_menu, None);
    assert_eq!(app.active_menu_entry, None);
    assert!(!app.should_quit);
}

#[test]
fn alt_mnemonic_opens_menu_without_mouse_enabled() {
    let mut app = AppState::new();

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('h'), TerminalKeyModifiers::ALT),
    );

    assert_eq!(app.active_menu, Some(3));
    assert_eq!(app.active_menu_entry, Some(0));
}

#[test]
fn keyboard_menu_enter_dispatches_selected_entry() {
    let mut app = AppState::new();

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('h'), TerminalKeyModifiers::ALT),
    );
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    assert_eq!(app.active_menu, None);
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::Help
    );
}

#[test]
fn keyboard_menu_arrows_switch_menu_and_entry() {
    let mut app = AppState::new();
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('f'), TerminalKeyModifiers::ALT),
    );
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Right, TerminalKeyModifiers::NONE),
    );
    assert_eq!(app.active_menu, Some(1));
    assert_eq!(app.active_menu_entry, Some(0));

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Right, TerminalKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Down, TerminalKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    assert_eq!(app.status_message, Some("Split vertically".to_string()));
    assert_eq!(app.workspace.window_count(), 2);
}

#[test]
fn mouse_drag_selects_text_in_editor_body() {
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));
    app.handle_text_input('a');
    app.handle_text_input('b');
    app.handle_text_input('c');
    app.handle_text_input('d');

    handle_mouse_event(&mut app, left_click(4, 2));
    handle_mouse_event(&mut app, left_drag(6, 2));
    handle_mouse_event(&mut app, left_up(6, 2));

    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .selection_range(),
        Some(TextRange::new(Position::new(0, 1), Position::new(0, 3)))
    );
    assert_eq!(app.mouse_drag, None);
}

#[test]
fn mouse_drag_selection_scrolls_when_dragged_to_window_edge() {
    let text = (0..20)
        .map(|index| format!("line{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.buffer_state_mut(BufferId(1)).unwrap().buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
    app.sync_view_for_area(Rect::new(0, 0, 80, 8));

    handle_mouse_event(&mut app, left_click(4, 6));
    handle_mouse_event(&mut app, left_drag(4, 8));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert!(state.first_line > 0);
    assert!(
        state
            .buffer
            .selection_range()
            .is_some_and(|range| range.end.line >= state.first_line)
    );
}

#[test]
fn mouse_drag_resizes_split_boundary() {
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));
    let left = app.workspace.focused;
    let right = app.workspace.split_focused(Axis::Horizontal).unwrap();

    handle_mouse_event(&mut app, left_click(40, 2));
    handle_mouse_event(&mut app, left_drag(60, 2));
    handle_mouse_event(&mut app, left_up(60, 2));

    let layouts = app.workspace.resolved_layout(Rect::new(0, 0, 80, 20));
    assert_eq!(
        layouts
            .iter()
            .find(|layout| layout.id == left)
            .unwrap()
            .rect,
        Rect::new(0, 0, 60, 20)
    );
    assert_eq!(
        layouts
            .iter()
            .find(|layout| layout.id == right)
            .unwrap()
            .rect,
        Rect::new(60, 0, 20, 20)
    );
    assert_eq!(app.mouse_drag, None);
}

#[test]
fn mouse_wheel_scroll_changes_file_dialog_click_target() {
    let directory = temp_file_path("open-dialog-wheel");
    std::fs::create_dir(&directory).unwrap();
    for index in 0..14 {
        std::fs::write(
            directory.join(format!("item{index:02}.txt")),
            format!("item{index:02}"),
        )
        .unwrap();
    }
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.sync_view_for_area(Rect::new(0, 0, 90, 14));

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/", directory.display()));
    handle_mouse_event(&mut app, scroll_down(20, 8));
    assert_eq!(
        app.file_dialog.as_ref().map(|dialog| dialog.scroll_offset),
        Some(1)
    );
    let (x, y) = file_dialog_list_point(&app, 0);
    handle_mouse_event(&mut app, left_click(x, y));

    let path = directory.join("item00.txt");
    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "item00");
    assert_eq!(state.path.as_ref(), Some(&path));

    for index in 0..14 {
        let _ = std::fs::remove_file(directory.join(format!("item{index:02}.txt")));
    }
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn mouse_click_open_dialog_file_opens_selected_file() {
    let directory = temp_file_path("open-dialog-mouse-file");
    let first = directory.join("a.txt");
    let second = directory.join("b.txt");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(&first, "first").unwrap();
    std::fs::write(&second, "second").unwrap();
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.sync_view_for_area(Rect::new(0, 0, 90, 14));

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/", directory.display()));
    let (x, y) = file_dialog_list_point(&app, 2);
    handle_mouse_event(&mut app, left_click(x, y));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "second");
    assert_eq!(state.path.as_ref(), Some(&second));
    assert_eq!(app.prompt_status_text(), None);

    let _ = std::fs::remove_file(first);
    let _ = std::fs::remove_file(second);
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn mouse_click_open_dialog_directory_enters_directory() {
    let directory = temp_file_path("open-dialog-mouse-dir");
    let child = directory.join("child");
    std::fs::create_dir(&directory).unwrap();
    std::fs::create_dir(&child).unwrap();
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.sync_view_for_area(Rect::new(0, 0, 90, 14));

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/", directory.display()));
    let (x, y) = file_dialog_list_point(&app, 1);
    handle_mouse_event(&mut app, left_click(x, y));

    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Open: {}/", child.display()))
    );
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");

    let _ = std::fs::remove_dir(child);
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn mouse_click_save_as_dialog_directory_updates_input_without_saving() {
    let directory = temp_file_path("save-as-dialog-mouse");
    let child = directory.join("child");
    std::fs::create_dir(&directory).unwrap();
    std::fs::create_dir(&child).unwrap();
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.sync_view_for_area(Rect::new(0, 0, 90, 14));
    app.handle_text_input('x');

    app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
    send_text(&mut app, &format!("{}/", directory.display()));
    let (x, y) = file_dialog_list_point(&app, 1);
    handle_mouse_event(&mut app, left_click(x, y));

    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Save As: {}/", child.display()))
    );
    let state = app.buffer_state(BufferId(1)).unwrap();
    assert!(state.buffer.is_dirty());
    assert_eq!(state.path, None);

    let _ = std::fs::remove_dir(child);
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn right_click_reports_paste_wait_status_when_mouse_is_enabled() {
    let mut app = AppState::new();
    app.mouse_enabled = true;

    handle_mouse_event(&mut app, right_click(3, 2));

    assert_eq!(
        app.status_message,
        Some("Paste: waiting for terminal bracketed paste data".to_string())
    );
}
