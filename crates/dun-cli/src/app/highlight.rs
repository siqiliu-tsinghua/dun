use std::time::{Duration, Instant};

use dun_plugin::{StyleId, StyleSpan};
use dun_ui::PluginIndicator;

use crate::plugins::PluginActivity;
use crate::*;

impl AppState {
    pub(crate) fn plugin_indicator(&self) -> Option<PluginIndicator> {
        if !self.plugin_status.status_bar || self.plugin_hosts.is_empty() {
            return None;
        }
        let idle_after = match self.plugin_status.idle_after_ms {
            0 => None,
            ms => Some(Duration::from_millis(ms)),
        };
        let now = Instant::now();
        let mut text = String::new();
        let mut alert = false;
        for host in self.plugin_hosts.iter() {
            let (suffix, host_alert) = match host.activity_at(now, idle_after) {
                PluginActivity::Off => (" off", false),
                PluginActivity::Active => ("", false),
                PluginActivity::Idle => (" idle", true),
                PluginActivity::Error => (" error", true),
            };
            text.push_str(&format!("[{}{suffix}]", host.plugin_id()));
            alert |= host_alert;
        }
        Some(PluginIndicator { text, alert })
    }

    /// One editor tick of plugin work: apply any finished worker events,
    /// then request a snapshot for the focused buffer if its content or
    /// viewport changed. Called from the event loop; never blocks (host I/O
    /// lives on each host's worker thread).
    pub(crate) fn pump_plugins(&mut self) {
        self.apply_plugin_events();
        self.schedule_focused_highlight();
    }

    /// Polls every host. Each host absorbs its own handshake results while
    /// polling; what remains — launch failures, highlight outcomes — is
    /// applied here, after the hosts borrow ends.
    fn apply_plugin_events(&mut self) {
        let mut pending: Vec<(String, HostEvent)> = Vec::new();
        for host in self.plugin_hosts.iter_mut() {
            let events = host.poll();
            if events.is_empty() {
                continue;
            }
            let plugin_id = host.plugin_id().to_string();
            pending.extend(events.into_iter().map(|event| (plugin_id.clone(), event)));
        }
        for (plugin_id, event) in pending {
            match event {
                // A handshake's menu contribution is absorbed inside `poll` and
                // never surfaces here; `refresh_plugin_menus` below picks it up.
                HostEvent::Started { .. } => {}
                HostEvent::StartFailed { error } => {
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_PLUGIN_FAILED,
                        &[&plugin_id, &error],
                    ));
                }
                HostEvent::Highlight(outcome) => self.apply_highlight_outcome(&plugin_id, outcome),
            }
        }
        self.refresh_plugin_menus();
    }

    /// Rebuilds the plugin-contributed menus shown after the built-in ones from
    /// the hosts' current contributions. Cheap and idempotent: a handshake
    /// (`Started`) is absorbed inside `poll` without surfacing an event, so
    /// rather than track that, every pump recomputes and only reassigns on an
    /// actual change (`menus` skips hosts with no contribution). `plugin
    /// load`/`unload` call it directly for a synchronous refresh.
    pub(crate) fn refresh_plugin_menus(&mut self) {
        let items = self
            .plugin_hosts
            .resolved_menu_items(&self.plugin_menu_tags);
        if items != self.shell.plugin_menu_items {
            self.shell.plugin_menu_items = items;
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
        if self.plugin_hosts.highlighter().is_none() {
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
        if let Some(host) = self.plugin_hosts.highlighter_mut() {
            host.schedule(job);
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
