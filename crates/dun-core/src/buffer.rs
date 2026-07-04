use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferKind {
    Untitled,
    File,
    ReadOnly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineEnding {
    #[default]
    Lf,
    CrLf,
}

impl LineEnding {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    pub const fn zero() -> Self {
        Self { line: 0, column: 0 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Cursor {
    pub position: Position,
    pub preferred_column: usize,
}

impl Cursor {
    pub const fn new(position: Position) -> Self {
        Self {
            position,
            preferred_column: position.column,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextRange {
    pub start: Position,
    pub end: Position,
}

impl TextRange {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }.normalized()
    }

    pub const fn empty(at: Position) -> Self {
        Self { start: at, end: at }
    }

    pub fn normalized(self) -> Self {
        if self.start <= self.end {
            self
        } else {
            Self {
                start: self.end,
                end: self.start,
            }
        }
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Selection {
    pub anchor: Position,
    pub cursor: Position,
}

impl Selection {
    pub const fn new(anchor: Position, cursor: Position) -> Self {
        Self { anchor, cursor }
    }

    pub fn range(self) -> TextRange {
        TextRange::new(self.anchor, self.cursor)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditTransaction {
    pub edits: Vec<TextEdit>,
    pub before_cursor: Position,
    pub after_cursor: Position,
    pub before_selection: Option<Selection>,
    pub after_selection: Option<Selection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextEdit {
    Replace {
        range: TextRange,
        old_text: String,
        new_text: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferError {
    InvalidPosition(Position),
    InvalidRange(TextRange),
    ReadOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextBuffer {
    kind: BufferKind,
    lines: Vec<String>,
    line_ending: LineEnding,
    cursor: Cursor,
    selection: Option<Selection>,
    undo_stack: Vec<EditTransaction>,
    redo_stack: Vec<EditTransaction>,
    revision: u64,
    saved_fingerprint: u64,
}

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
        self.cursor = Cursor::new(cursor);
        self.selection = if anchor == cursor {
            None
        } else {
            Some(Selection::new(anchor, cursor))
        };
        Ok(())
    }

    pub fn clear_selection(&mut self) {
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
        let position = Position::new(self.cursor.position.line, 0);
        let moved = self.cursor.position != position || self.selection.is_some();
        self.set_cursor_after_motion(position, false);
        moved
    }

    pub fn move_to_line_end(&mut self) -> bool {
        let line = self.cursor.position.line;
        let position = Position::new(line, self.lines[line].len());
        let moved = self.cursor.position != position || self.selection.is_some();
        self.set_cursor_after_motion(position, false);
        moved
    }

    pub fn insert_char(&mut self, ch: char) -> Result<(), BufferError> {
        let mut encoded = [0; 4];
        self.insert_str(ch.encode_utf8(&mut encoded))
    }

    pub fn insert_str(&mut self, text: &str) -> Result<(), BufferError> {
        if text.is_empty() {
            return Ok(());
        }

        self.ensure_editable()?;
        let text = normalize_edit_text(text);
        let range = self
            .selection_range()
            .unwrap_or_else(|| TextRange::empty(self.cursor.position));
        self.commit_replace(range, &text).map(|_| ())
    }

    pub fn insert_newline(&mut self) -> Result<(), BufferError> {
        self.insert_str("\n")
    }

    pub fn delete_backward(&mut self) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        if let Some(range) = self.selection_range() {
            return self.delete_range(range);
        }

        let Some(previous) = self.previous_position(self.cursor.position) else {
            return Ok(false);
        };

        self.commit_replace(TextRange::new(previous, self.cursor.position), "")
            .map(|_| true)
    }

    pub fn delete_forward(&mut self) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        if let Some(range) = self.selection_range() {
            return self.delete_range(range);
        }

        let Some(next) = self.next_position(self.cursor.position) else {
            return Ok(false);
        };

        self.commit_replace(TextRange::new(self.cursor.position, next), "")
            .map(|_| true)
    }

    pub fn delete_range(&mut self, range: TextRange) -> Result<bool, BufferError> {
        self.ensure_editable()?;
        let range = range.normalized();
        self.validate_range(range)?;
        if range.is_empty() {
            return Ok(false);
        }

        self.commit_replace(range, "").map(|_| true)
    }

    pub fn replace_range(&mut self, range: TextRange, new_text: &str) -> Result<(), BufferError> {
        self.ensure_editable()?;
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
        self.bump_revision();
        Ok(true)
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

    fn commit_replace(
        &mut self,
        range: TextRange,
        new_text: &str,
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
            return Ok(after_cursor);
        }

        self.replace_range_inner(range, new_text)?;
        self.cursor = Cursor::new(after_cursor);
        self.selection = None;
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
        });
        self.redo_stack.clear();
        self.bump_revision();
        Ok(after_cursor)
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
mod tests {
    use super::*;

    #[test]
    fn new_untitled_buffer_starts_empty_and_clean() {
        let buffer = TextBuffer::new_untitled();

        assert_eq!(buffer.kind(), BufferKind::Untitled);
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(buffer.line(0), Some(""));
        assert_eq!(buffer.cursor_position(), Position::zero());
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn from_text_preserves_lf_shape() {
        let buffer = TextBuffer::from_text("alpha\nbeta\n");

        assert_eq!(buffer.kind(), BufferKind::File);
        assert_eq!(buffer.line_ending(), LineEnding::Lf);
        assert_eq!(buffer.line_count(), 3);
        assert_eq!(buffer.line(0), Some("alpha"));
        assert_eq!(buffer.line(1), Some("beta"));
        assert_eq!(buffer.line(2), Some(""));
        assert_eq!(buffer.to_text(), "alpha\nbeta\n");
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn from_text_preserves_crlf_shape() {
        let buffer = TextBuffer::from_text("alpha\r\nbeta");

        assert_eq!(buffer.line_ending(), LineEnding::CrLf);
        assert_eq!(buffer.line(0), Some("alpha"));
        assert_eq!(buffer.line(1), Some("beta"));
        assert_eq!(buffer.to_text(), "alpha\r\nbeta");
    }

    #[test]
    fn set_cursor_rejects_invalid_utf8_boundary() {
        let mut buffer = TextBuffer::from_text("é");

        assert_eq!(
            buffer.set_cursor(Position::new(0, 1)),
            Err(BufferError::InvalidPosition(Position::new(0, 1)))
        );
        assert_eq!(buffer.cursor_position(), Position::zero());
    }

    #[test]
    fn cursor_moves_across_utf8_char_boundaries() {
        let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "aé\nb");

        assert!(buffer.move_right());
        assert_eq!(buffer.cursor_position(), Position::new(0, 1));
        assert!(buffer.move_right());
        assert_eq!(buffer.cursor_position(), Position::new(0, 3));
        assert!(buffer.move_right());
        assert_eq!(buffer.cursor_position(), Position::new(1, 0));
        assert!(buffer.move_left());
        assert_eq!(buffer.cursor_position(), Position::new(0, 3));
    }

    #[test]
    fn vertical_motion_keeps_preferred_column_when_possible() {
        let mut buffer = TextBuffer::from_text("abcd\nx\nwxyz");
        buffer.set_cursor(Position::new(0, 4)).unwrap();

        assert!(buffer.move_down());
        assert_eq!(buffer.cursor_position(), Position::new(1, 1));
        assert!(buffer.move_down());
        assert_eq!(buffer.cursor_position(), Position::new(2, 4));
    }

    #[test]
    fn insert_text_updates_line_and_cursor() {
        let mut buffer = TextBuffer::new_untitled();

        buffer.insert_str("hello").unwrap();

        assert_eq!(buffer.line(0), Some("hello"));
        assert_eq!(buffer.cursor_position(), Position::new(0, 5));
        assert!(buffer.is_dirty());
        assert!(buffer.can_undo());
        assert!(!buffer.can_redo());
    }

    #[test]
    fn insert_newline_splits_current_line() {
        let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hello");
        buffer.set_cursor(Position::new(0, 2)).unwrap();

        buffer.insert_newline().unwrap();

        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.line(0), Some("he"));
        assert_eq!(buffer.line(1), Some("llo"));
        assert_eq!(buffer.cursor_position(), Position::new(1, 0));
    }

    #[test]
    fn insert_replaces_active_selection() {
        let mut buffer = TextBuffer::from_text("abcdef");
        buffer
            .select(Position::new(0, 2), Position::new(0, 5))
            .unwrap();

        buffer.insert_str("X").unwrap();

        assert_eq!(buffer.to_text(), "abXf");
        assert_eq!(buffer.cursor_position(), Position::new(0, 3));
        assert_eq!(buffer.selection(), None);
    }

    #[test]
    fn delete_backward_removes_previous_utf8_character() {
        let mut buffer = TextBuffer::from_text("aé");
        buffer.set_cursor(Position::new(0, 3)).unwrap();

        assert!(buffer.delete_backward().unwrap());

        assert_eq!(buffer.to_text(), "a");
        assert_eq!(buffer.cursor_position(), Position::new(0, 1));
    }

    #[test]
    fn delete_backward_at_line_start_merges_with_previous_line() {
        let mut buffer = TextBuffer::from_text("one\ntwo");
        buffer.set_cursor(Position::new(1, 0)).unwrap();

        assert!(buffer.delete_backward().unwrap());

        assert_eq!(buffer.to_text(), "onetwo");
        assert_eq!(buffer.cursor_position(), Position::new(0, 3));
    }

    #[test]
    fn delete_forward_at_line_end_merges_with_next_line() {
        let mut buffer = TextBuffer::from_text("one\ntwo");
        buffer.set_cursor(Position::new(0, 3)).unwrap();

        assert!(buffer.delete_forward().unwrap());

        assert_eq!(buffer.to_text(), "onetwo");
        assert_eq!(buffer.cursor_position(), Position::new(0, 3));
    }

    #[test]
    fn delete_range_removes_multiline_text() {
        let mut buffer = TextBuffer::from_text("alpha\nbeta\ngamma");

        assert!(
            buffer
                .delete_range(TextRange::new(Position::new(0, 2), Position::new(2, 2)))
                .unwrap()
        );

        assert_eq!(buffer.to_text(), "almma");
        assert_eq!(buffer.cursor_position(), Position::new(0, 2));
    }

    #[test]
    fn replace_range_accepts_crlf_paste_as_internal_lf() {
        let mut buffer = TextBuffer::new_untitled();

        buffer
            .replace_range(TextRange::empty(Position::zero()), "a\r\nb")
            .unwrap();

        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.to_text(), "a\nb");
    }

    #[test]
    fn undo_and_redo_restore_content_and_cursor() {
        let mut buffer = TextBuffer::new_untitled();

        buffer.insert_str("hello").unwrap();
        assert_eq!(buffer.cursor_position(), Position::new(0, 5));

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "");
        assert_eq!(buffer.cursor_position(), Position::zero());
        assert!(buffer.can_redo());

        assert!(buffer.redo().unwrap());
        assert_eq!(buffer.to_text(), "hello");
        assert_eq!(buffer.cursor_position(), Position::new(0, 5));
    }

    #[test]
    fn undo_back_to_saved_content_clears_dirty_state() {
        let mut buffer = TextBuffer::new_untitled();

        buffer.insert_str("hello").unwrap();
        assert!(buffer.is_dirty());

        assert!(buffer.undo().unwrap());

        assert!(!buffer.is_dirty());
    }

    #[test]
    fn mark_saved_resets_dirty_baseline() {
        let mut buffer = TextBuffer::new_untitled();

        buffer.insert_str("hello").unwrap();
        assert!(buffer.is_dirty());

        buffer.mark_saved();

        assert!(!buffer.is_dirty());
    }

    #[test]
    fn readonly_buffer_rejects_editing() {
        let mut buffer = TextBuffer::from_text_with_kind(BufferKind::ReadOnly, "locked");

        assert_eq!(buffer.insert_char('!'), Err(BufferError::ReadOnly));
        assert_eq!(buffer.delete_forward(), Err(BufferError::ReadOnly));
        assert_eq!(buffer.undo(), Err(BufferError::ReadOnly));
        assert_eq!(buffer.to_text(), "locked");
    }
}
