use ratatui::layout::Rect as TuiRect;

use crate::render::surface_layers::{draw_active_menu, draw_menu_bar, draw_status};
use crate::render::surface_window::draw_window;
use crate::surface::Surface;
use crate::{UiFrame, UiShell};

/// Renders a `UiFrame` onto a `Surface` and returns the terminal cursor
/// position for the focused window, if any — the Surface twin of
/// `render_ui_frame`, which is the layout contract both must satisfy.
///
/// The cursor is returned instead of written because the Surface path has no
/// terminal handle: the caller (the dun-cli cutover slice) appends the CUP
/// and cursor-visibility bytes after the `emit_diff` stream. The overlay is
/// the remaining unported layer.
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
        // In window order, the last window reporting a cursor wins, matching
        // the ratatui path's repeated set_cursor_position calls.
        cursor = draw_window(surface, shell, window, workspace).or(cursor);
    }

    draw_active_menu(
        surface,
        shell,
        &ui_frame.menu,
        TuiRect::new(0, 0, width, height),
    );
    // Slice 3d: overlay drawing lands here, after the active menu.

    cursor
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
