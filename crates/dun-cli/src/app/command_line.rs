use crate::*;

impl AppState {
    pub(crate) fn run_command_line(&mut self, input: &str) {
        let tokens = match parse_command_line(input) {
            Ok(tokens) => tokens,
            Err(error) => {
                let detail = command_line_parse_error_text(&self.shell.catalog, error);
                let status = ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_COMMAND_PARSE_FAILED,
                    &[detail],
                );
                self.set_status(status);
                return;
            }
        };
        let Some((command, args)) = tokens.split_first() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_COMMAND_CANCELLED).to_string(),
            );
            return;
        };

        match normalize_command_line_token(command).as_str() {
            "help" | "h" | "?" => self.open_help_screen(),
            "config" | "diagnostics" | "configdiagnostics" => {
                self.run_config_diagnostics_command(args)
            }
            "reload" | "reloadconfig" => self.reload_config(),
            "status" | "statushistory" => self.open_status_history_screen(),
            "theme" => self.run_theme_command(args),
            "plugin" => self.run_plugin_command(args),
            "quit" | "q" => self.handle_app_command(&AppCommand::Quit),
            "shell" | "sh" => {
                self.run_no_arg_command(args, EditorCommand::App(AppCommand::ShellEscape))
            }
            "run" | "command" => self.run_external_command_line(args),
            "open" | "o" => self.run_open_command(args),
            "results" | "searchresults" | "matches" => self.run_search_results_command(args),
            "buffers" | "switch" | "switchbuffer" => {
                self.run_no_arg_command(args, EditorCommand::File(FileCommand::SwitchBuffer))
            }
            "save" | "write" | "w" => self.run_save_command(args),
            "saveas" | "writeas" => self.run_save_as_command(args),
            "new" => self.run_no_arg_command(args, EditorCommand::File(FileCommand::New)),
            "reloadfile" => self.run_no_arg_command(args, EditorCommand::File(FileCommand::Reload)),
            "close" => self.run_no_arg_command(args, EditorCommand::File(FileCommand::Close)),
            "wrap" => {
                self.run_no_arg_command(args, EditorCommand::Edit(EditCommand::ToggleWordWrap))
            }
            "find" => self.run_find_command(args),
            "replace" => self.run_replace_command(args),
            "goto" | "gotoline" | "line" => self.run_go_to_line_command(args),
            "commands" => {
                let status = command_line_help(&self.shell.catalog);
                self.set_status(status);
            }
            _ => self.run_command_id_command(command, args),
        }
    }

    fn run_command_id_command(&mut self, command: &str, args: &[String]) {
        match command_from_id(command) {
            Ok(command) => self.run_no_arg_command(args, command),
            Err(_) => self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_COMMAND_UNKNOWN,
                &[command],
            )),
        }
    }

    fn run_theme_command(&mut self, args: &[String]) {
        match args {
            [] => self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_THEME_CURRENT,
                &[self.shell.theme.name, theme_command_values()],
            )),
            [theme] => match parse_theme_command_value(theme) {
                Some(theme) => self.set_runtime_theme(theme),
                None => self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_THEME_UNKNOWN,
                    &[theme, theme_command_values()],
                )),
            },
            _ => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_COMMAND_THEME_ARITY).to_string(),
            ),
        }
    }

    fn run_plugin_command(&mut self, args: &[String]) {
        match args {
            [] => {
                let Some(highlighter) = self.highlighter.as_ref() else {
                    self.set_status(
                        ui_text::tr(&self.shell.catalog, ui_text::STATUS_PLUGIN_NOT_CONFIGURED)
                            .to_string(),
                    );
                    return;
                };
                let plugin_id = highlighter.plugin_id().to_string();
                let key = if highlighter.is_loaded() {
                    ui_text::STATUS_PLUGIN_IS_LOADED
                } else {
                    ui_text::STATUS_PLUGIN_IS_UNLOADED
                };
                self.set_status(ui_text::tr_fmt(&self.shell.catalog, key, &[&plugin_id]));
            }
            [action] if action == "unload" => {
                let Some(highlighter) = self.highlighter.as_mut() else {
                    self.set_status(
                        ui_text::tr(&self.shell.catalog, ui_text::STATUS_PLUGIN_NOT_CONFIGURED)
                            .to_string(),
                    );
                    return;
                };
                let plugin_id = highlighter.plugin_id().to_string();
                highlighter.unload();
                self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_PLUGIN_UNLOADED,
                    &[&plugin_id],
                ));
            }
            [action] if action == "load" => {
                let Some(highlighter) = self.highlighter.as_mut() else {
                    self.set_status(
                        ui_text::tr(&self.shell.catalog, ui_text::STATUS_PLUGIN_NOT_CONFIGURED)
                            .to_string(),
                    );
                    return;
                };
                let plugin_id = highlighter.plugin_id().to_string();
                highlighter.load();
                self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_PLUGIN_LOADED,
                    &[&plugin_id],
                ));
            }
            _ => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_PLUGIN_USAGE).to_string(),
            ),
        }
    }

    fn run_external_command_line(&mut self, args: &[String]) {
        match args {
            [] => self.handle_app_command(&AppCommand::RunCommand),
            [command] => self.run_external_command_to_buffer(command),
            _ => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_COMMAND_RUN_ARITY).to_string(),
            ),
        }
    }

    fn run_config_diagnostics_command(&mut self, args: &[String]) {
        match args {
            [] => self.open_config_diagnostics_screen(),
            [section] => match parse_config_diagnostics_section(section) {
                Some(section) => self.jump_config_diagnostics_section(section),
                None => self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_COMMAND_CONFIG_SECTION,
                    &[config_diagnostics_section_values()],
                )),
            },
            _ => self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_COMMAND_CONFIG_SECTION_ARITY,
                &[config_diagnostics_section_values()],
            )),
        }
    }

    fn run_search_results_command(&mut self, args: &[String]) {
        match args {
            [] => self.open_search_results_screen(),
            [index] => self.jump_search_result(index),
            _ => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_COMMAND_RESULTS_ARITY).to_string(),
            ),
        }
    }

    fn set_runtime_theme(&mut self, theme: ThemeName) {
        self.shell.theme = Theme::for_profile(theme, self.shell.profile);
        self.refresh_config_diagnostics_buffer();
        self.set_status(ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_THEME_CHANGED,
            &[theme.as_str()],
        ));
    }

    pub(crate) fn run_open_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_file_command(&FileCommand::Open),
            [path] => {
                if self.focused_buffer_is_dirty() {
                    self.set_status(
                        ui_text::tr(&self.shell.catalog, ui_text::STATUS_OPEN_DIRTY).to_string(),
                    );
                    return;
                }
                if let Err(error) = self.open_file_path(PathBuf::from(path)) {
                    let detail = path_error_status_text(&self.shell.catalog, &error);
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_OPEN_FAILED,
                        &[&detail],
                    ));
                }
            }
            _ => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_COMMAND_OPEN_ARITY).to_string(),
            ),
        }
    }

    fn run_save_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_file_command(&FileCommand::Save),
            [path] => {
                if let Err(error) = self.save_focused_buffer_as(PathBuf::from(path)) {
                    let detail = path_error_status_text(&self.shell.catalog, &error);
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_SAVE_AS_FAILED,
                        &[&detail],
                    ));
                }
            }
            _ => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_COMMAND_SAVE_ARITY).to_string(),
            ),
        }
    }

    fn run_save_as_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_file_command(&FileCommand::SaveAs),
            [path] => {
                if let Err(error) = self.save_focused_buffer_as(PathBuf::from(path)) {
                    let detail = path_error_status_text(&self.shell.catalog, &error);
                    self.set_status(ui_text::tr_fmt(
                        &self.shell.catalog,
                        ui_text::STATUS_SAVE_AS_FAILED,
                        &[&detail],
                    ));
                }
            }
            _ => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_COMMAND_SAVE_AS_ARITY).to_string(),
            ),
        }
    }

    fn run_no_arg_command(&mut self, args: &[String], command: EditorCommand) {
        if !args.is_empty() {
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_COMMAND_NO_ARGUMENTS,
                &[command_id(&command)],
            ));
            return;
        }

        self.handle_command(&command);
    }

    fn run_find_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_edit_command(&EditCommand::Find),
            [query] => {
                self.last_find_query = Some(query.clone());
                self.find_in_focused_buffer(SearchSpec::parse(query), SearchDirection::Forward);
            }
            _ => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_COMMAND_FIND_ARITY).to_string(),
            ),
        }
    }

    fn run_replace_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_edit_command(&EditCommand::Replace),
            [mode, query, replacement] if normalize_command_line_token(mode) == "all" => {
                self.last_find_query = Some(query.clone());
                self.replace_all_in_focused_buffer(SearchSpec::parse(query), replacement);
            }
            [query, replacement] => {
                self.last_find_query = Some(query.clone());
                self.replace_in_focused_buffer(SearchSpec::parse(query), replacement);
            }
            _ => self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_COMMAND_REPLACE_ARITY).to_string(),
            ),
        }
    }

    fn run_go_to_line_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_edit_command(&EditCommand::GoToLine),
            [line] => self.go_to_line(line),
            _ => self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_COMMAND_GO_TO_LINE_ARITY,
                )
                .to_string(),
            ),
        }
    }
}
