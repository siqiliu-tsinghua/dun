use dun_config::{Config, KeySequence, KeyStroke, Keymap};
use dun_core::{DisplaySanitizer, EditorCommand, Workspace};
use dun_term::{EncodingProfile, GlyphSet, TerminalProfile, Theme};

#[derive(Clone, Debug)]
pub struct UiShell {
    pub profile: TerminalProfile,
    pub glyphs: GlyphSet,
    pub theme: Theme,
    pub keymap: Keymap,
    pub display_sanitizer: DisplaySanitizer,
}

impl UiShell {
    pub fn from_config(config: &Config, detected_profile: TerminalProfile) -> Self {
        let profile = config.terminal_profile(detected_profile);
        let glyphs = GlyphSet::for_profile(profile);
        let theme = config.resolved_theme(detected_profile);
        let display_sanitizer = DisplaySanitizer {
            ascii_only: matches!(profile.encoding, EncodingProfile::Ascii),
            max_bytes: config.limits.line_display_soft_limit_bytes,
        };

        Self {
            profile,
            glyphs,
            theme,
            keymap: config.keybindings.clone(),
            display_sanitizer,
        }
    }

    pub fn command_for_sequence(&self, sequence: &KeySequence) -> Option<&EditorCommand> {
        self.keymap.command_for_sequence(sequence)
    }

    pub fn command_for_stroke(&self, stroke: KeyStroke) -> Option<&EditorCommand> {
        self.keymap.command_for_stroke(stroke)
    }

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

impl Default for UiShell {
    fn default() -> Self {
        Self::from_config(&Config::default(), TerminalProfile::default())
    }
}
