#![allow(unused_imports)]

use super::support::*;

#[test]
fn find_command_selects_first_match_from_prompt() {
    let mut app = app_with_text("one two one");

    app.handle_command(&EditorCommand::Edit(EditCommand::Find));
    assert_eq!(app.prompt_status_text(), Some("Find: ".to_string()));

    send_text(&mut app, "one");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(app.last_find_query, Some("one".to_string()));
    assert_eq!(app.status_message, Some("Find: 1/2 one".to_string()));
    assert_eq!(
        state.buffer.selection_range(),
        Some(TextRange::new(Position::new(0, 0), Position::new(0, 3)))
    );
}

#[test]
fn find_prompt_previews_matches_and_cancel_restores_cursor() {
    let mut app = app_with_text("zero one two one");
    app.buffers[0]
        .buffer
        .set_cursor(Position::new(0, 2))
        .unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::Find));
    send_text(&mut app, "one");

    assert_eq!(app.status_message, Some("Find: 1/2 one".to_string()));
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .selection_range(),
        Some(TextRange::new(Position::new(0, 5), Position::new(0, 8)))
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Esc, TerminalKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.cursor_position(), Position::new(0, 2));
    assert_eq!(state.buffer.selection_range(), None);
    assert_eq!(app.status_message, Some("Find cancelled".to_string()));
}

#[test]
fn find_prompt_supports_ignore_case_and_whole_word_flags() {
    let mut app = app_with_text("ERROR errors error_error error");

    app.handle_command(&EditorCommand::Edit(EditCommand::Find));
    send_text(&mut app, "/iw error");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    assert_eq!(
        app.status_message,
        Some("Find: 1/2 error (ignore-case, whole-word)".to_string())
    );
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .selection_range(),
        Some(TextRange::new(Position::new(0, 0), Position::new(0, 5)))
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));

    assert_eq!(
        app.status_message,
        Some("Find: 2/2 error (ignore-case, whole-word)".to_string())
    );
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .selection_range(),
        Some(TextRange::new(Position::new(0, 25), Position::new(0, 30)))
    );
}

#[test]
fn find_next_repeats_query_and_wraps() {
    let mut app = app_with_text("one two one");
    app.last_find_query = Some("one".to_string());

    app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));
    app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(app.status_message, Some("Find: 2/2 one".to_string()));
    assert_eq!(
        state.buffer.selection_range(),
        Some(TextRange::new(Position::new(0, 8), Position::new(0, 11)))
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(
        app.status_message,
        Some("Find: 1/2 one (wrapped)".to_string())
    );
    assert_eq!(
        state.buffer.selection_range(),
        Some(TextRange::new(Position::new(0, 0), Position::new(0, 3)))
    );
}

#[test]
fn find_previous_repeats_query_and_wraps() {
    let mut app = app_with_text("one two one");
    app.last_find_query = Some("one".to_string());

    app.handle_command(&EditorCommand::Edit(EditCommand::FindPrevious));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(
        app.status_message,
        Some("Find: 2/2 one (wrapped)".to_string())
    );
    assert_eq!(
        state.buffer.selection_range(),
        Some(TextRange::new(Position::new(0, 8), Position::new(0, 11)))
    );
}

#[test]
fn find_reports_missing_query_and_missing_match() {
    let mut app = app_with_text("abc");

    app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));
    assert_eq!(app.status_message, Some("Find: no query".to_string()));

    app.last_find_query = Some("z".to_string());
    app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(
        app.status_message,
        Some("Find: no matches for z".to_string())
    );
    assert_eq!(state.buffer.selection_range(), None);
}

#[test]
fn find_preview_reports_missing_match_without_committing_query() {
    let mut app = app_with_text("abc");

    app.commit_find_preview(SearchSpec::parse("z"));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(
        app.status_message,
        Some("Find: no matches for z".to_string())
    );
    assert_eq!(state.search_status(), Some("Find 0".to_string()));
    assert_eq!(state.buffer.selection_range(), None);
}

#[test]
fn replace_command_prompts_and_replaces_next_match() {
    let mut app = app_with_text("one two one");

    app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
    assert_eq!(
        app.prompt_status_text(),
        Some("Find to replace: ".to_string())
    );

    send_text(&mut app, "one");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );
    assert_eq!(app.prompt_status_text(), Some("Replace with: ".to_string()));

    send_text(&mut app, "uno");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );
    assert_eq!(
        app.confirm_status_text(),
        Some("Match 1/2; replaced 0, skipped 0".to_string())
    );
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('r'), TerminalKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "uno two one");
    assert!(state.buffer.is_dirty());
    assert_eq!(app.last_find_query, Some("one".to_string()));
    assert_eq!(
        app.status_message,
        Some("Replace confirm: 1/1 one -> uno".to_string())
    );
    assert_eq!(
        app.confirm_status_text(),
        Some("Match 1/1; replaced 1, skipped 0".to_string())
    );
    assert_eq!(
        state.buffer.selection_range(),
        Some(TextRange::new(Position::new(0, 8), Position::new(0, 11)))
    );
}

#[test]
fn replace_confirmation_can_skip_and_replace_next_match() {
    let mut app = app_with_text("one two one");

    app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
    send_text(&mut app, "one");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );
    send_text(&mut app, "uno");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    assert_eq!(
        app.confirm_status_text(),
        Some("Match 1/2; replaced 0, skipped 0".to_string())
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('s'), TerminalKeyModifiers::NONE),
    );
    assert_eq!(
        app.confirm_status_text(),
        Some("Match 2/2; replaced 0, skipped 1".to_string())
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('r'), TerminalKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "one two uno");
    assert_eq!(
        app.status_message,
        Some("Replace confirm: 1/1 one -> uno".to_string())
    );
    assert_eq!(
        app.confirm_status_text(),
        Some("Match 1/1; replaced 1, skipped 1".to_string())
    );
    assert_eq!(
        state.buffer.selection_range(),
        Some(TextRange::new(Position::new(0, 0), Position::new(0, 3)))
    );
}

#[test]
fn replace_confirmation_all_replaces_remaining_matches() {
    let mut app = app_with_text("one two one");

    app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
    send_text(&mut app, "one");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );
    send_text(&mut app, "uno");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('a'), TerminalKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "uno two uno");
    assert_eq!(app.replace_confirm, None);
    assert_eq!(
        app.status_message,
        Some("Replace All: 2 one -> uno".to_string())
    );
}

#[test]
fn replace_prefers_current_selected_match() {
    let mut app = app_with_text("one two one");
    app.last_find_query = Some("one".to_string());
    app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));
    app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));

    app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
    assert_eq!(
        app.prompt_status_text(),
        Some("Find to replace: one".to_string())
    );
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );
    send_text(&mut app, "uno");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );
    assert_eq!(
        app.confirm_status_text(),
        Some("Match 2/2; replaced 0, skipped 0".to_string())
    );
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('r'), TerminalKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "one two uno");
    assert_eq!(
        app.status_message,
        Some("Replace confirm: 1/1 one -> uno".to_string())
    );
    assert_eq!(
        app.confirm_status_text(),
        Some("Match 1/1; replaced 1, skipped 0".to_string())
    );
    assert_eq!(
        state.buffer.selection_range(),
        Some(TextRange::new(Position::new(0, 0), Position::new(0, 3)))
    );
}

#[test]
fn replace_accepts_empty_replacement_as_delete() {
    let mut app = app_with_text("one two");

    app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
    send_text(&mut app, "one");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );
    assert_eq!(
        app.confirm_status_text(),
        Some("Match 1/1; replaced 0, skipped 0".to_string())
    );
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('r'), TerminalKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), " two");
    assert_eq!(
        app.status_message,
        Some("Replace done: 1 replaced, 0 skipped".to_string())
    );
}

#[test]
fn find_populates_status_field_and_view_highlights() {
    let mut app = app_with_text("one two one");
    app.workspace_area = Rect::new(0, 0, 80, 8);

    app.last_find_query = Some("one".to_string());
    app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));
    app.sync_view_for_area(app.workspace_area);

    assert!(app.focused_detail_status().contains("[Find 1/2]"));
    let buffer_views = app.buffer_views();
    assert_eq!(buffer_views[0].search_matches.len(), 2);
    assert_eq!(buffer_views[0].active_search_match, Some(0));
}

#[test]
fn replace_reports_missing_match() {
    let mut app = app_with_text("abc");

    app.handle_command(&EditorCommand::Edit(EditCommand::Replace));
    send_text(&mut app, "z");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );
    send_text(&mut app, "x");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "abc");
    assert_eq!(
        app.status_message,
        Some("Replace: no matches for z".to_string())
    );
    assert_eq!(state.buffer.selection_range(), None);
}

#[test]
fn direct_replace_commands_report_empty_and_missing_queries() {
    let mut app = app_with_text("abc");

    app.replace_in_focused_buffer(SearchSpec::parse(""), "x");
    assert_eq!(app.status_message, Some("Replace: no query".to_string()));

    app.replace_in_focused_buffer(SearchSpec::parse("z"), "x");
    assert_eq!(
        app.status_message,
        Some("Replace: no matches for z".to_string())
    );

    app.replace_all_in_focused_buffer(SearchSpec::parse(""), "x");
    assert_eq!(
        app.status_message,
        Some("Replace All: no query".to_string())
    );

    app.replace_all_in_focused_buffer(SearchSpec::parse("z"), "x");
    assert_eq!(
        app.status_message,
        Some("Replace All: no matches for z".to_string())
    );
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "abc"
    );
}

#[test]
fn direct_replace_reports_next_or_no_remaining_matches() {
    let mut app = app_with_text("one two one");

    app.replace_in_focused_buffer(SearchSpec::parse("one"), "uno");
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "uno two one"
    );
    assert_eq!(
        app.status_message,
        Some("Replace: 1/2 one -> uno; next 1/1".to_string())
    );

    app.replace_in_focused_buffer(SearchSpec::parse("one"), "uno");
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "uno two uno"
    );
    assert_eq!(
        app.status_message,
        Some("Replace: 1/1 one -> uno; no matches left".to_string())
    );
}

#[test]
fn replace_all_reports_remaining_matches_when_replacement_contains_query() {
    let mut app = app_with_text("a a");

    app.replace_all_in_focused_buffer(SearchSpec::parse("a"), "aa");

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "aa aa"
    );
    assert_eq!(
        app.status_message,
        Some("Replace All: 2 a -> aa; 4 matches remain".to_string())
    );
}

#[test]
fn replace_confirmation_cancel_and_empty_query_paths_are_reported() {
    let mut app = app_with_text("one");

    assert!(!app.handle_replace_confirm_key_event(TerminalKeyEvent::new(
        TerminalKeyCode::Esc,
        TerminalKeyModifiers::NONE
    )));

    app.start_replace_confirmation(SearchSpec::parse(""), "x".to_string());
    assert_eq!(app.status_message, Some("Replace: no query".to_string()));
    assert!(app.replace_confirm.is_none());

    app.start_replace_confirmation(SearchSpec::parse("one"), "uno".to_string());
    assert!(app.replace_confirm.is_some());
    assert_eq!(
        app.confirm_status_text(),
        Some("Match 1/1; replaced 0, skipped 0".to_string())
    );

    assert!(app.handle_replace_confirm_key_event(TerminalKeyEvent::new(
        TerminalKeyCode::Char('c'),
        TerminalKeyModifiers::NONE
    )));
    assert!(app.replace_confirm.is_none());
    assert_eq!(
        app.status_message,
        Some("Replace cancelled: 0 replaced, 0 skipped".to_string())
    );
}

#[test]
fn replace_confirmation_skip_enter_and_unknown_keys_are_handled() {
    let mut app = app_with_text("one");

    app.start_replace_confirmation(SearchSpec::parse("one"), "uno".to_string());
    assert!(app.handle_replace_confirm_key_event(TerminalKeyEvent::new(
        TerminalKeyCode::Char('x'),
        TerminalKeyModifiers::NONE
    )));
    assert!(app.replace_confirm.is_some());
    assert_eq!(
        app.confirm_status_text(),
        Some("Match 1/1; replaced 0, skipped 0".to_string())
    );

    assert!(app.handle_replace_confirm_key_event(TerminalKeyEvent::new(
        TerminalKeyCode::Char('s'),
        TerminalKeyModifiers::NONE
    )));
    assert!(app.replace_confirm.is_none());
    assert_eq!(
        app.status_message,
        Some("Replace done: 0 replaced, 1 skipped".to_string())
    );

    app.start_replace_confirmation(SearchSpec::parse("one"), "uno".to_string());
    assert!(app.handle_replace_confirm_key_event(TerminalKeyEvent::new(
        TerminalKeyCode::Enter,
        TerminalKeyModifiers::NONE
    )));
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "uno"
    );
    assert_eq!(
        app.status_message,
        Some("Replace done: 1 replaced, 0 skipped".to_string())
    );

    app.start_replace_confirmation(SearchSpec::parse("one"), "uno".to_string());
    assert_eq!(
        app.status_message,
        Some("Replace: no matches for one".to_string())
    );
}

#[test]
fn repeat_find_treats_empty_last_query_as_missing() {
    let mut app = app_with_text("abc");

    app.last_find_query = Some(String::new());
    app.handle_command(&EditorCommand::Edit(EditCommand::FindNext));

    assert_eq!(app.status_message, Some("Find: no query".to_string()));
}

#[test]
fn search_results_report_empty_missing_and_invalid_targets() {
    let mut app = app_with_text("abc");

    app.open_search_results_screen();
    assert_eq!(
        app.status_message,
        Some("Search Results: no query".to_string())
    );

    app.last_find_query = Some("z".to_string());
    app.open_search_results_screen();
    assert_eq!(
        app.status_message,
        Some("Search Results: no matches for z".to_string())
    );

    app.last_find_query = Some("a".to_string());
    app.jump_search_result("abc");
    assert_eq!(
        app.status_message,
        Some("Search Results: match number expected".to_string())
    );

    app.jump_search_result("2");
    assert_eq!(
        app.status_message,
        Some("Search Results: match 2 out of range".to_string())
    );

    app.jump_search_result("1");
    assert_eq!(
        app.status_message,
        Some("Search Results: 1/1 a".to_string())
    );

    app.open_search_results_screen();
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::SearchResults
    );
    app.move_focused_numbered_aux_row(1, "Search Results");
    assert_eq!(
        app.status_message,
        Some("Search Results: selected 1/1".to_string())
    );
    assert_eq!(app.focus_first_numbered_aux_row("Search Results"), Some(0));
    assert_eq!(app.focus_last_numbered_aux_row("Search Results"), Some(0));
    app.jump_current_search_result();
    assert_eq!(
        app.status_message,
        Some("Search Results: 1/1 a".to_string())
    );
}

#[test]
fn go_to_line_prompt_moves_cursor_to_requested_line() {
    let mut app = app_with_text("ab\ncd\nef");
    app.buffers[0]
        .buffer
        .set_cursor(Position::new(0, 1))
        .unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::GoToLine));
    assert_eq!(app.prompt_status_text(), Some("Go To Line: ".to_string()));

    send_text(&mut app, "3");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.cursor_position(), Position::new(2, 1));
    assert_eq!(state.buffer.selection_range(), None);
    assert_eq!(app.status_message, Some("Go to line: 3".to_string()));
}

#[test]
fn go_to_line_rejects_invalid_or_out_of_range_input() {
    let mut app = app_with_text("ab\ncd");
    app.buffers[0]
        .buffer
        .set_cursor(Position::new(1, 1))
        .unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::GoToLine));
    send_text(&mut app, "abc");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    assert_eq!(
        app.status_message,
        Some("Go to line failed: invalid line number abc".to_string())
    );
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .cursor_position(),
        Position::new(1, 1)
    );

    app.handle_command(&EditorCommand::Edit(EditCommand::GoToLine));
    send_text(&mut app, "9");
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    assert_eq!(
        app.status_message,
        Some("Go to line failed: line 9 is past end (2 lines)".to_string())
    );
    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .cursor_position(),
        Position::new(1, 1)
    );
}
