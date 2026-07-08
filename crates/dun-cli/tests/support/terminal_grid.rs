#![allow(dead_code)]

use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerminalCursor {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalGrid {
    pub width: u16,
    pub height: u16,
    pub cursor: Option<TerminalCursor>,
    cells: Vec<TerminalCell>,
}

impl TerminalGrid {
    pub fn cell(&self, row: u16, col: u16) -> Option<&TerminalCell> {
        if row >= self.height || col >= self.width {
            return None;
        }
        self.cells
            .get(row as usize * self.width as usize + col as usize)
    }

    pub fn text_at(&self, row: u16, col: u16, width: u16) -> String {
        (col..col.saturating_add(width).min(self.width))
            .filter_map(|x| self.cell(row, x).map(|cell| cell.ch))
            .collect()
    }

    pub fn line_text(&self, row: u16) -> String {
        self.text_at(row, 0, self.width)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalCell {
    pub ch: char,
    pub style: TerminalStyle,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: TerminalStyle::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerminalStyle {
    pub fg: TerminalColor,
    pub bg: TerminalColor,
    pub bold: bool,
    pub underline: bool,
    pub reverse: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerminalColor {
    #[default]
    Default,
    Ansi(u8),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

pub fn parse_terminal_grid(
    input: &str,
    cols: u16,
    rows: u16,
    cursor: Option<TerminalCursor>,
) -> TerminalGrid {
    let mut parser = GridParser::new(cols, rows, cursor);
    parser.parse(input);
    parser.finish()
}

struct GridParser {
    width: usize,
    height: usize,
    external_cursor: Option<TerminalCursor>,
    cells: Vec<TerminalCell>,
    style: TerminalStyle,
    row: usize,
    col: usize,
    saved_cursor: Option<(usize, usize)>,
}

impl GridParser {
    fn new(cols: u16, rows: u16, external_cursor: Option<TerminalCursor>) -> Self {
        let width = cols as usize;
        let height = rows as usize;
        Self {
            width,
            height,
            external_cursor,
            cells: vec![TerminalCell::default(); width.saturating_mul(height)],
            style: TerminalStyle::default(),
            row: 0,
            col: 0,
            saved_cursor: None,
        }
    }

    fn parse(&mut self, input: &str) {
        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\x1b' => self.parse_escape(&mut chars),
                '\x08' => self.col = self.col.saturating_sub(1),
                '\t' => self.advance_to_next_tab_stop(),
                '\r' => self.col = 0,
                '\n' => {
                    self.row = self.row.saturating_add(1);
                    self.col = 0;
                    if self.row >= self.height {
                        break;
                    }
                }
                '\x00'..='\x1f' | '\x7f' => {}
                _ => self.put_char(ch),
            }
        }
    }

    fn finish(self) -> TerminalGrid {
        let cursor = self.external_cursor.or_else(|| {
            if self.width == 0 || self.height == 0 {
                None
            } else {
                Some(TerminalCursor {
                    x: self.col.min(self.width.saturating_sub(1)) as u16,
                    y: self.row.min(self.height.saturating_sub(1)) as u16,
                })
            }
        });

        TerminalGrid {
            width: self.width as u16,
            height: self.height as u16,
            cursor,
            cells: self.cells,
        }
    }

    fn parse_escape(&mut self, chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
        match chars.next() {
            Some('[') => self.parse_csi(chars),
            Some(']') => skip_osc(chars),
            Some('7') => self.saved_cursor = Some((self.row, self.col)),
            Some('8') => self.restore_cursor(),
            Some('(' | ')' | '*' | '+') => {
                let _ = chars.next();
            }
            Some(_) | None => {}
        }
    }

    fn parse_csi(&mut self, chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
        let mut sequence = String::new();
        for next in chars.by_ref() {
            if ('@'..='~').contains(&next) {
                self.apply_csi(&sequence, next);
                break;
            }
            sequence.push(next);
        }
    }

    fn apply_csi(&mut self, sequence: &str, final_byte: char) {
        match final_byte {
            'm' => apply_sgr(sequence, &mut self.style),
            'H' | 'f' => {
                let params = parse_numeric_params(sequence);
                let row = param_or(&params, 0, 1).saturating_sub(1);
                let col = param_or(&params, 1, 1).saturating_sub(1);
                self.move_to(row, col);
            }
            'A' => self.move_by(
                -(param_or(&parse_numeric_params(sequence), 0, 1) as isize),
                0,
            ),
            'B' => self.move_by(param_or(&parse_numeric_params(sequence), 0, 1) as isize, 0),
            'C' => self.move_by(0, param_or(&parse_numeric_params(sequence), 0, 1) as isize),
            'D' => self.move_by(
                0,
                -(param_or(&parse_numeric_params(sequence), 0, 1) as isize),
            ),
            'E' => {
                self.move_by(param_or(&parse_numeric_params(sequence), 0, 1) as isize, 0);
                self.col = 0;
            }
            'F' => {
                self.move_by(
                    -(param_or(&parse_numeric_params(sequence), 0, 1) as isize),
                    0,
                );
                self.col = 0;
            }
            'G' | '`' => {
                let col = param_or(&parse_numeric_params(sequence), 0, 1).saturating_sub(1);
                self.move_to(self.row, col);
            }
            'd' => {
                let row = param_or(&parse_numeric_params(sequence), 0, 1).saturating_sub(1);
                self.move_to(row, self.col);
            }
            'J' => self.erase_display(param_or(&parse_numeric_params(sequence), 0, 0)),
            'K' => self.erase_line(param_or(&parse_numeric_params(sequence), 0, 0)),
            's' => self.saved_cursor = Some((self.row, self.col)),
            'u' => self.restore_cursor(),
            _ => {}
        }
    }

    fn put_char(&mut self, ch: char) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        if self.col >= self.width {
            self.row = self.row.saturating_add(1);
            self.col = 0;
        }
        if self.row >= self.height {
            return;
        }

        let index = self.row * self.width + self.col;
        self.cells[index] = TerminalCell {
            ch,
            style: self.style,
        };
        let display_width = UnicodeWidthChar::width(ch).unwrap_or(1).max(1);
        for extra in 1..display_width {
            if self.col + extra < self.width {
                self.cells[self.row * self.width + self.col + extra] = TerminalCell {
                    ch: ' ',
                    style: self.style,
                };
            }
        }
        self.col = self.col.saturating_add(display_width);
    }

    fn advance_to_next_tab_stop(&mut self) {
        let next = self.col.saturating_add(8 - (self.col % 8));
        self.col = next.min(self.width);
    }

    fn move_to(&mut self, row: usize, col: usize) {
        if self.width == 0 || self.height == 0 {
            self.row = 0;
            self.col = 0;
            return;
        }
        self.row = row.min(self.height - 1);
        self.col = col.min(self.width - 1);
    }

    fn move_by(&mut self, rows: isize, cols: isize) {
        let row = self.row.saturating_add_signed(rows);
        let col = self.col.saturating_add_signed(cols);
        self.move_to(row, col);
    }

    fn restore_cursor(&mut self) {
        if let Some((row, col)) = self.saved_cursor {
            self.move_to(row, col);
        }
    }

    fn erase_display(&mut self, mode: usize) {
        match mode {
            0 => {
                self.erase_line_range(self.row, self.col, self.width);
                for row in self.row.saturating_add(1)..self.height {
                    self.erase_line_range(row, 0, self.width);
                }
            }
            1 => {
                for row in 0..self.row {
                    self.erase_line_range(row, 0, self.width);
                }
                self.erase_line_range(self.row, 0, self.col.saturating_add(1));
            }
            2 | 3 => {
                for row in 0..self.height {
                    self.erase_line_range(row, 0, self.width);
                }
            }
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: usize) {
        match mode {
            0 => self.erase_line_range(self.row, self.col, self.width),
            1 => self.erase_line_range(self.row, 0, self.col.saturating_add(1)),
            2 => self.erase_line_range(self.row, 0, self.width),
            _ => {}
        }
    }

    fn erase_line_range(&mut self, row: usize, start: usize, end: usize) {
        if row >= self.height {
            return;
        }
        for col in start.min(self.width)..end.min(self.width) {
            self.cells[row * self.width + col] = TerminalCell::default();
        }
    }
}

fn skip_osc(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    let mut saw_escape = false;
    for ch in chars.by_ref() {
        if saw_escape {
            if ch == '\\' {
                break;
            }
            saw_escape = ch == '\x1b';
            continue;
        }
        match ch {
            '\x07' => break,
            '\x1b' => saw_escape = true,
            _ => {}
        }
    }
}

fn apply_sgr(sequence: &str, style: &mut TerminalStyle) {
    let codes = parse_sgr_codes(sequence);
    if codes.is_empty() {
        *style = TerminalStyle::default();
        return;
    }

    let mut index = 0usize;
    while index < codes.len() {
        match codes[index] {
            0 => *style = TerminalStyle::default(),
            1 => style.bold = true,
            4 => style.underline = true,
            7 => style.reverse = true,
            22 => style.bold = false,
            24 => style.underline = false,
            27 => style.reverse = false,
            30..=37 => style.fg = TerminalColor::Ansi((codes[index] - 30) as u8),
            39 => style.fg = TerminalColor::Default,
            40..=47 => style.bg = TerminalColor::Ansi((codes[index] - 40) as u8),
            49 => style.bg = TerminalColor::Default,
            90..=97 => style.fg = TerminalColor::Ansi((codes[index] - 90 + 8) as u8),
            100..=107 => style.bg = TerminalColor::Ansi((codes[index] - 100 + 8) as u8),
            38 | 48 => {
                let target_is_fg = codes[index] == 38;
                if let Some((color, consumed)) = parse_extended_color(&codes[index + 1..]) {
                    if target_is_fg {
                        style.fg = color;
                    } else {
                        style.bg = color;
                    }
                    index = index.saturating_add(consumed);
                }
            }
            _ => {}
        }
        index = index.saturating_add(1);
    }
}

fn parse_sgr_codes(sequence: &str) -> Vec<u16> {
    if sequence.is_empty() {
        return vec![0];
    }

    sequence
        .split([';', ':'])
        .map(|part| {
            if part.is_empty() {
                0
            } else {
                part.parse::<u16>().unwrap_or(0)
            }
        })
        .collect()
}

fn parse_numeric_params(sequence: &str) -> Vec<usize> {
    sequence
        .trim_start_matches(['?', '>', '=', '!'])
        .split(';')
        .map(|part| part.parse::<usize>().unwrap_or(0))
        .collect()
}

fn param_or(params: &[usize], index: usize, default: usize) -> usize {
    match params.get(index).copied() {
        Some(0) | None => default,
        Some(value) => value,
    }
}

fn parse_extended_color(codes: &[u16]) -> Option<(TerminalColor, usize)> {
    match codes {
        [5, index, ..] => Some((
            TerminalColor::Indexed((*index).min(u8::MAX as u16) as u8),
            2,
        )),
        [2, r, g, b, ..] => Some((
            TerminalColor::Rgb(
                (*r).min(u8::MAX as u16) as u8,
                (*g).min(u8::MAX as u16) as u8,
                (*b).min(u8::MAX as u16) as u8,
            ),
            4,
        )),
        _ => None,
    }
}
