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
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_COPY_LINE_BUFFER_MISSING,
                )
                .to_string(),
            );
            return;
        };

        let text = self.buffer_state(buffer_id).and_then(|buffer| {
            let range = buffer.buffer.current_line_range();
            buffer.buffer.text_in_range(range).ok()
        });
        match text {
            Some(text) => {
                self.kill_ring = Some(text);
                self.set_status(
                    ui_text::tr(&self.shell.catalog, ui_text::STATUS_COPY_LINE_COPIED).to_string(),
                );
            }
            None => self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_COPY_LINE_BUFFER_MISSING,
                )
                .to_string(),
            ),
        }
    }

    fn delete_current_line(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_DELETE_LINE_BUFFER_MISSING,
                )
                .to_string(),
            );
            return;
        };

        let status = match buffer.buffer.delete_current_line() {
            Ok(true) => "Deleted line".to_string(),
            Ok(false) => "Delete line: nothing deleted".to_string(),
            Err(error) => ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_DELETE_LINE_FAILED,
                &[buffer_error_text(&self.shell.catalog, error)],
            ),
        };
        self.set_status(status);
    }

    fn move_current_line(&mut self, direction: isize) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_MOVE_LINE_BUFFER_MISSING,
                )
                .to_string(),
            );
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
            Err(error) => ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_MOVE_LINE_FAILED,
                &[buffer_error_text(&self.shell.catalog, error)],
            ),
        };
        self.set_status(status);
    }

    fn indent_selected_lines(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_INDENT_BUFFER_MISSING).to_string(),
            );
            return;
        };

        let status = match buffer.buffer.indent_selected_lines(EDITOR_INDENT) {
            Ok(0) => "Indent: nothing changed".to_string(),
            Ok(count) => format!("Indented {count} line(s)"),
            Err(error) => ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_INDENT_FAILED,
                &[buffer_error_text(&self.shell.catalog, error)],
            ),
        };
        self.set_status(status);
    }

    fn outdent_selected_lines(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_OUTDENT_BUFFER_MISSING)
                    .to_string(),
            );
            return;
        };

        let status = match buffer.buffer.outdent_selected_lines(EDITOR_INDENT.len()) {
            Ok(0) => "Outdent: nothing changed".to_string(),
            Ok(count) => format!("Outdented {count} line(s)"),
            Err(error) => ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_OUTDENT_FAILED,
                &[buffer_error_text(&self.shell.catalog, error)],
            ),
        };
        self.set_status(status);
    }

    fn trim_trailing_whitespace(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_TRIM_BUFFER_MISSING).to_string(),
            );
            return;
        };

        let status = match buffer.buffer.trim_trailing_whitespace() {
            Ok(0) => "Trim: no trailing whitespace".to_string(),
            Ok(count) => format!("Trimmed trailing whitespace on {count} line(s)"),
            Err(error) => ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_TRIM_FAILED,
                &[buffer_error_text(&self.shell.catalog, error)],
            ),
        };
        self.set_status(status);
    }

    fn undo_focused_buffer(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_UNDO_BUFFER_MISSING).to_string(),
            );
            return;
        };

        let status = match buffer.buffer.undo() {
            Ok(true) => "Undo".to_string(),
            Ok(false) => "Nothing to undo".to_string(),
            Err(error) => ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_UNDO_FAILED,
                &[buffer_error_text(&self.shell.catalog, error)],
            ),
        };
        self.set_status(status);
    }

    fn redo_focused_buffer(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_REDO_BUFFER_MISSING).to_string(),
            );
            return;
        };

        let status = match buffer.buffer.redo() {
            Ok(true) => "Redo".to_string(),
            Ok(false) => "Nothing to redo".to_string(),
            Err(error) => ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_REDO_FAILED,
                &[buffer_error_text(&self.shell.catalog, error)],
            ),
        };
        self.set_status(status);
    }

    fn copy_selection(&mut self) {
        match self.focused_selection_text() {
            Ok(text) => {
                self.kill_ring = Some(text);
                self.set_status(
                    ui_text::tr(&self.shell.catalog, ui_text::STATUS_COPY_COPIED).to_string(),
                );
            }
            Err(CopyTextError::MissingBuffer) => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_COPY_BUFFER_MISSING).to_string(),
            ),
            Err(CopyTextError::NoSelection) => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_COPY_NO_SELECTION).to_string(),
            ),
            Err(CopyTextError::Buffer(error)) => {
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_COPY_FAILED,
                    &[buffer_error_text(&self.shell.catalog, error)],
                );
                self.set_status(status);
            }
        }
    }

    fn copy_selection_external(&mut self) {
        match self.focused_selection_text() {
            Ok(text) => self.copy_text_external(text),
            Err(CopyTextError::MissingBuffer) => self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_EXTERNAL_COPY_BUFFER_MISSING,
                )
                .to_string(),
            ),
            Err(CopyTextError::NoSelection) => self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_EXTERNAL_COPY_NO_SELECTION,
                )
                .to_string(),
            ),
            Err(CopyTextError::Buffer(error)) => {
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_EXTERNAL_COPY_FAILED,
                    &[buffer_error_text(&self.shell.catalog, error)],
                );
                self.set_status(status);
            }
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

    fn copy_text_external(&mut self, text: String) {
        self.kill_ring = Some(text.clone());
        let byte_len = text.len();
        if !self.clipboard.osc52.enabled {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_EXTERNAL_COPY_DISABLED)
                    .to_string(),
            );
            return;
        }
        if byte_len > self.clipboard.osc52.max_bytes {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_EXTERNAL_COPY_TOO_LARGE,
                &[
                    &byte_len.to_string(),
                    &self.clipboard.osc52.max_bytes.to_string(),
                ],
            ));
            return;
        }

        self.runtime_action = Some(RuntimeAction::WriteTerminal(osc52_copy_sequence(&text)));
        self.set_status(
            ui_text::tr(&self.shell.catalog, ui_text::STATUS_EXTERNAL_COPY_COPIED).to_string(),
        );
    }

    fn cut_selection(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_CUT_BUFFER_MISSING).to_string(),
            );
            return;
        };

        if buffer.buffer.is_read_only() {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_CUT_READ_ONLY).to_string(),
            );
            return;
        }

        let Some(range) = buffer
            .buffer
            .selection_range()
            .filter(|range| !range.is_empty())
        else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_CUT_NO_SELECTION).to_string(),
            );
            return;
        };

        let text = match buffer.buffer.text_in_range(range) {
            Ok(text) => text,
            Err(error) => {
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_CUT_FAILED,
                    &[buffer_error_text(&self.shell.catalog, error)],
                );
                self.set_status(status);
                return;
            }
        };

        match buffer.buffer.delete_range(range) {
            Ok(true) => {
                self.kill_ring = Some(text);
                self.set_status(
                    ui_text::tr(&self.shell.catalog, ui_text::STATUS_CUT_SELECTION).to_string(),
                );
            }
            Ok(false) => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_CUT_NO_SELECTION).to_string(),
            ),
            Err(error) => {
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_CUT_FAILED,
                    &[buffer_error_text(&self.shell.catalog, error)],
                );
                self.set_status(status);
            }
        }
    }

    fn paste_internal_clipboard(&mut self) {
        let Some(text) = self.kill_ring.clone() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_PASTE_EMPTY).to_string(),
            );
            return;
        };
        if text.is_empty() {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_PASTE_EMPTY).to_string(),
            );
            return;
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_PASTE_BUFFER_MISSING).to_string(),
            );
            return;
        };

        match buffer.buffer.insert_str(&text) {
            Ok(()) => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_PASTE_SELECTION).to_string(),
            ),
            Err(error) => {
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_PASTE_FAILED,
                    &[buffer_error_text(&self.shell.catalog, error)],
                );
                self.set_status(status);
            }
        }
    }

    pub(crate) fn handle_text_input(&mut self, ch: char) {
        if self.refuse_edit_in_collapsed_pane() {
            return;
        }
        if let Some(buffer) = self.focused_buffer_mut() {
            let _ = buffer.buffer.insert_char(ch);
        }
    }

    /// A collapsed pane draws no body, so nothing may be edited through it.
    /// Without this, keystrokes kept editing the buffer behind the empty box:
    /// the user blind-typed into a file they could not see, and the dirty
    /// marker in the title was the only hint. The menu behaviour matrix caught
    /// it. Looking is still allowed, and so are the window commands -- expand
    /// is how you get out.
    pub(crate) fn refuse_edit_in_collapsed_pane(&mut self) -> bool {
        if !self.workspace.focused_is_collapsed() {
            return false;
        }

        // Name the key the user actually has bound, not the default: a message
        // that hardcodes `Ctrl+X,P` lies to anyone who remapped it.
        let expand = self
            .shell
            .keymap
            .sequence_for_command(&EditorCommand::Window(WindowCommand::Expand))
            .map(ToString::to_string);
        let status = match expand {
            Some(expand) => ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_PANE_COLLAPSED_WITH_KEY,
                &[&expand],
            ),
            None => ui_text::tr(&self.shell.catalog, ui_text::STATUS_PANE_COLLAPSED).to_string(),
        };
        self.set_status(status);
        true
    }
}
