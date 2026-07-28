use dun_core::Rect;

use crate::{BufferView, EditorVisualRows, UiCursor, UiShell, VisibleLine, WindowGeometry};

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
        let line_map = buffer.line_display();
        let position = buffer.buffer.cursor_position();
        let position_row = line_map.placement_for_source_line(position.line)?;
        let first_row = line_map.placement_for_source_line(buffer.top.anchor_line)?;
        let visible_line = position_row.checked_sub(first_row)?;
        if visible_line >= body_height {
            return None;
        }

        if matches!(
            line_map.item_for_visible_row(position_row),
            Some(VisibleLine::Fold { .. })
        ) {
            return Some(UiCursor {
                x: body.x,
                y: body.y.saturating_add(visible_line as u16),
            });
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
        let line_map = buffer.line_display();
        let rows = EditorVisualRows::new(buffer.buffer, line_map, display, body_width);
        let position_row = rows.global_row_for_position(position);
        let top_row = rows.global_row_for_top(buffer.top);
        let visual_y = position_row.checked_sub(top_row)?;
        if visual_y >= body_height {
            return None;
        }
        let display_column = if matches!(
            line_map.item_for_visible_row(line_map.placement_for_source_line(position.line)?),
            Some(VisibleLine::Fold { .. })
        ) {
            0
        } else {
            let line = buffer.buffer.line(position.line)?;
            display
                .wrapped_row_column_for_source_byte(line, position.column, body_width)?
                .1
        };
        Some(UiCursor {
            x: body.x.saturating_add(display_column as u16),
            y: body.y.saturating_add(visual_y as u16),
        })
    }
}
