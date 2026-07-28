use super::support::*;
use dun_core::{FoldRange, FoldSet};

fn install_folded_text(app: &mut AppState, text: &str, range: FoldRange) {
    let buffer = &mut app.buffer_state_mut(BufferId(1)).unwrap().buffer;
    *buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, text);
    buffer.set_folds(FoldSet::new(vec![range]).expect("test fold must be valid"));
}

fn frame_for_app(app: &AppState, area: Rect) -> dun_ui::UiFrame {
    let views = app.buffer_views();
    app.shell.frame_for_workspace(&app.workspace, area, &views)
}

fn snapshot_text_rows(snapshot: &str) -> Vec<&str> {
    let (_, text_and_rest) = snapshot
        .split_once("\ntext:\n")
        .expect("snapshot must contain text rows");
    let (text, _) = text_and_rest
        .split_once("\nstyle:\n")
        .expect("snapshot must contain style rows");
    text.lines()
        .map(|line| line.split_once('|').expect("numbered snapshot row").1)
        .collect()
}

#[test]
fn toggle_fold_folds_the_selected_lines_and_clears_the_selection() {
    let mut app = app_with_text("above\nfirst\nsecond\nexcluded\nafter");
    let buffer = &mut app.focused_buffer_mut().unwrap().buffer;
    buffer
        .select(Position::new(1, 2), Position::new(3, 0))
        .unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::ToggleFold));

    let buffer = &app.focused_buffer().unwrap().buffer;
    assert_eq!(buffer.folds().ranges(), [FoldRange::new(1, 3)]);
    assert_eq!(buffer.cursor_position(), Position::new(1, 0));
    assert_eq!(buffer.selection(), None);
    assert!(!buffer.is_dirty());
    assert_eq!(app.status_message.as_deref(), Some("Folded 2 lines"));

    app.focused_buffer_mut()
        .unwrap()
        .buffer
        .set_cursor(Position::new(0, 0))
        .unwrap();
    app.handle_command(&EditorCommand::Edit(EditCommand::MoveDown));
    assert_eq!(
        app.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(1, 0)
    );
    app.handle_command(&EditorCommand::Edit(EditCommand::MoveDown));
    assert_eq!(
        app.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(3, 0)
    );
}

#[test]
fn toggle_fold_without_a_selection_unfolds_at_the_cursor() {
    let mut app = AppState::new();
    install_folded_text(
        &mut app,
        "above\nfirst\nsecond\nafter",
        FoldRange::new(1, 3),
    );
    let buffer = &mut app.focused_buffer_mut().unwrap().buffer;
    buffer.set_cursor(Position::new(2, 3)).unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::ToggleFold));

    let buffer = &app.focused_buffer().unwrap().buffer;
    assert!(buffer.folds().is_empty());
    assert_eq!(buffer.cursor_position(), Position::new(2, 3));
    assert!(!buffer.is_dirty());
    assert_eq!(app.status_message.as_deref(), Some("Unfolded"));
}

#[test]
fn toggle_fold_reports_when_fewer_than_two_lines_are_selected() {
    let mut selected = app_with_text("first\nsecond");
    selected
        .focused_buffer_mut()
        .unwrap()
        .buffer
        .select(Position::new(0, 0), Position::new(0, 3))
        .unwrap();
    let before = selected.focused_buffer().unwrap().buffer.clone();

    selected.handle_command(&EditorCommand::Edit(EditCommand::ToggleFold));

    assert_eq!(selected.focused_buffer().unwrap().buffer, before);
    assert_eq!(
        selected.status_message.as_deref(),
        Some("Fold: select at least two lines")
    );

    let mut unselected = app_with_text("first\nsecond");
    let before = unselected.focused_buffer().unwrap().buffer.clone();
    unselected.handle_command(&EditorCommand::Edit(EditCommand::ToggleFold));
    assert_eq!(unselected.focused_buffer().unwrap().buffer, before);
    assert_eq!(
        unselected.status_message.as_deref(),
        Some("Fold: select at least two lines")
    );
}

#[test]
fn unfold_all_clears_every_fold_and_reports_the_count() {
    let mut app = app_with_text("zero\none\ntwo\nthree\nfour\nfive");
    app.focused_buffer_mut()
        .unwrap()
        .buffer
        .set_folds(FoldSet::new(vec![FoldRange::new(0, 2), FoldRange::new(3, 6)]).unwrap());

    app.handle_command(&EditorCommand::Edit(EditCommand::UnfoldAll));

    assert!(app.focused_buffer().unwrap().buffer.folds().is_empty());
    assert_eq!(app.status_message.as_deref(), Some("Unfolded 2 fold(s)"));

    app.handle_command(&EditorCommand::Edit(EditCommand::UnfoldAll));
    assert_eq!(
        app.status_message.as_deref(),
        Some("Unfold: nothing to unfold")
    );

    let focused = app.focused_buffer_id().unwrap();
    app.buffers.retain(|buffer| buffer.id != focused);
    app.handle_command(&EditorCommand::Edit(EditCommand::UnfoldAll));
    assert_eq!(
        app.status_message.as_deref(),
        Some("Fold failed: focused buffer is missing")
    );
}

#[test]
fn go_to_line_and_bookmark_jumps_expand_a_hidden_target() {
    let mut go_to = AppState::new();
    install_folded_text(
        &mut go_to,
        "above\nfirst\nsecond\nthird\nafter",
        FoldRange::new(1, 4),
    );
    go_to.go_to_line("3");
    assert!(go_to.focused_buffer().unwrap().buffer.folds().is_empty());
    assert_eq!(
        go_to.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(2, 0)
    );

    let mut next = AppState::new();
    install_folded_text(
        &mut next,
        "above\nfirst\nsecond\nthird\nafter",
        FoldRange::new(1, 4),
    );
    next.focused_buffer_mut()
        .unwrap()
        .buffer
        .set_bookmarks(vec![2]);
    next.handle_command(&EditorCommand::Edit(EditCommand::NextBookmark));
    assert!(next.focused_buffer().unwrap().buffer.folds().is_empty());
    assert_eq!(
        next.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(2, 0)
    );

    let mut previous = AppState::new();
    install_folded_text(
        &mut previous,
        "above\nfirst\nsecond\nthird\nafter",
        FoldRange::new(1, 4),
    );
    let buffer = &mut previous.focused_buffer_mut().unwrap().buffer;
    buffer.set_bookmarks(vec![2]);
    buffer.set_cursor(Position::new(4, 0)).unwrap();
    previous.handle_command(&EditorCommand::Edit(EditCommand::PreviousBookmark));
    assert!(previous.focused_buffer().unwrap().buffer.folds().is_empty());
    assert_eq!(
        previous.focused_buffer().unwrap().buffer.cursor_position(),
        Position::new(2, 0)
    );

    for command in [
        EditCommand::MoveLeft,
        EditCommand::MoveRight,
        EditCommand::MoveWordLeft,
        EditCommand::MoveWordRight,
        EditCommand::MoveLineStart,
        EditCommand::MoveLineEnd,
        EditCommand::ExtendSelectionWordLeft,
        EditCommand::ExtendSelectionWordRight,
    ] {
        let mut movement = AppState::new();
        install_folded_text(
            &mut movement,
            "above\nfirst words\nsecond\nafter",
            FoldRange::new(1, 3),
        );
        movement
            .focused_buffer_mut()
            .unwrap()
            .buffer
            .set_cursor(Position::new(1, 0))
            .unwrap();
        movement.handle_command(&EditorCommand::Edit(command));
        assert!(movement.focused_buffer().unwrap().buffer.folds().is_empty());
    }

    for key in [Key::Left, Key::Right, Key::Home, Key::End] {
        let mut movement = AppState::new();
        install_folded_text(
            &mut movement,
            "above\nfirst words\nsecond\nafter",
            FoldRange::new(1, 3),
        );
        movement
            .focused_buffer_mut()
            .unwrap()
            .buffer
            .set_cursor(Position::new(1, 0))
            .unwrap();
        movement.handle_selection_key_stroke(KeyStroke::new(key, KeyModifiers::SHIFT));
        assert!(movement.focused_buffer().unwrap().buffer.folds().is_empty());
    }
}

#[test]
fn committed_search_jump_expands_but_preview_does_not() {
    const TEXT: &str = "above\nneedle here\nhidden\nafter";
    const FOLD: FoldRange = FoldRange::new(1, 3);

    let mut submitted = AppState::new();
    install_folded_text(&mut submitted, TEXT, FOLD);
    submitted.handle_command(&EditorCommand::Edit(EditCommand::Find));
    send_text(&mut submitted, "needle");
    assert_eq!(
        submitted.focused_buffer().unwrap().buffer.folds().ranges(),
        [FOLD]
    );
    assert_eq!(
        submitted.focused_buffer().unwrap().buffer.selection_range(),
        Some(TextRange::new(Position::new(1, 0), Position::new(1, 6)))
    );

    handle_key_event(
        &mut submitted,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );
    assert!(
        submitted
            .focused_buffer()
            .unwrap()
            .buffer
            .folds()
            .is_empty()
    );

    for (command, cursor) in [
        (EditCommand::FindNext, Position::new(0, 0)),
        (EditCommand::FindPrevious, Position::new(3, 0)),
    ] {
        let mut repeated = AppState::new();
        install_folded_text(&mut repeated, TEXT, FOLD);
        repeated.last_find_query = Some("needle".to_string());
        repeated
            .focused_buffer_mut()
            .unwrap()
            .buffer
            .set_cursor(cursor)
            .unwrap();
        repeated.handle_command(&EditorCommand::Edit(command));
        assert!(repeated.focused_buffer().unwrap().buffer.folds().is_empty());
    }

    let mut replace = AppState::new();
    install_folded_text(&mut replace, TEXT, FOLD);
    replace.start_replace_confirmation(SearchSpec::parse("needle"), "replacement".to_string());
    assert!(replace.replace_confirm.is_some());
    assert!(replace.focused_buffer().unwrap().buffer.folds().is_empty());
}

#[test]
fn folded_range_draws_one_row_at_any_width() {
    const FOLDED: &str = "fold excerpt that would wrap many times";
    const VISIBLE: &str = "abcdefghijklmnopqrstuvwx";
    const PLACEHOLDER: &str = "▶ [2] fold excerpt that would wrap many times";
    let text = format!("{FOLDED}\nhidden\n{VISIBLE}");
    let mut saw_narrower_than_excerpt = false;
    let mut saw_wrapped_source = false;

    for width in 3..=32 {
        let mut app = AppState::new();
        install_folded_text(&mut app, &text, FoldRange::new(0, 2));
        app.buffer_state_mut(BufferId(1)).unwrap().word_wrap = true;
        let frame = frame_for_app(&app, Rect::new(0, 0, width, 80));
        let window = &frame.windows[0];
        let body_width = usize::from(window.geometry.body.width);
        if body_width == 0 {
            continue;
        }

        let wrapped_source_rows = app
            .shell
            .editor_text_display(false)
            .wrapped_row_count(VISIBLE, body_width);
        saw_narrower_than_excerpt |= body_width < FOLDED.len();
        saw_wrapped_source |= wrapped_source_rows > 1;
        assert_eq!(
            window.body.len(),
            1 + wrapped_source_rows,
            "body width {body_width} wrapped the placeholder"
        );
        assert_eq!(window.body[0].as_plain_text(), PLACEHOLDER);
        assert_eq!(
            window.body[1..]
                .iter()
                .map(|line| line.as_plain_text())
                .collect::<String>(),
            VISIBLE
        );

        let snapshot = dun_ui::frame_snapshot(&app.shell, &frame, width, 82);
        let rows = snapshot_text_rows(&snapshot);
        let placeholder_y = 1 + usize::from(window.rect.y) + usize::from(window.geometry.body.y);
        let body_x = usize::from(window.rect.x) + usize::from(window.geometry.body.x);
        let rendered = rows[placeholder_y]
            .chars()
            .skip(body_x)
            .take(body_width)
            .collect::<String>();
        let mut expected = PLACEHOLDER.chars().take(body_width).collect::<String>();
        let expected_width = expected.chars().count();
        expected.extend(std::iter::repeat_n(
            ' ',
            body_width.saturating_sub(expected_width),
        ));
        assert_eq!(rendered, expected, "body width {body_width} clipped badly");
    }

    assert!(saw_narrower_than_excerpt);
    assert!(saw_wrapped_source);

    let mut scrolled = AppState::new();
    install_folded_text(&mut scrolled, &text, FoldRange::new(0, 2));
    scrolled.buffer_state_mut(BufferId(1)).unwrap().first_column = 12;
    let scrolled_frame = frame_for_app(&scrolled, Rect::new(0, 0, 24, 6));
    assert_eq!(
        scrolled_frame.windows[0].body[0].as_plain_text(),
        "▶ [2] fold excerpt that would wrap many times"
    );

    let mut ascii = AppState::from_config(Config {
        terminal: TerminalOverrides {
            encoding: Some(EncodingProfile::Ascii),
            ..TerminalOverrides::default()
        },
        ..Config::default()
    });
    install_folded_text(&mut ascii, &text, FoldRange::new(0, 2));
    let ascii_frame = frame_for_app(&ascii, Rect::new(0, 0, 24, 6));
    assert_eq!(
        ascii_frame.windows[0].body[0].as_plain_text(),
        "> [2] fold excerpt that would wrap many times"
    );

    let mut remapped = AppState::new();
    install_folded_text(
        &mut remapped,
        "outside\nfold first\nfold second\nafter",
        FoldRange::new(1, 3),
    );
    remapped
        .buffer_state_mut(BufferId(1))
        .unwrap()
        .buffer
        .replace_range(TextRange::empty(Position::zero()), "new\n")
        .unwrap();
    assert_eq!(
        remapped
            .buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .folds()
            .ranges(),
        [FoldRange::new(2, 4)]
    );
    let remapped_frame = frame_for_app(&remapped, Rect::new(0, 0, 40, 8));
    assert_eq!(
        remapped_frame.windows[0]
            .body
            .iter()
            .map(|line| line.as_plain_text())
            .collect::<Vec<_>>(),
        ["new", "outside", "▶ [2] fold first", "after"]
    );
}

#[test]
fn cursor_and_click_inside_a_fold_resolve_to_the_start_line() {
    let mut app = AppState::new();
    app.mouse_enabled = true;
    install_folded_text(
        &mut app,
        "outside\nfold first\nfold second\nafter",
        FoldRange::new(1, 3),
    );
    let area = Rect::new(0, 0, 40, 8);
    app.sync_view_for_area(area);

    for position in [Position::new(1, 4), Position::new(2, 6)] {
        app.buffer_state_mut(BufferId(1))
            .unwrap()
            .buffer
            .set_cursor(position)
            .unwrap();
        let frame = frame_for_app(&app, area);
        let window = &frame.windows[0];
        assert_eq!(
            window.cursor,
            Some(dun_ui::UiCursor {
                x: window.geometry.body.x,
                y: window.geometry.body.y + 1,
            }),
            "cursor {position:?} did not map to the placeholder origin"
        );
    }

    let frame = frame_for_app(&app, area);
    let window = &frame.windows[0];
    let click_x = window.rect.x + window.geometry.body.x + 7;
    let click_y = 1 + window.rect.y + window.geometry.body.y + 1;
    handle_mouse_event(&mut app, left_click(click_x, click_y));

    assert_eq!(
        app.buffer_state(BufferId(1))
            .unwrap()
            .buffer
            .cursor_position(),
        Position::new(1, 0)
    );
    assert!(matches!(
        app.mouse_drag,
        Some(MouseDragState::Selection {
            buffer_id: BufferId(1),
            anchor: Position { line: 1, column: 0 },
        })
    ));
}

#[test]
fn empty_fold_set_renders_byte_identically() {
    let mut app = AppState::new();
    app.buffer_state_mut(BufferId(1)).unwrap().buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, "alpha\nbeta");
    let area = Rect::new(0, 0, 24, 6);
    let production_frame = frame_for_app(&app, area);
    let buffer = app.buffer_state(BufferId(1)).unwrap();
    assert!(buffer.buffer.folds().is_empty());

    let baseline_view = BufferView::new(BufferId(1), &buffer.buffer);
    let baseline_frame = app
        .shell
        .frame_for_workspace(&app.workspace, area, &[baseline_view]);

    assert_eq!(production_frame, baseline_frame);
    assert_eq!(
        production_frame.windows[0]
            .body
            .iter()
            .map(|line| line.as_plain_text())
            .collect::<Vec<_>>(),
        ["alpha", "beta"]
    );
    assert_eq!(
        production_frame.windows[0].gutter,
        vec![
            dun_ui::UiGutterLine {
                y: 1,
                label: "1 ".to_string(),
                marked: false,
            },
            dun_ui::UiGutterLine {
                y: 2,
                label: "2 ".to_string(),
                marked: false,
            },
        ]
    );
    assert_eq!(
        production_frame.windows[0].cursor,
        Some(dun_ui::UiCursor { x: 3, y: 1 })
    );

    let mut production_renderer = dun_ui::SurfaceRenderer::new();
    let production =
        production_renderer.render(&app.shell, &production_frame, area.width, area.height + 2);
    let mut baseline_renderer = dun_ui::SurfaceRenderer::new();
    let baseline =
        baseline_renderer.render(&app.shell, &baseline_frame, area.width, area.height + 2);
    assert_eq!(production.bytes, baseline.bytes);
    assert_eq!(production.cursor, baseline.cursor);
}
