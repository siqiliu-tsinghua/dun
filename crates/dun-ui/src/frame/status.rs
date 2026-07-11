use dun_core::Workspace;

use crate::{StatusBar, UiShell};

impl UiShell {
    pub(super) fn status_bar(&self, workspace: &Workspace, visible_windows: usize) -> StatusBar {
        StatusBar {
            left: format!("{} window(s)", visible_windows),
            right: format!("theme={} colors={:?}", self.theme.name, self.profile.colors),
            plugin: None,
            focused_window: workspace.focused,
        }
    }
}
