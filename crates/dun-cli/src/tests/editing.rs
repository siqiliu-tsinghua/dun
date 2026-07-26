#![allow(unused_imports)]

use super::support::*;

fn wide_app_with_text(text: &str) -> AppState {
    let mut config = Config::default();
    config.terminal.ambiguous_width = Some(AmbiguousWidth::Wide);
    let mut app = AppState::from_config(config);
    app.buffers[0].buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, text);
    app
}

#[test]
fn text_input_inserts_into_focused_buffer() {
    let mut app = AppState::new();

    app.handle_text_input('a');
    app.handle_text_input('é');

    let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
    assert_eq!(buffer.line(0), Some("aé"));
    assert_eq!(buffer.cursor_position(), Position::new(0, 3));
}

#[test]
fn edit_commands_apply_to_focused_buffer() {
    let mut app = AppState::new();
    app.handle_text_input('a');
    app.handle_text_input('b');
    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLeft));
    app.handle_text_input('x');
    app.handle_command(&EditorCommand::Edit(EditCommand::DeleteForward));
    app.handle_command(&EditorCommand::Edit(EditCommand::InsertNewline));
    app.handle_text_input('z');

    let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
    assert_eq!(buffer.line(0), Some("ax"));
    assert_eq!(buffer.line(1), Some("z"));
    assert_eq!(buffer.cursor_position(), Position::new(1, 1));
}

#[test]
fn line_edit_commands_apply_to_focused_buffer() {
    let mut app = app_with_text("one  \ntwo\nthree   ");
    app.handle_command(&EditorCommand::Edit(EditCommand::MoveDown));

    app.handle_command(&EditorCommand::Edit(EditCommand::CopyLine));
    assert_eq!(app.kill_ring.as_deref(), Some("two\n"));

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineUp));
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "two\none  \nthree   "
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::IndentLine));
    app.handle_command(&EditorCommand::Edit(EditCommand::OutdentLine));
    app.handle_command(&EditorCommand::Edit(EditCommand::TrimTrailingWhitespace));
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "two\none\nthree"
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::DeleteLine));
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "one\nthree"
    );
}

#[test]
fn line_edit_commands_report_edge_statuses() {
    let mut app = app_with_text("one");

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineUp));
    assert_eq!(
        app.status_message,
        Some("Move line: already at top".to_string())
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineDown));
    assert_eq!(
        app.status_message,
        Some("Move line: already at bottom".to_string())
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::OutdentLine));
    assert_eq!(
        app.status_message,
        Some("Outdent: nothing changed".to_string())
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::TrimTrailingWhitespace));
    assert_eq!(
        app.status_message,
        Some("Trim: no trailing whitespace".to_string())
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::DeleteLine));
    assert_eq!(app.status_message, Some("Deleted line".to_string()));
    app.handle_command(&EditorCommand::Edit(EditCommand::DeleteLine));
    assert_eq!(app.status_message, Some("Deleted line".to_string()));
}

#[test]
fn word_wrap_toggle_and_scroll_edges_report_status() {
    let mut app = app_with_text("0123456789abcdef\nsecond");
    app.sync_view_for_area(Rect::new(0, 0, 10, 4));

    app.handle_command(&EditorCommand::Edit(EditCommand::ToggleWordWrap));
    assert_eq!(app.status_message, Some("Word wrap on".to_string()));
    app.handle_command(&EditorCommand::Edit(EditCommand::ToggleWordWrap));
    assert_eq!(app.status_message, Some("Word wrap off".to_string()));

    app.handle_command(&EditorCommand::Edit(EditCommand::ScrollLeft));
    assert_eq!(app.status_message, Some("Already at left edge".to_string()));

    for _ in 0..10 {
        app.handle_command(&EditorCommand::Edit(EditCommand::ScrollRight));
    }
    assert_eq!(
        app.status_message,
        Some("Already at right edge".to_string())
    );
}

#[test]
fn editor_page_commands_move_cursor_by_visible_page() {
    let text = (0..20)
        .map(|index| format!("line{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = AppState::new();
    app.buffer_state_mut(BufferId(1)).unwrap().buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
    app.sync_view_for_area(Rect::new(0, 0, 80, 6));

    app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .cursor_position(),
        Position::new(3, 0)
    );
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().first_line, 0);

    app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .cursor_position(),
        Position::new(6, 0)
    );
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().first_line, 3);

    app.handle_command(&EditorCommand::Edit(EditCommand::MovePageUp));
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .cursor_position(),
        Position::new(3, 0)
    );
}

#[test]
fn shift_page_commands_extend_selection_by_visible_page() {
    let text = (0..20)
        .map(|index| format!("line{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = AppState::new();
    app.buffer_state_mut(BufferId(1)).unwrap().buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
    app.sync_view_for_area(Rect::new(0, 0, 80, 6));

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::PageDown, TerminalKeyModifiers::SHIFT),
    );

    let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
    assert_eq!(buffer.cursor_position(), Position::new(3, 0));
    assert_eq!(
        buffer.selection(),
        Some(dun_core::Selection::new(
            Position::zero(),
            Position::new(3, 0)
        ))
    );
}

#[test]
fn wrapped_page_commands_move_cursor_by_visual_rows() {
    let mut app = app_with_text("abcdefghijklmnop");
    app.buffer_state_mut(BufferId(1)).unwrap().word_wrap = true;
    app.sync_view_for_area(Rect::new(0, 0, 12, 3));

    app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.cursor_position(), Position::new(0, 8));
    assert_eq!(state.first_line, 0);

    app.handle_command(&EditorCommand::Edit(EditCommand::MovePageUp));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.cursor_position(), Position::zero());
    assert_eq!(state.first_visual_row, 0);
}

#[test]
fn wrapped_shift_page_commands_extend_selection_by_visual_rows() {
    let mut app = app_with_text("abcdefghijklmnop");
    app.buffer_state_mut(BufferId(1)).unwrap().word_wrap = true;
    app.sync_view_for_area(Rect::new(0, 0, 12, 3));

    app.handle_command(&EditorCommand::Edit(EditCommand::ExtendSelectionPageDown));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.cursor_position(), Position::new(0, 8));
    assert_eq!(
        state.buffer.selection(),
        Some(dun_core::Selection::new(
            Position::zero(),
            Position::new(0, 8)
        ))
    );
    assert_eq!(
        state
            .buffer
            .text_in_range(state.buffer.selection_range().unwrap())
            .unwrap(),
        "abcdefgh"
    );
}

#[test]
fn wrapped_page_commands_preserve_visual_column_across_wide_chars() {
    let mut app = app_with_text("界abcdefghi");
    let state = app.buffer_state_mut(BufferId(1)).unwrap();
    state.word_wrap = true;
    state
        .buffer
        .set_cursor(Position::new(0, "界a".len()))
        .unwrap();
    app.sync_view_for_area(Rect::new(0, 0, 10, 3));

    app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));

    let state = app.buffer_state(BufferId(1)).unwrap();
    let cursor = state.buffer.cursor_position();
    assert_eq!(
        state.buffer.line(0).unwrap().get(..cursor.column),
        Some("界abcdefg")
    );
}

#[test]
fn wrapped_page_commands_preserve_visual_column_across_tab_and_control() {
    let mut app = app_with_text("a\tbcdefgh\na\u{1}bcdefgh");
    let state = app.buffer_state_mut(BufferId(1)).unwrap();
    state.word_wrap = true;
    state
        .buffer
        .set_cursor(Position::new(0, "a\t".len()))
        .unwrap();
    app.sync_view_for_area(Rect::new(0, 0, 8, 3));

    app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));

    {
        let state = app.buffer_state(BufferId(1)).unwrap();
        let cursor = state.buffer.cursor_position();
        assert_eq!(
            state.buffer.line(0).unwrap().get(..cursor.column),
            Some("a\tbcde")
        );
    }

    app.buffer_state_mut(BufferId(1))
        .unwrap()
        .buffer
        .set_cursor(Position::new(1, "a\u{1}".len()))
        .unwrap();
    app.handle_command(&EditorCommand::Edit(EditCommand::MovePageDown));

    let state = app.buffer_state(BufferId(1)).unwrap();
    let cursor = state.buffer.cursor_position();
    assert_eq!(
        state.buffer.line(1).unwrap().get(..cursor.column),
        Some("a\u{1}bcde")
    );
}

#[test]
fn wide_sync_view_uses_rendered_body_width() {
    let mut app = wide_app_with_text("one line");
    let area = Rect::new(0, 0, 80, 8);

    app.sync_view_for_area(area);

    let context = app.focused_buffer_view_context(area).unwrap();
    assert_eq!(context.body_width, 73);
}

#[test]
fn wide_wrapping_counts_ambiguous_glyphs_as_two_columns() {
    let app = wide_app_with_text("◆◆");
    let state = app.buffer_state(BufferId(1)).unwrap();

    assert_eq!(
        state.wrapped_line_visual_rows(0, 3, AmbiguousWidth::Narrow),
        1
    );
    assert_eq!(
        state.wrapped_line_visual_rows(0, 3, AmbiguousWidth::Wide),
        2
    );
}

#[test]
fn wide_horizontal_scroll_keeps_cursor_inside_physical_body() {
    let mut app = wide_app_with_text(&"◆".repeat(40));
    let area = Rect::new(0, 0, 80, 8);
    app.sync_view_for_area(area);

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
    app.sync_view_for_area(area);

    let context = app.focused_buffer_view_context(area).unwrap();
    let state = app.buffer_state(BufferId(1)).unwrap();
    let cursor_column = state.cursor_display_column(AmbiguousWidth::Wide);
    assert_eq!(context.body_width, 73);
    assert_eq!(cursor_column, 80);
    assert_eq!(state.first_column, 8);
    assert_eq!(cursor_column - state.first_column, 72);
}

#[test]
fn horizontal_scroll_keeps_cursor_visible_and_reports_offset() {
    let mut app = app_with_text("0123456789abcdef");
    app.sync_view_for_area(Rect::new(0, 0, 10, 4));

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
    app.sync_view_for_area(Rect::new(0, 0, 10, 4));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert!(state.first_column > 0);
    assert!(app.focused_detail_status().contains(" X"));

    let buffer_views = app.buffer_views();
    let frame = app
        .shell
        .frame_for_workspace(&app.workspace, app.workspace_area, &buffer_views);
    assert!(
        frame.windows[0]
            .body
            .first()
            .is_some_and(|line| line.as_plain_text().ends_with("bcdef"))
    );
}

#[test]
fn horizontal_scroll_commands_move_viewport_without_moving_cursor() {
    let mut app = app_with_text("0123456789abcdef");
    app.sync_view_for_area(Rect::new(0, 0, 10, 4));

    app.handle_command(&EditorCommand::Edit(EditCommand::ScrollRight));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.first_column, 3);
    assert_eq!(state.buffer.cursor_position(), Position::zero());
    assert_eq!(
        app.status_message,
        Some("Scrolled right to column 4".to_string())
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::ScrollLeft));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.first_column, 0);
    assert_eq!(
        app.status_message,
        Some("Scrolled left to column 1".to_string())
    );
}

#[test]
fn undo_redo_commands_report_status() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::Edit(EditCommand::Undo));
    assert_eq!(app.status_message, Some("Nothing to undo".to_string()));

    app.handle_text_input('x');
    app.handle_command(&EditorCommand::Edit(EditCommand::Undo));
    assert_eq!(app.status_message, Some("Undo".to_string()));

    app.handle_command(&EditorCommand::Edit(EditCommand::Redo));
    assert_eq!(app.status_message, Some("Redo".to_string()));

    app.handle_command(&EditorCommand::Edit(EditCommand::Redo));
    assert_eq!(app.status_message, Some("Nothing to redo".to_string()));
}

#[test]
fn word_edit_commands_apply_to_focused_buffer() {
    let mut app = AppState::new();
    app.buffer_state_mut(BufferId(1)).unwrap().buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, "alpha beta");

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveWordRight));
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .cursor_position(),
        Position::new(0, 6)
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::ExtendSelectionWordRight));
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.selection(),
        Some(dun_core::Selection::new(
            Position::new(0, 6),
            Position::new(0, 10)
        ))
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::DeleteWordBackward));
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "alpha "
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::DeleteWordBackward));
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");
}

#[test]
fn shift_arrow_keys_extend_selection_in_editor_buffer() {
    let mut app = app_with_text("abcd");
    app.buffers[0]
        .buffer
        .set_cursor(Position::new(0, 1))
        .unwrap();

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Right, TerminalKeyModifiers::SHIFT),
    );
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .selection_range(),
        Some(TextRange::new(Position::new(0, 1), Position::new(0, 2)))
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Right, TerminalKeyModifiers::SHIFT),
    );
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .selection_range(),
        Some(TextRange::new(Position::new(0, 1), Position::new(0, 3)))
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Left, TerminalKeyModifiers::SHIFT),
    );
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .selection_range(),
        Some(TextRange::new(Position::new(0, 1), Position::new(0, 2)))
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Left, TerminalKeyModifiers::NONE),
    );
    let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
    assert_eq!(buffer.selection_range(), None);
    assert_eq!(buffer.cursor_position(), Position::new(0, 1));
}

#[test]
fn shift_home_end_extend_selection_to_line_edges() {
    let mut app = app_with_text("abc\ndef");
    app.buffers[0]
        .buffer
        .set_cursor(Position::new(1, 1))
        .unwrap();

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::End, TerminalKeyModifiers::SHIFT),
    );
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .selection_range(),
        Some(TextRange::new(Position::new(1, 1), Position::new(1, 3)))
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Home, TerminalKeyModifiers::SHIFT),
    );
    let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
    assert_eq!(
        buffer.selection_range(),
        Some(TextRange::new(Position::new(1, 0), Position::new(1, 1)))
    );
    assert_eq!(buffer.cursor_position(), Position::new(1, 0));
}

#[test]
fn select_line_command_selects_current_line() {
    let mut app = app_with_text("first\nsecond\nthird");
    app.buffers[0]
        .buffer
        .set_cursor(Position::new(1, 2))
        .unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::SelectLine));

    let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
    assert_eq!(
        buffer.selection_range(),
        Some(TextRange::new(Position::new(1, 0), Position::new(2, 0)))
    );
    assert_eq!(
        buffer
            .text_in_range(buffer.selection_range().unwrap())
            .unwrap(),
        "second\n"
    );
}

#[test]
fn multi_stroke_key_sequence_applies_command() {
    let mut app = AppState::new();
    app.sync_view_for_area(Rect::new(0, 0, 80, 20));

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('x'), TerminalKeyModifiers::CONTROL),
    );

    assert_eq!(app.workspace.window_count(), 1);

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('h'), TerminalKeyModifiers::NONE),
    );

    assert_eq!(app.workspace.window_count(), 2);
}

#[test]
fn invalid_pending_key_sequence_does_not_insert_text() {
    let mut app = AppState::new();

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('x'), TerminalKeyModifiers::CONTROL),
    );
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('m'), TerminalKeyModifiers::NONE),
    );

    let buffer = &app.buffer_state(BufferId(1)).unwrap().buffer;
    assert_eq!(buffer.line(0), Some(""));
}

#[test]
fn copy_selection_pastes_internal_clipboard_without_mutating_source() {
    let mut app = app_with_text("abc def");
    app.buffers[0]
        .buffer
        .select(Position::new(0, 0), Position::new(0, 3))
        .unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::Copy));

    assert_eq!(app.kill_ring.as_deref(), Some("abc"));
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "abc def"
    );
    assert_eq!(app.status_message, Some("Copied selection".to_string()));

    app.buffers[0]
        .buffer
        .set_cursor(Position::new(0, "abc def".len()))
        .unwrap();
    app.handle_command(&EditorCommand::Edit(EditCommand::Paste));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "abc defabc");
    assert_eq!(state.buffer.selection_range(), None);
    assert_eq!(app.status_message, Some("Pasted selection".to_string()));
}

#[test]
fn copy_external_requires_opt_in_and_preserves_internal_clipboard() {
    let mut app = app_with_text("abc def");
    app.buffers[0]
        .buffer
        .select(Position::new(0, 0), Position::new(0, 3))
        .unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::CopyExternal));

    assert_eq!(app.kill_ring.as_deref(), Some("abc"));
    assert_eq!(app.take_runtime_action(), None);
    assert_eq!(
        app.status_message,
        Some("External copy disabled: copied selection internally".to_string())
    );
}

#[test]
fn copy_external_emits_osc52_when_enabled_and_under_limit() {
    let mut config = Config::default();
    config.clipboard.osc52.enabled = true;
    config.clipboard.osc52.max_bytes = 8;
    let mut app = AppState::from_config(config);
    app.buffers[0].buffer.insert_str("abc").unwrap();
    app.buffers[0]
        .buffer
        .select(Position::new(0, 0), Position::new(0, 3))
        .unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::CopyExternal));

    assert_eq!(app.kill_ring.as_deref(), Some("abc"));
    assert_eq!(
        app.take_runtime_action(),
        Some(RuntimeAction::WriteTerminal(
            "\x1b]52;c;YWJj\x07".to_string()
        ))
    );
    assert_eq!(
        app.status_message,
        Some("Copied selection to external clipboard".to_string())
    );
}

#[test]
fn copy_external_honors_osc52_byte_limit() {
    let mut config = Config::default();
    config.clipboard.osc52.enabled = true;
    config.clipboard.osc52.max_bytes = 2;
    let mut app = AppState::from_config(config);
    app.buffers[0].buffer.insert_str("abc").unwrap();
    app.buffers[0]
        .buffer
        .select(Position::new(0, 0), Position::new(0, 3))
        .unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::CopyExternal));

    assert_eq!(app.kill_ring.as_deref(), Some("abc"));
    assert_eq!(app.take_runtime_action(), None);
    assert_eq!(
        app.status_message,
        Some("External copy failed: selection is 3 bytes; limit is 2".to_string())
    );
}

#[test]
fn paste_external_disabled_falls_back_without_querying_even_when_write_is_enabled() {
    let mut config = Config::default();
    config.clipboard.osc52.enabled = true;
    let mut app = AppState::from_config(config);
    app.buffers[0].buffer.insert_str("abc").unwrap();
    app.buffers[0]
        .buffer
        .set_cursor(Position::new(0, 3))
        .unwrap();
    app.kill_ring = Some(" internal".to_string());

    app.handle_command(&EditorCommand::Edit(EditCommand::PasteExternal));

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "abc internal"
    );
    assert_eq!(app.take_runtime_action(), None);
    assert_eq!(
        app.status_message,
        Some("External paste disabled; pasted internal clipboard".to_string())
    );
}

#[test]
fn paste_external_enabled_requests_typed_query_with_configured_limit() {
    let mut config = Config::default();
    config.clipboard.osc52.allow_read = true;
    config.clipboard.osc52.max_bytes = 37;
    let mut app = AppState::from_config(config);

    app.handle_command(&EditorCommand::Edit(EditCommand::PasteExternal));

    assert_eq!(
        app.take_runtime_action(),
        Some(RuntimeAction::QueryOsc52Clipboard { max_bytes: 37 })
    );
    assert_eq!(
        app.status_message,
        Some("External paste: waiting for terminal clipboard".to_string())
    );
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");
}

#[test]
fn external_paste_response_replaces_selection_and_keeps_controls_as_sanitized_content() {
    let mut app = app_with_text("abc");
    app.buffers[0]
        .buffer
        .select(Position::new(0, 1), Position::new(0, 2))
        .unwrap();

    app.complete_external_paste("X\x1b]0;owned\x07".to_string());

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "aX\x1b]0;owned\x07c"
    );
    assert_eq!(
        app.status_message,
        Some("Pasted terminal clipboard".to_string())
    );

    let buffer_views = app.buffer_views();
    let frame =
        app.shell
            .frame_for_workspace(&app.workspace, Rect::new(0, 0, 80, 10), &buffer_views);
    assert_eq!(frame.windows[0].body[0].as_plain_text(), "aX␛]0;owned␇c");
}

#[test]
fn external_paste_invalid_utf8_uses_the_decoder_escaped_text() {
    let mut app = AppState::new();
    let decoded = dun_core::decode_file_text(vec![b'o', b'k', 0xff]).text;
    assert_eq!(decoded, "ok\\xFF");

    app.complete_external_paste(decoded);

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "ok\\xFF"
    );
}

#[test]
fn empty_external_paste_does_not_use_stale_internal_clipboard() {
    let mut app = app_with_text("unchanged");
    app.kill_ring = Some("stale".to_string());

    app.complete_external_paste(String::new());

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "unchanged"
    );
    assert_eq!(
        app.status_message,
        Some("Terminal clipboard is empty".to_string())
    );
}

#[test]
fn external_paste_timeout_uses_internal_clipboard_exactly_once() {
    let mut app = app_with_text("replace me");
    app.kill_ring = Some("fallback".to_string());
    app.buffers[0]
        .buffer
        .select(Position::new(0, 0), Position::new(0, 10))
        .unwrap();

    app.complete_external_paste_timeout();

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "fallback"
    );
    assert_eq!(
        app.status_message,
        Some("Terminal clipboard unavailable; pasted internal clipboard".to_string())
    );
}

#[test]
fn external_paste_timeout_reports_empty_internal_clipboard() {
    let mut app = app_with_text("unchanged");

    app.complete_external_paste_timeout();

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "unchanged"
    );
    assert_eq!(
        app.status_message,
        Some("Terminal clipboard unavailable; internal clipboard empty".to_string())
    );
}

#[test]
fn external_paste_preserves_normal_read_only_failure_status() {
    let mut app = AppState::new();
    app.handle_command(&EditorCommand::App(AppCommand::Help));
    let buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    let before = app.buffer_state(buffer_id).unwrap().buffer.to_text();

    app.complete_external_paste("x".to_string());

    assert_eq!(
        app.buffer_state(buffer_id).unwrap().buffer.to_text(),
        before
    );
    assert_eq!(
        app.status_message,
        Some("Paste failed: buffer is read-only".to_string())
    );
}

#[test]
fn cut_selection_removes_text_and_preserves_internal_clipboard() {
    let mut app = app_with_text("one two");
    app.buffers[0]
        .buffer
        .select(Position::new(0, 4), Position::new(0, 7))
        .unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::Cut));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(app.kill_ring.as_deref(), Some("two"));
    assert_eq!(state.buffer.to_text(), "one ");
    assert_eq!(state.buffer.cursor_position(), Position::new(0, 4));
    assert_eq!(state.buffer.selection_range(), None);
    assert_eq!(app.status_message, Some("Cut selection".to_string()));

    app.handle_command(&EditorCommand::Edit(EditCommand::Undo));

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "one two"
    );
    assert_eq!(app.kill_ring.as_deref(), Some("two"));
}

#[test]
fn internal_paste_replaces_active_selection() {
    let mut app = app_with_text("abc");
    app.kill_ring = Some("X".to_string());
    app.buffers[0]
        .buffer
        .select(Position::new(0, 1), Position::new(0, 2))
        .unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::Paste));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "aXc");
    assert_eq!(state.buffer.cursor_position(), Position::new(0, 2));
    assert_eq!(state.buffer.selection_range(), None);
    assert_eq!(app.take_runtime_action(), None);
}

#[test]
fn cut_copy_and_internal_paste_report_empty_or_read_only_states() {
    let mut app = app_with_text("abc");

    app.handle_command(&EditorCommand::Edit(EditCommand::Copy));
    assert_eq!(app.kill_ring, None);
    assert_eq!(app.status_message, Some("Copy: no selection".to_string()));

    app.handle_command(&EditorCommand::Edit(EditCommand::Cut));
    assert_eq!(app.kill_ring, None);
    assert_eq!(app.status_message, Some("Cut: no selection".to_string()));

    app.handle_command(&EditorCommand::Edit(EditCommand::Paste));
    assert_eq!(
        app.status_message,
        Some("Paste: internal clipboard empty; use terminal paste".to_string())
    );

    app.handle_command(&EditorCommand::App(AppCommand::Help));
    let help_buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    app.buffer_state_mut(help_buffer_id)
        .unwrap()
        .buffer
        .select(Position::new(0, 0), Position::new(0, 3))
        .unwrap();
    app.kill_ring = Some("old".to_string());

    app.handle_command(&EditorCommand::Edit(EditCommand::Cut));
    assert_eq!(app.kill_ring.as_deref(), Some("old"));
    assert_eq!(
        app.status_message,
        Some("Cut failed: buffer is read-only".to_string())
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::Paste));
    assert_eq!(
        app.status_message,
        Some("Paste failed: buffer is read-only".to_string())
    );
}

#[test]
fn bracketed_paste_inserts_text_into_editor_buffer() {
    let mut app = AppState::new();

    app.handle_paste("a\r\nb\x1b[31m");

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "a\nb\x1b[31m");
    assert!(state.buffer.is_dirty());
}

#[test]
fn paste_command_reports_empty_internal_clipboard_hint() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::Edit(EditCommand::Paste));

    assert_eq!(
        app.status_message,
        Some("Paste: internal clipboard empty; use terminal paste".to_string())
    );
}

#[test]
fn bracketed_paste_rejects_read_only_focused_buffer() {
    let mut app = AppState::new();
    app.handle_command(&EditorCommand::App(AppCommand::Help));
    let buffer_id = app.workspace.focused_window().unwrap().buffer_id;
    let before = app.buffer_state(buffer_id).unwrap().buffer.to_text();

    app.handle_paste("x");

    assert_eq!(
        app.buffer_state(buffer_id).unwrap().buffer.to_text(),
        before
    );
    assert_eq!(
        app.status_message,
        Some("Paste failed: buffer is read-only".to_string())
    );
}

#[test]
fn bracketed_paste_targets_prompt_and_file_dialog_as_single_line() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    app.handle_paste("theme\r\nmsedit");
    assert_eq!(
        app.prompt_status_text(),
        Some("Command: theme msedit".to_string())
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Esc, TerminalKeyModifiers::NONE),
    );
    app.handle_command(&EditorCommand::File(FileCommand::Open));
    app.handle_paste("a\nb");
    assert_eq!(app.prompt_status_text(), Some("Open: a b".to_string()));
}

#[test]
fn bracketed_paste_is_ignored_during_unsaved_confirmation() {
    let mut app = AppState::new();
    app.handle_text_input('x');
    app.handle_command(&EditorCommand::App(AppCommand::Quit));

    app.handle_paste("y");

    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "x");
    assert!(app.confirm.is_some());
    assert_eq!(
        app.status_message,
        Some("Paste ignored during confirmation".to_string())
    );
}
