use super::support::*;
use dun_core::DisplaySanitizer;
use dun_term::AmbiguousWidth;

fn text_display() -> EditorTextDisplay {
    EditorTextDisplay::new(
        DisplaySanitizer::unlimited_utf8(),
        AmbiguousWidth::Narrow,
        GlyphSet::unicode_single_line(),
        false,
    )
}

#[test]
fn line_map_identity_matches_raw_ranges() {
    let folds = FoldSet::empty();
    for line_count in 0..32 {
        let display = EditorLineDisplay::new(line_count, &folds);
        assert_eq!(display.visible_row_count(), line_count);
        for row in 0..line_count {
            assert_eq!(
                display.item_for_visible_row(row),
                Some(VisibleLine::Source { line: row })
            );
            // Both directions, because the empty-fold fast path is a separate
            // branch from the folded one: asserting only row -> line lets an
            // off-by-one in line -> row through, and the empty set is the only
            // path production takes until folds can be created.
            assert_eq!(display.placement_for_source_line(row), Some(row));
        }
    }
}

#[test]
fn line_map_hides_folded_lines() {
    let range = FoldRange::new(2, 5);
    let folds = FoldSet::new(vec![range]).expect("valid fold set");
    let display = EditorLineDisplay::new(7, &folds);

    assert_eq!(display.visible_row_count(), 5);
    assert_eq!(
        display.item_for_visible_row(2),
        Some(VisibleLine::Fold { range })
    );
    assert_eq!(
        display.item_for_visible_row(3),
        Some(VisibleLine::Source { line: 5 })
    );
}

#[test]
fn line_map_round_trips_placement_and_lookup() {
    let first = FoldRange::new(1, 4);
    let second = FoldRange::new(6, 8);
    let folds = FoldSet::new(vec![first, second]).expect("valid fold set");
    let display = EditorLineDisplay::new(10, &folds);

    for line in 0..10 {
        let row = display
            .placement_for_source_line(line)
            .expect("source line has a placement");
        let item = display
            .item_for_visible_row(row)
            .expect("placement has a visible item");
        match item {
            VisibleLine::Source { line: visible_line } => assert_eq!(visible_line, line),
            VisibleLine::Fold { range } => {
                assert!(line >= range.start_line && line < range.end_line_exclusive);
            }
        }
    }
}

#[test]
fn visual_rows_compose_fold_then_wrap() {
    let buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdefgh\none\ntwo\nthree\nxy");
    let range = FoldRange::new(1, 4);
    let folds = FoldSet::new(vec![range]).expect("valid fold set");
    let line_map = EditorLineDisplay::new(buffer.line_count(), &folds);

    let width_four = EditorVisualRows::new(&buffer, line_map, text_display(), 4);
    assert_eq!(width_four.total_rows(), 4);
    assert_eq!(width_four.top_for_global_row(2), ViewportTop::new(1, 0));

    let width_two = EditorVisualRows::new(&buffer, line_map, text_display(), 2);
    assert_eq!(width_two.total_rows(), 6);
    assert_eq!(width_two.top_for_global_row(4), ViewportTop::new(1, 0));
}
