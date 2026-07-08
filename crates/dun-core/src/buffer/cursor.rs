use super::*;

impl TextBuffer {
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

    pub(super) fn clamp_existing_position(&self, position: Position) -> Position {
        let line = position.line.min(self.lines.len().saturating_sub(1));
        let mut column = position.column.min(self.lines[line].len());
        while !self.lines[line].is_char_boundary(column) {
            column = column.saturating_sub(1);
        }
        Position::new(line, column)
    }

    pub(super) fn previous_position(&self, position: Position) -> Option<Position> {
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

    pub(super) fn next_position(&self, position: Position) -> Option<Position> {
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

    pub(super) fn previous_word_boundary(&self, position: Position) -> Option<Position> {
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

    pub(super) fn next_word_boundary(&self, position: Position) -> Option<Position> {
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

    pub(super) fn clamp_column_to_char_boundary(&self, line_index: usize, target: usize) -> usize {
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
