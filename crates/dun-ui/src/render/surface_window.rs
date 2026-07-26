use dun_core::Rect as TuiRect;
use dun_core::{DisplayClass, SanitizedLine};
use dun_term::{AmbiguousWidth, Style, char_width};

use crate::render::surface_draw::draw_border;
use crate::render::window::{offset_rect, window_title_for_width};
use crate::surface::Surface;
use crate::{
    HighlightClass, UiCursor, UiGutterLine, UiHighlightLine, UiHorizontalEdgeLine, UiScrollbar,
    UiSearchMatchLine, UiSelectionLine, UiShell, UiWindow,
};

pub(crate) fn draw_window(
    surface: &mut Surface,
    shell: &UiShell,
    window: &UiWindow,
    workspace: TuiRect,
) -> Option<(u16, u16)> {
    let area = offset_rect(window.rect, workspace);
    if area.width == 0 || area.height == 0 {
        return None;
    }

    surface.fill_rect(
        area.x,
        area.y,
        area.width,
        area.height,
        ' ',
        shell.theme.palette.editor,
    );

    let border_style = if window.focused {
        shell.theme.palette.window_border_focused
    } else {
        shell.theme.palette.window_border
    };
    draw_border(
        surface,
        area.x,
        area.y,
        area.width,
        area.height,
        window.border,
        border_style,
    );
    draw_window_title(surface, shell, window, area);

    let inner_area = offset_rect(window.geometry.inner, area);
    let gutter_area = offset_rect(window.geometry.gutter, area);
    let body_area = offset_rect(window.geometry.body, area);
    if window.collapsed || inner_area.width == 0 || inner_area.height == 0 {
        return None;
    }

    if gutter_area.width > 0 {
        draw_gutter(
            surface,
            area,
            gutter_area,
            window.geometry.border_columns,
            &window.gutter,
            shell.theme.palette.gutter,
            shell.theme.palette.gutter_separator,
            shell.glyphs.border.vertical,
        );
    }

    if body_area.width > 0 {
        surface.fill_rect(
            body_area.x,
            body_area.y,
            body_area.width,
            body_area.height,
            ' ',
            shell.theme.palette.editor_text,
        );
        for (row, line) in window
            .body
            .iter()
            .take(usize::from(body_area.height))
            .enumerate()
        {
            draw_sanitized_line(
                surface,
                shell,
                line,
                body_area.x,
                body_area.y.saturating_add(row as u16),
                body_area.width,
            );
        }
    }

    draw_current_line(
        surface,
        area,
        body_area,
        window.cursor,
        shell.theme.palette.current_line,
    );
    draw_plugin_highlights(surface, area, body_area, shell, &window.highlights);
    draw_search_matches(
        surface,
        area,
        body_area,
        &window.search_matches,
        shell.theme.palette.search_match,
        shell.theme.palette.active_search_match,
    );
    draw_selection(
        surface,
        area,
        body_area,
        &window.selection,
        shell.theme.palette.selection_text,
    );
    draw_horizontal_edges(
        surface,
        shell,
        area,
        body_area,
        area.x.saturating_add(window.geometry.right_border_x),
        &window.horizontal_edges,
        shell.theme.palette.truncation,
    );
    draw_scrollbar(
        surface,
        shell,
        area,
        window.geometry.right_border_x,
        window.scrollbar.as_ref(),
        shell.theme.palette.scrollbar_thumb,
    );

    let cursor = window.cursor?;
    let x = area.x.saturating_add(cursor.x);
    let y = area.y.saturating_add(cursor.y);
    let inside = x < area.x.saturating_add(area.width) && y < area.y.saturating_add(area.height);
    inside.then_some((x, y))
}

fn draw_sanitized_line(
    surface: &mut Surface,
    shell: &UiShell,
    line: &SanitizedLine,
    x: u16,
    y: u16,
    width: u16,
) {
    let right = x.saturating_add(width).min(surface.width());
    let mut column = x;
    for segment in &line.segments {
        if column >= right {
            return;
        }

        let style = match segment.class {
            DisplayClass::Text => shell.theme.palette.editor_text,
            DisplayClass::Control => shell.theme.palette.control,
            DisplayClass::Escape => shell.theme.palette.escape,
            DisplayClass::Truncation => shell.theme.palette.truncation,
        };
        let (text, clipped) =
            prefix_for_width(&segment.text, right - column, shell.profile.ambiguous_width);
        column = column.saturating_add(surface.set_text(column, y, text, style));
        if clipped {
            return;
        }
    }
}

fn prefix_for_width(text: &str, max_width: u16, mode: AmbiguousWidth) -> (&str, bool) {
    let mut width = 0usize;
    for (index, ch) in text.char_indices() {
        let char_width = char_width(ch, mode).unwrap_or(0);
        if width.saturating_add(char_width) > usize::from(max_width) {
            return (&text[..index], true);
        }
        width = width.saturating_add(char_width);
    }
    (text, false)
}

fn draw_window_title(surface: &mut Surface, shell: &UiShell, window: &UiWindow, area: TuiRect) {
    if area.width <= 4 {
        return;
    }

    let title = window_title_for_width(shell, window, usize::from(area.width.saturating_sub(4)));
    if title.is_empty() {
        return;
    }

    let style = if window.focused {
        shell.theme.palette.title_focused
    } else {
        shell.theme.palette.title
    };
    surface.set_text(area.x + 2, area.y, &title, style);
}

#[allow(clippy::too_many_arguments)]
fn draw_gutter(
    surface: &mut Surface,
    window_area: TuiRect,
    gutter_area: TuiRect,
    separator_width: u16,
    gutter: &[UiGutterLine],
    style: Style,
    separator_style: Style,
    separator: char,
) {
    let right = gutter_area.x.saturating_add(gutter_area.width);
    for line in gutter {
        let y = window_area.y.saturating_add(line.y);
        if y < gutter_area.y || y >= gutter_area.y.saturating_add(gutter_area.height) {
            continue;
        }

        surface.style_run(gutter_area.x, y, gutter_area.width, style);
        surface.set_text(gutter_area.x, y, &line.label, style);
        if !line.marked && gutter_area.width >= separator_width && separator_width > 0 {
            set_char(
                surface,
                right.saturating_sub(separator_width),
                y,
                separator,
                separator_style,
            );
        }
    }
}

fn draw_current_line(
    surface: &mut Surface,
    window_area: TuiRect,
    body_area: TuiRect,
    cursor: Option<UiCursor>,
    style: Style,
) {
    let Some(cursor) = cursor else {
        return;
    };
    if body_area.width == 0 || body_area.height == 0 {
        return;
    }
    let y = window_area.y.saturating_add(cursor.y);
    if y < body_area.y || y >= body_area.y.saturating_add(body_area.height) {
        return;
    }

    surface.style_run(body_area.x, y, body_area.width, style);
}

fn draw_selection(
    surface: &mut Surface,
    window_area: TuiRect,
    body_area: TuiRect,
    selection: &[UiSelectionLine],
    style: Style,
) {
    for line in selection {
        style_window_run(
            surface,
            window_area,
            body_area,
            line.y,
            line.start_x,
            line.end_x,
            style,
        );
    }
}

fn draw_plugin_highlights(
    surface: &mut Surface,
    window_area: TuiRect,
    body_area: TuiRect,
    shell: &UiShell,
    highlights: &[UiHighlightLine],
) {
    for line in highlights {
        let palette = &shell.theme.palette;
        let style = match line.class {
            HighlightClass::Keyword => palette.syntax_keyword,
            HighlightClass::Comment => palette.syntax_comment,
            HighlightClass::StringLiteral => palette.syntax_string,
            HighlightClass::Number => palette.syntax_number,
            HighlightClass::Emphasis => palette.syntax_emphasis,
        };
        style_window_run(
            surface,
            window_area,
            body_area,
            line.y,
            line.start_x,
            line.end_x,
            style,
        );
    }
}

fn draw_search_matches(
    surface: &mut Surface,
    window_area: TuiRect,
    body_area: TuiRect,
    matches: &[UiSearchMatchLine],
    style: Style,
    active_style: Style,
) {
    for line in matches {
        style_window_run(
            surface,
            window_area,
            body_area,
            line.y,
            line.start_x,
            line.end_x,
            if line.active { active_style } else { style },
        );
    }
}

fn style_window_run(
    surface: &mut Surface,
    window_area: TuiRect,
    body_area: TuiRect,
    line_y: u16,
    start_x: u16,
    end_x: u16,
    style: Style,
) {
    let y = window_area.y.saturating_add(line_y);
    if y < body_area.y || y >= body_area.y.saturating_add(body_area.height) {
        return;
    }

    let start = window_area.x.saturating_add(start_x).max(body_area.x);
    let end = window_area
        .x
        .saturating_add(end_x)
        .min(body_area.x.saturating_add(body_area.width));
    surface.style_run(start, y, end.saturating_sub(start), style);
}

fn draw_scrollbar(
    surface: &mut Surface,
    shell: &UiShell,
    window_area: TuiRect,
    right_border_x: u16,
    scrollbar: Option<&UiScrollbar>,
    style: Style,
) {
    let Some(scrollbar) = scrollbar else {
        return;
    };
    if window_area.width == 0 || window_area.height <= 2 || scrollbar.height == 0 {
        return;
    }

    let x = window_area.x.saturating_add(right_border_x);
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
        set_char(surface, x, y, thumb, style);
    }
}

fn draw_horizontal_edges(
    surface: &mut Surface,
    shell: &UiShell,
    window_area: TuiRect,
    body_area: TuiRect,
    right_border_x: u16,
    edges: &[UiHorizontalEdgeLine],
    style: Style,
) {
    if body_area.width == 0 || body_area.height == 0 {
        return;
    }

    let (left, right) = if shell.profile.supports_unicode_glyphs() {
        ('‹', '›')
    } else {
        ('<', '>')
    };
    let left_width = u16::try_from(surface.glyph_width(left)).unwrap_or(1);
    let right_width = u16::try_from(surface.glyph_width(right)).unwrap_or(1);
    let right_x = right_border_x.saturating_sub(right_width);

    for edge in edges {
        let y = window_area.y.saturating_add(edge.y);
        if y < body_area.y || y >= body_area.y.saturating_add(body_area.height) {
            continue;
        }
        if edge.left && body_area.width >= left_width {
            set_char(surface, body_area.x, y, left, style);
        }
        if edge.right && body_area.width >= right_width && right_x >= body_area.x {
            set_char(surface, right_x, y, right, style);
        }
    }
}

fn set_char(surface: &mut Surface, x: u16, y: u16, ch: char, style: Style) {
    let mut encoded = [0; 4];
    surface.set_text(x, y, ch.encode_utf8(&mut encoded), style);
}
