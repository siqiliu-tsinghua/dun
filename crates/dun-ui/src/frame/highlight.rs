use dun_core::{Rect, TextRange};

use crate::{
    BufferView, UiSearchMatchLine, UiSelectionLine, UiShell, display_width,
    visible_whitespace_text, wrap_line_segments,
};

impl UiShell {
    pub(super) fn selection_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Vec<UiSelectionLine> {
        let Some(range) = buffer.buffer.selection_range() else {
            return Vec::new();
        };
        let Some(inner_width) = rect.width.checked_sub(2).map(|width| width as usize) else {
            return Vec::new();
        };
        let gutter_width = gutter_width.min(inner_width as u16) as usize;
        let body_width = inner_width.saturating_sub(gutter_width);
        let Some(body_height) = rect.height.checked_sub(2).map(|height| height as usize) else {
            return Vec::new();
        };
        if body_width == 0 || body_height == 0 || range.is_empty() {
            return Vec::new();
        }
        if buffer.wrap {
            return self.selection_for_wrapped_buffer(
                buffer,
                range,
                body_width,
                body_height,
                gutter_width,
            );
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
            if let Some(line) =
                self.selection_line(buffer, line_index, range, body_width, gutter_width)
            {
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
        body_width: usize,
        gutter_width: usize,
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
        if start_column >= end_column {
            return None;
        }

        let visible_byte_start = self.byte_column_for_display_column(line, buffer.first_column);
        if end_column <= visible_byte_start {
            return None;
        }
        let start_column = start_column.max(visible_byte_start);
        let body_origin = self.line_display_column_for_buffer(buffer, line, visible_byte_start)?;
        let last_column = body_origin.saturating_add(body_width);
        let start_display = self.line_display_column_for_buffer(buffer, line, start_column)?;
        let end_display = self.line_display_column_for_buffer(buffer, line, end_column)?;
        if end_display <= body_origin || start_display >= last_column {
            return None;
        }

        let start_x = start_display.saturating_sub(body_origin).min(body_width);
        let end_x = end_display.saturating_sub(body_origin).min(body_width);
        if start_x >= end_x {
            return None;
        }

        Some(UiSelectionLine {
            y: 1 + (line_index - buffer.first_line) as u16,
            start_x: 1 + start_x as u16 + gutter_width as u16,
            end_x: 1 + end_x as u16 + gutter_width as u16,
        })
    }

    fn selection_for_wrapped_buffer(
        &self,
        buffer: &BufferView<'_>,
        range: TextRange,
        body_width: usize,
        body_height: usize,
        gutter_width: usize,
    ) -> Vec<UiSelectionLine> {
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
                    buffer,
                    line,
                    start_column,
                    end_column,
                    visual_y,
                    body_width,
                    body_height,
                    gutter_width,
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
        rect: Rect,
        gutter_width: u16,
    ) -> Vec<UiSearchMatchLine> {
        if buffer.search_matches.is_empty() {
            return Vec::new();
        }
        let Some(inner_width) = rect.width.checked_sub(2).map(|width| width as usize) else {
            return Vec::new();
        };
        let gutter_width = gutter_width.min(inner_width as u16) as usize;
        let body_width = inner_width.saturating_sub(gutter_width);
        let Some(body_height) = rect.height.checked_sub(2).map(|height| height as usize) else {
            return Vec::new();
        };
        if body_width == 0 || body_height == 0 {
            return Vec::new();
        }
        if buffer.wrap {
            return self.search_matches_for_wrapped_buffer(
                buffer,
                body_width,
                body_height,
                gutter_width,
            );
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
            if let Some(line) =
                self.search_match_line(buffer, range, body_width, gutter_width, index)
            {
                lines.push(line);
            }
        }

        lines
    }

    fn search_match_line(
        &self,
        buffer: &BufferView<'_>,
        range: TextRange,
        body_width: usize,
        gutter_width: usize,
        index: usize,
    ) -> Option<UiSearchMatchLine> {
        let line = buffer.buffer.line(range.start.line)?;
        let visible_byte_start = self.byte_column_for_display_column(line, buffer.first_column);
        if range.end.column <= visible_byte_start {
            return None;
        }
        let start_column = range.start.column.max(visible_byte_start);
        let body_origin = self.line_display_column_for_buffer(buffer, line, visible_byte_start)?;
        let last_column = body_origin.saturating_add(body_width);
        let start_display = self.line_display_column_for_buffer(buffer, line, start_column)?;
        let end_display = self.line_display_column_for_buffer(buffer, line, range.end.column)?;
        if end_display <= body_origin || start_display >= last_column {
            return None;
        }

        let start_x = start_display.saturating_sub(body_origin).min(body_width);
        let end_x = end_display.saturating_sub(body_origin).min(body_width);
        if start_x >= end_x {
            return None;
        }

        Some(UiSearchMatchLine {
            y: 1 + (range.start.line - buffer.first_line) as u16,
            start_x: 1 + start_x as u16 + gutter_width as u16,
            end_x: 1 + end_x as u16 + gutter_width as u16,
            active: buffer.active_search_match == Some(index),
        })
    }

    fn search_matches_for_wrapped_buffer(
        &self,
        buffer: &BufferView<'_>,
        body_width: usize,
        body_height: usize,
        gutter_width: usize,
    ) -> Vec<UiSearchMatchLine> {
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
                buffer,
                line,
                range.start.column,
                range.end.column,
                visual_y,
                body_width,
                body_height,
                gutter_width,
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
        buffer: &BufferView<'_>,
        line: &str,
        start_column: usize,
        end_column: usize,
        visual_y: isize,
        body_width: usize,
        body_height: usize,
        gutter_width: usize,
    ) -> Vec<(u16, u16, u16)> {
        if start_column >= end_column {
            return Vec::new();
        }
        let Some(start_display) = self.line_display_column_for_buffer(buffer, line, start_column)
        else {
            return Vec::new();
        };
        let Some(end_display) = self.line_display_column_for_buffer(buffer, line, end_column)
        else {
            return Vec::new();
        };
        if start_display >= end_display {
            return Vec::new();
        }

        let visible = visible_whitespace_text(
            line,
            buffer.show_whitespace,
            self.display_sanitizer.ascii_only,
        );
        let mut spans = Vec::new();
        let mut segment_start = 0usize;
        for (row_offset, segment) in wrap_line_segments(&visible, body_width).iter().enumerate() {
            let row = visual_y.saturating_add(row_offset as isize);
            let segment_width = display_width(segment);
            let segment_end = segment_start.saturating_add(segment_width);
            if row < 0 {
                segment_start = segment_end;
                continue;
            }
            if row >= body_height as isize {
                break;
            }
            let start = start_display.max(segment_start);
            let end = end_display.min(segment_end);
            if start < end {
                spans.push((
                    1 + row as u16,
                    1 + gutter_width as u16 + (start - segment_start) as u16,
                    1 + gutter_width as u16 + (end - segment_start) as u16,
                ));
            }
            segment_start = segment_end;
        }

        spans
    }
}
