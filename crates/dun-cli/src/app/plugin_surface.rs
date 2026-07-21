use crate::*;

impl AppState {
    /// Route an invoked plugin action (from a menu item or a leader chord) by
    /// its kind: `Surface` opens/refreshes the read-only surface, `Scratch`
    /// opens the editable scratch window, `Execute` submits the scratch text to
    /// the host. All three require the `window` capability — the gate the
    /// `menu`/`keybinding` trigger does not itself grant.
    pub(crate) fn dispatch_plugin_action(
        &mut self,
        plugin_id: &str,
        action_id: &str,
        kind: PluginActionKind,
    ) {
        let Some(host) = self.plugin_hosts.get(plugin_id) else {
            return;
        };
        if !host.holds_window() {
            return;
        }
        match kind {
            PluginActionKind::Surface => self.dispatch_surface_action(plugin_id, action_id),
            PluginActionKind::Scratch => self.dispatch_scratch_action(plugin_id, action_id),
            PluginActionKind::Execute => self.dispatch_execute_action(plugin_id),
        }
    }

    /// A `Surface`-kind action: open (or reuse) the read-only surface window and,
    /// if the host holds `surface-write`, ask it for content (filled by
    /// `apply_surface_outcome` on the next pump). A `window`-only host gets an
    /// empty surface.
    fn dispatch_surface_action(&mut self, plugin_id: &str, action_id: &str) {
        let holds_surface_write = self
            .plugin_hosts
            .get(plugin_id)
            .is_some_and(|host| host.holds_surface_write());
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

    /// A `Scratch`-kind action: open (or reuse) the plugin's editable scratch
    /// window (`scratch-input`). Ignored for a host without that grant.
    fn dispatch_scratch_action(&mut self, plugin_id: &str, action_id: &str) {
        if !self
            .plugin_hosts
            .get(plugin_id)
            .is_some_and(|host| host.holds_scratch_input())
        {
            return;
        }
        self.ensure_plugin_scratch_window(plugin_id, action_id);
    }

    /// An `Execute`-kind action: submit the plugin's scratch buffer text to the
    /// host (`execute`); the result fills its surface window on the next pump
    /// (`apply_surface_outcome`). Ignored without a scratch window or the grant.
    fn dispatch_execute_action(&mut self, plugin_id: &str) {
        if !self
            .plugin_hosts
            .get(plugin_id)
            .is_some_and(|host| host.holds_scratch_input())
        {
            return;
        }
        let Some(snippet) = self.plugin_scratch_text(plugin_id) else {
            return;
        };
        if let Some(host) = self.plugin_hosts.get_mut(plugin_id) {
            host.send_execute_request(&snippet);
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
            host.send_stream_chunks(stream_id, lines);
        }
    }

    /// Apply one stream-read chunk verdict: accumulate the kept lines across the
    /// stream's chunks and show the running result in the host's surface window.
    /// A failed verdict still answers one sent chunk, so it discards that chunk
    /// to keep the queue aligned; a length-mismatched verdict drops its chunk.
    pub(crate) fn apply_stream_verdict(
        &mut self,
        plugin_id: &str,
        result: Result<Vec<bool>, String>,
    ) {
        let keep = match result {
            Ok(keep) => keep,
            Err(message) => {
                if let Some(host) = self.plugin_hosts.get_mut(plugin_id) {
                    host.discard_pending_stream_chunk();
                }
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
        let Some((stream_id, accumulated)) = host.apply_stream_chunk_verdict(&keep) else {
            return;
        };
        self.fill_plugin_surface(plugin_id, &stream_id, &accumulated);
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

    /// The plugin's existing editable scratch window, if one is still open.
    fn plugin_scratch_window(&self, plugin_id: &str) -> Option<WindowId> {
        self.workspace
            .windows
            .iter()
            .find(|window| {
                window.kind == WindowKind::PluginScratch
                    && self.plugin_windows.owns(plugin_id, window.id)
            })
            .map(|window| window.id)
    }

    /// Open (or focus) the plugin's editable scratch window (`scratch-input`),
    /// subject to the per-plugin window cap. Unlike a surface, the buffer is an
    /// editable `dun`-native buffer the user types into.
    fn ensure_plugin_scratch_window(
        &mut self,
        plugin_id: &str,
        action_id: &str,
    ) -> Option<WindowId> {
        if let Some(window_id) = self.plugin_scratch_window(plugin_id) {
            if let Ok(window) = self.workspace.window_mut(window_id) {
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

        let window_id = self.workspace.split_focused(Axis::Horizontal).ok()?;
        let buffer_id = self.workspace.window(window_id).ok()?.buffer_id;
        let scratch = BufferState::new(
            buffer_id,
            TextBuffer::from_text_with_kind(BufferKind::Untitled, ""),
        );
        if let Some(existing) = self.buffer_state_mut(buffer_id) {
            *existing = scratch;
        } else {
            self.buffers.push(scratch);
        }
        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = format!("{plugin_id}: {action_id}");
            window.kind = WindowKind::PluginScratch;
            window.buffer_kind = BufferKind::Untitled;
            window.collapsed = false;
        }
        self.workspace.focused = window_id;
        self.plugin_windows.record_opened(plugin_id, window_id);
        Some(window_id)
    }

    /// The whole text of the plugin's scratch buffer, if a scratch window is
    /// open — the blob an `execute` action submits.
    fn plugin_scratch_text(&self, plugin_id: &str) -> Option<String> {
        let window_id = self.plugin_scratch_window(plugin_id)?;
        let buffer_id = self.workspace.window(window_id).ok()?.buffer_id;
        Some(self.buffer_state(buffer_id)?.buffer.to_text())
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
