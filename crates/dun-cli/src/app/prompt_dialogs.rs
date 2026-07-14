use crate::*;

impl AppState {
    pub(crate) fn start_prompt(&mut self, kind: PromptKind, initial_input: String) {
        self.pending_keys.clear();
        self.status_message = None;
        self.confirm = None;
        self.file_dialog = None;
        self.replace_confirm = None;
        if !matches!(kind, PromptKind::ReplaceWith) {
            self.pending_replace_query = None;
        }
        let preview = self.prompt_preview_for(kind);
        self.prompt = Some(PromptState::new(kind, initial_input, preview));
        self.refresh_prompt_preview();
    }

    fn prompt_preview_for(&self, kind: PromptKind) -> Option<PromptPreviewState> {
        if !matches!(kind, PromptKind::Find | PromptKind::ReplaceFind) {
            return None;
        }

        let buffer_id = self.focused_buffer_id()?;
        let buffer = self.buffer_state(buffer_id)?;
        Some(PromptPreviewState {
            buffer_id,
            cursor: buffer.buffer.cursor_position(),
            selection: buffer.buffer.selection(),
            search: buffer.search.clone(),
        })
    }

    pub(crate) fn refresh_prompt_preview(&mut self) {
        let Some(prompt) = &self.prompt else {
            return;
        };
        if !matches!(prompt.kind, PromptKind::Find | PromptKind::ReplaceFind) {
            return;
        }

        let kind = prompt.kind;
        let input = prompt.input.as_str().trim().to_string();
        let preview = prompt.preview.clone();
        if input.is_empty() {
            self.restore_prompt_preview(preview.as_ref());
            let label = kind.label(&self.shell.catalog).to_string();
            self.status_message = Some(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_PROMPT_TYPE_TO_SEARCH,
                &[&label],
            ));
            return;
        }

        let spec = SearchSpec::parse(&input);
        self.preview_find_query(kind, spec, preview.as_ref());
    }

    fn preview_find_query(
        &mut self,
        kind: PromptKind,
        spec: SearchSpec,
        preview: Option<&PromptPreviewState>,
    ) {
        let buffer_id = preview
            .map(|preview| preview.buffer_id)
            .or_else(|| self.focused_buffer_id());
        let Some(buffer_id) = buffer_id else {
            let name = kind.name(&self.shell.catalog).to_string();
            self.status_message = Some(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_PROMPT_BUFFER_MISSING,
                &[&name],
            ));
            return;
        };
        self.focus_window_for_buffer(buffer_id);

        let Some(buffer) = self.buffer_state_mut(buffer_id) else {
            let name = kind.name(&self.shell.catalog).to_string();
            self.status_message = Some(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_PROMPT_BUFFER_MISSING,
                &[&name],
            ));
            return;
        };
        let matches = buffer
            .buffer
            .find_all_with_options(&spec.query, spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(spec.clone(), matches, None);
            let label = kind.label(&self.shell.catalog).to_string();
            self.status_message = Some(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_PROMPT_NO_MATCHES,
                &[&label, &spec.display()],
            ));
            return;
        }

        let selection = preview
            .and_then(|preview| preview_selection_match(preview.selection, &matches))
            .or_else(|| current_match_selection(&buffer.buffer, &matches))
            .unwrap_or_else(|| {
                let origin = preview
                    .map(|preview| preview.cursor)
                    .unwrap_or_else(|| buffer.buffer.cursor_position());
                choose_search_match(&matches, origin, SearchDirection::Forward)
            });
        let selected = matches[selection.index].range;
        let _ = buffer.buffer.select(selected.start, selected.end);
        let match_count = matches.len();
        buffer.set_search(spec.clone(), matches, Some(selection.index));
        let label = kind.label(&self.shell.catalog).to_string();
        self.status_message = Some(ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_PROMPT_MATCH,
            &[
                &label,
                &(selection.index + 1).to_string(),
                &match_count.to_string(),
                &spec.display(),
            ],
        ));
    }

    fn restore_prompt_preview(&mut self, preview: Option<&PromptPreviewState>) {
        let Some(preview) = preview else {
            return;
        };
        let Some(buffer) = self.buffer_state_mut(preview.buffer_id) else {
            return;
        };
        if let Some(selection) = preview.selection {
            let _ = buffer.buffer.select(selection.anchor, selection.cursor);
        } else {
            let _ = buffer.buffer.set_cursor(preview.cursor);
        }
        buffer.search = preview.search.clone();
    }

    pub(crate) fn handle_paste(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        self.pending_keys.clear();

        if self.confirm.is_some() {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_PASTE_IGNORED_CONFIRMATION,
                )
                .to_string(),
            );
            return;
        }
        if self.replace_confirm.is_some() {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_PASTE_IGNORED_REPLACE).to_string(),
            );
            return;
        }
        if self.buffer_switcher.is_some() {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_PASTE_IGNORED_SWITCHER)
                    .to_string(),
            );
            return;
        }

        if let Some(dialog) = &mut self.file_dialog {
            let text = single_line_paste_text(text);
            dialog.insert_text(&text);
            return;
        }

        if let Some(prompt) = &mut self.prompt {
            prompt.detach_history();
            let text = single_line_paste_text(text);
            prompt.input.insert_str(&text);
            self.refresh_prompt_preview();
            return;
        }

        self.clear_active_menu();
        if self.refuse_edit_in_collapsed_pane() {
            return;
        }
        let Some(buffer) = self.focused_buffer_mut() else {
            return;
        };

        if let Err(error) = buffer.buffer.insert_str(text) {
            let status = ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_PASTE_FAILED,
                &[buffer_error_text(&self.shell.catalog, error)],
            );
            self.set_status(status);
        }
    }

    pub(crate) fn note_right_click_paste(&mut self) {
        self.set_status(
            ui_text::tr(&self.shell.catalog, ui_text::STATUS_PASTE_WAITING).to_string(),
        );
    }

    pub(crate) fn handle_prompt_key_event(&mut self, event: CrosstermKeyEvent) -> bool {
        if self.prompt.is_none() {
            return false;
        }

        match event.code {
            CrosstermKeyCode::Esc => {
                self.cancel_prompt();
            }
            CrosstermKeyCode::Enter => {
                self.submit_prompt();
            }
            CrosstermKeyCode::Up => {
                self.recall_previous_prompt_history();
            }
            CrosstermKeyCode::Down => {
                self.recall_next_prompt_history();
            }
            CrosstermKeyCode::Tab => {
                self.complete_command_line_prompt(true);
            }
            CrosstermKeyCode::BackTab => {
                self.complete_command_line_prompt(false);
            }
            CrosstermKeyCode::Left => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.clear_completion();
                    prompt.input.move_left();
                }
            }
            CrosstermKeyCode::Right => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.clear_completion();
                    prompt.input.move_right();
                }
            }
            CrosstermKeyCode::Home => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.clear_completion();
                    prompt.input.move_start();
                }
            }
            CrosstermKeyCode::End => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.clear_completion();
                    prompt.input.move_end();
                }
            }
            CrosstermKeyCode::Delete => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.detach_history();
                    prompt.clear_completion();
                    prompt.input.delete_forward();
                }
            }
            CrosstermKeyCode::Backspace => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.detach_history();
                    prompt.clear_completion();
                    prompt.input.delete_backward();
                }
            }
            _ => {
                if let Some(ch) = text_input_from_crossterm(event) {
                    if let Some(prompt) = &mut self.prompt {
                        prompt.detach_history();
                        prompt.clear_completion();
                        prompt.input.insert_char(ch);
                    }
                }
            }
        }

        self.refresh_prompt_preview();
        true
    }

    fn complete_command_line_prompt(&mut self, forward: bool) {
        let catalog = &self.shell.catalog;
        let Some(prompt) = &mut self.prompt else {
            return;
        };
        if prompt.kind != PromptKind::CommandLine {
            return;
        }
        let input = prompt.input.as_str().to_string();
        if prompt.input.cursor_index != input.len() {
            prompt.clear_completion();
            self.status_message =
                Some(ui_text::tr(catalog, ui_text::STATUS_COMPLETION_CURSOR_END).to_string());
            return;
        }

        if let Some(replacement) = prompt.next_completion_replacement(&input, forward) {
            prompt.detach_history();
            prompt.input.set_text(replacement);
            self.status_message = prompt
                .completion
                .as_ref()
                .map(|completion| completion.status_text(catalog))
                .or_else(|| {
                    Some(ui_text::tr(catalog, ui_text::STATUS_COMPLETION_READY).to_string())
                });
            return;
        }

        let completion = command_line_completion(&input);
        match completion {
            CommandCompletion::None => {
                prompt.clear_completion();
                self.status_message =
                    Some(ui_text::tr(catalog, ui_text::STATUS_COMPLETION_NO_MATCHES).to_string());
            }
            CommandCompletion::Unique(text) => {
                prompt.detach_history();
                prompt.clear_completion();
                prompt.input.set_text(text);
                self.status_message =
                    Some(ui_text::tr(catalog, ui_text::STATUS_COMPLETION_READY).to_string());
            }
            CommandCompletion::CommonPrefix(text, count) => {
                prompt.detach_history();
                prompt.clear_completion();
                prompt.input.set_text(text);
                self.status_message = Some(ui_text::tr_fmt(
                    catalog,
                    ui_text::STATUS_COMPLETION_MATCHES,
                    &[&count.to_string()],
                ));
            }
            CommandCompletion::Candidates(candidates) => {
                prompt.completion = Some(PromptCompletionState::new(input, candidates));
                self.status_message = prompt
                    .completion
                    .as_ref()
                    .map(|completion| completion.status_text(catalog));
            }
        }
    }

    fn recall_previous_prompt_history(&mut self) {
        let Some(kind) = self
            .prompt
            .as_ref()
            .and_then(|prompt| prompt.kind.history_kind())
        else {
            return;
        };
        let history_len = self.prompt_history_len(kind);
        if history_len == 0 {
            return;
        }

        let next_index = {
            let Some(prompt) = self.prompt_history_prompt_mut(kind) else {
                return;
            };
            let next_index = match prompt.history_index {
                Some(0) => 0,
                Some(index) => index - 1,
                None => {
                    prompt.history_draft = prompt.input.as_str().to_string();
                    history_len - 1
                }
            };
            prompt.history_index = Some(next_index);
            prompt.clear_completion();
            next_index
        };

        let Some(input) = self.prompt_history_entry(kind, next_index) else {
            return;
        };
        if let Some(prompt) = self.prompt_history_prompt_mut(kind) {
            prompt.input.set_text(input);
        }
    }

    fn recall_next_prompt_history(&mut self) {
        let Some(kind) = self
            .prompt
            .as_ref()
            .and_then(|prompt| prompt.kind.history_kind())
        else {
            return;
        };
        let history_len = self.prompt_history_len(kind);
        let (entry_index, draft) = {
            let Some(prompt) = self.prompt_history_prompt_mut(kind) else {
                return;
            };
            let Some(index) = prompt.history_index else {
                return;
            };
            if index + 1 < history_len {
                let next_index = index + 1;
                prompt.history_index = Some(next_index);
                prompt.clear_completion();
                (Some(next_index), None)
            } else {
                prompt.history_index = None;
                prompt.clear_completion();
                (None, Some(std::mem::take(&mut prompt.history_draft)))
            }
        };

        let input = entry_index
            .and_then(|index| self.prompt_history_entry(kind, index))
            .or(draft)
            .unwrap_or_default();
        if let Some(prompt) = self.prompt_history_prompt_mut(kind) {
            prompt.input.set_text(input);
        }
    }

    fn prompt_history_prompt_mut(&mut self, kind: PromptHistoryKind) -> Option<&mut PromptState> {
        self.prompt
            .as_mut()
            .filter(|prompt| prompt.kind.history_kind() == Some(kind))
    }

    fn prompt_history_len(&self, kind: PromptHistoryKind) -> usize {
        self.prompt_history(kind).len()
    }

    fn prompt_history_entry(&self, kind: PromptHistoryKind, index: usize) -> Option<String> {
        self.prompt_history(kind).get(index).cloned()
    }

    fn prompt_history(&self, kind: PromptHistoryKind) -> &[String] {
        match kind {
            PromptHistoryKind::CommandLine => &self.command_history,
            PromptHistoryKind::RunCommand => &self.run_command_history,
        }
    }

    fn prompt_history_mut(&mut self, kind: PromptHistoryKind) -> &mut Vec<String> {
        match kind {
            PromptHistoryKind::CommandLine => &mut self.command_history,
            PromptHistoryKind::RunCommand => &mut self.run_command_history,
        }
    }

    fn cancel_prompt(&mut self) {
        if let Some(prompt) = self.prompt.take() {
            self.restore_prompt_preview(prompt.preview.as_ref());
            if prompt.kind.is_replace() {
                self.pending_replace_query = None;
            }
            let name = prompt.kind.name(&self.shell.catalog).to_string();
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_PROMPT_CANCELLED,
                &[&name],
            ));
        }
    }

    fn submit_prompt(&mut self) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };

        match prompt.kind {
            PromptKind::Find => {
                let input = prompt.input.as_str().trim().to_string();
                let spec = SearchSpec::parse(&input);
                if spec.is_empty() {
                    let name = prompt.kind.name(&self.shell.catalog).to_string();
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_PROMPT_CANCELLED,
                        &[&name],
                    ));
                    return;
                }

                self.last_find_query = Some(input.clone());
                self.commit_find_preview(spec);
            }
            PromptKind::ReplaceFind => {
                let input = prompt.input.as_str().trim().to_string();
                let spec = SearchSpec::parse(&input);
                if spec.is_empty() {
                    self.pending_replace_query = None;
                    let name = prompt.kind.name(&self.shell.catalog).to_string();
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_PROMPT_CANCELLED,
                        &[&name],
                    ));
                    return;
                }

                self.pending_replace_query = Some(input);
                self.start_prompt(PromptKind::ReplaceWith, String::new());
            }
            PromptKind::ReplaceWith => {
                let Some(query) = self.pending_replace_query.take() else {
                    self.set_status(
                        ui_text::tr(&self.shell.catalog, ui_text::STATUS_REPLACE_NO_QUERY)
                            .to_string(),
                    );
                    return;
                };

                self.last_find_query = Some(query.clone());
                self.start_replace_confirmation(
                    SearchSpec::parse(&query),
                    prompt.input.as_str().to_string(),
                );
            }
            PromptKind::GoToLine => {
                let input = prompt.input.as_str().trim();
                if input.is_empty() {
                    let name = prompt.kind.name(&self.shell.catalog).to_string();
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_PROMPT_CANCELLED,
                        &[&name],
                    ));
                    return;
                }

                self.go_to_line(input);
            }
            PromptKind::RunCommand => {
                let input = prompt.input.as_str().trim().to_string();
                if input.is_empty() {
                    let name = prompt.kind.name(&self.shell.catalog).to_string();
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_PROMPT_CANCELLED,
                        &[&name],
                    ));
                    return;
                }

                self.record_prompt_history(PromptHistoryKind::RunCommand, input.clone());
                self.run_external_command_to_buffer(&input);
            }
            PromptKind::CommandLine => {
                let input = prompt.input.as_str().trim().to_string();
                if input.is_empty() {
                    let name = prompt.kind.name(&self.shell.catalog).to_string();
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_PROMPT_CANCELLED,
                        &[&name],
                    ));
                    return;
                }

                self.record_command_history(input.clone());
                self.run_command_line(&input);
            }
        }
    }

    pub(crate) fn record_command_history(&mut self, input: String) {
        self.record_prompt_history(PromptHistoryKind::CommandLine, input);
    }

    fn record_prompt_history(&mut self, kind: PromptHistoryKind, input: String) {
        let history = self.prompt_history_mut(kind);
        if history.last().is_some_and(|previous| previous == &input) {
            return;
        }

        history.push(input);
        if history.len() > COMMAND_HISTORY_LIMIT {
            let overflow = history.len() - COMMAND_HISTORY_LIMIT;
            history.drain(0..overflow);
        }
    }

    #[cfg(test)]
    pub(crate) fn prompt_status_text(&self) -> Option<String> {
        self.prompt
            .as_ref()
            .map(PromptState::status_text)
            .or_else(|| self.file_dialog.as_ref().map(FileDialogState::status_text))
    }

    #[cfg(test)]
    pub(crate) fn confirm_status_text(&self) -> Option<String> {
        if let Some(confirm) = &self.confirm {
            let action = match confirm.action {
                PendingAction::Quit => "Save(s) Quit without saving(d) Cancel(c)",
                PendingAction::New
                | PendingAction::OpenPrompt
                | PendingAction::ReloadBuffer
                | PendingAction::CloseFile
                | PendingAction::CloseWindow
                | PendingAction::OnlyWindow(_) => "Save(s) Discard(d) Cancel(c)",
            };
            return Some(format!(
                "Unsaved changes in {}: {action}",
                self.buffer_display_name(confirm.buffer_id)
            ));
        }

        self.replace_confirm
            .as_ref()
            .map(|confirm| self.replace_confirm_status_text(confirm))
    }

    pub(crate) fn active_overlay(&self) -> Option<UiOverlay> {
        let catalog = &self.shell.catalog;
        if let Some(confirm) = &self.confirm {
            // The (s)/(d)/(c) letters are the keys the dialog answers to;
            // translations change the words, never the letters.
            let action = format!(
                "[{}(s)] [{}(d)] [{}(c)]",
                ui_text::tr(catalog, ui_text::CONFIRM_SAVE),
                ui_text::tr(catalog, ui_text::CONFIRM_DISCARD),
                ui_text::tr(catalog, ui_text::CONFIRM_CANCEL),
            );
            return Some(UiOverlay::message(
                ui_text::tr(catalog, ui_text::CONFIRM_UNSAVED_TITLE),
                vec![ui_text::tr_fmt(
                    catalog,
                    ui_text::CONFIRM_UNSAVED_BODY,
                    &[&self.buffer_display_name(confirm.buffer_id)],
                )],
                vec![action],
            ));
        }

        if let Some(confirm) = &self.replace_confirm {
            return Some(self.replace_confirm_overlay(confirm));
        }

        if let Some(switcher) = &self.buffer_switcher {
            return Some(self.buffer_switcher_overlay(switcher));
        }

        if let Some(dialog) = &self.file_dialog {
            return Some(dialog.overlay(&self.file_dialog_keys, catalog));
        }

        let prompt = self.prompt.as_ref()?;
        let title_key = match prompt.kind {
            PromptKind::CommandLine => ui_text::PROMPT_COMMAND_TITLE,
            PromptKind::Find => ui_text::PROMPT_FIND_TITLE,
            PromptKind::ReplaceFind | PromptKind::ReplaceWith => ui_text::PROMPT_REPLACE_TITLE,
            PromptKind::GoToLine => ui_text::PROMPT_GO_TO_LINE_TITLE,
            PromptKind::RunCommand => ui_text::PROMPT_RUN_COMMAND_TITLE,
        };
        let mut overlay = UiOverlay::prompt(
            ui_text::tr(catalog, title_key),
            prompt.input.as_str().to_string(),
            prompt.input.cursor_display_column(),
        );
        if let Some(completion) = &prompt.completion {
            overlay.lines.push(completion.status_text(catalog));
        }
        Some(overlay)
    }

    fn replace_confirm_overlay(&self, confirm: &ReplaceConfirmState) -> UiOverlay {
        let catalog = &self.shell.catalog;
        let find_display = confirm.spec.display();
        let with_display = replacement_status_text(catalog, &confirm.replacement);
        UiOverlay::message(
            ui_text::tr(catalog, ui_text::CONFIRM_REPLACE_TITLE),
            vec![
                ui_text::tr_fmt(catalog, ui_text::CONFIRM_REPLACE_FIND, &[&find_display]),
                ui_text::tr_fmt(catalog, ui_text::CONFIRM_REPLACE_WITH, &[with_display]),
                self.replace_confirm_status_text(confirm),
            ],
            vec![format!(
                "[{}(r)] [{}(s)] [{}(a)] [{}(c)]",
                ui_text::tr(catalog, ui_text::CONFIRM_REPLACE),
                ui_text::tr(catalog, ui_text::CONFIRM_SKIP),
                ui_text::tr(catalog, ui_text::CONFIRM_ALL),
                ui_text::tr(catalog, ui_text::CONFIRM_CANCEL),
            )],
        )
    }

    fn replace_confirm_status_text(&self, confirm: &ReplaceConfirmState) -> String {
        let catalog = &self.shell.catalog;
        let match_status = self
            .buffer_state(confirm.buffer_id)
            .and_then(|buffer| buffer.search.as_ref())
            .filter(|search| search.spec == confirm.spec)
            .and_then(|search| match (search.matches.len(), search.active_index) {
                (0, _) => None,
                (total, Some(index)) => Some(ui_text::tr_fmt(
                    catalog,
                    ui_text::CONFIRM_MATCH_OF,
                    &[&(index + 1).to_string(), &total.to_string()],
                )),
                (total, None) => Some(ui_text::tr_fmt(
                    catalog,
                    ui_text::CONFIRM_MATCH_TOTAL,
                    &[&total.to_string()],
                )),
            })
            .unwrap_or_else(|| ui_text::tr(catalog, ui_text::CONFIRM_MATCH_NONE).to_string());

        format!(
            "{match_status}{}",
            ui_text::tr_fmt(
                catalog,
                ui_text::CONFIRM_PROGRESS,
                &[&confirm.replaced.to_string(), &confirm.skipped.to_string()],
            )
        )
    }
}
