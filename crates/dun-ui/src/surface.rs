use dun_term::AmbiguousWidth;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceCell {
    pub(crate) symbol: String,
    pub(crate) style: dun_term::Style,
    pub(crate) wide_continuation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Surface {
    width: u16,
    height: u16,
    cells: Vec<SurfaceCell>,
    ambiguous_width: AmbiguousWidth,
}

impl Surface {
    pub(crate) fn new(width: u16, height: u16, fill_style: dun_term::Style) -> Self {
        let cell = SurfaceCell {
            symbol: String::from(" "),
            style: fill_style,
            wide_continuation: false,
        };
        let cell_count = usize::from(width) * usize::from(height);

        Self {
            width,
            height,
            cells: vec![cell; cell_count],
            ambiguous_width: AmbiguousWidth::Narrow,
        }
    }

    /// Set the ambiguous-width reading used when placing glyphs (so a wide
    /// terminal budgets box-drawing glyphs as 2 columns). Production render and
    /// snapshot paths opt in; tests keep the default Narrow.
    pub(crate) fn with_ambiguous_width(mut self, ambiguous_width: AmbiguousWidth) -> Self {
        self.ambiguous_width = ambiguous_width;
        self
    }

    pub(crate) fn width(&self) -> u16 {
        self.width
    }

    pub(crate) fn height(&self) -> u16 {
        self.height
    }

    pub(crate) fn glyph_width(&self, ch: char) -> usize {
        dun_term::char_width(ch, self.ambiguous_width).unwrap_or(1)
    }

    pub(crate) fn cell(&self, x: u16, y: u16) -> Option<&SurfaceCell> {
        let index = self.index(x, y)?;
        self.cells.get(index)
    }

    pub(crate) fn set_text(&mut self, x: u16, y: u16, text: &str, style: dun_term::Style) -> u16 {
        if x >= self.width || y >= self.height {
            return 0;
        }

        let mut column = x;
        let mut previous_cell: Option<usize> = None;

        for ch in text.chars() {
            debug_assert!(!ch.is_control());
            if ch.is_control() {
                continue;
            }

            let char_width = dun_term::char_width(ch, self.ambiguous_width).unwrap_or(0);
            if char_width == 0 {
                if let Some(index) = previous_cell {
                    if let Some(cell) = self.cells.get_mut(index) {
                        cell.symbol.push(ch);
                    }
                }
                continue;
            }

            debug_assert!(char_width <= 2);
            let char_width = match char_width {
                1 => 1,
                2 => 2,
                _ => continue,
            };
            if self.width - column < char_width {
                break;
            }

            self.clear_wide_at(column, y);
            if char_width == 2 {
                self.clear_wide_at(column + 1, y);
            }

            let Some(index) = self.index(column, y) else {
                break;
            };
            if let Some(cell) = self.cells.get_mut(index) {
                cell.symbol.clear();
                cell.symbol.push(ch);
                cell.style = style;
                cell.wide_continuation = false;
            }

            if char_width == 2 {
                if let Some(cell) = self.cells.get_mut(index + 1) {
                    cell.symbol.clear();
                    cell.style = style;
                    cell.wide_continuation = true;
                }
            }

            previous_cell = Some(index);
            column += char_width;
        }

        column - x
    }

    pub(crate) fn fill_rect(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        symbol: char,
        style: dun_term::Style,
    ) {
        let symbol_width = dun_term::char_width(symbol, self.ambiguous_width);
        debug_assert_eq!(symbol_width, Some(1));
        if symbol_width != Some(1) {
            return;
        }

        let end_x = x.saturating_add(w).min(self.width);
        let end_y = y.saturating_add(h).min(self.height);
        for row in y..end_y {
            for column in x..end_x {
                self.clear_wide_at(column, row);
                if let Some(index) = self.index(column, row) {
                    if let Some(cell) = self.cells.get_mut(index) {
                        cell.symbol.clear();
                        cell.symbol.push(symbol);
                        cell.style = style;
                        cell.wide_continuation = false;
                    }
                }
            }
        }
    }

    /// Restyle a single cell in place, preserving its glyph and its
    /// wide-glyph role. Out-of-bounds coordinates are a no-op.
    pub(crate) fn set_style(&mut self, x: u16, y: u16, style: dun_term::Style) {
        if let Some(index) = self.index(x, y) {
            if let Some(cell) = self.cells.get_mut(index) {
                cell.style = style;
            }
        }
    }

    /// Restyle a horizontal run of `width` cells starting at (x, y),
    /// preserving glyphs and clipping at the right edge. The overlay passes
    /// (selection, current line, search matches, syntax highlights) recolor
    /// already-painted body text without disturbing it. A run that partially
    /// covers a wide glyph restyles only the covered columns, matching the
    /// per-cell model this replaces.
    pub(crate) fn style_run(&mut self, x: u16, y: u16, width: u16, style: dun_term::Style) {
        if y >= self.height {
            return;
        }
        let end = x.saturating_add(width).min(self.width);
        for column in x..end {
            self.set_style(column, y, style);
        }
    }

    #[cfg(test)]
    pub(crate) fn row_text(&self, y: u16) -> String {
        if y >= self.height {
            return String::new();
        }

        let start = usize::from(y) * usize::from(self.width);
        let end = start + usize::from(self.width);
        let mut text = String::new();
        if let Some(row) = self.cells.get(start..end) {
            for cell in row {
                if !cell.wide_continuation {
                    text.push_str(&cell.symbol);
                }
            }
        }
        text
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }

        Some(usize::from(y) * usize::from(self.width) + usize::from(x))
    }

    fn clear_wide_at(&mut self, x: u16, y: u16) {
        let Some(index) = self.index(x, y) else {
            return;
        };
        let is_continuation = self
            .cells
            .get(index)
            .is_some_and(|cell| cell.wide_continuation);
        let has_continuation = x + 1 < self.width
            && self
                .cells
                .get(index + 1)
                .is_some_and(|cell| cell.wide_continuation);

        Self::blank_cell(self.cells.get_mut(index));
        if is_continuation && x > 0 {
            Self::blank_cell(self.cells.get_mut(index - 1));
        } else if has_continuation {
            Self::blank_cell(self.cells.get_mut(index + 1));
        }
    }

    fn blank_cell(cell: Option<&mut SurfaceCell>) {
        if let Some(cell) = cell {
            cell.symbol.clear();
            cell.symbol.push(' ');
            cell.wide_continuation = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use dun_term::{AnsiColor, Style, TerminalColor};

    use super::{Surface, SurfaceCell};

    const FILL_STYLE: Style = Style::plain(TerminalColor::Default, TerminalColor::Default);
    const TEXT_STYLE: Style = Style::plain(
        TerminalColor::Ansi(AnsiColor::White),
        TerminalColor::Ansi(AnsiColor::Blue),
    );

    #[test]
    fn new_fills_cells_and_reports_dimensions() {
        let surface = Surface::new(3, 2, FILL_STYLE);

        assert_eq!(surface.width(), 3);
        assert_eq!(surface.height(), 2);
        assert!(surface.cells.iter().all(|cell| {
            cell.symbol == " " && cell.style == FILL_STYLE && !cell.wide_continuation
        }));
        assert_eq!(
            surface.cell(2, 1),
            Some(&SurfaceCell {
                symbol: String::from(" "),
                style: FILL_STYLE,
                wide_continuation: false,
            })
        );
    }

    #[test]
    fn set_text_writes_ascii_and_returns_display_width() {
        let mut surface = Surface::new(5, 1, FILL_STYLE);

        assert_eq!(surface.set_text(1, 0, "abc", TEXT_STYLE), 3);
        assert_eq!(surface.row_text(0), " abc ");
        assert_eq!(surface.cell(2, 0).map(|cell| cell.style), Some(TEXT_STYLE));
    }

    #[test]
    fn set_text_clips_wide_character_at_right_edge() {
        let mut surface = Surface::new(3, 1, FILL_STYLE);

        assert_eq!(surface.set_text(1, 0, "a界", TEXT_STYLE), 1);
        assert_eq!(surface.row_text(0), " a ");
        assert_eq!(surface.cell(2, 0).map(|cell| cell.style), Some(FILL_STYLE));
    }

    #[test]
    fn set_text_marks_wide_character_continuation() {
        let mut surface = Surface::new(3, 1, FILL_STYLE);

        assert_eq!(surface.set_text(0, 0, "界", TEXT_STYLE), 2);
        assert_eq!(
            surface.cell(0, 0).map(|cell| cell.symbol.as_str()),
            Some("界")
        );
        assert_eq!(
            surface.cell(1, 0),
            Some(&SurfaceCell {
                symbol: String::new(),
                style: TEXT_STYLE,
                wide_continuation: true,
            })
        );
        assert_eq!(surface.row_text(0), "界 ");
    }

    #[test]
    fn overwriting_wide_head_blanks_continuation() {
        let mut surface = Surface::new(4, 1, FILL_STYLE);
        assert_eq!(surface.set_text(1, 0, "界", TEXT_STYLE), 2);

        assert_eq!(surface.set_text(1, 0, "x", FILL_STYLE), 1);

        assert_eq!(surface.row_text(0), " x  ");
        assert_eq!(
            surface
                .cell(2, 0)
                .map(|cell| (cell.symbol.as_str(), cell.wide_continuation)),
            Some((" ", false))
        );
    }

    #[test]
    fn overwriting_wide_continuation_blanks_head() {
        let mut surface = Surface::new(4, 1, FILL_STYLE);
        assert_eq!(surface.set_text(1, 0, "界", TEXT_STYLE), 2);

        assert_eq!(surface.set_text(2, 0, "x", FILL_STYLE), 1);

        assert_eq!(surface.row_text(0), "  x ");
        assert_eq!(
            surface
                .cell(1, 0)
                .map(|cell| (cell.symbol.as_str(), cell.wide_continuation)),
            Some((" ", false))
        );
    }

    #[test]
    fn zero_width_character_appends_to_previous_written_cell() {
        let mut surface = Surface::new(3, 1, FILL_STYLE);

        assert_eq!(surface.set_text(0, 0, "e\u{301}", TEXT_STYLE), 1);
        assert_eq!(
            surface.cell(0, 0).map(|cell| cell.symbol.as_str()),
            Some("e\u{301}")
        );

        let before = surface.clone();
        assert_eq!(surface.set_text(2, 0, "\u{301}", TEXT_STYLE), 0);
        assert_eq!(surface, before);
    }

    #[test]
    fn out_of_bounds_access_and_writes_do_nothing() {
        let mut surface = Surface::new(2, 1, FILL_STYLE);

        assert_eq!(surface.set_text(2, 0, "x", TEXT_STYLE), 0);
        assert_eq!(surface.set_text(0, 1, "x", TEXT_STYLE), 0);
        assert_eq!(surface.cell(2, 0), None);
        assert_eq!(surface.cell(0, 1), None);
        assert_eq!(surface.row_text(0), "  ");
    }

    #[test]
    fn set_style_preserves_glyph_and_continuation() {
        let mut surface = Surface::new(3, 1, FILL_STYLE);
        assert_eq!(surface.set_text(0, 0, "界", TEXT_STYLE), 2);

        surface.set_style(0, 0, FILL_STYLE);
        surface.set_style(1, 0, FILL_STYLE);

        assert_eq!(
            surface.cell(0, 0),
            Some(&SurfaceCell {
                symbol: String::from("界"),
                style: FILL_STYLE,
                wide_continuation: false,
            })
        );
        assert_eq!(
            surface.cell(1, 0),
            Some(&SurfaceCell {
                symbol: String::new(),
                style: FILL_STYLE,
                wide_continuation: true,
            })
        );
        assert_eq!(surface.row_text(0), "界 ");
    }

    #[test]
    fn style_run_restyles_span_preserving_text() {
        let mut surface = Surface::new(5, 1, FILL_STYLE);
        assert_eq!(surface.set_text(0, 0, "abcde", FILL_STYLE), 5);

        surface.style_run(1, 0, 3, TEXT_STYLE);

        assert_eq!(surface.row_text(0), "abcde");
        assert_eq!(surface.cell(0, 0).map(|cell| cell.style), Some(FILL_STYLE));
        assert_eq!(surface.cell(1, 0).map(|cell| cell.style), Some(TEXT_STYLE));
        assert_eq!(surface.cell(3, 0).map(|cell| cell.style), Some(TEXT_STYLE));
        assert_eq!(surface.cell(4, 0).map(|cell| cell.style), Some(FILL_STYLE));
    }

    #[test]
    fn style_run_clips_at_right_edge_and_ignores_out_of_bounds() {
        let mut surface = Surface::new(3, 1, FILL_STYLE);
        assert_eq!(surface.set_text(0, 0, "abc", FILL_STYLE), 3);

        surface.style_run(2, 0, 10, TEXT_STYLE);
        surface.style_run(0, 5, 3, TEXT_STYLE);
        surface.set_style(9, 0, TEXT_STYLE);

        assert_eq!(surface.cell(0, 0).map(|cell| cell.style), Some(FILL_STYLE));
        assert_eq!(surface.cell(1, 0).map(|cell| cell.style), Some(FILL_STYLE));
        assert_eq!(surface.cell(2, 0).map(|cell| cell.style), Some(TEXT_STYLE));
        assert_eq!(surface.row_text(0), "abc");
    }

    #[test]
    fn fill_rect_clips_at_surface_bounds() {
        let mut surface = Surface::new(3, 2, FILL_STYLE);

        surface.fill_rect(2, 1, 3, 2, '#', TEXT_STYLE);

        assert_eq!(surface.row_text(0), "   ");
        assert_eq!(surface.row_text(1), "  #");
        assert_eq!(surface.cell(2, 1).map(|cell| cell.style), Some(TEXT_STYLE));
    }
}
