use super::super::*;

#[test]
fn insert_char_run_undoes_as_one_transaction() {
    let mut buffer = TextBuffer::new_untitled();

    buffer.insert_char('a').unwrap();
    buffer.insert_char('é').unwrap();
    buffer.insert_char('b').unwrap();

    assert_eq!(buffer.to_text(), "aéb");
    assert_eq!(buffer.undo_stack.len(), 1);

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "");
    assert_eq!(buffer.cursor_position(), Position::zero());
    assert!(buffer.can_redo());

    assert!(buffer.redo().unwrap());
    assert_eq!(buffer.to_text(), "aéb");
    assert_eq!(buffer.cursor_position(), Position::new(0, 4));
}

#[test]
fn cursor_motion_breaks_insert_char_merge() {
    let mut buffer = TextBuffer::new_untitled();

    buffer.insert_char('a').unwrap();
    assert!(buffer.move_left());
    assert!(buffer.move_right());
    buffer.insert_char('b').unwrap();

    assert_eq!(buffer.to_text(), "ab");
    assert_eq!(buffer.undo_stack.len(), 2);

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "a");
    assert_eq!(buffer.cursor_position(), Position::new(0, 1));

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "");
    assert_eq!(buffer.cursor_position(), Position::zero());
}

#[test]
fn insert_str_does_not_merge_with_insert_char_runs() {
    let mut buffer = TextBuffer::new_untitled();

    buffer.insert_char('a').unwrap();
    buffer.insert_str("bc").unwrap();
    buffer.insert_char('d').unwrap();

    assert_eq!(buffer.to_text(), "abcd");
    assert_eq!(buffer.undo_stack.len(), 3);

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "abc");

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "a");

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "");
}

#[test]
fn redoed_insert_run_does_not_absorb_new_typing() {
    let mut buffer = TextBuffer::new_untitled();

    buffer.insert_char('a').unwrap();
    buffer.insert_char('b').unwrap();
    assert!(buffer.undo().unwrap());
    assert!(buffer.redo().unwrap());
    buffer.insert_char('c').unwrap();

    assert_eq!(buffer.to_text(), "abc");
    assert_eq!(buffer.undo_stack.len(), 2);

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "ab");

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "");
}

#[test]
fn delete_backward_run_undoes_as_one_transaction() {
    let mut buffer = TextBuffer::from_text("abcd");
    buffer.set_cursor(Position::new(0, 4)).unwrap();

    assert!(buffer.delete_backward().unwrap());
    assert!(buffer.delete_backward().unwrap());

    assert_eq!(buffer.to_text(), "ab");
    assert_eq!(buffer.cursor_position(), Position::new(0, 2));
    assert_eq!(buffer.undo_stack.len(), 1);

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "abcd");
    assert_eq!(buffer.cursor_position(), Position::new(0, 4));

    assert!(buffer.redo().unwrap());
    assert_eq!(buffer.to_text(), "ab");
    assert_eq!(buffer.cursor_position(), Position::new(0, 2));
}

#[test]
fn delete_forward_run_undoes_as_one_transaction() {
    let mut buffer = TextBuffer::from_text("abcd");

    assert!(buffer.delete_forward().unwrap());
    assert!(buffer.delete_forward().unwrap());

    assert_eq!(buffer.to_text(), "cd");
    assert_eq!(buffer.cursor_position(), Position::zero());
    assert_eq!(buffer.undo_stack.len(), 1);

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "abcd");
    assert_eq!(buffer.cursor_position(), Position::zero());
}

#[test]
fn switching_delete_direction_breaks_delete_merge() {
    let mut buffer = TextBuffer::from_text("abcd");
    buffer.set_cursor(Position::new(0, 2)).unwrap();

    assert!(buffer.delete_backward().unwrap());
    assert!(buffer.delete_forward().unwrap());

    assert_eq!(buffer.to_text(), "ad");
    assert_eq!(buffer.undo_stack.len(), 2);

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "acd");
    assert_eq!(buffer.cursor_position(), Position::new(0, 1));
}

#[test]
fn undo_and_redo_restore_content_and_cursor() {
    let mut buffer = TextBuffer::new_untitled();

    buffer.insert_str("hello").unwrap();
    assert_eq!(buffer.cursor_position(), Position::new(0, 5));

    assert!(buffer.undo().unwrap());
    assert_eq!(buffer.to_text(), "");
    assert_eq!(buffer.cursor_position(), Position::zero());
    assert!(buffer.can_redo());

    assert!(buffer.redo().unwrap());
    assert_eq!(buffer.to_text(), "hello");
    assert_eq!(buffer.cursor_position(), Position::new(0, 5));
}

#[test]
fn undo_back_to_saved_content_clears_dirty_state() {
    let mut buffer = TextBuffer::new_untitled();

    buffer.insert_str("hello").unwrap();
    assert!(buffer.is_dirty());

    assert!(buffer.undo().unwrap());

    assert!(!buffer.is_dirty());
}

#[test]
fn replace_all_is_one_undo_transaction() {
    let mut buffer = TextBuffer::from_text("one two one");

    assert_eq!(buffer.replace_all("one", "uno"), Ok(2));
    assert_eq!(buffer.to_text(), "uno two uno");
    assert!(buffer.can_undo());

    assert_eq!(buffer.undo(), Ok(true));
    assert_eq!(buffer.to_text(), "one two one");

    assert_eq!(buffer.redo(), Ok(true));
    assert_eq!(buffer.to_text(), "uno two uno");
}
