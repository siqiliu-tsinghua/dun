use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferViewContext {
    pub(crate) buffer_id: BufferId,
    pub(crate) body_height: usize,
    pub(crate) body_width: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BufferHighlight {
    pub(crate) revision: u64,
    pub(crate) spans: Vec<BufferHighlightSpan>,
}

pub(crate) struct BufferState {
    pub(crate) id: BufferId,
    pub(crate) buffer: TextBuffer,
    pub(crate) path: Option<PathBuf>,
    pub(crate) encoding: FileTextEncoding,
    pub(crate) file_snapshot: Option<FileReadSnapshot>,
    pub(crate) first_line: usize,
    pub(crate) first_visual_row: usize,
    pub(crate) first_column: usize,
    pub(crate) search: Option<BufferSearchState>,
    pub(crate) word_wrap: bool,
    pub(crate) highlight: Option<BufferHighlight>,
}

impl BufferState {
    pub(crate) fn new(id: BufferId, buffer: TextBuffer) -> Self {
        Self {
            id,
            buffer,
            path: None,
            encoding: FileTextEncoding::Utf8,
            file_snapshot: None,
            first_line: 0,
            first_visual_row: 0,
            first_column: 0,
            search: None,
            word_wrap: false,
            highlight: None,
        }
    }

    pub(crate) fn from_file(id: BufferId, path: PathBuf, loaded: LoadedTextBuffer) -> Self {
        Self {
            id,
            buffer: loaded.buffer,
            path: Some(path),
            encoding: loaded.encoding,
            file_snapshot: loaded.snapshot,
            first_line: 0,
            first_visual_row: 0,
            first_column: 0,
            search: None,
            word_wrap: false,
            highlight: None,
        }
    }

    pub(crate) fn set_search(
        &mut self,
        spec: SearchSpec,
        matches: Vec<SearchMatch>,
        active_index: Option<usize>,
    ) {
        let active_index = active_index.filter(|index| *index < matches.len());
        self.search = Some(BufferSearchState {
            spec,
            matches,
            revision: self.buffer.revision(),
            active_index,
        });
    }

    pub(crate) fn refresh_search_cache(&mut self) {
        if let Some(search) = &mut self.search {
            search.refresh(&self.buffer);
        }
    }

    pub(crate) fn search_status(&self) -> Option<String> {
        let search = self.search.as_ref()?;
        (search.revision == self.buffer.revision()).then(|| search.status_text())
    }

    pub(crate) fn ensure_cursor_visible(
        &mut self,
        body_height: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) {
        if self.word_wrap {
            self.ensure_cursor_visible_wrapped(body_height, body_width, mode);
            return;
        }
        self.first_visual_row = 0;
        if body_height == 0 {
            self.first_line = self.buffer.cursor_position().line;
        } else {
            let cursor_line = self.buffer.cursor_position().line;
            if cursor_line < self.first_line {
                self.first_line = cursor_line;
            } else if cursor_line >= self.first_line.saturating_add(body_height) {
                self.first_line = cursor_line.saturating_sub(body_height - 1);
            }
        }

        self.ensure_cursor_column_visible(body_width, mode);
    }

    pub(crate) fn ensure_cursor_visible_wrapped(
        &mut self,
        body_height: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) {
        self.first_column = 0;
        let body_width = body_width.max(1);
        let cursor_row =
            self.wrapped_visual_row_for_position(self.buffer.cursor_position(), body_width, mode);
        if body_height == 0 {
            self.set_wrapped_top_visual_row(cursor_row, body_width, mode);
            return;
        }

        let top = self.wrapped_top_visual_row(body_width, mode);
        let height = body_height.max(1);
        if cursor_row < top {
            self.set_wrapped_top_visual_row(cursor_row, body_width, mode);
        } else if cursor_row >= top.saturating_add(height) {
            self.set_wrapped_top_visual_row(
                cursor_row.saturating_sub(height - 1),
                body_width,
                mode,
            );
        } else {
            self.normalize_wrapped_top(body_width, mode);
        }
    }

    pub(crate) fn move_page_up(&mut self, lines: usize) -> bool {
        let mut moved = false;
        for _ in 0..lines.max(1) {
            moved |= self.buffer.move_up();
        }
        moved
    }

    pub(crate) fn move_page_down(&mut self, lines: usize) -> bool {
        let mut moved = false;
        for _ in 0..lines.max(1) {
            moved |= self.buffer.move_down();
        }
        moved
    }

    pub(crate) fn move_wrapped_page(
        &mut self,
        direction: isize,
        rows: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> bool {
        let body_width = body_width.max(1);
        let current = self.buffer.cursor_position();
        let current_row = self.wrapped_visual_row_for_position(current, body_width, mode);
        let current_column = self.wrapped_visual_column_for_position(current, body_width, mode);
        let max_row = self
            .wrapped_total_visual_rows(body_width, mode)
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
            mode,
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
        mode: AmbiguousWidth,
    ) -> bool {
        if body_height == 0 || self.buffer.line_count() == 0 {
            return false;
        }
        if self.word_wrap {
            return self.scroll_wrapped_visual_rows(delta, body_height, body_width, mode);
        }

        let old_first_line = self.first_line;
        self.first_visual_row = 0;
        let max_first_line = self.buffer.line_count().saturating_sub(body_height.max(1));
        self.first_line = if delta < 0 {
            self.first_line.saturating_sub(delta.unsigned_abs())
        } else {
            self.first_line
                .saturating_add(delta as usize)
                .min(max_first_line)
        };

        self.keep_cursor_inside_visible_lines(body_height);
        self.first_line != old_first_line
    }

    pub(crate) fn scroll_view_to_line(
        &mut self,
        first_line: usize,
        first_visual_row: usize,
        body_height: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> bool {
        if body_height == 0 || self.buffer.line_count() == 0 {
            return false;
        }
        if self.word_wrap {
            let old = (self.first_line, self.first_visual_row);
            let target = self
                .wrapped_visual_row_for_line(first_line, body_width.max(1), mode)
                .saturating_add(first_visual_row);
            self.set_wrapped_top_visual_row(target, body_width.max(1), mode);
            self.keep_cursor_inside_visible_wrapped_rows(body_height, body_width.max(1), mode);
            return old != (self.first_line, self.first_visual_row);
        }

        let old_first_line = self.first_line;
        let max_first_line = self.buffer.line_count().saturating_sub(body_height.max(1));
        self.first_line = first_line.min(max_first_line);
        self.first_visual_row = 0;
        self.keep_cursor_inside_visible_lines(body_height);
        self.first_line != old_first_line
    }

    pub(crate) fn scroll_view_columns(
        &mut self,
        delta: isize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> bool {
        if self.word_wrap {
            self.first_column = 0;
            return false;
        }

        if body_width == 0 {
            return false;
        }

        let old_first_column = self.first_column;
        let max_first_column = self
            .max_line_display_width(mode)
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
            moved |= self.buffer.extend_selection_up();
        }
        moved
    }

    pub(crate) fn extend_page_down(&mut self, lines: usize) -> bool {
        let mut moved = false;
        for _ in 0..lines.max(1) {
            moved |= self.buffer.extend_selection_down();
        }
        moved
    }

    pub(crate) fn extend_wrapped_page(
        &mut self,
        direction: isize,
        rows: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> bool {
        let body_width = body_width.max(1);
        let current = self.buffer.cursor_position();
        let current_row = self.wrapped_visual_row_for_position(current, body_width, mode);
        let current_column = self.wrapped_visual_column_for_position(current, body_width, mode);
        let max_row = self
            .wrapped_total_visual_rows(body_width, mode)
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
            mode,
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

    pub(crate) fn ensure_cursor_column_visible(&mut self, body_width: usize, mode: AmbiguousWidth) {
        if self.word_wrap {
            self.first_column = 0;
            self.normalize_wrapped_top(body_width.max(1), mode);
            return;
        }

        let cursor_column = self.cursor_display_column(mode);
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

    pub(crate) fn cursor_display_column(&self, mode: AmbiguousWidth) -> usize {
        let position = self.buffer.cursor_position();
        self.buffer
            .line(position.line)
            .and_then(|line| line.get(..position.column))
            .map(|prefix| str_width(prefix, mode))
            .unwrap_or(0)
    }

    pub(crate) fn max_line_display_width(&self, mode: AmbiguousWidth) -> usize {
        self.buffer
            .lines()
            .iter()
            .map(|line| str_width(line.as_str(), mode))
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn scroll_wrapped_visual_rows(
        &mut self,
        delta: isize,
        body_height: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> bool {
        let body_width = body_width.max(1);
        let old = (self.first_line, self.first_visual_row);
        let top = self.wrapped_top_visual_row(body_width, mode);
        let max_top = self
            .wrapped_total_visual_rows(body_width, mode)
            .saturating_sub(body_height.max(1));
        let next = if delta < 0 {
            top.saturating_sub(delta.unsigned_abs())
        } else {
            top.saturating_add(delta as usize).min(max_top)
        };
        self.set_wrapped_top_visual_row(next, body_width, mode);
        self.keep_cursor_inside_visible_wrapped_rows(body_height, body_width, mode);
        old != (self.first_line, self.first_visual_row)
    }

    pub(crate) fn normalize_wrapped_top(&mut self, body_width: usize, mode: AmbiguousWidth) {
        if !self.word_wrap {
            self.first_visual_row = 0;
            return;
        }
        let body_width = body_width.max(1);
        let top = self.wrapped_top_visual_row(body_width, mode);
        self.set_wrapped_top_visual_row(top, body_width, mode);
    }

    pub(crate) fn wrapped_total_visual_rows(
        &self,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> usize {
        (0..self.buffer.line_count())
            .map(|line_index| self.wrapped_line_visual_rows(line_index, body_width, mode))
            .sum::<usize>()
            .max(1)
    }

    pub(crate) fn wrapped_top_visual_row(&self, body_width: usize, mode: AmbiguousWidth) -> usize {
        self.wrapped_visual_row_for_line(self.first_line, body_width, mode)
            .saturating_add(
                self.first_visual_row.min(
                    self.wrapped_line_visual_rows(self.first_line, body_width, mode)
                        .saturating_sub(1),
                ),
            )
    }

    pub(crate) fn wrapped_visual_row_for_line(
        &self,
        line_index: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> usize {
        (0..line_index.min(self.buffer.line_count()))
            .map(|line| self.wrapped_line_visual_rows(line, body_width, mode))
            .sum()
    }

    pub(crate) fn set_wrapped_top_visual_row(
        &mut self,
        target_row: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) {
        let body_width = body_width.max(1);
        let max_row = self
            .wrapped_total_visual_rows(body_width, mode)
            .saturating_sub(1);
        let mut remaining = target_row.min(max_row);
        for line_index in 0..self.buffer.line_count() {
            let rows = self.wrapped_line_visual_rows(line_index, body_width, mode);
            if remaining < rows {
                self.first_line = line_index;
                self.first_visual_row = remaining;
                self.first_column = 0;
                return;
            }
            remaining = remaining.saturating_sub(rows);
        }

        self.first_line = self.buffer.line_count().saturating_sub(1);
        self.first_visual_row = 0;
        self.first_column = 0;
    }

    pub(crate) fn wrapped_visual_row_for_position(
        &self,
        position: Position,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> usize {
        self.wrapped_visual_row_for_line(position.line, body_width, mode)
            .saturating_add(self.wrapped_row_offset_for_position(position, body_width, mode))
    }

    pub(crate) fn wrapped_row_offset_for_position(
        &self,
        position: Position,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> usize {
        self.wrapped_row_column_for_position(position, body_width, mode)
            .0
    }

    pub(crate) fn wrapped_visual_column_for_position(
        &self,
        position: Position,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> usize {
        self.wrapped_row_column_for_position(position, body_width, mode)
            .1
    }

    pub(crate) fn wrapped_row_column_for_position(
        &self,
        position: Position,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> (usize, usize) {
        let body_width = body_width.max(1);
        let Some(line) = self.buffer.line(position.line) else {
            return (0, 0);
        };
        let prefix = line.get(..position.column).unwrap_or(line);
        let mut row = 0usize;
        let mut column = 0usize;
        for ch in prefix.chars() {
            advance_wrapped_column(
                &mut row,
                &mut column,
                display_width_for_editor_char(ch, mode),
                body_width,
            );
        }
        if column >= body_width && position.column < line.len() {
            row = row.saturating_add(1);
            column = 0;
        }
        (row, column)
    }

    pub(crate) fn wrapped_line_visual_rows(
        &self,
        line_index: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> usize {
        let body_width = body_width.max(1);
        let Some(line) = self.buffer.line(line_index) else {
            return 1;
        };
        let mut row = 0usize;
        let mut column = 0usize;
        if line.is_empty() {
            return 1;
        }
        for ch in line.chars() {
            advance_wrapped_column(
                &mut row,
                &mut column,
                display_width_for_editor_char(ch, mode),
                body_width,
            );
        }
        row.saturating_add(1)
    }

    pub(crate) fn position_for_wrapped_visual_row(
        &self,
        target_row: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> Position {
        let body_width = body_width.max(1);
        let mut remaining = target_row;
        for line_index in 0..self.buffer.line_count() {
            let rows = self.wrapped_line_visual_rows(line_index, body_width, mode);
            if remaining < rows {
                let line = self.buffer.line(line_index).unwrap_or_default();
                return Position::new(
                    line_index,
                    byte_column_for_wrapped_row_start(line, remaining, body_width, mode),
                );
            }
            remaining = remaining.saturating_sub(rows);
        }
        buffer_end_position(&self.buffer)
    }

    pub(crate) fn position_for_wrapped_visual_row_column(
        &self,
        target_row: usize,
        target_column: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) -> Position {
        let body_width = body_width.max(1);
        let mut remaining = target_row;
        for line_index in 0..self.buffer.line_count() {
            let rows = self.wrapped_line_visual_rows(line_index, body_width, mode);
            if remaining < rows {
                let line = self.buffer.line(line_index).unwrap_or_default();
                return Position::new(
                    line_index,
                    byte_column_for_wrapped_row_column(
                        line,
                        remaining,
                        target_column,
                        body_width,
                        mode,
                    ),
                );
            }
            remaining = remaining.saturating_sub(rows);
        }
        buffer_end_position(&self.buffer)
    }

    pub(crate) fn keep_cursor_inside_visible_lines(&mut self, body_height: usize) {
        if body_height == 0 {
            return;
        }

        let cursor = self.buffer.cursor_position();
        let last_visible = self
            .first_line
            .saturating_add(body_height.saturating_sub(1))
            .min(self.buffer.line_count().saturating_sub(1));
        let target_line = cursor.line.clamp(self.first_line, last_visible);
        if target_line == cursor.line {
            return;
        }

        let target_column = self.clamp_column_to_line(target_line, cursor.column);
        let _ = self
            .buffer
            .set_cursor(Position::new(target_line, target_column));
    }

    pub(crate) fn keep_cursor_inside_visible_wrapped_rows(
        &mut self,
        body_height: usize,
        body_width: usize,
        mode: AmbiguousWidth,
    ) {
        if body_height == 0 {
            return;
        }

        let body_width = body_width.max(1);
        let top = self.wrapped_top_visual_row(body_width, mode);
        let bottom = top.saturating_add(body_height.saturating_sub(1));
        let cursor_row =
            self.wrapped_visual_row_for_position(self.buffer.cursor_position(), body_width, mode);
        let target_row = cursor_row.clamp(top, bottom);
        if target_row == cursor_row {
            return;
        }

        let _ = self
            .buffer
            .set_cursor(self.position_for_wrapped_visual_row(target_row, body_width, mode));
    }

    pub(crate) fn clamp_column_to_line(&self, line_index: usize, target_column: usize) -> usize {
        let Some(line) = self.buffer.line(line_index) else {
            return 0;
        };
        let mut column = target_column.min(line.len());
        while !line.is_char_boundary(column) {
            column -= 1;
        }
        column
    }
}

pub(crate) fn editor_body_width(shell: &UiShell, buffer: &BufferState, rect: Rect) -> usize {
    let geometry = shell.window_geometry(rect.width, rect.height, Some(buffer.buffer.line_count()));
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
