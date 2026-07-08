use super::super::*;

#[test]
fn find_all_returns_utf8_match_ranges() {
    let buffer = TextBuffer::from_text("one\né one é");

    let matches = buffer.find_all("é");

    assert_eq!(
        matches,
        vec![
            SearchMatch {
                range: TextRange::new(Position::new(1, 0), Position::new(1, 2)),
            },
            SearchMatch {
                range: TextRange::new(Position::new(1, 7), Position::new(1, 9)),
            },
        ]
    );
}

#[test]
fn find_all_honors_case_and_whole_word_options() {
    let buffer = TextBuffer::from_text("ERROR errors error_error error");

    let matches = buffer.find_all_with_options(
        "error",
        SearchOptions {
            case_sensitive: false,
            whole_word: true,
        },
    );

    assert_eq!(
        matches,
        vec![
            SearchMatch {
                range: TextRange::new(Position::new(0, 0), Position::new(0, 5)),
            },
            SearchMatch {
                range: TextRange::new(Position::new(0, 25), Position::new(0, 30)),
            },
        ]
    );
}

#[test]
fn find_all_ignores_empty_query() {
    let buffer = TextBuffer::from_text("text");

    assert!(buffer.find_all("").is_empty());
}

#[test]
fn replace_all_reports_zero_for_missing_or_empty_query() {
    let mut buffer = TextBuffer::from_text("abc");

    assert_eq!(buffer.replace_all("z", "x"), Ok(0));
    assert_eq!(buffer.replace_all("", "x"), Ok(0));
    assert_eq!(buffer.to_text(), "abc");
    assert!(!buffer.can_undo());
}
