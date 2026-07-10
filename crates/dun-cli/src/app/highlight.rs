use crate::*;

impl AppState {
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
                buffer.highlight = Some(BufferHighlight {
                    revision: outcome.revision,
                    first_line: outcome.first_line,
                    spans,
                });
            }
            Err(message) => {
                self.set_status(format!("Plugin {plugin_id} failed: {message}"));
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
