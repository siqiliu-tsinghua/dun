//! Cell-level parity between the Surface render path and the ratatui path.
//!
//! Both renderers draw the same `UiFrame`; these tests assert the Surface
//! twin produces the same glyphs and colors cell by cell, region by region.
//! The comparison region grows as render layers are ported: today the menu
//! row, the status row, and the active dropdown panel; the window-body port
//! (slice 3c) extends it to the full frame. Full-frame parity across the
//! existing rendering fixtures is the acceptance bar for the cutover.
//!
//! Glyph, foreground, and background must match exactly — those are what the
//! `emit_diff` byte stream will carry and what a user sees. Text modifiers
//! are asserted as `surface ⊆ ratatui`, not equality: ratatui's
//! `Cell::set_style` patches (`modifier.insert(add); modifier.remove(sub)`)
//! and `to_ratatui_style` never sets `sub_modifier`, so a modifier inserted
//! by one layer (e.g. a focused window border's BOLD) bleeds through every
//! plain style painted over it (the dropdown panel fill and border). The
//! Surface path replaces styles cleanly and carries only the intended
//! modifier; the per-layer `surface_layers` unit tests pin those intended
//! modifiers exactly, so the subset relation here is the precise statement
//! of the corrected behavior rather than a weakened check.

use super::support::*;
use crate::render::surface_frame::render_ui_frame_to_surface;
use crate::surface::Surface;
use dun_term::{AnsiColor, Style as DunStyle, StyleAttrs, TerminalColor};
use ratatui::style::{Color, Modifier};

/// The inverse of `render::chrome::to_ratatui_color`. Panics on colors the
/// forward map can never produce, so drift in either direction fails loudly.
fn dun_color(color: Color) -> TerminalColor {
    match color {
        Color::Reset => TerminalColor::Default,
        Color::Indexed(index) => TerminalColor::Indexed(index),
        Color::Black => TerminalColor::Ansi(AnsiColor::Black),
        Color::Red => TerminalColor::Ansi(AnsiColor::Red),
        Color::Green => TerminalColor::Ansi(AnsiColor::Green),
        Color::Yellow => TerminalColor::Ansi(AnsiColor::Yellow),
        Color::Blue => TerminalColor::Ansi(AnsiColor::Blue),
        Color::Magenta => TerminalColor::Ansi(AnsiColor::Magenta),
        Color::Cyan => TerminalColor::Ansi(AnsiColor::Cyan),
        Color::White => TerminalColor::Ansi(AnsiColor::White),
        Color::DarkGray => TerminalColor::Ansi(AnsiColor::BrightBlack),
        Color::LightRed => TerminalColor::Ansi(AnsiColor::BrightRed),
        Color::LightGreen => TerminalColor::Ansi(AnsiColor::BrightGreen),
        Color::LightYellow => TerminalColor::Ansi(AnsiColor::BrightYellow),
        Color::LightBlue => TerminalColor::Ansi(AnsiColor::BrightBlue),
        Color::LightMagenta => TerminalColor::Ansi(AnsiColor::BrightMagenta),
        Color::LightCyan => TerminalColor::Ansi(AnsiColor::BrightCyan),
        Color::Gray => TerminalColor::Ansi(AnsiColor::BrightWhite),
        other => panic!("ratatui color {other:?} is outside dun's forward map"),
    }
}

/// The intended-modifier subset relation: every modifier the Surface carries
/// must also be present in the ratatui cell. ratatui may carry extra bled
/// modifiers (see the module comment); the Surface must never carry one the
/// ratatui path lacks, which would signal a genuinely missing/spurious attr.
fn surface_modifiers_subset(surface: StyleAttrs, ratatui: Modifier) -> bool {
    (!surface.bold || ratatui.contains(Modifier::BOLD))
        && (!surface.underline || ratatui.contains(Modifier::UNDERLINED))
        && (!surface.reverse || ratatui.contains(Modifier::REVERSED))
}

/// Asserts glyph and color equality plus the modifier subset relation over a
/// rect. Surface continuation cells (the second column of a wide glyph) map
/// to ratatui's reset cell: ratatui blanks the covered cell to symbol " ",
/// while Surface keeps the head's style there, so continuations compare
/// symbol-only.
pub(super) fn assert_region_matches(
    surface: &Surface,
    buffer: &Buffer,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) {
    for row in y..y.saturating_add(height) {
        for column in x..x.saturating_add(width) {
            let surface_cell = surface
                .cell(column, row)
                .unwrap_or_else(|| panic!("surface has no cell ({column}, {row})"));
            let buffer_cell = &buffer[(column, row)];
            if surface_cell.wide_continuation {
                assert_eq!(
                    buffer_cell.symbol(),
                    " ",
                    "({column}, {row}): ratatui cell behind a wide glyph must be reset"
                );
                continue;
            }
            assert_eq!(
                surface_cell.symbol,
                buffer_cell.symbol(),
                "symbol mismatch at ({column}, {row})"
            );
            let ratatui_style = buffer_cell.style();
            assert_eq!(
                surface_cell.style.fg,
                dun_color(ratatui_style.fg.unwrap_or(Color::Reset)),
                "fg mismatch at ({column}, {row}) on symbol {:?}",
                surface_cell.symbol
            );
            assert_eq!(
                surface_cell.style.bg,
                dun_color(ratatui_style.bg.unwrap_or(Color::Reset)),
                "bg mismatch at ({column}, {row}) on symbol {:?}",
                surface_cell.symbol
            );
            assert!(
                surface_modifiers_subset(surface_cell.style.attrs, ratatui_style.add_modifier),
                "modifier at ({column}, {row}) on symbol {:?}: surface {:?} not a subset of \
                 ratatui {:?}",
                surface_cell.symbol,
                surface_cell.style.attrs,
                ratatui_style.add_modifier
            );
        }
    }
}

const INITIAL_STYLE: DunStyle = DunStyle::new(
    TerminalColor::Indexed(254),
    TerminalColor::Indexed(255),
    StyleAttrs::BOLD_REVERSE,
);

fn render_both(
    shell: &UiShell,
    ui_frame: &UiFrame,
    width: u16,
    height: u16,
) -> (Surface, Terminal<TestBackend>, Option<(u16, u16)>) {
    let mut surface = Surface::new(width, height, INITIAL_STYLE);
    let cursor = render_ui_frame_to_surface(&mut surface, shell, ui_frame);

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| shell.render(frame, ui_frame))
        .unwrap();

    (surface, terminal, cursor)
}

#[test]
fn plain_single_window_full_frame_matches_ratatui() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hello\nworld");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 60, 8), &[buffer_view]);

    let (surface, terminal, _) = render_both(&shell, &ui_frame, 60, 10);

    assert_region_matches(&surface, terminal.backend().buffer(), 0, 0, 60, 10);
}

#[test]
fn tiny_tiled_split_full_frame_matches_ratatui() {
    let mut workspace = Workspace::new_untitled();
    workspace.split_focused(Axis::Horizontal).unwrap();
    let first = TextBuffer::from_text_with_kind(BufferKind::Untitled, "left");
    let second = TextBuffer::from_text_with_kind(BufferKind::Untitled, "right");
    let buffers = [
        BufferView::new(BufferId(1), &first),
        BufferView::new(BufferId(2), &second),
    ];
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 8, 2), &buffers);

    let (surface, terminal, _) = render_both(&shell, &ui_frame, 8, 4);

    assert_region_matches(&surface, terminal.backend().buffer(), 0, 0, 8, 4);
}

#[test]
fn viewport_polish_full_frame_matches_ratatui() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(
        BufferKind::Untitled,
        "0123456789\nline2\nline3\nline4\nline5\nline6\nline7\nline8",
    );
    let buffer_view = BufferView::scrolled_xy(BufferId(1), &buffer, 2, 2);
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 20, 6), &[buffer_view]);

    let (surface, terminal, _) = render_both(&shell, &ui_frame, 20, 8);

    assert_region_matches(&surface, terminal.backend().buffer(), 0, 0, 20, 8);
}

#[test]
fn plugin_highlight_full_frame_matches_ratatui() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "fn main() {}");
    let highlights = [BufferHighlightSpan {
        line: 0,
        start_column: 0,
        end_column: 2,
        class: HighlightClass::Keyword,
    }];
    let buffer_view = BufferView::new(BufferId(1), &buffer).with_highlight_spans(&highlights);
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 40, 8), &[buffer_view]);

    let (surface, terminal, _) = render_both(&shell, &ui_frame, 40, 10);

    assert_region_matches(&surface, terminal.backend().buffer(), 0, 0, 40, 10);
}

#[test]
fn menu_window_status_full_frame_matches_ratatui() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "alpha\nbeta");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 40, 6), &[buffer_view]);

    let (surface, terminal, _) = render_both(&shell, &ui_frame, 40, 8);

    assert_region_matches(&surface, terminal.backend().buffer(), 0, 0, 40, 8);
}

#[test]
fn active_dropdown_full_frame_matches_ratatui() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace_with_menu(
        &workspace,
        Rect::new(0, 0, 80, 12),
        &[buffer_view],
        Some(0),
    );

    let (surface, terminal, _) = render_both(&shell, &ui_frame, 80, 14);

    assert_region_matches(&surface, terminal.backend().buffer(), 0, 0, 80, 14);
}

#[test]
fn prompt_overlay_full_frame_matches_ratatui() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let mut ui_frame =
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);
    ui_frame.overlay = Some(UiOverlay::prompt("Go To Line", "12", 2));

    let (surface, mut terminal, cursor) = render_both(&shell, &ui_frame, 80, 12);

    assert_region_matches(&surface, terminal.backend().buffer(), 0, 0, 80, 12);
    let ratatui_cursor = terminal.get_cursor_position().unwrap();
    assert_eq!(cursor, Some((ratatui_cursor.x, ratatui_cursor.y)));
}

#[test]
fn file_dialog_overlay_full_frame_matches_ratatui() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let mut ui_frame =
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 90, 14), &[buffer_view]);
    ui_frame.overlay = Some(UiOverlay::file_dialog(
        "Open",
        vec![
            "Directory: /tmp".to_string(),
            "Select a file or type a path.".to_string(),
        ],
        "/tmp/al",
        7,
        vec!["[D] logs/".to_string(), "    alpha.log".to_string()],
        Some(1),
        vec!["Enter  Tab complete  Up/Down select  Esc cancel".to_string()],
    ));

    let (surface, mut terminal, cursor) = render_both(&shell, &ui_frame, 90, 16);

    assert_region_matches(&surface, terminal.backend().buffer(), 0, 0, 90, 16);
    let ratatui_cursor = terminal.get_cursor_position().unwrap();
    assert_eq!(cursor, Some((ratatui_cursor.x, ratatui_cursor.y)));
}

#[test]
fn modal_list_overflow_full_frame_matches_ratatui() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let mut ui_frame =
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 60, 10), &[buffer_view]);
    ui_frame.overlay = Some(
        UiOverlay::message(
            "Switch Buffer",
            vec!["Showing 5-7 of 20".to_string()],
            vec![],
        )
        .with_list(
            vec![
                "  buffer-05".to_string(),
                "> buffer-06".to_string(),
                "  buffer-07".to_string(),
            ],
            Some(1),
            32,
        )
        .with_list_overflow(true, true),
    );

    let (surface, terminal, _) = render_both(&shell, &ui_frame, 60, 12);

    assert_region_matches(&surface, terminal.backend().buffer(), 0, 0, 60, 12);
}

#[test]
fn scrolled_dropdown_panel_matches_ratatui() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let last_entry = shell.menu_entry_count(2).unwrap() - 1;
    let ui_frame = shell.frame_for_workspace_with_menu_selection(
        &workspace,
        Rect::new(0, 0, 80, 10),
        &[buffer_view],
        Some(MenuSelection::with_entry(2, last_entry)),
    );

    let (surface, terminal, _) = render_both(&shell, &ui_frame, 80, 12);

    let rect = crate::render::menu::clamp_menu_rect(
        crate::render::menu::dropdown_rect_for_menu(&shell, &ui_frame.menu, 2).unwrap(),
        TuiRect::new(0, 0, 80, 12),
    )
    .unwrap();
    assert_region_matches(
        &surface,
        terminal.backend().buffer(),
        rect.x,
        rect.y,
        rect.width,
        rect.height,
    );
}

#[test]
fn cursor_position_matches_ratatui() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hello\nworld");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 60, 8), &[buffer_view]);

    let (_, mut terminal, cursor) = render_both(&shell, &ui_frame, 60, 10);

    let ratatui_cursor = terminal.get_cursor_position().unwrap();
    assert_eq!(cursor, Some((ratatui_cursor.x, ratatui_cursor.y)));
}
