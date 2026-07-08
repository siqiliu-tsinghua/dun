use dun_core::Rect;

use crate::{BufferView, UiGutterLine, UiShell, decimal_digits};

const MIN_BODY_COLUMNS_WITH_GUTTER: u16 = 4;

impl UiShell {
    pub(super) fn gutter_width_for_buffer(&self, buffer: &BufferView<'_>, rect: Rect) -> u16 {
        let inner_width = rect.width.saturating_sub(2);
        let digits = decimal_digits(buffer.buffer.line_count().max(1));
        let width = (digits + 1) as u16;
        if inner_width < width.saturating_add(MIN_BODY_COLUMNS_WITH_GUTTER) {
            return 0;
        }

        width
    }

    pub(super) fn gutter_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Vec<UiGutterLine> {
        let body_height = rect.height.saturating_sub(2) as usize;
        if gutter_width == 0 || body_height == 0 {
            return Vec::new();
        }

        let label_digits = gutter_width.saturating_sub(1) as usize;
        let mut lines = Vec::new();
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if lines.len() >= body_height {
                break;
            }

            let marker = if buffer.bookmarks.contains(&line_index) {
                '*'
            } else {
                ' '
            };
            let visual_rows = if buffer.wrap {
                let inner_width = rect.width.saturating_sub(2) as usize;
                let body_width = inner_width.saturating_sub(gutter_width as usize).max(1);
                self.wrapped_visual_line_count(buffer, line_index, body_width)
            } else {
                1
            };
            let start_offset = if buffer.wrap && line_index == buffer.first_line {
                buffer.first_visual_row.min(visual_rows.saturating_sub(1))
            } else {
                0
            };
            for row_offset in start_offset..visual_rows {
                if lines.len() >= body_height {
                    break;
                }
                let label = if row_offset == 0 {
                    format!("{:>label_digits$}{marker}", line_index + 1)
                } else {
                    format!("{:>label_digits$} ", "")
                };
                lines.push(UiGutterLine {
                    y: 1 + lines.len() as u16,
                    label,
                });
            }
        }

        lines
    }
}
