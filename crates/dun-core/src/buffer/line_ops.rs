use super::*;

impl TextBuffer {
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
