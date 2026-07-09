#![allow(unused_imports)]

use super::support::*;

#[test]
fn window_command_creates_focused_buffer_for_split() {
    let mut app = AppState::new();
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));

    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));

    let focused = app.workspace.focused_window().unwrap();
    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(app.buffers.len(), 2);
    assert!(app.buffer_state(focused.buffer_id).is_some());
}

#[test]
fn window_focus_and_resize_commands_report_status() {
    let mut app = AppState::new();
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));

    app.handle_command(&EditorCommand::Window(WindowCommand::FocusLeft));
    assert_eq!(
        app.status_message,
        Some("Focus left failed: no neighboring pane".to_string())
    );

    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    assert_eq!(app.status_message, Some("Split horizontally".to_string()));

    let right = app.workspace.focused;
    app.handle_command(&EditorCommand::Window(WindowCommand::FocusLeft));
    assert_ne!(app.workspace.focused, right);
    assert_eq!(app.status_message, Some("Focused left".to_string()));

    app.handle_command(&EditorCommand::Window(WindowCommand::ResizeDown));
    assert_eq!(
        app.status_message,
        Some("Resize down failed: no matching split".to_string())
    );

    app.handle_command(&EditorCommand::Window(WindowCommand::ResizeRight));
    assert_eq!(app.status_message, Some("Resized right".to_string()));
}

#[test]
fn window_layout_commands_report_status() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::Window(WindowCommand::RotateSplit));
    assert_eq!(
        app.status_message,
        Some("Rotate split failed: no matching split".to_string())
    );

    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    app.handle_command(&EditorCommand::Window(WindowCommand::RotateSplit));
    assert_eq!(
        app.status_message,
        Some("Rotated focused split to vertical".to_string())
    );

    app.handle_command(&EditorCommand::Window(WindowCommand::ToggleCollapse));
    assert!(app.workspace.focused_window().unwrap().collapsed);
    assert_eq!(app.status_message, Some("Collapsed pane".to_string()));

    app.handle_command(&EditorCommand::Window(WindowCommand::Expand));
    assert!(!app.workspace.focused_window().unwrap().collapsed);
    assert_eq!(app.status_message, Some("Expanded pane".to_string()));

    app.handle_command(&EditorCommand::Window(WindowCommand::Equalize));
    assert_eq!(app.status_message, Some("Equalized splits".to_string()));

    app.handle_command(&EditorCommand::Window(WindowCommand::Only));
    assert_eq!(
        app.status_message,
        Some("Only window is not implemented yet".to_string())
    );
}

#[test]
fn window_close_drops_unreferenced_buffer() {
    let mut app = AppState::new();
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    let closed_buffer_id = app.workspace.focused_window().unwrap().buffer_id;

    app.handle_command(&EditorCommand::Window(WindowCommand::Close));

    assert_eq!(app.workspace.window_count(), 1);
    assert_eq!(app.buffers.len(), 1);
    assert!(app.buffer_state(closed_buffer_id).is_none());
    assert_eq!(app.status_message, Some("Closed window".to_string()));
}

#[test]
fn buffer_switcher_focuses_selected_buffer() {
    let mut app = AppState::new();
    let first_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    let second_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    assert_ne!(first_buffer_id, second_buffer_id);

    app.handle_command(&EditorCommand::File(FileCommand::SwitchBuffer));
    assert!(
        app.active_overlay()
            .unwrap()
            .title
            .contains("Switch Buffer")
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Up, CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(
        app.workspace.focused_window().unwrap().buffer_id,
        first_buffer_id
    );
    assert_eq!(app.status_message, Some("Switched to Untitled".to_string()));
}

#[test]
fn buffer_switcher_reports_single_buffer_and_escape_cancels() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::SwitchBuffer));

    assert!(app.buffer_switcher.is_none());
    assert_eq!(
        app.status_message,
        Some("Buffer switcher: only one buffer".to_string())
    );

    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    app.handle_command(&EditorCommand::File(FileCommand::SwitchBuffer));
    assert!(app.buffer_switcher.is_some());

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Esc, CrosstermKeyModifiers::NONE),
    );

    assert!(app.buffer_switcher.is_none());
    assert_eq!(
        app.status_message,
        Some("Switch buffer cancelled".to_string())
    );
}

#[test]
fn buffer_switcher_overlay_reports_scroll_overflow() {
    let mut app = AppState::new();
    for _ in 0..13 {
        app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    }

    app.handle_command(&EditorCommand::File(FileCommand::SwitchBuffer));
    let overlay = app.active_overlay().expect("buffer switcher overlay");
    assert_eq!(overlay.title, "Switch Buffer");
    assert!(overlay.list_has_more_above);
    assert!(!overlay.list_has_more_below);

    app.move_buffer_switcher_selection(-20);
    let overlay = app.active_overlay().expect("buffer switcher overlay");
    assert!(!overlay.list_has_more_above);
    assert!(overlay.list_has_more_below);
}

#[test]
fn buffer_switcher_home_end_jump_to_first_and_last_buffer() {
    let mut app = AppState::new();
    let first_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    for _ in 0..13 {
        app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    }
    let last_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    assert_ne!(first_buffer_id, last_buffer_id);

    app.handle_command(&EditorCommand::File(FileCommand::SwitchBuffer));
    let overlay = app.active_overlay().expect("buffer switcher overlay");
    assert!(
        overlay
            .buttons
            .iter()
            .any(|button| button.contains("Home/End"))
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Home, CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(
        app.workspace.focused_window().unwrap().buffer_id,
        first_buffer_id
    );

    app.handle_command(&EditorCommand::File(FileCommand::SwitchBuffer));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::End, CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(
        app.workspace.focused_window().unwrap().buffer_id,
        last_buffer_id
    );
}

#[test]
fn buffer_switcher_page_keys_and_mouse_select_visible_entries() {
    let mut app = AppState::new();
    app.mouse_enabled = true;
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));
    let first_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    for _ in 0..13 {
        app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    }
    let last_buffer_id = app.workspace.focused_window().unwrap().buffer_id;

    app.handle_command(&EditorCommand::File(FileCommand::SwitchBuffer));
    let last_index = app.buffer_switcher.as_ref().unwrap().selected_index;
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::PageUp, CrosstermKeyModifiers::NONE),
    );
    assert!(app.buffer_switcher.as_ref().unwrap().selected_index < last_index);
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::PageDown, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.buffer_switcher.as_ref().unwrap().selected_index,
        last_index
    );

    handle_mouse_event(&mut app, scroll_up(20, 8));
    assert_eq!(
        app.buffer_switcher.as_ref().unwrap().selected_index,
        last_index - 1
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Home, CrosstermKeyModifiers::NONE),
    );
    let (x, y) = buffer_switcher_list_point(&app, 0);
    handle_mouse_event(&mut app, left_click(x, y));

    assert_eq!(
        app.workspace.focused_window().unwrap().buffer_id,
        first_buffer_id
    );
    assert_eq!(app.status_message, Some("Switched to Untitled".to_string()));

    app.handle_command(&EditorCommand::File(FileCommand::SwitchBuffer));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::End, CrosstermKeyModifiers::NONE),
    );
    let (x, y) = buffer_switcher_list_point(
        &app,
        app.active_overlay().unwrap().selected_list_index.unwrap(),
    );
    handle_mouse_event(&mut app, left_click(x, y));

    assert_eq!(
        app.workspace.focused_window().unwrap().buffer_id,
        last_buffer_id
    );
}

fn buffer_switcher_list_point(app: &AppState, visible_index: usize) -> (u16, u16) {
    let overlay = app.active_overlay().expect("buffer switcher overlay");
    let area = app.overlay_area();
    for y in 0..area.height {
        if app.shell.hit_test_overlay_list(&overlay, area, 20, y) == Some(visible_index) {
            return (20, y);
        }
    }

    panic!("visible buffer switcher row {visible_index} was not hittable");
}

#[test]
fn window_close_reports_last_window_failure() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::Window(WindowCommand::Close));

    assert_eq!(app.workspace.window_count(), 1);
    assert_eq!(
        app.status_message,
        Some("Close failed: cannot close the last window".to_string())
    );
}

#[test]
fn close_dirty_window_can_be_cancelled() {
    let mut app = AppState::new();
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    let focused = app.workspace.focused;
    app.handle_text_input('x');

    app.handle_command(&EditorCommand::Window(WindowCommand::Close));

    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(app.workspace.focused, focused);
    assert_eq!(
        app.confirm_status_text(),
        Some("Unsaved changes in Untitled-2: Save(s) Discard(d) Cancel(c)".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Esc, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 2);
    assert_eq!(app.workspace.focused, focused);
    assert_eq!(
        app.status_message,
        Some("Unsaved changes cancelled".to_string())
    );
}
