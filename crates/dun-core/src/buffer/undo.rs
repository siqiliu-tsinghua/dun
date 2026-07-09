use super::edit::end_position_after_text;
use super::model::MergeEdit;
use super::*;

impl TextBuffer {
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
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

    pub(super) fn try_merge_transaction(&mut self, edit: MergeEdit<'_>) -> bool {
        match edit.merge_kind {
            EditMergeKind::InsertRun => self.try_merge_insert_run(&edit),
            EditMergeKind::DeleteBackwardRun | EditMergeKind::DeleteForwardRun => {
                self.try_merge_delete_run(&edit)
            }
            EditMergeKind::None => false,
        }
    }

    fn try_merge_insert_run(&mut self, edit: &MergeEdit<'_>) -> bool {
        if !edit.range.is_empty() || !edit.old_text.is_empty() || edit.before_selection.is_some() {
            return false;
        }

        if !self.redo_stack.is_empty() {
            return false;
        }

        let Some(transaction) = self.undo_stack.last_mut() else {
            return false;
        };
        if transaction.merge_kind != EditMergeKind::InsertRun
            || transaction.after_cursor != edit.before_cursor
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

        if end_position_after_text(previous_range.start, previous_new_text) != edit.range.start {
            return false;
        }

        previous_new_text.push_str(edit.new_text);
        transaction.after_cursor = edit.after_cursor;
        true
    }

    fn try_merge_delete_run(&mut self, edit: &MergeEdit<'_>) -> bool {
        if edit.range.is_empty()
            || edit.old_text.is_empty()
            || !edit.new_text.is_empty()
            || edit.before_selection.is_some()
            || !self.redo_stack.is_empty()
        {
            return false;
        }

        let Some(transaction) = self.undo_stack.last_mut() else {
            return false;
        };
        if transaction.merge_kind != edit.merge_kind
            || transaction.after_cursor != edit.before_cursor
            || transaction.after_selection.is_some()
            || transaction.edits.is_empty()
        {
            return false;
        }

        match edit.merge_kind {
            EditMergeKind::DeleteBackwardRun if edit.range.end != edit.before_cursor => {
                return false;
            }
            EditMergeKind::DeleteForwardRun if edit.range.start != edit.before_cursor => {
                return false;
            }
            EditMergeKind::DeleteBackwardRun | EditMergeKind::DeleteForwardRun => {}
            EditMergeKind::None | EditMergeKind::InsertRun => return false,
        }

        transaction.edits.push(TextEdit::Replace {
            range: edit.range,
            old_text: edit.old_text.to_string(),
            new_text: String::new(),
        });
        transaction.after_cursor = edit.after_cursor;
        true
    }

    pub(super) fn break_undo_merge(&mut self) {
        if let Some(transaction) = self.undo_stack.last_mut() {
            transaction.merge_kind = EditMergeKind::None;
        }
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
}
