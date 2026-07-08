use crate::*;

impl AppState {
    pub(crate) fn focused_buffer_mut(&mut self) -> Option<&mut BufferState> {
        let buffer_id = self.focused_buffer_id()?;
        self.buffer_state_mut(buffer_id)
    }

    pub(crate) fn focused_buffer(&self) -> Option<&BufferState> {
        let buffer_id = self.focused_buffer_id()?;
        self.buffer_state(buffer_id)
    }

    pub(crate) fn focused_buffer_id(&self) -> Option<BufferId> {
        self.workspace
            .focused_window()
            .ok()
            .map(|window| window.buffer_id)
    }

    pub(crate) fn focused_buffer_is_dirty(&self) -> bool {
        self.focused_buffer_id()
            .and_then(|buffer_id| self.buffer_state(buffer_id))
            .is_some_and(|buffer| buffer.buffer.is_dirty())
    }

    pub(crate) fn buffer_state(&self, id: BufferId) -> Option<&BufferState> {
        self.buffers.iter().find(|buffer| buffer.id == id)
    }

    pub(crate) fn buffer_state_mut(&mut self, id: BufferId) -> Option<&mut BufferState> {
        self.buffers.iter_mut().find(|buffer| buffer.id == id)
    }

    pub(crate) fn set_status(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.status_message = Some(message.clone());
        self.record_status(message);
    }

    fn record_status(&mut self, message: String) {
        self.status_history.push(StatusEntry {
            level: StatusLevel::for_message(&message),
            message,
        });
        if self.status_history.len() > STATUS_HISTORY_LIMIT {
            let overflow = self.status_history.len() - STATUS_HISTORY_LIMIT;
            self.status_history.drain(0..overflow);
        }
        self.refresh_status_history_buffer();
    }

    fn refresh_status_history_buffer(&mut self) {
        let Some(buffer_id) = self.status_history_buffer_id() else {
            return;
        };
        let text = self.status_history_text();

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = BufferState::new(buffer_id, status_history_buffer(&text));
        }
    }

    pub(crate) fn focused_buffer_view_context(&self, area: Rect) -> Option<BufferViewContext> {
        let window = self.workspace.focused_window().ok()?;
        self.buffer_view_context(window.buffer_id, area)
    }

    pub(crate) fn buffer_view_context(
        &self,
        buffer_id: BufferId,
        area: Rect,
    ) -> Option<BufferViewContext> {
        let window = self
            .workspace
            .windows
            .iter()
            .find(|window| window.buffer_id == buffer_id)?;
        let layout = self
            .workspace
            .resolved_layout(area)
            .into_iter()
            .find(|layout| layout.id == window.id)?;
        let buffer = self.buffer_state(buffer_id)?;
        let body_height = layout.rect.height.saturating_sub(2) as usize;
        let body_width = editor_body_width(buffer, layout.rect);
        Some(BufferViewContext {
            buffer_id,
            body_height,
            body_width,
        })
    }
}
