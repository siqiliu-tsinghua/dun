use dun_core::{DisplaySanitizer, Rect, SanitizedLine};
use dun_term::str_width;

use crate::{BufferView, UiShell, wrap_line_segments};

impl UiShell {
    pub(super) fn sanitize_buffer_body(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Vec<SanitizedLine> {
        let body_height = rect.height.saturating_sub(2) as usize;
        if body_height == 0 {
            return Vec::new();
        }
        if buffer.wrap {
            return self.sanitize_wrapped_buffer_body(buffer, rect, gutter_width);
        }

        let mut lines = Vec::new();
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if lines.len() >= body_height {
                break;
            }

            let line = buffer.buffer.line(line_index).unwrap_or_default();
            let start = self.byte_column_for_display_column(line, buffer.first_column);
            lines.push(self.display_sanitizer.sanitize_line(&line[start..]));
        }

        lines
    }

    fn sanitize_wrapped_buffer_body(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Vec<SanitizedLine> {
        let body_height = rect.height.saturating_sub(2) as usize;
        let inner_width = rect.width.saturating_sub(2) as usize;
        let body_width = inner_width.saturating_sub(gutter_width as usize).max(1);
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
            for segment in wrap_line_segments(line, body_width, self.profile.ambiguous_width)
                .iter()
                .skip(start_offset)
            {
                if lines.len() >= body_height {
                    break;
                }
                lines.push(self.display_sanitizer.sanitize_line(segment));
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
        wrap_line_segments(line, body_width.max(1), self.profile.ambiguous_width)
            .len()
            .max(1)
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
    pub(super) fn display_column(&self, line: &str, byte_column: usize) -> Option<usize> {
        let prefix = line.get(..byte_column)?;
        let display_sanitizer = DisplaySanitizer {
            ascii_only: self.display_sanitizer.ascii_only,
            max_bytes: usize::MAX,
        };
        let display_text = display_sanitizer.sanitize_line(prefix).as_plain_text();
        Some(str_width(
            display_text.as_str(),
            self.profile.ambiguous_width,
        ))
    }

    pub(crate) fn byte_column_for_display_column(&self, line: &str, target: usize) -> usize {
        if target == 0 {
            return 0;
        }

        let display_sanitizer = DisplaySanitizer {
            ascii_only: self.display_sanitizer.ascii_only,
            max_bytes: usize::MAX,
        };
        let mut width = 0usize;
        for (index, ch) in line.char_indices() {
            let next_index = index + ch.len_utf8();
            let mut raw = [0; 4];
            let rendered = display_sanitizer
                .sanitize_line(ch.encode_utf8(&mut raw))
                .as_plain_text();
            width =
                width.saturating_add(str_width(rendered.as_str(), self.profile.ambiguous_width));
            if width >= target {
                return next_index;
            }
        }

        line.len()
    }
}
