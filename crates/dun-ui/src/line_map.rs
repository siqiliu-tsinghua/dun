use dun_core::{Position, TextBuffer};

use crate::EditorTextDisplay;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoldRange {
    pub start_line: usize,
    pub end_line_exclusive: usize,
}

impl FoldRange {
    pub const fn new(start_line: usize, end_line_exclusive: usize) -> Self {
        Self {
            start_line,
            end_line_exclusive,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoldSet {
    ranges: Vec<FoldRange>,
}

impl FoldSet {
    pub const fn empty() -> Self {
        Self { ranges: Vec::new() }
    }

    pub fn new(ranges: Vec<FoldRange>) -> Option<Self> {
        let valid = ranges.iter().enumerate().all(|(index, range)| {
            range.start_line < range.end_line_exclusive
                && (index == 0 || ranges[index - 1].end_line_exclusive <= range.start_line)
        });
        valid.then_some(Self { ranges })
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisibleLine {
    Source { line: usize },
    Fold { range: FoldRange },
}

#[derive(Clone, Copy, Debug)]
pub struct EditorLineDisplay<'a> {
    line_count: usize,
    folds: &'a FoldSet,
}

impl<'a> EditorLineDisplay<'a> {
    pub const fn new(line_count: usize, folds: &'a FoldSet) -> Self {
        Self { line_count, folds }
    }

    pub fn visible_row_count(self) -> usize {
        if self.folds.is_empty() {
            return self.line_count;
        }

        self.folds
            .ranges
            .iter()
            .filter_map(|range| self.clipped_range(*range))
            .fold(self.line_count, |count, range| {
                count.saturating_sub(
                    range
                        .end_line_exclusive
                        .saturating_sub(range.start_line)
                        .saturating_sub(1),
                )
            })
    }

    pub fn placement_for_source_line(self, line: usize) -> Option<usize> {
        if line >= self.line_count {
            return None;
        }
        if self.folds.is_empty() {
            return Some(line);
        }

        let mut hidden_before = 0usize;
        for range in self
            .folds
            .ranges
            .iter()
            .filter_map(|range| self.clipped_range(*range))
        {
            if line < range.start_line {
                break;
            }
            if line < range.end_line_exclusive {
                return Some(range.start_line.saturating_sub(hidden_before));
            }
            hidden_before = hidden_before.saturating_add(
                range
                    .end_line_exclusive
                    .saturating_sub(range.start_line)
                    .saturating_sub(1),
            );
        }

        Some(line.saturating_sub(hidden_before))
    }

    pub fn item_for_visible_row(self, row: usize) -> Option<VisibleLine> {
        if row >= self.visible_row_count() {
            return None;
        }
        if self.folds.is_empty() {
            return Some(VisibleLine::Source { line: row });
        }

        let mut source_line = 0usize;
        let mut visible_row = 0usize;
        for range in self
            .folds
            .ranges
            .iter()
            .filter_map(|range| self.clipped_range(*range))
        {
            let source_rows = range.start_line.saturating_sub(source_line);
            if row < visible_row.saturating_add(source_rows) {
                return Some(VisibleLine::Source {
                    line: source_line.saturating_add(row.saturating_sub(visible_row)),
                });
            }
            visible_row = visible_row.saturating_add(source_rows);
            if row == visible_row {
                return Some(VisibleLine::Fold { range });
            }
            visible_row = visible_row.saturating_add(1);
            source_line = range.end_line_exclusive;
        }

        Some(VisibleLine::Source {
            line: source_line.saturating_add(row.saturating_sub(visible_row)),
        })
    }

    pub fn source_anchor_for_visible_row(self, row: usize) -> Option<usize> {
        match self.item_for_visible_row(row)? {
            VisibleLine::Source { line } => Some(line),
            VisibleLine::Fold { range } => Some(range.start_line),
        }
    }

    pub fn next_visible_anchor(self, line: usize) -> Option<usize> {
        let row = self.placement_for_source_line(line)?;
        self.source_anchor_for_visible_row(row.saturating_add(1))
    }

    pub fn previous_visible_anchor(self, line: usize) -> Option<usize> {
        let row = self.placement_for_source_line(line)?;
        self.source_anchor_for_visible_row(row.checked_sub(1)?)
    }

    pub fn iter_from_visible_row(self, row: usize) -> VisibleLineIter<'a> {
        VisibleLineIter {
            display: self,
            next_row: row,
        }
    }

    fn clipped_range(self, range: FoldRange) -> Option<FoldRange> {
        let range = FoldRange {
            start_line: range.start_line.min(self.line_count),
            end_line_exclusive: range.end_line_exclusive.min(self.line_count),
        };
        (range.start_line < range.end_line_exclusive).then_some(range)
    }
}

#[derive(Clone, Debug)]
pub struct VisibleLineIter<'a> {
    display: EditorLineDisplay<'a>,
    next_row: usize,
}

impl Iterator for VisibleLineIter<'_> {
    type Item = VisibleLine;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.display.item_for_visible_row(self.next_row)?;
        self.next_row = self.next_row.saturating_add(1);
        Some(item)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewportTop {
    pub anchor_line: usize,
    pub wrapped_row: usize,
}

impl ViewportTop {
    pub const fn new(anchor_line: usize, wrapped_row: usize) -> Self {
        Self {
            anchor_line,
            wrapped_row,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EditorVisualRows<'buffer, 'folds> {
    buffer: &'buffer TextBuffer,
    line_map: EditorLineDisplay<'folds>,
    text_display: EditorTextDisplay,
    body_width: usize,
}

impl<'buffer, 'folds> EditorVisualRows<'buffer, 'folds> {
    pub fn new(
        buffer: &'buffer TextBuffer,
        line_map: EditorLineDisplay<'folds>,
        text_display: EditorTextDisplay,
        body_width: usize,
    ) -> Self {
        Self {
            buffer,
            line_map,
            text_display,
            body_width: body_width.max(1),
        }
    }

    pub fn total_rows(self) -> usize {
        self.line_map
            .iter_from_visible_row(0)
            .fold(0usize, |rows, item| {
                rows.saturating_add(self.rows_for_item(item))
            })
    }

    pub fn global_row_for_position(self, position: Position) -> usize {
        let Some(visible_row) = self.line_map.placement_for_source_line(position.line) else {
            return self.total_rows();
        };
        let base = self.rows_before_visible_row(visible_row);
        match self.line_map.item_for_visible_row(visible_row) {
            Some(VisibleLine::Source { line }) if line == position.line => {
                let row = self
                    .buffer
                    .line(line)
                    .and_then(|source| {
                        self.text_display.wrapped_row_column_for_source_byte(
                            source,
                            position.column,
                            self.body_width,
                        )
                    })
                    .map(|(row, _)| row)
                    .unwrap_or(0);
                base.saturating_add(row)
            }
            Some(VisibleLine::Fold { .. }) | Some(VisibleLine::Source { .. }) | None => base,
        }
    }

    pub fn global_row_for_top(self, top: ViewportTop) -> usize {
        let Some(visible_row) = self.line_map.placement_for_source_line(top.anchor_line) else {
            return self.total_rows().saturating_sub(1);
        };
        let base = self.rows_before_visible_row(visible_row);
        let rows = self
            .line_map
            .item_for_visible_row(visible_row)
            .map(|item| self.rows_for_item(item))
            .unwrap_or(1);
        base.saturating_add(top.wrapped_row.min(rows.saturating_sub(1)))
    }

    pub fn top_for_global_row(self, row: usize) -> ViewportTop {
        let total_rows = self.total_rows();
        if total_rows == 0 {
            return ViewportTop::default();
        }

        let mut remaining = row.min(total_rows.saturating_sub(1));
        for item in self.line_map.iter_from_visible_row(0) {
            let rows = self.rows_for_item(item);
            if remaining < rows {
                return match item {
                    VisibleLine::Source { line } => ViewportTop::new(line, remaining),
                    VisibleLine::Fold { range } => ViewportTop::new(range.start_line, 0),
                };
            }
            remaining = remaining.saturating_sub(rows);
        }

        ViewportTop::default()
    }

    pub fn position_for_global_row_column(self, row: usize, column: usize) -> Position {
        let mut remaining = row;
        for item in self.line_map.iter_from_visible_row(0) {
            let rows = self.rows_for_item(item);
            if remaining < rows {
                return match item {
                    VisibleLine::Source { line } => {
                        let source = self.buffer.line(line).unwrap_or_default();
                        Position::new(
                            line,
                            self.text_display.source_byte_for_wrapped_row_column(
                                source,
                                remaining,
                                column,
                                self.body_width,
                            ),
                        )
                    }
                    VisibleLine::Fold { range } => Position::new(range.start_line, 0),
                };
            }
            remaining = remaining.saturating_sub(rows);
        }

        let last_line = self.buffer.line_count().saturating_sub(1);
        Position::new(
            last_line,
            self.buffer.line(last_line).map(str::len).unwrap_or(0),
        )
    }

    fn rows_before_visible_row(self, visible_row: usize) -> usize {
        self.line_map
            .iter_from_visible_row(0)
            .take(visible_row)
            .fold(0usize, |rows, item| {
                rows.saturating_add(self.rows_for_item(item))
            })
    }

    fn rows_for_item(self, item: VisibleLine) -> usize {
        match item {
            VisibleLine::Source { line } => self
                .buffer
                .line(line)
                .map(|source| self.text_display.wrapped_row_count(source, self.body_width))
                .unwrap_or(1),
            VisibleLine::Fold { .. } => 1,
        }
    }
}
