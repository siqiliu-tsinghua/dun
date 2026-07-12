#![allow(unused_imports)]

use super::support::*;

fn split_with_dirty_first_window(app: &mut AppState) -> (WindowId, BufferId) {
    let first = app.workspace.focused;
    let dirty_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    let target = app.workspace.focused;
    app.workspace.focused = first;
    app.handle_text_input('x');
    app.workspace.focused = target;
    (target, dirty_buffer_id)
}

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

    // A fresh split is already even, so there is nothing to equalize -- and the
    // command must say so rather than claim work it did not do.
    app.handle_command(&EditorCommand::Window(WindowCommand::Equalize));
    assert_eq!(
        app.status_message,
        Some("Splits are already even".to_string())
    );
}

/// Both commands used to report success on a no-op: Equalize announced
/// "Equalized splits" with a single window and no splits at all, and Expand
/// announced "Expanded pane" at a pane that was never collapsed. That is the
/// same lie as the Save that rewrote an unchanged file -- and the rest of this
/// codebase already reports no-ops honestly ("Already at left edge", "Already
/// the only window", "Outdent: nothing changed").
#[test]
fn equalize_and_expand_do_not_claim_work_they_did_not_do() {
    let mut app = AppState::new();
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));

    app.handle_command(&EditorCommand::Window(WindowCommand::Equalize));
    assert_eq!(
        app.status_message,
        Some("Splits are already even".to_string()),
        "a single window has no splits to equalize"
    );

    app.handle_command(&EditorCommand::Window(WindowCommand::Expand));
    assert_eq!(
        app.status_message,
        Some("Pane is already expanded".to_string()),
        "a pane that was never collapsed has nothing to expand"
    );

    // And when there is real work, it is reported as real work.
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    app.handle_command(&EditorCommand::Window(WindowCommand::Equalize));
    assert_eq!(
        app.status_message,
        Some("Splits are already even".to_string()),
        "a fresh split is already even"
    );

    // The split lands focus on the right pane, which has no split to its right;
    // move left first so the resize has something to grab.
    app.handle_command(&EditorCommand::Window(WindowCommand::FocusLeft));
    app.handle_command(&EditorCommand::Window(WindowCommand::ResizeRight));
    assert_eq!(app.status_message, Some("Resized right".to_string()));

    app.handle_command(&EditorCommand::Window(WindowCommand::Equalize));
    assert_eq!(app.status_message, Some("Equalized 1 split(s)".to_string()));
}

#[test]
fn window_only_closes_other_windows_and_keeps_the_focused_one() {
    let mut app = AppState::new();
    let first_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    let target = app.workspace.focused;
    let target_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitVertical));
    let third_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    app.workspace.focused = target;

    app.handle_command(&EditorCommand::Window(WindowCommand::Only));

    assert_eq!(app.workspace.window_count(), 1);
    assert_eq!(app.workspace.focused, target);
    assert_eq!(
        app.workspace.focused_window().unwrap().buffer_id,
        target_buffer_id
    );
    assert_eq!(app.buffers.len(), 1);
    assert!(app.buffer_state(target_buffer_id).is_some());
    assert!(app.buffer_state(first_buffer_id).is_none());
    assert!(app.buffer_state(third_buffer_id).is_none());
    assert_eq!(
        app.status_message,
        Some("Closed 2 other window(s)".to_string())
    );
}

#[test]
fn window_only_reports_a_single_window_no_op() {
    let mut app = AppState::new();
    let before = app.workspace.clone();

    app.handle_command(&EditorCommand::Window(WindowCommand::Only));

    assert_eq!(app.workspace, before);
    assert_eq!(
        app.status_message,
        Some("Already the only window".to_string())
    );
}

#[test]
fn window_only_cancel_keeps_every_window_and_dirty_buffer() {
    let mut app = AppState::new();
    let (target, dirty_buffer_id) = split_with_dirty_first_window(&mut app);
    let dirty_text = app.buffer_state(dirty_buffer_id).unwrap().buffer.to_text();

    app.handle_command(&EditorCommand::Window(WindowCommand::Only));

    assert_eq!(app.workspace.window_count(), 2);
    assert_ne!(app.workspace.focused, target);
    assert!(app.confirm.is_some());

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Esc, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 2);
    assert!(app.workspace.window(target).is_ok());
    assert!(app.confirm.is_none());
    let dirty = app.buffer_state(dirty_buffer_id).unwrap();
    assert_eq!(dirty.buffer.to_text(), dirty_text);
    assert!(dirty.buffer.is_dirty());
}

#[test]
fn window_only_save_writes_the_dirty_buffer_and_restores_the_target() {
    let path = temp_file_path("window-only-save.txt");
    std::fs::write(&path, "old").unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();
    let (target, dirty_buffer_id) = split_with_dirty_first_window(&mut app);

    app.handle_command(&EditorCommand::Window(WindowCommand::Only));
    assert_ne!(app.workspace.focused, target);

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('s'), CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 1);
    assert_eq!(app.workspace.focused, target);
    assert!(app.confirm.is_none());
    assert!(app.buffer_state(dirty_buffer_id).is_none());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xold");

    let _ = std::fs::remove_file(path);
}

#[test]
fn window_only_discard_closes_once_without_saving_and_restores_the_target() {
    let mut app = AppState::new();
    let (target, dirty_buffer_id) = split_with_dirty_first_window(&mut app);

    app.handle_command(&EditorCommand::Window(WindowCommand::Only));
    assert!(app.confirm.is_some());
    assert_ne!(app.workspace.focused, target);

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('d'), CrosstermKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 1);
    assert_eq!(app.workspace.focused, target);
    assert!(app.confirm.is_none());
    assert!(app.file_dialog.is_none());
    assert!(app.buffer_state(dirty_buffer_id).is_none());
    assert_eq!(
        app.status_message,
        Some("Closed 1 other window(s)".to_string())
    );
}

#[test]
fn window_only_does_not_prompt_for_a_dirty_buffer_shown_in_the_target() {
    let mut app = AppState::new();
    let first = app.workspace.focused;
    let shared_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    let target = app.workspace.focused;
    let unused_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    app.workspace.window_mut(target).unwrap().buffer_id = shared_buffer_id;
    app.buffers.retain(|buffer| buffer.id != unused_buffer_id);
    app.workspace.focused = first;
    app.handle_text_input('x');
    app.workspace.focused = target;

    app.handle_command(&EditorCommand::Window(WindowCommand::Only));

    assert_eq!(app.workspace.window_count(), 1);
    assert_eq!(app.workspace.focused, target);
    assert!(app.confirm.is_none());
    assert!(
        app.buffer_state(shared_buffer_id)
            .unwrap()
            .buffer
            .is_dirty()
    );
}

#[test]
fn window_only_expands_the_collapsed_survivor() {
    let mut app = AppState::new();
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    app.handle_command(&EditorCommand::Window(WindowCommand::Collapse));
    assert!(app.workspace.focused_window().unwrap().collapsed);

    app.handle_command(&EditorCommand::Window(WindowCommand::Only));

    assert_eq!(app.workspace.window_count(), 1);
    assert!(!app.workspace.focused_window().unwrap().collapsed);
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

/// A collapsed pane draws no body. Keystrokes used to keep editing the buffer
/// behind the empty box, so the user blind-typed into a file they could not
/// see and the dirty marker in the title was the only hint. The menu behaviour
/// matrix caught the single-window half of this; this pins the rest.
#[test]
fn a_collapsed_pane_cannot_be_edited_through() {
    let mut app = AppState::new();
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    app.handle_command(&EditorCommand::Window(WindowCommand::Collapse));
    assert!(app.workspace.focused_is_collapsed());

    let before = app
        .focused_buffer()
        .expect("a focused buffer")
        .buffer
        .to_text();

    send_text(&mut app, "blind");
    app.handle_command(&EditorCommand::Edit(EditCommand::DeleteLine));
    app.handle_paste("pasted");

    let after = app
        .focused_buffer()
        .expect("a focused buffer")
        .buffer
        .to_text();
    assert_eq!(after, before, "an invisible pane must not be editable");
    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|status| status.starts_with("Pane is collapsed")),
        "and the refusal must say why: {:?}",
        app.status_message
    );

    // Expanding hands the pane back.
    app.handle_command(&EditorCommand::Window(WindowCommand::Expand));
    send_text(&mut app, "ok");
    assert!(
        app.focused_buffer()
            .unwrap()
            .buffer
            .to_text()
            .contains("ok"),
        "an expanded pane edits normally again"
    );
}
