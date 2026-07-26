use super::support::*;

fn toggle_visible_whitespace(app: &mut AppState) {
    app.handle_command(&EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace));
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
