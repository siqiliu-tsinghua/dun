use dun_core::{EditorCommand, Position, Rect, Workspace};

use crate::{BufferView, MenuSelection, UiMouseHit, UiMouseTarget, UiOverlay, UiShell, UiWindow};

impl UiShell {
    pub fn hit_test_workspace(
        &self,
        workspace: &Workspace,
        area: Rect,
        buffers: &[BufferView<'_>],
        x: u16,
        y: u16,
    ) -> Option<UiMouseHit> {
        let ui_frame = self.frame_for_workspace(workspace, area, buffers);
        let window = ui_frame
            .windows
            .iter()
            .find(|window| window.rect.contains(x, y))?;
        let local_x = x.saturating_sub(window.rect.x);
        let local_y = y.saturating_sub(window.rect.y);
        let target = self.hit_target_for_window(window, buffers, local_x, local_y);

        Some(UiMouseHit {
            window_id: window.id,
            buffer_id: window.buffer_id,
            target,
        })
    }

    pub fn hit_test_overlay_list(
        &self,
        overlay: &UiOverlay,
        area: Rect,
        x: u16,
        y: u16,
    ) -> Option<usize> {
        let layout = super::overlay_layout(self, overlay, area)?;
        let content_start = layout.rect.x.saturating_add(2);
        let content_end = layout
            .rect
            .x
            .saturating_add(layout.rect.width)
            .saturating_sub(2);
        if x < content_start || x >= content_end || y < layout.list_start_row {
            return None;
        }

        let index = y.saturating_sub(layout.list_start_row) as usize;
        if index < layout.list_rows {
            Some(index)
        } else {
            None
        }
    }

    pub fn menu_index_at_column(&self, column: u16) -> Option<usize> {
        let menu = self.menu_bar(None);
        for index in 0..menu.items.len() {
            let (start, end) = super::menu_item_column_range(&menu, index)?;
            if column >= start && column < end {
                return Some(index);
            }
        }

        None
    }

    pub fn menu_entry_command_at(
        &self,
        active_menu: usize,
        column: u16,
        row: u16,
    ) -> Option<EditorCommand> {
        self.menu_entry_command_at_in_area(
            MenuSelection::menu_only(active_menu),
            column,
            row,
            Rect::new(0, 0, u16::MAX, u16::MAX),
        )
    }

    pub fn menu_entry_command_at_in_area(
        &self,
        active: MenuSelection,
        column: u16,
        row: u16,
        area: Rect,
    ) -> Option<EditorCommand> {
        let menu = self.menu_bar(None);
        let item = menu.items.get(active.menu_index)?;
        let dropdown = super::dropdown_rect_for_menu(self, &menu, active.menu_index)?;
        let dropdown = super::clamp_menu_rect(dropdown, area)?;
        if column <= dropdown.x
            || column >= dropdown.x.saturating_add(dropdown.width).saturating_sub(1)
            || row <= dropdown.y
            || row >= dropdown.y.saturating_add(dropdown.height).saturating_sub(1)
        {
            return None;
        }

        let max_rows = dropdown.height.saturating_sub(2) as usize;
        let (start, end) =
            super::menu_visible_entry_range(item.entries.len(), active.entry_index, max_rows)?;
        let entry_index = start.saturating_add(row.saturating_sub(dropdown.y + 1) as usize);
        if entry_index >= end {
            return None;
        }
        item.entries
            .get(entry_index)
            .map(|entry| entry.command.clone())
    }

    pub fn menu_entry_command(
        &self,
        menu_index: usize,
        entry_index: usize,
    ) -> Option<EditorCommand> {
        self.menu_bar(None)
            .items
            .get(menu_index)?
            .entries
            .get(entry_index)
            .map(|entry| entry.command.clone())
    }

    pub fn menu_entry_count(&self, menu_index: usize) -> Option<usize> {
        Some(self.menu_bar(None).items.get(menu_index)?.entries.len())
    }

    pub fn menu_count(&self) -> usize {
        self.menu_bar(None).items.len()
    }

    pub fn menu_index_for_mnemonic(&self, ch: char) -> Option<usize> {
        self.menu_bar(None)
            .items
            .iter()
            .position(|item| mnemonic_matches(item.label, ch))
    }

    /// The entry an open dropdown should run for a bare letter key. Entries
    /// advertise the letter in trailing parens ("Open... (O)"), so failing to
    /// dispatch it leaves the label promising a key that does nothing.
    pub fn menu_entry_index_for_mnemonic(&self, menu_index: usize, ch: char) -> Option<usize> {
        self.menu_bar(None)
            .items
            .get(menu_index)?
            .entries
            .iter()
            .position(|entry| {
                entry_mnemonic(entry.label)
                    .is_some_and(|mnemonic| mnemonic.eq_ignore_ascii_case(&ch) || mnemonic == ch)
            })
    }

    fn hit_target_for_window(
        &self,
        window: &UiWindow,
        buffers: &[BufferView<'_>],
        local_x: u16,
        local_y: u16,
    ) -> UiMouseTarget {
        if window.collapsed || window.rect.width <= 2 || window.rect.height <= 2 {
            return UiMouseTarget::Chrome;
        }
        if local_x == window.rect.width.saturating_sub(1)
            && local_y > 0
            && local_y < window.rect.height.saturating_sub(1)
        {
            if let Some(buffer) = buffers.iter().find(|buffer| buffer.id == window.buffer_id) {
                if let Some((first_line, first_visual_row)) =
                    self.scrollbar_target_line_for_buffer(buffer, window.rect, local_y)
                {
                    return UiMouseTarget::Scrollbar {
                        first_line,
                        first_visual_row,
                    };
                }
            }
        }
        if local_x == 0
            || local_y == 0
            || local_x >= window.rect.width.saturating_sub(1)
            || local_y >= window.rect.height.saturating_sub(1)
        {
            return UiMouseTarget::Chrome;
        }

        let Some(buffer) = buffers.iter().find(|buffer| buffer.id == window.buffer_id) else {
            return UiMouseTarget::Chrome;
        };

        let inner_width = window.rect.width.saturating_sub(2);
        let gutter_width = window.gutter_width.min(inner_width);
        let inner_x = local_x.saturating_sub(1);
        if inner_x < gutter_width {
            return UiMouseTarget::Gutter;
        }

        if buffer.wrap {
            return self.hit_test_wrapped_body(buffer, window.rect, inner_x, local_y, gutter_width);
        }

        let line_index = buffer
            .first_line
            .saturating_add(local_y.saturating_sub(1) as usize);
        if line_index >= buffer.buffer.line_count() {
            return UiMouseTarget::Body(super::buffer_end_position(buffer.buffer));
        }

        let body_x = buffer
            .first_column
            .saturating_add(inner_x.saturating_sub(gutter_width) as usize);
        let line = buffer.buffer.line(line_index).unwrap_or_default();
        UiMouseTarget::Body(Position::new(
            line_index,
            self.byte_column_for_display_column(line, body_x),
        ))
    }

    fn hit_test_wrapped_body(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        inner_x: u16,
        local_y: u16,
        gutter_width: u16,
    ) -> UiMouseTarget {
        let inner_width = rect.width.saturating_sub(2) as usize;
        let body_width = inner_width.saturating_sub(gutter_width as usize).max(1);
        let body_x = inner_x.saturating_sub(gutter_width) as usize;
        let target_row = local_y.saturating_sub(1) as usize;
        let mut visual_row = 0usize;

        for line_index in buffer.first_line..buffer.buffer.line_count() {
            let line_rows = self.wrapped_visual_line_count(buffer, line_index, body_width);
            let start_offset = if line_index == buffer.first_line {
                buffer.first_visual_row.min(line_rows.saturating_sub(1))
            } else {
                0
            };
            let visible_rows = line_rows.saturating_sub(start_offset);
            if target_row < visual_row.saturating_add(visible_rows) {
                let row_offset = start_offset.saturating_add(target_row.saturating_sub(visual_row));
                let display_column = row_offset
                    .saturating_mul(body_width)
                    .saturating_add(body_x.min(body_width.saturating_sub(1)));
                let line = buffer.buffer.line(line_index).unwrap_or_default();
                return UiMouseTarget::Body(Position::new(
                    line_index,
                    self.byte_column_for_display_column(line, display_column),
                ));
            }
            visual_row = visual_row.saturating_add(visible_rows);
        }

        UiMouseTarget::Body(super::buffer_end_position(buffer.buffer))
    }
}

fn mnemonic_matches(label: &str, ch: char) -> bool {
    label
        .chars()
        .next()
        .is_some_and(|mnemonic| mnemonic.eq_ignore_ascii_case(&ch))
}

/// Menu-bar items take their mnemonic from the first letter ("File" -> F), but
/// dropdown entries carry it in trailing parens: "Open... (O)", "Scroll Left
/// ([)". Anything else in the parens is not a mnemonic.
pub(crate) fn entry_mnemonic(label: &str) -> Option<char> {
    let open = label.rfind('(')?;
    let rest = &label[open + 1..];
    let close = rest.find(')')?;
    let mut chars = rest[..close].chars();
    let mnemonic = chars.next()?;
    chars.next().is_none().then_some(mnemonic)
}
