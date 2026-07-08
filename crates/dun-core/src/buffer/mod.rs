use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

mod model;
mod search;

pub use model::{
    BufferError, BufferId, BufferKind, Cursor, EditMergeKind, EditTransaction, LineEnding,
    Position, SearchMatch, SearchOptions, Selection, TextBuffer, TextEdit, TextRange,
};

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new_untitled()
    }
}

impl TextBuffer {
    pub fn new_untitled() -> Self {
        Self::from_parts(BufferKind::Untitled, vec![String::new()], LineEnding::Lf)
    }

    pub fn from_text(text: &str) -> Self {
        Self::from_text_with_kind(BufferKind::File, text)
    }

    pub fn from_text_with_kind(kind: BufferKind, text: &str) -> Self {
        let line_ending = detect_line_ending(text);
        let lines = parse_lines(text, line_ending);
        Self::from_parts(kind, lines, line_ending)
    }

    pub fn kind(&self) -> BufferKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: BufferKind) {
        self.kind = kind;
    }

    pub fn is_read_only(&self) -> bool {
        matches!(self.kind, BufferKind::ReadOnly)
    }

    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(String::as_str)
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn to_text(&self) -> String {
        self.lines.join(self.line_ending.as_str())
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn cursor_position(&self) -> Position {
        self.cursor.position
    }

    pub fn set_cursor(&mut self, position: Position) -> Result<(), BufferError> {
        self.validate_position(position)?;
        self.break_undo_merge();
        self.cursor = Cursor::new(position);
        self.selection = None;
        Ok(())
    }

    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    pub fn selection_range(&self) -> Option<TextRange> {
        self.selection.map(Selection::range)
    }

    pub fn select(&mut self, anchor: Position, cursor: Position) -> Result<(), BufferError> {
        self.validate_position(anchor)?;
        self.validate_position(cursor)?;
        self.break_undo_merge();
        self.cursor = Cursor::new(cursor);
        self.selection = if anchor == cursor {
            None
        } else {
            Some(Selection::new(anchor, cursor))
        };
        Ok(())
    }

    pub fn select_current_line(&mut self) -> Result<(), BufferError> {
        self.break_undo_merge();
        let line = self
            .cursor
            .position
            .line
            .min(self.lines.len().saturating_sub(1));
        let start = Position::new(line, 0);
        let end = if line + 1 < self.lines.len() {
            Position::new(line + 1, 0)
        } else {
            Position::new(line, self.lines[line].len())
        };
        self.select(start, end)
    }

    pub fn current_line_range(&self) -> TextRange {
        let line = self
            .cursor
            .position
            .line
            .min(self.lines.len().saturating_sub(1));
        let start = Position::new(line, 0);
        let end = if line + 1 < self.lines.len() {
            Position::new(line + 1, 0)
        } else {
            Position::new(line, self.lines[line].len())
        };
        TextRange::new(start, end)
    }

    pub fn clear_selection(&mut self) {
        self.break_undo_merge();
        self.selection = None;
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_dirty(&self) -> bool {
        self.current_fingerprint() != self.saved_fingerprint
    }

    pub fn mark_saved(&mut self) {
        self.saved_fingerprint = self.current_fingerprint();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn move_left(&mut self) -> bool {
        self.break_undo_merge();
        if let Some(range) = self.selection_range() {
            self.set_cursor_after_motion(range.start, false);
            return true;
        }

        match self.previous_position(self.cursor.position) {
            Some(position) => {
                self.set_cursor_after_motion(position, false);
                true
            }
            None => false,
        }
    }

    pub fn move_right(&mut self) -> bool {
        self.break_undo_merge();
        if let Some(range) = self.selection_range() {
            self.set_cursor_after_motion(range.end, false);
            return true;
        }

        match self.next_position(self.cursor.position) {
            Some(position) => {
                self.set_cursor_after_motion(position, false);
                true
            }
            None => false,
        }
    }

    pub fn move_up(&mut self) -> bool {
        self.break_undo_merge();
        let position = self.cursor.position;
        if position.line == 0 {
            self.selection = None;
            return false;
        }

        let column =
            self.clamp_column_to_char_boundary(position.line - 1, self.cursor.preferred_column);
        self.set_cursor_after_motion(Position::new(position.line - 1, column), true);
        true
    }

    pub fn move_down(&mut self) -> bool {
        self.break_undo_merge();
        let position = self.cursor.position;
        if position.line + 1 >= self.lines.len() {
            self.selection = None;
            return false;
        }

        let column =
            self.clamp_column_to_char_boundary(position.line + 1, self.cursor.preferred_column);
        self.set_cursor_after_motion(Position::new(position.line + 1, column), true);
        true
    }

    pub fn move_to_line_start(&mut self) -> bool {
        self.break_undo_merge();
        let position = Position::new(self.cursor.position.line, 0);
        let moved = self.cursor.position != position || self.selection.is_some();
        self.set_cursor_after_motion(position, false);
        moved
    }

    pub fn move_to_line_end(&mut self) -> bool {
        self.break_undo_merge();
        let line = self.cursor.position.line;
        let position = Position::new(line, self.lines[line].len());
        let moved = self.cursor.position != position || self.selection.is_some();
        self.set_cursor_after_motion(position, false);
        moved
    }

    pub fn move_word_left(&mut self) -> bool {
        self.break_undo_merge();
        if let Some(range) = self.selection_range() {
            self.set_cursor_after_motion(range.start, false);
            return true;
        }

        match self.previous_word_boundary(self.cursor.position) {
            Some(position) => {
                self.set_cursor_after_motion(position, false);
                true
            }
            None => false,
        }
    }

    pub fn move_word_right(&mut self) -> bool {
        self.break_undo_merge();
        if let Some(range) = self.selection_range() {
            self.set_cursor_after_motion(range.end, false);
            return true;
        }

        match self.next_word_boundary(self.cursor.position) {
            Some(position) => {
                self.set_cursor_after_motion(position, false);
                true
            }
            None => false,
        }
    }

    pub fn extend_selection_left(&mut self) -> bool {
        self.break_undo_merge();
        let Some(position) = self.previous_position(self.cursor.position) else {
            return false;
        };
        self.extend_selection_to(position, false);
        true
    }

    pub fn extend_selection_right(&mut self) -> bool {
        self.break_undo_merge();
        let Some(position) = self.next_position(self.cursor.position) else {
            return false;
        };
        self.extend_selection_to(position, false);
        true
    }

    pub fn extend_selection_up(&mut self) -> bool {
        self.break_undo_merge();
        let position = self.cursor.position;
        if position.line == 0 {
            return false;
        }

        let column =
            self.clamp_column_to_char_boundary(position.line - 1, self.cursor.preferred_column);
        self.extend_selection_to(Position::new(position.line - 1, column), true);
        true
    }

    pub fn extend_selection_down(&mut self) -> bool {
        self.break_undo_merge();
        let position = self.cursor.position;
        if position.line + 1 >= self.lines.len() {
            return false;
        }

        let column =
            self.clamp_column_to_char_boundary(position.line + 1, self.cursor.preferred_column);
        self.extend_selection_to(Position::new(position.line + 1, column), true);
        true
    }

    pub fn extend_selection_to_line_start(&mut self) -> bool {
        self.break_undo_merge();
        let position = Position::new(self.cursor.position.line, 0);
        if self.cursor.position == position {
            return false;
        }
        self.extend_selection_to(position, false);
        true
    }

    pub fn extend_selection_to_line_end(&mut self) -> bool {
        self.break_undo_merge();
        let line = self.cursor.position.line;
        let position = Position::new(line, self.lines[line].len());
        if self.cursor.position == position {
            return false;
        }
        self.extend_selection_to(position, false);
        true
    }

    pub fn extend_selection_word_left(&mut self) -> bool {
        self.break_undo_merge();
        let Some(position) = self.previous_word_boundary(self.cursor.position) else {
            return false;
        };
        self.extend_selection_to(position, false);
        true
    }

    pub fn extend_selection_word_right(&mut self) -> bool {
        self.break_undo_merge();
        let Some(position) = self.next_word_boundary(self.cursor.position) else {
            return false;
        };
        self.extend_selection_to(position, false);
        true
    }

    pub fn insert_char(&mut self, ch: char) -> Result<(), BufferError> {
        let mut encoded = [0; 4];
        let merge_kind = if self.selection.is_none() && is_mergeable_insert_char(ch) {
            EditMergeKind::InsertRun
        } else {
            EditMergeKind::None
        };
        self.insert_text_with_merge(ch.encode_utf8(&mut encoded), merge_kind)
    }

    pub fn insert_str(&mut self, text: &str) -> Result<(), BufferError> {
        self.insert_text_with_merge(text, EditMergeKind::None)
    }

    fn insert_text_with_merge(
        &mut self,
        text: &str,
        merge_kind: EditMergeKind,
    ) -> Result<(), BufferError> {
        if text.is_empty() {
            return Ok(());
        }

        self.ensure_editable()?;
        let text = normalize_edit_text(text);
        let range = self
            .selection_range()
            .unwrap_or_else(|| TextRange::empty(self.cursor.position));
        self.commit_replace_with_merge(range, &text, merge_kind)
            .map(|_| ())
    }

    pub fn insert_newline(&mut self) -> Result<(), BufferError> {
        self.insert_str("\n")
    }

    pub fn delete_backward(&mut self) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        if let Some(range) = self.selection_range() {
            self.break_undo_merge();
            return self.delete_range(range);
        }

        let Some(previous) = self.previous_position(self.cursor.position) else {
            self.break_undo_merge();
            return Ok(false);
        };

        self.commit_replace_with_merge(
            TextRange::new(previous, self.cursor.position),
            "",
            EditMergeKind::DeleteBackwardRun,
        )
        .map(|_| true)
    }

    pub fn delete_forward(&mut self) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        if let Some(range) = self.selection_range() {
            self.break_undo_merge();
            return self.delete_range(range);
        }

        let Some(next) = self.next_position(self.cursor.position) else {
            self.break_undo_merge();
            return Ok(false);
        };

        self.commit_replace_with_merge(
            TextRange::new(self.cursor.position, next),
            "",
            EditMergeKind::DeleteForwardRun,
        )
        .map(|_| true)
    }

    pub fn delete_word_backward(&mut self) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        if let Some(range) = self.selection_range() {
            return self.delete_range(range);
        }

        let Some(previous) = self.previous_word_boundary(self.cursor.position) else {
            return Ok(false);
        };

        self.commit_replace(TextRange::new(previous, self.cursor.position), "")
            .map(|_| true)
    }

    pub fn delete_word_forward(&mut self) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        if let Some(range) = self.selection_range() {
            return self.delete_range(range);
        }

        let Some(next) = self.next_word_boundary(self.cursor.position) else {
            return Ok(false);
        };

        self.commit_replace(TextRange::new(self.cursor.position, next), "")
            .map(|_| true)
    }

    pub fn delete_range(&mut self, range: TextRange) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        let range = range.normalized();
        self.validate_range(range)?;
        if range.is_empty() {
            return Ok(false);
        }

        self.commit_replace(range, "").map(|_| true)
    }

    pub fn delete_current_line(&mut self) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        let line = self
            .cursor
            .position
            .line
            .min(self.lines.len().saturating_sub(1));

        let range = if self.lines.len() == 1 {
            TextRange::new(Position::new(0, 0), Position::new(0, self.lines[0].len()))
        } else if line + 1 < self.lines.len() {
            TextRange::new(Position::new(line, 0), Position::new(line + 1, 0))
        } else {
            let previous_len = self.lines[line - 1].len();
            TextRange::new(
                Position::new(line - 1, previous_len),
                Position::new(line, self.lines[line].len()),
            )
        };

        self.commit_replace(range, "").map(|_| true)
    }

    pub fn indent_selected_lines(&mut self, indent: &str) -> Result<usize, BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        if indent.is_empty() {
            return Ok(0);
        }

        let (start_line, end_line) = self.selected_line_bounds();
        let before_cursor = self.cursor.position;
        let before_selection = self.selection;
        let mut edits = Vec::new();

        for line_index in (start_line..=end_line).rev() {
            let range = TextRange::empty(Position::new(line_index, 0));
            self.replace_range_inner(range, indent)?;
            edits.push(TextEdit::Replace {
                range,
                old_text: String::new(),
                new_text: indent.to_string(),
            });
        }

        let added_bytes_for_line = |line: usize| {
            if (start_line..=end_line).contains(&line) {
                indent.len()
            } else {
                0
            }
        };
        let after_cursor = Position::new(
            before_cursor.line,
            before_cursor
                .column
                .saturating_add(added_bytes_for_line(before_cursor.line)),
        );
        let after_selection = before_selection.map(|selection| Selection {
            anchor: Position::new(
                selection.anchor.line,
                selection
                    .anchor
                    .column
                    .saturating_add(added_bytes_for_line(selection.anchor.line)),
            ),
            cursor: Position::new(
                selection.cursor.line,
                selection
                    .cursor
                    .column
                    .saturating_add(added_bytes_for_line(selection.cursor.line)),
            ),
        });
        self.cursor = Cursor::new(after_cursor);
        self.selection = after_selection;
        self.undo_stack.push(EditTransaction {
            edits,
            before_cursor,
            after_cursor,
            before_selection,
            after_selection,
            merge_kind: EditMergeKind::None,
        });
        self.redo_stack.clear();
        self.bump_revision();

        Ok(end_line - start_line + 1)
    }

    pub fn outdent_selected_lines(&mut self, indent_width: usize) -> Result<usize, BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        if indent_width == 0 {
            return Ok(0);
        }

        let (start_line, end_line) = self.selected_line_bounds();
        let before_cursor = self.cursor.position;
        let before_selection = self.selection;
        let removals = (start_line..=end_line)
            .map(|line_index| {
                let remove_bytes = self
                    .lines
                    .get(line_index)
                    .map(|line| leading_indent_remove_bytes(line, indent_width))
                    .unwrap_or(0);
                (line_index, remove_bytes)
            })
            .collect::<Vec<_>>();
        let mut edits = Vec::new();
        let mut removed_on_cursor_line = 0usize;

        for (line_index, remove_bytes) in removals.iter().copied().rev() {
            if remove_bytes == 0 {
                continue;
            }
            let range = TextRange::new(
                Position::new(line_index, 0),
                Position::new(line_index, remove_bytes),
            );
            let old_text = self.text_in_range(range)?;
            self.replace_range_inner(range, "")?;
            edits.push(TextEdit::Replace {
                range,
                old_text,
                new_text: String::new(),
            });
            if line_index == before_cursor.line {
                removed_on_cursor_line = remove_bytes;
            }
        }

        if edits.is_empty() {
            return Ok(0);
        }

        let after_cursor = Position::new(
            before_cursor.line,
            before_cursor.column.saturating_sub(removed_on_cursor_line),
        );
        let after_selection = before_selection.map(|selection| Selection {
            anchor: Position::new(
                selection.anchor.line,
                selection
                    .anchor
                    .column
                    .saturating_sub(removed_bytes_for_line(&removals, selection.anchor.line)),
            ),
            cursor: Position::new(
                selection.cursor.line,
                selection
                    .cursor
                    .column
                    .saturating_sub(removed_bytes_for_line(&removals, selection.cursor.line)),
            ),
        });
        self.cursor = Cursor::new(after_cursor);
        self.selection = after_selection;
        self.undo_stack.push(EditTransaction {
            edits,
            before_cursor,
            after_cursor,
            before_selection,
            after_selection,
            merge_kind: EditMergeKind::None,
        });
        self.redo_stack.clear();
        self.bump_revision();

        Ok(end_line - start_line + 1)
    }

    pub fn move_current_line_up(&mut self) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        let line = self.cursor.position.line;
        if line == 0 || line >= self.lines.len() {
            return Ok(false);
        }

        self.swap_adjacent_lines(line - 1)
    }

    pub fn move_current_line_down(&mut self) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        let line = self.cursor.position.line;
        if line + 1 >= self.lines.len() {
            return Ok(false);
        }

        self.swap_adjacent_lines(line)
    }

    pub fn trim_trailing_whitespace(&mut self) -> Result<usize, BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        let before_cursor = self.cursor.position;
        let before_selection = self.selection;
        let mut edits = Vec::new();

        for line_index in (0..self.lines.len()).rev() {
            let trimmed_len = trim_trailing_whitespace_len(&self.lines[line_index]);
            if trimmed_len == self.lines[line_index].len() {
                continue;
            }
            let range = TextRange::new(
                Position::new(line_index, trimmed_len),
                Position::new(line_index, self.lines[line_index].len()),
            );
            let old_text = self.text_in_range(range)?;
            self.replace_range_inner(range, "")?;
            edits.push(TextEdit::Replace {
                range,
                old_text,
                new_text: String::new(),
            });
        }

        if edits.is_empty() {
            return Ok(0);
        }

        let after_cursor = self.clamp_existing_position(before_cursor);
        let after_selection = before_selection.map(|selection| Selection {
            anchor: self.clamp_existing_position(selection.anchor),
            cursor: self.clamp_existing_position(selection.cursor),
        });
        self.cursor = Cursor::new(after_cursor);
        self.selection = after_selection;
        let count = edits.len();
        self.undo_stack.push(EditTransaction {
            edits,
            before_cursor,
            after_cursor,
            before_selection,
            after_selection,
            merge_kind: EditMergeKind::None,
        });
        self.redo_stack.clear();
        self.bump_revision();

        Ok(count)
    }

    pub fn replace_range(&mut self, range: TextRange, new_text: &str) -> Result<(), BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        let new_text = normalize_edit_text(new_text);
        self.commit_replace(range, &new_text).map(|_| ())
    }

    pub fn text_in_range(&self, range: TextRange) -> Result<String, BufferError> {
        let range = range.normalized();
        self.validate_range(range)?;

        if range.start.line == range.end.line {
            let line = &self.lines[range.start.line];
            return Ok(line[range.start.column..range.end.column].to_string());
        }

        let mut out = String::new();
        out.push_str(&self.lines[range.start.line][range.start.column..]);

        for line_index in (range.start.line + 1)..range.end.line {
            out.push('\n');
            out.push_str(&self.lines[line_index]);
        }

        out.push('\n');
        out.push_str(&self.lines[range.end.line][..range.end.column]);
        Ok(out)
    }

    pub fn undo(&mut self) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        let Some(transaction) = self.undo_stack.pop() else {
            return Ok(false);
        };

        self.apply_transaction_undo(&transaction)?;
        self.redo_stack.push(transaction);
        self.break_undo_merge();
        self.bump_revision();
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        let Some(transaction) = self.redo_stack.pop() else {
            return Ok(false);
        };

        self.apply_transaction_redo(&transaction)?;
        self.undo_stack.push(transaction);
        self.break_undo_merge();
        self.bump_revision();
        Ok(true)
    }

    fn selected_line_bounds(&self) -> (usize, usize) {
        let range = self
            .selection_range()
            .unwrap_or_else(|| TextRange::empty(self.cursor.position));
        let mut start = range.start.line.min(self.lines.len().saturating_sub(1));
        let mut end = range.end.line.min(self.lines.len().saturating_sub(1));
        if range.end.column == 0 && range.end.line > range.start.line {
            end = end.saturating_sub(1);
        }
        if start > end {
            std::mem::swap(&mut start, &mut end);
        }
        (start, end)
    }

    fn swap_adjacent_lines(&mut self, first_line: usize) -> Result<bool, BufferError> {
        let second_line = first_line + 1;
        if second_line >= self.lines.len() {
            return Ok(false);
        }

        let old_text = format!("{}\n{}", self.lines[first_line], self.lines[second_line]);
        let new_text = format!("{}\n{}", self.lines[second_line], self.lines[first_line]);
        let range = TextRange::new(
            Position::new(first_line, 0),
            Position::new(second_line, self.lines[second_line].len()),
        );
        let before_cursor = self.cursor.position;
        let before_selection = self.selection;
        self.replace_range_inner(range, &new_text)?;

        let after_cursor = self.clamp_existing_position(Position::new(
            swapped_adjacent_line(before_cursor.line, first_line),
            before_cursor.column,
        ));
        let after_selection = before_selection.map(|selection| Selection {
            anchor: self.clamp_existing_position(Position::new(
                swapped_adjacent_line(selection.anchor.line, first_line),
                selection.anchor.column,
            )),
            cursor: self.clamp_existing_position(Position::new(
                swapped_adjacent_line(selection.cursor.line, first_line),
                selection.cursor.column,
            )),
        });

        self.cursor = Cursor::new(after_cursor);
        self.selection = after_selection;
        self.undo_stack.push(EditTransaction {
            edits: vec![TextEdit::Replace {
                range,
                old_text,
                new_text,
            }],
            before_cursor,
            after_cursor,
            before_selection,
            after_selection,
            merge_kind: EditMergeKind::None,
        });
        self.redo_stack.clear();
        self.bump_revision();
        Ok(true)
    }

    fn clamp_existing_position(&self, position: Position) -> Position {
        let line = position.line.min(self.lines.len().saturating_sub(1));
        let mut column = position.column.min(self.lines[line].len());
        while !self.lines[line].is_char_boundary(column) {
            column = column.saturating_sub(1);
        }
        Position::new(line, column)
    }

    fn from_parts(kind: BufferKind, mut lines: Vec<String>, line_ending: LineEnding) -> Self {
        if lines.is_empty() {
            lines.push(String::new());
        }

        let mut buffer = Self {
            kind,
            lines,
            line_ending,
            cursor: Cursor::default(),
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            revision: 0,
            saved_fingerprint: 0,
        };
        buffer.saved_fingerprint = buffer.current_fingerprint();
        buffer
    }

    fn ensure_editable(&self) -> Result<(), BufferError> {
        if self.is_read_only() {
            Err(BufferError::ReadOnly)
        } else {
            Ok(())
        }
    }

    fn validate_position(&self, position: Position) -> Result<(), BufferError> {
        let Some(line) = self.lines.get(position.line) else {
            return Err(BufferError::InvalidPosition(position));
        };

        if position.column > line.len() || !line.is_char_boundary(position.column) {
            return Err(BufferError::InvalidPosition(position));
        }

        Ok(())
    }

    fn validate_range(&self, range: TextRange) -> Result<(), BufferError> {
        if range.start > range.end {
            return Err(BufferError::InvalidRange(range));
        }

        self.validate_position(range.start)
            .map_err(|_| BufferError::InvalidRange(range))?;
        self.validate_position(range.end)
            .map_err(|_| BufferError::InvalidRange(range))?;
        Ok(())
    }

    fn previous_position(&self, position: Position) -> Option<Position> {
        if position.column > 0 {
            let line = &self.lines[position.line];
            let previous_column = line[..position.column]
                .char_indices()
                .last()
                .map(|(column, _)| column)
                .unwrap_or(0);
            return Some(Position::new(position.line, previous_column));
        }

        if position.line > 0 {
            let previous_line = position.line - 1;
            Some(Position::new(
                previous_line,
                self.lines[previous_line].len(),
            ))
        } else {
            None
        }
    }

    fn next_position(&self, position: Position) -> Option<Position> {
        let line = &self.lines[position.line];
        if position.column < line.len() {
            let ch = line[position.column..].chars().next()?;
            return Some(Position::new(
                position.line,
                position.column + ch.len_utf8(),
            ));
        }

        if position.line + 1 < self.lines.len() {
            Some(Position::new(position.line + 1, 0))
        } else {
            None
        }
    }

    fn previous_logical_char(&self, position: Position) -> Option<(Position, Position, char)> {
        if position.column > 0 {
            let start = self.previous_position(position)?;
            let ch = self.lines[start.line][start.column..position.column]
                .chars()
                .next()?;
            return Some((start, position, ch));
        }

        if position.line > 0 {
            let start = Position::new(position.line - 1, self.lines[position.line - 1].len());
            Some((start, position, '\n'))
        } else {
            None
        }
    }

    fn next_logical_char(&self, position: Position) -> Option<(Position, Position, char)> {
        let line = &self.lines[position.line];
        if position.column < line.len() {
            let ch = line[position.column..].chars().next()?;
            let end = Position::new(position.line, position.column + ch.len_utf8());
            return Some((position, end, ch));
        }

        if position.line + 1 < self.lines.len() {
            Some((position, Position::new(position.line + 1, 0), '\n'))
        } else {
            None
        }
    }

    fn previous_word_boundary(&self, position: Position) -> Option<Position> {
        let mut cursor = position;

        while let Some((start, _, ch)) = self.previous_logical_char(cursor) {
            if is_word_boundary_whitespace(ch) {
                cursor = start;
            } else {
                let class = WordClass::from_char(ch);
                cursor = start;
                while let Some((start, _, ch)) = self.previous_logical_char(cursor) {
                    if WordClass::from_char(ch) != class {
                        break;
                    }
                    cursor = start;
                }
                return Some(cursor);
            }
        }

        if cursor == position {
            None
        } else {
            Some(cursor)
        }
    }

    fn next_word_boundary(&self, position: Position) -> Option<Position> {
        let mut cursor = position;
        let (_, end, ch) = self.next_logical_char(cursor)?;

        if is_word_boundary_whitespace(ch) {
            cursor = end;
            while let Some((_, end, ch)) = self.next_logical_char(cursor) {
                if !is_word_boundary_whitespace(ch) {
                    break;
                }
                cursor = end;
            }
            return Some(cursor);
        }

        let class = WordClass::from_char(ch);
        cursor = end;
        while let Some((_, end, ch)) = self.next_logical_char(cursor) {
            if WordClass::from_char(ch) != class {
                break;
            }
            cursor = end;
        }
        while let Some((_, end, ch)) = self.next_logical_char(cursor) {
            if !is_word_boundary_whitespace(ch) {
                break;
            }
            cursor = end;
        }

        Some(cursor)
    }

    fn clamp_column_to_char_boundary(&self, line_index: usize, target: usize) -> usize {
        let line = &self.lines[line_index];
        let mut column = target.min(line.len());
        while !line.is_char_boundary(column) {
            column -= 1;
        }
        column
    }

    fn set_cursor_after_motion(&mut self, position: Position, keep_preferred_column: bool) {
        self.cursor.position = position;
        if !keep_preferred_column {
            self.cursor.preferred_column = position.column;
        }
        self.selection = None;
    }

    fn extend_selection_to(&mut self, position: Position, keep_preferred_column: bool) {
        let anchor = self
            .selection
            .map(|selection| selection.anchor)
            .unwrap_or(self.cursor.position);
        self.cursor.position = position;
        if !keep_preferred_column {
            self.cursor.preferred_column = position.column;
        }
        self.selection = if anchor == position {
            None
        } else {
            Some(Selection::new(anchor, position))
        };
    }

    fn commit_replace(
        &mut self,
        range: TextRange,
        new_text: &str,
    ) -> Result<Position, BufferError> {
        self.commit_replace_with_merge(range, new_text, EditMergeKind::None)
    }

    fn commit_replace_with_merge(
        &mut self,
        range: TextRange,
        new_text: &str,
        merge_kind: EditMergeKind,
    ) -> Result<Position, BufferError> {
        let range = range.normalized();
        self.validate_range(range)?;

        let old_text = self.text_in_range(range)?;
        let before_cursor = self.cursor.position;
        let before_selection = self.selection;
        let after_cursor = end_position_after_text(range.start, new_text);

        if old_text == new_text {
            self.cursor = Cursor::new(after_cursor);
            self.selection = None;
            if merge_kind == EditMergeKind::None {
                self.break_undo_merge();
            }
            return Ok(after_cursor);
        }

        self.replace_range_inner(range, new_text)?;
        self.cursor = Cursor::new(after_cursor);
        self.selection = None;
        if self.try_merge_transaction(
            merge_kind,
            range,
            &old_text,
            new_text,
            before_cursor,
            after_cursor,
            before_selection,
        ) {
            self.redo_stack.clear();
            self.bump_revision();
            return Ok(after_cursor);
        }
        if merge_kind == EditMergeKind::None {
            self.break_undo_merge();
        }
        self.undo_stack.push(EditTransaction {
            edits: vec![TextEdit::Replace {
                range,
                old_text,
                new_text: new_text.to_string(),
            }],
            before_cursor,
            after_cursor,
            before_selection,
            after_selection: None,
            merge_kind,
        });
        self.redo_stack.clear();
        self.bump_revision();
        Ok(after_cursor)
    }

    fn try_merge_transaction(
        &mut self,
        merge_kind: EditMergeKind,
        range: TextRange,
        old_text: &str,
        new_text: &str,
        before_cursor: Position,
        after_cursor: Position,
        before_selection: Option<Selection>,
    ) -> bool {
        match merge_kind {
            EditMergeKind::InsertRun => self.try_merge_insert_run(
                range,
                old_text,
                new_text,
                before_cursor,
                after_cursor,
                before_selection,
            ),
            EditMergeKind::DeleteBackwardRun | EditMergeKind::DeleteForwardRun => self
                .try_merge_delete_run(
                    merge_kind,
                    range,
                    old_text,
                    new_text,
                    before_cursor,
                    after_cursor,
                    before_selection,
                ),
            EditMergeKind::None => false,
        }
    }

    fn try_merge_insert_run(
        &mut self,
        range: TextRange,
        old_text: &str,
        new_text: &str,
        before_cursor: Position,
        after_cursor: Position,
        before_selection: Option<Selection>,
    ) -> bool {
        if !range.is_empty() || !old_text.is_empty() || before_selection.is_some() {
            return false;
        }

        if !self.redo_stack.is_empty() {
            return false;
        }

        let Some(transaction) = self.undo_stack.last_mut() else {
            return false;
        };
        if transaction.merge_kind != EditMergeKind::InsertRun
            || transaction.after_cursor != before_cursor
            || transaction.after_selection.is_some()
            || transaction.edits.len() != 1
        {
            return false;
        }

        let TextEdit::Replace {
            range: previous_range,
            old_text: previous_old_text,
            new_text: previous_new_text,
        } = &mut transaction.edits[0];
        if !previous_range.is_empty() || !previous_old_text.is_empty() {
            return false;
        }

        if end_position_after_text(previous_range.start, previous_new_text) != range.start {
            return false;
        }

        previous_new_text.push_str(new_text);
        transaction.after_cursor = after_cursor;
        true
    }

    fn try_merge_delete_run(
        &mut self,
        merge_kind: EditMergeKind,
        range: TextRange,
        old_text: &str,
        new_text: &str,
        before_cursor: Position,
        after_cursor: Position,
        before_selection: Option<Selection>,
    ) -> bool {
        if range.is_empty()
            || old_text.is_empty()
            || !new_text.is_empty()
            || before_selection.is_some()
            || !self.redo_stack.is_empty()
        {
            return false;
        }

        let Some(transaction) = self.undo_stack.last_mut() else {
            return false;
        };
        if transaction.merge_kind != merge_kind
            || transaction.after_cursor != before_cursor
            || transaction.after_selection.is_some()
            || transaction.edits.is_empty()
        {
            return false;
        }

        match merge_kind {
            EditMergeKind::DeleteBackwardRun if range.end != before_cursor => return false,
            EditMergeKind::DeleteForwardRun if range.start != before_cursor => return false,
            EditMergeKind::DeleteBackwardRun | EditMergeKind::DeleteForwardRun => {}
            EditMergeKind::None | EditMergeKind::InsertRun => return false,
        }

        transaction.edits.push(TextEdit::Replace {
            range,
            old_text: old_text.to_string(),
            new_text: String::new(),
        });
        transaction.after_cursor = after_cursor;
        true
    }

    fn break_undo_merge(&mut self) {
        if let Some(transaction) = self.undo_stack.last_mut() {
            transaction.merge_kind = EditMergeKind::None;
        }
    }

    fn replace_range_inner(
        &mut self,
        range: TextRange,
        new_text: &str,
    ) -> Result<Position, BufferError> {
        let range = range.normalized();
        self.validate_range(range)?;

        let prefix = self.lines[range.start.line][..range.start.column].to_string();
        let suffix = self.lines[range.end.line][range.end.column..].to_string();
        let parts: Vec<&str> = new_text.split('\n').collect();
        let mut replacement = Vec::with_capacity(parts.len());

        if parts.len() == 1 {
            replacement.push(format!("{}{}{}", prefix, parts[0], suffix));
        } else {
            replacement.push(format!("{}{}", prefix, parts[0]));
            for part in &parts[1..parts.len() - 1] {
                replacement.push((*part).to_string());
            }
            replacement.push(format!("{}{}", parts[parts.len() - 1], suffix));
        }

        self.lines
            .splice(range.start.line..=range.end.line, replacement);
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }

        Ok(end_position_after_text(range.start, new_text))
    }

    fn apply_transaction_undo(&mut self, transaction: &EditTransaction) -> Result<(), BufferError> {
        for edit in transaction.edits.iter().rev() {
            match edit {
                TextEdit::Replace {
                    range,
                    old_text,
                    new_text,
                } => {
                    let inserted_range =
                        TextRange::new(range.start, end_position_after_text(range.start, new_text));
                    self.replace_range_inner(inserted_range, old_text)?;
                }
            }
        }

        self.cursor = Cursor::new(transaction.before_cursor);
        self.selection = transaction.before_selection;
        Ok(())
    }

    fn apply_transaction_redo(&mut self, transaction: &EditTransaction) -> Result<(), BufferError> {
        for edit in &transaction.edits {
            match edit {
                TextEdit::Replace {
                    range, new_text, ..
                } => {
                    self.replace_range_inner(*range, new_text)?;
                }
            }
        }

        self.cursor = Cursor::new(transaction.after_cursor);
        self.selection = transaction.after_selection;
        Ok(())
    }

    fn current_fingerprint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.lines.hash(&mut hasher);
        self.line_ending.hash(&mut hasher);
        hasher.finish()
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
}

fn detect_line_ending(text: &str) -> LineEnding {
    match text.find('\n') {
        Some(index) if index > 0 && text.as_bytes()[index - 1] == b'\r' => LineEnding::CrLf,
        _ => LineEnding::Lf,
    }
}

fn parse_lines(text: &str, line_ending: LineEnding) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    text.split('\n')
        .map(|line| match line_ending {
            LineEnding::Lf => line.to_string(),
            LineEnding::CrLf => line.strip_suffix('\r').unwrap_or(line).to_string(),
        })
        .collect()
}

fn normalize_edit_text(text: &str) -> String {
    if text.contains("\r\n") {
        text.replace("\r\n", "\n")
    } else {
        text.to_string()
    }
}

fn leading_indent_remove_bytes(line: &str, indent_width: usize) -> usize {
    let mut columns = 0usize;
    let mut bytes = 0usize;
    for ch in line.chars() {
        match ch {
            ' ' if columns < indent_width => {
                columns += 1;
                bytes += 1;
            }
            '\t' if columns == 0 => return ch.len_utf8(),
            _ => break,
        }
        if columns >= indent_width {
            break;
        }
    }
    bytes
}

fn removed_bytes_for_line(removals: &[(usize, usize)], line: usize) -> usize {
    removals
        .iter()
        .find(|(line_index, _)| *line_index == line)
        .map(|(_, bytes)| *bytes)
        .unwrap_or(0)
}

fn trim_trailing_whitespace_len(line: &str) -> usize {
    line.trim_end_matches([' ', '\t']).len()
}

const fn swapped_adjacent_line(line: usize, first_line: usize) -> usize {
    if line == first_line {
        first_line + 1
    } else if line == first_line + 1 {
        first_line
    } else {
        line
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WordClass {
    Word,
    Punctuation,
}

impl WordClass {
    fn from_char(ch: char) -> Self {
        if ch == '_' || ch.is_alphanumeric() {
            Self::Word
        } else {
            Self::Punctuation
        }
    }
}

fn is_word_boundary_whitespace(ch: char) -> bool {
    ch.is_whitespace()
}

fn is_mergeable_insert_char(ch: char) -> bool {
    ch != '\n' && ch != '\r' && !ch.is_control()
}

fn end_position_after_text(start: Position, text: &str) -> Position {
    let mut line = start.line;
    let mut column = start.column;

    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += ch.len_utf8();
        }
    }

    Position::new(line, column)
}

#[cfg(test)]
mod tests;
