#![forbid(unsafe_code)]

use dun_config::{Config, KeySequence, KeyStroke, Keymap};
use dun_core::{
    BufferId, DisplayClass, DisplaySanitizer, DisplaySegment, EditorCommand, Position, Rect,
    SanitizedLine, TextBuffer, TextRange, WindowId, WindowState, Workspace,
};
use dun_term::{
    AnsiColor, BorderGlyphs, EncodingProfile, GlyphSet, Style as DunStyle, StyleAttrs,
    TerminalColor, TerminalProfile, Theme,
};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position as TuiPosition, Rect as TuiRect};
use ratatui::prelude::{Color, Frame, Line, Modifier, Span, Style};
use ratatui::widgets::{Block, Paragraph};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
        let mut windows = Vec::new();

        for layout in workspace.resolved_layout(area) {
            if let Ok(window) = workspace.window(layout.id) {
                windows.push(self.window_model(window, layout.rect, workspace.focused, buffers));
            }
        }

        UiFrame {
            menu: self.menu_bar(),
            status: self.status_bar(workspace, windows.len()),
            windows,
        }
    }

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

    pub fn menu_command_at_column(&self, column: u16) -> Option<EditorCommand> {
        let mut x = 1usize;
        for item in self.menu_bar().items {
            let width = display_width(item.label).saturating_add(2);
            let end = x.saturating_add(width);
            if (column as usize) >= x && (column as usize) < end {
                return Some(item.command);
            }
            x = end;
        }

        None
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

        let line_index = buffer
            .first_line
            .saturating_add(local_y.saturating_sub(1) as usize);
        if line_index >= buffer.buffer.line_count() {
            return UiMouseTarget::Body(buffer_end_position(buffer.buffer));
        }

        let body_x = inner_x.saturating_sub(gutter_width) as usize;
        let line = buffer.buffer.line(line_index).unwrap_or_default();
        UiMouseTarget::Body(Position::new(
            line_index,
            self.byte_column_for_display_column(line, body_x),
        ))
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
            (false, Some(buffer)) => self.sanitize_buffer_body(buffer, rect),
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
            body,
        }
    }

    fn sanitize_buffer_body(&self, buffer: &BufferView<'_>, rect: Rect) -> Vec<SanitizedLine> {
        let body_height = rect.height.saturating_sub(2) as usize;
        if body_height == 0 {
            return Vec::new();
        }

        let mut lines = Vec::new();
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if lines.len() >= body_height {
                break;
            }

            let line = buffer.buffer.line(line_index).unwrap_or_default();
            lines.push(self.display_sanitizer.sanitize_line(line));
        }

        lines
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

        let position = buffer.buffer.cursor_position();
        if position.line < buffer.first_line {
            return None;
        }

        let visible_line = position.line - buffer.first_line;
        if visible_line >= body_height {
            return None;
        }

        let line = buffer.buffer.line(position.line)?;
        let display_column = self.display_column(line, position.column)?;
        let display_column = display_column.min(body_width.saturating_sub(1));

        Some(UiCursor {
            x: 1 + gutter_width as u16 + display_column as u16,
            y: 1 + visible_line as u16,
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

        let start_x = self.display_column(line, start_column)?.min(body_width);
        let end_x = self.display_column(line, end_column)?.min(body_width);
        if start_x >= end_x {
            return None;
        }

        Some(UiSelectionLine {
            y: 1 + (line_index - buffer.first_line) as u16,
            start_x: 1 + start_x as u16 + gutter_width as u16,
            end_x: 1 + end_x as u16 + gutter_width as u16,
        })
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

            lines.push(UiGutterLine {
                y: 1 + lines.len() as u16,
                label: format!("{:>label_digits$} ", line_index + 1),
            });
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

    fn menu_bar(&self) -> MenuBar {
        MenuBar {
            items: vec![
                MenuItem::new("New", EditorCommand::File(dun_core::FileCommand::New)),
                MenuItem::new("Open", EditorCommand::File(dun_core::FileCommand::Open)),
                MenuItem::new("Save", EditorCommand::File(dun_core::FileCommand::Save)),
                MenuItem::new("Find", EditorCommand::Edit(dun_core::EditCommand::Find)),
                MenuItem::new("Go", EditorCommand::Edit(dun_core::EditCommand::GoToLine)),
                MenuItem::new(
                    "Split",
                    EditorCommand::Window(dun_core::WindowCommand::SplitHorizontal),
                ),
                MenuItem::new(
                    "Status",
                    EditorCommand::App(dun_core::AppCommand::StatusHistory),
                ),
                MenuItem::new(
                    "Reload",
                    EditorCommand::App(dun_core::AppCommand::ReloadConfig),
                ),
                MenuItem::new(
                    "Config",
                    EditorCommand::App(dun_core::AppCommand::ConfigDiagnostics),
                ),
                MenuItem::new("Help", EditorCommand::App(dun_core::AppCommand::Help)),
                MenuItem::new("Quit", EditorCommand::App(dun_core::AppCommand::Quit)),
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

pub fn render_ui_frame(frame: &mut Frame<'_>, shell: &UiShell, ui_frame: &UiFrame) {
    let area = frame.area();
    render_background(frame, area, shell.theme.palette.editor);

    if area.height == 0 || area.width == 0 {
        return;
    }

    let menu_area = TuiRect::new(area.x, area.y, area.width, 1);
    render_menu(frame, shell, &ui_frame.menu, menu_area);

    if area.height == 1 {
        return;
    }

    let status_area = TuiRect::new(area.x, area.y + area.height - 1, area.width, 1);
    render_status(frame, shell, &ui_frame.status, status_area);

    if area.height <= 2 {
        return;
    }

    let workspace_area = TuiRect::new(area.x, area.y + 1, area.width, area.height - 2);
    for window in &ui_frame.windows {
        render_window(frame, shell, window, workspace_area);
    }
}

fn render_background(frame: &mut Frame<'_>, area: TuiRect, style: DunStyle) {
    frame.render_widget(Block::default().style(to_ratatui_style(style)), area);
}

fn render_menu(frame: &mut Frame<'_>, shell: &UiShell, menu: &MenuBar, area: TuiRect) {
    let mut spans = Vec::new();
    spans.push(Span::styled(
        " ",
        to_ratatui_style(shell.theme.palette.menu_text),
    ));

    for item in &menu.items {
        spans.push(Span::styled(
            " ",
            to_ratatui_style(shell.theme.palette.menu_text),
        ));
        let mut chars = item.label.chars();
        if let Some(first) = chars.next() {
            spans.push(Span::styled(
                first.to_string(),
                to_ratatui_style(shell.theme.palette.menu_hotkey),
            ));
            spans.push(Span::styled(
                chars.collect::<String>(),
                to_ratatui_style(shell.theme.palette.menu_text),
            ));
        }
        spans.push(Span::styled(
            " ",
            to_ratatui_style(shell.theme.palette.menu_text),
        ));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(to_ratatui_style(shell.theme.palette.menu_bar)),
        area,
    );
}

fn render_status(frame: &mut Frame<'_>, shell: &UiShell, status: &StatusBar, area: TuiRect) {
    let text = sanitized_status_text_for_width(shell, status, area.width as usize);

    frame.render_widget(
        Paragraph::new(text).style(to_ratatui_style(shell.theme.palette.status_bar)),
        area,
    );
}

fn render_window(frame: &mut Frame<'_>, shell: &UiShell, window: &UiWindow, workspace: TuiRect) {
    let area = offset_rect(window.rect, workspace);
    if area.width == 0 || area.height == 0 {
        return;
    }

    frame.render_widget(
        Block::default().style(to_ratatui_style(shell.theme.palette.editor)),
        area,
    );

    let border_style = if window.focused {
        shell.theme.palette.window_border_focused
    } else {
        shell.theme.palette.window_border
    };
    render_border(
        frame.buffer_mut(),
        area,
        window.border,
        to_ratatui_style(border_style),
    );
    render_window_title(frame.buffer_mut(), shell, window, area);

    if window.collapsed || area.width <= 2 || area.height <= 2 {
        return;
    }

    let inner_area = TuiRect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2);
    let gutter_width = window.gutter_width.min(inner_area.width);
    if gutter_width > 0 {
        render_gutter(
            frame.buffer_mut(),
            area,
            gutter_width,
            &window.gutter,
            to_ratatui_style(shell.theme.palette.gutter),
        );
    }

    let body_area = TuiRect::new(
        inner_area.x.saturating_add(gutter_width),
        inner_area.y,
        inner_area.width.saturating_sub(gutter_width),
        inner_area.height,
    );
    let body_lines = window
        .body
        .iter()
        .map(|line| sanitized_line_to_ratatui(shell, line))
        .collect::<Vec<_>>();
    if body_area.width > 0 {
        frame.render_widget(
            Paragraph::new(body_lines).style(to_ratatui_style(shell.theme.palette.editor_text)),
            body_area,
        );
    }
    render_selection(
        frame.buffer_mut(),
        area,
        &window.selection,
        to_ratatui_style(shell.theme.palette.selection_text),
    );

    if let Some(cursor) = window.cursor {
        let x = area.x.saturating_add(cursor.x);
        let y = area.y.saturating_add(cursor.y);
        if x < area.x.saturating_add(area.width) && y < area.y.saturating_add(area.height) {
            frame.set_cursor_position(TuiPosition::new(x, y));
        }
    }
}

fn render_gutter(
    buffer: &mut Buffer,
    window_area: TuiRect,
    gutter_width: u16,
    gutter: &[UiGutterLine],
    style: Style,
) {
    let right = window_area
        .x
        .saturating_add(window_area.width)
        .min(window_area.x.saturating_add(1).saturating_add(gutter_width));
    for line in gutter {
        let y = window_area.y.saturating_add(line.y);
        if y >= window_area.y.saturating_add(window_area.height) {
            continue;
        }

        for x in (window_area.x + 1)..right {
            buffer[(x, y)].set_style(style);
        }
        buffer.set_string(window_area.x + 1, y, &line.label, style);
    }
}

fn render_selection(
    buffer: &mut Buffer,
    window_area: TuiRect,
    selection: &[UiSelectionLine],
    style: Style,
) {
    for line in selection {
        let y = window_area.y.saturating_add(line.y);
        if y >= window_area.y.saturating_add(window_area.height) {
            continue;
        }

        let start = window_area.x.saturating_add(line.start_x);
        let end = window_area.x.saturating_add(line.end_x);
        let right = window_area.x.saturating_add(window_area.width);
        for x in start..end.min(right) {
            buffer[(x, y)].set_style(style);
        }
    }
}

fn offset_rect(rect: Rect, origin: TuiRect) -> TuiRect {
    TuiRect::new(
        origin.x.saturating_add(rect.x),
        origin.y.saturating_add(rect.y),
        rect.width.min(origin.width.saturating_sub(rect.x)),
        rect.height.min(origin.height.saturating_sub(rect.y)),
    )
}

fn decimal_digits(mut value: usize) -> usize {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}

fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn buffer_end_position(buffer: &TextBuffer) -> Position {
    let last_line = buffer.line_count().saturating_sub(1);
    let last_column = buffer.line(last_line).map(str::len).unwrap_or(0);
    Position::new(last_line, last_column)
}

fn fit_text_to_width(text: &str, max_width: usize, truncation: char) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let truncation_width = UnicodeWidthChar::width(truncation).unwrap_or(1);
    if truncation_width > max_width {
        return String::new();
    }

    let body_width = max_width - truncation_width;
    let mut out = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > body_width {
            break;
        }
        out.push(ch);
        width += ch_width;
    }
    out.push(truncation);
    out
}

fn status_text_for_width(left: &str, right: &str, width: usize, truncation: char) -> String {
    if width == 0 {
        return String::new();
    }

    let right_width = display_width(right);
    if !right.is_empty() && width >= right_width.saturating_add(2) {
        let left_width = width - right_width - 1;
        let left = fit_text_to_width(left, left_width, truncation);
        let gap = width.saturating_sub(display_width(&left).saturating_add(right_width));
        let mut out = left;
        out.push_str(&" ".repeat(gap));
        out.push_str(right);
        return out;
    }

    fit_text_to_width(left, width, truncation)
}

fn sanitized_status_text_for_width(shell: &UiShell, status: &StatusBar, width: usize) -> String {
    let left = sanitize_chrome_text(shell, &status.left);
    let right = sanitize_chrome_text(shell, &status.right);
    status_text_for_width(&left, &right, width, shell.glyphs.indicators.truncation)
}

fn sanitize_chrome_text(shell: &UiShell, text: &str) -> String {
    shell.display_sanitizer.sanitize_line(text).as_plain_text()
}

fn render_border(buffer: &mut Buffer, area: TuiRect, glyphs: BorderGlyphs, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let right = area.x + area.width - 1;
    let bottom = area.y + area.height - 1;

    if area.width == 1 || area.height == 1 {
        for y in area.y..=bottom {
            for x in area.x..=right {
                buffer[(x, y)].set_char(glyphs.horizontal).set_style(style);
            }
        }
        return;
    }

    buffer[(area.x, area.y)]
        .set_char(glyphs.top_left)
        .set_style(style);
    buffer[(right, area.y)]
        .set_char(glyphs.top_right)
        .set_style(style);
    buffer[(area.x, bottom)]
        .set_char(glyphs.bottom_left)
        .set_style(style);
    buffer[(right, bottom)]
        .set_char(glyphs.bottom_right)
        .set_style(style);

    for x in (area.x + 1)..right {
        buffer[(x, area.y)]
            .set_char(glyphs.horizontal)
            .set_style(style);
        buffer[(x, bottom)]
            .set_char(glyphs.horizontal)
            .set_style(style);
    }

    for y in (area.y + 1)..bottom {
        buffer[(area.x, y)]
            .set_char(glyphs.vertical)
            .set_style(style);
        buffer[(right, y)]
            .set_char(glyphs.vertical)
            .set_style(style);
    }
}

fn render_window_title(frame: &mut Buffer, shell: &UiShell, window: &UiWindow, area: TuiRect) {
    if area.width <= 4 {
        return;
    }

    let max_width = area.width.saturating_sub(4) as usize;
    let title = window_title_for_width(shell, window, max_width);
    if title.is_empty() {
        return;
    }

    let style = if window.focused {
        shell.theme.palette.title_focused
    } else {
        shell.theme.palette.title
    };
    frame.set_string(area.x + 2, area.y, title, to_ratatui_style(style));
}

fn window_title_for_width(shell: &UiShell, window: &UiWindow, max_width: usize) -> String {
    let mut title = String::new();
    title.push(' ');
    if window.focused {
        title.push(shell.glyphs.indicators.focused);
        title.push(' ');
    }
    title.push_str(&sanitize_chrome_text(shell, &window.title));
    if window.dirty {
        title.push(' ');
        title.push(shell.glyphs.indicators.dirty);
    }
    if window.read_only {
        title.push(' ');
        title.push(shell.glyphs.indicators.read_only);
    }
    if window.collapsed {
        title.push(' ');
        title.push(shell.glyphs.indicators.collapsed);
    }
    title.push(' ');

    fit_text_to_width(&title, max_width, shell.glyphs.indicators.truncation)
}

fn sanitized_line_to_ratatui<'a>(shell: &UiShell, line: &'a SanitizedLine) -> Line<'a> {
    let spans = line
        .segments
        .iter()
        .map(|segment| Span::styled(segment.text.clone(), display_segment_style(shell, segment)))
        .collect::<Vec<_>>();
    Line::from(spans)
}

fn display_segment_style(shell: &UiShell, segment: &DisplaySegment) -> Style {
    let style = match segment.class {
        DisplayClass::Text => shell.theme.palette.editor_text,
        DisplayClass::Control => shell.theme.palette.control,
        DisplayClass::Escape => shell.theme.palette.escape,
        DisplayClass::Truncation => shell.theme.palette.truncation,
    };
    to_ratatui_style(style)
}

fn to_ratatui_style(style: DunStyle) -> Style {
    let mut out = Style::default()
        .fg(to_ratatui_color(style.fg))
        .bg(to_ratatui_color(style.bg));

    let attrs = to_ratatui_modifier(style.attrs);
    if !attrs.is_empty() {
        out = out.add_modifier(attrs);
    }

    out
}

fn to_ratatui_modifier(attrs: StyleAttrs) -> Modifier {
    let mut modifier = Modifier::empty();
    if attrs.bold {
        modifier |= Modifier::BOLD;
    }
    if attrs.underline {
        modifier |= Modifier::UNDERLINED;
    }
    if attrs.reverse {
        modifier |= Modifier::REVERSED;
    }
    modifier
}

fn to_ratatui_color(color: TerminalColor) -> Color {
    match color {
        TerminalColor::Default => Color::Reset,
        TerminalColor::Indexed(index) => Color::Indexed(index),
        TerminalColor::Ansi(color) => match color {
            AnsiColor::Black => Color::Black,
            AnsiColor::Red => Color::Red,
            AnsiColor::Green => Color::Green,
            AnsiColor::Yellow => Color::Yellow,
            AnsiColor::Blue => Color::Blue,
            AnsiColor::Magenta => Color::Magenta,
            AnsiColor::Cyan => Color::Cyan,
            AnsiColor::White => Color::White,
            AnsiColor::BrightBlack => Color::DarkGray,
            AnsiColor::BrightRed => Color::LightRed,
            AnsiColor::BrightGreen => Color::LightGreen,
            AnsiColor::BrightYellow => Color::LightYellow,
            AnsiColor::BrightBlue => Color::LightBlue,
            AnsiColor::BrightMagenta => Color::LightMagenta,
            AnsiColor::BrightCyan => Color::LightCyan,
            AnsiColor::BrightWhite => Color::Gray,
        },
    }
}

impl Default for UiShell {
    fn default() -> Self {
        Self::from_config(&Config::default(), TerminalProfile::default())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BufferView<'a> {
    pub id: BufferId,
    pub buffer: &'a TextBuffer,
    pub first_line: usize,
}

impl<'a> BufferView<'a> {
    pub const fn new(id: BufferId, buffer: &'a TextBuffer) -> Self {
        Self {
            id,
            buffer,
            first_line: 0,
        }
    }

    pub const fn scrolled(id: BufferId, buffer: &'a TextBuffer, first_line: usize) -> Self {
        Self {
            id,
            buffer,
            first_line,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiFrame {
    pub menu: MenuBar,
    pub status: StatusBar,
    pub windows: Vec<UiWindow>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiMouseHit {
    pub window_id: WindowId,
    pub buffer_id: BufferId,
    pub target: UiMouseTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiMouseTarget {
    Chrome,
    Gutter,
    Body(Position),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuBar {
    pub items: Vec<MenuItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    pub label: &'static str,
    pub command: EditorCommand,
}

impl MenuItem {
    pub const fn new(label: &'static str, command: EditorCommand) -> Self {
        Self { label, command }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusBar {
    pub left: String,
    pub right: String,
    pub focused_window: WindowId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiWindow {
    pub id: WindowId,
    pub buffer_id: BufferId,
    pub title: String,
    pub rect: Rect,
    pub focused: bool,
    pub collapsed: bool,
    pub dirty: bool,
    pub read_only: bool,
    pub border: BorderGlyphs,
    pub gutter_width: u16,
    pub gutter: Vec<UiGutterLine>,
    pub cursor: Option<UiCursor>,
    pub selection: Vec<UiSelectionLine>,
    pub body: Vec<SanitizedLine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiGutterLine {
    pub y: u16,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiCursor {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiSelectionLine {
    pub y: u16,
    pub start_x: u16,
    pub end_x: u16,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use dun_config::{ColorProfile, EncodingProfile, KeySequence, TerminalOverrides};
    use dun_core::{AppCommand, Axis, BufferKind, FileCommand, Position};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn assert_no_raw_controls(text: &str) {
        assert!(
            !text.chars().any(char::is_control),
            "raw control text was rendered: {text:?}"
        );
        assert!(!text.contains('\x1b'), "raw ESC was rendered: {text:?}");
        assert!(
            !text.contains('\u{009b}'),
            "raw C1 CSI was rendered: {text:?}"
        );
    }

    #[test]
    fn shell_applies_configured_terminal_fallbacks() {
        let config = Config {
            terminal: TerminalOverrides {
                encoding: Some(EncodingProfile::Ascii),
                colors: Some(ColorProfile::Color16),
            },
            ..Config::default()
        };

        let shell = UiShell::from_config(&config, TerminalProfile::default());

        assert_eq!(shell.profile, TerminalProfile::ascii_16());
        assert_eq!(shell.glyphs, GlyphSet::ascii());
        assert_eq!(shell.theme.colors, ColorProfile::Color16);
        assert!(shell.display_sanitizer.ascii_only);
    }

    #[test]
    fn shell_resolves_keymap_commands() {
        let shell = UiShell::default();
        let sequence = KeySequence::from_str("Ctrl+S").unwrap();

        assert_eq!(
            shell.command_for_sequence(&sequence),
            Some(&EditorCommand::File(FileCommand::Save))
        );
    }

    #[test]
    fn frame_contains_menu_status_and_sanitized_buffer_content() {
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "safe\x1b]0;x\x07");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let shell = UiShell::default();

        let frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

        assert_eq!(frame.menu.items[0].label, "New");
        assert_eq!(frame.status.focused_window, WindowId(1));
        assert_eq!(frame.windows.len(), 1);
        assert_eq!(frame.windows[0].body[0].as_plain_text(), "safe␛]0;x␇");
        assert!(frame.windows[0].body[0].has_non_text_segments());
        assert_eq!(frame.windows[0].gutter_width, 2);
        assert_eq!(
            frame.windows[0].gutter,
            vec![UiGutterLine {
                y: 1,
                label: "1 ".to_string(),
            }]
        );
        assert_eq!(frame.windows[0].cursor, Some(UiCursor { x: 3, y: 1 }));
    }

    #[test]
    fn menu_command_hit_test_maps_columns_to_commands() {
        let shell = UiShell::default();

        assert_eq!(
            shell.menu_command_at_column(2),
            Some(EditorCommand::File(FileCommand::New))
        );
        assert_eq!(
            shell.menu_command_at_column(61),
            Some(EditorCommand::App(AppCommand::Help))
        );
        assert_eq!(shell.menu_command_at_column(0), None);
    }

    #[test]
    fn status_chrome_sanitizes_terminal_control_payloads() {
        let shell = UiShell::default();
        let status = StatusBar {
            left: "Opened \x1b]0;owned\x07.log".to_string(),
            right: "Ln 1 \x1b[31mred\x1b[0m".to_string(),
            focused_window: WindowId(1),
        };

        let text = sanitized_status_text_for_width(&shell, &status, 80);

        assert_no_raw_controls(&text);
        assert!(text.contains("␛]0;owned␇"));
        assert!(text.contains("␛[31mred␛[0m"));
    }

    #[test]
    fn window_title_sanitizes_terminal_control_payloads() {
        let mut workspace = Workspace::new_untitled();
        workspace.window_mut(WindowId(1)).unwrap().title = "evil\x1b]0;owned\x07.log".to_string();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let shell = UiShell::default();
        let frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

        let title = window_title_for_width(&shell, &frame.windows[0], 40);

        assert_no_raw_controls(&title);
        assert!(title.contains("evil␛]0;owned␇.log"));
    }

    #[test]
    fn ascii_chrome_sanitization_stays_ascii() {
        let config = Config {
            terminal: TerminalOverrides {
                encoding: Some(EncodingProfile::Ascii),
                colors: Some(ColorProfile::Color16),
            },
            ..Config::default()
        };
        let shell = UiShell::from_config(&config, TerminalProfile::default());
        let status = StatusBar {
            left: "打开 \x1b[2J".to_string(),
            right: "\u{009b}31m".to_string(),
            focused_window: WindowId(1),
        };
        let text = sanitized_status_text_for_width(&shell, &status, 80);

        assert_no_raw_controls(&text);
        assert!(text.is_ascii());
        assert!(text.contains("\\u{6253}\\u{5f00} ^[[2J"));
        assert!(text.contains("<U+009B>31m"));
    }

    #[test]
    fn frame_maps_buffer_cursor_to_window_body() {
        let workspace = Workspace::new_untitled();
        let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abc\né");
        buffer.set_cursor(dun_core::Position::new(1, 2)).unwrap();
        let buffer_view = BufferView::new(BufferId(1), &buffer);

        let frame = UiShell::default().frame_for_workspace(
            &workspace,
            Rect::new(0, 0, 80, 10),
            &[buffer_view],
        );

        assert_eq!(frame.windows[0].cursor, Some(UiCursor { x: 4, y: 2 }));
    }

    #[test]
    fn frame_maps_cursor_after_wide_utf8_text() {
        let workspace = Workspace::new_untitled();
        let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "中x");
        buffer
            .set_cursor(dun_core::Position::new(0, "中".len()))
            .unwrap();
        let buffer_view = BufferView::new(BufferId(1), &buffer);

        let frame = UiShell::default().frame_for_workspace(
            &workspace,
            Rect::new(0, 0, 80, 10),
            &[buffer_view],
        );

        assert_eq!(frame.windows[0].cursor, Some(UiCursor { x: 5, y: 1 }));
    }

    #[test]
    fn hit_test_maps_body_click_to_buffer_position() {
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcd");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let shell = UiShell::default();

        let hit = shell
            .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view], 5, 1)
            .unwrap();

        assert_eq!(hit.window_id, WindowId(1));
        assert_eq!(hit.buffer_id, BufferId(1));
        assert_eq!(hit.target, UiMouseTarget::Body(Position::new(0, 2)));
    }

    #[test]
    fn hit_test_maps_wide_character_click_to_valid_utf8_boundary() {
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "中x");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let shell = UiShell::default();

        let hit = shell
            .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view], 4, 1)
            .unwrap();

        assert_eq!(
            hit.target,
            UiMouseTarget::Body(Position::new(0, "中".len()))
        );
    }

    #[test]
    fn hit_test_separates_window_chrome_and_gutter() {
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcd");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let shell = UiShell::default();

        let chrome = shell
            .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view], 0, 0)
            .unwrap();
        let gutter = shell
            .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view], 1, 1)
            .unwrap();

        assert_eq!(chrome.target, UiMouseTarget::Chrome);
        assert_eq!(gutter.target, UiMouseTarget::Gutter);
    }

    #[test]
    fn hit_test_maps_empty_body_area_to_buffer_end() {
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcd");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let shell = UiShell::default();

        let hit = shell
            .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view], 10, 5)
            .unwrap();

        assert_eq!(hit.target, UiMouseTarget::Body(Position::new(0, 4)));
    }

    #[test]
    fn frame_maps_buffer_selection_to_window_body() {
        let workspace = Workspace::new_untitled();
        let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abc\n中x");
        buffer
            .select(Position::new(1, 0), Position::new(1, "中".len()))
            .unwrap();
        let buffer_view = BufferView::new(BufferId(1), &buffer);

        let frame = UiShell::default().frame_for_workspace(
            &workspace,
            Rect::new(0, 0, 80, 10),
            &[buffer_view],
        );

        assert_eq!(
            frame.windows[0].selection,
            vec![UiSelectionLine {
                y: 2,
                start_x: 3,
                end_x: 5,
            }]
        );
    }

    #[test]
    fn frame_maps_scrolled_line_number_gutter() {
        let workspace = Workspace::new_untitled();
        let buffer =
            TextBuffer::from_text_with_kind(BufferKind::Untitled, "1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
        let buffer_view = BufferView::scrolled(BufferId(1), &buffer, 8);

        let frame = UiShell::default().frame_for_workspace(
            &workspace,
            Rect::new(0, 0, 80, 6),
            &[buffer_view],
        );

        assert_eq!(frame.windows[0].gutter_width, 3);
        assert_eq!(
            frame.windows[0].gutter,
            vec![
                UiGutterLine {
                    y: 1,
                    label: " 9 ".to_string(),
                },
                UiGutterLine {
                    y: 2,
                    label: "10 ".to_string(),
                },
            ]
        );
    }

    #[test]
    fn narrow_window_omits_wide_gutter_to_keep_body_columns() {
        let workspace = Workspace::new_untitled();
        let text = (0..1000).map(|_| "x").collect::<Vec<_>>().join("\n");
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
        let buffer_view = BufferView::new(BufferId(1), &buffer);

        let frame = UiShell::default().frame_for_workspace(
            &workspace,
            Rect::new(0, 0, 8, 6),
            &[buffer_view],
        );

        assert_eq!(frame.windows[0].gutter_width, 0);
        assert!(frame.windows[0].gutter.is_empty());
        assert_eq!(frame.windows[0].body[0].as_plain_text(), "x");
        assert_eq!(frame.windows[0].cursor, Some(UiCursor { x: 1, y: 1 }));
    }

    #[test]
    fn tiny_windows_have_no_body_gutter_or_cursor() {
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hidden");
        let buffer_view = BufferView::new(BufferId(1), &buffer);

        let frame = UiShell::default().frame_for_workspace(
            &workspace,
            Rect::new(0, 0, 4, 2),
            &[buffer_view],
        );

        assert!(frame.windows[0].body.is_empty());
        assert_eq!(frame.windows[0].gutter_width, 0);
        assert!(frame.windows[0].gutter.is_empty());
        assert_eq!(frame.windows[0].cursor, None);
    }

    #[test]
    fn status_text_is_clipped_by_display_width() {
        let text = status_text_for_width(
            "日志服务-error.log*",
            "Ln 100/200 Col 42 | utf-8/256",
            12,
            '…',
        );

        assert!(display_width(&text) <= 12);
        assert_eq!(text.chars().last(), Some('…'));

        let text = status_text_for_width("file", "Ln 1", 12, '…');

        assert_eq!(display_width(&text), 12);
        assert!(text.starts_with("file"));
        assert!(text.ends_with("Ln 1"));
    }

    #[test]
    fn window_title_is_clipped_by_display_width() {
        let mut workspace = Workspace::new_untitled();
        workspace.window_mut(WindowId(1)).unwrap().title = "日志服务-error.log".to_string();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let shell = UiShell::default();
        let frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

        let title = window_title_for_width(&shell, &frame.windows[0], 8);

        assert!(display_width(&title) <= 8);
        assert_eq!(
            title.chars().last(),
            Some(shell.glyphs.indicators.truncation)
        );
    }

    #[test]
    fn frame_uses_tiled_workspace_rectangles() {
        let mut workspace = Workspace::new_untitled();
        workspace.split_focused(Axis::Horizontal).unwrap();

        let first = TextBuffer::from_text_with_kind(BufferKind::Untitled, "left");
        let second = TextBuffer::from_text_with_kind(BufferKind::Untitled, "right");
        let buffers = [
            BufferView::new(BufferId(1), &first),
            BufferView::new(BufferId(2), &second),
        ];

        let frame =
            UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 20), &buffers);

        assert_eq!(frame.windows.len(), 2);
        assert_eq!(frame.windows[0].rect, Rect::new(0, 0, 40, 20));
        assert_eq!(frame.windows[1].rect, Rect::new(40, 0, 40, 20));
        assert!(frame.windows[1].focused);
    }

    #[test]
    fn collapsed_window_has_no_body_lines() {
        let mut workspace = Workspace::new_untitled();
        workspace.collapse_focused().unwrap();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hidden");
        let buffer_view = BufferView::new(BufferId(1), &buffer);

        let frame = UiShell::default().frame_for_workspace(
            &workspace,
            Rect::new(0, 0, 80, 10),
            &[buffer_view],
        );

        assert!(frame.windows[0].collapsed);
        assert!(frame.windows[0].body.is_empty());
    }

    #[test]
    fn dirty_and_readonly_flags_follow_buffer_state() {
        let workspace = Workspace::new_untitled();
        let mut buffer = TextBuffer::from_text_with_kind(BufferKind::ReadOnly, "locked");
        buffer.set_cursor(Position::new(0, 0)).unwrap();
        let buffer_view = BufferView::new(BufferId(1), &buffer);

        let frame = UiShell::default().frame_for_workspace(
            &workspace,
            Rect::new(0, 0, 80, 10),
            &[buffer_view],
        );

        assert!(frame.windows[0].read_only);
        assert!(!frame.windows[0].dirty);
    }

    #[test]
    fn menu_exposes_help_and_quit_commands() {
        let menu = UiShell::default().menu_bar();

        assert!(
            menu.items
                .iter()
                .any(|item| item.command == EditorCommand::App(AppCommand::Help))
        );
        assert!(
            menu.items
                .iter()
                .any(|item| item.command == EditorCommand::App(AppCommand::StatusHistory))
        );
        assert!(
            menu.items
                .iter()
                .any(|item| item.command == EditorCommand::App(AppCommand::ReloadConfig))
        );
        assert!(
            menu.items
                .iter()
                .any(|item| item.command == EditorCommand::App(AppCommand::ConfigDiagnostics))
        );
        assert!(
            menu.items
                .iter()
                .any(|item| item.command == EditorCommand::App(AppCommand::Quit))
        );
    }

    #[test]
    fn ratatui_renderer_draws_frame_without_panicking() {
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hello\nworld");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let shell = UiShell::default();
        let ui_frame =
            shell.frame_for_workspace(&workspace, Rect::new(0, 0, 60, 8), &[buffer_view]);
        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| shell.render(frame, &ui_frame))
            .unwrap();
    }

    #[test]
    fn ratatui_renderer_draws_tiny_tiled_frame_without_panicking() {
        let mut workspace = Workspace::new_untitled();
        workspace.split_focused(Axis::Horizontal).unwrap();
        let first = TextBuffer::from_text_with_kind(BufferKind::Untitled, "left");
        let second = TextBuffer::from_text_with_kind(BufferKind::Untitled, "right");
        let buffers = [
            BufferView::new(BufferId(1), &first),
            BufferView::new(BufferId(2), &second),
        ];
        let shell = UiShell::default();
        let ui_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 8, 2), &buffers);
        let backend = TestBackend::new(8, 4);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| shell.render(frame, &ui_frame))
            .unwrap();
    }

    #[test]
    fn ratatui_renderer_does_not_emit_raw_controls_from_untrusted_text() {
        let mut workspace = Workspace::new_untitled();
        workspace.window_mut(WindowId(1)).unwrap().title = "title\x1b]0;owned\x07".to_string();
        let buffer = TextBuffer::from_text_with_kind(
            BufferKind::Untitled,
            "body\x1b[31mred\x1b[0m\n\u{009b}clear",
        );
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let shell = UiShell::default();
        let mut ui_frame =
            shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 8), &[buffer_view]);
        ui_frame.status.left = "Opened \x1b]52;c;SGVsbG8=\x07".to_string();
        ui_frame.status.right = "Ln \x1b[2J".to_string();
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| shell.render(frame, &ui_frame))
            .unwrap();

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert_no_raw_controls(&rendered);
        assert!(rendered.contains("␛]0;owned␇"));
        assert!(rendered.contains("␛[31mred␛[0m"));
        assert!(rendered.contains("<U+009B>clear"));
        assert!(rendered.contains("␛]52;c;SGVsbG8=␇"));
    }
}
