use super::super::*;

fn numbered_text(line_count: usize) -> String {
    (0..line_count)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn fold_set(ranges: Vec<FoldRange>) -> FoldSet {
    FoldSet::new(ranges).expect("valid fold set")
}

fn assert_folds(buffer: &TextBuffer, expected: &[FoldRange]) {
    let ranges = buffer.folds().ranges();
    assert_eq!(ranges, expected);
    assert!(ranges.iter().all(|range| {
        range.end_line_exclusive <= buffer.line_count()
            && range.end_line_exclusive.saturating_sub(range.start_line) >= 2
    }));
    assert!(ranges.windows(2).all(|pair| {
        pair[0].start_line <= pair[1].start_line && pair[0].end_line_exclusive <= pair[1].start_line
    }));
}

fn assert_fold_metadata_only(buffer: &TextBuffer, revision: u64) {
    assert_eq!(buffer.revision(), revision);
    assert!(!buffer.is_dirty());
    assert!(!buffer.can_undo());
    assert!(!buffer.can_redo());
}

#[test]
fn fold_above_an_edit_is_untouched() {
    let mut buffer = TextBuffer::from_text(&numbered_text(8));
    buffer.set_folds(fold_set(vec![FoldRange::new(0, 2)]));

    buffer
        .replace_range(
            TextRange::new(Position::new(2, 0), Position::new(2, 1)),
            "L",
        )
        .unwrap();

    assert_folds(&buffer, &[FoldRange::new(0, 2)]);
}

#[test]
fn fold_below_an_edit_shifts_by_the_line_delta() {
    let mut buffer = TextBuffer::from_text(&numbered_text(8));
    buffer.set_folds(fold_set(vec![FoldRange::new(4, 7)]));

    buffer
        .replace_range(TextRange::empty(Position::new(1, 0)), "inserted line\n")
        .unwrap();

    assert_folds(&buffer, &[FoldRange::new(5, 8)]);
}

#[test]
fn fold_touched_by_an_edit_is_dropped() {
    let mut inside = TextBuffer::from_text(&numbered_text(8));
    inside.set_folds(fold_set(vec![FoldRange::new(1, 6)]));
    inside
        .replace_range(
            TextRange::new(Position::new(3, 0), Position::new(3, 1)),
            "L",
        )
        .unwrap();
    assert_folds(&inside, &[]);

    let mut first_line = TextBuffer::from_text(&numbered_text(8));
    first_line.set_folds(fold_set(vec![FoldRange::new(2, 5)]));
    first_line
        .delete_range(TextRange::new(
            Position::new(1, "line 1".len()),
            Position::new(2, "line 2".len()),
        ))
        .unwrap();
    assert_folds(&first_line, &[]);

    let mut last_line = TextBuffer::from_text(&numbered_text(8));
    last_line.set_folds(fold_set(vec![FoldRange::new(2, 5)]));
    last_line
        .delete_range(TextRange::new(Position::new(4, 0), Position::new(5, 0)))
        .unwrap();
    assert_folds(&last_line, &[]);
}

#[test]
fn replace_all_remaps_every_fold_once() {
    let text = (0..12)
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
    buffer.set_folds(fold_set(vec![
        FoldRange::new(2, 4),
        FoldRange::new(5, 7),
        FoldRange::new(8, 10),
    ]));

    assert_eq!(buffer.replace_all("hit", "hit\ninserted"), Ok(3));

    assert_folds(
        &buffer,
        &[
            FoldRange::new(3, 5),
            FoldRange::new(7, 9),
            FoldRange::new(11, 13),
        ],
    );
}

#[test]
fn undo_shifts_folds_back_but_does_not_resurrect_a_dropped_one() {
    let mut buffer = TextBuffer::from_text(&numbered_text(10));
    buffer.set_folds(fold_set(vec![FoldRange::new(1, 3), FoldRange::new(6, 8)]));

    buffer
        .replace_range(TextRange::empty(Position::new(2, 0)), "inserted line\n")
        .unwrap();
    assert_folds(&buffer, &[FoldRange::new(7, 9)]);

    assert_eq!(buffer.undo(), Ok(true));
    assert_folds(&buffer, &[FoldRange::new(6, 8)]);
}

#[test]
fn swapping_lines_drops_a_fold_over_them() {
    let mut buffer = TextBuffer::from_text("zero\none\ntwo\nthree");
    buffer.set_folds(fold_set(vec![FoldRange::new(1, 3)]));
    buffer.set_cursor(Position::new(2, 0)).unwrap();

    assert_eq!(buffer.move_current_line_up(), Ok(true));

    assert_eq!(buffer.to_text(), "zero\ntwo\none\nthree");
    assert_folds(&buffer, &[]);
}

#[test]
fn fold_mutations_do_not_dirty_the_buffer_or_enter_undo() {
    let mut buffer = TextBuffer::from_text(&numbered_text(6));
    let revision = buffer.revision();
    assert_fold_metadata_only(&buffer, revision);

    buffer.set_folds(fold_set(vec![FoldRange::new(1, 9)]));
    assert_folds(&buffer, &[FoldRange::new(1, 6)]);
    assert_fold_metadata_only(&buffer, revision);

    buffer.insert_fold(FoldRange::new(0, 3));
    assert_folds(&buffer, &[FoldRange::new(0, 6)]);
    assert_fold_metadata_only(&buffer, revision);

    buffer.insert_fold(FoldRange::new(5, 6));
    assert_folds(&buffer, &[FoldRange::new(0, 6)]);
    assert_fold_metadata_only(&buffer, revision);

    assert!(buffer.remove_fold_at(2));
    assert_folds(&buffer, &[]);
    assert_fold_metadata_only(&buffer, revision);

    buffer.set_folds(fold_set(vec![FoldRange::new(0, 2), FoldRange::new(3, 5)]));
    assert_folds(&buffer, &[FoldRange::new(0, 2), FoldRange::new(3, 5)]);
    assert_fold_metadata_only(&buffer, revision);

    buffer.clear_folds();
    assert_folds(&buffer, &[]);
    assert_fold_metadata_only(&buffer, revision);
}

#[test]
fn insert_fold_drops_a_range_shorter_than_two_lines() {
    // `FoldSet::new` rejects a degenerate range, but `insert_fold` takes a
    // bare `FoldRange` and cannot, so normalisation is the only guard on this
    // entry point. A one-line fold would hide nothing while replacing the line
    // it covers with a placeholder.
    let mut buffer = TextBuffer::from_text(&numbered_text(8));
    buffer.insert_fold(FoldRange::new(3, 4));
    assert_folds(&buffer, &[]);

    buffer.insert_fold(FoldRange::new(3, 3));
    assert_folds(&buffer, &[]);

    buffer.insert_fold(FoldRange::new(3, 5));
    assert_folds(&buffer, &[FoldRange::new(3, 5)]);
}
