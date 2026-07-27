use crate::*;

impl AppState {
    pub(crate) fn open_read_only_aux_window(
        &mut self,
        kind: WindowKind,
        title: &str,
        buffer: TextBuffer,
    ) {
        if let Some(window_id) = self
            .workspace
            .windows
            .iter()
            .find(|window| window.kind == kind)
            .map(|window| window.id)
        {
            self.workspace.focused = window_id;
            let Ok(window) = self.workspace.window(window_id) else {
                return;
            };
            let buffer_id = window.buffer_id;
            if let Some(state) = self.buffer_state_mut(buffer_id) {
                *state = BufferState::new(buffer_id, buffer);
            }
            if let Ok(window) = self.workspace.window_mut(window_id) {
                window.title = title.to_string();
                window.kind = kind;
                window.buffer_kind = BufferKind::ReadOnly;
                window.collapsed = false;
            }
            return;
        }

        let Ok(window_id) = self.workspace.split_focused(Axis::Horizontal) else {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_AUX_FOCUSED_WINDOW_MISSING,
                &[title],
            ));
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_AUX_WINDOW_MISSING,
                &[title],
            ));
            return;
        };
        let buffer_id = window.buffer_id;
        let state = BufferState::new(buffer_id, buffer);
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = state;
        } else {
            self.buffers.push(state);
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = title.to_string();
            window.kind = kind;
            window.buffer_kind = BufferKind::ReadOnly;
            window.collapsed = false;
        }
    }

    pub(crate) fn open_help_screen(&mut self) {
        if let Some(window_id) = self.help_window_id() {
            self.workspace.focused = window_id;
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_HELP_OPENED).to_string(),
            );
            return;
        }

        let Ok(window_id) = self.workspace.split_focused(Axis::Horizontal) else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_HELP_FOCUSED_WINDOW_MISSING,
                )
                .to_string(),
            );
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_HELP_WINDOW_MISSING).to_string(),
            );
            return;
        };
        let buffer_id = window.buffer_id;
        let help = BufferState::new(
            buffer_id,
            help_buffer(
                &self.shell.keymap,
                &self.file_dialog_keys,
                &self.shell.catalog,
                self.shell.profile.ambiguous_width,
            ),
        );

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = help;
        } else {
            self.buffers.push(help);
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = ui_text::tr(&self.shell.catalog, ui_text::WINDOW_HELP_TITLE).to_string();
            window.kind = WindowKind::Help;
            window.buffer_kind = BufferKind::ReadOnly;
            window.collapsed = false;
        }

        self.set_status(ui_text::tr(&self.shell.catalog, ui_text::STATUS_HELP_OPENED).to_string());
    }

    fn help_window_id(&self) -> Option<WindowId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::Help)
            .map(|window| window.id)
    }

    fn help_buffer_id(&self) -> Option<BufferId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::Help)
            .map(|window| window.buffer_id)
    }

    pub(crate) fn refresh_help_buffer(&mut self) {
        let Some(buffer_id) = self.help_buffer_id() else {
            return;
        };
        let help = BufferState::new(
            buffer_id,
            help_buffer(
                &self.shell.keymap,
                &self.file_dialog_keys,
                &self.shell.catalog,
                self.shell.profile.ambiguous_width,
            ),
        );

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = help;
        }
    }

    pub(crate) fn open_config_diagnostics_screen(&mut self) {
        self.set_status(
            ui_text::tr(
                &self.shell.catalog,
                ui_text::STATUS_CONFIG_DIAGNOSTICS_OPENED,
            )
            .to_string(),
        );

        if let Some(window_id) = self.config_diagnostics_window_id() {
            self.workspace.focused = window_id;
            self.refresh_config_diagnostics_buffer();
            return;
        }

        let Ok(window_id) = self.workspace.split_focused(Axis::Horizontal) else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_CONFIG_DIAGNOSTICS_FOCUSED_WINDOW_MISSING,
                )
                .to_string(),
            );
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_CONFIG_DIAGNOSTICS_WINDOW_MISSING,
                )
                .to_string(),
            );
            return;
        };
        let buffer_id = window.buffer_id;
        let text = self.config_diagnostics_text();
        let diagnostics = BufferState::new(buffer_id, config_diagnostics_buffer(&text));

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = diagnostics;
        } else {
            self.buffers.push(diagnostics);
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = ui_text::tr(
                &self.shell.catalog,
                ui_text::WINDOW_CONFIG_DIAGNOSTICS_TITLE,
            )
            .to_string();
            window.kind = WindowKind::ConfigDiagnostics;
            window.buffer_kind = BufferKind::ReadOnly;
            window.collapsed = false;
        }
    }

    fn config_diagnostics_window_id(&self) -> Option<WindowId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::ConfigDiagnostics)
            .map(|window| window.id)
    }

    fn config_diagnostics_buffer_id(&self) -> Option<BufferId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::ConfigDiagnostics)
            .map(|window| window.buffer_id)
    }

    pub(crate) fn refresh_config_diagnostics_buffer(&mut self) {
        let Some(buffer_id) = self.config_diagnostics_buffer_id() else {
            return;
        };
        let text = self.config_diagnostics_text();
        let diagnostics = BufferState::new(buffer_id, config_diagnostics_buffer(&text));

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = diagnostics;
        }
    }

    pub(crate) fn jump_config_diagnostics_section(&mut self, section: ConfigDiagnosticsSection) {
        self.open_config_diagnostics_screen();
        let Some(window_id) = self.config_diagnostics_window_id() else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_CONFIG_DIAGNOSTICS_WINDOW_MISSING,
                )
                .to_string(),
            );
            return;
        };
        let Some(buffer_id) = self.config_diagnostics_buffer_id() else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_CONFIG_DIAGNOSTICS_BUFFER_MISSING,
                )
                .to_string(),
            );
            return;
        };
        let Some(line_index) = self.buffer_state(buffer_id).and_then(|buffer| {
            line_with_exact_text(&buffer.buffer, section.heading(&self.shell.catalog))
        }) else {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_CONFIG_DIAGNOSTICS_SECTION_NOT_FOUND,
                &[section.label()],
            ));
            return;
        };

        self.workspace.focused = window_id;
        let context = self
            .focused_buffer_view_context(self.workspace_area)
            .unwrap_or(BufferViewContext {
                buffer_id,
                body_height: 1,
                body_width: 1,
            });
        let display = self.shell.editor_text_display(false);
        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            let _ = buffer.buffer.set_cursor(Position::new(line_index, 0));
            buffer.ensure_cursor_visible(context.body_height, context.body_width, display);
        }
        self.set_status(ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_CONFIG_DIAGNOSTICS_SECTION,
            &[section.label()],
        ));
    }

    pub(crate) fn open_status_history_screen(&mut self) {
        self.set_status(
            ui_text::tr(&self.shell.catalog, ui_text::STATUS_HISTORY_OPENED).to_string(),
        );

        if let Some(window_id) = self.status_history_window_id() {
            self.workspace.focused = window_id;
            return;
        }

        let Ok(window_id) = self.workspace.split_focused(Axis::Horizontal) else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_HISTORY_FOCUSED_WINDOW_MISSING,
                )
                .to_string(),
            );
            return;
        };
        let Ok(window) = self.workspace.window(window_id) else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_HISTORY_WINDOW_MISSING)
                    .to_string(),
            );
            return;
        };
        let buffer_id = window.buffer_id;
        let text = self.status_history_text();

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = BufferState::new(buffer_id, status_history_buffer(&text));
        } else {
            self.buffers
                .push(BufferState::new(buffer_id, status_history_buffer(&text)));
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title =
                ui_text::tr(&self.shell.catalog, ui_text::WINDOW_STATUS_HISTORY_TITLE).to_string();
            window.kind = WindowKind::StatusHistory;
            window.buffer_kind = BufferKind::ReadOnly;
            window.collapsed = false;
        }
    }

    fn status_history_window_id(&self) -> Option<WindowId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::StatusHistory)
            .map(|window| window.id)
    }

    pub(crate) fn status_history_buffer_id(&self) -> Option<BufferId> {
        self.workspace
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::StatusHistory)
            .map(|window| window.buffer_id)
    }

    pub(crate) fn status_history_text(&self) -> String {
        let mut out =
            ui_text::tr(&self.shell.catalog, ui_text::WINDOW_STATUS_HISTORY_HEADING).to_string();
        out.push_str("\n\n");
        if self.status_history.is_empty() {
            out.push_str(ui_text::tr(
                &self.shell.catalog,
                ui_text::WINDOW_STATUS_HISTORY_EMPTY,
            ));
            out.push('\n');
            return out;
        }

        for (index, entry) in self.status_history.iter().enumerate() {
            let level_key = match entry.level {
                StatusLevel::Info => ui_text::WINDOW_STATUS_HISTORY_LEVEL_INFO,
                StatusLevel::Error => ui_text::WINDOW_STATUS_HISTORY_LEVEL_ERROR,
            };
            let level = ui_text::tr(&self.shell.catalog, level_key);
            out.push_str(&format!(
                "{:>3}. [{}] {}\n",
                index + 1,
                level,
                entry.message
            ));
        }

        out
    }

    fn config_diagnostics_text(&self) -> String {
        let mut out = ui_text::tr(
            &self.shell.catalog,
            ui_text::WINDOW_CONFIG_DIAGNOSTICS_HEADING,
        )
        .to_string();
        out.push_str("\n\n");
        let important_unbound = important_config_diagnostic_commands()
            .iter()
            .filter(|command| self.shell.keymap.sequence_for_command(command).is_none())
            .map(command_id)
            .collect::<Vec<_>>();
        let important_unbound_text = if important_unbound.is_empty() {
            "none".to_string()
        } else {
            important_unbound.join(", ")
        };

        out.push_str(ConfigDiagnosticsSection::Summary.heading(&self.shell.catalog));
        out.push('\n');
        out.push_str(&format!(
            "  config: {}\n",
            self.config_source.diagnostics_text()
        ));
        out.push_str(&format!(
            "  request: {}\n",
            self.config_request.diagnostics_text()
        ));
        out.push_str(&format!(
            "  terminal: {}\n",
            terminal_profile_status(self.shell.profile)
        ));
        out.push_str(&format!(
            "  theme: {} ({})\n",
            self.shell.theme.name,
            color_status(self.shell.theme.colors)
        ));
        out.push_str(&format!(
            "  mouse: {}\n",
            if self.mouse_enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));
        out.push_str(&format!(
            "  osc52_write: {}, osc52_read: {} (max {} bytes)\n",
            if self.clipboard.osc52.enabled {
                "enabled"
            } else {
                "disabled"
            },
            if self.clipboard.osc52.allow_read {
                "enabled"
            } else {
                "disabled"
            },
            self.clipboard.osc52.max_bytes
        ));
        out.push_str(&format!(
            "  keymap: {} bindings, important_unbound: {}\n",
            self.shell.keymap.bindings.len(),
            important_unbound_text
        ));

        out.push('\n');
        out.push_str(ConfigDiagnosticsSection::Paths.heading(&self.shell.catalog));
        out.push('\n');
        out.push_str(&format!("  {DUN_CONFIG_ENV}: {}\n", env_config_path_text()));
        out.push_str(&format!("  default path: {}\n", default_config_path_text()));
        out.push_str("  defaults: dun --dump-config\n");

        out.push('\n');
        out.push_str(ConfigDiagnosticsSection::Source.heading(&self.shell.catalog));
        out.push('\n');
        out.push_str(&format!(
            "  active: {}\n",
            self.config_source.diagnostics_text()
        ));
        out.push_str(&format!(
            "  request: {}\n",
            self.config_request.diagnostics_text()
        ));

        out.push('\n');
        out.push_str(ConfigDiagnosticsSection::Terminal.heading(&self.shell.catalog));
        out.push('\n');
        out.push_str(&format!(
            "  detected: {}\n",
            terminal_profile_status(self.detected_profile)
        ));
        out.push_str(&format!(
            "  effective: {}\n",
            terminal_profile_status(self.shell.profile)
        ));
        out.push_str(&format!(
            "  theme: {} ({})\n",
            self.shell.theme.name,
            color_status(self.shell.theme.colors)
        ));
        out.push_str(&format!(
            "  glyphs: {}\n",
            if self.shell.profile.supports_unicode_glyphs() {
                "unicode"
            } else {
                "ascii"
            }
        ));

        out.push('\n');
        out.push_str(ConfigDiagnosticsSection::Input.heading(&self.shell.catalog));
        out.push('\n');
        out.push_str(&format!(
            "  mouse: {}\n",
            if self.mouse_enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));

        out.push('\n');
        out.push_str(ConfigDiagnosticsSection::Clipboard.heading(&self.shell.catalog));
        out.push('\n');
        out.push_str(&format!(
            "  osc52_write: {}\n",
            if self.clipboard.osc52.enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));
        out.push_str(&format!(
            "  osc52_read: {}\n",
            if self.clipboard.osc52.allow_read {
                "enabled"
            } else {
                "disabled"
            }
        ));
        out.push_str(&format!(
            "  osc52_max_bytes: {}\n",
            self.clipboard.osc52.max_bytes
        ));

        out.push('\n');
        out.push_str(ConfigDiagnosticsSection::Limits.heading(&self.shell.catalog));
        out.push('\n');
        out.push_str(&format!(
            "  editable_file_soft_limit_bytes: {}\n",
            self.limits.editable_file_soft_limit_bytes
        ));
        out.push_str(&format!(
            "  line_display_soft_limit_bytes: {}\n",
            self.limits.line_display_soft_limit_bytes
        ));

        out.push('\n');
        out.push_str(ConfigDiagnosticsSection::Keymap.heading(&self.shell.catalog));
        out.push('\n');
        out.push_str(&format!(
            "  bindings: {}\n",
            self.shell.keymap.bindings.len()
        ));
        out.push_str(&format!("  important_unbound: {important_unbound_text}\n"));
        let mut bindings = self
            .shell
            .keymap
            .bindings
            .iter()
            .map(|binding| (command_id(&binding.command), binding.sequence.to_string()))
            .collect::<Vec<_>>();
        bindings.sort_by(|left, right| left.0.cmp(right.0));
        for (command, sequence) in bindings {
            out.push_str(&format!("  {command:<28} {sequence}\n"));
        }

        out.push('\n');
        out.push_str(ConfigDiagnosticsSection::FileDialogKeymap.heading(&self.shell.catalog));
        out.push('\n');
        out.push_str(&format!(
            "  bindings: {}\n",
            self.file_dialog_keys.bindings.len()
        ));
        let mut bindings = self
            .file_dialog_keys
            .bindings
            .iter()
            .map(|binding| {
                (
                    file_dialog_action_id(binding.action),
                    binding.stroke.to_string(),
                )
            })
            .collect::<Vec<_>>();
        bindings.sort_by(|left, right| left.0.cmp(right.0));
        for (action, stroke) in bindings {
            out.push_str(&format!("  {action:<28} {stroke}\n"));
        }

        out
    }
}
