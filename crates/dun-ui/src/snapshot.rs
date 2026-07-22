use std::fmt::Write as _;

use dun_term::{AnsiColor, Style, TerminalColor};

use crate::render::surface_frame::render_ui_frame_to_surface;
use crate::surface::Surface;
use crate::{UiFrame, UiShell};

const STYLE_KEY_CHARS: &[u8; 62] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Render a frame exactly as the terminal backend would, and format it for a
/// golden file: the glyph grid, a per-cell style map, and a legend.
pub fn frame_snapshot(shell: &UiShell, frame: &UiFrame, width: u16, height: u16) -> String {
    let mut surface = Surface::new(width, height, shell.theme.palette.editor)
        .with_ambiguous_width(shell.profile.ambiguous_width);
    let cursor = render_ui_frame_to_surface(&mut surface, shell, frame);
    let row_digits = height.saturating_sub(1).to_string().len();
    let mut text_rows = Vec::with_capacity(usize::from(height));
    let mut style_rows = Vec::with_capacity(usize::from(height));
    let mut styles = Vec::new();

    for y in 0..height {
        let mut text = String::new();
        let mut style_map = String::new();
        for x in 0..width {
            let cell = surface
                .cell(x, y)
                .expect("coordinates within the surface must have a cell");
            if !cell.wide_continuation {
                text.push_str(&cell.symbol);
            }

            let style_index = styles
                .iter()
                .position(|style| style == &cell.style)
                .unwrap_or_else(|| {
                    styles.push(cell.style);
                    styles.len() - 1
                });
            style_map.push_str(&style_key(style_index));
        }
        text_rows.push(text);
        style_rows.push(style_map);
    }

    let mut output = String::new();
    writeln!(
        output,
        "size: {width}x{height}  theme: {}  colors: {:?}",
        shell.theme.name, shell.profile.colors
    )
    .expect("writing to a String cannot fail");
    match cursor {
        Some((x, y)) => writeln!(output, "cursor: {x},{y}"),
        None => writeln!(output, "cursor: none"),
    }
    .expect("writing to a String cannot fail");

    output.push_str("\ntext:\n");
    for (row, text) in text_rows.iter().enumerate() {
        writeln!(output, "{row:>row_digits$}|{text}").expect("writing to a String cannot fail");
    }

    output.push_str("\nstyle:\n");
    for (row, style_map) in style_rows.iter().enumerate() {
        writeln!(output, "{row:>row_digits$}|{style_map}")
            .expect("writing to a String cannot fail");
    }

    output.push_str("\nlegend:\n");
    for (index, style) in styles.iter().enumerate() {
        writeln!(output, "{} = {}", style_key(index), style_text(*style))
            .expect("writing to a String cannot fail");
    }

    output
}

fn style_key(index: usize) -> String {
    if index < STYLE_KEY_CHARS.len() {
        return char::from(STYLE_KEY_CHARS[index]).to_string();
    }

    let radix = STYLE_KEY_CHARS.len();
    let mut value = index - radix;
    let mut digits = Vec::new();
    loop {
        digits.push(STYLE_KEY_CHARS[value % radix]);
        value /= radix;
        if value == 0 {
            break;
        }
    }
    while digits.len() < 2 {
        digits.push(STYLE_KEY_CHARS[0]);
    }
    digits.reverse();
    digits.into_iter().map(char::from).collect()
}

fn style_text(style: Style) -> String {
    let mut text = format!("{}/{}", color_text(style.fg), color_text(style.bg));
    if style.attrs.bold || style.attrs.underline || style.attrs.reverse {
        text.push(' ');
        if style.attrs.bold {
            text.push('b');
        }
        if style.attrs.underline {
            text.push('u');
        }
        if style.attrs.reverse {
            text.push('r');
        }
    }
    text
}

fn color_text(color: TerminalColor) -> String {
    match color {
        TerminalColor::Default => "d".to_string(),
        TerminalColor::Indexed(index) => index.to_string(),
        TerminalColor::Ansi(color) => ansi_color_text(color).to_string(),
    }
}

const fn ansi_color_text(color: AnsiColor) -> &'static str {
    match color {
        AnsiColor::Black => "black",
        AnsiColor::Red => "red",
        AnsiColor::Green => "green",
        AnsiColor::Yellow => "yellow",
        AnsiColor::Blue => "blue",
        AnsiColor::Magenta => "magenta",
        AnsiColor::Cyan => "cyan",
        AnsiColor::White => "white",
        AnsiColor::BrightBlack => "bright_black",
        AnsiColor::BrightRed => "bright_red",
        AnsiColor::BrightGreen => "bright_green",
        AnsiColor::BrightYellow => "bright_yellow",
        AnsiColor::BrightBlue => "bright_blue",
        AnsiColor::BrightMagenta => "bright_magenta",
        AnsiColor::BrightCyan => "bright_cyan",
        AnsiColor::BrightWhite => "bright_white",
    }
}

#[cfg(test)]
mod tests {
    use dun_core::{BufferId, BufferKind, Rect, TextBuffer, Workspace};
    use dun_term::StyleAttrs;

    use super::*;
    use crate::BufferView;

    #[test]
    fn keys_cross_from_single_to_double_characters() {
        assert_eq!(style_key(0), "A");
        assert_eq!(style_key(61), "9");
        assert_eq!(style_key(62), "AA");
        assert_eq!(style_key(63), "AB");
    }

    #[test]
    fn style_spelling_covers_ansi_and_attributes() {
        let style = Style::new(
            TerminalColor::Ansi(AnsiColor::BrightBlue),
            TerminalColor::Default,
            StyleAttrs {
                bold: true,
                underline: true,
                reverse: true,
            },
        );

        assert_eq!(style_text(style), "bright_blue/d bur");
    }

    #[test]
    fn snapshot_renders_wide_cells_without_touching_a_terminal() {
        let shell = UiShell::default();
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "界x");
        let view = BufferView::new(BufferId(1), &buffer);
        let frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 12, 3), &[view]);

        let snapshot = frame_snapshot(&shell, &frame, 12, 5);

        assert!(snapshot.starts_with("size: 12x5  theme: dun  colors: Color256\n"));
        assert!(snapshot.contains("\ntext:\n"));
        assert!(snapshot.contains("界x"));
        assert!(snapshot.contains("\nstyle:\n"));
        assert!(snapshot.contains("\nlegend:\n"));
        assert!(snapshot.ends_with('\n'));
    }
}
