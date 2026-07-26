use dun_core::{Rect, TextRange};

use crate::{
    BufferView, EditorTextDisplay, UiHighlightLine, UiSearchMatchLine, UiSelectionLine, UiShell,
    WindowGeometry,
};

#[derive(Clone, Copy)]
struct WrappedLineLayout {
    visual_y: isize,
    body: Rect,
}

impl UiShell {
    pub(super) fn selection_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        geometry: WindowGeometry,
    ) -> Vec<UiSelectionLine> {
        let Some(range) = buffer.buffer.selection_range() else {
            return Vec::new();
        };
        let body = geometry.body;
        let body_width = usize::from(body.width);
        let body_height = usize::from(body.height);
        if body_width == 0 || body_height == 0 || range.is_empty() {
            return Vec::new();
        }
        if buffer.wrap {
            return self.selection_for_wrapped_buffer(buffer, range, body);
        }

        let mut lines = Vec::new();
        let visible_start = buffer.first_line;
        let visible_end = buffer.first_line.saturating_add(body_height);
        let start_line = range.start.line.max(visible_start);
        let end_line = range.end.line.min(visible_end.saturating_sub(1));
        if start_line > end_line {
            return Vec::new();
        }

        for line_index in start_line..=end_line {
            if let Some(line) = self.selection_line(buffer, line_index, range, body) {
                lines.push(line);
            }
        }

        lines
    }

    fn selection_line(
        &self,
        buffer: &BufferView<'_>,
        line_index: usize,
        range: TextRange,
        body: Rect,
    ) -> Option<UiSelectionLine> {
        let line = buffer.buffer.line(line_index)?;
        let start_column = if line_index == range.start.line {
            range.start.column
        } else {
            0
        };
        let end_column = if line_index == range.end.line {
            range.end.column
        } else {
            line.len()
        };
        let (y, start_x, end_x) =
            self.body_span_for_columns(buffer, line_index, start_column, end_column, body)?;
        Some(UiSelectionLine { y, start_x, end_x })
    }

    /// Maps a byte-column range on one logical line to body-relative
    /// window coordinates, clipped to the horizontal viewport. Shared by
    /// selection, search-match, and plugin-highlight mapping.
    fn body_span_for_columns(
        &self,
        buffer: &BufferView<'_>,
        line_index: usize,
        start_column: usize,
        end_column: usize,
        body: Rect,
    ) -> Option<(u16, u16, u16)> {
        let body_width = usize::from(body.width);
        let line = buffer.buffer.line(line_index)?;
        let display = self.editor_text_display(buffer.visible_whitespace);
        if start_column >= end_column {
            return None;
        }

        let visible_byte_start = display.display_column_to_source_byte(line, buffer.first_column);
        if end_column <= visible_byte_start {
            return None;
        }
        let start_column = start_column.max(visible_byte_start);
        let body_origin = display.source_byte_to_display_column(line, visible_byte_start)?;
        let last_column = body_origin.saturating_add(body_width);
        let start_display = display.source_byte_to_display_column(line, start_column)?;
        let end_display = display.source_byte_to_display_column(line, end_column)?;
        if end_display <= body_origin || start_display >= last_column {
            return None;
        }

        let start_x = start_display.saturating_sub(body_origin).min(body_width);
        let end_x = end_display.saturating_sub(body_origin).min(body_width);
        if start_x >= end_x {
            return None;
        }

        Some((
            body.y
                .saturating_add((line_index - buffer.first_line) as u16),
            body.x.saturating_add(start_x as u16),
            body.x.saturating_add(end_x as u16),
        ))
    }

    pub(super) fn plugin_highlights_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        geometry: WindowGeometry,
    ) -> Vec<UiHighlightLine> {
        if buffer.highlights.is_empty() {
            return Vec::new();
        }
        let body = geometry.body;
        let body_width = usize::from(body.width);
        let body_height = usize::from(body.height);
        if body_width == 0 || body_height == 0 {
            return Vec::new();
        }
        if buffer.wrap {
            return self.plugin_highlights_for_wrapped_buffer(buffer, body);
        }

        let visible_start = buffer.first_line;
        let visible_end = buffer.first_line.saturating_add(body_height);
        let mut lines = Vec::new();
        for span in buffer.highlights {
            if span.line < visible_start || span.line >= visible_end {
                continue;
            }
            if let Some((y, start_x, end_x)) = self.body_span_for_columns(
                buffer,
                span.line,
                span.start_column,
                span.end_column,
                body,
            ) {
                lines.push(UiHighlightLine {
                    y,
                    start_x,
                    end_x,
                    class: span.class,
                });
            }
        }

        lines
    }

    fn plugin_highlights_for_wrapped_buffer(
        &self,
        buffer: &BufferView<'_>,
        body: Rect,
    ) -> Vec<UiHighlightLine> {
        let body_width = usize::from(body.width);
        let body_height = usize::from(body.height);
        let display = self.editor_text_display(buffer.visible_whitespace);
        let mut lines = Vec::new();
        let mut visual_y = -(buffer.first_visual_row as isize);
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if visual_y >= body_height as isize {
                break;
            }
            let visual_rows = self.wrapped_visual_line_count(buffer, line_index, body_width);
            let Some(line) = buffer.buffer.line(line_index) else {
                visual_y = visual_y.saturating_add(visual_rows as isize);
                continue;
            };
            for span in buffer
                .highlights
                .iter()
                .filter(|span| span.line == line_index)
            {
                for (y, start_x, end_x) in self.wrapped_highlight_spans(
                    line,
                    span.start_column,
                    span.end_column,
                    WrappedLineLayout { visual_y, body },
                    display,
                ) {
                    lines.push(UiHighlightLine {
                        y,
                        start_x,
                        end_x,
                        class: span.class,
                    });
                }
            }
            visual_y = visual_y.saturating_add(visual_rows as isize);
        }

        lines
    }

    fn selection_for_wrapped_buffer(
        &self,
        buffer: &BufferView<'_>,
        range: TextRange,
        body: Rect,
    ) -> Vec<UiSelectionLine> {
        let body_width = usize::from(body.width);
        let body_height = usize::from(body.height);
        let display = self.editor_text_display(buffer.visible_whitespace);
        let mut lines = Vec::new();
        let mut visual_y = -(buffer.first_visual_row as isize);
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if visual_y >= body_height as isize {
                break;
            }
            let visual_rows = self.wrapped_visual_line_count(buffer, line_index, body_width);
            if line_index >= range.start.line && line_index <= range.end.line {
                let Some(line) = buffer.buffer.line(line_index) else {
                    visual_y = visual_y.saturating_add(visual_rows as isize);
                    continue;
                };
                let start_column = if line_index == range.start.line {
                    range.start.column
                } else {
                    0
                };
                let end_column = if line_index == range.end.line {
                    range.end.column
                } else {
                    line.len()
                };
                for (y, start_x, end_x) in self.wrapped_highlight_spans(
                    line,
                    start_column,
                    end_column,
                    WrappedLineLayout { visual_y, body },
                    display,
                ) {
                    lines.push(UiSelectionLine { y, start_x, end_x });
                }
            }
            visual_y = visual_y.saturating_add(visual_rows as isize);
        }

        lines
    }

    pub(super) fn search_matches_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        geometry: WindowGeometry,
    ) -> Vec<UiSearchMatchLine> {
        if buffer.search_matches.is_empty() {
            return Vec::new();
        }
        let body = geometry.body;
        let body_width = usize::from(body.width);
        let body_height = usize::from(body.height);
        if body_width == 0 || body_height == 0 {
            return Vec::new();
        }
        if buffer.wrap {
            return self.search_matches_for_wrapped_buffer(buffer, body);
        }

        let visible_start = buffer.first_line;
        let visible_end = buffer.first_line.saturating_add(body_height);
        let mut lines = Vec::new();
        for (index, item) in buffer.search_matches.iter().enumerate() {
            let range = item.range;
            if range.is_empty() || range.start.line != range.end.line {
                continue;
            }
            if range.start.line < visible_start || range.start.line >= visible_end {
                continue;
            }
            if let Some(line) = self.search_match_line(buffer, range, body, index) {
                lines.push(line);
            }
        }

        lines
    }

    fn search_match_line(
        &self,
        buffer: &BufferView<'_>,
        range: TextRange,
        body: Rect,
        index: usize,
    ) -> Option<UiSearchMatchLine> {
        let body_width = usize::from(body.width);
        let line = buffer.buffer.line(range.start.line)?;
        let display = self.editor_text_display(buffer.visible_whitespace);
        let visible_byte_start = display.display_column_to_source_byte(line, buffer.first_column);
        if range.end.column <= visible_byte_start {
            return None;
        }
        let start_column = range.start.column.max(visible_byte_start);
        let body_origin = display.source_byte_to_display_column(line, visible_byte_start)?;
        let last_column = body_origin.saturating_add(body_width);
        let start_display = display.source_byte_to_display_column(line, start_column)?;
        let end_display = display.source_byte_to_display_column(line, range.end.column)?;
        if end_display <= body_origin || start_display >= last_column {
            return None;
        }

        let start_x = start_display.saturating_sub(body_origin).min(body_width);
        let end_x = end_display.saturating_sub(body_origin).min(body_width);
        if start_x >= end_x {
            return None;
        }

        Some(UiSearchMatchLine {
            y: body
                .y
                .saturating_add((range.start.line - buffer.first_line) as u16),
            start_x: body.x.saturating_add(start_x as u16),
            end_x: body.x.saturating_add(end_x as u16),
            active: buffer.active_search_match == Some(index),
        })
    }

    fn search_matches_for_wrapped_buffer(
        &self,
        buffer: &BufferView<'_>,
        body: Rect,
    ) -> Vec<UiSearchMatchLine> {
        let body_width = usize::from(body.width);
        let body_height = usize::from(body.height);
        let display = self.editor_text_display(buffer.visible_whitespace);
        let mut first_visible_row_by_line = Vec::new();
        let mut visual_y = -(buffer.first_visual_row as isize);
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if visual_y >= body_height as isize {
                break;
            }
            first_visible_row_by_line.push((line_index, visual_y));
            visual_y = visual_y.saturating_add(
                self.wrapped_visual_line_count(buffer, line_index, body_width) as isize,
            );
        }

        let mut lines = Vec::new();
        for (index, item) in buffer.search_matches.iter().enumerate() {
            let range = item.range;
            if range.is_empty() || range.start.line != range.end.line {
                continue;
            }
            let Some((_, visual_y)) = first_visible_row_by_line
                .iter()
                .find(|(line_index, _)| *line_index == range.start.line)
                .copied()
            else {
                continue;
            };
            let Some(line) = buffer.buffer.line(range.start.line) else {
                continue;
            };
            for (y, start_x, end_x) in self.wrapped_highlight_spans(
                line,
                range.start.column,
                range.end.column,
                WrappedLineLayout { visual_y, body },
                display,
            ) {
                lines.push(UiSearchMatchLine {
                    y,
                    start_x,
                    end_x,
                    active: buffer.active_search_match == Some(index),
                });
            }
        }

        lines
    }

    fn wrapped_highlight_spans(
        &self,
        line: &str,
        start_column: usize,
        end_column: usize,
        layout: WrappedLineLayout,
        display: EditorTextDisplay,
    ) -> Vec<(u16, u16, u16)> {
        if start_column >= end_column {
            return Vec::new();
        }
        if display
            .source_byte_to_display_column(line, start_column)
            .is_none()
            || display
                .source_byte_to_display_column(line, end_column)
                .is_none()
        {
            return Vec::new();
        }

        let mut spans = Vec::new();
        for (row_offset, segment) in display
            .wrapped_segments(line, usize::from(layout.body.width))
            .enumerate()
        {
            let row = layout.visual_y.saturating_add(row_offset as isize);
            if row < 0 {
                continue;
            }
            if row >= layout.body.height as isize {
                break;
            }
            let start = start_column.max(segment.start_byte());
            let end = end_column.min(segment.end_byte());
            if start < end {
                let local_start = start.saturating_sub(segment.start_byte());
                let local_end = end.saturating_sub(segment.start_byte());
                let Some(start_x) =
                    display.source_byte_to_display_column(segment.source(), local_start)
                else {
                    continue;
                };
                let Some(end_x) =
                    display.source_byte_to_display_column(segment.source(), local_end)
                else {
                    continue;
                };
                if start_x >= end_x {
                    continue;
                }
                spans.push((
                    layout.body.y.saturating_add(row as u16),
                    layout.body.x.saturating_add(start_x as u16),
                    layout.body.x.saturating_add(end_x as u16),
                ));
            }
        }

        spans
    }
}
