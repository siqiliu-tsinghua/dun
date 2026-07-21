use crate::*;

impl AppState {
    /// Route an invoked plugin action (from a menu item or a leader chord).
    /// Opens (or reuses) the plugin's surface window, then — if the host holds
    /// `surface-write` — asks it for the content to show; the response fills the
    /// window on the next pump (`apply_surface_outcome`). A host with `window`
    /// but not `surface-write` gets an empty surface.
    pub(crate) fn dispatch_plugin_action(&mut self, plugin_id: &str, action_id: &str) {
        // The `menu`/`keybinding` grant let the trigger appear; opening a `dun`
        // window is the separate `window` capability. The only role that grants
        // `menu`/`keybinding` (LogFilter) also grants `window`, so this gate is
        // defense in depth: an action from a host without `window` opens nothing.
        let Some(host) = self.plugin_hosts.get(plugin_id) else {
            return;
        };
        if !host.holds_window() {
            return;
        }
        let holds_surface_write = host.holds_surface_write();
        if self
            .ensure_plugin_surface_window(plugin_id, action_id)
            .is_none()
        {
            return;
        }
        if holds_surface_write {
            if let Some(host) = self.plugin_hosts.get_mut(plugin_id) {
                host.send_surface_request(action_id);
            }
        }
    }

    /// Apply a surface-write response: fill the plugin's surface window with the
    /// host's lines, or surface the error. Runs after the request round-trip.
    pub(crate) fn apply_surface_outcome(
        &mut self,
        plugin_id: &str,
        action_id: &str,
        result: Result<Vec<String>, String>,
    ) {
        let lines = match result {
            Ok(lines) => lines,
            Err(message) => {
                self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_PLUGIN_FAILED,
                    &[plugin_id, &message],
                ));
                return;
            }
        };
        self.fill_plugin_surface(plugin_id, action_id, &lines);
    }

    /// Open-or-reuse the plugin's surface window and fill its read-only buffer
    /// with `lines`. Shared by the surface-write response and the stream-read
    /// filter output.
    fn fill_plugin_surface(&mut self, plugin_id: &str, title_action: &str, lines: &[String]) {
        let Some(window_id) = self.ensure_plugin_surface_window(plugin_id, title_action) else {
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            return;
        };
        let buffer_id = window.buffer_id;
        let surface = BufferState::new(
            buffer_id,
            TextBuffer::from_text_with_kind(BufferKind::ReadOnly, &lines.join("\n")),
        );
        if let Some(existing) = self.buffer_state_mut(buffer_id) {
            *existing = surface;
        } else {
            self.buffers.push(surface);
        }
    }

    /// Feed a stream of output lines to every host granted `stream-read`,
    /// remembering the lines so the returned verdict can be applied
    /// positionally. Each host filters the whole stream as one chunk.
    pub(crate) fn feed_stream_to_filters(&mut self, stream_id: &str, lines: &[String]) {
        for host in self.plugin_hosts.iter_mut() {
            if !host.holds_stream_read() {
                continue;
            }
            host.send_stream_request(stream_id, lines);
        }
    }

    /// Apply a stream-read verdict: keep the fed lines the host marked, and show
    /// them in the host's surface window. A verdict whose length no longer
    /// matches the remembered lines (a stale or racing feed) is dropped.
    pub(crate) fn apply_stream_verdict(
        &mut self,
        plugin_id: &str,
        result: Result<Vec<bool>, String>,
    ) {
        let keep = match result {
            Ok(keep) => keep,
            Err(message) => {
                self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_PLUGIN_FAILED,
                    &[plugin_id, &message],
                ));
                return;
            }
        };
        let Some(host) = self.plugin_hosts.get_mut(plugin_id) else {
            return;
        };
        let (stream_id, fed) = host.take_pending_stream();
        if keep.len() != fed.len() {
            return;
        }
        let kept: Vec<String> = fed
            .into_iter()
            .zip(keep)
            .filter_map(|(line, keep)| keep.then_some(line))
            .collect();
        self.fill_plugin_surface(plugin_id, &stream_id, &kept);
    }

    /// The plugin's existing surface window, if one is still open.
    fn plugin_surface_window(&self, plugin_id: &str) -> Option<WindowId> {
        self.workspace
            .windows
            .iter()
            .find(|window| {
                window.kind == WindowKind::PluginSurface
                    && self.plugin_windows.owns(plugin_id, window.id)
            })
            .map(|window| window.id)
    }

    /// Return the plugin's surface window, reusing an open one or splitting off a
    /// new read-only one (subject to the per-plugin cap). Reuse means a plugin
    /// keeps a single surface rather than spawning one per invoke.
    fn ensure_plugin_surface_window(
        &mut self,
        plugin_id: &str,
        action_id: &str,
    ) -> Option<WindowId> {
        if let Some(window_id) = self.plugin_surface_window(plugin_id) {
            if let Ok(window) = self.workspace.window_mut(window_id) {
                window.title = format!("{plugin_id}: {action_id}");
                window.collapsed = false;
            }
            self.workspace.focused = window_id;
            return Some(window_id);
        }

        if !self.plugin_windows.can_open(plugin_id) {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_PLUGIN_WINDOW_LIMIT,
                &[plugin_id],
            ));
            return None;
        }

        // FocusMissing is effectively unreachable (there is always a focused
        // window); if the split cannot happen, nothing is opened or recorded.
        let window_id = self.workspace.split_focused(Axis::Horizontal).ok()?;
        let buffer_id = self.workspace.window(window_id).ok()?.buffer_id;
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
        Some(window_id)
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
