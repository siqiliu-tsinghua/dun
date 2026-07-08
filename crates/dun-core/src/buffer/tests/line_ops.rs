use super::super::*;

#[test]
fn line_commands_delete_move_indent_and_trim() {
    let mut buffer = TextBuffer::from_text("alpha  \nbeta\ngamma   ");
    buffer.set_cursor(Position::new(1, 0)).unwrap();

    assert_eq!(buffer.move_current_line_up(), Ok(true));
    assert_eq!(buffer.to_text(), "beta\nalpha  \ngamma   ");
    assert_eq!(buffer.undo(), Ok(true));
    assert_eq!(buffer.to_text(), "alpha  \nbeta\ngamma   ");

    assert_eq!(buffer.indent_selected_lines("    "), Ok(1));
    assert_eq!(buffer.to_text(), "alpha  \n    beta\ngamma   ");
    assert_eq!(buffer.outdent_selected_lines(4), Ok(1));
    assert_eq!(buffer.to_text(), "alpha  \nbeta\ngamma   ");

    assert_eq!(buffer.trim_trailing_whitespace(), Ok(2));
    assert_eq!(buffer.to_text(), "alpha\nbeta\ngamma");

    assert_eq!(buffer.delete_current_line(), Ok(true));
    assert_eq!(buffer.to_text(), "alpha\ngamma");
}

#[test]
fn line_commands_apply_to_selected_line_range() {
    let mut buffer = TextBuffer::from_text("one\ntwo\nthree");
    buffer
        .select(Position::new(0, 1), Position::new(2, 0))
        .unwrap();

    assert_eq!(buffer.indent_selected_lines("    "), Ok(2));
    assert_eq!(buffer.to_text(), "    one\n    two\nthree");

    assert_eq!(buffer.outdent_selected_lines(4), Ok(2));
    assert_eq!(buffer.to_text(), "one\ntwo\nthree");
}
