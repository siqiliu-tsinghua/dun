use crate::{BufferView, UiGutterLine, UiShell, WindowGeometry};

impl UiShell {
    pub(super) fn gutter_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        geometry: WindowGeometry,
    ) -> Vec<UiGutterLine> {
        let gutter_height = usize::from(geometry.gutter.height);
        if geometry.gutter.width == 0 || gutter_height == 0 {
            return Vec::new();
        }

        let label_digits = usize::from(
            geometry
                .gutter
                .width
                .saturating_sub(geometry.border_columns),
        );
        let mut lines = Vec::new();
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if lines.len() >= gutter_height {
                break;
            }

            let marker = ' ';
            let visual_rows = if buffer.wrap {
                let body_width = usize::from(geometry.body.width).max(1);
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
                if lines.len() >= gutter_height {
                    break;
                }
                let label = if row_offset == 0 {
                    format!("{:>label_digits$}{marker}", line_index + 1)
                } else {
                    format!("{:>label_digits$} ", "")
                };
                lines.push(UiGutterLine {
                    y: geometry.gutter.y.saturating_add(lines.len() as u16),
                    label,
                });
            }
        }

        lines
    }
}
