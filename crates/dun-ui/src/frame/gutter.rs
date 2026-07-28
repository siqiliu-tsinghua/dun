use crate::{BufferView, UiGutterLine, UiShell, VisibleLine, WindowGeometry};

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
        let line_map = buffer.line_display();
        let Some(first_row) = line_map.placement_for_source_line(buffer.top.anchor_line) else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        for (item_index, item) in line_map.iter_from_visible_row(first_row).enumerate() {
            if lines.len() >= gutter_height {
                break;
            }

            let line_index = match item {
                VisibleLine::Source { line } => line,
                VisibleLine::Fold { range } => range.start_line,
            };
            let bookmarked = buffer.bookmarks.contains(&line_index);
            let visual_rows = match item {
                VisibleLine::Source { .. } if buffer.wrap => {
                    let body_width = usize::from(geometry.body.width).max(1);
                    self.wrapped_visual_line_count(buffer, line_index, body_width)
                }
                VisibleLine::Source { .. } | VisibleLine::Fold { .. } => 1,
            };
            let start_offset = if buffer.wrap && item_index == 0 {
                buffer.top.wrapped_row.min(visual_rows.saturating_sub(1))
            } else {
                0
            };
            for row_offset in start_offset..visual_rows {
                if lines.len() >= gutter_height {
                    break;
                }
                let marked = bookmarked && row_offset == 0;
                let label = if row_offset == 0 {
                    let marker = if marked {
                        self.glyphs.indicators.bookmark
                    } else {
                        ' '
                    };
                    format!("{:>label_digits$}{marker}", line_index + 1)
                } else {
                    format!("{:>label_digits$} ", "")
                };
                lines.push(UiGutterLine {
                    y: geometry.gutter.y.saturating_add(lines.len() as u16),
                    label,
                    marked,
                });
            }
        }

        lines
    }
}
