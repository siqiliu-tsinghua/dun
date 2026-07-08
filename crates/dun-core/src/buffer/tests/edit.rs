use super::super::*;

#[test]
fn insert_text_updates_line_and_cursor() {
    let mut buffer = TextBuffer::new_untitled();

    buffer.insert_str("hello").unwrap();

    assert_eq!(buffer.line(0), Some("hello"));
    assert_eq!(buffer.cursor_position(), Position::new(0, 5));
    assert!(buffer.is_dirty());
    assert!(buffer.can_undo());
    assert!(!buffer.can_redo());
}

#[test]
fn delete_word_commands_remove_to_word_boundaries() {
    let mut buffer = TextBuffer::from_text("alpha beta gamma");

    assert!(buffer.delete_word_forward().unwrap());
    assert_eq!(buffer.to_text(), "beta gamma");
    assert_eq!(buffer.cursor_position(), Position::zero());

    buffer
        .set_cursor(Position::new(0, "beta gamma".len()))
        .unwrap();
    assert!(buffer.delete_word_backward().unwrap());
    assert_eq!(buffer.to_text(), "beta ");
    assert_eq!(buffer.cursor_position(), Position::new(0, 5));
}

#[test]
fn insert_newline_splits_current_line() {
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hello");
    buffer.set_cursor(Position::new(0, 2)).unwrap();

    buffer.insert_newline().unwrap();

    assert_eq!(buffer.line_count(), 2);
    assert_eq!(buffer.line(0), Some("he"));
    assert_eq!(buffer.line(1), Some("llo"));
    assert_eq!(buffer.cursor_position(), Position::new(1, 0));
}

#[test]
fn insert_replaces_active_selection() {
    let mut buffer = TextBuffer::from_text("abcdef");
    buffer
        .select(Position::new(0, 2), Position::new(0, 5))
        .unwrap();

    buffer.insert_str("X").unwrap();

    assert_eq!(buffer.to_text(), "abXf");
    assert_eq!(buffer.cursor_position(), Position::new(0, 3));
    assert_eq!(buffer.selection(), None);
}

#[test]
fn delete_backward_removes_previous_utf8_character() {
    let mut buffer = TextBuffer::from_text("aé");
    buffer.set_cursor(Position::new(0, 3)).unwrap();

    assert!(buffer.delete_backward().unwrap());

    assert_eq!(buffer.to_text(), "a");
    assert_eq!(buffer.cursor_position(), Position::new(0, 1));
}

#[test]
fn delete_backward_at_line_start_merges_with_previous_line() {
    let mut buffer = TextBuffer::from_text("one\ntwo");
    buffer.set_cursor(Position::new(1, 0)).unwrap();

    assert!(buffer.delete_backward().unwrap());

    assert_eq!(buffer.to_text(), "onetwo");
    assert_eq!(buffer.cursor_position(), Position::new(0, 3));
}

#[test]
fn delete_forward_at_line_end_merges_with_next_line() {
    let mut buffer = TextBuffer::from_text("one\ntwo");
    buffer.set_cursor(Position::new(0, 3)).unwrap();

    assert!(buffer.delete_forward().unwrap());

    assert_eq!(buffer.to_text(), "onetwo");
    assert_eq!(buffer.cursor_position(), Position::new(0, 3));
}

#[test]
fn delete_range_removes_multiline_text() {
    let mut buffer = TextBuffer::from_text("alpha\nbeta\ngamma");

    assert!(
        buffer
            .delete_range(TextRange::new(Position::new(0, 2), Position::new(2, 2)))
            .unwrap()
    );

    assert_eq!(buffer.to_text(), "almma");
    assert_eq!(buffer.cursor_position(), Position::new(0, 2));
}

#[test]
fn replace_range_accepts_crlf_paste_as_internal_lf() {
    let mut buffer = TextBuffer::new_untitled();

    buffer
        .replace_range(TextRange::empty(Position::zero()), "a\r\nb")
        .unwrap();

    assert_eq!(buffer.line_count(), 2);
    assert_eq!(buffer.to_text(), "a\nb");
}

#[test]
fn readonly_buffer_rejects_editing() {
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::ReadOnly, "locked");

    assert_eq!(buffer.insert_char('!'), Err(BufferError::ReadOnly));
    assert_eq!(buffer.delete_forward(), Err(BufferError::ReadOnly));
    assert_eq!(buffer.undo(), Err(BufferError::ReadOnly));
    assert_eq!(buffer.to_text(), "locked");
}
