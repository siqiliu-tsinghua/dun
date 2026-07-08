#![forbid(unsafe_code)]

use dun_config::{Config, KeySequence, KeyStroke, Keymap};
use dun_core::{
    DisplaySanitizer, EditorCommand, Rect, SanitizedLine, TextRange, WindowId, WindowState,
    Workspace,
};
use dun_term::{EncodingProfile, GlyphSet, TerminalProfile, Theme};
use ratatui::prelude::Frame;
use unicode_width::UnicodeWidthStr;

mod hit;
mod model;
mod render;
mod text;

pub use model::{
    BufferView, MenuBar, MenuEntry, MenuItem, MenuSelection, StatusBar, UiCursor, UiFrame,
    UiGutterLine, UiHorizontalEdgeLine, UiMouseHit, UiMouseTarget, UiOverlay, UiScrollbar,
    UiSearchMatchLine, UiSelectionLine, UiWindow,
};
#[cfg(test)]
pub(crate) use render::chrome::{vertical_overflow_down, vertical_overflow_up};
pub(crate) use render::menu::{
    clamp_menu_rect, dropdown_rect_for_menu, menu_item_column_range, menu_visible_entry_range,
};
pub(crate) use render::overlay::overlay_layout;
pub use render::render_ui_frame;
#[cfg(test)]
pub(crate) use render::status::sanitized_status_text_for_width;
#[cfg(test)]
pub(crate) use render::window::window_title_for_width;
pub(crate) use text::{
    buffer_end_position, decimal_digits, display_width, fit_text_to_width, status_text_for_width,
    visible_whitespace_prefix_text, visible_whitespace_text, wrap_line_segments,
};

const MIN_BODY_COLUMNS_WITH_GUTTER: u16 = 4;

#[derive(Clone, Debug)]
pub struct UiShell {
    pub profile: TerminalProfile,
    pub glyphs: GlyphSet,
    pub theme: Theme,
    pub keymap: Keymap,
    pub display_sanitizer: DisplaySanitizer,
}

impl UiShell {
    pub fn from_config(config: &Config, detected_profile: TerminalProfile) -> Self {
        let profile = config.terminal_profile(detected_profile);
        let glyphs = GlyphSet::for_profile(profile);
        let theme = config.resolved_theme(detected_profile);
        let display_sanitizer = DisplaySanitizer {
            ascii_only: matches!(profile.encoding, EncodingProfile::Ascii),
            max_bytes: config.limits.line_display_soft_limit_bytes,
        };

        Self {
            profile,
            glyphs,
            theme,
            keymap: config.keybindings.clone(),
            display_sanitizer,
        }
    }

    pub fn command_for_sequence(&self, sequence: &KeySequence) -> Option<&EditorCommand> {
        self.keymap.command_for_sequence(sequence)
    }

    pub fn command_for_stroke(&self, stroke: KeyStroke) -> Option<&EditorCommand> {
        self.keymap.command_for_stroke(stroke)
    }

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

    pub fn describe_workspace(&self, workspace: &Workspace) -> String {
        format!(
            "theme={} windows={} border={}{}{}{}",
            self.theme.name,
            workspace.window_count(),
            self.glyphs.border.top_left,
            self.glyphs.border.horizontal,
            self.glyphs.border.horizontal,
            self.glyphs.border.top_right,
        )
    }

    pub fn render(&self, frame: &mut Frame<'_>, ui_frame: &UiFrame) {
        render_ui_frame(frame, self, ui_frame);
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
            horizontal_edges,
            scrollbar,
            body,
        }
    }

    fn sanitize_buffer_body(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Vec<SanitizedLine> {
        let body_height = rect.height.saturating_sub(2) as usize;
        if body_height == 0 {
            return Vec::new();
        }
        if buffer.wrap {
            return self.sanitize_wrapped_buffer_body(buffer, rect, gutter_width);
        }

        let mut lines = Vec::new();
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if lines.len() >= body_height {
                break;
            }

            let line = buffer.buffer.line(line_index).unwrap_or_default();
            let start = self.byte_column_for_display_column(line, buffer.first_column);
            lines.push(self.sanitize_visible_line(&line[start..], buffer.show_whitespace));
        }

        lines
    }

    fn sanitize_wrapped_buffer_body(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Vec<SanitizedLine> {
        let body_height = rect.height.saturating_sub(2) as usize;
        let inner_width = rect.width.saturating_sub(2) as usize;
        let body_width = inner_width.saturating_sub(gutter_width as usize).max(1);
        let mut lines = Vec::new();

        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if lines.len() >= body_height {
                break;
            }

            let line = buffer.buffer.line(line_index).unwrap_or_default();
            let visible = visible_whitespace_text(
                line,
                buffer.show_whitespace,
                self.display_sanitizer.ascii_only,
            );
            let start_offset = if line_index == buffer.first_line {
                buffer.first_visual_row.min(
                    self.wrapped_visual_line_count(buffer, line_index, body_width)
                        .saturating_sub(1),
                )
            } else {
                0
            };
            for segment in wrap_line_segments(&visible, body_width)
                .iter()
                .skip(start_offset)
            {
                if lines.len() >= body_height {
                    break;
                }
                lines.push(self.display_sanitizer.sanitize_line(segment));
            }
        }

        lines
    }

    fn sanitize_visible_line(&self, line: &str, show_whitespace: bool) -> SanitizedLine {
        if !show_whitespace {
            return self.display_sanitizer.sanitize_line(line);
        }

        let visible =
            visible_whitespace_text(line, show_whitespace, self.display_sanitizer.ascii_only);
        self.display_sanitizer.sanitize_line(&visible)
    }

    fn cursor_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Option<UiCursor> {
        let inner_width = rect.width.checked_sub(2)? as usize;
        let gutter_width = gutter_width.min(inner_width as u16) as usize;
        let body_width = inner_width.saturating_sub(gutter_width);
        let body_height = rect.height.checked_sub(2)? as usize;
        if body_width == 0 || body_height == 0 {
            return None;
        }
        if buffer.wrap {
            return self.wrapped_cursor_for_buffer(buffer, gutter_width, body_width, body_height);
        }

        let position = buffer.buffer.cursor_position();
        if position.line < buffer.first_line {
            return None;
        }

        let visible_line = position.line - buffer.first_line;
        if visible_line >= body_height {
            return None;
        }

        let line = buffer.buffer.line(position.line)?;
        let visible_byte_start = self.byte_column_for_display_column(line, buffer.first_column);
        if position.column < visible_byte_start {
            return None;
        }
        let body_origin = self.display_column(line, visible_byte_start)?;
        let display_column = self.display_column(line, position.column)?;
        if display_column < body_origin {
            return None;
        }
        let display_column = display_column
            .saturating_sub(body_origin)
            .min(body_width.saturating_sub(1));

        Some(UiCursor {
            x: 1 + gutter_width as u16 + display_column as u16,
            y: 1 + visible_line as u16,
        })
    }

    fn wrapped_cursor_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        gutter_width: usize,
        body_width: usize,
        body_height: usize,
    ) -> Option<UiCursor> {
        let position = buffer.buffer.cursor_position();
        if position.line < buffer.first_line {
            return None;
        }

        let mut visual_y = -(buffer.first_visual_row as isize);
        for line_index in buffer.first_line..position.line {
            visual_y = visual_y.saturating_add(
                self.wrapped_visual_line_count(buffer, line_index, body_width) as isize,
            );
            if visual_y >= body_height as isize {
                return None;
            }
        }

        let line = buffer.buffer.line(position.line)?;
        let display_column = self.line_display_column_for_buffer(buffer, line, position.column)?;
        let row_offset = display_column / body_width;
        visual_y = visual_y.saturating_add(row_offset as isize);
        if visual_y < 0 || visual_y >= body_height as isize {
            return None;
        }
        let display_column = display_column % body_width;

        Some(UiCursor {
            x: 1 + gutter_width as u16 + display_column as u16,
            y: 1 + visual_y as u16,
        })
    }

    fn selection_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Vec<UiSelectionLine> {
        let Some(range) = buffer.buffer.selection_range() else {
            return Vec::new();
        };
        let Some(inner_width) = rect.width.checked_sub(2).map(|width| width as usize) else {
            return Vec::new();
        };
        let gutter_width = gutter_width.min(inner_width as u16) as usize;
        let body_width = inner_width.saturating_sub(gutter_width);
        let Some(body_height) = rect.height.checked_sub(2).map(|height| height as usize) else {
            return Vec::new();
        };
        if body_width == 0 || body_height == 0 || range.is_empty() {
            return Vec::new();
        }
        if buffer.wrap {
            return self.selection_for_wrapped_buffer(
                buffer,
                range,
                body_width,
                body_height,
                gutter_width,
            );
        }

        let mut lines = Vec::new();
        let visible_start = buffer.first_line;
        let visible_end = buffer.first_line.saturating_add(body_height);
        let start_line = range.start.line.max(visible_start);
        let end_line = range.end.line.min(visible_end.saturating_sub(1));
        if start_line > end_line {
            return Vec::new();
        }

        for line_index in start_line..=end_line {
            if let Some(line) =
                self.selection_line(buffer, line_index, range, body_width, gutter_width)
            {
                lines.push(line);
            }
        }

        lines
    }

    fn selection_line(
        &self,
        buffer: &BufferView<'_>,
        line_index: usize,
        range: TextRange,
        body_width: usize,
        gutter_width: usize,
    ) -> Option<UiSelectionLine> {
        let line = buffer.buffer.line(line_index)?;
        let start_column = if line_index == range.start.line {
            range.start.column
        } else {
            0
        };
        let end_column = if line_index == range.end.line {
            range.end.column
        } else {
            line.len()
        };
        if start_column >= end_column {
            return None;
        }

        let visible_byte_start = self.byte_column_for_display_column(line, buffer.first_column);
        if end_column <= visible_byte_start {
            return None;
        }
        let start_column = start_column.max(visible_byte_start);
        let body_origin = self.line_display_column_for_buffer(buffer, line, visible_byte_start)?;
        let last_column = body_origin.saturating_add(body_width);
        let start_display = self.line_display_column_for_buffer(buffer, line, start_column)?;
        let end_display = self.line_display_column_for_buffer(buffer, line, end_column)?;
        if end_display <= body_origin || start_display >= last_column {
            return None;
        }

        let start_x = start_display.saturating_sub(body_origin).min(body_width);
        let end_x = end_display.saturating_sub(body_origin).min(body_width);
        if start_x >= end_x {
            return None;
        }

        Some(UiSelectionLine {
            y: 1 + (line_index - buffer.first_line) as u16,
            start_x: 1 + start_x as u16 + gutter_width as u16,
            end_x: 1 + end_x as u16 + gutter_width as u16,
        })
    }

    fn selection_for_wrapped_buffer(
        &self,
        buffer: &BufferView<'_>,
        range: TextRange,
        body_width: usize,
        body_height: usize,
        gutter_width: usize,
    ) -> Vec<UiSelectionLine> {
        let mut lines = Vec::new();
        let mut visual_y = -(buffer.first_visual_row as isize);
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if visual_y >= body_height as isize {
                break;
            }
            let visual_rows = self.wrapped_visual_line_count(buffer, line_index, body_width);
            if line_index >= range.start.line && line_index <= range.end.line {
                let Some(line) = buffer.buffer.line(line_index) else {
                    visual_y = visual_y.saturating_add(visual_rows as isize);
                    continue;
                };
                let start_column = if line_index == range.start.line {
                    range.start.column
                } else {
                    0
                };
                let end_column = if line_index == range.end.line {
                    range.end.column
                } else {
                    line.len()
                };
                for (y, start_x, end_x) in self.wrapped_highlight_spans(
                    buffer,
                    line,
                    start_column,
                    end_column,
                    visual_y,
                    body_width,
                    body_height,
                    gutter_width,
                ) {
                    lines.push(UiSelectionLine { y, start_x, end_x });
                }
            }
            visual_y = visual_y.saturating_add(visual_rows as isize);
        }

        lines
    }

    fn search_matches_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Vec<UiSearchMatchLine> {
        if buffer.search_matches.is_empty() {
            return Vec::new();
        }
        let Some(inner_width) = rect.width.checked_sub(2).map(|width| width as usize) else {
            return Vec::new();
        };
        let gutter_width = gutter_width.min(inner_width as u16) as usize;
        let body_width = inner_width.saturating_sub(gutter_width);
        let Some(body_height) = rect.height.checked_sub(2).map(|height| height as usize) else {
            return Vec::new();
        };
        if body_width == 0 || body_height == 0 {
            return Vec::new();
        }
        if buffer.wrap {
            return self.search_matches_for_wrapped_buffer(
                buffer,
                body_width,
                body_height,
                gutter_width,
            );
        }

        let visible_start = buffer.first_line;
        let visible_end = buffer.first_line.saturating_add(body_height);
        let mut lines = Vec::new();
        for (index, item) in buffer.search_matches.iter().enumerate() {
            let range = item.range;
            if range.is_empty() || range.start.line != range.end.line {
                continue;
            }
            if range.start.line < visible_start || range.start.line >= visible_end {
                continue;
            }
            if let Some(line) =
                self.search_match_line(buffer, range, body_width, gutter_width, index)
            {
                lines.push(line);
            }
        }

        lines
    }

    fn search_match_line(
        &self,
        buffer: &BufferView<'_>,
        range: TextRange,
        body_width: usize,
        gutter_width: usize,
        index: usize,
    ) -> Option<UiSearchMatchLine> {
        let line = buffer.buffer.line(range.start.line)?;
        let visible_byte_start = self.byte_column_for_display_column(line, buffer.first_column);
        if range.end.column <= visible_byte_start {
            return None;
        }
        let start_column = range.start.column.max(visible_byte_start);
        let body_origin = self.line_display_column_for_buffer(buffer, line, visible_byte_start)?;
        let last_column = body_origin.saturating_add(body_width);
        let start_display = self.line_display_column_for_buffer(buffer, line, start_column)?;
        let end_display = self.line_display_column_for_buffer(buffer, line, range.end.column)?;
        if end_display <= body_origin || start_display >= last_column {
            return None;
        }

        let start_x = start_display.saturating_sub(body_origin).min(body_width);
        let end_x = end_display.saturating_sub(body_origin).min(body_width);
        if start_x >= end_x {
            return None;
        }

        Some(UiSearchMatchLine {
            y: 1 + (range.start.line - buffer.first_line) as u16,
            start_x: 1 + start_x as u16 + gutter_width as u16,
            end_x: 1 + end_x as u16 + gutter_width as u16,
            active: buffer.active_search_match == Some(index),
        })
    }

    fn search_matches_for_wrapped_buffer(
        &self,
        buffer: &BufferView<'_>,
        body_width: usize,
        body_height: usize,
        gutter_width: usize,
    ) -> Vec<UiSearchMatchLine> {
        let mut first_visible_row_by_line = Vec::new();
        let mut visual_y = -(buffer.first_visual_row as isize);
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if visual_y >= body_height as isize {
                break;
            }
            first_visible_row_by_line.push((line_index, visual_y));
            visual_y = visual_y.saturating_add(
                self.wrapped_visual_line_count(buffer, line_index, body_width) as isize,
            );
        }

        let mut lines = Vec::new();
        for (index, item) in buffer.search_matches.iter().enumerate() {
            let range = item.range;
            if range.is_empty() || range.start.line != range.end.line {
                continue;
            }
            let Some((_, visual_y)) = first_visible_row_by_line
                .iter()
                .find(|(line_index, _)| *line_index == range.start.line)
                .copied()
            else {
                continue;
            };
            let Some(line) = buffer.buffer.line(range.start.line) else {
                continue;
            };
            for (y, start_x, end_x) in self.wrapped_highlight_spans(
                buffer,
                line,
                range.start.column,
                range.end.column,
                visual_y,
                body_width,
                body_height,
                gutter_width,
            ) {
                lines.push(UiSearchMatchLine {
                    y,
                    start_x,
                    end_x,
                    active: buffer.active_search_match == Some(index),
                });
            }
        }

        lines
    }

    fn wrapped_highlight_spans(
        &self,
        buffer: &BufferView<'_>,
        line: &str,
        start_column: usize,
        end_column: usize,
        visual_y: isize,
        body_width: usize,
        body_height: usize,
        gutter_width: usize,
    ) -> Vec<(u16, u16, u16)> {
        if start_column >= end_column {
            return Vec::new();
        }
        let Some(start_display) = self.line_display_column_for_buffer(buffer, line, start_column)
        else {
            return Vec::new();
        };
        let Some(end_display) = self.line_display_column_for_buffer(buffer, line, end_column)
        else {
            return Vec::new();
        };
        if start_display >= end_display {
            return Vec::new();
        }

        let visible = visible_whitespace_text(
            line,
            buffer.show_whitespace,
            self.display_sanitizer.ascii_only,
        );
        let mut spans = Vec::new();
        let mut segment_start = 0usize;
        for (row_offset, segment) in wrap_line_segments(&visible, body_width).iter().enumerate() {
            let row = visual_y.saturating_add(row_offset as isize);
            let segment_width = display_width(segment);
            let segment_end = segment_start.saturating_add(segment_width);
            if row < 0 {
                segment_start = segment_end;
                continue;
            }
            if row >= body_height as isize {
                break;
            }
            let start = start_display.max(segment_start);
            let end = end_display.min(segment_end);
            if start < end {
                spans.push((
                    1 + row as u16,
                    1 + gutter_width as u16 + (start - segment_start) as u16,
                    1 + gutter_width as u16 + (end - segment_start) as u16,
                ));
            }
            segment_start = segment_end;
        }

        spans
    }

    fn scrollbar_for_buffer(&self, buffer: &BufferView<'_>, rect: Rect) -> Option<UiScrollbar> {
        let body_height = rect.height.checked_sub(2)? as usize;
        if body_height == 0 {
            return None;
        }
        let (total, top) = if buffer.wrap {
            let inner_width = rect.width.saturating_sub(2) as usize;
            let gutter_width = self.gutter_width_for_buffer(buffer, rect) as usize;
            let body_width = inner_width.saturating_sub(gutter_width).max(1);
            (
                self.wrapped_total_visual_rows(buffer, body_width),
                self.wrapped_top_visual_row(buffer, body_width),
            )
        } else {
            (buffer.buffer.line_count(), buffer.first_line)
        };
        if total <= body_height {
            return None;
        }

        let thumb_height = body_height
            .saturating_mul(body_height)
            .saturating_add(total.saturating_sub(1))
            / total;
        let thumb_height = thumb_height.max(1).min(body_height);
        let max_thumb_top = body_height.saturating_sub(thumb_height);
        let max_first_row = total.saturating_sub(body_height);
        let thumb_top = if max_first_row == 0 {
            0
        } else {
            top.min(max_first_row).saturating_mul(max_thumb_top) / max_first_row
        };

        Some(UiScrollbar {
            y: 1 + thumb_top as u16,
            height: thumb_height as u16,
        })
    }

    fn scrollbar_target_line_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        local_y: u16,
    ) -> Option<(usize, usize)> {
        let body_height = rect.height.checked_sub(2)? as usize;
        if body_height == 0 {
            return None;
        }
        let inner_width = rect.width.saturating_sub(2) as usize;
        let gutter_width = self.gutter_width_for_buffer(buffer, rect) as usize;
        let body_width = inner_width.saturating_sub(gutter_width).max(1);
        let total = if buffer.wrap {
            self.wrapped_total_visual_rows(buffer, body_width)
        } else {
            buffer.buffer.line_count()
        };
        if total <= body_height {
            return None;
        }

        let track_y = local_y.saturating_sub(1) as usize;
        let max_track_y = body_height.saturating_sub(1);
        let max_first_row = total.saturating_sub(body_height);
        if max_track_y == 0 {
            return Some((0, 0));
        }

        let target_row = track_y.min(max_track_y).saturating_mul(max_first_row) / max_track_y;
        if buffer.wrap {
            Some(self.wrapped_position_for_top_row(buffer, body_width, target_row))
        } else {
            Some((target_row, 0))
        }
    }

    fn wrapped_visual_line_count(
        &self,
        buffer: &BufferView<'_>,
        line_index: usize,
        body_width: usize,
    ) -> usize {
        let Some(line) = buffer.buffer.line(line_index) else {
            return 1;
        };
        let visible = visible_whitespace_text(
            line,
            buffer.show_whitespace,
            self.display_sanitizer.ascii_only,
        );
        wrap_line_segments(&visible, body_width.max(1)).len().max(1)
    }

    fn wrapped_total_visual_rows(&self, buffer: &BufferView<'_>, body_width: usize) -> usize {
        (0..buffer.buffer.line_count())
            .map(|line_index| self.wrapped_visual_line_count(buffer, line_index, body_width))
            .sum::<usize>()
            .max(1)
    }

    fn wrapped_top_visual_row(&self, buffer: &BufferView<'_>, body_width: usize) -> usize {
        let previous_rows = (0..buffer.first_line.min(buffer.buffer.line_count()))
            .map(|line_index| self.wrapped_visual_line_count(buffer, line_index, body_width))
            .sum::<usize>();
        let current_rows = self.wrapped_visual_line_count(buffer, buffer.first_line, body_width);
        previous_rows.saturating_add(buffer.first_visual_row.min(current_rows.saturating_sub(1)))
    }

    fn wrapped_position_for_top_row(
        &self,
        buffer: &BufferView<'_>,
        body_width: usize,
        target_row: usize,
    ) -> (usize, usize) {
        let mut remaining = target_row;
        for line_index in 0..buffer.buffer.line_count() {
            let rows = self.wrapped_visual_line_count(buffer, line_index, body_width);
            if remaining < rows {
                return (line_index, remaining);
            }
            remaining = remaining.saturating_sub(rows);
        }

        (buffer.buffer.line_count().saturating_sub(1), 0)
    }

    fn horizontal_edges_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Vec<UiHorizontalEdgeLine> {
        if buffer.wrap {
            return Vec::new();
        }

        let Some(inner_width) = rect.width.checked_sub(2).map(|width| width as usize) else {
            return Vec::new();
        };
        let gutter_width = gutter_width.min(inner_width as u16) as usize;
        let body_width = inner_width.saturating_sub(gutter_width);
        let Some(body_height) = rect.height.checked_sub(2).map(|height| height as usize) else {
            return Vec::new();
        };
        if body_width == 0 || body_height == 0 {
            return Vec::new();
        }

        let mut lines = Vec::new();
        for (visible_y, line_index) in (buffer.first_line..buffer.buffer.line_count())
            .take(body_height)
            .enumerate()
        {
            let line = buffer.buffer.line(line_index).unwrap_or_default();
            let width = display_width(line);
            let visible_byte_start = self.byte_column_for_display_column(line, buffer.first_column);
            let body_origin = self.display_column(line, visible_byte_start).unwrap_or(0);
            let left = visible_byte_start > 0;
            let right = width > body_origin.saturating_add(body_width);
            if left || right {
                lines.push(UiHorizontalEdgeLine {
                    y: 1 + visible_y as u16,
                    left,
                    right,
                });
            }
        }

        lines
    }

    fn gutter_width_for_buffer(&self, buffer: &BufferView<'_>, rect: Rect) -> u16 {
        let inner_width = rect.width.saturating_sub(2);
        let digits = decimal_digits(buffer.buffer.line_count().max(1));
        let width = (digits + 1) as u16;
        if inner_width < width.saturating_add(MIN_BODY_COLUMNS_WITH_GUTTER) {
            return 0;
        }

        width
    }

    fn gutter_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        rect: Rect,
        gutter_width: u16,
    ) -> Vec<UiGutterLine> {
        let body_height = rect.height.saturating_sub(2) as usize;
        if gutter_width == 0 || body_height == 0 {
            return Vec::new();
        }

        let label_digits = gutter_width.saturating_sub(1) as usize;
        let mut lines = Vec::new();
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if lines.len() >= body_height {
                break;
            }

            let marker = if buffer.bookmarks.contains(&line_index) {
                '*'
            } else {
                ' '
            };
            let visual_rows = if buffer.wrap {
                let inner_width = rect.width.saturating_sub(2) as usize;
                let body_width = inner_width.saturating_sub(gutter_width as usize).max(1);
                self.wrapped_visual_line_count(buffer, line_index, body_width)
            } else {
                1
            };
            let start_offset = if buffer.wrap && line_index == buffer.first_line {
                buffer.first_visual_row.min(visual_rows.saturating_sub(1))
            } else {
                0
            };
            for row_offset in start_offset..visual_rows {
                if lines.len() >= body_height {
                    break;
                }
                let label = if row_offset == 0 {
                    format!("{:>label_digits$}{marker}", line_index + 1)
                } else {
                    format!("{:>label_digits$} ", "")
                };
                lines.push(UiGutterLine {
                    y: 1 + lines.len() as u16,
                    label,
                });
            }
        }

        lines
    }

    fn display_column(&self, line: &str, byte_column: usize) -> Option<usize> {
        let prefix = line.get(..byte_column)?;
        let display_sanitizer = DisplaySanitizer {
            ascii_only: self.display_sanitizer.ascii_only,
            max_bytes: usize::MAX,
        };
        let display_text = display_sanitizer.sanitize_line(prefix).as_plain_text();
        Some(UnicodeWidthStr::width(display_text.as_str()))
    }

    fn line_display_column_for_buffer(
        &self,
        buffer: &BufferView<'_>,
        line: &str,
        byte_column: usize,
    ) -> Option<usize> {
        let prefix = line.get(..byte_column)?;
        if !buffer.show_whitespace {
            return self.display_column(line, byte_column);
        }
        let display_sanitizer = DisplaySanitizer {
            ascii_only: self.display_sanitizer.ascii_only,
            max_bytes: usize::MAX,
        };
        let visible = visible_whitespace_prefix_text(prefix, self.display_sanitizer.ascii_only);
        let display_text = display_sanitizer.sanitize_line(&visible).as_plain_text();
        Some(UnicodeWidthStr::width(display_text.as_str()))
    }

    fn byte_column_for_display_column(&self, line: &str, target: usize) -> usize {
        if target == 0 {
            return 0;
        }

        let display_sanitizer = DisplaySanitizer {
            ascii_only: self.display_sanitizer.ascii_only,
            max_bytes: usize::MAX,
        };
        let mut width = 0usize;
        for (index, ch) in line.char_indices() {
            let next_index = index + ch.len_utf8();
            let mut raw = [0; 4];
            let rendered = display_sanitizer
                .sanitize_line(ch.encode_utf8(&mut raw))
                .as_plain_text();
            width = width.saturating_add(UnicodeWidthStr::width(rendered.as_str()));
            if width >= target {
                return next_index;
            }
        }

        line.len()
    }

    fn menu_bar(&self, active: Option<MenuSelection>) -> MenuBar {
        MenuBar {
            active,
            items: vec![
                MenuItem::new(
                    "File",
                    vec![
                        MenuEntry::new("New (N)", EditorCommand::File(dun_core::FileCommand::New)),
                        MenuEntry::new(
                            "Open... (O)",
                            EditorCommand::File(dun_core::FileCommand::Open),
                        ),
                        MenuEntry::new(
                            "Switch Buffer (B)",
                            EditorCommand::File(dun_core::FileCommand::SwitchBuffer),
                        ),
                        MenuEntry::new(
                            "Save (S)",
                            EditorCommand::File(dun_core::FileCommand::Save),
                        ),
                        MenuEntry::new(
                            "Save As... (A)",
                            EditorCommand::File(dun_core::FileCommand::SaveAs),
                        ),
                        MenuEntry::new(
                            "Reload (E)",
                            EditorCommand::File(dun_core::FileCommand::Reload),
                        ),
                        MenuEntry::new(
                            "Close (C)",
                            EditorCommand::File(dun_core::FileCommand::Close),
                        ),
                        MenuEntry::new(
                            "Run Command (R)",
                            EditorCommand::App(dun_core::AppCommand::RunCommand),
                        ),
                        MenuEntry::new(
                            "Shell Escape (H)",
                            EditorCommand::App(dun_core::AppCommand::ShellEscape),
                        ),
                        MenuEntry::new("Quit (Q)", EditorCommand::App(dun_core::AppCommand::Quit)),
                    ],
                ),
                MenuItem::new(
                    "Edit",
                    vec![
                        MenuEntry::new(
                            "Undo (U)",
                            EditorCommand::Edit(dun_core::EditCommand::Undo),
                        ),
                        MenuEntry::new(
                            "Redo (R)",
                            EditorCommand::Edit(dun_core::EditCommand::Redo),
                        ),
                        MenuEntry::new("Cut (T)", EditorCommand::Edit(dun_core::EditCommand::Cut)),
                        MenuEntry::new(
                            "Copy (C)",
                            EditorCommand::Edit(dun_core::EditCommand::Copy),
                        ),
                        MenuEntry::new(
                            "Copy External (X)",
                            EditorCommand::Edit(dun_core::EditCommand::CopyExternal),
                        ),
                        MenuEntry::new(
                            "Paste (P)",
                            EditorCommand::Edit(dun_core::EditCommand::Paste),
                        ),
                        MenuEntry::new(
                            "Select All (A)",
                            EditorCommand::Edit(dun_core::EditCommand::SelectAll),
                        ),
                        MenuEntry::new(
                            "Select Line (L)",
                            EditorCommand::Edit(dun_core::EditCommand::SelectLine),
                        ),
                        MenuEntry::new(
                            "Copy Line (Y)",
                            EditorCommand::Edit(dun_core::EditCommand::CopyLine),
                        ),
                        MenuEntry::new(
                            "Delete Line (K)",
                            EditorCommand::Edit(dun_core::EditCommand::DeleteLine),
                        ),
                        MenuEntry::new(
                            "Indent Line (I)",
                            EditorCommand::Edit(dun_core::EditCommand::IndentLine),
                        ),
                        MenuEntry::new(
                            "Outdent Line (O)",
                            EditorCommand::Edit(dun_core::EditCommand::OutdentLine),
                        ),
                        MenuEntry::new(
                            "Trim Whitespace (W)",
                            EditorCommand::Edit(dun_core::EditCommand::TrimTrailingWhitespace),
                        ),
                        MenuEntry::new(
                            "Find (F)",
                            EditorCommand::Edit(dun_core::EditCommand::Find),
                        ),
                        MenuEntry::new(
                            "Find Next (N)",
                            EditorCommand::Edit(dun_core::EditCommand::FindNext),
                        ),
                        MenuEntry::new(
                            "Replace (B)",
                            EditorCommand::Edit(dun_core::EditCommand::Replace),
                        ),
                        MenuEntry::new(
                            "Go To Line (G)",
                            EditorCommand::Edit(dun_core::EditCommand::GoToLine),
                        ),
                    ],
                ),
                MenuItem::new(
                    "View",
                    vec![
                        MenuEntry::new(
                            "Split Horizontal (H)",
                            EditorCommand::Window(dun_core::WindowCommand::SplitHorizontal),
                        ),
                        MenuEntry::new(
                            "Split Vertical (V)",
                            EditorCommand::Window(dun_core::WindowCommand::SplitVertical),
                        ),
                        MenuEntry::new(
                            "Equalize (E)",
                            EditorCommand::Window(dun_core::WindowCommand::Equalize),
                        ),
                        MenuEntry::new(
                            "Toggle Collapse (C)",
                            EditorCommand::Window(dun_core::WindowCommand::ToggleCollapse),
                        ),
                        MenuEntry::new(
                            "Word Wrap (Z)",
                            EditorCommand::Edit(dun_core::EditCommand::ToggleWordWrap),
                        ),
                        MenuEntry::new(
                            "Visible Whitespace (.)",
                            EditorCommand::Edit(dun_core::EditCommand::ToggleVisibleWhitespace),
                        ),
                        MenuEntry::new(
                            "Toggle Bookmark (M)",
                            EditorCommand::Edit(dun_core::EditCommand::ToggleBookmark),
                        ),
                        MenuEntry::new(
                            "Next Bookmark (N)",
                            EditorCommand::Edit(dun_core::EditCommand::NextBookmark),
                        ),
                        MenuEntry::new(
                            "Previous Bookmark (P)",
                            EditorCommand::Edit(dun_core::EditCommand::PreviousBookmark),
                        ),
                        MenuEntry::new(
                            "Scroll Left ([)",
                            EditorCommand::Edit(dun_core::EditCommand::ScrollLeft),
                        ),
                        MenuEntry::new(
                            "Scroll Right (])",
                            EditorCommand::Edit(dun_core::EditCommand::ScrollRight),
                        ),
                        MenuEntry::new(
                            "Close Window (X)",
                            EditorCommand::Window(dun_core::WindowCommand::Close),
                        ),
                        MenuEntry::new(
                            "Outline (/)",
                            EditorCommand::App(dun_core::AppCommand::Outline),
                        ),
                        MenuEntry::new(
                            "Search Results (W)",
                            EditorCommand::App(dun_core::AppCommand::SearchResults),
                        ),
                        MenuEntry::new(
                            "Status History (S)",
                            EditorCommand::App(dun_core::AppCommand::StatusHistory),
                        ),
                        MenuEntry::new(
                            "Output Index (I)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputIndex),
                        ),
                        MenuEntry::new(
                            "Output Summary (B)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputSummary),
                        ),
                        MenuEntry::new(
                            "Output Status (T)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputStatus),
                        ),
                        MenuEntry::new(
                            "Output Stdout (U)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputStdout),
                        ),
                        MenuEntry::new(
                            "Output Stdout Body (J)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputStdoutBody),
                        ),
                        MenuEntry::new(
                            "Output Stderr (O)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputStderr),
                        ),
                        MenuEntry::new(
                            "Output Stderr Body (L)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputStderrBody),
                        ),
                        MenuEntry::new(
                            "Output Truncated (G)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputTruncated),
                        ),
                        MenuEntry::new(
                            "Output Next Match (F)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputNextMatch),
                        ),
                        MenuEntry::new(
                            "Output Previous Match (Q)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputPreviousMatch),
                        ),
                        MenuEntry::new(
                            "Output Next Section (1)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputNextSection),
                        ),
                        MenuEntry::new(
                            "Output Previous Section (2)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputPreviousSection),
                        ),
                        MenuEntry::new(
                            "Output Only Stdout (3)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputOnlyStdout),
                        ),
                        MenuEntry::new(
                            "Output Only Stderr (4)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputOnlyStderr),
                        ),
                        MenuEntry::new(
                            "Output Copy (Y)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputCopy),
                        ),
                        MenuEntry::new(
                            "Output Save... (A)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputSave),
                        ),
                        MenuEntry::new(
                            "Output Clear (K)",
                            EditorCommand::App(dun_core::AppCommand::CommandOutputClear),
                        ),
                        MenuEntry::new(
                            "Config Diagnostics (D)",
                            EditorCommand::App(dun_core::AppCommand::ConfigDiagnostics),
                        ),
                        MenuEntry::new(
                            "Reload Config (R)",
                            EditorCommand::App(dun_core::AppCommand::ReloadConfig),
                        ),
                    ],
                ),
                MenuItem::new(
                    "Help",
                    vec![MenuEntry::new(
                        "Help (H)",
                        EditorCommand::App(dun_core::AppCommand::Help),
                    )],
                ),
            ],
        }
    }

    fn status_bar(&self, workspace: &Workspace, visible_windows: usize) -> StatusBar {
        StatusBar {
            left: format!("{} window(s)", visible_windows),
            right: format!("theme={} colors={:?}", self.theme.name, self.profile.colors),
            focused_window: workspace.focused,
        }
    }
}

impl Default for UiShell {
    fn default() -> Self {
        Self::from_config(&Config::default(), TerminalProfile::default())
    }
}

#[cfg(test)]
mod tests;
