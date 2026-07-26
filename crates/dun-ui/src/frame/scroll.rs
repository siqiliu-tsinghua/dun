use crate::{BufferView, UiHorizontalEdgeLine, UiScrollbar, UiShell, WindowGeometry};

impl UiShell {
    pub(super) fn scrollbar_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        geometry: WindowGeometry,
    ) -> Option<UiScrollbar> {
        let body_height = usize::from(geometry.body.height);
        if body_height == 0 {
            return None;
        }
        let (total, top) = if buffer.wrap {
            let body_width = usize::from(geometry.body.width).max(1);
            (
                self.wrapped_total_visual_rows(buffer, body_width),
                self.wrapped_top_visual_row(buffer, body_width),
            )
        } else {
            (buffer.buffer.line_count(), buffer.first_line)
        };
        if total <= body_height {
            return None;
        }

        let thumb_height = body_height
            .saturating_mul(body_height)
            .saturating_add(total.saturating_sub(1))
            / total;
        let thumb_height = thumb_height.max(1).min(body_height);
        let max_thumb_top = body_height.saturating_sub(thumb_height);
        let max_first_row = total.saturating_sub(body_height);
        let thumb_top = if max_first_row == 0 {
            0
        } else {
            top.min(max_first_row).saturating_mul(max_thumb_top) / max_first_row
        };

        Some(UiScrollbar {
            y: geometry.body.y.saturating_add(thumb_top as u16),
            height: thumb_height as u16,
        })
    }

    pub(crate) fn scrollbar_target_line_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        geometry: WindowGeometry,
        local_y: u16,
    ) -> Option<(usize, usize)> {
        let body_height = usize::from(geometry.body.height);
        if body_height == 0 {
            return None;
        }
        let body_width = usize::from(geometry.body.width).max(1);
        let total = if buffer.wrap {
            self.wrapped_total_visual_rows(buffer, body_width)
        } else {
            buffer.buffer.line_count()
        };
        if total <= body_height {
            return None;
        }

        let track_y = local_y.saturating_sub(geometry.body.y) as usize;
        let max_track_y = body_height.saturating_sub(1);
        let max_first_row = total.saturating_sub(body_height);
        if max_track_y == 0 {
            return Some((0, 0));
        }

        let target_row = track_y.min(max_track_y).saturating_mul(max_first_row) / max_track_y;
        if buffer.wrap {
            Some(self.wrapped_position_for_top_row(buffer, body_width, target_row))
        } else {
            Some((target_row, 0))
        }
    }
    pub(super) fn horizontal_edges_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        geometry: WindowGeometry,
    ) -> Vec<UiHorizontalEdgeLine> {
        if buffer.wrap {
            return Vec::new();
        }

        let body_width = usize::from(geometry.body.width);
        let body_height = usize::from(geometry.body.height);
        if body_width == 0 || body_height == 0 {
            return Vec::new();
        }

        let display = self.editor_text_display(buffer.visible_whitespace);
        let mut lines = Vec::new();
        for (visible_y, line_index) in (buffer.first_line..buffer.buffer.line_count())
            .take(body_height)
            .enumerate()
        {
            let line = buffer.buffer.line(line_index).unwrap_or_default();
            let width = display.line_display_width(line);
            let visible_byte_start =
                display.display_column_to_source_byte(line, buffer.first_column);
            let body_origin = display
                .source_byte_to_display_column(line, visible_byte_start)
                .unwrap_or(0);
            let left = visible_byte_start > 0;
            let right = width > body_origin.saturating_add(body_width);
            if left || right {
                lines.push(UiHorizontalEdgeLine {
                    y: geometry.body.y.saturating_add(visible_y as u16),
                    left,
                    right,
                });
            }
        }

        lines
    }
}
