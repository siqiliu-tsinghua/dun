use crate::*;

pub(crate) fn buffer_end_position(buffer: &TextBuffer) -> Position {
    let last_line = buffer.line_count().saturating_sub(1);
    let last_column = buffer.line(last_line).map(str::len).unwrap_or(0);
    Position::new(last_line, last_column)
}

pub(crate) fn clamp_to_char_boundary(line: &str, column: usize) -> usize {
    let mut column = column.min(line.len());
    while !line.is_char_boundary(column) {
        column -= 1;
    }
    column
}

pub(crate) fn clamp_to_display_column(line: &str, target: usize, mode: AmbiguousWidth) -> usize {
    let mut display = 0usize;
    for (index, ch) in line.char_indices() {
        let width = char_width(ch, mode).unwrap_or(0);
        if display.saturating_add(width) > target {
            return index;
        }
        display = display.saturating_add(width);
    }
    line.len()
}

pub(crate) fn display_width_for_editor_char(ch: char, mode: AmbiguousWidth) -> usize {
    char_width(ch, mode).unwrap_or(0).max(1)
}

pub(crate) fn advance_wrapped_column(
    row: &mut usize,
    column: &mut usize,
    width: usize,
    body_width: usize,
) {
    let width = width.max(1);
    let body_width = body_width.max(1);
    if *column > 0 && (*column).saturating_add(width) > body_width {
        *row = (*row).saturating_add(1);
        *column = 0;
    }
    *column = (*column).saturating_add(width);
}

pub(crate) fn byte_column_for_wrapped_row_start(
    line: &str,
    target_row: usize,
    body_width: usize,
    mode: AmbiguousWidth,
) -> usize {
    if target_row == 0 {
        return 0;
    }

    let mut row = 0usize;
    let mut column = 0usize;
    for (index, ch) in line.char_indices() {
        let width = display_width_for_editor_char(ch, mode);
        if column > 0 && column.saturating_add(width) > body_width.max(1) {
            row = row.saturating_add(1);
            column = 0;
            if row == target_row {
                return index;
            }
        }
        column = column.saturating_add(width);
    }

    line.len()
}

pub(crate) fn byte_column_for_wrapped_row_column(
    line: &str,
    target_row: usize,
    target_column: usize,
    body_width: usize,
    mode: AmbiguousWidth,
) -> usize {
    let body_width = body_width.max(1);
    let row_start = byte_column_for_wrapped_row_start(line, target_row, body_width, mode);
    if target_column == 0 {
        return row_start;
    }

    let mut visual_column = 0usize;
    for (offset, ch) in line[row_start..].char_indices() {
        let index = row_start.saturating_add(offset);
        let width = display_width_for_editor_char(ch, mode);
        if visual_column > 0 && visual_column.saturating_add(width) > body_width {
            return index;
        }
        if visual_column.saturating_add(width) > target_column {
            return index;
        }
        visual_column = visual_column.saturating_add(width);
    }

    line.len()
}
