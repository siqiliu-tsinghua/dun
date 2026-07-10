use crate::*;

impl AppState {
    pub(crate) fn handle_edit_command(&mut self, command: &EditCommand) {
        match command {
            EditCommand::Find => {
                self.start_prompt(
                    PromptKind::Find,
                    self.last_find_query.clone().unwrap_or_default(),
                );
                return;
            }
            EditCommand::FindNext => {
                self.repeat_find(SearchDirection::Forward);
                return;
            }
            EditCommand::FindPrevious => {
                self.repeat_find(SearchDirection::Backward);
                return;
            }
            EditCommand::Replace => {
                self.start_prompt(
                    PromptKind::ReplaceFind,
                    self.last_find_query.clone().unwrap_or_default(),
                );
                return;
            }
            EditCommand::GoToLine => {
                self.start_prompt(PromptKind::GoToLine, String::new());
                return;
            }
            EditCommand::Cut => {
                self.cut_selection();
                return;
            }
            EditCommand::Copy => {
                self.copy_selection();
                return;
            }
            EditCommand::CopyExternal => {
                self.copy_selection_external();
                return;
            }
            EditCommand::CopyLine => {
                self.copy_current_line();
                return;
            }
            EditCommand::Paste => {
                self.paste_internal_clipboard();
                return;
            }
            EditCommand::DeleteLine => {
                self.delete_current_line();
                return;
            }
            EditCommand::MoveLineUp => {
                self.move_current_line(-1);
                return;
            }
            EditCommand::MoveLineDown => {
                self.move_current_line(1);
                return;
            }
            EditCommand::IndentLine => {
                self.indent_selected_lines();
                return;
            }
            EditCommand::OutdentLine => {
                self.outdent_selected_lines();
                return;
            }
            EditCommand::TrimTrailingWhitespace => {
                self.trim_trailing_whitespace();
                return;
            }
            EditCommand::ToggleWordWrap => {
                self.toggle_word_wrap();
                return;
            }
            EditCommand::Undo => {
                self.undo_focused_buffer();
                return;
            }
            EditCommand::Redo => {
                self.redo_focused_buffer();
                return;
            }
            EditCommand::MovePageUp => {
                self.move_focused_page(-1);
                return;
            }
            EditCommand::MovePageDown => {
                self.move_focused_page(1);
                return;
            }
            EditCommand::MoveDocumentStart => {
                self.move_focused_document_edge(false);
                return;
            }
            EditCommand::MoveDocumentEnd => {
                self.move_focused_document_edge(true);
                return;
            }
            EditCommand::ScrollLeft => {
                self.scroll_focused_columns(-1);
                return;
            }
            EditCommand::ScrollRight => {
                self.scroll_focused_columns(1);
                return;
            }
            EditCommand::ExtendSelectionPageUp => {
                self.extend_focused_page(-1);
                return;
            }
            EditCommand::ExtendSelectionPageDown => {
                self.extend_focused_page(1);
                return;
            }
            _ => {}
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            return;
        };

        match command {
            EditCommand::SelectAll => {
                let end = buffer_end_position(&buffer.buffer);
                let _ = buffer.buffer.select(Position::zero(), end);
            }
            EditCommand::SelectLine => {
                let _ = buffer.buffer.select_current_line();
            }
            EditCommand::MoveLeft => {
                buffer.buffer.move_left();
            }
            EditCommand::MoveRight => {
                buffer.buffer.move_right();
            }
            EditCommand::MoveUp => {
                buffer.buffer.move_up();
            }
            EditCommand::MoveDown => {
                buffer.buffer.move_down();
            }
            EditCommand::MoveWordLeft => {
                buffer.buffer.move_word_left();
            }
            EditCommand::MoveWordRight => {
                buffer.buffer.move_word_right();
            }
            EditCommand::MoveLineStart => {
                buffer.buffer.move_to_line_start();
            }
            EditCommand::MoveLineEnd => {
                buffer.buffer.move_to_line_end();
            }
            EditCommand::ExtendSelectionWordLeft => {
                buffer.buffer.extend_selection_word_left();
            }
            EditCommand::ExtendSelectionWordRight => {
                buffer.buffer.extend_selection_word_right();
            }
            EditCommand::InsertNewline => {
                let _ = buffer.buffer.insert_newline();
            }
            EditCommand::DeleteBackward => {
                let _ = buffer.buffer.delete_backward();
            }
            EditCommand::DeleteForward => {
                let _ = buffer.buffer.delete_forward();
            }
            EditCommand::DeleteWordBackward => {
                let _ = buffer.buffer.delete_word_backward();
            }
            EditCommand::DeleteWordForward => {
                let _ = buffer.buffer.delete_word_forward();
            }
            EditCommand::Cut
            | EditCommand::Copy
            | EditCommand::CopyExternal
            | EditCommand::CopyLine
            | EditCommand::Paste
            | EditCommand::DeleteLine
            | EditCommand::MoveLineUp
            | EditCommand::MoveLineDown
            | EditCommand::IndentLine
            | EditCommand::OutdentLine
            | EditCommand::TrimTrailingWhitespace
            | EditCommand::ToggleWordWrap
            | EditCommand::Undo
            | EditCommand::Redo
            | EditCommand::Find
            | EditCommand::FindNext
            | EditCommand::FindPrevious
            | EditCommand::Replace
            | EditCommand::GoToLine
            | EditCommand::MovePageUp
            | EditCommand::MovePageDown
            | EditCommand::MoveDocumentStart
            | EditCommand::MoveDocumentEnd
            | EditCommand::ScrollLeft
            | EditCommand::ScrollRight
            | EditCommand::ExtendSelectionPageUp
            | EditCommand::ExtendSelectionPageDown => {}
        }
    }

    fn copy_current_line(&mut self) {
        let Some(buffer_id) = self.focused_buffer_id() else {
            self.set_status("Copy line failed: focused buffer is missing");
            return;
        };

        let text = self.buffer_state(buffer_id).and_then(|buffer| {
            let range = buffer.buffer.current_line_range();
            buffer.buffer.text_in_range(range).ok()
        });
        match text {
            Some(text) => {
                self.kill_ring = Some(text);
                self.set_status("Copied line");
            }
            None => self.set_status("Copy line failed: focused buffer is missing"),
        }
    }

    fn delete_current_line(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Delete line failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.delete_current_line() {
            Ok(true) => "Deleted line".to_string(),
            Ok(false) => "Delete line: nothing deleted".to_string(),
            Err(error) => format!("Delete line failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn move_current_line(&mut self, direction: isize) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Move line failed: focused buffer is missing");
            return;
        };

        let moved = if direction < 0 {
            buffer.buffer.move_current_line_up()
        } else {
            buffer.buffer.move_current_line_down()
        };
        let status = match moved {
            Ok(true) => {
                if direction < 0 {
                    "Moved line up".to_string()
                } else {
                    "Moved line down".to_string()
                }
            }
            Ok(false) if direction < 0 => "Move line: already at top".to_string(),
            Ok(false) => "Move line: already at bottom".to_string(),
            Err(error) => format!("Move line failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn indent_selected_lines(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Indent failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.indent_selected_lines(EDITOR_INDENT) {
            Ok(0) => "Indent: nothing changed".to_string(),
            Ok(count) => format!("Indented {count} line(s)"),
            Err(error) => format!("Indent failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn outdent_selected_lines(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Outdent failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.outdent_selected_lines(EDITOR_INDENT.len()) {
            Ok(0) => "Outdent: nothing changed".to_string(),
            Ok(count) => format!("Outdented {count} line(s)"),
            Err(error) => format!("Outdent failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn trim_trailing_whitespace(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Trim failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.trim_trailing_whitespace() {
            Ok(0) => "Trim: no trailing whitespace".to_string(),
            Ok(count) => format!("Trimmed trailing whitespace on {count} line(s)"),
            Err(error) => format!("Trim failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn toggle_word_wrap(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Wrap failed: focused buffer is missing");
            return;
        };

        buffer.word_wrap = !buffer.word_wrap;
        if buffer.word_wrap {
            buffer.first_column = 0;
            buffer.first_visual_row = 0;
            self.set_status("Word wrap on");
        } else {
            buffer.first_visual_row = 0;
            self.set_status("Word wrap off");
        }
    }

    fn undo_focused_buffer(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Undo failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.undo() {
            Ok(true) => "Undo".to_string(),
            Ok(false) => "Nothing to undo".to_string(),
            Err(error) => format!("Undo failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn redo_focused_buffer(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Redo failed: focused buffer is missing");
            return;
        };

        let status = match buffer.buffer.redo() {
            Ok(true) => "Redo".to_string(),
            Ok(false) => "Nothing to redo".to_string(),
            Err(error) => format!("Redo failed: {}", buffer_error_text(error)),
        };
        self.set_status(status);
    }

    fn move_focused_page(&mut self, direction: isize) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let body_height = context.body_height;
        let page_lines = body_height.saturating_sub(1).max(1);
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let moved = if buffer.word_wrap {
            buffer.move_wrapped_page(direction, page_lines, context.body_width)
        } else if direction < 0 {
            buffer.move_page_up(page_lines)
        } else {
            buffer.move_page_down(page_lines)
        };
        buffer.ensure_cursor_visible(body_height, context.body_width);
        moved
    }

    fn move_focused_document_edge(&mut self, end: bool) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let target = if end {
            buffer_end_position(&buffer.buffer)
        } else {
            Position::zero()
        };
        let moved =
            buffer.buffer.cursor_position() != target || buffer.buffer.selection().is_some();
        let _ = buffer.buffer.set_cursor(target);
        buffer.ensure_cursor_visible(context.body_height, context.body_width);
        moved
    }

    pub(crate) fn scroll_focused_columns(&mut self, direction: isize) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let step = context.body_width.saturating_div(2).max(1) as isize;
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let moved = buffer.scroll_view_columns(direction.saturating_mul(step), context.body_width);
        let first_column = buffer.first_column;
        let status = if moved {
            if direction < 0 {
                format!("Scrolled left to column {}", first_column + 1)
            } else {
                format!("Scrolled right to column {}", first_column + 1)
            }
        } else if direction < 0 {
            "Already at left edge".to_string()
        } else {
            "Already at right edge".to_string()
        };
        self.set_status(status);
        moved
    }

    pub(crate) fn extend_focused_page(&mut self, direction: isize) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let body_height = context.body_height;
        let page_lines = body_height.saturating_sub(1).max(1);
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let moved = if buffer.word_wrap {
            buffer.extend_wrapped_page(direction, page_lines, context.body_width)
        } else if direction < 0 {
            buffer.extend_page_up(page_lines)
        } else {
            buffer.extend_page_down(page_lines)
        };
        buffer.ensure_cursor_visible(body_height, context.body_width);
        moved
    }

    fn copy_selection(&mut self) {
        match self.focused_selection_text() {
            Ok(text) => {
                self.kill_ring = Some(text);
                self.set_status("Copied selection");
            }
            Err(CopyTextError::MissingBuffer) => {
                self.set_status("Copy failed: focused buffer is missing")
            }
            Err(CopyTextError::NoSelection) => self.set_status("Copy: no selection"),
            Err(CopyTextError::Buffer(error)) => {
                self.set_status(format!("Copy failed: {}", buffer_error_text(error)))
            }
        }
    }

    fn copy_selection_external(&mut self) {
        match self.focused_selection_text() {
            Ok(text) => self.copy_text_external(text, "selection"),
            Err(CopyTextError::MissingBuffer) => {
                self.set_status("External copy failed: focused buffer is missing")
            }
            Err(CopyTextError::NoSelection) => self.set_status("External copy: no selection"),
            Err(CopyTextError::Buffer(error)) => self.set_status(format!(
                "External copy failed: {}",
                buffer_error_text(error)
            )),
        }
    }

    fn focused_selection_text(&self) -> Result<String, CopyTextError> {
        let Some(buffer) = self.focused_buffer() else {
            return Err(CopyTextError::MissingBuffer);
        };
        let Some(range) = buffer
            .buffer
            .selection_range()
            .filter(|range| !range.is_empty())
        else {
            return Err(CopyTextError::NoSelection);
        };

        buffer
            .buffer
            .text_in_range(range)
            .map_err(CopyTextError::Buffer)
    }

    fn copy_text_external(&mut self, text: String, label: &str) {
        self.kill_ring = Some(text.clone());
        let byte_len = text.len();
        if !self.clipboard.osc52.enabled {
            self.set_status(format!("External copy disabled: copied {label} internally"));
            return;
        }
        if byte_len > self.clipboard.osc52.max_bytes {
            self.set_status(format!(
                "External copy failed: {label} is {byte_len} bytes; limit is {}",
                self.clipboard.osc52.max_bytes
            ));
            return;
        }

        self.runtime_action = Some(RuntimeAction::WriteTerminal(osc52_copy_sequence(&text)));
        self.set_status(format!("Copied {label} to external clipboard"));
    }

    fn cut_selection(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Cut failed: focused buffer is missing");
            return;
        };

        if buffer.buffer.is_read_only() {
            self.set_status("Cut failed: buffer is read-only");
            return;
        }

        let Some(range) = buffer
            .buffer
            .selection_range()
            .filter(|range| !range.is_empty())
        else {
            self.set_status("Cut: no selection");
            return;
        };

        let text = match buffer.buffer.text_in_range(range) {
            Ok(text) => text,
            Err(error) => {
                self.set_status(format!("Cut failed: {}", buffer_error_text(error)));
                return;
            }
        };

        match buffer.buffer.delete_range(range) {
            Ok(true) => {
                self.kill_ring = Some(text);
                self.set_status("Cut selection");
            }
            Ok(false) => self.set_status("Cut: no selection"),
            Err(error) => self.set_status(format!("Cut failed: {}", buffer_error_text(error))),
        }
    }

    fn paste_internal_clipboard(&mut self) {
        let Some(text) = self.kill_ring.clone() else {
            self.set_status("Paste: internal clipboard empty; use terminal paste");
            return;
        };
        if text.is_empty() {
            self.set_status("Paste: internal clipboard empty; use terminal paste");
            return;
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status("Paste failed: focused buffer is missing");
            return;
        };

        match buffer.buffer.insert_str(&text) {
            Ok(()) => self.set_status("Pasted selection"),
            Err(error) => self.set_status(format!("Paste failed: {}", buffer_error_text(error))),
        }
    }

    pub(crate) fn handle_text_input(&mut self, ch: char) {
        if let Some(buffer) = self.focused_buffer_mut() {
            let _ = buffer.buffer.insert_char(ch);
        }
    }
}
