use std::time::{Duration, Instant};

use dun_plugin::{StyleId, StyleSpan};
use dun_ui::PluginIndicator;

use crate::plugins::PluginActivity;
use crate::*;

impl AppState {
    pub(crate) fn plugin_indicator(&self) -> Option<PluginIndicator> {
        if !self.plugin_status.status_bar {
            return None;
        }
        let highlighter = self.highlighter.as_ref()?;
        let idle_after = match self.plugin_status.idle_after_ms {
            0 => None,
            ms => Some(Duration::from_millis(ms)),
        };
        let id = highlighter.plugin_id();
        let (suffix, alert) = match highlighter.activity_at(Instant::now(), idle_after) {
            PluginActivity::Off => (" off", false),
            PluginActivity::Active => ("", false),
            PluginActivity::Idle => (" idle", true),
            PluginActivity::Error => (" error", true),
        };
        Some(PluginIndicator {
            text: format!("[{id}{suffix}]"),
            alert,
        })
    }

    /// One editor tick of plugin work: apply any finished highlight results,
    /// then request a snapshot for the focused buffer if its content or
    /// viewport changed. Called from the event loop; never blocks (host I/O
    /// lives on the highlighter's worker thread).
    pub(crate) fn pump_plugins(&mut self) {
        self.apply_highlight_outcomes();
        self.schedule_focused_highlight();
    }

    fn apply_highlight_outcomes(&mut self) {
        let Some(highlighter) = self.highlighter.as_mut() else {
            return;
        };
        let outcomes = highlighter.poll();
        if outcomes.is_empty() {
            return;
        }
        let plugin_id = highlighter.plugin_id().to_string();
        for outcome in outcomes {
            self.apply_highlight_outcome(&plugin_id, outcome);
        }
    }

    /// Applies one worker outcome. Results for a revision the buffer has
    /// moved past are discarded so plugin output can never paint stale
    /// state; errors surface as bounded status text (the status line is
    /// sanitized at render time like all untrusted text).
    pub(crate) fn apply_highlight_outcome(&mut self, plugin_id: &str, outcome: HighlightOutcome) {
        match outcome.result {
            Ok(spans) => {
                let Some(buffer) = self.buffer_state_mut(outcome.buffer_id) else {
                    return;
                };
                if buffer.buffer.revision() != outcome.revision {
                    return;
                }
                let spans = convert_highlight_spans(&buffer.buffer, &spans);
                buffer.highlight = Some(BufferHighlight {
                    revision: outcome.revision,
                    spans,
                });
            }
            Err(message) => {
                self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_PLUGIN_FAILED,
                    &[plugin_id, &message],
                ));
            }
        }
    }

    fn schedule_focused_highlight(&mut self) {
        if self.highlighter.is_none() {
            return;
        }
        let Ok(window) = self.workspace.focused_window() else {
            return;
        };
        if window.kind != WindowKind::Edit {
            return;
        }
        let buffer_id = window.buffer_id;
        let Some(context) = self.focused_buffer_view_context(self.workspace_area) else {
            return;
        };
        let Some(buffer) = self.buffer_state(buffer_id) else {
            return;
        };

        let first_line = buffer.first_line;
        let end_line = first_line
            .saturating_add(context.body_height.max(1))
            .min(buffer.buffer.line_count());
        if first_line >= end_line {
            return;
        }
        let lines = (first_line..end_line)
            .map(|index| buffer.buffer.line(index).unwrap_or_default().to_string())
            .collect();
        let job = HighlightJob {
            buffer_id,
            revision: buffer.buffer.revision(),
            language: language_hint(buffer.path.as_ref()),
            first_line,
            lines,
        };
        if let Some(highlighter) = self.highlighter.as_mut() {
            highlighter.schedule(job);
        }
    }
}

/// Converts validated protocol spans (character columns, per
/// docs/plugin-protocol.md) into buffer spans (byte columns, the unit used
/// by selections and search matches). Spans that no longer fit the buffer
/// are dropped defensively; validation already checked them against the
/// snapshot the plugin saw.
fn convert_highlight_spans(buffer: &TextBuffer, spans: &[StyleSpan]) -> Vec<BufferHighlightSpan> {
    let mut converted = Vec::with_capacity(spans.len());
    for span in spans {
        let line_index = span.line as usize;
        let Some(line) = buffer.line(line_index) else {
            continue;
        };
        let Some(start_column) = byte_column_for_char_index(line, span.start_col as usize) else {
            continue;
        };
        let Some(end_column) = byte_column_for_char_index(line, span.end_col as usize) else {
            continue;
        };
        if start_column >= end_column {
            continue;
        }
        converted.push(BufferHighlightSpan {
            line: line_index,
            start_column,
            end_column,
            class: highlight_class_for_style(span.style),
        });
    }
    converted
}

fn byte_column_for_char_index(line: &str, char_index: usize) -> Option<usize> {
    if char_index == 0 {
        return Some(0);
    }
    let mut seen = 0usize;
    for (byte_index, _) in line.char_indices() {
        if seen == char_index {
            return Some(byte_index);
        }
        seen += 1;
    }
    if seen == char_index {
        Some(line.len())
    } else {
        None
    }
}

const fn highlight_class_for_style(style: StyleId) -> HighlightClass {
    match style {
        StyleId::Keyword => HighlightClass::Keyword,
        StyleId::Comment => HighlightClass::Comment,
        StyleId::StringLiteral => HighlightClass::StringLiteral,
        StyleId::Number => HighlightClass::Number,
        StyleId::Emphasis => HighlightClass::Emphasis,
    }
}
