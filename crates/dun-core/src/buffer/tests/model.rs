use super::super::*;

#[test]
fn new_untitled_buffer_starts_empty_and_clean() {
    let buffer = TextBuffer::new_untitled();

    assert_eq!(buffer.kind(), BufferKind::Untitled);
    assert_eq!(buffer.line_count(), 1);
    assert_eq!(buffer.line(0), Some(""));
    assert_eq!(buffer.cursor_position(), Position::zero());
    assert!(!buffer.is_dirty());
}

#[test]
fn from_text_preserves_lf_shape() {
    let buffer = TextBuffer::from_text("alpha\nbeta\n");

    assert_eq!(buffer.kind(), BufferKind::File);
    assert_eq!(buffer.line_ending(), LineEnding::Lf);
    assert_eq!(buffer.line_count(), 3);
    assert_eq!(buffer.line(0), Some("alpha"));
    assert_eq!(buffer.line(1), Some("beta"));
    assert_eq!(buffer.line(2), Some(""));
    assert_eq!(buffer.to_text(), "alpha\nbeta\n");
    assert!(!buffer.is_dirty());
}

#[test]
fn from_text_preserves_crlf_shape() {
    let buffer = TextBuffer::from_text("alpha\r\nbeta");

    assert_eq!(buffer.line_ending(), LineEnding::CrLf);
    assert_eq!(buffer.line(0), Some("alpha"));
    assert_eq!(buffer.line(1), Some("beta"));
    assert_eq!(buffer.to_text(), "alpha\r\nbeta");
}

#[test]
fn mark_saved_resets_dirty_baseline() {
    let mut buffer = TextBuffer::new_untitled();

    buffer.insert_str("hello").unwrap();
    assert!(buffer.is_dirty());

    buffer.mark_saved();

    assert!(!buffer.is_dirty());
}

#[test]
fn dirty_state_survives_repeated_queries_and_undo_to_saved_state() {
    let mut buffer = TextBuffer::from_text("one");

    assert!(!buffer.is_dirty());
    assert!(!buffer.is_dirty());

    buffer.insert_char('x').unwrap();
    assert!(buffer.is_dirty());
    assert!(buffer.is_dirty());

    buffer.undo().unwrap();
    assert!(!buffer.is_dirty(), "undo back to saved content is clean");

    buffer.redo().unwrap();
    assert!(buffer.is_dirty());

    buffer.mark_saved();
    assert!(!buffer.is_dirty());
}
