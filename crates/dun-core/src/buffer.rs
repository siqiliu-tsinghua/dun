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
    pub merge_kind: EditMergeKind,
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
pub enum EditMergeKind {
    None,
    InsertRun,
    DeleteBackwardRun,
    DeleteForwardRun,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferError {
    InvalidPosition(Position),
    InvalidRange(TextRange),
    ReadOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchMatch {
    pub range: TextRange,
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

    pub fn find_all(&self, query: &str) -> Vec<SearchMatch> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut matches = Vec::new();
        for (line_index, line) in self.lines.iter().enumerate() {
            for (column, text) in line.match_indices(query) {
                matches.push(SearchMatch {
                    range: TextRange::new(
                        Position::new(line_index, column),
                        Position::new(line_index, column + text.len()),
                    ),
                });
            }
        }
        matches
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

    pub fn replace_range(&mut self, range: TextRange, new_text: &str) -> Result<(), BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        let new_text = normalize_edit_text(new_text);
        self.commit_replace(range, &new_text).map(|_| ())
    }

    pub fn replace_all(&mut self, query: &str, new_text: &str) -> Result<usize, BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        if query.is_empty() {
            return Ok(0);
        }

        let matches = self.find_all(query);
        if matches.is_empty() {
            return Ok(0);
        }

        let new_text = normalize_edit_text(new_text);
        let before_cursor = self.cursor.position;
        let before_selection = self.selection;
        let mut replacements = Vec::with_capacity(matches.len());
        for item in &matches {
            replacements.push((item.range, self.text_in_range(item.range)?));
        }

        let mut edits = Vec::with_capacity(replacements.len());
        for (range, old_text) in replacements.into_iter().rev() {
            self.replace_range_inner(range, &new_text)?;
            edits.push(TextEdit::Replace {
                range,
                old_text,
                new_text: new_text.clone(),
            });
        }

        let after_cursor = matches
            .first()
            .map(|item| end_position_after_text(item.range.start, &new_text))
            .unwrap_or(before_cursor);
        self.cursor = Cursor::new(after_cursor);
        self.selection = None;
        self.undo_stack.push(EditTransaction {
            edits,
            before_cursor,
            after_cursor,
            before_selection,
            after_selection: None,
            merge_kind: EditMergeKind::None,
        });
        self.redo_stack.clear();
        self.bump_revision();

        Ok(matches.len())
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
    fn extend_selection_tracks_anchor_and_utf8_boundaries() {
        let mut buffer = TextBuffer::from_text("aébc");
        buffer.set_cursor(Position::new(0, 1)).unwrap();

        assert!(buffer.extend_selection_right());
        assert_eq!(buffer.cursor_position(), Position::new(0, 3));
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::new(0, 1), Position::new(0, 3)))
        );
        assert_eq!(
            buffer.selection_range(),
            Some(TextRange::new(Position::new(0, 1), Position::new(0, 3)))
        );

        assert!(buffer.extend_selection_right());
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::new(0, 1), Position::new(0, 4)))
        );

        assert!(buffer.extend_selection_left());
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::new(0, 1), Position::new(0, 3)))
        );
        assert!(buffer.extend_selection_left());
        assert_eq!(buffer.selection(), None);
        assert_eq!(buffer.cursor_position(), Position::new(0, 1));
    }

    #[test]
    fn extend_selection_crosses_lines_and_keeps_preferred_column() {
        let mut buffer = TextBuffer::from_text("abcd\nx\nwxyz");
        buffer.set_cursor(Position::new(0, 4)).unwrap();

        assert!(buffer.extend_selection_down());
        assert_eq!(buffer.cursor_position(), Position::new(1, 1));
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::new(0, 4), Position::new(1, 1)))
        );

        assert!(buffer.extend_selection_down());
        assert_eq!(buffer.cursor_position(), Position::new(2, 4));
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::new(0, 4), Position::new(2, 4)))
        );

        assert!(buffer.extend_selection_up());
        assert_eq!(buffer.cursor_position(), Position::new(1, 1));
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::new(0, 4), Position::new(1, 1)))
        );
    }

    #[test]
    fn extend_selection_to_line_edges() {
        let mut buffer = TextBuffer::from_text("abc\ndef");
        buffer.set_cursor(Position::new(1, 1)).unwrap();

        assert!(buffer.extend_selection_to_line_end());
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::new(1, 1), Position::new(1, 3)))
        );

        assert!(buffer.extend_selection_to_line_start());
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::new(1, 1), Position::new(1, 0)))
        );
        assert!(buffer.extend_selection_to_line_end());
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::new(1, 1), Position::new(1, 3)))
        );
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
    fn insert_char_run_undoes_as_one_transaction() {
        let mut buffer = TextBuffer::new_untitled();

        buffer.insert_char('a').unwrap();
        buffer.insert_char('é').unwrap();
        buffer.insert_char('b').unwrap();

        assert_eq!(buffer.to_text(), "aéb");
        assert_eq!(buffer.undo_stack.len(), 1);

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "");
        assert_eq!(buffer.cursor_position(), Position::zero());
        assert!(buffer.can_redo());

        assert!(buffer.redo().unwrap());
        assert_eq!(buffer.to_text(), "aéb");
        assert_eq!(buffer.cursor_position(), Position::new(0, 4));
    }

    #[test]
    fn cursor_motion_breaks_insert_char_merge() {
        let mut buffer = TextBuffer::new_untitled();

        buffer.insert_char('a').unwrap();
        assert!(buffer.move_left());
        assert!(buffer.move_right());
        buffer.insert_char('b').unwrap();

        assert_eq!(buffer.to_text(), "ab");
        assert_eq!(buffer.undo_stack.len(), 2);

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "a");
        assert_eq!(buffer.cursor_position(), Position::new(0, 1));

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "");
        assert_eq!(buffer.cursor_position(), Position::zero());
    }

    #[test]
    fn insert_str_does_not_merge_with_insert_char_runs() {
        let mut buffer = TextBuffer::new_untitled();

        buffer.insert_char('a').unwrap();
        buffer.insert_str("bc").unwrap();
        buffer.insert_char('d').unwrap();

        assert_eq!(buffer.to_text(), "abcd");
        assert_eq!(buffer.undo_stack.len(), 3);

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "abc");

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "a");

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "");
    }

    #[test]
    fn redoed_insert_run_does_not_absorb_new_typing() {
        let mut buffer = TextBuffer::new_untitled();

        buffer.insert_char('a').unwrap();
        buffer.insert_char('b').unwrap();
        assert!(buffer.undo().unwrap());
        assert!(buffer.redo().unwrap());
        buffer.insert_char('c').unwrap();

        assert_eq!(buffer.to_text(), "abc");
        assert_eq!(buffer.undo_stack.len(), 2);

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "ab");

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "");
    }

    #[test]
    fn delete_backward_run_undoes_as_one_transaction() {
        let mut buffer = TextBuffer::from_text("abcd");
        buffer.set_cursor(Position::new(0, 4)).unwrap();

        assert!(buffer.delete_backward().unwrap());
        assert!(buffer.delete_backward().unwrap());

        assert_eq!(buffer.to_text(), "ab");
        assert_eq!(buffer.cursor_position(), Position::new(0, 2));
        assert_eq!(buffer.undo_stack.len(), 1);

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "abcd");
        assert_eq!(buffer.cursor_position(), Position::new(0, 4));

        assert!(buffer.redo().unwrap());
        assert_eq!(buffer.to_text(), "ab");
        assert_eq!(buffer.cursor_position(), Position::new(0, 2));
    }

    #[test]
    fn delete_forward_run_undoes_as_one_transaction() {
        let mut buffer = TextBuffer::from_text("abcd");

        assert!(buffer.delete_forward().unwrap());
        assert!(buffer.delete_forward().unwrap());

        assert_eq!(buffer.to_text(), "cd");
        assert_eq!(buffer.cursor_position(), Position::zero());
        assert_eq!(buffer.undo_stack.len(), 1);

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "abcd");
        assert_eq!(buffer.cursor_position(), Position::zero());
    }

    #[test]
    fn switching_delete_direction_breaks_delete_merge() {
        let mut buffer = TextBuffer::from_text("abcd");
        buffer.set_cursor(Position::new(0, 2)).unwrap();

        assert!(buffer.delete_backward().unwrap());
        assert!(buffer.delete_forward().unwrap());

        assert_eq!(buffer.to_text(), "ad");
        assert_eq!(buffer.undo_stack.len(), 2);

        assert!(buffer.undo().unwrap());
        assert_eq!(buffer.to_text(), "acd");
        assert_eq!(buffer.cursor_position(), Position::new(0, 1));
    }

    #[test]
    fn word_motion_uses_utf8_safe_boundaries() {
        let mut buffer = TextBuffer::from_text("éclair  two_2!\nthree");

        assert!(buffer.move_word_right());
        assert_eq!(buffer.cursor_position(), Position::new(0, 9));

        assert!(buffer.move_word_right());
        assert_eq!(buffer.cursor_position(), Position::new(0, 14));

        assert!(buffer.move_word_right());
        assert_eq!(buffer.cursor_position(), Position::new(1, 0));

        buffer.set_cursor(Position::new(1, 5)).unwrap();
        assert!(buffer.move_word_left());
        assert_eq!(buffer.cursor_position(), Position::new(1, 0));

        assert!(buffer.move_word_left());
        assert_eq!(buffer.cursor_position(), Position::new(0, 14));
    }

    #[test]
    fn word_selection_extends_from_anchor() {
        let mut buffer = TextBuffer::from_text("alpha beta");

        assert!(buffer.extend_selection_word_right());
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::zero(), Position::new(0, 6)))
        );

        assert!(buffer.extend_selection_word_right());
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::zero(), Position::new(0, 10)))
        );

        assert!(buffer.extend_selection_word_left());
        assert_eq!(
            buffer.selection(),
            Some(Selection::new(Position::zero(), Position::new(0, 6)))
        );
    }

    #[test]
    fn delete_word_commands_remove_to_word_boundaries() {
        let mut buffer = TextBuffer::from_text("alpha beta gamma");

        assert!(buffer.delete_word_forward().unwrap());
        assert_eq!(buffer.to_text(), "beta gamma");
        assert_eq!(buffer.cursor_position(), Position::zero());

        buffer
            .set_cursor(Position::new(0, "beta gamma".len()))
            .unwrap();
        assert!(buffer.delete_word_backward().unwrap());
        assert_eq!(buffer.to_text(), "beta ");
        assert_eq!(buffer.cursor_position(), Position::new(0, 5));
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

    #[test]
    fn find_all_returns_utf8_match_ranges() {
        let buffer = TextBuffer::from_text("one\né one é");

        let matches = buffer.find_all("é");

        assert_eq!(
            matches,
            vec![
                SearchMatch {
                    range: TextRange::new(Position::new(1, 0), Position::new(1, 2)),
                },
                SearchMatch {
                    range: TextRange::new(Position::new(1, 7), Position::new(1, 9)),
                },
            ]
        );
    }

    #[test]
    fn find_all_ignores_empty_query() {
        let buffer = TextBuffer::from_text("text");

        assert!(buffer.find_all("").is_empty());
    }

    #[test]
    fn replace_all_is_one_undo_transaction() {
        let mut buffer = TextBuffer::from_text("one two one");

        assert_eq!(buffer.replace_all("one", "uno"), Ok(2));
        assert_eq!(buffer.to_text(), "uno two uno");
        assert!(buffer.can_undo());

        assert_eq!(buffer.undo(), Ok(true));
        assert_eq!(buffer.to_text(), "one two one");

        assert_eq!(buffer.redo(), Ok(true));
        assert_eq!(buffer.to_text(), "uno two uno");
    }

    #[test]
    fn replace_all_reports_zero_for_missing_or_empty_query() {
        let mut buffer = TextBuffer::from_text("abc");

        assert_eq!(buffer.replace_all("z", "x"), Ok(0));
        assert_eq!(buffer.replace_all("", "x"), Ok(0));
        assert_eq!(buffer.to_text(), "abc");
        assert!(!buffer.can_undo());
    }
}
