use crate::*;

impl AppState {
    pub(crate) fn handle_command(&mut self, command: &EditorCommand) {
        self.clear_active_menu();
        match command {
            EditorCommand::App(command) => self.handle_app_command(command),
            EditorCommand::Edit(command) => {
                if self.refuse_edit_in_collapsed_pane() {
                    return;
                }
                self.handle_edit_command(command);
            }
            EditorCommand::Window(command) => self.handle_window_command(command),
            EditorCommand::File(command) => self.handle_file_command(command),
        }
    }

    pub(crate) fn handle_key_stroke(&mut self, stroke: KeyStroke) -> bool {
        let had_pending_keys = !self.pending_keys.is_empty();
        self.pending_keys.push(stroke);
        let sequence = KeySequence {
            strokes: self.pending_keys.clone(),
        };

        if let Some(command) = self.shell.command_for_sequence(&sequence).cloned() {
            self.pending_keys.clear();
            self.handle_command(&command);
            return true;
        }

        if self.shell.keymap.has_sequence_prefix(&sequence) {
            return true;
        }

        self.pending_keys.clear();
        had_pending_keys
    }

    pub(crate) fn handle_selection_key_stroke(&mut self, stroke: KeyStroke) -> bool {
        if stroke.modifiers != KeyModifiers::SHIFT {
            return false;
        }

        match stroke.key {
            Key::PageUp => return self.extend_focused_page(-1),
            Key::PageDown => return self.extend_focused_page(1),
            _ => {}
        }

        let Some(buffer) = self.focused_buffer_mut() else {
            return false;
        };

        match stroke.key {
            Key::Left => buffer.buffer.extend_selection_left(),
            Key::Right => buffer.buffer.extend_selection_right(),
            Key::Up => buffer.buffer.extend_selection_up(),
            Key::Down => buffer.buffer.extend_selection_down(),
            Key::Home => buffer.buffer.extend_selection_to_line_start(),
            Key::End => buffer.buffer.extend_selection_to_line_end(),
            _ => false,
        }
    }

    pub(crate) fn handle_auxiliary_window_key_stroke(&mut self, stroke: KeyStroke) -> bool {
        if stroke.modifiers != KeyModifiers::NONE && stroke.modifiers != KeyModifiers::SHIFT {
            return false;
        }

        let Ok(window) = self.workspace.focused_window() else {
            return false;
        };

        match window.kind {
            WindowKind::SearchResults => match stroke.key {
                Key::Enter => {
                    self.jump_current_search_result();
                    true
                }
                Key::Char('n') | Key::Char('N') => {
                    let label =
                        ui_text::tr(&self.shell.catalog, ui_text::WINDOW_SEARCH_RESULTS_TITLE)
                            .to_string();
                    self.move_focused_numbered_aux_row(1, &label);
                    true
                }
                Key::Char('p') | Key::Char('P') => {
                    let label =
                        ui_text::tr(&self.shell.catalog, ui_text::WINDOW_SEARCH_RESULTS_TITLE)
                            .to_string();
                    self.move_focused_numbered_aux_row(-1, &label);
                    true
                }
                Key::Home => {
                    let label =
                        ui_text::tr(&self.shell.catalog, ui_text::WINDOW_SEARCH_RESULTS_TITLE)
                            .to_string();
                    self.focus_first_numbered_aux_row(&label);
                    true
                }
                Key::End => {
                    let label =
                        ui_text::tr(&self.shell.catalog, ui_text::WINDOW_SEARCH_RESULTS_TITLE)
                            .to_string();
                    self.focus_last_numbered_aux_row(&label);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub(crate) fn handle_auxiliary_enter_key_stroke(&mut self, stroke: KeyStroke) -> bool {
        if stroke.modifiers != KeyModifiers::NONE || stroke.key != Key::Enter {
            return false;
        }

        let Ok(window) = self.workspace.focused_window() else {
            return false;
        };

        match window.kind {
            WindowKind::SearchResults => {
                self.jump_current_search_result();
                true
            }
            _ => false,
        }
    }

    pub(crate) fn handle_app_command(&mut self, command: &AppCommand) {
        match command {
            AppCommand::ConfigDiagnostics => self.open_config_diagnostics_screen(),
            AppCommand::ConfigDiagnosticsClipboard => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Clipboard)
            }
            AppCommand::ConfigDiagnosticsFileDialogKeymap => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::FileDialogKeymap)
            }
            AppCommand::ConfigDiagnosticsInput => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Input)
            }
            AppCommand::ConfigDiagnosticsKeymap => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Keymap)
            }
            AppCommand::ConfigDiagnosticsLimits => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Limits)
            }
            AppCommand::ConfigDiagnosticsPaths => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Paths)
            }
            AppCommand::ConfigDiagnosticsSource => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Source)
            }
            AppCommand::ConfigDiagnosticsSummary => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Summary)
            }
            AppCommand::ConfigDiagnosticsTerminal => {
                self.jump_config_diagnostics_section(ConfigDiagnosticsSection::Terminal)
            }
            AppCommand::Help => self.open_help_screen(),
            AppCommand::ReloadConfig => self.reload_config(),
            AppCommand::RunCommand => self.start_prompt(PromptKind::RunCommand, String::new()),
            AppCommand::SearchResults => self.open_search_results_screen(),
            AppCommand::ShellEscape => self.request_runtime_action(RuntimeAction::ShellEscape),
            AppCommand::StatusHistory => self.open_status_history_screen(),
            AppCommand::Quit => {
                if self.confirm_any_dirty(PendingAction::Quit) {
                    return;
                }
                self.should_quit = true;
            }
            AppCommand::CommandLine => {
                self.start_prompt(PromptKind::CommandLine, String::new());
            }
        }
    }

    fn request_runtime_action(&mut self, action: RuntimeAction) {
        self.clear_active_menu();
        self.pending_keys.clear();
        self.prompt = None;
        self.file_dialog = None;
        self.buffer_switcher = None;
        self.runtime_action = Some(action);
        self.set_status(ui_text::tr(&self.shell.catalog, ui_text::STATUS_SHELL_ESCAPE).to_string());
    }

    pub(crate) fn take_runtime_action(&mut self) -> Option<RuntimeAction> {
        self.runtime_action.take()
    }

    pub(crate) fn reload_config(&mut self) {
        match load_config(&self.config_request) {
            Ok(loaded_config) => {
                let status = loaded_config.source.status_text();
                let i18n_diagnostic = self.apply_loaded_config(loaded_config);
                self.set_status(match i18n_diagnostic {
                    Some(diagnostic) => format!("{status}; {diagnostic}"),
                    None => status,
                });
            }
            Err(error) => self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_CONFIG_RELOAD_FAILED,
                &[&error.to_string()],
            )),
        }
    }

    fn apply_loaded_config(&mut self, loaded_config: LoadedConfig) -> Option<String> {
        self.highlighter = PluginHighlighter::from_entries(&loaded_config.config.plugins);
        self.pending_keys.clear();
        self.mouse_drag = None;
        self.clear_active_menu();
        self.shell = UiShell::from_config(&loaded_config.config, self.detected_profile);
        let loaded_catalog = load_ui_catalog(&loaded_config.source, self.shell.profile.encoding);
        self.shell.catalog = loaded_catalog.catalog;
        self.limits = loaded_config.config.limits;
        self.file_dialog_keys = loaded_config.config.file_dialog_keys.clone();
        self.clipboard = loaded_config.config.clipboard;
        self.mouse_enabled = loaded_config.config.mouse.enabled;
        self.plugin_status = loaded_config.config.plugin_status;
        self.config_source = loaded_config.source;
        self.refresh_help_buffer();
        self.refresh_config_diagnostics_buffer();
        loaded_catalog.diagnostic
    }

    pub(crate) fn handle_file_command(&mut self, command: &FileCommand) {
        match command {
            FileCommand::New => {
                if self.confirm_focused_dirty(PendingAction::New) {
                    return;
                }
                self.reset_focused_to_untitled();
            }
            FileCommand::Save => {
                if let Err(error) = self.save_focused_buffer() {
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_SAVE_FAILED,
                        &[&error.to_string()],
                    ));
                }
            }
            FileCommand::Open => {
                if self.confirm_focused_dirty(PendingAction::OpenPrompt) {
                    return;
                }
                self.start_file_dialog(FileDialogKind::Open, self.default_open_dialog_input());
            }
            FileCommand::SwitchBuffer => {
                self.start_buffer_switcher();
            }
            FileCommand::SaveAs => {
                let focused_path = self.focused_path_text();
                let input = if focused_path.is_empty() {
                    self.recent_file_dialog_input.clone().unwrap_or_default()
                } else {
                    focused_path
                };
                self.start_file_dialog(FileDialogKind::SaveAs, input);
            }
            FileCommand::Reload => {
                if self.confirm_focused_dirty(PendingAction::ReloadBuffer) {
                    return;
                }
                if let Err(error) = self.reload_focused_buffer() {
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_RELOAD_FAILED,
                        &[&error.to_string()],
                    ));
                }
            }
            FileCommand::Close => {
                self.close_focused_file();
            }
        }
    }
}
