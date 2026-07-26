use crate::*;

impl AppState {
    pub(super) fn toggle_word_wrap(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_WRAP_BUFFER_MISSING).to_string(),
            );
            return;
        };

        buffer.word_wrap = !buffer.word_wrap;
        if buffer.word_wrap {
            buffer.first_column = 0;
            buffer.first_visual_row = 0;
            self.set_status(ui_text::tr(&self.shell.catalog, ui_text::STATUS_WRAP_ON).to_string());
        } else {
            buffer.first_visual_row = 0;
            self.set_status(ui_text::tr(&self.shell.catalog, ui_text::STATUS_WRAP_OFF).to_string());
        }
    }

    pub(super) fn move_focused_page(&mut self, direction: isize) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let body_height = context.body_height;
        let page_lines = body_height.saturating_sub(1).max(1);
        let display = self.shell.editor_text_display(false);
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let moved = if buffer.word_wrap {
            buffer.move_wrapped_page(direction, page_lines, context.body_width, display)
        } else if direction < 0 {
            buffer.move_page_up(page_lines)
        } else {
            buffer.move_page_down(page_lines)
        };
        buffer.ensure_cursor_visible(body_height, context.body_width, display);
        moved
    }

    pub(super) fn move_focused_document_edge(&mut self, end: bool) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let display = self.shell.editor_text_display(false);
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let target = if end {
            buffer_end_position(&buffer.buffer)
        } else {
            Position::zero()
        };
        let moved =
            buffer.buffer.cursor_position() != target || buffer.buffer.selection().is_some();
        let _ = buffer.buffer.set_cursor(target);
        buffer.ensure_cursor_visible(context.body_height, context.body_width, display);
        moved
    }

    pub(crate) fn scroll_focused_columns(&mut self, direction: isize) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let step = context.body_width.saturating_div(2).max(1) as isize;
        let display = self.shell.editor_text_display(false);
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let moved =
            buffer.scroll_view_columns(direction.saturating_mul(step), context.body_width, display);
        let first_column = buffer.first_column;
        let status = if moved {
            let key = if direction < 0 {
                ui_text::STATUS_SCROLL_LEFT
            } else {
                ui_text::STATUS_SCROLL_RIGHT
            };
            ui_text::tr_fmt(&self.shell.catalog, key, &[&(first_column + 1).to_string()])
        } else if direction < 0 {
            ui_text::tr(&self.shell.catalog, ui_text::STATUS_SCROLL_LEFT_EDGE).to_string()
        } else {
            ui_text::tr(&self.shell.catalog, ui_text::STATUS_SCROLL_RIGHT_EDGE).to_string()
        };
        self.set_status(status);
        moved
    }

    pub(crate) fn extend_focused_page(&mut self, direction: isize) -> bool {
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id: BufferId(0),
                body_height: 1,
                body_width: 1,
            });
        let body_height = context.body_height;
        let page_lines = body_height.saturating_sub(1).max(1);
        let display = self.shell.editor_text_display(false);
        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        let moved = if buffer.word_wrap {
            buffer.extend_wrapped_page(direction, page_lines, context.body_width, display)
        } else if direction < 0 {
            buffer.extend_page_up(page_lines)
        } else {
            buffer.extend_page_down(page_lines)
        };
        buffer.ensure_cursor_visible(body_height, context.body_width, display);
        moved
    }
}
