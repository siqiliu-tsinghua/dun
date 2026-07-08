use dun_core::{DisplayClass, DisplaySegment, SanitizedLine};
use dun_term::{AnsiColor, BorderGlyphs, Style as DunStyle, StyleAttrs, TerminalColor};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as TuiRect;
use ratatui::prelude::{Color, Frame, Line, Modifier, Span, Style};
use ratatui::widgets::Block;

use crate::UiShell;

pub(crate) fn render_background(frame: &mut Frame<'_>, area: TuiRect, style: DunStyle) {
    frame.render_widget(Block::default().style(to_ratatui_style(style)), area);
}

pub(crate) fn render_vertical_overflow_indicators(
    buffer: &mut Buffer,
    shell: &UiShell,
    rect: TuiRect,
    has_more_above: bool,
    has_more_below: bool,
    style: Style,
) {
    if rect.width < 4 || rect.height < 3 {
        return;
    }

    let x = rect.x.saturating_add(rect.width).saturating_sub(2);
    if has_more_above {
        buffer[(x, rect.y)]
            .set_char(vertical_overflow_up(shell))
            .set_style(style);
    }
    if has_more_below {
        let y = rect.y.saturating_add(rect.height).saturating_sub(1);
        buffer[(x, y)]
            .set_char(vertical_overflow_down(shell))
            .set_style(style);
    }
}

pub(crate) fn vertical_overflow_up(shell: &UiShell) -> char {
    if shell.profile.supports_unicode_glyphs() {
        '↑'
    } else {
        '^'
    }
}

pub(crate) fn vertical_overflow_down(shell: &UiShell) -> char {
    if shell.profile.supports_unicode_glyphs() {
        '↓'
    } else {
        'v'
    }
}

pub(crate) fn sanitize_chrome_text(shell: &UiShell, text: &str) -> String {
    shell.display_sanitizer.sanitize_line(text).as_plain_text()
}

pub(crate) fn render_border(
    buffer: &mut Buffer,
    area: TuiRect,
    glyphs: BorderGlyphs,
    style: Style,
) {
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

pub(crate) fn sanitized_line_to_ratatui<'a>(shell: &UiShell, line: &'a SanitizedLine) -> Line<'a> {
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

pub(crate) fn to_ratatui_style(style: DunStyle) -> Style {
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
