use crate::*;
use dun_core::DisplaySanitizer;
use dun_term::GlyphSet;
use dun_ui::{EditorLineDisplay, EditorTextDisplay, EditorVisualRows, ViewportTop, VisibleLine};

pub(crate) trait EditorDisplayArg {
    fn into_editor_text_display(self) -> EditorTextDisplay;
}

impl EditorDisplayArg for EditorTextDisplay {
    fn into_editor_text_display(self) -> EditorTextDisplay {
        self
    }
}

// Kept for the existing direct BufferState tests. Runtime call sites pass the
// shell's fully configured EditorTextDisplay instead.
impl EditorDisplayArg for AmbiguousWidth {
    fn into_editor_text_display(self) -> EditorTextDisplay {
        EditorTextDisplay::new(
            DisplaySanitizer::unlimited_utf8(),
            self,
            GlyphSet::unicode_single_line(),
            false,
        )
    }
}

impl BufferState {
    pub(crate) fn ensure_cursor_visible(
        &mut self,
        body_height: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) {
        let display = display.into_editor_text_display();
        if self.word_wrap {
            self.ensure_cursor_visible_wrapped(body_height, body_width, display);
            return;
        }
        self.first_visual_row = 0;
        let line_map = self.line_display();
        let cursor_line = self.buffer.cursor_position().line;
        let Some(cursor_row) = line_map.placement_for_source_line(cursor_line) else {
            return;
        };
        if body_height == 0 {
            self.first_line = line_map
                .source_anchor_for_visible_row(cursor_row)
                .unwrap_or(cursor_line);
        } else {
            let first_row = line_map
                .placement_for_source_line(self.first_line)
                .unwrap_or(0);
            if cursor_row < first_row {
                self.first_line = line_map
                    .source_anchor_for_visible_row(cursor_row)
                    .unwrap_or(cursor_line);
            } else if cursor_row >= first_row.saturating_add(body_height) {
                let target_row = cursor_row.saturating_sub(body_height - 1);
                self.first_line = line_map
                    .source_anchor_for_visible_row(target_row)
                    .unwrap_or(cursor_line);
            }
        }

        self.ensure_cursor_column_visible(body_width, display);
    }

    pub(crate) fn ensure_cursor_visible_wrapped(
        &mut self,
        body_height: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) {
        let display = display.into_editor_text_display();
        self.first_column = 0;
        let body_width = body_width.max(1);
        let cursor_row = self.wrapped_visual_row_for_position(
            self.buffer.cursor_position(),
            body_width,
            display,
        );
        if body_height == 0 {
            self.set_wrapped_top_visual_row(cursor_row, body_width, display);
            return;
        }

        let top = self.wrapped_top_visual_row(body_width, display);
        let height = body_height.max(1);
        if cursor_row < top {
            self.set_wrapped_top_visual_row(cursor_row, body_width, display);
        } else if cursor_row >= top.saturating_add(height) {
            self.set_wrapped_top_visual_row(
                cursor_row.saturating_sub(height - 1),
                body_width,
                display,
            );
        } else {
            self.normalize_wrapped_top(body_width, display);
        }
    }

    pub(crate) fn move_page_up(&mut self, lines: usize) -> bool {
        let mut moved = false;
        for _ in 0..lines.max(1) {
            let current = self.buffer.cursor_position();
            let target = self.line_display().previous_visible_anchor(current.line);
            if target == current.line.checked_sub(1) || target.is_none() {
                moved |= self.buffer.move_up();
            } else if let Some(target_line) = target {
                let target_column =
                    self.clamp_column_to_line(target_line, self.buffer.cursor().preferred_column);
                moved |= self
                    .buffer
                    .set_cursor(Position::new(target_line, target_column))
                    .is_ok();
            }
        }
        moved
    }

    pub(crate) fn move_page_down(&mut self, lines: usize) -> bool {
        let mut moved = false;
        for _ in 0..lines.max(1) {
            let current = self.buffer.cursor_position();
            let target = self.line_display().next_visible_anchor(current.line);
            if target == current.line.checked_add(1) || target.is_none() {
                moved |= self.buffer.move_down();
            } else if let Some(target_line) = target {
                let target_column =
                    self.clamp_column_to_line(target_line, self.buffer.cursor().preferred_column);
                moved |= self
                    .buffer
                    .set_cursor(Position::new(target_line, target_column))
                    .is_ok();
            }
        }
        moved
    }

    pub(crate) fn move_wrapped_page(
        &mut self,
        direction: isize,
        rows: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> bool {
        let display = display.into_editor_text_display();
        let body_width = body_width.max(1);
        let current = self.buffer.cursor_position();
        let current_row = self.wrapped_visual_row_for_position(current, body_width, display);
        let current_column = self.wrapped_visual_column_for_position(current, body_width, display);
        let max_row = self
            .wrapped_total_visual_rows(body_width, display)
            .saturating_sub(1);
        let target_row = if direction < 0 {
            current_row.saturating_sub(rows.max(1))
        } else {
            current_row.saturating_add(rows.max(1)).min(max_row)
        };
        let target = self.position_for_wrapped_visual_row_column(
            target_row,
            current_column,
            body_width,
            display,
        );
        let moved = target != current;
        let _ = self.buffer.set_cursor(target);
        moved
    }

    pub(crate) fn scroll_view_lines(
        &mut self,
        delta: isize,
        body_height: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> bool {
        let display = display.into_editor_text_display();
        if body_height == 0 || self.buffer.line_count() == 0 {
            return false;
        }
        if self.word_wrap {
            return self.scroll_wrapped_visual_rows(delta, body_height, body_width, display);
        }

        let old_first_line = self.first_line;
        self.first_visual_row = 0;
        let line_map = self.line_display();
        let first_row = line_map
            .placement_for_source_line(self.first_line)
            .unwrap_or(0);
        let max_first_row = line_map
            .visible_row_count()
            .saturating_sub(body_height.max(1));
        let target_row = if delta < 0 {
            first_row.saturating_sub(delta.unsigned_abs())
        } else {
            first_row.saturating_add(delta as usize).min(max_first_row)
        };
        self.first_line = line_map
            .source_anchor_for_visible_row(target_row)
            .unwrap_or(0);

        self.keep_cursor_inside_visible_lines(body_height);
        self.first_line != old_first_line
    }

    pub(crate) fn scroll_view_to_line(
        &mut self,
        first_line: usize,
        first_visual_row: usize,
        body_height: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> bool {
        let display = display.into_editor_text_display();
        if body_height == 0 || self.buffer.line_count() == 0 {
            return false;
        }
        if self.word_wrap {
            let old = (self.first_line, self.first_visual_row);
            let target = self
                .wrapped_visual_row_for_line(first_line, body_width.max(1), display)
                .saturating_add(first_visual_row);
            self.set_wrapped_top_visual_row(target, body_width.max(1), display);
            self.keep_cursor_inside_visible_wrapped_rows(body_height, body_width.max(1), display);
            return old != (self.first_line, self.first_visual_row);
        }

        let old_first_line = self.first_line;
        let line_map = self.line_display();
        let target_row = line_map
            .placement_for_source_line(first_line)
            .unwrap_or_else(|| line_map.visible_row_count().saturating_sub(1));
        let max_first_row = line_map
            .visible_row_count()
            .saturating_sub(body_height.max(1));
        self.first_line = line_map
            .source_anchor_for_visible_row(target_row.min(max_first_row))
            .unwrap_or(0);
        self.first_visual_row = 0;
        self.keep_cursor_inside_visible_lines(body_height);
        self.first_line != old_first_line
    }

    pub(crate) fn scroll_view_columns(
        &mut self,
        delta: isize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> bool {
        let display = display.into_editor_text_display();
        if self.word_wrap {
            self.first_column = 0;
            return false;
        }

        if body_width == 0 {
            return false;
        }

        let old_first_column = self.first_column;
        let max_first_column = self
            .max_line_display_width(display)
            .saturating_sub(body_width.max(1));
        self.first_column = if delta < 0 {
            self.first_column.saturating_sub(delta.unsigned_abs())
        } else {
            self.first_column
                .saturating_add(delta as usize)
                .min(max_first_column)
        };
        self.first_column != old_first_column
    }

    pub(crate) fn extend_page_up(&mut self, lines: usize) -> bool {
        let mut moved = false;
        for _ in 0..lines.max(1) {
            let current = self.buffer.cursor_position();
            let target = self.line_display().previous_visible_anchor(current.line);
            if target == current.line.checked_sub(1) || target.is_none() {
                moved |= self.buffer.extend_selection_up();
            } else if let Some(target_line) = target {
                let anchor = self
                    .buffer
                    .selection()
                    .map(|selection| selection.anchor)
                    .unwrap_or(current);
                let target_column =
                    self.clamp_column_to_line(target_line, self.buffer.cursor().preferred_column);
                let target = Position::new(target_line, target_column);
                moved |= self.buffer.select(anchor, target).is_ok() && target != current;
            }
        }
        moved
    }

    pub(crate) fn extend_page_down(&mut self, lines: usize) -> bool {
        let mut moved = false;
        for _ in 0..lines.max(1) {
            let current = self.buffer.cursor_position();
            let target = self.line_display().next_visible_anchor(current.line);
            if target == current.line.checked_add(1) || target.is_none() {
                moved |= self.buffer.extend_selection_down();
            } else if let Some(target_line) = target {
                let anchor = self
                    .buffer
                    .selection()
                    .map(|selection| selection.anchor)
                    .unwrap_or(current);
                let target_column =
                    self.clamp_column_to_line(target_line, self.buffer.cursor().preferred_column);
                let target = Position::new(target_line, target_column);
                moved |= self.buffer.select(anchor, target).is_ok() && target != current;
            }
        }
        moved
    }

    pub(crate) fn extend_wrapped_page(
        &mut self,
        direction: isize,
        rows: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> bool {
        let display = display.into_editor_text_display();
        let body_width = body_width.max(1);
        let current = self.buffer.cursor_position();
        let current_row = self.wrapped_visual_row_for_position(current, body_width, display);
        let current_column = self.wrapped_visual_column_for_position(current, body_width, display);
        let max_row = self
            .wrapped_total_visual_rows(body_width, display)
            .saturating_sub(1);
        let target_row = if direction < 0 {
            current_row.saturating_sub(rows.max(1))
        } else {
            current_row.saturating_add(rows.max(1)).min(max_row)
        };
        let target = self.position_for_wrapped_visual_row_column(
            target_row,
            current_column,
            body_width,
            display,
        );
        let anchor = self
            .buffer
            .selection()
            .map(|selection| selection.anchor)
            .unwrap_or(current);
        let moved = target != current;
        let _ = self.buffer.select(anchor, target);
        moved
    }

    pub(crate) fn ensure_cursor_column_visible(
        &mut self,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) {
        let display = display.into_editor_text_display();
        if self.word_wrap {
            self.first_column = 0;
            self.normalize_wrapped_top(body_width.max(1), display);
            return;
        }

        let cursor_column = self.cursor_display_column(display);
        if body_width == 0 {
            self.first_column = cursor_column;
            return;
        }

        if cursor_column < self.first_column {
            self.first_column = cursor_column;
        } else if cursor_column >= self.first_column.saturating_add(body_width) {
            self.first_column = cursor_column.saturating_sub(body_width - 1);
        }
    }

    pub(crate) fn cursor_display_column(&self, display: impl EditorDisplayArg) -> usize {
        let display = display.into_editor_text_display();
        let position = self.buffer.cursor_position();
        self.buffer
            .line(position.line)
            .and_then(|line| display.source_byte_to_display_column(line, position.column))
            .unwrap_or(0)
    }

    pub(crate) fn max_line_display_width(&self, display: impl EditorDisplayArg) -> usize {
        let display = display.into_editor_text_display();
        self.buffer
            .lines()
            .iter()
            .map(|line| display.line_display_width(line))
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn scroll_wrapped_visual_rows(
        &mut self,
        delta: isize,
        body_height: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> bool {
        let display = display.into_editor_text_display();
        let body_width = body_width.max(1);
        let old = (self.first_line, self.first_visual_row);
        let top = self.wrapped_top_visual_row(body_width, display);
        let max_top = self
            .wrapped_total_visual_rows(body_width, display)
            .saturating_sub(body_height.max(1));
        let next = if delta < 0 {
            top.saturating_sub(delta.unsigned_abs())
        } else {
            top.saturating_add(delta as usize).min(max_top)
        };
        self.set_wrapped_top_visual_row(next, body_width, display);
        self.keep_cursor_inside_visible_wrapped_rows(body_height, body_width, display);
        old != (self.first_line, self.first_visual_row)
    }

    pub(crate) fn normalize_wrapped_top(
        &mut self,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) {
        let display = display.into_editor_text_display();
        if !self.word_wrap {
            self.first_visual_row = 0;
            return;
        }
        let body_width = body_width.max(1);
        let top = self.wrapped_top_visual_row(body_width, display);
        self.set_wrapped_top_visual_row(top, body_width, display);
    }

    pub(crate) fn wrapped_total_visual_rows(
        &self,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> usize {
        let display = display.into_editor_text_display();
        self.visual_rows(body_width, display).total_rows().max(1)
    }

    pub(crate) fn wrapped_top_visual_row(
        &self,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> usize {
        let display = display.into_editor_text_display();
        let top = ViewportTop::new(
            self.first_line,
            self.first_visual_row.min(
                self.wrapped_line_visual_rows(self.first_line, body_width, display)
                    .saturating_sub(1),
            ),
        );
        self.visual_rows(body_width, display)
            .global_row_for_top(top)
    }

    pub(crate) fn wrapped_visual_row_for_line(
        &self,
        line_index: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> usize {
        let display = display.into_editor_text_display();
        self.visual_rows(body_width, display)
            .global_row_for_position(Position::new(line_index, 0))
    }

    pub(crate) fn set_wrapped_top_visual_row(
        &mut self,
        target_row: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) {
        let display = display.into_editor_text_display();
        let body_width = body_width.max(1);
        let max_row = self
            .wrapped_total_visual_rows(body_width, display)
            .saturating_sub(1);
        let top = self
            .visual_rows(body_width, display)
            .top_for_global_row(target_row.min(max_row));
        self.first_line = top.anchor_line;
        self.first_visual_row = top.wrapped_row;
        self.first_column = 0;
    }

    pub(crate) fn wrapped_visual_row_for_position(
        &self,
        position: Position,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> usize {
        let display = display.into_editor_text_display();
        self.visual_rows(body_width, display)
            .global_row_for_position(position)
    }

    pub(crate) fn wrapped_visual_column_for_position(
        &self,
        position: Position,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> usize {
        self.wrapped_row_column_for_position(position, body_width, display)
            .1
    }

    pub(crate) fn wrapped_row_column_for_position(
        &self,
        position: Position,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> (usize, usize) {
        let display = display.into_editor_text_display();
        let Some(line) = self.buffer.line(position.line) else {
            return (0, 0);
        };
        display
            .wrapped_row_column_for_source_byte(line, position.column, body_width)
            .unwrap_or_else(|| {
                let prefix = line.get(..position.column).unwrap_or(line);
                let mut row = 0usize;
                let mut column = 0usize;
                for ch in prefix.chars() {
                    advance_wrapped_column(
                        &mut row,
                        &mut column,
                        display_width_for_editor_char(ch, display),
                        body_width,
                        display,
                    );
                }
                (row, column)
            })
    }

    pub(crate) fn wrapped_line_visual_rows(
        &self,
        line_index: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> usize {
        let display = display.into_editor_text_display();
        let line_map = self.line_display();
        let Some(row) = line_map.placement_for_source_line(line_index) else {
            return 1;
        };
        match line_map.item_for_visible_row(row) {
            Some(VisibleLine::Source { line }) if line == line_index => self
                .buffer
                .line(line)
                .map(|source| display.wrapped_row_count(source, body_width))
                .unwrap_or(1),
            Some(VisibleLine::Fold { range }) if range.start_line == line_index => 1,
            Some(VisibleLine::Fold { .. }) => 0,
            Some(VisibleLine::Source { .. }) | None => 1,
        }
    }

    pub(crate) fn position_for_wrapped_visual_row(
        &self,
        target_row: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> Position {
        let display = display.into_editor_text_display();
        let body_width = body_width.max(1);
        let visual_rows = self.visual_rows(body_width, display);
        if target_row >= visual_rows.total_rows() {
            return visual_rows.position_for_global_row_column(target_row, 0);
        }
        let top = visual_rows.top_for_global_row(target_row);
        let line_map = self.line_display();
        let Some(row) = line_map.placement_for_source_line(top.anchor_line) else {
            return buffer_end_position(&self.buffer);
        };
        match line_map.item_for_visible_row(row) {
            Some(VisibleLine::Source { line }) => Position::new(
                line,
                byte_column_for_wrapped_row_start(
                    self.buffer.line(line).unwrap_or_default(),
                    top.wrapped_row,
                    body_width,
                    display,
                ),
            ),
            Some(VisibleLine::Fold { range }) => Position::new(range.start_line, 0),
            None => buffer_end_position(&self.buffer),
        }
    }

    pub(crate) fn position_for_wrapped_visual_row_column(
        &self,
        target_row: usize,
        target_column: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) -> Position {
        let display = display.into_editor_text_display();
        let body_width = body_width.max(1);
        let visual_rows = self.visual_rows(body_width, display);
        if target_row >= visual_rows.total_rows() {
            return visual_rows.position_for_global_row_column(target_row, target_column);
        }
        let top = visual_rows.top_for_global_row(target_row);
        let line_map = self.line_display();
        let Some(row) = line_map.placement_for_source_line(top.anchor_line) else {
            return buffer_end_position(&self.buffer);
        };
        match line_map.item_for_visible_row(row) {
            Some(VisibleLine::Source { line }) => Position::new(
                line,
                byte_column_for_wrapped_row_column(
                    self.buffer.line(line).unwrap_or_default(),
                    top.wrapped_row,
                    target_column,
                    body_width,
                    display,
                ),
            ),
            Some(VisibleLine::Fold { range }) => Position::new(range.start_line, 0),
            None => buffer_end_position(&self.buffer),
        }
    }

    pub(crate) fn keep_cursor_inside_visible_wrapped_rows(
        &mut self,
        body_height: usize,
        body_width: usize,
        display: impl EditorDisplayArg,
    ) {
        let display = display.into_editor_text_display();
        if body_height == 0 {
            return;
        }

        let body_width = body_width.max(1);
        let top = self.wrapped_top_visual_row(body_width, display);
        let bottom = top.saturating_add(body_height.saturating_sub(1));
        let cursor_row = self.wrapped_visual_row_for_position(
            self.buffer.cursor_position(),
            body_width,
            display,
        );
        let target_row = cursor_row.clamp(top, bottom);
        if target_row == cursor_row {
            return;
        }

        let _ = self
            .buffer
            .set_cursor(self.position_for_wrapped_visual_row(target_row, body_width, display));
    }

    fn line_display(&self) -> EditorLineDisplay<'_> {
        EditorLineDisplay::new(self.buffer.line_count(), &self.folds)
    }

    fn visual_rows(
        &self,
        body_width: usize,
        display: EditorTextDisplay,
    ) -> EditorVisualRows<'_, '_> {
        EditorVisualRows::new(
            &self.buffer,
            self.line_display(),
            display,
            body_width.max(1),
        )
    }
}

pub(crate) fn editor_body_width(shell: &UiShell, buffer: &BufferState, rect: Rect) -> usize {
    let line_map = EditorLineDisplay::new(buffer.buffer.line_count(), &buffer.folds);
    let geometry =
        shell.window_geometry(rect.width, rect.height, Some(line_map.visible_row_count()));
    debug_assert_eq!(
        usize::from(geometry.border_columns),
        char_width(shell.glyphs.border.vertical, shell.profile.ambiguous_width).unwrap_or(1)
    );
    debug_assert!(
        geometry.gutter.width == 0
            || geometry.inner.width
                >= geometry
                    .gutter
                    .width
                    .saturating_add(MIN_BODY_COLUMNS_WITH_GUTTER)
    );
    usize::from(geometry.body.width)
}
