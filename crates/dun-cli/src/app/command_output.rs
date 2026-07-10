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

    fn command_output_buffer_id(&self) -> Option<BufferId> {
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
        match run_command_capture(
            input,
            COMMAND_OUTPUT_STREAM_SOFT_LIMIT_BYTES,
            std::time::Duration::from_millis(self.limits.run_command_timeout_ms),
        ) {
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
}
