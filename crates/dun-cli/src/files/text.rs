use crate::*;
use dun_ui::EditorTextDisplay;

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

pub(crate) fn clamp_to_display_column(
    line: &str,
    target: usize,
    display: EditorTextDisplay,
) -> usize {
    display.display_column_to_source_byte(line, target)
}

pub(crate) fn display_width_for_editor_char(ch: char, display: EditorTextDisplay) -> usize {
    display.source_char_display_width(ch)
}

pub(crate) fn advance_wrapped_column(
    row: &mut usize,
    column: &mut usize,
    width: usize,
    body_width: usize,
    display: EditorTextDisplay,
) {
    (*row, *column) = display.advance_wrapped_position(*row, *column, width, body_width);
}

pub(crate) fn byte_column_for_wrapped_row_start(
    line: &str,
    target_row: usize,
    body_width: usize,
    display: EditorTextDisplay,
) -> usize {
    display.source_byte_for_wrapped_row_column(line, target_row, 0, body_width)
}

pub(crate) fn byte_column_for_wrapped_row_column(
    line: &str,
    target_row: usize,
    target_column: usize,
    body_width: usize,
    display: EditorTextDisplay,
) -> usize {
    display.source_byte_for_wrapped_row_column(line, target_row, target_column, body_width)
}
