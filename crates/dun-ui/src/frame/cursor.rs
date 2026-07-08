use dun_core::Rect;

use crate::{BufferView, UiCursor, UiShell};

impl UiShell {
    pub(super) fn cursor_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Option<UiCursor> {
        let inner_width = rect.width.checked_sub(2)? as usize;
        let gutter_width = gutter_width.min(inner_width as u16) as usize;
        let body_width = inner_width.saturating_sub(gutter_width);
        let body_height = rect.height.checked_sub(2)? as usize;
        if body_width == 0 || body_height == 0 {
            return None;
        }
        if buffer.wrap {
            return self.wrapped_cursor_for_buffer(buffer, gutter_width, body_width, body_height);
        }

        let position = buffer.buffer.cursor_position();
        if position.line < buffer.first_line {
            return None;
        }

        let visible_line = position.line - buffer.first_line;
        if visible_line >= body_height {
            return None;
        }

        let line = buffer.buffer.line(position.line)?;
        let visible_byte_start = self.byte_column_for_display_column(line, buffer.first_column);
        if position.column < visible_byte_start {
            return None;
        }
        let body_origin = self.display_column(line, visible_byte_start)?;
        let display_column = self.display_column(line, position.column)?;
        if display_column < body_origin {
            return None;
        }
        let display_column = display_column
            .saturating_sub(body_origin)
            .min(body_width.saturating_sub(1));

        Some(UiCursor {
            x: 1 + gutter_width as u16 + display_column as u16,
            y: 1 + visible_line as u16,
        })
    }

    fn wrapped_cursor_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        gutter_width: usize,
        body_width: usize,
        body_height: usize,
    ) -> Option<UiCursor> {
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
        let display_column = self.line_display_column_for_buffer(buffer, line, position.column)?;
        let row_offset = display_column / body_width;
        visual_y = visual_y.saturating_add(row_offset as isize);
        if visual_y < 0 || visual_y >= body_height as isize {
            return None;
        }
        let display_column = display_column % body_width;

        Some(UiCursor {
            x: 1 + gutter_width as u16 + display_column as u16,
            y: 1 + visual_y as u16,
        })
    }
}
