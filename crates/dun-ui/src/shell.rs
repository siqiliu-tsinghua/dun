use dun_config::{Config, KeySequence, KeyStroke, Keymap, TextCatalog};
use dun_core::{DisplaySanitizer, EditorCommand, Workspace};
use dun_term::{EncodingProfile, GlyphSet, TerminalProfile, Theme, char_width};

use crate::MenuItem;

#[derive(Clone, Debug)]
pub struct UiShell {
    pub profile: TerminalProfile,
    pub glyphs: GlyphSet,
    pub theme: Theme,
    pub keymap: Keymap,
    pub display_sanitizer: DisplaySanitizer,
    /// Loaded UI translations; empty means built-in English. Loading is
    /// the caller's job (rendering stays free of file I/O).
    pub catalog: TextCatalog,
    /// Plugin-contributed top-level menus, already resolved to display
    /// labels and `PluginAction` commands by the caller (dun-cli owns
    /// the locale and the `dun-plugin` types). `menu_bar` appends these
    /// after the built-in menus, so rendering, hit testing, and keyboard
    /// dispatch all see one consistent menu list.
    pub plugin_menu_items: Vec<MenuItem>,
    /// Plugin-contributed keybindings (`[leader, chord] -> PluginAction`),
    /// resolved and collision-checked by the caller. The event loop consults
    /// this after the built-in keymap, so a plugin leader can never shadow a
    /// built-in binding.
    pub plugin_keymap: Keymap,
    /// Plugin ids whose keybinding contribution was rejected (leader collision
    /// or unparseable). Tracked so the caller can report a newly-rejected
    /// binding once rather than every frame.
    pub plugin_keybinding_rejections: Vec<String>,
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
            catalog: TextCatalog::empty(),
            plugin_menu_items: Vec::new(),
            plugin_keymap: Keymap::empty(),
            plugin_keybinding_rejections: Vec::new(),
        }
    }

    pub fn command_for_sequence(&self, sequence: &KeySequence) -> Option<&EditorCommand> {
        self.keymap.command_for_sequence(sequence)
    }

    pub fn command_for_stroke(&self, stroke: KeyStroke) -> Option<&EditorCommand> {
        self.keymap.command_for_stroke(stroke)
    }

    pub fn border_columns(&self) -> u16 {
        u16::try_from(
            char_width(self.glyphs.border.vertical, self.profile.ambiguous_width).unwrap_or(1),
        )
        .unwrap_or(1)
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
