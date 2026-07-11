use dun_term::{AnsiColor, Style, TerminalColor};

use crate::surface::{Surface, SurfaceCell};

pub(crate) fn emit_full(next: &Surface, out: &mut Vec<u8>) {
    if next.width() == 0 || next.height() == 0 {
        return;
    }

    let mut emitter = Emitter::new(out);
    for row in 0..next.height() {
        emitter.force_cursor(row, 0);
        emit_cells(next, row, 0, next.width(), &mut emitter);
    }
}

pub(crate) fn emit_diff(prev: &Surface, next: &Surface, out: &mut Vec<u8>) {
    if prev.width() != next.width() || prev.height() != next.height() {
        emit_full(next, out);
        return;
    }
    if next.width() == 0 || next.height() == 0 {
        return;
    }

    let mut emitter = Emitter::new(out);
    for row in 0..next.height() {
        let mut column = 0;
        while column < next.width() {
            while column < next.width() && !cell_changed(prev, next, column, row) {
                column += 1;
            }
            if column == next.width() {
                break;
            }

            let raw_start = column;
            while column < next.width() && cell_changed(prev, next, column, row) {
                column += 1;
            }

            let run_start = if raw_start > 0
                && next
                    .cell(raw_start, row)
                    .is_some_and(|cell| cell.wide_continuation)
            {
                raw_start - 1
            } else {
                raw_start
            };
            emitter.move_cursor(row, run_start);
            emit_cells(next, row, run_start, column, &mut emitter);
        }
    }
}

fn cell_changed(prev: &Surface, next: &Surface, column: u16, row: u16) -> bool {
    prev.cell(column, row) != next.cell(column, row)
}

fn emit_cells(surface: &Surface, row: u16, start: u16, end: u16, emitter: &mut Emitter<'_>) {
    let mut column = start;
    while column < end {
        let Some(cell) = surface.cell(column, row) else {
            break;
        };
        if cell.wide_continuation {
            column += 1;
            continue;
        }

        let display_width = if column + 1 < surface.width()
            && surface
                .cell(column + 1, row)
                .is_some_and(|next_cell| next_cell.wide_continuation)
        {
            2
        } else {
            1
        };
        emitter.write_cell(cell, display_width);
        column += display_width;
    }
}

struct Emitter<'a> {
    out: &'a mut Vec<u8>,
    pen: Option<Style>,
    cursor: Option<(u16, u32)>,
}

impl<'a> Emitter<'a> {
    fn new(out: &'a mut Vec<u8>) -> Self {
        Self {
            out,
            pen: None,
            cursor: None,
        }
    }

    fn force_cursor(&mut self, row: u16, column: u16) {
        emit_cup(row, column, self.out);
        self.cursor = Some((row, u32::from(column)));
    }

    fn move_cursor(&mut self, row: u16, column: u16) {
        let position = (row, u32::from(column));
        if self.cursor != Some(position) {
            emit_cup(row, column, self.out);
            self.cursor = Some(position);
        }
    }

    fn write_cell(&mut self, cell: &SurfaceCell, display_width: u16) {
        if self.pen != Some(cell.style) {
            emit_sgr(cell.style, self.out);
            self.pen = Some(cell.style);
        }
        self.out.extend_from_slice(cell.symbol.as_bytes());
        if let Some((row, column)) = self.cursor {
            self.cursor = Some((row, column + u32::from(display_width)));
        }
    }
}

fn emit_cup(row: u16, column: u16, out: &mut Vec<u8>) {
    out.extend_from_slice(b"\x1b[");
    push_decimal(u32::from(row) + 1, out);
    out.push(b';');
    push_decimal(u32::from(column) + 1, out);
    out.push(b'H');
}

fn emit_sgr(style: Style, out: &mut Vec<u8>) {
    out.extend_from_slice(b"\x1b[0");
    if style.attrs.bold {
        push_parameter(1, out);
    }
    if style.attrs.underline {
        push_parameter(4, out);
    }
    if style.attrs.reverse {
        push_parameter(7, out);
    }
    emit_foreground(style.fg, out);
    emit_background(style.bg, out);
    out.push(b'm');
}

fn emit_foreground(color: TerminalColor, out: &mut Vec<u8>) {
    match color {
        TerminalColor::Default => push_parameter(39, out),
        TerminalColor::Ansi(color) => {
            push_parameter(u32::from(ansi_code(color, 30, 90)), out);
        }
        TerminalColor::Indexed(index) => {
            push_parameter(38, out);
            push_parameter(5, out);
            push_parameter(u32::from(index), out);
        }
    }
}

fn emit_background(color: TerminalColor, out: &mut Vec<u8>) {
    match color {
        TerminalColor::Default => push_parameter(49, out),
        TerminalColor::Ansi(color) => {
            push_parameter(u32::from(ansi_code(color, 40, 100)), out);
        }
        TerminalColor::Indexed(index) => {
            push_parameter(48, out);
            push_parameter(5, out);
            push_parameter(u32::from(index), out);
        }
    }
}

fn ansi_code(color: AnsiColor, normal_base: u8, bright_base: u8) -> u8 {
    match color {
        AnsiColor::Black => normal_base,
        AnsiColor::Red => normal_base + 1,
        AnsiColor::Green => normal_base + 2,
        AnsiColor::Yellow => normal_base + 3,
        AnsiColor::Blue => normal_base + 4,
        AnsiColor::Magenta => normal_base + 5,
        AnsiColor::Cyan => normal_base + 6,
        AnsiColor::White => normal_base + 7,
        AnsiColor::BrightBlack => bright_base,
        AnsiColor::BrightRed => bright_base + 1,
        AnsiColor::BrightGreen => bright_base + 2,
        AnsiColor::BrightYellow => bright_base + 3,
        AnsiColor::BrightBlue => bright_base + 4,
        AnsiColor::BrightMagenta => bright_base + 5,
        AnsiColor::BrightCyan => bright_base + 6,
        AnsiColor::BrightWhite => bright_base + 7,
    }
}

fn push_parameter(value: u32, out: &mut Vec<u8>) {
    out.push(b';');
    push_decimal(value, out);
}

fn push_decimal(mut value: u32, out: &mut Vec<u8>) {
    let mut digits = [0; 10];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[start..]);
}

#[cfg(test)]
mod tests {
    use dun_term::{AnsiColor, Style, StyleAttrs, TerminalColor};

    use super::{emit_diff, emit_full};
    use crate::surface::Surface;

    const DEFAULT_STYLE: Style = Style::plain(TerminalColor::Default, TerminalColor::Default);
    const ACCENT_STYLE: Style = Style::plain(
        TerminalColor::Ansi(AnsiColor::White),
        TerminalColor::Ansi(AnsiColor::Blue),
    );

    fn full_text(surface: &Surface) -> String {
        let mut out = Vec::new();
        emit_full(surface, &mut out);
        String::from_utf8(out).expect("emitter output is UTF-8")
    }

    fn diff_text(prev: &Surface, next: &Surface) -> String {
        let mut out = Vec::new();
        emit_diff(prev, next, &mut out);
        String::from_utf8(out).expect("emitter output is UTF-8")
    }

    #[test]
    fn identical_surfaces_emit_nothing() {
        let prev = Surface::new(2, 1, DEFAULT_STYLE);
        let next = prev.clone();

        assert_eq!(diff_text(&prev, &next), "");
    }

    #[test]
    fn full_repaint_golden_bytes() {
        let mut surface = Surface::new(3, 2, DEFAULT_STYLE);
        surface.set_text(1, 0, "X", ACCENT_STYLE);

        assert_eq!(
            full_text(&surface),
            "\x1b[1;1H\x1b[0;39;49m \x1b[0;37;44mX\x1b[0;39;49m \x1b[2;1H   "
        );
    }

    #[test]
    fn single_cell_change_emits_cup_sgr_symbol() {
        let prev = Surface::new(3, 1, DEFAULT_STYLE);
        let mut next = prev.clone();
        next.set_text(1, 0, "X", ACCENT_STYLE);

        assert_eq!(diff_text(&prev, &next), "\x1b[1;2H\x1b[0;37;44mX");
    }

    #[test]
    fn adjacent_same_style_changes_share_one_cup_and_sgr() {
        let prev = Surface::new(4, 1, DEFAULT_STYLE);
        let mut next = prev.clone();
        next.set_text(1, 0, "XY", ACCENT_STYLE);

        assert_eq!(diff_text(&prev, &next), "\x1b[1;2H\x1b[0;37;44mXY");
    }

    #[test]
    fn style_change_within_run_emits_sgr_without_cup() {
        let prev = Surface::new(2, 1, DEFAULT_STYLE);
        let mut next = prev.clone();
        next.set_text(0, 0, "A", DEFAULT_STYLE);
        next.set_text(1, 0, "B", ACCENT_STYLE);

        assert_eq!(
            diff_text(&prev, &next),
            "\x1b[1;1H\x1b[0;39;49mA\x1b[0;37;44mB"
        );
    }

    #[test]
    fn wide_head_change_rewrites_head_and_skips_continuation() {
        let mut prev = Surface::new(4, 1, DEFAULT_STYLE);
        prev.set_text(1, 0, "界", ACCENT_STYLE);
        let mut next = prev.clone();
        next.set_text(1, 0, "語", ACCENT_STYLE);

        assert_eq!(diff_text(&prev, &next), "\x1b[1;2H\x1b[0;37;44m語");
    }

    #[test]
    fn continuation_change_reemits_head() {
        let mut prev = Surface::new(3, 1, DEFAULT_STYLE);
        prev.set_text(0, 0, "ab", ACCENT_STYLE);
        let mut next = prev.clone();
        next.set_text(0, 0, "界", ACCENT_STYLE);

        assert_eq!(diff_text(&prev, &next), "\x1b[1;1H\x1b[0;37;44m界");
    }

    #[test]
    fn dimension_mismatch_falls_back_to_full_repaint() {
        let prev = Surface::new(1, 1, DEFAULT_STYLE);
        let mut next = Surface::new(2, 1, DEFAULT_STYLE);
        next.set_text(0, 0, "X", ACCENT_STYLE);
        let mut diff = Vec::new();
        let mut full = Vec::new();

        emit_diff(&prev, &next, &mut diff);
        emit_full(&next, &mut full);

        assert_eq!(diff, full);
        assert_eq!(
            String::from_utf8(diff).expect("emitter output is UTF-8"),
            "\x1b[1;1H\x1b[0;37;44mX\x1b[0;39;49m "
        );
    }

    #[test]
    fn color_and_attr_code_table() {
        let all_attrs = StyleAttrs {
            bold: true,
            underline: true,
            reverse: true,
        };
        let cases = [
            (DEFAULT_STYLE, "\x1b[1;1H\x1b[0;39;49mx"),
            (ansi_style(AnsiColor::Black), "\x1b[1;1H\x1b[0;30;40mx"),
            (ansi_style(AnsiColor::Red), "\x1b[1;1H\x1b[0;31;41mx"),
            (ansi_style(AnsiColor::Green), "\x1b[1;1H\x1b[0;32;42mx"),
            (ansi_style(AnsiColor::Yellow), "\x1b[1;1H\x1b[0;33;43mx"),
            (ansi_style(AnsiColor::Blue), "\x1b[1;1H\x1b[0;34;44mx"),
            (ansi_style(AnsiColor::Magenta), "\x1b[1;1H\x1b[0;35;45mx"),
            (ansi_style(AnsiColor::Cyan), "\x1b[1;1H\x1b[0;36;46mx"),
            (ansi_style(AnsiColor::White), "\x1b[1;1H\x1b[0;37;47mx"),
            (
                ansi_style(AnsiColor::BrightBlack),
                "\x1b[1;1H\x1b[0;90;100mx",
            ),
            (ansi_style(AnsiColor::BrightRed), "\x1b[1;1H\x1b[0;91;101mx"),
            (
                ansi_style(AnsiColor::BrightGreen),
                "\x1b[1;1H\x1b[0;92;102mx",
            ),
            (
                ansi_style(AnsiColor::BrightYellow),
                "\x1b[1;1H\x1b[0;93;103mx",
            ),
            (
                ansi_style(AnsiColor::BrightBlue),
                "\x1b[1;1H\x1b[0;94;104mx",
            ),
            (
                ansi_style(AnsiColor::BrightMagenta),
                "\x1b[1;1H\x1b[0;95;105mx",
            ),
            (
                ansi_style(AnsiColor::BrightCyan),
                "\x1b[1;1H\x1b[0;96;106mx",
            ),
            (
                ansi_style(AnsiColor::BrightWhite),
                "\x1b[1;1H\x1b[0;97;107mx",
            ),
            (
                Style::plain(TerminalColor::Indexed(117), TerminalColor::Indexed(23)),
                "\x1b[1;1H\x1b[0;38;5;117;48;5;23mx",
            ),
            (
                Style::new(
                    TerminalColor::Default,
                    TerminalColor::Default,
                    StyleAttrs::BOLD,
                ),
                "\x1b[1;1H\x1b[0;1;39;49mx",
            ),
            (
                Style::new(
                    TerminalColor::Default,
                    TerminalColor::Default,
                    StyleAttrs::UNDERLINE,
                ),
                "\x1b[1;1H\x1b[0;4;39;49mx",
            ),
            (
                Style::new(
                    TerminalColor::Default,
                    TerminalColor::Default,
                    StyleAttrs::REVERSE,
                ),
                "\x1b[1;1H\x1b[0;7;39;49mx",
            ),
            (
                Style::new(TerminalColor::Default, TerminalColor::Default, all_attrs),
                "\x1b[1;1H\x1b[0;1;4;7;39;49mx",
            ),
        ];

        for (style, expected) in cases {
            let prev = Surface::new(1, 1, DEFAULT_STYLE);
            let mut next = prev.clone();
            next.set_text(0, 0, "x", style);

            assert_eq!(diff_text(&prev, &next), expected);
        }
    }

    #[test]
    fn abutting_runs_skip_cup() {
        let mut prev = Surface::new(4, 1, DEFAULT_STYLE);
        prev.set_text(0, 0, "界", ACCENT_STYLE);
        let mut next = prev.clone();
        next.set_text(0, 0, "語", ACCENT_STYLE);
        next.set_text(2, 0, "X", ACCENT_STYLE);

        assert_eq!(diff_text(&prev, &next), "\x1b[1;1H\x1b[0;37;44m語X");
    }

    #[test]
    fn zero_sized_surfaces_emit_nothing() {
        let zero_width = Surface::new(0, 2, DEFAULT_STYLE);
        let zero_height = Surface::new(2, 0, DEFAULT_STYLE);

        assert_eq!(full_text(&zero_width), "");
        assert_eq!(full_text(&zero_height), "");
        assert_eq!(diff_text(&zero_width, &zero_width), "");
        assert_eq!(diff_text(&zero_height, &zero_height), "");
    }

    fn ansi_style(color: AnsiColor) -> Style {
        Style::plain(TerminalColor::Ansi(color), TerminalColor::Ansi(color))
    }
}
