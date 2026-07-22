use crate::*;

impl AppState {
    pub(crate) fn start_buffer_switcher(&mut self) {
        if self.buffers.len() <= 1 {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_SWITCHER_ONLY_ONE).to_string(),
            );
            return;
        }

        let selected = self
            .focused_buffer_id()
            .and_then(|id| self.buffers.iter().position(|buffer| buffer.id == id))
            .unwrap_or(0);
        self.clear_active_menu();
        self.pending_keys.clear();
        self.buffer_switcher = Some(BufferSwitcherState::new(selected, self.buffers.len()));
        self.set_status(
            ui_text::tr(&self.shell.catalog, ui_text::STATUS_SWITCHER_OPENED).to_string(),
        );
    }

    pub(crate) fn handle_buffer_switcher_key_event(&mut self, event: TerminalKeyEvent) -> bool {
        if self.buffer_switcher.is_none() {
            return false;
        }

        match event.code {
            TerminalKeyCode::Esc => self.cancel_buffer_switcher(),
            TerminalKeyCode::Enter => self.submit_buffer_switcher(),
            TerminalKeyCode::Up => self.move_buffer_switcher_selection(-1),
            TerminalKeyCode::Down => self.move_buffer_switcher_selection(1),
            TerminalKeyCode::Home => self.select_buffer_switcher_first(),
            TerminalKeyCode::End => self.select_buffer_switcher_last(),
            TerminalKeyCode::PageUp => self.page_buffer_switcher_selection(-1),
            TerminalKeyCode::PageDown => self.page_buffer_switcher_selection(1),
            _ => {}
        }

        true
    }

    fn cancel_buffer_switcher(&mut self) {
        self.buffer_switcher = None;
        self.set_status(
            ui_text::tr(&self.shell.catalog, ui_text::STATUS_SWITCHER_CANCELLED).to_string(),
        );
    }

    pub(crate) fn move_buffer_switcher_selection(&mut self, delta: isize) {
        if let Some(switcher) = &mut self.buffer_switcher {
            switcher.move_selection(delta, self.buffers.len());
        }
    }

    fn page_buffer_switcher_selection(&mut self, delta: isize) {
        if let Some(switcher) = &mut self.buffer_switcher {
            switcher.page_selection(delta, self.buffers.len());
        }
    }

    pub(crate) fn select_buffer_switcher_first(&mut self) {
        if let Some(switcher) = &mut self.buffer_switcher {
            switcher.select_first(self.buffers.len());
        }
    }

    pub(crate) fn select_buffer_switcher_last(&mut self) {
        if let Some(switcher) = &mut self.buffer_switcher {
            switcher.select_last(self.buffers.len());
        }
    }

    pub(crate) fn scroll_buffer_switcher(&mut self, delta: isize) {
        self.move_buffer_switcher_selection(delta);
    }

    fn submit_buffer_switcher(&mut self) {
        let Some(switcher) = self.buffer_switcher.take() else {
            return;
        };
        let Some(index) = switcher.selected_index(self.buffers.len()) else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_SWITCHER_NO_BUFFERS).to_string(),
            );
            return;
        };
        self.switch_to_buffer_index(index);
    }

    fn click_buffer_switcher_visible_index(&mut self, visible_index: usize) {
        let Some(mut switcher) = self.buffer_switcher.take() else {
            return;
        };
        let Some(index) = switcher.select_visible_index(visible_index, self.buffers.len()) else {
            self.buffer_switcher = Some(switcher);
            return;
        };
        self.switch_to_buffer_index(index);
    }

    fn switch_to_buffer_index(&mut self, index: usize) {
        let Some(buffer) = self.buffers.get(index) else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_SWITCHER_BUFFER_MISSING)
                    .to_string(),
            );
            return;
        };
        let buffer_id = buffer.id;
        let display_name = self.buffer_display_name(buffer_id);
        if self.focus_window_for_buffer(buffer_id) {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_SWITCHER_SWITCHED,
                &[&display_name],
            ));
        } else {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_SWITCHER_NO_WINDOW,
                &[&display_name],
            ));
        }
    }

    pub(crate) fn handle_buffer_switcher_mouse_down(
        &mut self,
        screen_x: u16,
        screen_y: u16,
    ) -> bool {
        let Some(overlay) = self.active_overlay() else {
            return false;
        };
        let Some(visible_index) =
            self.shell
                .hit_test_overlay_list(&overlay, self.overlay_area(), screen_x, screen_y)
        else {
            return false;
        };

        self.pending_keys.clear();
        self.click_buffer_switcher_visible_index(visible_index);
        true
    }

    pub(crate) fn buffer_switcher_overlay(&self, switcher: &BufferSwitcherState) -> UiOverlay {
        let catalog = &self.shell.catalog;
        let entries = self.buffer_switcher_entries();
        let (list, selected) = switcher.visible_entry_texts(&entries);
        let mut lines = vec![ui_text::tr_fmt(
            catalog,
            ui_text::SWITCHER_OPEN_BUFFERS,
            &[&entries.len().to_string()],
        )];
        if entries.len() > BUFFER_SWITCHER_VISIBLE_ENTRIES {
            if let Some((start, end, _)) = switcher.visible_entry_range(entries.len()) {
                lines.push(ui_text::tr_fmt(
                    catalog,
                    ui_text::SWITCHER_SHOWING,
                    &[
                        &(start + 1).to_string(),
                        &end.to_string(),
                        &entries.len().to_string(),
                    ],
                ));
            }
        }
        let mut overlay = UiOverlay::message(
            ui_text::tr(catalog, ui_text::SWITCHER_TITLE),
            lines,
            vec![
                ui_text::tr(catalog, ui_text::SWITCHER_HINT_MOVE).to_string(),
                ui_text::tr(catalog, ui_text::SWITCHER_HINT_ACTIONS).to_string(),
            ],
        )
        .with_list(list, selected, 48);
        if let Some((start, end, _)) = switcher.visible_entry_range(entries.len()) {
            overlay = overlay.with_list_overflow(start > 0, end < entries.len());
        }
        overlay
    }

    fn buffer_switcher_entries(&self) -> Vec<BufferSwitcherEntry> {
        let focused = self.focused_buffer_id();
        self.buffers
            .iter()
            .map(|buffer| {
                let active = if Some(buffer.id) == focused { ">" } else { " " };
                let dirty = if buffer.buffer.is_dirty() { "*" } else { " " };
                let disk = buffer_disk_state(buffer)
                    .map(|state| format!(" {state}"))
                    .unwrap_or_default();
                let name = self.buffer_display_name(buffer.id);
                let path = buffer
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "(no path)".to_string());
                BufferSwitcherEntry {
                    buffer_id: buffer.id,
                    text: format!("{active} {dirty} {name}{disk}  {path}"),
                }
            })
            .collect()
    }
}
