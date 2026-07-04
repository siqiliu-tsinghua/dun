#![forbid(unsafe_code)]

use dun_core::Workspace;
use dun_term::{GlyphSet, TerminalProfile, Theme};

#[derive(Clone, Debug)]
pub struct UiShell {
    pub profile: TerminalProfile,
    pub glyphs: GlyphSet,
    pub theme: Theme,
}

impl Default for UiShell {
    fn default() -> Self {
        Self {
            profile: TerminalProfile::default(),
            glyphs: GlyphSet::default(),
            theme: Theme::default(),
        }
    }
}

impl UiShell {
    pub fn describe_workspace(&self, workspace: &Workspace) -> String {
        format!(
            "theme={} windows={} border={}{}{}{}",
            self.theme.name,
            workspace.window_count(),
            self.glyphs.border.top_left,
            self.glyphs.border.horizontal,
            self.glyphs.border.horizontal,
            self.glyphs.border.top_right,
        )
    }
}
