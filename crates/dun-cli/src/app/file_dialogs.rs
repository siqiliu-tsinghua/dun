use std::path::Path;

use crate::*;

impl AppState {
    pub(crate) fn start_file_dialog(&mut self, kind: FileDialogKind, initial_input: String) {
        self.start_file_dialog_after(kind, initial_input, None);
    }

    pub(crate) fn default_open_dialog_input(&self) -> String {
        self.recent_file_dialog_input.clone().unwrap_or_default()
    }

    fn start_file_dialog_after(
        &mut self,
        kind: FileDialogKind,
        initial_input: String,
        after_success: Option<PendingAction>,
    ) {
        self.pending_keys.clear();
        self.status_message = None;
        self.confirm = None;
        self.prompt = None;
        self.replace_confirm = None;
        self.pending_replace_query = None;
        self.file_dialog = Some(FileDialogState::new(kind, initial_input, after_success));
    }

    fn start_confirm(&mut self, action: PendingAction, buffer_id: BufferId) {
        self.pending_keys.clear();
        self.status_message = None;
        self.prompt = None;
        self.file_dialog = None;
        self.replace_confirm = None;
        self.confirm = Some(ConfirmState { action, buffer_id });
    }

    pub(crate) fn confirm_focused_dirty(&mut self, action: PendingAction) -> bool {
        let Some(buffer_id) = self.focused_buffer_id() else {
            return false;
        };

        if self
            .buffer_state(buffer_id)
            .is_some_and(|buffer| buffer.buffer.is_dirty())
        {
            self.start_confirm(action, buffer_id);
            true
        } else {
            false
        }
    }

    pub(crate) fn confirm_any_dirty(&mut self, action: PendingAction) -> bool {
        let Some(buffer_id) = self
            .buffers
            .iter()
            .find(|buffer| buffer.buffer.is_dirty())
            .map(|buffer| buffer.id)
        else {
            return false;
        };

        self.focus_window_for_buffer(buffer_id);
        self.start_confirm(action, buffer_id);
        true
    }

    pub(crate) fn confirm_dirty_buffer_losing_its_last_window(&mut self, target: WindowId) -> bool {
        let Some(buffer_id) = self
            .buffers
            .iter()
            .find(|buffer| {
                buffer.buffer.is_dirty()
                    && self
                        .workspace
                        .windows
                        .iter()
                        .any(|window| window.id != target && window.buffer_id == buffer.id)
                    && !self
                        .workspace
                        .windows
                        .iter()
                        .any(|window| window.id == target && window.buffer_id == buffer.id)
            })
            .map(|buffer| buffer.id)
        else {
            return false;
        };

        self.focus_window_for_buffer(buffer_id);
        self.start_confirm(PendingAction::OnlyWindow(target), buffer_id);
        true
    }

    pub(crate) fn handle_confirm_key_event(&mut self, event: CrosstermKeyEvent) -> bool {
        if self.confirm.is_none() {
            return false;
        }

        match event.code {
            CrosstermKeyCode::Esc => self.cancel_confirm(),
            CrosstermKeyCode::Char(ch) => match ch.to_ascii_lowercase() {
                's' => self.save_confirmed_action(),
                'd' => self.discard_confirmed_action(),
                'c' => self.cancel_confirm(),
                _ => {}
            },
            _ => {}
        }

        true
    }

    fn cancel_confirm(&mut self) {
        self.confirm = None;
        self.set_status(
            ui_text::tr(&self.shell.catalog, ui_text::STATUS_UNSAVED_CANCELLED).to_string(),
        );
    }

    fn save_confirmed_action(&mut self) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };

        self.focus_window_for_buffer(confirm.buffer_id);
        if self
            .buffer_state(confirm.buffer_id)
            .and_then(|buffer| buffer.path.as_ref())
            .is_none()
        {
            self.start_file_dialog_after(
                FileDialogKind::SaveAs,
                self.path_text_for_buffer(confirm.buffer_id),
                Some(confirm.action),
            );
            return;
        }

        match self.save_buffer(confirm.buffer_id) {
            Ok(_) => self.continue_pending_action(confirm.action),
            Err(error) => {
                let detail = path_error_status_text(&self.shell.catalog, &error);
                self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_SAVE_FAILED,
                    &[&detail],
                ));
            }
        }
    }

    fn discard_confirmed_action(&mut self) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };

        match confirm.action {
            PendingAction::Quit => self.should_quit = true,
            PendingAction::OnlyWindow(target) => {
                self.workspace.focused = target;
                self.only_focused_window_unchecked(target);
            }
            action => {
                self.focus_window_for_buffer(confirm.buffer_id);
                self.continue_pending_action(action);
            }
        }
    }

    fn continue_pending_action(&mut self, action: PendingAction) {
        match action {
            PendingAction::Quit => {
                if !self.confirm_any_dirty(PendingAction::Quit) {
                    self.should_quit = true;
                }
            }
            PendingAction::New => self.reset_focused_to_untitled(),
            PendingAction::OpenPrompt => {
                self.start_file_dialog(FileDialogKind::Open, self.default_open_dialog_input())
            }
            PendingAction::ReloadBuffer => {
                if let Err(error) = self.reload_focused_buffer() {
                    let detail = path_error_status_text(&self.shell.catalog, &error);
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_RELOAD_FAILED,
                        &[&detail],
                    ));
                }
            }
            PendingAction::CloseFile => self.close_focused_file_unchecked(),
            PendingAction::CloseWindow => self.close_focused_window_unchecked(),
            PendingAction::OnlyWindow(target) => {
                self.workspace.focused = target;
                self.only_focused_window();
            }
        }
    }

    pub(crate) fn handle_file_dialog_key_event(&mut self, event: CrosstermKeyEvent) -> bool {
        if self.file_dialog.is_none() {
            return false;
        }

        if let Some(action) = key_stroke_from_crossterm(event)
            .and_then(|stroke| self.file_dialog_keys.action_for_stroke(stroke))
        {
            self.handle_file_dialog_action(action);
            return true;
        }

        if let Some(ch) = text_input_from_crossterm(event) {
            if let Some(dialog) = &mut self.file_dialog {
                dialog.insert_char(ch);
            }
        }

        self.refresh_prompt_preview();
        true
    }

    fn handle_file_dialog_action(&mut self, action: FileDialogAction) {
        match action {
            FileDialogAction::Cancel => self.cancel_file_dialog(),
            FileDialogAction::Submit => self.submit_file_dialog(),
            FileDialogAction::CompleteForward => self.complete_file_dialog(true),
            FileDialogAction::CompleteBackward => self.complete_file_dialog(false),
            FileDialogAction::ToggleHidden => self.toggle_file_dialog_hidden(),
            FileDialogAction::MoveSelectionUp => self.move_file_dialog_selection(-1),
            FileDialogAction::MoveSelectionDown => self.move_file_dialog_selection(1),
            FileDialogAction::PageSelectionUp => self.page_file_dialog_selection(-1),
            FileDialogAction::PageSelectionDown => self.page_file_dialog_selection(1),
            FileDialogAction::MoveInputLeft => self.move_file_dialog_input_left(),
            FileDialogAction::MoveInputRight => self.move_file_dialog_input_right(),
            FileDialogAction::MoveInputStart => self.move_file_dialog_input_start(),
            FileDialogAction::MoveInputEnd => self.move_file_dialog_input_end(),
            FileDialogAction::DeleteBackward => self.delete_file_dialog_backward(),
            FileDialogAction::DeleteForward => self.delete_file_dialog_forward(),
        }
    }

    fn cancel_file_dialog(&mut self) {
        if let Some(dialog) = self.file_dialog.take() {
            let name = dialog.kind.name(&self.shell.catalog).to_string();
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_DIALOG_CANCELLED,
                &[&name],
            ));
        }
    }

    fn move_file_dialog_selection(&mut self, delta: isize) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.move_selection(delta);
        }
    }

    fn page_file_dialog_selection(&mut self, delta: isize) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.page_selection(delta);
        }
    }

    pub(crate) fn scroll_file_dialog(&mut self, delta: isize) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.scroll(delta);
        }
    }

    fn move_file_dialog_input_left(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.move_input_left();
        }
    }

    fn move_file_dialog_input_right(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.move_input_right();
        }
    }

    fn move_file_dialog_input_start(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.move_input_start();
        }
    }

    fn move_file_dialog_input_end(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.move_input_end();
        }
    }

    fn delete_file_dialog_backward(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.delete_backward();
        }
    }

    fn delete_file_dialog_forward(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.delete_forward();
        }
    }

    fn toggle_file_dialog_hidden(&mut self) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.toggle_hidden();
        }
    }

    fn complete_file_dialog(&mut self, forward: bool) {
        if let Some(dialog) = &mut self.file_dialog {
            dialog.complete(forward);
        }
    }

    pub(crate) fn submit_file_dialog(&mut self) {
        let Some(mut dialog) = self.file_dialog.take() else {
            return;
        };
        let submit = dialog.submit();
        self.finish_file_dialog_submit(dialog, submit);
    }

    pub(crate) fn click_file_dialog_visible_index(&mut self, visible_index: usize) {
        let Some(mut dialog) = self.file_dialog.take() else {
            return;
        };
        let submit = dialog.click_visible_entry(visible_index);
        self.finish_file_dialog_submit(dialog, submit);
    }

    fn finish_file_dialog_submit(&mut self, dialog: FileDialogState, submit: FileDialogSubmit) {
        match submit {
            FileDialogSubmit::Cancel => {
                let name = dialog.kind.name(&self.shell.catalog).to_string();
                self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_DIALOG_CANCELLED,
                    &[&name],
                ));
            }
            FileDialogSubmit::ContinueEditing => {
                self.file_dialog = Some(dialog);
            }
            FileDialogSubmit::Path(path) => match dialog.kind {
                FileDialogKind::Open => {
                    if let Err(error) = self.open_file_path(path.clone()) {
                        let detail = path_error_status_text(&self.shell.catalog, &error);
                        let status = ui_text::tr_fmt(
                            &self.shell.catalog,
                            ui_text::STATUS_OPEN_FAILED,
                            &[&detail],
                        );
                        let mut dialog = dialog;
                        dialog.message = Some(FileDialogMessage::Text(status.clone()));
                        self.file_dialog = Some(dialog);
                        self.set_status(status);
                    } else {
                        self.note_recent_file_dialog_path(&path);
                    }
                }
                FileDialogKind::SaveAs => {
                    if let Err(error) = self.save_focused_buffer_as(path.clone()) {
                        let detail = path_error_status_text(&self.shell.catalog, &error);
                        let status = ui_text::tr_fmt(
                            &self.shell.catalog,
                            ui_text::STATUS_SAVE_AS_FAILED,
                            &[&detail],
                        );
                        let mut dialog = dialog;
                        dialog.message = Some(FileDialogMessage::Text(status.clone()));
                        self.file_dialog = Some(dialog);
                        self.set_status(status);
                    } else if let Some(action) = dialog.after_success {
                        self.note_recent_file_dialog_path(&path);
                        self.continue_pending_action(action);
                    } else {
                        self.note_recent_file_dialog_path(&path);
                    }
                }
            },
        }
    }

    fn note_recent_file_dialog_path(&mut self, path: &Path) {
        self.recent_file_dialog_input = Some(file_dialog_recent_input_for_path(path));
    }
}
