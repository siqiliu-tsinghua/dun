use crate::*;

impl AppState {
    pub(crate) fn commit_find_preview(&mut self, spec: SearchSpec) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_FIND_BUFFER_MISSING).to_string(),
            );
            return;
        };
        let matches = buffer
            .buffer
            .find_all_with_options(&spec.query, spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(spec.clone(), matches, None);
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_FIND_NO_MATCHES,
                &[&spec.display()],
            ));
            return;
        }

        let selection = current_match_selection(&buffer.buffer, &matches).unwrap_or_else(|| {
            choose_search_match(
                &matches,
                buffer.buffer.cursor_position(),
                SearchDirection::Forward,
            )
        });
        let selected = matches[selection.index].range;
        let _ = buffer.buffer.select(selected.start, selected.end);
        let match_count = matches.len();
        buffer.set_search(spec.clone(), matches, Some(selection.index));
        self.set_status(ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_FIND_MATCH,
            &[
                &(selection.index + 1).to_string(),
                &match_count.to_string(),
                &spec.display(),
            ],
        ));
    }

    pub(crate) fn start_replace_confirmation(&mut self, spec: SearchSpec, replacement: String) {
        if spec.is_empty() {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_REPLACE_NO_QUERY).to_string(),
            );
            return;
        }

        let Some(buffer_id) = self.focused_buffer_id() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_REPLACE_BUFFER_MISSING)
                    .to_string(),
            );
            return;
        };
        self.replace_confirm = Some(ReplaceConfirmState {
            buffer_id,
            spec,
            replacement,
            replaced: 0,
            skipped: 0,
            skipped_in_cycle: 0,
        });
        if !self.select_replace_confirm_match(SearchDirection::Forward) {
            self.replace_confirm = None;
        }
    }

    pub(crate) fn handle_replace_confirm_key_event(&mut self, event: CrosstermKeyEvent) -> bool {
        if self.replace_confirm.is_none() {
            return false;
        }

        match event.code {
            CrosstermKeyCode::Esc => self.cancel_replace_confirmation(),
            CrosstermKeyCode::Enter => self.replace_confirm_current(),
            CrosstermKeyCode::Char(ch) => match ch.to_ascii_lowercase() {
                'r' => self.replace_confirm_current(),
                's' => self.skip_replace_confirm_current(),
                'a' => self.replace_confirm_all(),
                'c' => self.cancel_replace_confirmation(),
                _ => {}
            },
            _ => {}
        }

        true
    }

    fn cancel_replace_confirmation(&mut self) {
        let Some(confirm) = self.replace_confirm.take() else {
            return;
        };
        self.set_status(ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_REPLACE_CANCELLED,
            &[&confirm.replaced.to_string(), &confirm.skipped.to_string()],
        ));
    }

    fn select_replace_confirm_match(&mut self, direction: SearchDirection) -> bool {
        let Some(confirm) = self.replace_confirm.clone() else {
            return false;
        };
        self.focus_window_for_buffer(confirm.buffer_id);
        let Some(buffer) = self.buffer_state_mut(confirm.buffer_id) else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_REPLACE_BUFFER_MISSING)
                    .to_string(),
            );
            return false;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&confirm.spec.query, confirm.spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(confirm.spec.clone(), matches, None);
            if confirm.replaced == 0 && confirm.skipped == 0 {
                self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_REPLACE_NO_MATCHES,
                    &[&confirm.spec.display()],
                ));
            } else {
                self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_REPLACE_DONE,
                    &[&confirm.replaced.to_string(), &confirm.skipped.to_string()],
                ));
            }
            return false;
        }

        let selection = current_match_selection(&buffer.buffer, &matches).unwrap_or_else(|| {
            let origin = match direction {
                SearchDirection::Forward => buffer
                    .buffer
                    .selection_range()
                    .map(|range| range.end)
                    .unwrap_or_else(|| buffer.buffer.cursor_position()),
                SearchDirection::Backward => buffer
                    .buffer
                    .selection_range()
                    .map(|range| range.start)
                    .unwrap_or_else(|| buffer.buffer.cursor_position()),
            };
            choose_search_match(&matches, origin, direction)
        });
        let selected = matches[selection.index].range;
        let _ = buffer.buffer.select(selected.start, selected.end);
        let match_count = matches.len();
        buffer.set_search(confirm.spec.clone(), matches, Some(selection.index));
        self.status_message = Some(format!(
            "Replace confirm: {}/{} {} -> {}",
            selection.index + 1,
            match_count,
            confirm.spec.display(),
            replacement_status_text(&confirm.replacement)
        ));
        true
    }

    fn replace_confirm_current(&mut self) {
        let Some(mut confirm) = self.replace_confirm.clone() else {
            return;
        };
        self.focus_window_for_buffer(confirm.buffer_id);
        let Some(buffer) = self.buffer_state_mut(confirm.buffer_id) else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_REPLACE_BUFFER_MISSING)
                    .to_string(),
            );
            self.replace_confirm = None;
            return;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&confirm.spec.query, confirm.spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(confirm.spec.clone(), matches, None);
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_REPLACE_DONE,
                &[&confirm.replaced.to_string(), &confirm.skipped.to_string()],
            ));
            self.replace_confirm = None;
            return;
        }

        let selection = current_match_selection(&buffer.buffer, &matches).unwrap_or_else(|| {
            choose_search_match(
                &matches,
                buffer.buffer.cursor_position(),
                SearchDirection::Forward,
            )
        });
        let target = matches[selection.index].range;
        match buffer.buffer.replace_range(target, &confirm.replacement) {
            Ok(()) => {
                confirm.replaced += 1;
                confirm.skipped_in_cycle = 0;
                self.replace_confirm = Some(confirm);
                if !self.select_replace_confirm_match(SearchDirection::Forward) {
                    self.replace_confirm = None;
                }
            }
            Err(error) => {
                self.replace_confirm = None;
                self.set_status(format!("Replace failed: {}", buffer_error_text(error)));
            }
        }
    }

    fn skip_replace_confirm_current(&mut self) {
        let Some(mut confirm) = self.replace_confirm.clone() else {
            return;
        };
        self.focus_window_for_buffer(confirm.buffer_id);
        let Some(buffer) = self.buffer_state_mut(confirm.buffer_id) else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_REPLACE_BUFFER_MISSING)
                    .to_string(),
            );
            self.replace_confirm = None;
            return;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&confirm.spec.query, confirm.spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(confirm.spec.clone(), matches, None);
            self.replace_confirm = None;
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_REPLACE_DONE,
                &[&confirm.replaced.to_string(), &confirm.skipped.to_string()],
            ));
            return;
        }
        if matches.len() <= 1 || confirm.skipped_in_cycle + 1 >= matches.len() {
            confirm.skipped += 1;
            self.replace_confirm = None;
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_REPLACE_DONE,
                &[&confirm.replaced.to_string(), &confirm.skipped.to_string()],
            ));
            return;
        }

        if let Some(selection) = current_match_selection(&buffer.buffer, &matches) {
            let _ = buffer.buffer.set_cursor(matches[selection.index].range.end);
        }
        confirm.skipped += 1;
        confirm.skipped_in_cycle += 1;
        self.replace_confirm = Some(confirm);
        let _ = self.select_replace_confirm_match(SearchDirection::Forward);
    }

    fn replace_confirm_all(&mut self) {
        let Some(confirm) = self.replace_confirm.take() else {
            return;
        };
        self.focus_window_for_buffer(confirm.buffer_id);
        let Some(buffer) = self.buffer_state_mut(confirm.buffer_id) else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_REPLACE_ALL_BUFFER_MISSING,
                )
                .to_string(),
            );
            return;
        };

        match buffer.buffer.replace_all_with_options(
            &confirm.spec.query,
            &confirm.replacement,
            confirm.spec.options,
        ) {
            Ok(count) => {
                let new_matches = buffer
                    .buffer
                    .find_all_with_options(&confirm.spec.query, confirm.spec.options);
                let remaining = new_matches.len();
                buffer.set_search(confirm.spec.clone(), new_matches, None);
                let total = confirm.replaced + count;
                let suffix = if remaining == 0 {
                    String::new()
                } else {
                    format!("; {remaining} matches remain")
                };
                self.set_status(format!(
                    "Replace All: {total} {} -> {}{suffix}",
                    confirm.spec.display(),
                    replacement_status_text(&confirm.replacement)
                ));
            }
            Err(error) => {
                self.set_status(format!("Replace All failed: {}", buffer_error_text(error)))
            }
        }
    }

    pub(crate) fn repeat_find(&mut self, direction: SearchDirection) {
        let Some(query) = self.last_find_query.clone() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_FIND_NO_QUERY).to_string(),
            );
            return;
        };

        let spec = SearchSpec::parse(&query);
        if spec.is_empty() {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_FIND_NO_QUERY).to_string(),
            );
            return;
        }

        self.find_in_focused_buffer(spec, direction);
    }

    pub(crate) fn open_search_results_screen(&mut self) {
        let Some(source_buffer_id) = self.search_results_source_for_command() else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_RESULTS_FOCUSED_BUFFER_MISSING,
                )
                .to_string(),
            );
            return;
        };
        let Some((spec, matches)) = self.search_results_for_source(source_buffer_id) else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_RESULTS_NO_QUERY).to_string(),
            );
            return;
        };
        if matches.is_empty() {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_RESULTS_NO_MATCHES,
                &[&spec.display()],
            ));
            return;
        }

        let source_name = self.buffer_display_name(source_buffer_id);
        let Some(source) = self.buffer_state(source_buffer_id) else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_RESULTS_SOURCE_BUFFER_MISSING,
                )
                .to_string(),
            );
            return;
        };
        let text = search_results_text(&source_name, &spec, &matches, &source.buffer);
        self.search_results_source = Some(source_buffer_id);
        let title =
            ui_text::tr(&self.shell.catalog, ui_text::WINDOW_SEARCH_RESULTS_TITLE).to_string();
        self.open_read_only_aux_window(
            WindowKind::SearchResults,
            &title,
            search_results_buffer(&text),
        );
        self.set_status(ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_RESULTS_MATCHES,
            &[&matches.len().to_string()],
        ));
    }

    fn search_results_source_for_command(&self) -> Option<BufferId> {
        let focused = self.workspace.focused_window().ok()?;
        if focused.kind == WindowKind::SearchResults {
            return self.search_results_source;
        }
        Some(focused.buffer_id)
    }

    fn search_results_for_source(
        &self,
        source_buffer_id: BufferId,
    ) -> Option<(SearchSpec, Vec<SearchMatch>)> {
        let buffer = self.buffer_state(source_buffer_id)?;
        if let Some(search) = &buffer.search {
            return Some((search.spec.clone(), search.matches.clone()));
        }
        let query = self.last_find_query.as_ref()?;
        let spec = SearchSpec::parse(query);
        if spec.is_empty() {
            return None;
        }
        Some((
            spec.clone(),
            buffer
                .buffer
                .find_all_with_options(&spec.query, spec.options),
        ))
    }

    pub(crate) fn jump_search_result(&mut self, target: &str) {
        let Some(source_buffer_id) = self
            .search_results_source_for_command()
            .or(self.search_results_source)
        else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_RESULTS_SOURCE_BUFFER_MISSING,
                )
                .to_string(),
            );
            return;
        };
        let Some((spec, matches)) = self.search_results_for_source(source_buffer_id) else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_RESULTS_NO_QUERY).to_string(),
            );
            return;
        };
        if matches.is_empty() {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_RESULTS_NO_MATCHES,
                &[&spec.display()],
            ));
            return;
        }
        let Ok(number) = target.parse::<usize>() else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_RESULTS_MATCH_NUMBER_EXPECTED,
                )
                .to_string(),
            );
            return;
        };
        let Some(index) = number.checked_sub(1).filter(|index| *index < matches.len()) else {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_RESULTS_OUT_OF_RANGE,
                &[&number.to_string()],
            ));
            return;
        };

        if !self.focus_window_for_buffer(source_buffer_id) {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_RESULTS_SOURCE_WINDOW_MISSING,
                )
                .to_string(),
            );
            return;
        }
        let selected = matches[index].range;
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: source_buffer_id,
                body_height: 1,
                body_width: 1,
            });
        if let Some(buffer) = self.buffer_state_mut(source_buffer_id) {
            let _ = buffer.buffer.select(selected.start, selected.end);
            buffer.set_search(spec.clone(), matches.clone(), Some(index));
            buffer.ensure_cursor_visible(context.body_height, context.body_width);
        }
        self.set_status(ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_RESULTS_SELECTED,
            &[
                &(index + 1).to_string(),
                &matches.len().to_string(),
                &spec.display(),
            ],
        ));
    }

    pub(crate) fn jump_current_search_result(&mut self) {
        let label =
            ui_text::tr(&self.shell.catalog, ui_text::WINDOW_SEARCH_RESULTS_TITLE).to_string();
        let Some(index) = self.current_or_next_numbered_aux_index(&label) else {
            return;
        };
        self.jump_search_result(&(index + 1).to_string());
    }

    pub(crate) fn find_in_focused_buffer(&mut self, spec: SearchSpec, direction: SearchDirection) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_FIND_BUFFER_MISSING).to_string(),
            );
            return;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&spec.query, spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(spec.clone(), matches, None);
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_FIND_NO_MATCHES,
                &[&spec.display()],
            ));
            return;
        }

        let origin = match direction {
            SearchDirection::Forward => buffer
                .buffer
                .selection_range()
                .map(|range| range.end)
                .unwrap_or_else(|| buffer.buffer.cursor_position()),
            SearchDirection::Backward => buffer
                .buffer
                .selection_range()
                .map(|range| range.start)
                .unwrap_or_else(|| buffer.buffer.cursor_position()),
        };
        let selection = choose_search_match(&matches, origin, direction);
        let selected = matches[selection.index].range;
        let _ = buffer.buffer.select(selected.start, selected.end);
        let match_count = matches.len();
        buffer.set_search(spec.clone(), matches, Some(selection.index));

        let key = if selection.wrapped {
            ui_text::STATUS_FIND_MATCH_WRAPPED
        } else {
            ui_text::STATUS_FIND_MATCH
        };
        self.set_status(ui_text::tr_fmt(
            &self.shell.catalog,
            key,
            &[
                &(selection.index + 1).to_string(),
                &match_count.to_string(),
                &spec.display(),
            ],
        ));
    }

    pub(crate) fn current_or_next_numbered_aux_index(&mut self, label: &str) -> Option<usize> {
        let buffer_id = self.workspace.focused_window().ok()?.buffer_id;
        let current_line = self.buffer_state(buffer_id)?.buffer.cursor_position().line;
        if let Some(index) = self
            .buffer_state(buffer_id)
            .and_then(|buffer| buffer.buffer.line(current_line))
            .and_then(numbered_list_index_for_line)
        {
            return Some(index);
        }

        self.move_focused_numbered_aux_row(1, label)
    }

    pub(crate) fn move_focused_numbered_aux_row(
        &mut self,
        delta: isize,
        label: &str,
    ) -> Option<usize> {
        let buffer_id = self.workspace.focused_window().ok()?.buffer_id;
        let rows = self
            .buffer_state(buffer_id)
            .map(|buffer| numbered_list_rows(&buffer.buffer))
            .unwrap_or_default();
        if rows.is_empty() {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_LIST_NO_ENTRIES,
                &[label],
            ));
            return None;
        }

        let current_line = self.buffer_state(buffer_id)?.buffer.cursor_position().line;
        let current_row = rows.iter().position(|row| row.line == current_line);
        let next_row = if let Some(current_row) = current_row {
            wrapping_index(current_row, rows.len(), delta)
        } else if delta < 0 {
            rows.iter()
                .rposition(|row| row.line < current_line)
                .unwrap_or(rows.len() - 1)
        } else {
            rows.iter()
                .position(|row| row.line > current_line)
                .unwrap_or(0)
        };
        let row = rows[next_row];
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            let _ = buffer.buffer.set_cursor(Position::new(row.line, 0));
            buffer.ensure_cursor_visible(context.body_height, context.body_width);
        }
        self.set_status(ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_LIST_SELECTED,
            &[label, &(row.index + 1).to_string(), &rows.len().to_string()],
        ));
        Some(row.index)
    }

    pub(crate) fn focus_first_numbered_aux_row(&mut self, label: &str) -> Option<usize> {
        self.focus_numbered_aux_row_at(NumberedAuxRowPosition::First, label)
    }

    pub(crate) fn focus_last_numbered_aux_row(&mut self, label: &str) -> Option<usize> {
        self.focus_numbered_aux_row_at(NumberedAuxRowPosition::Last, label)
    }

    fn focus_numbered_aux_row_at(
        &mut self,
        position: NumberedAuxRowPosition,
        label: &str,
    ) -> Option<usize> {
        let buffer_id = self.workspace.focused_window().ok()?.buffer_id;
        let rows = self
            .buffer_state(buffer_id)
            .map(|buffer| numbered_list_rows(&buffer.buffer))
            .unwrap_or_default();
        if rows.is_empty() {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_LIST_NO_ENTRIES,
                &[label],
            ));
            return None;
        }

        let row = match position {
            NumberedAuxRowPosition::First => rows[0],
            NumberedAuxRowPosition::Last => rows[rows.len() - 1],
        };
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            let _ = buffer.buffer.set_cursor(Position::new(row.line, 0));
            buffer.ensure_cursor_visible(context.body_height, context.body_width);
        }
        self.set_status(ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_LIST_SELECTED,
            &[label, &(row.index + 1).to_string(), &rows.len().to_string()],
        ));
        Some(row.index)
    }

    pub(crate) fn replace_in_focused_buffer(&mut self, spec: SearchSpec, replacement: &str) {
        if spec.is_empty() {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_REPLACE_NO_QUERY).to_string(),
            );
            return;
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_REPLACE_BUFFER_MISSING)
                    .to_string(),
            );
            return;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&spec.query, spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(spec.clone(), matches, None);
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_REPLACE_NO_MATCHES,
                &[&spec.display()],
            ));
            return;
        }

        let selection = current_match_selection(&buffer.buffer, &matches).unwrap_or_else(|| {
            let origin = buffer
                .buffer
                .selection_range()
                .map(|range| range.end)
                .unwrap_or_else(|| buffer.buffer.cursor_position());
            choose_search_match(&matches, origin, SearchDirection::Forward)
        });
        let target = matches[selection.index].range;
        let old_total = matches.len();

        match buffer.buffer.replace_range(target, replacement) {
            Ok(()) => {
                let suffix = if selection.wrapped { " (wrapped)" } else { "" };
                let new_matches = buffer
                    .buffer
                    .find_all_with_options(&spec.query, spec.options);
                let next_selection = if new_matches.is_empty() {
                    None
                } else {
                    Some(choose_search_match(
                        &new_matches,
                        buffer.buffer.cursor_position(),
                        SearchDirection::Forward,
                    ))
                };
                if let Some(next) = next_selection {
                    let selected = new_matches[next.index].range;
                    let _ = buffer.buffer.select(selected.start, selected.end);
                }
                let next_status = match next_selection {
                    Some(next) => format!("; next {}/{}", next.index + 1, new_matches.len()),
                    None => "; no matches left".to_string(),
                };
                buffer.set_search(
                    spec.clone(),
                    new_matches,
                    next_selection.map(|selection| selection.index),
                );
                self.set_status(format!(
                    "Replace: {}/{} {} -> {}{suffix}{next_status}",
                    selection.index + 1,
                    old_total,
                    spec.display(),
                    replacement_status_text(replacement)
                ));
            }
            Err(error) => self.set_status(format!("Replace failed: {}", buffer_error_text(error))),
        }
    }

    pub(crate) fn replace_all_in_focused_buffer(&mut self, spec: SearchSpec, replacement: &str) {
        if spec.is_empty() {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_REPLACE_ALL_NO_QUERY).to_string(),
            );
            return;
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_REPLACE_ALL_BUFFER_MISSING,
                )
                .to_string(),
            );
            return;
        };

        let matches = buffer
            .buffer
            .find_all_with_options(&spec.query, spec.options);
        if matches.is_empty() {
            buffer.buffer.clear_selection();
            buffer.set_search(spec.clone(), matches, None);
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_REPLACE_ALL_NO_MATCHES,
                &[&spec.display()],
            ));
            return;
        }

        match buffer
            .buffer
            .replace_all_with_options(&spec.query, replacement, spec.options)
        {
            Ok(count) => {
                let new_matches = buffer
                    .buffer
                    .find_all_with_options(&spec.query, spec.options);
                let remaining = new_matches.len();
                buffer.set_search(spec.clone(), new_matches, None);
                let suffix = if remaining == 0 {
                    String::new()
                } else {
                    format!("; {remaining} matches remain")
                };
                self.set_status(format!(
                    "Replace All: {count} {} -> {}{suffix}",
                    spec.display(),
                    replacement_status_text(replacement)
                ));
            }
            Err(error) => {
                self.set_status(format!("Replace All failed: {}", buffer_error_text(error)))
            }
        }
    }

    pub(crate) fn go_to_line(&mut self, input: &str) {
        let Ok(line_number) = input.parse::<usize>() else {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_GO_TO_LINE_INVALID,
                &[input],
            ));
            return;
        };
        if line_number == 0 {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_GO_TO_LINE_STARTS_AT_ONE,
                )
                .to_string(),
            );
            return;
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_GO_TO_LINE_BUFFER_MISSING,
                )
                .to_string(),
            );
            return;
        };

        let line_count = buffer.buffer.line_count();
        if line_number > line_count {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_GO_TO_LINE_PAST_END,
                &[&line_number.to_string(), &line_count.to_string()],
            ));
            return;
        }

        let target_line = line_number - 1;
        let current_column = buffer.buffer.cursor_position().column;
        let target_column = buffer
            .buffer
            .line(target_line)
            .map(|line| clamp_to_char_boundary(line, current_column))
            .unwrap_or(0);

        match buffer
            .buffer
            .set_cursor(Position::new(target_line, target_column))
        {
            Ok(()) => self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_GO_TO_LINE_MOVED,
                &[&line_number.to_string()],
            )),
            Err(error) => {
                self.set_status(format!("Go to line failed: {}", buffer_error_text(error)))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumberedAuxRowPosition {
    First,
    Last,
}
