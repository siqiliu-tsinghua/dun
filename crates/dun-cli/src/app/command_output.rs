use crate::*;

impl AppState {
    fn open_command_output_screen(&mut self, result: &CommandRunResult) {
        let text = command_output_text(result);

        if let Some(window_id) = self.command_output_window_id() {
            self.workspace.focused = window_id;
            self.refresh_command_output_buffer(&text);
            if let Ok(window) = self.workspace.window_mut(window_id) {
                window.title = "Command Output".to_string();
                window.kind = WindowKind::CommandOutput;
                window.buffer_kind = BufferKind::ReadOnly;
                window.collapsed = false;
            }
            return;
        }

        let Ok(window_id) = self.workspace.split_focused(Axis::Horizontal) else {
            self.set_status("Run command failed: focused window is missing");
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            self.set_status("Run command failed: output window is missing");
            return;
        };
        let buffer_id = window.buffer_id;
        let output = BufferState::new(buffer_id, command_output_buffer(&text));

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = output;
        } else {
            self.buffers.push(output);
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = "Command Output".to_string();
            window.kind = WindowKind::CommandOutput;
            window.buffer_kind = BufferKind::ReadOnly;
            window.collapsed = false;
        }
    }

    fn command_output_window_id(&self) -> Option<WindowId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::CommandOutput)
            .map(|window| window.id)
    }

    pub(crate) fn command_output_buffer_id(&self) -> Option<BufferId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::CommandOutput)
            .map(|window| window.buffer_id)
    }

    fn refresh_command_output_buffer(&mut self, text: &str) {
        let Some(buffer_id) = self.command_output_buffer_id() else {
            return;
        };
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = BufferState::new(buffer_id, command_output_buffer(text));
        }
    }

    pub(crate) fn run_external_command_to_buffer(&mut self, input: &str) {
        self.set_status(format!("Running command: {input}"));
        match run_command_capture(input, COMMAND_OUTPUT_STREAM_SOFT_LIMIT_BYTES) {
            Ok(result) => {
                let status = command_run_status(&result);
                self.open_command_output_screen(&result);
                self.set_status(status);
            }
            Err(error) => {
                self.set_status(format!("Run command failed: {error}"));
            }
        }
    }

    pub(crate) fn clear_command_output(&mut self) {
        let Some(buffer_id) = self.command_output_buffer_id() else {
            self.set_status("Command Output: no output window");
            return;
        };
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = BufferState::new(buffer_id, command_output_empty_buffer());
        }
        if let Some(window_id) = self.command_output_window_id() {
            self.workspace.focused = window_id;
        }
        self.set_status("Command Output cleared");
    }

    pub(crate) fn copy_command_output(&mut self) {
        let Some(text) = self.command_output_text_current() else {
            self.set_status("Command Output: no output window");
            return;
        };
        self.kill_ring = Some(text);
        self.set_status("Copied Command Output");
    }

    pub(crate) fn jump_command_output_summary(&mut self) {
        self.jump_command_output_line(command_output_summary_line, "summary");
    }

    pub(crate) fn jump_command_output_index(&mut self) {
        self.jump_command_output_line(command_output_index_line, "index");
    }

    pub(crate) fn jump_command_output_stdout(&mut self) {
        self.jump_command_output_line(command_output_stdout_line, "stdout");
    }

    pub(crate) fn jump_command_output_stdout_body(&mut self) {
        self.jump_command_output_line(command_output_stdout_body_line, "stdout body");
    }

    pub(crate) fn jump_command_output_stderr(&mut self) {
        self.jump_command_output_line(command_output_stderr_line, "stderr");
    }

    pub(crate) fn jump_command_output_stderr_body(&mut self) {
        self.jump_command_output_line(command_output_stderr_body_line, "stderr body");
    }

    pub(crate) fn jump_command_output_status(&mut self) {
        self.jump_command_output_line(command_output_status_line, "status");
    }

    pub(crate) fn jump_command_output_truncated(&mut self) {
        self.jump_command_output_line(command_output_truncated_line, "truncated");
    }

    pub(crate) fn jump_command_output_relative_section(&mut self, direction: SearchDirection) {
        let Some(window_id) = self.command_output_window_id() else {
            self.set_status("Command Output: no output window");
            return;
        };
        let Some(buffer_id) = self.command_output_buffer_id() else {
            self.set_status("Command Output: no output buffer");
            return;
        };
        let Some((line_index, label)) = self.buffer_state(buffer_id).and_then(|buffer| {
            command_output_relative_section_line(
                &buffer.buffer,
                buffer.buffer.cursor_position().line,
                direction,
            )
        }) else {
            self.set_status("Command Output: no sections");
            return;
        };

        self.workspace.focused = window_id;
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            let _ = buffer.buffer.set_cursor(Position::new(line_index, 0));
            buffer.ensure_cursor_visible(context.body_height, context.body_width);
        }
        self.set_status(format!("Command Output: {label}"));
    }

    pub(crate) fn open_command_output_section_view(&mut self, section: CommandOutputSection) {
        let Some(buffer_id) = self.command_output_buffer_id() else {
            self.set_status("Command Output: no output window");
            return;
        };
        let Some(text) = self
            .buffer_state(buffer_id)
            .and_then(|buffer| command_output_section_view_text(&buffer.buffer, section))
        else {
            self.set_status(format!(
                "Command Output: {} section not found",
                section.label()
            ));
            return;
        };

        self.open_read_only_aux_window(
            WindowKind::CommandOutputView,
            section.view_title(),
            command_output_buffer(&text),
        );
        self.set_status(format!("Command Output: only {}", section.label()));
    }

    fn jump_command_output_line(
        &mut self,
        line_finder: fn(&TextBuffer) -> Option<usize>,
        label: &'static str,
    ) {
        let Some(window_id) = self.command_output_window_id() else {
            self.set_status("Command Output: no output window");
            return;
        };
        let Some(buffer_id) = self.command_output_buffer_id() else {
            self.set_status("Command Output: no output buffer");
            return;
        };
        let Some(line_index) = self
            .buffer_state(buffer_id)
            .and_then(|buffer| line_finder(&buffer.buffer))
        else {
            self.set_status(format!("Command Output: {label} section not found"));
            return;
        };

        self.workspace.focused = window_id;
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            let _ = buffer.buffer.set_cursor(Position::new(line_index, 0));
            buffer.ensure_cursor_visible(context.body_height, context.body_width);
        }
        self.set_status(format!("Command Output: {label}"));
    }

    pub(crate) fn repeat_find_in_command_output(&mut self, direction: SearchDirection) {
        let Some((window_id, buffer_id)) = self.command_output_search_target() else {
            self.set_status("Command Output: no output window");
            return;
        };
        let spec = self
            .buffer_state(buffer_id)
            .and_then(|buffer| buffer.search.as_ref().map(|search| search.spec.clone()))
            .or_else(|| {
                self.last_find_query
                    .as_ref()
                    .map(|query| SearchSpec::parse(query))
                    .filter(|spec| !spec.is_empty())
            });
        let Some(spec) = spec else {
            self.set_status("Command Output find: no query");
            return;
        };

        self.workspace.focused = window_id;
        self.find_in_focused_buffer(spec, direction);
    }

    pub(crate) fn find_in_command_output(&mut self, spec: SearchSpec) {
        if spec.is_empty() {
            self.set_status("Command Output find: no query");
            return;
        }
        let Some((window_id, _)) = self.command_output_search_target() else {
            self.set_status("Command Output: no output window");
            return;
        };
        self.workspace.focused = window_id;
        self.last_find_query = Some(spec.input.clone());
        self.find_in_focused_buffer(spec, SearchDirection::Forward);
    }

    pub(crate) fn start_command_output_save_dialog(&mut self) {
        if self.command_output_buffer_id().is_none() {
            self.set_status("Command Output: no output window");
            return;
        }
        let input = self
            .recent_file_dialog_input
            .clone()
            .unwrap_or_else(|| "command-output.txt".to_string());
        self.start_file_dialog(FileDialogKind::CommandOutputSave, input);
    }

    pub(crate) fn save_command_output_path(&mut self, path: PathBuf) {
        let Some(text) = self.command_output_text_current() else {
            self.set_status("Command Output: no output window");
            return;
        };
        match atomic_write_text_file(&path, &text).map_err(|error| path_io_error(&path, error)) {
            Ok(report) => {
                self.set_status(status_with_atomic_temp_report(
                    format!("Saved Command Output {}", path.display()),
                    &report.temp_reconcile,
                ));
            }
            Err(error) => self.set_status(format!("Command Output save failed: {error}")),
        }
    }

    pub(crate) fn command_output_text_current(&self) -> Option<String> {
        if let Ok(window) = self.workspace.focused_window() {
            if window.kind == WindowKind::CommandOutputView {
                return Some(self.buffer_state(window.buffer_id)?.buffer.to_text());
            }
        }

        let buffer_id = self.command_output_buffer_id()?;
        Some(self.buffer_state(buffer_id)?.buffer.to_text())
    }

    fn command_output_search_target(&self) -> Option<(WindowId, BufferId)> {
        if let Ok(window) = self.workspace.focused_window() {
            if matches!(
                window.kind,
                WindowKind::CommandOutput | WindowKind::CommandOutputView
            ) {
                return Some((window.id, window.buffer_id));
            }
        }

        let window_id = self.command_output_window_id()?;
        let buffer_id = self.command_output_buffer_id()?;
        Some((window_id, buffer_id))
    }
}
