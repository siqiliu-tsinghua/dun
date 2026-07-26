use dun_core::SanitizedLine;

use crate::{BufferView, UiShell, WindowGeometry};

impl UiShell {
    pub(super) fn sanitize_buffer_body(
        &self,
        buffer: &BufferView<'_>,
        geometry: WindowGeometry,
    ) -> Vec<SanitizedLine> {
        let body_height = usize::from(geometry.body.height);
        if body_height == 0 {
            return Vec::new();
        }
        if buffer.wrap {
            return self.sanitize_wrapped_buffer_body(buffer, geometry);
        }

        let display = self.editor_text_display(buffer.visible_whitespace);
        let mut lines = Vec::new();
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if lines.len() >= body_height {
                break;
            }

            let line = buffer.buffer.line(line_index).unwrap_or_default();
            let start = display.display_column_to_source_byte(line, buffer.first_column);
            lines.push(display.sanitize_line(&line[start..]));
        }

        lines
    }

    fn sanitize_wrapped_buffer_body(
        &self,
        buffer: &BufferView<'_>,
        geometry: WindowGeometry,
    ) -> Vec<SanitizedLine> {
        let body_height = usize::from(geometry.body.height);
        let body_width = usize::from(geometry.body.width).max(1);
        let display = self.editor_text_display(buffer.visible_whitespace);
        let mut lines = Vec::new();

        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if lines.len() >= body_height {
                break;
            }

            let line = buffer.buffer.line(line_index).unwrap_or_default();
            let start_offset = if line_index == buffer.first_line {
                buffer.first_visual_row.min(
                    self.wrapped_visual_line_count(buffer, line_index, body_width)
                        .saturating_sub(1),
                )
            } else {
                0
            };
            for segment in display
                .wrapped_segments(line, body_width)
                .skip(start_offset)
            {
                if lines.len() >= body_height {
                    break;
                }
                lines.push(display.sanitize_wrapped_segment(segment));
            }
        }

        lines
    }

    pub(crate) fn wrapped_visual_line_count(
        &self,
        buffer: &BufferView<'_>,
        line_index: usize,
        body_width: usize,
    ) -> usize {
        let Some(line) = buffer.buffer.line(line_index) else {
            return 1;
        };
        self.editor_text_display(buffer.visible_whitespace)
            .wrapped_row_count(line, body_width.max(1))
    }

    pub(super) fn wrapped_total_visual_rows(
        &self,
        buffer: &BufferView<'_>,
        body_width: usize,
    ) -> usize {
        (0..buffer.buffer.line_count())
            .map(|line_index| self.wrapped_visual_line_count(buffer, line_index, body_width))
            .sum::<usize>()
            .max(1)
    }

    pub(super) fn wrapped_top_visual_row(
        &self,
        buffer: &BufferView<'_>,
        body_width: usize,
    ) -> usize {
        let previous_rows = (0..buffer.first_line.min(buffer.buffer.line_count()))
            .map(|line_index| self.wrapped_visual_line_count(buffer, line_index, body_width))
            .sum::<usize>();
        let current_rows = self.wrapped_visual_line_count(buffer, buffer.first_line, body_width);
        previous_rows.saturating_add(buffer.first_visual_row.min(current_rows.saturating_sub(1)))
    }

    pub(super) fn wrapped_position_for_top_row(
        &self,
        buffer: &BufferView<'_>,
        body_width: usize,
        target_row: usize,
    ) -> (usize, usize) {
        let mut remaining = target_row;
        for line_index in 0..buffer.buffer.line_count() {
            let rows = self.wrapped_visual_line_count(buffer, line_index, body_width);
            if remaining < rows {
                return (line_index, remaining);
            }
            remaining = remaining.saturating_sub(rows);
        }

        (buffer.buffer.line_count().saturating_sub(1), 0)
    }
}
