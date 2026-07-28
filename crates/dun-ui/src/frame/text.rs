use dun_core::SanitizedLine;

use crate::{BufferView, EditorVisualRows, UiShell, ViewportTop, VisibleLine, WindowGeometry};

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
        let line_map = buffer.line_display();
        let Some(first_row) = line_map.placement_for_source_line(buffer.top.anchor_line) else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        for item in line_map.iter_from_visible_row(first_row) {
            if lines.len() >= body_height {
                break;
            }

            match item {
                VisibleLine::Source { line } => {
                    let source = buffer.buffer.line(line).unwrap_or_default();
                    let start = display.display_column_to_source_byte(source, buffer.first_column);
                    lines.push(display.sanitize_line(&source[start..]));
                }
                VisibleLine::Fold { .. } => {
                    lines.push(self.display_sanitizer.sanitize_line(""));
                }
            }
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
        let line_map = buffer.line_display();
        let Some(first_row) = line_map.placement_for_source_line(buffer.top.anchor_line) else {
            return Vec::new();
        };
        let mut lines = Vec::new();

        for (item_index, item) in line_map.iter_from_visible_row(first_row).enumerate() {
            if lines.len() >= body_height {
                break;
            }

            match item {
                VisibleLine::Source { line } => {
                    let source = buffer.buffer.line(line).unwrap_or_default();
                    let visual_rows = display.wrapped_row_count(source, body_width);
                    let start_offset = if item_index == 0 {
                        buffer.top.wrapped_row.min(visual_rows.saturating_sub(1))
                    } else {
                        0
                    };
                    for segment in display
                        .wrapped_segments(source, body_width)
                        .skip(start_offset)
                    {
                        if lines.len() >= body_height {
                            break;
                        }
                        lines.push(display.sanitize_wrapped_segment(segment));
                    }
                }
                VisibleLine::Fold { .. } => {
                    if item_index != 0 || buffer.top.wrapped_row == 0 {
                        lines.push(self.display_sanitizer.sanitize_line(""));
                    }
                }
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
        let line_map = buffer.line_display();
        let Some(row) = line_map.placement_for_source_line(line_index) else {
            return 1;
        };
        match line_map.item_for_visible_row(row) {
            Some(VisibleLine::Source { line }) if line == line_index => buffer
                .buffer
                .line(line)
                .map(|source| {
                    self.editor_text_display(buffer.visible_whitespace)
                        .wrapped_row_count(source, body_width.max(1))
                })
                .unwrap_or(1),
            Some(VisibleLine::Fold { range }) if range.start_line == line_index => 1,
            Some(VisibleLine::Fold { .. }) => 0,
            Some(VisibleLine::Source { .. }) | None => 1,
        }
    }

    pub(super) fn wrapped_total_visual_rows(
        &self,
        buffer: &BufferView<'_>,
        body_width: usize,
    ) -> usize {
        EditorVisualRows::new(
            buffer.buffer,
            buffer.line_display(),
            self.editor_text_display(buffer.visible_whitespace),
            body_width,
        )
        .total_rows()
        .max(1)
    }

    pub(super) fn wrapped_top_visual_row(
        &self,
        buffer: &BufferView<'_>,
        body_width: usize,
    ) -> usize {
        EditorVisualRows::new(
            buffer.buffer,
            buffer.line_display(),
            self.editor_text_display(buffer.visible_whitespace),
            body_width,
        )
        .global_row_for_top(buffer.top)
    }

    pub(super) fn wrapped_position_for_top_row(
        &self,
        buffer: &BufferView<'_>,
        body_width: usize,
        target_row: usize,
    ) -> ViewportTop {
        EditorVisualRows::new(
            buffer.buffer,
            buffer.line_display(),
            self.editor_text_display(buffer.visible_whitespace),
            body_width,
        )
        .top_for_global_row(target_row)
    }
}
