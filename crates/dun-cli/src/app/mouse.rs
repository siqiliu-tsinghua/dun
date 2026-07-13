use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MouseDragState {
    Selection {
        buffer_id: BufferId,
        anchor: Position,
    },
    Split {
        handle: SplitDragHandle,
    },
    Scrollbar {
        buffer_id: BufferId,
    },
}

impl AppState {
    pub(crate) fn handle_mouse_down(&mut self, screen_x: u16, screen_y: u16) -> bool {
        self.mouse_drag = None;
        if screen_y == 0 {
            if let Some(menu_index) = self.shell.menu_index_at_column(screen_x) {
                self.pending_keys.clear();
                self.open_mouse_menu(menu_index);
                return true;
            }
            self.clear_active_menu();
            return false;
        }

        if let Some(selection) = self.menu_selection() {
            if let Some(command) = self.shell.menu_entry_command_at_in_area(
                selection,
                screen_x,
                screen_y,
                self.overlay_area(),
            ) {
                self.clear_active_menu();
                self.pending_keys.clear();
                self.handle_command(&command);
                return true;
            }
            self.clear_active_menu();
        }

        let Some((x, y)) = self.workspace_point_from_screen(screen_x, screen_y) else {
            return false;
        };
        if let Some(handle) = self.workspace.split_at(self.workspace_area, x, y) {
            let _ = self.workspace.focus_at(self.workspace_area, x, y);
            self.pending_keys.clear();
            self.mouse_drag = Some(MouseDragState::Split { handle });
            return true;
        }

        let hit = {
            let buffer_views = self.buffer_views();
            self.shell
                .hit_test_workspace(&self.workspace, self.workspace_area, &buffer_views, x, y)
        };
        let Some(hit) = hit else {
            return false;
        };

        if self.workspace.focus_at(self.workspace_area, x, y).is_none() {
            return false;
        }

        self.pending_keys.clear();
        match hit.target {
            UiMouseTarget::Body(position) => {
                if let Some(buffer) = self.buffer_state_mut(hit.buffer_id) {
                    let _ = buffer.buffer.set_cursor(position);
                }
                self.mouse_drag = Some(MouseDragState::Selection {
                    buffer_id: hit.buffer_id,
                    anchor: position,
                });
                self.sync_view_for_area(self.workspace_area);
            }
            UiMouseTarget::Scrollbar {
                first_line,
                first_visual_row,
            } => {
                self.scroll_buffer_to_line(hit.buffer_id, first_line, first_visual_row);
                self.mouse_drag = Some(MouseDragState::Scrollbar {
                    buffer_id: hit.buffer_id,
                });
            }
            UiMouseTarget::Chrome | UiMouseTarget::Gutter => {}
        }

        true
    }

    pub(crate) fn handle_mouse_drag(&mut self, screen_x: u16, screen_y: u16) -> bool {
        if self.active_menu.is_some() {
            return false;
        }
        let Some(drag) = self.mouse_drag.clone() else {
            return false;
        };

        match drag {
            MouseDragState::Selection { buffer_id, anchor } => {
                self.update_mouse_selection(buffer_id, anchor, screen_x, screen_y)
            }
            MouseDragState::Split { handle } => {
                let Some((x, y)) = self.clamped_workspace_point_from_screen(screen_x, screen_y)
                else {
                    return false;
                };
                if self
                    .workspace
                    .resize_split_to(&handle, self.workspace_area, x, y)
                    .is_ok()
                {
                    self.sync_view_for_area(self.workspace_area);
                    true
                } else {
                    false
                }
            }
            MouseDragState::Scrollbar { buffer_id } => {
                self.update_scrollbar_drag(buffer_id, screen_x, screen_y)
            }
        }
    }

    pub(crate) fn handle_mouse_up(&mut self) {
        self.mouse_drag = None;
    }

    pub(crate) fn handle_file_dialog_mouse_down(&mut self, screen_x: u16, screen_y: u16) -> bool {
        self.mouse_drag = None;
        let Some(dialog) = &self.file_dialog else {
            return false;
        };
        let overlay = dialog.overlay(&self.file_dialog_keys, &self.shell.catalog);
        let Some(visible_index) =
            self.shell
                .hit_test_overlay_list(&overlay, self.overlay_area(), screen_x, screen_y)
        else {
            return false;
        };

        self.pending_keys.clear();
        self.click_file_dialog_visible_index(visible_index);
        true
    }

    pub(crate) fn overlay_area(&self) -> Rect {
        Rect::new(
            0,
            0,
            self.workspace_area.width,
            self.workspace_area.height.saturating_add(2),
        )
    }

    fn update_mouse_selection(
        &mut self,
        buffer_id: BufferId,
        anchor: Position,
        screen_x: u16,
        screen_y: u16,
    ) -> bool {
        let Some((x, y)) = self.clamped_workspace_point_from_screen(screen_x, screen_y) else {
            return false;
        };
        let hit = {
            let buffer_views = self.buffer_views();
            self.shell
                .hit_test_workspace(&self.workspace, self.workspace_area, &buffer_views, x, y)
        };
        let Some(hit) = hit else {
            return false;
        };
        if hit.buffer_id != buffer_id {
            return false;
        }
        let position = match hit.target {
            UiMouseTarget::Body(position) => position,
            UiMouseTarget::Chrome | UiMouseTarget::Gutter | UiMouseTarget::Scrollbar { .. } => {
                let Some(position) = self.drag_scroll_selection_position(buffer_id, x, y) else {
                    return false;
                };
                position
            }
        };

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            let _ = buffer.buffer.select(anchor, position);
            self.sync_view_for_area(self.workspace_area);
            true
        } else {
            false
        }
    }

    fn drag_scroll_selection_position(
        &mut self,
        buffer_id: BufferId,
        workspace_x: u16,
        workspace_y: u16,
    ) -> Option<Position> {
        let layout = self
            .workspace
            .resolved_layout(self.workspace_area)
            .into_iter()
            .find(|layout| {
                self.workspace
                    .window(layout.id)
                    .ok()
                    .is_some_and(|window| window.buffer_id == buffer_id)
            })?;
        if layout.rect.width <= 2 || layout.rect.height <= 2 {
            return None;
        }

        let body_height = layout.rect.height.saturating_sub(2) as usize;
        let body_width = layout.rect.width.saturating_sub(2) as usize;
        let top = layout.rect.y.saturating_add(1);
        let bottom = layout
            .rect
            .y
            .saturating_add(layout.rect.height)
            .saturating_sub(2);
        let target_line = {
            let buffer = self.buffer_state_mut(buffer_id)?;
            if workspace_y <= top {
                buffer.scroll_view_lines(-1, body_height, body_width);
                buffer.first_line
            } else if workspace_y >= bottom {
                buffer.scroll_view_lines(1, body_height, body_width);
                buffer
                    .first_line
                    .saturating_add(body_height.saturating_sub(1))
                    .min(buffer.buffer.line_count().saturating_sub(1))
            } else {
                buffer
                    .first_line
                    .saturating_add(workspace_y.saturating_sub(top) as usize)
            }
        };

        let x = workspace_x
            .clamp(
                layout.rect.x.saturating_add(1),
                layout
                    .rect
                    .x
                    .saturating_add(layout.rect.width)
                    .saturating_sub(2),
            )
            .saturating_sub(layout.rect.x.saturating_add(1)) as usize;
        let buffer = self.buffer_state(buffer_id)?;
        let line = buffer.buffer.line(target_line)?;
        let display_column = buffer
            .first_column
            .saturating_add(x.min(body_width.saturating_sub(1)));
        let column = clamp_to_display_column(line, display_column);
        Some(Position::new(target_line, column))
    }

    pub(crate) fn handle_mouse_scroll(
        &mut self,
        screen_x: u16,
        screen_y: u16,
        delta: isize,
    ) -> bool {
        if self.active_menu.is_some() {
            self.clear_active_menu();
        }
        let Some((x, y)) = self.workspace_point_from_screen(screen_x, screen_y) else {
            return false;
        };
        let Some(window_id) = self.workspace.focus_at(self.workspace_area, x, y) else {
            return false;
        };
        let Some(buffer_id) = self
            .workspace
            .windows
            .iter()
            .find(|window| window.id == window_id)
            .map(|window| window.buffer_id)
        else {
            return false;
        };
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });

        self.pending_keys.clear();
        self.buffer_state_mut(buffer_id).is_some_and(|buffer| {
            let moved = buffer.scroll_view_lines(delta, context.body_height, context.body_width);
            buffer.ensure_cursor_column_visible(context.body_width);
            moved
        })
    }

    fn update_scrollbar_drag(&mut self, buffer_id: BufferId, screen_x: u16, screen_y: u16) -> bool {
        let Some((x, y)) = self.clamped_workspace_point_from_screen(screen_x, screen_y) else {
            return false;
        };
        let hit = {
            let buffer_views = self.buffer_views();
            self.shell
                .hit_test_workspace(&self.workspace, self.workspace_area, &buffer_views, x, y)
        };
        let Some(hit) = hit else {
            return false;
        };
        if hit.buffer_id != buffer_id {
            return false;
        }
        let UiMouseTarget::Scrollbar {
            first_line,
            first_visual_row,
        } = hit.target
        else {
            return false;
        };

        self.scroll_buffer_to_line(buffer_id, first_line, first_visual_row)
    }

    fn scroll_buffer_to_line(
        &mut self,
        buffer_id: BufferId,
        first_line: usize,
        first_visual_row: usize,
    ) -> bool {
        let context = self
            .buffer_view_context(buffer_id, self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });
        self.pending_keys.clear();
        self.buffer_state_mut(buffer_id).is_some_and(|buffer| {
            let moved = buffer.scroll_view_to_line(
                first_line,
                first_visual_row,
                context.body_height,
                context.body_width,
            );
            buffer.ensure_cursor_column_visible(context.body_width);
            moved
        })
    }

    fn workspace_point_from_screen(&self, column: u16, row: u16) -> Option<(u16, u16)> {
        if self.workspace_area.width == 0 || self.workspace_area.height == 0 || row == 0 {
            return None;
        }

        let y = row - 1;
        if column >= self.workspace_area.width || y >= self.workspace_area.height {
            return None;
        }

        Some((column, y))
    }

    fn clamped_workspace_point_from_screen(&self, column: u16, row: u16) -> Option<(u16, u16)> {
        if self.workspace_area.width == 0 || self.workspace_area.height == 0 {
            return None;
        }

        Some((
            column.min(self.workspace_area.width.saturating_sub(1)),
            row.saturating_sub(1)
                .min(self.workspace_area.height.saturating_sub(1)),
        ))
    }
}
