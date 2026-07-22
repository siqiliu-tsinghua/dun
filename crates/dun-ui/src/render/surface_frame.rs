use dun_core::Rect as TuiRect;

use crate::render::surface_layers::{draw_active_menu, draw_menu_bar, draw_status};
use crate::render::surface_overlay::draw_overlay;
use crate::render::surface_window::draw_window;
use crate::surface::Surface;
use crate::surface_emit::{emit_diff, emit_full};
use crate::{UiFrame, UiShell};

/// One rendered frame: the terminal bytes to write, and where to place the
/// cursor afterward (if any). The caller writes `bytes` to the terminal, then
/// positions and shows the cursor at `cursor`.
pub struct RenderedFrame {
    pub bytes: Vec<u8>,
    pub cursor: Option<(u16, u16)>,
}

/// The public rendering entry point for the in-house Surface backend. It owns
/// the previously emitted frame and produces a minimal diff against it; the
/// `Surface` grid and the SGR encoder stay private to `dun-ui`. This is the
/// backend the dun-cli event loop drives each frame.
#[derive(Default)]
pub struct SurfaceRenderer {
    previous: Option<Surface>,
}

impl SurfaceRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render `ui_frame` at the given terminal size, returning the bytes to
    /// write and the cursor position. The first frame after construction or
    /// [`invalidate`](Self::invalidate), and any frame whose size differs from
    /// the cached one, is a full repaint; otherwise only changed cells are
    /// emitted.
    pub fn render(
        &mut self,
        shell: &UiShell,
        ui_frame: &UiFrame,
        width: u16,
        height: u16,
    ) -> RenderedFrame {
        let mut surface = Surface::new(width, height, shell.theme.palette.editor)
            .with_ambiguous_width(shell.profile.ambiguous_width);
        let cursor = render_ui_frame_to_surface(&mut surface, shell, ui_frame);

        let mut bytes = Vec::new();
        match &self.previous {
            Some(previous) => emit_diff(previous, &surface, &mut bytes),
            None => emit_full(&surface, &mut bytes),
        }
        self.previous = Some(surface);

        RenderedFrame { bytes, cursor }
    }

    /// Drop the cached frame so the next [`render`](Self::render) repaints in
    /// full. Call after the terminal is cleared or resized out from under the
    /// renderer (e.g. resuming from a shell escape).
    pub fn invalidate(&mut self) {
        self.previous = None;
    }
}

/// Renders a `UiFrame` onto a `Surface` and returns the terminal cursor
/// position, if any.
///
/// The cursor is returned instead of written because the Surface path has no
/// terminal handle: the caller (`SurfaceBackend` in dun-cli) appends the CUP
/// and cursor-visibility bytes after the `emit_diff` stream.
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
        // In window order, the last window reporting a cursor wins.
        cursor = draw_window(surface, shell, window, workspace).or(cursor);
    }

    draw_active_menu(
        surface,
        shell,
        &ui_frame.menu,
        TuiRect::new(0, 0, width, height),
    );
    if let Some(overlay) = &ui_frame.overlay {
        cursor =
            draw_overlay(surface, shell, overlay, TuiRect::new(0, 0, width, height)).or(cursor);
    }

    cursor
}

#[cfg(test)]
mod tests {
    use dun_core::{BufferId, BufferKind, Rect, TextBuffer, Workspace};
    use dun_term::{Style, StyleAttrs, TerminalColor};

    use super::{SurfaceRenderer, render_ui_frame_to_surface};
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

    #[test]
    fn renderer_diffs_and_invalidates() {
        let shell = UiShell::default();
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hello\nworld");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let ui_frame =
            shell.frame_for_workspace(&workspace, Rect::new(0, 0, 40, 8), &[buffer_view]);
        let mut renderer = SurfaceRenderer::new();

        // First frame is a full repaint and reports the cursor.
        let first = renderer.render(&shell, &ui_frame, 40, 10);
        assert!(!first.bytes.is_empty());
        assert!(first.cursor.is_some());

        // An identical frame diffs to nothing.
        let second = renderer.render(&shell, &ui_frame, 40, 10);
        assert!(second.bytes.is_empty());
        assert_eq!(second.cursor, first.cursor);

        // Invalidation forces the next frame to repaint fully again.
        renderer.invalidate();
        let third = renderer.render(&shell, &ui_frame, 40, 10);
        assert_eq!(third.bytes, first.bytes);

        // A size change also repaints fully rather than emitting a stale diff.
        let resized = renderer.render(&shell, &ui_frame, 50, 12);
        assert!(!resized.bytes.is_empty());
    }
}
