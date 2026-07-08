use super::super::*;

#[test]
fn extend_selection_tracks_anchor_and_utf8_boundaries() {
    let mut buffer = TextBuffer::from_text("aébc");
    buffer.set_cursor(Position::new(0, 1)).unwrap();

    assert!(buffer.extend_selection_right());
    assert_eq!(buffer.cursor_position(), Position::new(0, 3));
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::new(0, 1), Position::new(0, 3)))
    );
    assert_eq!(
        buffer.selection_range(),
        Some(TextRange::new(Position::new(0, 1), Position::new(0, 3)))
    );

    assert!(buffer.extend_selection_right());
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::new(0, 1), Position::new(0, 4)))
    );

    assert!(buffer.extend_selection_left());
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::new(0, 1), Position::new(0, 3)))
    );
    assert!(buffer.extend_selection_left());
    assert_eq!(buffer.selection(), None);
    assert_eq!(buffer.cursor_position(), Position::new(0, 1));
}

#[test]
fn extend_selection_crosses_lines_and_keeps_preferred_column() {
    let mut buffer = TextBuffer::from_text("abcd\nx\nwxyz");
    buffer.set_cursor(Position::new(0, 4)).unwrap();

    assert!(buffer.extend_selection_down());
    assert_eq!(buffer.cursor_position(), Position::new(1, 1));
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::new(0, 4), Position::new(1, 1)))
    );

    assert!(buffer.extend_selection_down());
    assert_eq!(buffer.cursor_position(), Position::new(2, 4));
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::new(0, 4), Position::new(2, 4)))
    );

    assert!(buffer.extend_selection_up());
    assert_eq!(buffer.cursor_position(), Position::new(1, 1));
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::new(0, 4), Position::new(1, 1)))
    );
}

#[test]
fn extend_selection_to_line_edges() {
    let mut buffer = TextBuffer::from_text("abc\ndef");
    buffer.set_cursor(Position::new(1, 1)).unwrap();

    assert!(buffer.extend_selection_to_line_end());
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::new(1, 1), Position::new(1, 3)))
    );

    assert!(buffer.extend_selection_to_line_start());
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::new(1, 1), Position::new(1, 0)))
    );
    assert!(buffer.extend_selection_to_line_end());
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::new(1, 1), Position::new(1, 3)))
    );
}

#[test]
fn word_selection_extends_from_anchor() {
    let mut buffer = TextBuffer::from_text("alpha beta");

    assert!(buffer.extend_selection_word_right());
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::zero(), Position::new(0, 6)))
    );

    assert!(buffer.extend_selection_word_right());
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::zero(), Position::new(0, 10)))
    );

    assert!(buffer.extend_selection_word_left());
    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::zero(), Position::new(0, 6)))
    );
}

#[test]
fn select_current_line_selects_line_plus_separator_when_possible() {
    let mut buffer = TextBuffer::from_text("first\nsecond\nthird");
    buffer.set_cursor(Position::new(1, 2)).unwrap();

    buffer.select_current_line().unwrap();

    assert_eq!(
        buffer.selection(),
        Some(Selection::new(Position::new(1, 0), Position::new(2, 0)))
    );
    assert_eq!(
        buffer
            .text_in_range(buffer.selection_range().unwrap())
            .unwrap(),
        "second\n"
    );
}
