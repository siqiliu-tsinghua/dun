use super::super::*;

#[test]
fn set_cursor_rejects_invalid_utf8_boundary() {
    let mut buffer = TextBuffer::from_text("é");

    assert_eq!(
        buffer.set_cursor(Position::new(0, 1)),
        Err(BufferError::InvalidPosition(Position::new(0, 1)))
    );
    assert_eq!(buffer.cursor_position(), Position::zero());
}

#[test]
fn cursor_moves_across_utf8_char_boundaries() {
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "aé\nb");

    assert!(buffer.move_right());
    assert_eq!(buffer.cursor_position(), Position::new(0, 1));
    assert!(buffer.move_right());
    assert_eq!(buffer.cursor_position(), Position::new(0, 3));
    assert!(buffer.move_right());
    assert_eq!(buffer.cursor_position(), Position::new(1, 0));
    assert!(buffer.move_left());
    assert_eq!(buffer.cursor_position(), Position::new(0, 3));
}

#[test]
fn vertical_motion_keeps_preferred_column_when_possible() {
    let mut buffer = TextBuffer::from_text("abcd\nx\nwxyz");
    buffer.set_cursor(Position::new(0, 4)).unwrap();

    assert!(buffer.move_down());
    assert_eq!(buffer.cursor_position(), Position::new(1, 1));
    assert!(buffer.move_down());
    assert_eq!(buffer.cursor_position(), Position::new(2, 4));
}

#[test]
fn word_motion_uses_utf8_safe_boundaries() {
    let mut buffer = TextBuffer::from_text("éclair  two_2!\nthree");

    assert!(buffer.move_word_right());
    assert_eq!(buffer.cursor_position(), Position::new(0, 9));

    assert!(buffer.move_word_right());
    assert_eq!(buffer.cursor_position(), Position::new(0, 14));

    assert!(buffer.move_word_right());
    assert_eq!(buffer.cursor_position(), Position::new(1, 0));

    buffer.set_cursor(Position::new(1, 5)).unwrap();
    assert!(buffer.move_word_left());
    assert_eq!(buffer.cursor_position(), Position::new(1, 0));

    assert!(buffer.move_word_left());
    assert_eq!(buffer.cursor_position(), Position::new(0, 14));
}
