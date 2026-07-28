use super::super::*;

fn numbered_text(line_count: usize) -> String {
    (0..line_count)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn bookmark_shifts_when_lines_are_inserted_above() {
    let mut buffer = TextBuffer::from_text(&numbered_text(20));
    buffer.set_bookmarks(vec![10]);

    buffer
        .replace_range(
            TextRange::empty(Position::zero()),
            "inserted 0\ninserted 1\ninserted 2\ninserted 3\ninserted 4\n",
        )
        .unwrap();

    assert_eq!(buffer.bookmarks(), [15]);
    assert_eq!(buffer.line(15), Some("line 10"));
}

#[test]
fn bookmark_shifts_when_lines_are_deleted_above() {
    let mut buffer = TextBuffer::from_text(&numbered_text(20));
    buffer.set_bookmarks(vec![10]);

    buffer
        .replace_range(TextRange::new(Position::zero(), Position::new(5, 0)), "")
        .unwrap();

    assert_eq!(buffer.bookmarks(), [5]);
    assert_eq!(buffer.line(5), Some("line 10"));
}

#[test]
fn bookmark_inside_a_replaced_span_clamps_to_the_edit_start() {
    let mut buffer = TextBuffer::from_text(&numbered_text(20));
    buffer.set_bookmarks(vec![12]);

    buffer
        .replace_range(
            TextRange::new(Position::new(10, 0), Position::new(15, "line 15".len())),
            "replacement",
        )
        .unwrap();

    assert_eq!(buffer.bookmarks(), [10]);
    assert_eq!(buffer.line(10), Some("replacement"));
}

#[test]
fn replace_all_shifts_every_bookmark_once() {
    let text = (0..10)
        .map(|line| {
            if [1, 4, 7].contains(&line) {
                "hit".to_string()
            } else {
                format!("line {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut buffer = TextBuffer::from_text(&text);
    buffer.set_bookmarks(vec![0, 3, 6, 9]);
    buffer.set_cursor(Position::new(1, 0)).unwrap();

    assert_eq!(buffer.replace_all("hit", "hit\ninserted"), Ok(3));

    assert_eq!(buffer.bookmarks(), [0, 4, 8, 12]);
    assert_eq!(
        [0, 4, 8, 12].map(|line| buffer.line(line).unwrap()),
        ["line 0", "line 3", "line 6", "line 9"]
    );
}

#[test]
fn undo_restores_bookmark_positions() {
    let mut buffer = TextBuffer::from_text(&numbered_text(20));
    buffer.set_bookmarks(vec![10]);

    buffer
        .replace_range(
            TextRange::empty(Position::zero()),
            "inserted 0\ninserted 1\ninserted 2\ninserted 3\ninserted 4\n",
        )
        .unwrap();
    assert_eq!(buffer.bookmarks(), [15]);

    assert_eq!(buffer.undo(), Ok(true));
    assert_eq!(buffer.bookmarks(), [10]);
    assert_eq!(buffer.line(10), Some("line 10"));
}

#[test]
fn move_line_still_swaps_its_bookmark() {
    let mut buffer = TextBuffer::from_text("a\nb\nc");
    buffer.set_bookmarks(vec![0]);
    buffer.set_cursor(Position::new(1, 0)).unwrap();

    assert_eq!(buffer.move_current_line_up(), Ok(true));

    assert_eq!(buffer.bookmarks(), [1]);
    assert_eq!(buffer.line(1), Some("a"));
}
