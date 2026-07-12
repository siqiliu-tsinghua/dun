use crate::*;

impl AppState {
    pub(crate) fn handle_window_command(&mut self, command: &WindowCommand) {
        match command {
            WindowCommand::SplitHorizontal => {
                self.split_focused(Axis::Horizontal, "Split horizontally")
            }
            WindowCommand::SplitVertical => self.split_focused(Axis::Vertical, "Split vertically"),
            WindowCommand::FocusLeft => self.focus_window_direction(Direction::Left, "left"),
            WindowCommand::FocusRight => self.focus_window_direction(Direction::Right, "right"),
            WindowCommand::FocusUp => self.focus_window_direction(Direction::Up, "up"),
            WindowCommand::FocusDown => self.focus_window_direction(Direction::Down, "down"),
            WindowCommand::ResizeLeft => self.resize_window_direction(Direction::Left, "left"),
            WindowCommand::ResizeRight => self.resize_window_direction(Direction::Right, "right"),
            WindowCommand::ResizeUp => self.resize_window_direction(Direction::Up, "up"),
            WindowCommand::ResizeDown => self.resize_window_direction(Direction::Down, "down"),
            WindowCommand::Equalize => {
                self.workspace.equalize();
                self.set_status("Equalized splits");
            }
            WindowCommand::RotateSplit => match self.workspace.rotate_focused_split() {
                Ok(axis) => {
                    self.set_status(format!("Rotated focused split to {}", axis_name(axis)))
                }
                Err(error) => self.set_status(format!(
                    "Rotate split failed: {}",
                    workspace_error_text(error)
                )),
            },
            WindowCommand::Collapse => match self.workspace.collapse_focused() {
                Ok(()) => self.set_status("Collapsed pane"),
                Err(error) => {
                    self.set_status(format!("Collapse failed: {}", workspace_error_text(error)))
                }
            },
            WindowCommand::Expand => match self.workspace.expand_focused() {
                Ok(()) => self.set_status("Expanded pane"),
                Err(error) => {
                    self.set_status(format!("Expand failed: {}", workspace_error_text(error)))
                }
            },
            WindowCommand::ToggleCollapse => match self.workspace.toggle_focused_collapse() {
                Ok(true) => self.set_status("Collapsed pane"),
                Ok(false) => self.set_status("Expanded pane"),
                Err(error) => self.set_status(format!(
                    "Toggle collapse failed: {}",
                    workspace_error_text(error)
                )),
            },
            WindowCommand::Close => {
                if self.workspace.window_count() > 1
                    && self.confirm_focused_dirty(PendingAction::CloseWindow)
                {
                    return;
                }
                self.close_focused_window_unchecked();
            }
            WindowCommand::Only => self.set_status("Only window is not implemented yet"),
        }
    }

    fn focus_window_direction(&mut self, direction: Direction, label: &str) {
        match self
            .workspace
            .focus_direction(direction, self.workspace_area)
        {
            Ok(_) => self.set_status(format!("Focused {label}")),
            Err(error) => self.set_status(format!(
                "Focus {label} failed: {}",
                workspace_error_text(error)
            )),
        }
    }

    fn resize_window_direction(&mut self, direction: Direction, label: &str) {
        match self.workspace.resize_focused(direction) {
            Ok(_) => self.set_status(format!("Resized {label}")),
            Err(error) => self.set_status(format!(
                "Resize {label} failed: {}",
                workspace_error_text(error)
            )),
        }
    }

    fn split_focused(&mut self, axis: Axis, success_status: &'static str) {
        let window_id = match self.workspace.split_focused(axis) {
            Ok(window_id) => window_id,
            Err(error) => {
                self.set_status(format!("Split failed: {}", workspace_error_text(error)));
                return;
            }
        };
        let window = match self.workspace.window(window_id) {
            Ok(window) => window,
            Err(error) => {
                self.set_status(format!("Split failed: {}", workspace_error_text(error)));
                return;
            }
        };

        if self.buffer_state(window.buffer_id).is_none() {
            self.buffers.push(BufferState::new(
                window.buffer_id,
                TextBuffer::new_untitled(),
            ));
        }

        self.set_status(success_status);
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
        self.set_status(format!("Closed {name}"));
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
                self.set_status("Closed window");
            }
            Err(error) => {
                self.set_status(format!("Close failed: {}", workspace_error_text(error)));
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
