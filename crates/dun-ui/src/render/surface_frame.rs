use ratatui::layout::Rect as TuiRect;

use crate::render::surface_layers::{draw_active_menu, draw_menu_bar, draw_status};
use crate::render::window::offset_rect;
use crate::surface::Surface;
use crate::{UiFrame, UiShell, UiWindow};

/// Renders a `UiFrame` onto a `Surface` and returns the terminal cursor
/// position for the focused window, if any — the Surface twin of
/// `render_ui_frame`, which is the layout contract both must satisfy.
///
/// The cursor is returned instead of written because the Surface path has no
/// terminal handle: the caller (the dun-cli cutover slice) appends the CUP
/// and cursor-visibility bytes after the `emit_diff` stream. Window bodies
/// and overlays are the remaining unported layers (slice 3c); their absence
/// does not affect the cursor contract, which is computed from the frame
/// model alone.
pub(crate) fn render_ui_frame_to_surface(
    surface: &mut Surface,
    shell: &UiShell,
    ui_frame: &UiFrame,
) -> Option<(u16, u16)> {
    let width = surface.width();
    let height = surface.height();
    surface.fill_rect(0, 0, width, height, ' ', shell.theme.palette.editor);
    if width == 0 || height == 0 {
        return None;
    }

    draw_menu_bar(surface, shell, &ui_frame.menu, TuiRect::new(0, 0, width, 1));
    if height == 1 {
        return None;
    }

    draw_status(
        surface,
        shell,
        &ui_frame.status,
        TuiRect::new(0, height - 1, width, 1),
    );
    if height <= 2 {
        return None;
    }

    let workspace = TuiRect::new(0, 1, width, height - 2);
    let mut cursor = None;
    for window in &ui_frame.windows {
        // Slice 3c draws the window body here; the cursor contract is final:
        // in window order, the last window reporting a cursor wins, matching
        // the ratatui path's repeated set_cursor_position calls.
        cursor = window_cursor_position(window, workspace).or(cursor);
    }

    draw_active_menu(
        surface,
        shell,
        &ui_frame.menu,
        TuiRect::new(0, 0, width, height),
    );
    // Slice 3c: overlay drawing lands here, after the active menu.

    cursor
}

/// Mirror of the cursor placement at the end of `render_window`: collapsed
/// and degenerate windows never report a cursor, and the position must fall
/// inside the window's clipped area.
fn window_cursor_position(window: &UiWindow, workspace: TuiRect) -> Option<(u16, u16)> {
    let area = offset_rect(window.rect, workspace);
    if area.width == 0 || area.height == 0 {
        return None;
    }
    if window.collapsed || area.width <= 2 || area.height <= 2 {
        return None;
    }

    let cursor = window.cursor?;
    let x = area.x.saturating_add(cursor.x);
    let y = area.y.saturating_add(cursor.y);
    let inside = x < area.x.saturating_add(area.width) && y < area.y.saturating_add(area.height);
    inside.then_some((x, y))
}

#[cfg(test)]
mod tests {
    use dun_core::{BufferId, BufferKind, Rect, TextBuffer, Workspace};
    use dun_term::{Style, StyleAttrs, TerminalColor};

    use super::render_ui_frame_to_surface;
    use crate::surface::Surface;
    use crate::{BufferView, UiShell};

    const INITIAL_STYLE: Style = Style::new(
        TerminalColor::Indexed(254),
        TerminalColor::Indexed(255),
        StyleAttrs::BOLD_REVERSE,
    );

    #[test]
    fn tiny_surfaces_render_without_cursor_or_panic() {
        let shell = UiShell::default();
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
        let buffer_view = BufferView::new(BufferId(1), &buffer);

        for (width, height) in [(0u16, 0u16), (10, 0), (0, 5), (10, 1), (10, 2)] {
            let ui_frame = shell.frame_for_workspace(
                &workspace,
                Rect::new(0, 0, width, height.saturating_sub(2)),
                &[buffer_view],
            );
            let mut surface = Surface::new(width, height, INITIAL_STYLE);

            let cursor = render_ui_frame_to_surface(&mut surface, &shell, &ui_frame);

            assert_eq!(cursor, None, "size {width}x{height}");
        }
    }

    #[test]
    fn frame_reports_focused_window_cursor() {
        let shell = UiShell::default();
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hello\nworld");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let ui_frame =
            shell.frame_for_workspace(&workspace, Rect::new(0, 0, 40, 8), &[buffer_view]);
        let mut surface = Surface::new(40, 10, INITIAL_STYLE);

        let cursor = render_ui_frame_to_surface(&mut surface, &shell, &ui_frame);

        let window = &ui_frame.windows[0];
        let expected = window.cursor.map(|c| {
            // workspace origin is (0, 1); the single window sits at rect 0,0.
            (c.x, 1 + c.y)
        });
        assert!(cursor.is_some());
        assert_eq!(cursor, expected);
    }
}
