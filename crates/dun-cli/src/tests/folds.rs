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
