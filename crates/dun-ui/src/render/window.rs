use dun_core::Rect;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position as TuiPosition, Rect as TuiRect};
use ratatui::prelude::{Frame, Style};
use ratatui::widgets::{Block, Paragraph};

use crate::render::chrome::{
    render_border, sanitize_chrome_text, sanitized_line_to_ratatui, to_ratatui_style,
};
use crate::{
    HighlightClass, UiCursor, UiGutterLine, UiHighlightLine, UiHorizontalEdgeLine, UiScrollbar,
    UiSearchMatchLine, UiSelectionLine, UiShell, UiWindow, fit_text_to_width,
};

pub(crate) fn render_window(
    frame: &mut Frame<'_>,
    shell: &UiShell,
    window: &UiWindow,
    workspace: TuiRect,
) {
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
            to_ratatui_style(shell.theme.palette.gutter_separator),
            shell.glyphs.border.vertical,
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
    render_current_line(
        frame.buffer_mut(),
        body_area,
        window.cursor,
        to_ratatui_style(shell.theme.palette.current_line),
    );
    render_plugin_highlights(frame.buffer_mut(), area, shell, &window.highlights);
    render_search_matches(
        frame.buffer_mut(),
        area,
        &window.search_matches,
        to_ratatui_style(shell.theme.palette.search_match),
        to_ratatui_style(shell.theme.palette.active_search_match),
    );
    render_selection(
        frame.buffer_mut(),
        area,
        &window.selection,
        to_ratatui_style(shell.theme.palette.selection_text),
    );
    render_horizontal_edges(
        frame.buffer_mut(),
        shell,
        body_area,
        &window.horizontal_edges,
        to_ratatui_style(shell.theme.palette.truncation),
    );
    render_scrollbar(
        frame.buffer_mut(),
        shell,
        area,
        window.scrollbar.as_ref(),
        to_ratatui_style(shell.theme.palette.scrollbar_thumb),
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
    separator_style: Style,
    separator: char,
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
        if gutter_width > 0 && right > window_area.x + 1 {
            buffer[(right - 1, y)]
                .set_char(separator)
                .set_style(separator_style);
        }
    }
}

fn render_current_line(
    buffer: &mut Buffer,
    body_area: TuiRect,
    cursor: Option<UiCursor>,
    style: Style,
) {
    let Some(cursor) = cursor else {
        return;
    };
    if body_area.width == 0 || body_area.height == 0 || cursor.y == 0 {
        return;
    }
    let y = body_area.y.saturating_add(cursor.y.saturating_sub(1));
    if y >= body_area.y.saturating_add(body_area.height) {
        return;
    }

    for x in body_area.x..body_area.x.saturating_add(body_area.width) {
        buffer[(x, y)].set_style(style);
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

fn render_plugin_highlights(
    buffer: &mut Buffer,
    window_area: TuiRect,
    shell: &UiShell,
    highlights: &[UiHighlightLine],
) {
    for line in highlights {
        let y = window_area.y.saturating_add(line.y);
        if y >= window_area.y.saturating_add(window_area.height) {
            continue;
        }

        let palette = &shell.theme.palette;
        let style = to_ratatui_style(match line.class {
            HighlightClass::Keyword => palette.syntax_keyword,
            HighlightClass::Comment => palette.syntax_comment,
            HighlightClass::StringLiteral => palette.syntax_string,
            HighlightClass::Number => palette.syntax_number,
            HighlightClass::Emphasis => palette.syntax_emphasis,
        });
        let start = window_area.x.saturating_add(line.start_x);
        let end = window_area.x.saturating_add(line.end_x);
        let right = window_area.x.saturating_add(window_area.width);
        for x in start..end.min(right) {
            buffer[(x, y)].set_style(style);
        }
    }
}

fn render_search_matches(
    buffer: &mut Buffer,
    window_area: TuiRect,
    matches: &[UiSearchMatchLine],
    style: Style,
    active_style: Style,
) {
    for line in matches {
        let y = window_area.y.saturating_add(line.y);
        if y >= window_area.y.saturating_add(window_area.height) {
            continue;
        }

        let start = window_area.x.saturating_add(line.start_x);
        let end = window_area.x.saturating_add(line.end_x);
        let right = window_area.x.saturating_add(window_area.width);
        let style = if line.active { active_style } else { style };
        for x in start..end.min(right) {
            buffer[(x, y)].set_style(style);
        }
    }
}

fn render_scrollbar(
    buffer: &mut Buffer,
    shell: &UiShell,
    window_area: TuiRect,
    scrollbar: Option<&UiScrollbar>,
    style: Style,
) {
    let Some(scrollbar) = scrollbar else {
        return;
    };
    if window_area.width == 0 || window_area.height <= 2 || scrollbar.height == 0 {
        return;
    }

    let x = window_area
        .x
        .saturating_add(window_area.width)
        .saturating_sub(1);
    let bottom = window_area
        .y
        .saturating_add(window_area.height)
        .saturating_sub(1);
    let thumb = if shell.profile.supports_unicode_glyphs() {
        '█'
    } else {
        '#'
    };

    for offset in 0..scrollbar.height {
        let y = window_area
            .y
            .saturating_add(scrollbar.y)
            .saturating_add(offset);
        if y >= bottom {
            break;
        }
        buffer[(x, y)].set_char(thumb).set_style(style);
    }
}

fn render_horizontal_edges(
    buffer: &mut Buffer,
    shell: &UiShell,
    body_area: TuiRect,
    edges: &[UiHorizontalEdgeLine],
    style: Style,
) {
    if body_area.width == 0 || body_area.height == 0 {
        return;
    }

    let left = if shell.profile.supports_unicode_glyphs() {
        '‹'
    } else {
        '<'
    };
    let right = if shell.profile.supports_unicode_glyphs() {
        '›'
    } else {
        '>'
    };
    let right_x = body_area
        .x
        .saturating_add(body_area.width)
        .saturating_sub(1);

    for edge in edges {
        if edge.y == 0 {
            continue;
        }
        let y = body_area.y.saturating_add(edge.y.saturating_sub(1));
        if y >= body_area.y.saturating_add(body_area.height) {
            continue;
        }
        if edge.left {
            buffer[(body_area.x, y)].set_char(left).set_style(style);
        }
        if edge.right {
            buffer[(right_x, y)].set_char(right).set_style(style);
        }
    }
}

pub(crate) fn offset_rect(rect: Rect, origin: TuiRect) -> TuiRect {
    TuiRect::new(
        origin.x.saturating_add(rect.x),
        origin.y.saturating_add(rect.y),
        rect.width.min(origin.width.saturating_sub(rect.x)),
        rect.height.min(origin.height.saturating_sub(rect.y)),
    )
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

pub(crate) fn window_title_for_width(
    shell: &UiShell,
    window: &UiWindow,
    max_width: usize,
) -> String {
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
