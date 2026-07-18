use crate::*;

impl AppState {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::from_config(Config::default())
    }

    #[cfg(test)]
    pub(crate) fn from_config(config: Config) -> Self {
        Self::from_config_with_catalog(config, TextCatalog::empty())
    }

    #[cfg(test)]
    pub(crate) fn from_config_with_catalog(config: Config, catalog: TextCatalog) -> Self {
        let config_request = ConfigLoadRequest::new(None, true);
        let loaded_config = LoadedConfig {
            config,
            source: ConfigSource::Disabled,
        };
        let detected_profile = detect_terminal_profile();
        let mut shell = UiShell::from_config(&loaded_config.config, detected_profile);
        shell.catalog = catalog;
        Self::finish_loaded_config(config_request, loaded_config, detected_profile, shell, None)
    }

    pub(crate) fn from_loaded_config(
        config_request: ConfigLoadRequest,
        loaded_config: LoadedConfig,
    ) -> Self {
        let detected_profile = detect_terminal_profile();
        let mut shell = UiShell::from_config(&loaded_config.config, detected_profile);
        let loaded_catalog = load_ui_catalog(&loaded_config.source, shell.profile.encoding);
        shell.catalog = loaded_catalog.catalog;
        Self::finish_loaded_config(
            config_request,
            loaded_config,
            detected_profile,
            shell,
            loaded_catalog.diagnostic,
        )
    }

    fn finish_loaded_config(
        config_request: ConfigLoadRequest,
        loaded_config: LoadedConfig,
        detected_profile: TerminalProfile,
        shell: UiShell,
        catalog_diagnostic: Option<String>,
    ) -> Self {
        let limits = loaded_config.config.limits;
        let file_dialog_keys = loaded_config.config.file_dialog_keys.clone();
        let clipboard = loaded_config.config.clipboard;
        let mouse_enabled = loaded_config.config.mouse.enabled;
        let plugin_status = loaded_config.config.plugin_status;

        let plugin_hosts = PluginHosts::from_entries(&loaded_config.config.plugins);
        // Plugin menu labels resolve against the same locale chain as `dun`'s
        // own UI text, and stay English on ASCII terminals and with config
        // disabled (the sanitizer would only escape non-ASCII labels there).
        let plugin_menu_tags = match (&loaded_config.source, shell.profile.encoding) {
            (ConfigSource::Disabled, _) | (_, EncodingProfile::Ascii) => Vec::new(),
            _ => dun_config::locale_candidates(&locale_value().unwrap_or_default()),
        };
        let mut workspace = Workspace::new_untitled();
        let initial_window = workspace.focused;
        if let Ok(window) = workspace.window_mut(initial_window) {
            window.title = ui_text::tr(&shell.catalog, ui_text::WINDOW_UNTITLED).to_string();
        }

        let mut app = Self {
            plugin_hosts,
            workspace,
            buffers: vec![BufferState::new(BufferId(1), TextBuffer::new_untitled())],
            config_request,
            config_source: loaded_config.source,
            detected_profile,
            shell,
            limits,
            file_dialog_keys,
            clipboard,
            mouse_enabled,
            plugin_status,
            mouse_drag: None,
            active_menu: None,
            active_menu_entry: None,
            should_quit: false,
            workspace_area: Rect::default(),
            pending_keys: Vec::new(),
            status_message: None,
            prompt: None,
            file_dialog: None,
            buffer_switcher: None,
            confirm: None,
            replace_confirm: None,
            status_history: Vec::new(),
            command_history: Vec::new(),
            run_command_history: Vec::new(),
            last_find_query: None,
            pending_replace_query: None,
            search_results_source: None,
            kill_ring: None,
            recent_file_dialog_input: None,
            runtime_action: None,
            plugin_menu_tags,
            plugin_windows: PluginWindows::default(),
        };
        if let Some(diagnostic) = catalog_diagnostic {
            app.set_status(diagnostic);
        }
        app
    }

    #[cfg(test)]
    pub(crate) fn from_path(path: Option<PathBuf>) -> io::Result<Self> {
        let mut app = Self::new();
        if let Some(path) = path {
            app.open_file_path(path)?;
        }
        Ok(app)
    }

    #[cfg(test)]
    pub(crate) fn from_config_path(config: Config, path: Option<PathBuf>) -> io::Result<Self> {
        let mut app = Self::from_config(config);
        if let Some(path) = path {
            app.open_file_path(path)?;
        }
        Ok(app)
    }

    pub(crate) fn from_loaded_config_path(
        config_request: ConfigLoadRequest,
        loaded_config: LoadedConfig,
        path: Option<PathBuf>,
    ) -> io::Result<Self> {
        let mut app = Self::from_loaded_config(config_request, loaded_config);
        if let Some(path) = path {
            // Open reports itself, but on startup there is no user action to
            // report: the file is plainly on screen and its path is already in
            // the title and the right of the status bar. Start at rest so the
            // first frame shows the buffer readout rather than a truncated
            // "Opened /very/long/path". The message stays in the history. A
            // startup diagnostic (broken i18n file) must survive the open.
            let startup_status = app.status_message.take();
            app.open_file_path(path)?;
            app.status_message = startup_status;
        }
        Ok(app)
    }
}
