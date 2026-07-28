//! The syntax-highlight facet: scheduling, outcomes, revision guards.
//!
//! Split out of `tests/plugins.rs` when it passed the 45k test-file debt
//! threshold in docs/dev/code-organization-guidelines.md. Tests moved verbatim;
//! shared helpers stay in the parent module.

use super::*;

#[test]
fn language_hint_uses_lowercased_extension() {
    assert_eq!(language_hint(Some(&PathBuf::from("/tmp/Main.RS"))), "rs");
    assert_eq!(language_hint(Some(&PathBuf::from("/tmp/README"))), "");
    assert_eq!(language_hint(None), "");
}

#[test]
fn highlight_outcome_applies_only_for_the_current_revision() {
    let mut app = app_with_text("fn main");

    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            result: Ok(vec![span(0, 0, 2)]),
        },
    );
    let stored = app.buffer_state(BufferId(1)).unwrap().highlight.clone();
    assert_eq!(
        stored,
        Some(BufferHighlight {
            revision: 0,
            spans: vec![BufferHighlightSpan {
                line: 0,
                start_column: 0,
                end_column: 2,
                class: HighlightClass::Keyword,
            }],
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
            result: Ok(vec![span(0, 0, 1)]),
        },
    );
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().highlight,
        stored,
        "stale outcome must not overwrite the cache"
    );
}

#[test]
fn highlight_conversion_maps_char_columns_to_byte_columns() {
    // "a中b": char columns 1..2 cover 中, which is bytes 1..4.
    let mut app = app_with_text("a\u{4e2d}b");

    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            result: Ok(vec![span(0, 1, 2), span(0, 9, 10)]),
        },
    );

    let highlight = app
        .buffer_state(BufferId(1))
        .unwrap()
        .highlight
        .clone()
        .unwrap();
    assert_eq!(
        highlight.spans,
        vec![BufferHighlightSpan {
            line: 0,
            start_column: 1,
            end_column: 4,
            class: HighlightClass::Keyword,
        }],
        "wide char widens to its byte range; the out-of-range span is dropped"
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
            result: Err("plugin host timed out".to_string()),
        },
    );

    assert_eq!(
        app.status_message,
        Some("Plugin demo failed: plugin host timed out".to_string())
    );
}

#[test]
fn highlight_failure_leaves_buffer_and_prior_highlight_untouched() {
    let mut app = app_with_text("fn main");
    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            result: Ok(vec![span(0, 0, 2)]),
        },
    );
    let highlight_before = app.buffer_state(BufferId(1)).unwrap().highlight.clone();
    let revision_before = app.buffer_state(BufferId(1)).unwrap().buffer.revision();
    assert!(highlight_before.is_some());

    // A later failure for the same buffer must be inert beyond the status
    // line: it never touches buffer text, revision, or the valid highlight.
    app.apply_highlight_outcome(
        "demo",
        HighlightOutcome {
            buffer_id: BufferId(1),
            revision: 0,
            result: Err("plugin host crashed".to_string()),
        },
    );

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().highlight,
        highlight_before,
        "a plugin failure must not clear the existing highlight"
    );
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.revision(),
        revision_before,
        "a plugin failure must not mutate the buffer"
    );
    assert_eq!(
        app.status_message,
        Some("Plugin demo failed: plugin host crashed".to_string())
    );
}

#[test]
fn schedule_dedupes_identical_snapshots_and_sends_changed_ones() {
    let (mut host, jobs, _events) = PluginHost::for_tests();

    assert!(host.schedule(job(0, 0, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(WorkerMessage::Job(job(0, 0, 3))));

    assert!(!host.schedule(job(0, 0, 3)));
    assert!(jobs.try_recv().is_err());

    assert!(host.schedule(job(1, 0, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(WorkerMessage::Job(job(1, 0, 3))));

    assert!(host.schedule(job(1, 5, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(WorkerMessage::Job(job(1, 5, 3))));
}
