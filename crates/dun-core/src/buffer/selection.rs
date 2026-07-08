use super::*;

impl TextBuffer {
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
}
