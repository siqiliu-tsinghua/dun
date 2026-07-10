use crate::*;

impl AppState {
    pub(crate) fn run_command_line(&mut self, input: &str) {
        let tokens = match parse_command_line(input) {
            Ok(tokens) => tokens,
            Err(error) => {
                self.set_status(format!(
                    "Command failed: {}",
                    command_line_parse_error_text(error)
                ));
                return;
            }
        };
        let Some((command, args)) = tokens.split_first() else {
            self.set_status("Command cancelled");
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
            "whitespace" => self.run_no_arg_command(
                args,
                EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace),
            ),
            "mark" | "bookmark" => {
                self.run_no_arg_command(args, EditorCommand::Edit(EditCommand::ToggleBookmark))
            }
            "find" => self.run_find_command(args),
            "replace" => self.run_replace_command(args),
            "goto" | "gotoline" | "line" => self.run_go_to_line_command(args),
            "commands" => self.set_status(COMMAND_LINE_HELP),
            _ => self.run_command_id_command(command, args),
        }
    }

    fn run_command_id_command(&mut self, command: &str, args: &[String]) {
        match command_from_id(command) {
            Ok(command) => self.run_no_arg_command(args, command),
            Err(_) => self.set_status(format!("Unknown command: {command}")),
        }
    }

    fn run_theme_command(&mut self, args: &[String]) {
        match args {
            [] => self.set_status(format!(
                "Theme: {} ({})",
                self.shell.theme.name,
                theme_command_values()
            )),
            [theme] => match parse_theme_command_value(theme) {
                Some(theme) => self.set_runtime_theme(theme),
                None => self.set_status(format!(
                    "Theme failed: unknown theme {theme}; expected {}",
                    theme_command_values()
                )),
            },
            _ => self.set_status("Command failed: theme expects zero or one theme name"),
        }
    }

    fn run_external_command_line(&mut self, args: &[String]) {
        match args {
            [] => self.handle_app_command(&AppCommand::RunCommand),
            [command] => self.run_external_command_to_buffer(command),
            _ => self.set_status("Command failed: run expects zero args or one quoted command"),
        }
    }

    fn run_config_diagnostics_command(&mut self, args: &[String]) {
        match args {
            [] => self.open_config_diagnostics_screen(),
            [section] => match parse_config_diagnostics_section(section) {
                Some(section) => self.jump_config_diagnostics_section(section),
                None => self.set_status(format!(
                    "Command failed: config expects one of {}",
                    config_diagnostics_section_values()
                )),
            },
            _ => self.set_status(format!(
                "Command failed: config expects zero args or one of {}",
                config_diagnostics_section_values()
            )),
        }
    }

    fn run_search_results_command(&mut self, args: &[String]) {
        match args {
            [] => self.open_search_results_screen(),
            [index] => self.jump_search_result(index),
            _ => self.set_status("Command failed: results expects zero args or one match number"),
        }
    }

    fn set_runtime_theme(&mut self, theme: ThemeName) {
        self.shell.theme = Theme::for_profile(theme, self.shell.profile);
        self.refresh_config_diagnostics_buffer();
        self.set_status(format!("Theme: {}", theme.as_str()));
    }

    pub(crate) fn run_open_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_file_command(&FileCommand::Open),
            [path] => {
                if self.focused_buffer_is_dirty() {
                    self.set_status("Open failed: focused buffer has unsaved changes");
                    return;
                }
                if let Err(error) = self.open_file_path(PathBuf::from(path)) {
                    self.set_status(format!("Open failed: {error}"));
                }
            }
            _ => self.set_status("Command failed: open expects zero or one path"),
        }
    }

    fn run_save_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_file_command(&FileCommand::Save),
            [path] => {
                if let Err(error) = self.save_focused_buffer_as(PathBuf::from(path)) {
                    self.set_status(format!("Save As failed: {error}"));
                }
            }
            _ => self.set_status("Command failed: save expects zero or one path"),
        }
    }

    fn run_save_as_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_file_command(&FileCommand::SaveAs),
            [path] => {
                if let Err(error) = self.save_focused_buffer_as(PathBuf::from(path)) {
                    self.set_status(format!("Save As failed: {error}"));
                }
            }
            _ => self.set_status("Command failed: save-as expects zero or one path"),
        }
    }

    fn run_no_arg_command(&mut self, args: &[String], command: EditorCommand) {
        if !args.is_empty() {
            self.set_status(format!(
                "Command failed: {} expects no arguments",
                command_id(&command)
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
            _ => self.set_status("Command failed: find expects zero or one query"),
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
                "Command failed: replace expects query and replacement, or all query replacement",
            ),
        }
    }

    fn run_go_to_line_command(&mut self, args: &[String]) {
        match args {
            [] => self.handle_edit_command(&EditCommand::GoToLine),
            [line] => self.go_to_line(line),
            _ => self.set_status("Command failed: go-to-line expects one line number"),
        }
    }
}
