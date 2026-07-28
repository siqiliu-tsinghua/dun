use super::support::*;
use crate::render::surface_frame::render_ui_frame_to_surface;
use crate::surface::Surface;
use dun_core::{FoldRange, FoldSet};

const WIDTH: u16 = 64;
const HEIGHT: u16 = 10;

fn folded_buffer(text: &str, range: FoldRange) -> TextBuffer {
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, text);
    buffer.set_folds(FoldSet::new(vec![range]).expect("test fold must be valid"));
    buffer
}

fn frame_for_buffer<'a>(
    shell: &UiShell,
    buffer: &'a TextBuffer,
    configure: impl FnOnce(BufferView<'a>) -> BufferView<'a>,
) -> UiFrame {
    let view = configure(
        BufferView::new(BufferId(1), buffer)
            .with_folds(buffer.folds())
            .with_bookmarks(buffer.bookmarks()),
    );
    shell.frame_for_workspace(
        &Workspace::new_untitled(),
        Rect::new(0, 0, WIDTH, HEIGHT - 2),
        &[view],
    )
}

fn rendered_surface(shell: &UiShell, frame: &UiFrame) -> Surface {
    let mut surface = Surface::new(WIDTH, HEIGHT, shell.theme.palette.editor);
    render_ui_frame_to_surface(&mut surface, shell, frame);
    surface
}

fn body_cell_style(surface: &Surface, window: &UiWindow, row: u16, column: u16) -> dun_term::Style {
    let x = window
        .rect
        .x
        .saturating_add(window.geometry.body.x)
        .saturating_add(column);
    let y = 1u16
        .saturating_add(window.rect.y)
        .saturating_add(window.geometry.body.y)
        .saturating_add(row);
    surface.cell(x, y).expect("body cell must exist").style
}

#[test]
fn placeholder_excerpt_is_sanitised() {
    let mut buffer = folded_buffer(
        "bad\x1b[31m\u{202e}tail\nhidden\nafter",
        FoldRange::new(0, 2),
    );
    buffer.set_cursor(Position::new(2, 0)).unwrap();
    let shell = UiShell::default();
    let frame = frame_for_buffer(&shell, &buffer, |view| view);
    let expected = "▶ [2] bad␛[31m<U+202E>tail";

    assert_eq!(frame.windows[0].body[0].as_plain_text(), expected);

    let surface = rendered_surface(&shell, &frame);
    let window = &frame.windows[0];
    let x = window.rect.x.saturating_add(window.geometry.body.x);
    let y = 1u16
        .saturating_add(window.rect.y)
        .saturating_add(window.geometry.body.y);
    let actual = (0..expected.chars().count() as u16)
        .map(|column| {
            surface
                .cell(x.saturating_add(column), y)
                .expect("expected placeholder cell")
                .symbol
                .as_str()
        })
        .collect::<String>();

    assert_eq!(actual, expected);
}

#[test]
fn placeholder_gutter_shows_start_line_and_aggregated_bookmark() {
    let mut buffer = folded_buffer("one\ntwo\nthree\nfour\nfive", FoldRange::new(1, 4));
    buffer.set_bookmarks(vec![3]);
    let frame = frame_for_buffer(&UiShell::default(), &buffer, |view| view);

    assert_eq!(
        frame.windows[0].gutter,
        vec![
            UiGutterLine {
                y: 1,
                label: "1 ".to_string(),
                marked: false,
            },
            UiGutterLine {
                y: 2,
                label: "2*".to_string(),
                marked: true,
            },
            UiGutterLine {
                y: 3,
                label: "5 ".to_string(),
                marked: false,
            },
        ]
    );
}

#[test]
fn placeholder_takes_current_line_selection_and_search_styles() {
    let shell = UiShell::default();
    let range = FoldRange::new(1, 4);

    let mut current = folded_buffer("outside\nneedle one\nneedle two\nfold end\nafter", range);
    current.set_cursor(Position::new(2, 3)).unwrap();
    let current_frame = frame_for_buffer(&shell, &current, |view| view);
    let current_window = &current_frame.windows[0];
    let current_surface = rendered_surface(&shell, &current_frame);
    assert_eq!(
        current_window.cursor,
        Some(UiCursor {
            x: current_window.geometry.body.x,
            y: current_window.geometry.body.y + 1,
        })
    );
    assert_eq!(
        body_cell_style(&current_surface, current_window, 1, 0),
        shell.theme.palette.current_line
    );

    let mut selected = folded_buffer("outside\nneedle one\nneedle two\nfold end\nafter", range);
    selected
        .select(Position::new(2, 1), Position::new(2, 4))
        .unwrap();
    let selection_frame = frame_for_buffer(&shell, &selected, |view| view);
    let selection_window = &selection_frame.windows[0];
    let body = selection_window.geometry.body;
    assert_eq!(
        selection_window.selection,
        vec![UiSelectionLine {
            y: body.y + 1,
            start_x: body.x,
            end_x: body.x + body.width,
        }]
    );
    let selection_surface = rendered_surface(&shell, &selection_frame);
    assert_eq!(
        body_cell_style(&selection_surface, selection_window, 1, 0),
        shell.theme.palette.selection_text
    );
    assert_eq!(
        body_cell_style(
            &selection_surface,
            selection_window,
            1,
            body.width.saturating_sub(1),
        ),
        shell.theme.palette.selection_text
    );

    let mut searched = folded_buffer("outside\nneedle one\nneedle two\nfold end\nafter", range);
    searched.set_cursor(Position::new(0, 0)).unwrap();
    let matches = searched.find_all("needle");
    let ordinary_frame =
        frame_for_buffer(&shell, &searched, |view| view.with_search(&matches, None));
    let ordinary_window = &ordinary_frame.windows[0];
    assert_eq!(
        ordinary_window.search_matches,
        vec![UiSearchMatchLine {
            y: ordinary_window.geometry.body.y + 1,
            start_x: ordinary_window.geometry.body.x,
            end_x: ordinary_window.geometry.body.x + ordinary_window.geometry.body.width,
            active: false,
        }]
    );
    let ordinary_surface = rendered_surface(&shell, &ordinary_frame);
    assert_eq!(
        body_cell_style(&ordinary_surface, ordinary_window, 1, 0),
        shell.theme.palette.search_match
    );

    let active_frame = frame_for_buffer(&shell, &searched, |view| {
        view.with_search(&matches, Some(0))
    });
    let active_window = &active_frame.windows[0];
    assert_eq!(active_window.search_matches.len(), 1);
    assert!(active_window.search_matches[0].active);
    let active_surface = rendered_surface(&shell, &active_frame);
    assert_eq!(
        body_cell_style(&active_surface, active_window, 1, 0),
        shell.theme.palette.active_search_match
    );
}

#[test]
fn plugin_spans_never_paint_the_placeholder() {
    let mut buffer = folded_buffer(
        "outside\nfold first\nfold second\nafter",
        FoldRange::new(1, 3),
    );
    buffer.set_cursor(Position::new(0, 0)).unwrap();
    let highlights = [
        BufferHighlightSpan {
            line: 1,
            start_column: 0,
            end_column: 4,
            class: HighlightClass::Keyword,
        },
        BufferHighlightSpan {
            line: 2,
            start_column: 0,
            end_column: 4,
            class: HighlightClass::Comment,
        },
    ];
    let shell = UiShell::default();

    for wrap in [false, true] {
        let frame = frame_for_buffer(&shell, &buffer, |view| {
            view.with_wrap(wrap).with_highlight_spans(&highlights)
        });
        let window = &frame.windows[0];
        assert!(
            window.highlights.is_empty(),
            "wrap={wrap} exposed a hidden plugin span"
        );
        let surface = rendered_surface(&shell, &frame);
        assert_eq!(
            body_cell_style(&surface, window, 1, 0),
            shell.theme.palette.editor_text,
            "wrap={wrap} painted the placeholder"
        );
    }
}
