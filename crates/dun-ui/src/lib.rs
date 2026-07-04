#![forbid(unsafe_code)]

use dun_config::{Config, KeySequence, KeyStroke, Keymap};
use dun_core::{
    BufferId, DisplaySanitizer, EditorCommand, Rect, SanitizedLine, TextBuffer, WindowId,
    WindowState, Workspace,
};
use dun_term::{BorderGlyphs, EncodingProfile, GlyphSet, TerminalProfile, Theme};

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

    pub fn frame_for_workspace(
        &self,
        workspace: &Workspace,
        area: Rect,
        buffers: &[BufferView<'_>],
    ) -> UiFrame {
        let mut windows = Vec::new();

        for layout in workspace.resolved_layout(area) {
            if let Ok(window) = workspace.window(layout.id) {
                windows.push(self.window_model(window, layout.rect, workspace.focused, buffers));
            }
        }

        UiFrame {
            menu: self.menu_bar(),
            status: self.status_bar(workspace, windows.len()),
            windows,
        }
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

    fn window_model(
        &self,
        window: &WindowState,
        rect: Rect,
        focused: WindowId,
        buffers: &[BufferView<'_>],
    ) -> UiWindow {
        let buffer = buffers.iter().find(|buffer| buffer.id == window.buffer_id);
        let body = match (window.collapsed, buffer) {
            (true, _) => Vec::new(),
            (false, Some(buffer)) => self.sanitize_buffer_body(buffer, rect),
            (false, None) => vec![self.display_sanitizer.sanitize_line("[missing buffer]")],
        };

        UiWindow {
            id: window.id,
            buffer_id: window.buffer_id,
            title: window.title.clone(),
            rect,
            focused: window.id == focused,
            collapsed: window.collapsed,
            dirty: buffer
                .map(|buffer| buffer.buffer.is_dirty())
                .unwrap_or(false),
            read_only: buffer
                .map(|buffer| buffer.buffer.is_read_only())
                .unwrap_or(matches!(window.buffer_kind, dun_core::BufferKind::ReadOnly)),
            border: self.glyphs.border,
            body,
        }
    }

    fn sanitize_buffer_body(&self, buffer: &BufferView<'_>, rect: Rect) -> Vec<SanitizedLine> {
        let body_height = rect.height.saturating_sub(2) as usize;
        if body_height == 0 {
            return Vec::new();
        }

        let mut lines = Vec::new();
        for line_index in buffer.first_line..buffer.buffer.line_count() {
            if lines.len() >= body_height {
                break;
            }

            let line = buffer.buffer.line(line_index).unwrap_or_default();
            lines.push(self.display_sanitizer.sanitize_line(line));
        }

        lines
    }

    fn menu_bar(&self) -> MenuBar {
        MenuBar {
            items: vec![
                MenuItem::new("New", EditorCommand::File(dun_core::FileCommand::New)),
                MenuItem::new("Open", EditorCommand::File(dun_core::FileCommand::Open)),
                MenuItem::new("Save", EditorCommand::File(dun_core::FileCommand::Save)),
                MenuItem::new("Find", EditorCommand::Edit(dun_core::EditCommand::Find)),
                MenuItem::new(
                    "Split",
                    EditorCommand::Window(dun_core::WindowCommand::SplitHorizontal),
                ),
                MenuItem::new("Help", EditorCommand::App(dun_core::AppCommand::Help)),
                MenuItem::new("Quit", EditorCommand::App(dun_core::AppCommand::Quit)),
            ],
        }
    }

    fn status_bar(&self, workspace: &Workspace, visible_windows: usize) -> StatusBar {
        StatusBar {
            left: format!("{} window(s)", visible_windows),
            right: format!("theme={} colors={:?}", self.theme.name, self.profile.colors),
            focused_window: workspace.focused,
        }
    }
}

impl Default for UiShell {
    fn default() -> Self {
        Self::from_config(&Config::default(), TerminalProfile::default())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BufferView<'a> {
    pub id: BufferId,
    pub buffer: &'a TextBuffer,
    pub first_line: usize,
}

impl<'a> BufferView<'a> {
    pub const fn new(id: BufferId, buffer: &'a TextBuffer) -> Self {
        Self {
            id,
            buffer,
            first_line: 0,
        }
    }

    pub const fn scrolled(id: BufferId, buffer: &'a TextBuffer, first_line: usize) -> Self {
        Self {
            id,
            buffer,
            first_line,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiFrame {
    pub menu: MenuBar,
    pub status: StatusBar,
    pub windows: Vec<UiWindow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuBar {
    pub items: Vec<MenuItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    pub label: &'static str,
    pub command: EditorCommand,
}

impl MenuItem {
    pub const fn new(label: &'static str, command: EditorCommand) -> Self {
        Self { label, command }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusBar {
    pub left: String,
    pub right: String,
    pub focused_window: WindowId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiWindow {
    pub id: WindowId,
    pub buffer_id: BufferId,
    pub title: String,
    pub rect: Rect,
    pub focused: bool,
    pub collapsed: bool,
    pub dirty: bool,
    pub read_only: bool,
    pub border: BorderGlyphs,
    pub body: Vec<SanitizedLine>,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use dun_config::{ColorProfile, EncodingProfile, KeySequence, TerminalOverrides};
    use dun_core::{AppCommand, Axis, BufferKind, FileCommand, Position};

    use super::*;

    #[test]
    fn shell_applies_configured_terminal_fallbacks() {
        let config = Config {
            terminal: TerminalOverrides {
                encoding: Some(EncodingProfile::Ascii),
                colors: Some(ColorProfile::Color16),
            },
            ..Config::default()
        };

        let shell = UiShell::from_config(&config, TerminalProfile::default());

        assert_eq!(shell.profile, TerminalProfile::ascii_16());
        assert_eq!(shell.glyphs, GlyphSet::ascii());
        assert_eq!(shell.theme.colors, ColorProfile::Color16);
        assert!(shell.display_sanitizer.ascii_only);
    }

    #[test]
    fn shell_resolves_keymap_commands() {
        let shell = UiShell::default();
        let sequence = KeySequence::from_str("Ctrl+S").unwrap();

        assert_eq!(
            shell.command_for_sequence(&sequence),
            Some(&EditorCommand::File(FileCommand::Save))
        );
    }

    #[test]
    fn frame_contains_menu_status_and_sanitized_buffer_content() {
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "safe\x1b]0;x\x07");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        let shell = UiShell::default();

        let frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

        assert_eq!(frame.menu.items[0].label, "New");
        assert_eq!(frame.status.focused_window, WindowId(1));
        assert_eq!(frame.windows.len(), 1);
        assert_eq!(frame.windows[0].body[0].as_plain_text(), "safe␛]0;x␇");
        assert!(frame.windows[0].body[0].has_non_text_segments());
    }

    #[test]
    fn frame_uses_tiled_workspace_rectangles() {
        let mut workspace = Workspace::new_untitled();
        workspace.split_focused(Axis::Horizontal).unwrap();

        let first = TextBuffer::from_text_with_kind(BufferKind::Untitled, "left");
        let second = TextBuffer::from_text_with_kind(BufferKind::Untitled, "right");
        let buffers = [
            BufferView::new(BufferId(1), &first),
            BufferView::new(BufferId(2), &second),
        ];

        let frame =
            UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 20), &buffers);

        assert_eq!(frame.windows.len(), 2);
        assert_eq!(frame.windows[0].rect, Rect::new(0, 0, 40, 20));
        assert_eq!(frame.windows[1].rect, Rect::new(40, 0, 40, 20));
        assert!(frame.windows[1].focused);
    }

    #[test]
    fn collapsed_window_has_no_body_lines() {
        let mut workspace = Workspace::new_untitled();
        workspace.collapse_focused().unwrap();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hidden");
        let buffer_view = BufferView::new(BufferId(1), &buffer);

        let frame = UiShell::default().frame_for_workspace(
            &workspace,
            Rect::new(0, 0, 80, 10),
            &[buffer_view],
        );

        assert!(frame.windows[0].collapsed);
        assert!(frame.windows[0].body.is_empty());
    }

    #[test]
    fn dirty_and_readonly_flags_follow_buffer_state() {
        let workspace = Workspace::new_untitled();
        let mut buffer = TextBuffer::from_text_with_kind(BufferKind::ReadOnly, "locked");
        buffer.set_cursor(Position::new(0, 0)).unwrap();
        let buffer_view = BufferView::new(BufferId(1), &buffer);

        let frame = UiShell::default().frame_for_workspace(
            &workspace,
            Rect::new(0, 0, 80, 10),
            &[buffer_view],
        );

        assert!(frame.windows[0].read_only);
        assert!(!frame.windows[0].dirty);
    }

    #[test]
    fn menu_exposes_help_and_quit_commands() {
        let menu = UiShell::default().menu_bar();

        assert!(
            menu.items
                .iter()
                .any(|item| item.command == EditorCommand::App(AppCommand::Help))
        );
        assert!(
            menu.items
                .iter()
                .any(|item| item.command == EditorCommand::App(AppCommand::Quit))
        );
    }
}
