use super::support::*;

fn toggle_visible_whitespace(app: &mut AppState) {
    app.handle_command(&EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace));
}

fn toggle_bookmark(app: &mut AppState) {
    app.handle_command(&EditorCommand::Edit(EditCommand::ToggleBookmark));
}

fn send_ctrl_x_command(app: &mut AppState, suffix: char) {
    handle_key_event(
        app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('x'), TerminalKeyModifiers::CONTROL),
    );
    handle_key_event(
        app,
        TerminalKeyEvent::new(TerminalKeyCode::Char(suffix), TerminalKeyModifiers::NONE),
    );
}

#[test]
fn visible_whitespace_toggle_is_per_buffer_and_reports_exact_statuses() {
    let mut app = app_with_text("first");
    let first_buffer_id = app.focused_buffer_id().unwrap();
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    let second_buffer_id = app.focused_buffer_id().unwrap();
    assert_ne!(first_buffer_id, second_buffer_id);

    let cursor_before = app
        .buffer_state(second_buffer_id)
        .unwrap()
        .buffer
        .cursor_position();
    toggle_visible_whitespace(&mut app);

    assert!(
        app.buffer_state(second_buffer_id)
            .unwrap()
            .visible_whitespace
    );
    assert!(
        !app.buffer_state(first_buffer_id)
            .unwrap()
            .visible_whitespace,
        "the unfocused buffer must not inherit the toggle"
    );
    assert_eq!(
        app.buffer_state(second_buffer_id)
            .unwrap()
            .buffer
            .cursor_position(),
        cursor_before
    );
    assert!(
        !app.buffer_state(second_buffer_id)
            .unwrap()
            .buffer
            .is_dirty(),
        "display state must not dirty buffer text"
    );
    assert_eq!(app.status_message.as_deref(), Some("Visible whitespace on"));

    toggle_visible_whitespace(&mut app);
    assert!(
        !app.buffer_state(second_buffer_id)
            .unwrap()
            .visible_whitespace
    );
    assert_eq!(
        app.status_message.as_deref(),
        Some("Visible whitespace off")
    );
}

#[test]
fn visible_whitespace_toggle_reports_a_missing_focused_buffer() {
    let mut app = AppState::new();
    let focused = app.focused_buffer_id().unwrap();
    app.buffers.retain(|buffer| buffer.id != focused);

    toggle_visible_whitespace(&mut app);

    assert_eq!(
        app.status_message.as_deref(),
        Some("Whitespace failed: focused buffer is missing")
    );
}

#[test]
fn visible_whitespace_lifecycle_matches_other_per_buffer_view_state() {
    let reload_path = temp_file_path("visible-whitespace-reload.txt");
    std::fs::write(&reload_path, "before").unwrap();
    let mut reload_app = AppState::from_path(Some(reload_path.clone())).unwrap();
    toggle_visible_whitespace(&mut reload_app);
    std::fs::write(&reload_path, "after").unwrap();

    reload_app.handle_command(&EditorCommand::File(FileCommand::Reload));

    let reloaded = reload_app.focused_buffer().unwrap();
    assert_eq!(reloaded.buffer.to_text(), "after");
    assert!(reloaded.visible_whitespace, "reload preserves view state");

    let mut new_app = AppState::new();
    toggle_visible_whitespace(&mut new_app);
    new_app.handle_command(&EditorCommand::File(FileCommand::New));
    assert!(
        !new_app.focused_buffer().unwrap().visible_whitespace,
        "New starts with the toggle off"
    );

    let open_path = temp_file_path("visible-whitespace-open.txt");
    std::fs::write(&open_path, "opened").unwrap();
    let mut open_app = AppState::new();
    toggle_visible_whitespace(&mut open_app);
    open_app.open_file_path(open_path.clone()).unwrap();
    assert!(
        !open_app.focused_buffer().unwrap().visible_whitespace,
        "Open starts with the toggle off"
    );

    let mut close_app = AppState::new();
    close_app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    let closing_buffer_id = close_app.focused_buffer_id().unwrap();
    toggle_visible_whitespace(&mut close_app);
    close_app.handle_command(&EditorCommand::Window(WindowCommand::Close));
    assert!(
        close_app.buffer_state(closing_buffer_id).is_none(),
        "final close drops the BufferState and its view flag"
    );

    let _ = std::fs::remove_file(reload_path);
    let _ = std::fs::remove_file(open_path);
}

#[test]
fn toggle_preserves_source_cursor_and_renormalizes_the_wide_viewport() {
    let mut config = Config::default();
    config.terminal.ambiguous_width = Some(AmbiguousWidth::Wide);
    let mut app = AppState::from_config(config);
    let state = app.focused_buffer_mut().unwrap();
    state.buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "    x");
    state.buffer.set_cursor(Position::new(0, 5)).unwrap();
    let area = Rect::new(0, 0, 8, 3);
    app.sync_view_for_area(area);
    assert_eq!(app.focused_buffer().unwrap().first_column, 2);

    toggle_visible_whitespace(&mut app);

    let state = app.focused_buffer().unwrap();
    assert_eq!(state.buffer.cursor_position(), Position::new(0, 5));
    assert_eq!(state.first_column, 6);
}

#[test]
fn detail_status_places_whitespace_at_index_four_only_when_enabled() {
    let mut app = app_with_text("one");
    assert!(
        !app.focused_detail_status().contains("[Whitespace]"),
        "default-off status must have no marker bracket"
    );

    let state = app.focused_buffer_mut().unwrap();
    state.word_wrap = true;
    state.visible_whitespace = true;
    let detail = app.focused_detail_status();
    let position = detail.find("1:1").expect("cursor position");
    let whitespace = detail
        .find("[Whitespace]")
        .expect("[Whitespace] bracket should be present");
    let wrap = detail.find("[Wrap]").expect("[Wrap] bracket");
    let view = detail.find("[View ").expect("[View] bracket");

    assert!(
        position < whitespace && whitespace < wrap && wrap < view,
        "expected index-4 ordering after the cursor: {detail}"
    );
}

#[test]
fn command_alias_binding_and_completion_dispatch_visible_whitespace() {
    let mut command_app = AppState::new();
    submit_command_line(&mut command_app, "whitespace");
    assert!(
        command_app.focused_buffer().unwrap().visible_whitespace,
        "the command-line alias must dispatch the toggle"
    );

    let mut key_app = AppState::new();
    handle_key_event(
        &mut key_app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('x'), TerminalKeyModifiers::CONTROL),
    );
    handle_key_event(
        &mut key_app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('.'), TerminalKeyModifiers::NONE),
    );
    assert!(
        key_app.focused_buffer().unwrap().visible_whitespace,
        "Ctrl+X,. must dispatch the toggle"
    );

    assert_eq!(
        command_line_completion("white"),
        CommandCompletion::Unique("whitespace ".to_string())
    );
}

#[test]
fn visible_whitespace_status_uses_the_shipped_chinese_catalog() {
    let catalog =
        dun_config::parse_catalog(include_str!("../../../../i18n/zh-Hans.conf"), "zh-Hans")
            .expect("shipped file parses");
    let mut app = AppState::new();
    app.shell.catalog = catalog;

    toggle_visible_whitespace(&mut app);

    assert_eq!(app.status_message.as_deref(), Some("空白字符显示已开启"));
}

#[test]
fn bookmark_toggle_adds_removes_sorts_deduplicates_and_stays_per_buffer() {
    let mut app = app_with_text("zero\none\ntwo\nthree");
    let first_buffer_id = app.focused_buffer_id().unwrap();
    let state = app.focused_buffer_mut().unwrap();
    state.bookmarks = vec![3, 1, 1];
    state.buffer.set_cursor(Position::new(2, 0)).unwrap();

    toggle_bookmark(&mut app);

    assert_eq!(app.focused_buffer().unwrap().bookmarks, [1, 2, 3]);
    assert_eq!(app.status_message.as_deref(), Some("Bookmarked line 3"));
    assert!(
        !app.focused_buffer().unwrap().buffer.is_dirty(),
        "bookmark view state must not dirty buffer text"
    );

    toggle_bookmark(&mut app);
    assert_eq!(app.focused_buffer().unwrap().bookmarks, [1, 3]);
    assert_eq!(
        app.status_message.as_deref(),
        Some("Removed bookmark at line 3")
    );

    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    let second_buffer_id = app.focused_buffer_id().unwrap();
    assert_ne!(first_buffer_id, second_buffer_id);
    toggle_bookmark(&mut app);

    assert_eq!(app.buffer_state(second_buffer_id).unwrap().bookmarks, [0]);
    assert_eq!(
        app.buffer_state(first_buffer_id).unwrap().bookmarks,
        [1, 3],
        "a second buffer must not inherit or mutate the first buffer's bookmarks"
    );
}

#[test]
fn bookmark_toggle_reports_a_missing_focused_buffer() {
    let mut app = AppState::new();
    let focused = app.focused_buffer_id().unwrap();
    app.buffers.retain(|buffer| buffer.id != focused);

    toggle_bookmark(&mut app);

    assert_eq!(
        app.status_message.as_deref(),
        Some("Bookmark failed: focused buffer is missing")
    );
}

#[test]
fn bookmark_navigation_is_strict_circular_and_clamps_columns() {
    let mut app = app_with_text("zero\nabcdef\nmiddle\nx\nlast");
    app.focused_buffer_mut().unwrap().bookmarks = vec![1, 3];

    app.focused_buffer_mut()
        .unwrap()
        .buffer
        .set_cursor(Position::new(1, 5))
        .unwrap();
    app.handle_command(&EditorCommand::Edit(EditCommand::NextBookmark));
    assert_eq!(
        app.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(3, 1),
        "next from an exact bookmark must advance, clamping on a shorter line"
    );

    app.focused_buffer_mut()
        .unwrap()
        .buffer
        .set_cursor(Position::new(3, 1))
        .unwrap();
    app.handle_command(&EditorCommand::Edit(EditCommand::PreviousBookmark));
    assert_eq!(
        app.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(1, 1),
        "previous from an exact bookmark must retreat"
    );

    app.focused_buffer_mut()
        .unwrap()
        .buffer
        .set_cursor(Position::new(0, 3))
        .unwrap();
    app.handle_command(&EditorCommand::Edit(EditCommand::NextBookmark));
    assert_eq!(
        app.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(1, 3),
        "navigation retains the current byte column when it fits"
    );

    app.focused_buffer_mut()
        .unwrap()
        .buffer
        .set_cursor(Position::new(4, 2))
        .unwrap();
    app.handle_command(&EditorCommand::Edit(EditCommand::NextBookmark));
    assert_eq!(
        app.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(1, 2),
        "next after the final bookmark wraps to the first"
    );

    app.focused_buffer_mut()
        .unwrap()
        .buffer
        .set_cursor(Position::new(0, 2))
        .unwrap();
    app.handle_command(&EditorCommand::Edit(EditCommand::PreviousBookmark));
    assert_eq!(
        app.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(3, 1),
        "previous before the first bookmark wraps to the last"
    );

    app.focused_buffer_mut().unwrap().bookmarks = vec![2];
    app.focused_buffer_mut()
        .unwrap()
        .buffer
        .set_cursor(Position::new(2, 3))
        .unwrap();
    app.handle_command(&EditorCommand::Edit(EditCommand::NextBookmark));
    assert_eq!(
        app.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(2, 3),
        "a single bookmark wraps to itself"
    );
    app.handle_command(&EditorCommand::Edit(EditCommand::PreviousBookmark));
    assert_eq!(
        app.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(2, 3)
    );
    assert_eq!(app.status_message.as_deref(), Some("Bookmark: line 3"));
}

#[test]
fn bookmark_navigation_reports_empty_and_ensures_the_target_is_visible() {
    let text = (0..30)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = app_with_text(&text);

    app.handle_command(&EditorCommand::Edit(EditCommand::NextBookmark));
    assert_eq!(app.status_message.as_deref(), Some("Bookmark: none set"));

    app.focused_buffer_mut().unwrap().bookmarks = vec![29];
    app.sync_view_for_area(Rect::new(0, 0, 20, 6));
    app.handle_command(&EditorCommand::Edit(EditCommand::NextBookmark));

    let state = app.focused_buffer().unwrap();
    assert_eq!(state.buffer.cursor_position(), Position::new(29, 0));
    assert!(
        state.first_line > 0,
        "the wrapped target must be brought into the viewport"
    );
}

#[test]
fn bookmark_reload_clamps_and_deduplicates_and_new_open_start_empty() {
    let reload_path = temp_file_path("bookmark-reload.txt");
    std::fs::write(&reload_path, "zero\none\ntwo\nthree\nfour").unwrap();
    let mut reload_app = AppState::from_path(Some(reload_path.clone())).unwrap();
    for line in [1, 3, 4] {
        reload_app
            .focused_buffer_mut()
            .unwrap()
            .buffer
            .set_cursor(Position::new(line, 0))
            .unwrap();
        toggle_bookmark(&mut reload_app);
    }
    std::fs::write(&reload_path, "short\nx").unwrap();

    reload_app.handle_command(&EditorCommand::File(FileCommand::Reload));

    assert_eq!(reload_app.focused_buffer().unwrap().bookmarks, [1]);
    assert_eq!(
        reload_app.focused_buffer().unwrap().buffer.to_text(),
        "short\nx"
    );

    reload_app.handle_command(&EditorCommand::File(FileCommand::New));
    assert!(
        reload_app.focused_buffer().unwrap().bookmarks.is_empty(),
        "New starts with no bookmarks"
    );

    let open_path = temp_file_path("bookmark-open.txt");
    std::fs::write(&open_path, "opened").unwrap();
    toggle_bookmark(&mut reload_app);
    reload_app.open_file_path(open_path.clone()).unwrap();
    assert!(
        reload_app.focused_buffer().unwrap().bookmarks.is_empty(),
        "Open starts with no bookmarks"
    );

    let _ = std::fs::remove_file(reload_path);
    let _ = std::fs::remove_file(open_path);
}

#[test]
fn delete_line_normalizes_bookmarks() {
    let mut app = app_with_text("a\nb\nc");
    let state = app.focused_buffer_mut().unwrap();
    state.bookmarks = vec![0, 2];
    state.buffer.set_cursor(Position::new(0, 0)).unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::DeleteLine));

    assert_eq!(app.focused_buffer().unwrap().buffer.to_text(), "b\nc");
    assert_eq!(app.focused_buffer().unwrap().bookmarks, [0, 1]);
}

#[test]
fn move_line_swaps_source_and_destination_bookmarks() {
    let mut app = app_with_text("a\nb\nc");
    let state = app.focused_buffer_mut().unwrap();
    state.bookmarks = vec![0, 2];
    state.buffer.set_cursor(Position::new(1, 0)).unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineUp));

    assert_eq!(app.focused_buffer().unwrap().buffer.to_text(), "b\na\nc");
    assert_eq!(app.focused_buffer().unwrap().bookmarks, [1, 2]);

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineDown));
    assert_eq!(app.focused_buffer().unwrap().buffer.to_text(), "a\nb\nc");
    assert_eq!(app.focused_buffer().unwrap().bookmarks, [0, 2]);
}

#[test]
fn shared_buffer_views_retain_bookmarks_until_the_final_view_closes() {
    let mut app = app_with_text("shared");
    let shared_buffer_id = app.focused_buffer_id().unwrap();
    let original_window_id = app.workspace.focused;
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    let redundant_buffer_id = app.focused_buffer_id().unwrap();
    let shared_window_id = app.workspace.focused;
    app.workspace
        .window_mut(shared_window_id)
        .unwrap()
        .buffer_id = shared_buffer_id;
    app.drop_buffer_if_unreferenced(redundant_buffer_id);

    toggle_bookmark(&mut app);
    assert_eq!(app.buffer_state(shared_buffer_id).unwrap().bookmarks, [0]);

    app.handle_command(&EditorCommand::Window(WindowCommand::Close));
    assert_eq!(
        app.buffer_state(shared_buffer_id).unwrap().bookmarks,
        [0],
        "closing one shared view must retain the shared BufferState"
    );

    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    app.workspace.focused = original_window_id;
    app.handle_command(&EditorCommand::Window(WindowCommand::Close));
    assert!(
        app.buffer_state(shared_buffer_id).is_none(),
        "closing the final view must reap the BufferState and its bookmarks"
    );
}

#[test]
fn detail_status_places_mark_before_whitespace_and_wrap_only_on_its_line() {
    let mut app = app_with_text("one\ntwo");
    assert!(!app.focused_detail_status().contains("[Mark]"));

    let state = app.focused_buffer_mut().unwrap();
    state.word_wrap = true;
    state.visible_whitespace = true;
    state.bookmarks = vec![0];
    let detail = app.focused_detail_status();
    let position = detail.find("1:1").expect("cursor position");
    let mark = detail.find("[Mark]").expect("[Mark] bracket");
    let whitespace = detail.find("[Whitespace]").expect("[Whitespace] bracket");
    let wrap = detail.find("[Wrap]").expect("[Wrap] bracket");
    let view = detail.find("[View ").expect("[View] bracket");

    assert!(
        position < mark && mark < whitespace && whitespace < wrap && wrap < view,
        "expected marker ordering after the cursor: {detail}"
    );

    app.focused_buffer_mut()
        .unwrap()
        .buffer
        .set_cursor(Position::new(1, 0))
        .unwrap();
    assert!(
        !app.focused_detail_status().contains("[Mark]"),
        "[Mark] must disappear off the bookmarked line"
    );
}

#[test]
fn bookmark_aliases_completion_and_ctrl_x_bindings_dispatch() {
    let mut command_app = AppState::new();
    submit_command_line(&mut command_app, "mark");
    assert_eq!(command_app.focused_buffer().unwrap().bookmarks, [0]);
    submit_command_line(&mut command_app, "bookmark");
    assert!(command_app.focused_buffer().unwrap().bookmarks.is_empty());
    assert_eq!(
        command_line_completion("mar"),
        CommandCompletion::Unique("mark ".to_string())
    );
    assert_eq!(command_line_completion("book"), CommandCompletion::None);

    let mut key_app = app_with_text("zero\none\ntwo");
    send_ctrl_x_command(&mut key_app, 'k');
    assert_eq!(key_app.focused_buffer().unwrap().bookmarks, [0]);

    key_app.focused_buffer_mut().unwrap().bookmarks = vec![0, 2];
    send_ctrl_x_command(&mut key_app, 'n');
    assert_eq!(
        key_app.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(2, 0)
    );
    send_ctrl_x_command(&mut key_app, 'l');
    assert_eq!(
        key_app.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(0, 0)
    );
}

#[test]
fn bookmark_status_uses_the_shipped_chinese_catalog() {
    let catalog =
        dun_config::parse_catalog(include_str!("../../../../i18n/zh-Hans.conf"), "zh-Hans")
            .expect("shipped file parses");
    let mut app = AppState::new();
    app.shell.catalog = catalog;

    toggle_bookmark(&mut app);

    assert_eq!(app.status_message.as_deref(), Some("已为第 1 行添加书签"));
}
