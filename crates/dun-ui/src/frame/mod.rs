use dun_core::{Rect, WindowId, WindowState, Workspace};

use crate::{BufferView, MenuSelection, UiFrame, UiShell, UiWindow};

mod cursor;
mod gutter;
mod highlight;
mod menu;
mod scroll;
mod status;
mod text;

impl UiShell {
    pub fn frame_for_workspace(
        &self,
        workspace: &Workspace,
        area: Rect,
        buffers: &[BufferView<'_>],
    ) -> UiFrame {
        self.frame_for_workspace_with_menu(workspace, area, buffers, None)
    }

    pub fn frame_for_workspace_with_menu(
        &self,
        workspace: &Workspace,
        area: Rect,
        buffers: &[BufferView<'_>],
        active_menu: Option<usize>,
    ) -> UiFrame {
        self.frame_for_workspace_with_menu_selection(
            workspace,
            area,
            buffers,
            active_menu.map(MenuSelection::menu_only),
        )
    }

    pub fn frame_for_workspace_with_menu_selection(
        &self,
        workspace: &Workspace,
        area: Rect,
        buffers: &[BufferView<'_>],
        active_menu: Option<MenuSelection>,
    ) -> UiFrame {
        let mut windows = Vec::new();

        for layout in workspace.resolved_layout(area) {
            if let Ok(window) = workspace.window(layout.id) {
                windows.push(self.window_model(window, layout.rect, workspace.focused, buffers));
            }
        }

        UiFrame {
            menu: self.menu_bar(active_menu),
            status: self.status_bar(workspace, windows.len()),
            windows,
            overlay: None,
        }
    }
    fn window_model(
        &self,
        window: &WindowState,
        rect: Rect,
        focused: WindowId,
        buffers: &[BufferView<'_>],
    ) -> UiWindow {
        let buffer = buffers.iter().find(|buffer| buffer.id == window.buffer_id);
        let gutter_width = match (window.collapsed, buffer) {
            (false, Some(buffer)) => self.gutter_width_for_buffer(buffer, rect),
            _ => 0,
        };
        let body = match (window.collapsed, buffer) {
            (true, _) => Vec::new(),
            (false, Some(buffer)) => self.sanitize_buffer_body(buffer, rect, gutter_width),
            (false, None) => vec![self.display_sanitizer.sanitize_line("[missing buffer]")],
        };
        let cursor = if window.id == focused && !window.collapsed {
            buffer.and_then(|buffer| self.cursor_for_buffer(buffer, rect, gutter_width))
        } else {
            None
        };
        let selection = if !window.collapsed {
            buffer
                .map(|buffer| self.selection_for_buffer(buffer, rect, gutter_width))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let search_matches = if !window.collapsed {
            buffer
                .map(|buffer| self.search_matches_for_buffer(buffer, rect, gutter_width))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let highlights = if !window.collapsed {
            buffer
                .map(|buffer| self.plugin_highlights_for_buffer(buffer, rect, gutter_width))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let horizontal_edges = if !window.collapsed {
            buffer
                .map(|buffer| self.horizontal_edges_for_buffer(buffer, rect, gutter_width))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let scrollbar = if !window.collapsed {
            buffer.and_then(|buffer| self.scrollbar_for_buffer(buffer, rect))
        } else {
            None
        };
        let gutter = if !window.collapsed {
            buffer
                .map(|buffer| self.gutter_for_buffer(buffer, rect, gutter_width))
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        UiWindow {
            id: window.id,
            buffer_id: window.buffer_id,
            title: window.title.clone(),
            rect,
            focused: window.id == focused,
            collapsed: window.collapsed,
            dirty: buffer
                .map(|buffer| buffer.buffer.is_dirty())
                .unwrap_or(false),
            read_only: buffer
                .map(|buffer| buffer.buffer.is_read_only())
                .unwrap_or(matches!(window.buffer_kind, dun_core::BufferKind::ReadOnly)),
            border: self.glyphs.border,
            gutter_width,
            gutter,
            cursor,
            selection,
            search_matches,
            highlights,
            horizontal_edges,
            scrollbar,
            body,
        }
    }
}
