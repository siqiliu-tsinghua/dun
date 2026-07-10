use crate::*;

impl AppState {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::from_config(Config::default())
    }

    #[cfg(test)]
    pub(crate) fn from_config(config: Config) -> Self {
        Self::from_loaded_config(
            ConfigLoadRequest::new(None, true),
            LoadedConfig {
                config,
                source: ConfigSource::Disabled,
            },
        )
    }

    pub(crate) fn from_loaded_config(
        config_request: ConfigLoadRequest,
        loaded_config: LoadedConfig,
    ) -> Self {
        let detected_profile = detect_terminal_profile();
        let shell = UiShell::from_config(&loaded_config.config, detected_profile);
        let limits = loaded_config.config.limits;
        let file_dialog_keys = loaded_config.config.file_dialog_keys.clone();
        let clipboard = loaded_config.config.clipboard;
        let mouse_enabled = loaded_config.config.mouse.enabled;

        Self {
            workspace: Workspace::new_untitled(),
            buffers: vec![BufferState::new(BufferId(1), TextBuffer::new_untitled())],
            config_request,
            config_source: loaded_config.source,
            detected_profile,
            shell,
            limits,
            file_dialog_keys,
            clipboard,
            mouse_enabled,
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
        }
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
            app.open_file_path(path)?;
        }
        Ok(app)
    }
}
