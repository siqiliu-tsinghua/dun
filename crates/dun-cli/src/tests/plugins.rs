use dun_plugin::{StyleId, StyleSpan};

use super::support::app_with_text;
use crate::plugins::{WorkerMessage, next_worker_job_for_tests};
use crate::*;

fn span(line: u32, start_col: u32, end_col: u32) -> StyleSpan {
    StyleSpan {
        line,
        start_col,
        end_col,
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
    let (mut highlighter, jobs) = PluginHighlighter::for_tests();

    assert!(highlighter.schedule(job(0, 0, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(WorkerMessage::Job(job(0, 0, 3))));

    assert!(!highlighter.schedule(job(0, 0, 3)));
    assert!(jobs.try_recv().is_err());

    assert!(highlighter.schedule(job(1, 0, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(WorkerMessage::Job(job(1, 0, 3))));

    assert!(highlighter.schedule(job(1, 5, 3)));
    assert_eq!(jobs.try_recv().ok(), Some(WorkerMessage::Job(job(1, 5, 3))));
}

#[test]
fn unload_then_load_resets_dedupe_so_next_snapshot_resends() {
    let (mut highlighter, messages) = PluginHighlighter::for_tests();
    let snapshot = job(7, 2, 4);

    assert!(highlighter.schedule(snapshot.clone()));
    assert_eq!(
        messages.try_recv().ok(),
        Some(WorkerMessage::Job(snapshot.clone()))
    );
    assert!(!highlighter.schedule(snapshot.clone()));

    highlighter.unload();
    assert!(!highlighter.is_loaded());
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Unload));

    highlighter.load();
    assert!(highlighter.is_loaded());
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Load));

    assert!(highlighter.schedule(snapshot.clone()));
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Job(snapshot)));
}

#[test]
fn worker_unload_drops_jobs_until_load_reenables_them() {
    let (mut highlighter, messages) = PluginHighlighter::for_tests();
    let snapshot = job(3, 0, 2);
    let mut worker_unloaded = false;

    highlighter.unload();
    assert!(highlighter.schedule(snapshot.clone()));
    assert_eq!(
        next_worker_job_for_tests(&messages, &mut worker_unloaded).unwrap(),
        None,
        "an unloaded worker must not return a job to the launch/request path"
    );
    assert!(worker_unloaded);

    highlighter.load();
    assert!(highlighter.schedule(snapshot.clone()));
    assert_eq!(
        next_worker_job_for_tests(&messages, &mut worker_unloaded).unwrap(),
        Some(snapshot)
    );
    assert!(!worker_unloaded);
}

#[test]
fn plugin_command_reports_and_controls_the_highlighter() {
    let mut app = AppState::new();
    let (highlighter, messages) = PluginHighlighter::for_tests();
    app.highlighter = Some(highlighter);

    app.run_command_line("plugin");
    assert_eq!(
        app.status_message,
        Some("Plugin test-plugin is loaded".to_string())
    );

    app.run_command_line("plugin unload");
    assert_eq!(
        app.status_message,
        Some("Plugin test-plugin unloaded".to_string())
    );
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Unload));

    app.run_command_line("plugin");
    assert_eq!(
        app.status_message,
        Some("Plugin test-plugin is unloaded".to_string())
    );

    app.run_command_line("plugin load");
    assert_eq!(
        app.status_message,
        Some("Plugin test-plugin loaded (starts on the next edit)".to_string())
    );
    assert_eq!(messages.try_recv().ok(), Some(WorkerMessage::Load));

    app.run_command_line("plugin restart");
    assert_eq!(
        app.status_message,
        Some("Usage: plugin [load|unload]".to_string())
    );
}

#[test]
fn plugin_command_reports_when_no_highlighter_is_configured() {
    let mut app = AppState::new();

    for command in ["plugin", "plugin load", "plugin unload"] {
        app.run_command_line(command);
        assert_eq!(
            app.status_message,
            Some("No syntax-highlight plugin configured".to_string())
        );
    }
}
