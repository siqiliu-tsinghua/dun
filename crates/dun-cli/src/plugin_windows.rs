//! Per-plugin window ownership bookkeeping for the invariants in
//! `docs/plugin-protocol.md`. This module holds no workspace state and is not
//! yet wired; its temporary `#![allow(dead_code)]` is removed when a later slice
//! adds the grant-gated workspace wiring.

// Temporary scaffolding until the grant-gated workspace wiring lands.
#![allow(dead_code)]

use dun_core::WindowId;

pub(crate) const MAX_WINDOWS_PER_PLUGIN: usize = 2;

#[derive(Debug)]
struct PluginWindowEntry {
    plugin_id: String,
    windows: Vec<WindowId>,
}

#[derive(Debug, Default)]
pub(crate) struct PluginWindows {
    entries: Vec<PluginWindowEntry>,
}

impl PluginWindows {
    pub(crate) fn count(&self, plugin_id: &str) -> usize {
        self.entries
            .iter()
            .find(|entry| entry.plugin_id == plugin_id)
            .map_or(0, |entry| entry.windows.len())
    }

    pub(crate) fn can_open(&self, plugin_id: &str) -> bool {
        self.count(plugin_id) < MAX_WINDOWS_PER_PLUGIN
    }

    pub(crate) fn record_opened(&mut self, plugin_id: &str, window: WindowId) -> bool {
        if !self.can_open(plugin_id) || self.owns(plugin_id, window) {
            return false;
        }

        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.plugin_id == plugin_id)
        {
            entry.windows.push(window);
        } else {
            self.entries.push(PluginWindowEntry {
                plugin_id: plugin_id.to_owned(),
                windows: vec![window],
            });
        }
        true
    }

    pub(crate) fn owns(&self, plugin_id: &str, window: WindowId) -> bool {
        self.entries.iter().any(|entry| {
            entry.plugin_id == plugin_id && entry.windows.iter().any(|owned| *owned == window)
        })
    }

    pub(crate) fn record_closed(&mut self, plugin_id: &str, window: WindowId) -> bool {
        let Some(entry_index) = self
            .entries
            .iter()
            .position(|entry| entry.plugin_id == plugin_id)
        else {
            return false;
        };
        let Some(window_index) = self.entries[entry_index]
            .windows
            .iter()
            .position(|owned| *owned == window)
        else {
            return false;
        };

        self.entries[entry_index].windows.remove(window_index);
        if self.entries[entry_index].windows.is_empty() {
            self.entries.remove(entry_index);
        }
        true
    }

    pub(crate) fn take_all(&mut self, plugin_id: &str) -> Vec<WindowId> {
        self.entries
            .iter()
            .position(|entry| entry.plugin_id == plugin_id)
            .map_or_else(Vec::new, |index| self.entries.remove(index).windows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_per_plugin_window_cap() {
        let mut windows = PluginWindows::default();

        assert!(windows.can_open("plugin"));
        assert!(windows.record_opened("plugin", WindowId(1)));
        assert!(windows.can_open("plugin"));
        assert!(windows.record_opened("plugin", WindowId(2)));
        assert!(!windows.can_open("plugin"));
        assert!(!windows.record_opened("plugin", WindowId(3)));
        assert_eq!(windows.count("plugin"), 2);
    }

    #[test]
    fn only_owner_can_close_window() {
        let mut windows = PluginWindows::default();
        let window = WindowId(1);

        assert!(windows.record_opened("a", window));
        assert!(windows.owns("a", window));
        assert!(!windows.owns("b", window));
        assert!(!windows.record_closed("b", window));
        assert!(windows.owns("a", window));
        assert!(windows.record_closed("a", window));
        assert_eq!(windows.count("a"), 0);
    }

    #[test]
    fn take_all_reaps_only_target_plugin_windows() {
        let mut windows = PluginWindows::default();
        assert!(windows.record_opened("a", WindowId(1)));
        assert!(windows.record_opened("a", WindowId(2)));
        assert!(windows.record_opened("b", WindowId(3)));

        let mut reaped = windows.take_all("a");
        reaped.sort_by_key(|window| window.0);
        assert_eq!(reaped, vec![WindowId(1), WindowId(2)]);
        assert_eq!(windows.count("a"), 0);
        assert!(windows.owns("b", WindowId(3)));
        assert_eq!(windows.count("b"), 1);
    }

    #[test]
    fn duplicate_window_is_not_recorded_twice() {
        let mut windows = PluginWindows::default();
        let window = WindowId(1);

        assert!(windows.record_opened("plugin", window));
        assert!(!windows.record_opened("plugin", window));
        assert_eq!(windows.count("plugin"), 1);
    }
}
