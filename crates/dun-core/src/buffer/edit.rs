use super::model::MergeEdit;
use super::*;

impl TextBuffer {
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

    pub(super) fn commit_replace(
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
        if self.try_merge_transaction(MergeEdit {
            merge_kind,
            range,
            old_text: &old_text,
            new_text,
            before_cursor,
            after_cursor,
            before_selection,
        }) {
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

    pub(super) fn replace_range_inner(
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
}

pub(super) fn normalize_edit_text(text: &str) -> String {
    if text.contains("\r\n") {
        text.replace("\r\n", "\n")
    } else {
        text.to_string()
    }
}

fn is_mergeable_insert_char(ch: char) -> bool {
    ch != '\n' && ch != '\r' && !ch.is_control()
}

pub(super) fn end_position_after_text(start: Position, text: &str) -> Position {
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
