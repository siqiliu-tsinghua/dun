use dun_plugin::StyleId;

use crate::*;

fn span(line: u32) -> StyleSpan {
    StyleSpan {
        line,
        start_col: 0,
        end_col: 1,
        style: StyleId::Keyword,
    }
}

fn job(revision: u64, first_line: usize, line_count: usize) -> HighlightJob {
    HighlightJob {
        buffer_id: BufferId(1),
        revision,
        language: "rust".to_string(),
        first_line,
        lines: vec!["fn main() {}".to_string(); line_count],
    }
}

#[test]
fn language_hint_uses_lowercased_extension() {
    assert_eq!(language_hint(Some(&PathBuf::from("/tmp/Main.RS"))), "rs");
    assert_eq!(language_hint(Some(&PathBuf::from("/tmp/README"))), "");
    assert_eq!(language_hint(None), "");
}

#[test]
fn highlight_outcome_applies_only_for_the_current_revision() {
    let mut app = AppState::new();

    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            first_line: 0,
            result: Ok(vec![span(0)]),
        },
    );
    let stored = app.buffer_state(BufferId(1)).unwrap().highlight.clone();
    assert_eq!(
        stored,
        Some(BufferHighlight {
            revision: 0,
            first_line: 0,
            spans: vec![span(0)],
        })
    );

    app.buffer_state_mut(BufferId(1))
        .unwrap()
        .buffer
        .insert_char('x')
        .unwrap();
    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            first_line: 0,
            result: Ok(vec![span(0), span(0)]),
        },
    );
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().highlight,
        stored,
        "stale outcome must not overwrite the cache"
    );
}

#[test]
fn highlight_error_outcome_reports_plugin_status() {
    let mut app = AppState::new();

    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            first_line: 0,
            result: Err("plugin host timed out".to_string()),
        },
    );

    assert_eq!(
        app.status_message,
        Some("Plugin demo failed: plugin host timed out".to_string())
    );
}

#[test]
fn schedule_dedupes_identical_snapshots_and_sends_changed_ones() {
    let (mut highlighter, jobs) = PluginHighlighter::for_tests();

    assert!(highlighter.schedule(job(0, 0, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(job(0, 0, 3)));

    assert!(!highlighter.schedule(job(0, 0, 3)));
    assert!(jobs.try_recv().is_err());

    assert!(highlighter.schedule(job(1, 0, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(job(1, 0, 3)));

    assert!(highlighter.schedule(job(1, 5, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(job(1, 5, 3)));
}
