use dun_core::Rect;

use crate::{BufferView, UiCursor, UiShell, WindowGeometry};

impl UiShell {
    pub(super) fn cursor_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        geometry: WindowGeometry,
    ) -> Option<UiCursor> {
        let body = geometry.body;
        let body_width = usize::from(body.width);
        let body_height = usize::from(body.height);
        if body_width == 0 || body_height == 0 {
            return None;
        }
        if buffer.wrap {
            return self.wrapped_cursor_for_buffer(buffer, body);
        }

        let display = self.editor_text_display(buffer.visible_whitespace);
        let position = buffer.buffer.cursor_position();
        if position.line < buffer.first_line {
            return None;
        }

        let visible_line = position.line - buffer.first_line;
        if visible_line >= body_height {
            return None;
        }

        let line = buffer.buffer.line(position.line)?;
        let visible_byte_start = display.display_column_to_source_byte(line, buffer.first_column);
        if position.column < visible_byte_start {
            return None;
        }
        let body_origin = display.source_byte_to_display_column(line, visible_byte_start)?;
        let display_column = display.source_byte_to_display_column(line, position.column)?;
        if display_column < body_origin {
            return None;
        }
        let display_column = display_column
            .saturating_sub(body_origin)
            .min(body_width.saturating_sub(1));

        Some(UiCursor {
            x: body.x.saturating_add(display_column as u16),
            y: body.y.saturating_add(visible_line as u16),
        })
    }

    fn wrapped_cursor_for_buffer(&self, buffer: &BufferView<'_>, body: Rect) -> Option<UiCursor> {
        let body_width = usize::from(body.width);
        let body_height = usize::from(body.height);
        let display = self.editor_text_display(buffer.visible_whitespace);
        let position = buffer.buffer.cursor_position();
        if position.line < buffer.first_line {
            return None;
        }

        let mut visual_y = -(buffer.first_visual_row as isize);
        for line_index in buffer.first_line..position.line {
            visual_y = visual_y.saturating_add(
                self.wrapped_visual_line_count(buffer, line_index, body_width) as isize,
            );
            if visual_y >= body_height as isize {
                return None;
            }
        }

        let line = buffer.buffer.line(position.line)?;
        let (row_offset, display_column) =
            display.wrapped_row_column_for_source_byte(line, position.column, body_width)?;
        visual_y = visual_y.saturating_add(row_offset as isize);
        if visual_y < 0 || visual_y >= body_height as isize {
            return None;
        }
        Some(UiCursor {
            x: body.x.saturating_add(display_column as u16),
            y: body.y.saturating_add(visual_y as u16),
        })
    }
}
