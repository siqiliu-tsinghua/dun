use dun_core::{EditorCommand, Position, Rect, Workspace};

use crate::{
    BufferView, MenuSelection, UiMouseHit, UiMouseTarget, UiOverlay, UiShell, UiWindow,
    WindowGeometry,
};

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
        let panel_inset = self.border_columns().saturating_add(1);
        let content_start = layout.rect.x.saturating_add(panel_inset);
        let content_end = layout
            .rect
            .x
            .saturating_add(layout.rect.width)
            .saturating_sub(panel_inset);
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
            let (start, end) =
                super::menu_item_column_range(&menu, index, self.profile.ambiguous_width)?;
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
        let border_columns = self.border_columns();
        // Narrow menus historically include their padding in the hit target;
        // Wide menus must also skip the second border cell.
        let hit_inset = if border_columns == 1 {
            border_columns
        } else {
            border_columns.saturating_add(1)
        };
        let content_start = dropdown.x.saturating_add(hit_inset);
        let content_end = dropdown
            .x
            .saturating_add(dropdown.width)
            .saturating_sub(hit_inset);
        if column < content_start
            || column >= content_end
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

    pub fn menu_entry_mnemonic(&self, menu_index: usize, entry_index: usize) -> Option<char> {
        let menu = self.menu_bar(None);
        let label = &menu.items.get(menu_index)?.entries.get(entry_index)?.label;
        entry_mnemonic(label)
    }

    pub fn menu_count(&self) -> usize {
        self.menu_bar(None).items.len()
    }

    pub fn menu_index_for_mnemonic(&self, ch: char) -> Option<usize> {
        self.menu_bar(None)
            .items
            .iter()
            .position(|item| mnemonic_matches(&item.label, ch))
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
                entry_mnemonic(&entry.label)
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
        let geometry = window.geometry;
        if window.collapsed || geometry.inner.width == 0 || geometry.inner.height == 0 {
            return UiMouseTarget::Chrome;
        }
        let on_right_border = local_x >= geometry.right_border_x;
        let on_body_row = local_y >= geometry.body.y
            && local_y < geometry.body.y.saturating_add(geometry.body.height);
        if on_right_border && on_body_row {
            if let Some(buffer) = buffers.iter().find(|buffer| buffer.id == window.buffer_id) {
                if let Some((first_line, first_visual_row)) =
                    self.scrollbar_target_line_for_buffer(buffer, geometry, local_y)
                {
                    return UiMouseTarget::Scrollbar {
                        first_line,
                        first_visual_row,
                    };
                }
            }
        }
        if !geometry.inner.contains(local_x, local_y) {
            return UiMouseTarget::Chrome;
        }

        let Some(buffer) = buffers.iter().find(|buffer| buffer.id == window.buffer_id) else {
            return UiMouseTarget::Chrome;
        };

        if geometry.gutter.contains(local_x, local_y) {
            return UiMouseTarget::Gutter;
        }
        if !geometry.body.contains(local_x, local_y) {
            return UiMouseTarget::Chrome;
        }

        if buffer.wrap {
            return self.hit_test_wrapped_body(buffer, geometry, local_x, local_y);
        }

        let line_index = buffer
            .first_line
            .saturating_add(local_y.saturating_sub(geometry.body.y) as usize);
        if line_index >= buffer.buffer.line_count() {
            return UiMouseTarget::Body(super::buffer_end_position(buffer.buffer));
        }

        let body_x = buffer
            .first_column
            .saturating_add(local_x.saturating_sub(geometry.body.x) as usize);
        let line = buffer.buffer.line(line_index).unwrap_or_default();
        let display = self.editor_text_display(buffer.visible_whitespace);
        UiMouseTarget::Body(Position::new(
            line_index,
            display.display_column_to_source_byte(line, body_x),
        ))
    }

    fn hit_test_wrapped_body(
        &self,
        buffer: &BufferView<'_>,
        geometry: WindowGeometry,
        local_x: u16,
        local_y: u16,
    ) -> UiMouseTarget {
        let body_width = usize::from(geometry.body.width).max(1);
        let body_x = local_x.saturating_sub(geometry.body.x) as usize;
        let target_row = local_y.saturating_sub(geometry.body.y) as usize;
        let display = self.editor_text_display(buffer.visible_whitespace);
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
                let line = buffer.buffer.line(line_index).unwrap_or_default();
                return UiMouseTarget::Body(Position::new(
                    line_index,
                    display.source_byte_for_wrapped_row_column(
                        line,
                        row_offset,
                        body_x.min(body_width.saturating_sub(1)),
                        body_width,
                    ),
                ));
            }
            visual_row = visual_row.saturating_add(visible_rows);
        }

        UiMouseTarget::Body(super::buffer_end_position(buffer.buffer))
    }
}

/// A translated menu-bar label carries its English mnemonic in trailing
/// parens ("文件 (F)"), which must win over the first letter; untranslated
/// labels ("File") keep the first-letter rule.
fn mnemonic_matches(label: &str, ch: char) -> bool {
    menu_label_mnemonic(label).is_some_and(|mnemonic| mnemonic.eq_ignore_ascii_case(&ch))
}

/// Read the mnemonic from a rendered top-level menu label without changing case.
///
/// The matching rule prefers a trailing parenthesized mnemonic and otherwise
/// uses the first character. It differs from English source-label derivation
/// because translated labels carry the invariant English mnemonic in a suffix.
pub fn menu_label_mnemonic(label: &str) -> Option<char> {
    entry_mnemonic(label).or_else(|| label.chars().next())
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
