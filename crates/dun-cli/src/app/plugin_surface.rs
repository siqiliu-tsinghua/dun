use crate::*;

impl AppState {
    /// Route an invoked plugin action (from a menu item or a leader chord). For
    /// now every action opens a plugin-owned surface window; a richer per-action
    /// request round-trip is deferred until the first real consumer
    /// (docs/plugin-protocol.md).
    pub(crate) fn dispatch_plugin_action(&mut self, plugin_id: &str, action_id: &str) {
        // The `menu`/`keybinding` grant let the trigger appear; opening a `dun`
        // window is the separate `window` capability. The only role that grants
        // `menu`/`keybinding` (LogFilter) also grants `window`, so this gate is
        // defense in depth: an action from a host without `window` opens nothing.
        if !self
            .plugin_hosts
            .get(plugin_id)
            .is_some_and(|host| host.holds_window())
        {
            return;
        }
        self.open_plugin_surface_window(plugin_id, action_id);
    }

    /// Split off a read-only surface window owned by `plugin_id`, subject to the
    /// per-plugin window cap. The surface starts empty; `surface-write` fills it
    /// once that capability is wired.
    fn open_plugin_surface_window(&mut self, plugin_id: &str, action_id: &str) {
        if !self.plugin_windows.can_open(plugin_id) {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_PLUGIN_WINDOW_LIMIT,
                &[plugin_id],
            ));
            return;
        }

        // FocusMissing is effectively unreachable (there is always a focused
        // window); if the split cannot happen, nothing is opened or recorded.
        let Ok(window_id) = self.workspace.split_focused(Axis::Horizontal) else {
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            return;
        };
        let buffer_id = window.buffer_id;
        let surface = BufferState::new(
            buffer_id,
            TextBuffer::from_text_with_kind(BufferKind::ReadOnly, ""),
        );
        if let Some(existing) = self.buffer_state_mut(buffer_id) {
            *existing = surface;
        } else {
            self.buffers.push(surface);
        }
        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = format!("{plugin_id}: {action_id}");
            window.kind = WindowKind::PluginSurface;
            window.buffer_kind = BufferKind::ReadOnly;
            window.collapsed = false;
        }
        self.plugin_windows.record_opened(plugin_id, window_id);
    }

    /// Reap the surface windows of any plugin no longer loaded — unloaded by
    /// the user, or dropped by a config reload that rebuilt the host set.
    /// Ownership is per-plugin, so only that plugin's windows close.
    pub(crate) fn reconcile_plugin_windows(&mut self) {
        let stale: Vec<String> = self
            .plugin_windows
            .plugin_ids()
            .filter(|id| {
                self.plugin_hosts
                    .get(id)
                    .is_none_or(|host| !host.is_loaded())
            })
            .map(str::to_owned)
            .collect();
        for id in stale {
            for window in self.plugin_windows.take_all(&id) {
                self.close_plugin_window(window);
            }
        }
    }

    /// Reap every plugin surface window. A config reload rebuilds the whole host
    /// layer, so the old surfaces (and any content they held) are gone; the
    /// fresh hosts start with no windows.
    pub(crate) fn reap_all_plugin_windows(&mut self) {
        let ids: Vec<String> = self
            .plugin_windows
            .plugin_ids()
            .map(str::to_owned)
            .collect();
        for id in ids {
            for window in self.plugin_windows.take_all(&id) {
                self.close_plugin_window(window);
            }
        }
    }

    /// Close one window by id, reusing the focused-close tree repair. The
    /// workspace must keep at least one window, so a lone survivor is left in
    /// place (its plugin is already gone from the registry).
    fn close_plugin_window(&mut self, window_id: WindowId) {
        if self.workspace.window_count() <= 1 {
            return;
        }
        let buffer_id = self
            .workspace
            .window(window_id)
            .ok()
            .map(|window| window.buffer_id);
        self.workspace.focused = window_id;
        if self.workspace.close_focused().is_ok() {
            if let Some(buffer_id) = buffer_id {
                self.drop_buffer_if_unreferenced(buffer_id);
            }
        }
    }
}
