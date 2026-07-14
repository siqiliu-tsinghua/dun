use crate::*;

impl AppState {
    pub(crate) fn handle_window_command(&mut self, command: &WindowCommand) {
        match command {
            WindowCommand::SplitHorizontal => {
                self.split_focused(Axis::Horizontal, ui_text::STATUS_WINDOW_SPLIT_HORIZONTAL)
            }
            WindowCommand::SplitVertical => {
                self.split_focused(Axis::Vertical, ui_text::STATUS_WINDOW_SPLIT_VERTICAL)
            }
            WindowCommand::FocusLeft => self.focus_window_direction(
                Direction::Left,
                ui_text::STATUS_WINDOW_FOCUSED_LEFT,
                ui_text::STATUS_WINDOW_FOCUS_LEFT_FAILED,
            ),
            WindowCommand::FocusRight => self.focus_window_direction(
                Direction::Right,
                ui_text::STATUS_WINDOW_FOCUSED_RIGHT,
                ui_text::STATUS_WINDOW_FOCUS_RIGHT_FAILED,
            ),
            WindowCommand::FocusUp => self.focus_window_direction(
                Direction::Up,
                ui_text::STATUS_WINDOW_FOCUSED_UP,
                ui_text::STATUS_WINDOW_FOCUS_UP_FAILED,
            ),
            WindowCommand::FocusDown => self.focus_window_direction(
                Direction::Down,
                ui_text::STATUS_WINDOW_FOCUSED_DOWN,
                ui_text::STATUS_WINDOW_FOCUS_DOWN_FAILED,
            ),
            WindowCommand::ResizeLeft => self.resize_window_direction(
                Direction::Left,
                ui_text::STATUS_WINDOW_RESIZED_LEFT,
                ui_text::STATUS_WINDOW_RESIZE_LEFT_FAILED,
            ),
            WindowCommand::ResizeRight => self.resize_window_direction(
                Direction::Right,
                ui_text::STATUS_WINDOW_RESIZED_RIGHT,
                ui_text::STATUS_WINDOW_RESIZE_RIGHT_FAILED,
            ),
            WindowCommand::ResizeUp => self.resize_window_direction(
                Direction::Up,
                ui_text::STATUS_WINDOW_RESIZED_UP,
                ui_text::STATUS_WINDOW_RESIZE_UP_FAILED,
            ),
            WindowCommand::ResizeDown => self.resize_window_direction(
                Direction::Down,
                ui_text::STATUS_WINDOW_RESIZED_DOWN,
                ui_text::STATUS_WINDOW_RESIZE_DOWN_FAILED,
            ),
            WindowCommand::Equalize => match self.workspace.equalize() {
                0 => self.set_status(
                    ui_text::tr(&self.shell.catalog, ui_text::STATUS_WINDOW_SPLITS_EVEN)
                        .to_string(),
                ),
                count => self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_WINDOW_EQUALIZED,
                    &[&count.to_string()],
                )),
            },
            WindowCommand::RotateSplit => match self.workspace.rotate_focused_split() {
                Ok(axis) => {
                    let status = ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_WINDOW_ROTATED,
                        &[axis_name(&self.shell.catalog, axis)],
                    );
                    self.set_status(status);
                }
                Err(error) => {
                    let status = ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_WINDOW_ROTATE_FAILED,
                        &[workspace_error_text(&self.shell.catalog, error)],
                    );
                    self.set_status(status);
                }
            },
            WindowCommand::Collapse => match self.workspace.collapse_focused() {
                Ok(()) => self.set_status(
                    ui_text::tr(&self.shell.catalog, ui_text::STATUS_WINDOW_COLLAPSED).to_string(),
                ),
                Err(error) => {
                    let status = ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_WINDOW_COLLAPSE_FAILED,
                        &[workspace_error_text(&self.shell.catalog, error)],
                    );
                    self.set_status(status);
                }
            },
            WindowCommand::Expand => match self.workspace.expand_focused() {
                Ok(true) => self.set_status(
                    ui_text::tr(&self.shell.catalog, ui_text::STATUS_WINDOW_EXPANDED).to_string(),
                ),
                Ok(false) => self.set_status(
                    ui_text::tr(&self.shell.catalog, ui_text::STATUS_WINDOW_ALREADY_EXPANDED)
                        .to_string(),
                ),
                Err(error) => {
                    let status = ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_WINDOW_EXPAND_FAILED,
                        &[workspace_error_text(&self.shell.catalog, error)],
                    );
                    self.set_status(status);
                }
            },
            WindowCommand::ToggleCollapse => match self.workspace.toggle_focused_collapse() {
                Ok(true) => self.set_status(
                    ui_text::tr(&self.shell.catalog, ui_text::STATUS_WINDOW_COLLAPSED).to_string(),
                ),
                Ok(false) => self.set_status(
                    ui_text::tr(&self.shell.catalog, ui_text::STATUS_WINDOW_EXPANDED).to_string(),
                ),
                Err(error) => {
                    let status = ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_WINDOW_TOGGLE_COLLAPSE_FAILED,
                        &[workspace_error_text(&self.shell.catalog, error)],
                    );
                    self.set_status(status);
                }
            },
            WindowCommand::Close => {
                if self.workspace.window_count() > 1
                    && self.confirm_focused_dirty(PendingAction::CloseWindow)
                {
                    return;
                }
                self.close_focused_window_unchecked();
            }
            WindowCommand::Only => self.only_focused_window(),
        }
    }

    fn focus_window_direction(
        &mut self,
        direction: Direction,
        success: ui_text::TextKey,
        failure: ui_text::TextKey,
    ) {
        match self
            .workspace
            .focus_direction(direction, self.workspace_area)
        {
            Ok(_) => self.set_status(ui_text::tr(&self.shell.catalog, success).to_string()),
            Err(error) => {
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    failure,
                    &[workspace_error_text(&self.shell.catalog, error)],
                );
                self.set_status(status);
            }
        }
    }

    fn resize_window_direction(
        &mut self,
        direction: Direction,
        success: ui_text::TextKey,
        failure: ui_text::TextKey,
    ) {
        match self.workspace.resize_focused(direction) {
            Ok(_) => self.set_status(ui_text::tr(&self.shell.catalog, success).to_string()),
            Err(error) => {
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    failure,
                    &[workspace_error_text(&self.shell.catalog, error)],
                );
                self.set_status(status);
            }
        }
    }

    fn split_focused(&mut self, axis: Axis, success_status: ui_text::TextKey) {
        let window_id = match self.workspace.split_focused(axis) {
            Ok(window_id) => window_id,
            Err(error) => {
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_WINDOW_SPLIT_FAILED,
                    &[workspace_error_text(&self.shell.catalog, error)],
                );
                self.set_status(status);
                return;
            }
        };
        let window = match self.workspace.window(window_id) {
            Ok(window) => window,
            Err(error) => {
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_WINDOW_SPLIT_FAILED,
                    &[workspace_error_text(&self.shell.catalog, error)],
                );
                self.set_status(status);
                return;
            }
        };

        if self.buffer_state(window.buffer_id).is_none() {
            self.buffers.push(BufferState::new(
                window.buffer_id,
                TextBuffer::new_untitled(),
            ));
        }

        self.set_status(ui_text::tr(&self.shell.catalog, success_status).to_string());
    }

    /// `file.close`: close the focused *file*, which is what a File menu's
    /// "Close" means everywhere else. This is not `window.close` -- that closes
    /// a pane and refuses on the last window, which left File > Close dead in
    /// the single-window case, the common one.
    pub(crate) fn close_focused_file(&mut self) {
        if self.confirm_focused_dirty(PendingAction::CloseFile) {
            return;
        }
        self.close_focused_file_unchecked();
    }

    pub(crate) fn close_focused_file_unchecked(&mut self) {
        let name = self
            .workspace
            .focused_window()
            .ok()
            .map(|window| window.title.clone())
            .unwrap_or_default();

        if self.workspace.window_count() > 1 {
            // Other panes remain, so drop this view; the buffer itself goes
            // once nothing references it.
            self.close_focused_window_unchecked();
        } else {
            // Closing the only file leaves an empty editor rather than
            // refusing, the way msedit and Notepad behave.
            self.reset_focused_to_untitled();
        }
        self.set_status(ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_WINDOW_CLOSED_ITEM,
            &[&name],
        ));
    }

    pub(crate) fn close_focused_window_unchecked(&mut self) {
        let focused = self.workspace.focused_window().ok().cloned();
        let closing_buffer_id = focused.as_ref().map(|window| window.buffer_id);
        let return_buffer_id = focused
            .as_ref()
            .and_then(|window| self.auxiliary_return_buffer_id(window));
        match self.workspace.close_focused() {
            Ok(_) => {
                if let Some(buffer_id) = closing_buffer_id {
                    self.drop_buffer_if_unreferenced(buffer_id);
                }
                if let Some(buffer_id) = return_buffer_id {
                    self.focus_window_for_buffer(buffer_id);
                }
                self.set_status(
                    ui_text::tr(&self.shell.catalog, ui_text::STATUS_WINDOW_CLOSED).to_string(),
                );
            }
            Err(error) => {
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_WINDOW_CLOSE_FAILED,
                    &[workspace_error_text(&self.shell.catalog, error)],
                );
                self.set_status(status);
            }
        }
    }

    pub(crate) fn only_focused_window(&mut self) {
        if self.workspace.window_count() <= 1 {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_WINDOW_ONLY_ONE).to_string(),
            );
            return;
        }

        let target = self.workspace.focused;
        if self.confirm_dirty_buffer_losing_its_last_window(target) {
            return;
        }
        self.only_focused_window_unchecked(target);
    }

    pub(crate) fn only_focused_window_unchecked(&mut self, target: WindowId) {
        self.workspace.focused = target;
        match self.workspace.only_focused() {
            Ok(removed) => {
                let closed = removed.len();
                for window in removed {
                    self.drop_buffer_if_unreferenced(window.buffer_id);
                }
                self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_WINDOW_CLOSED_OTHERS,
                    &[&closed.to_string()],
                ));
            }
            Err(error) => {
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_WINDOW_ONLY_FAILED,
                    &[workspace_error_text(&self.shell.catalog, error)],
                );
                self.set_status(status);
            }
        }
    }

    fn auxiliary_return_buffer_id(&self, window: &WindowState) -> Option<BufferId> {
        match window.kind {
            WindowKind::SearchResults => self.search_results_source,
            _ => None,
        }
    }

    pub(crate) fn focus_window_for_buffer(&mut self, buffer_id: BufferId) -> bool {
        let Some(window) = self
            .workspace
            .windows
            .iter()
            .find(|window| window.buffer_id == buffer_id)
        else {
            return false;
        };

        self.workspace.focused = window.id;
        true
    }

    pub(crate) fn drop_buffer_if_unreferenced(&mut self, id: BufferId) {
        if self
            .workspace
            .windows
            .iter()
            .any(|window| window.buffer_id == id)
        {
            return;
        }

        self.buffers.retain(|buffer| buffer.id != id);
    }
}
