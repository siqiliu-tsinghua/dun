use crate::*;
use dun_config::PluginStatusConfig;

pub(crate) struct AppState {
    pub(crate) workspace: Workspace,
    pub(crate) buffers: Vec<BufferState>,
    pub(crate) config_request: ConfigLoadRequest,
    pub(crate) config_source: ConfigSource,
    /// The installed configuration the user layer was applied on top of.
    pub(crate) installed_config: Option<PathBuf>,
    pub(crate) detected_profile: TerminalProfile,
    pub(crate) shell: UiShell,
    pub(crate) limits: Limits,
    pub(crate) file_dialog_keys: FileDialogKeymap,
    pub(crate) clipboard: ClipboardConfig,
    pub(crate) mouse_enabled: bool,
    pub(crate) plugin_status: PluginStatusConfig,
    pub(crate) mouse_drag: Option<MouseDragState>,
    pub(crate) active_menu: Option<usize>,
    pub(crate) active_menu_entry: Option<usize>,
    pub(crate) should_quit: bool,
    pub(crate) workspace_area: Rect,
    pub(crate) pending_keys: Vec<KeyStroke>,
    pub(crate) status_message: Option<String>,
    pub(crate) prompt: Option<PromptState>,
    pub(crate) file_dialog: Option<FileDialogState>,
    pub(crate) buffer_switcher: Option<BufferSwitcherState>,
    pub(crate) confirm: Option<ConfirmState>,
    pub(crate) replace_confirm: Option<ReplaceConfirmState>,
    pub(crate) status_history: Vec<StatusEntry>,
    pub(crate) command_history: Vec<String>,
    pub(crate) run_command_history: Vec<String>,
    pub(crate) last_find_query: Option<String>,
    pub(crate) pending_replace_query: Option<String>,
    pub(crate) search_results_source: Option<BufferId>,
    pub(crate) kill_ring: Option<String>,
    pub(crate) recent_file_dialog_input: Option<String>,
    pub(crate) runtime_action: Option<RuntimeAction>,
    /// Configured hosts plus the current plugin-menu rejection set, retained
    /// across refreshes so each newly rejected subtree is reported once.
    pub(crate) plugin_hosts: PluginHosts,
    /// Active locale chain used to resolve plugin-contributed menu labels
    /// (empty falls back to the required `en_US`, matching the English-on-ASCII
    /// and `--no-config` rules for `dun`'s own UI text).
    pub(crate) plugin_menu_tags: Vec<String>,
    /// Per-plugin ownership of the surface windows opened from plugin menus,
    /// mirrored against real `WindowId`s in the workspace.
    pub(crate) plugin_windows: PluginWindows,
}

impl AppState {
    #[cfg(test)]
    pub(crate) fn plugin_menu_rejections(&self) -> &[crate::plugins::PluginMenuRejection] {
        self.plugin_hosts.menu_rejections()
    }
}
