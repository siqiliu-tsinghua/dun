#![allow(unused_imports)]

pub(super) use crate::*;
pub(super) use dun_config::{
    ColorProfile, Config, EncodingProfile, KeySequence, KeyStroke, TerminalOverrides,
};
pub(super) use dun_core::{
    AppCommand, Axis, BufferId, BufferKind, EditorCommand, FileCommand, Position, Rect, TextBuffer,
    WindowId, Workspace,
};
pub(super) use dun_term::{GlyphSet, TerminalProfile};
pub(super) use std::str::FromStr;

use crate::render::surface_frame::render_ui_frame_to_surface;
use crate::surface::Surface;

/// Render a frame through the Surface backend and join the cell text row by
/// row — the Surface equivalent of the old ratatui `terminal_text_snapshot`,
/// used to assert on the glyphs that actually reach the screen.
pub(super) fn render_frame_text(
    shell: &UiShell,
    ui_frame: &UiFrame,
    width: u16,
    height: u16,
) -> String {
    let mut surface = Surface::new(width, height, shell.theme.palette.editor);
    render_ui_frame_to_surface(&mut surface, shell, ui_frame);
    // Concatenate rows without separators: the security check rejects every
    // control character, newlines included, and the callers only assert on
    // glyph presence/absence, not row structure.
    (0..height).map(|y| surface.row_text(y)).collect()
}

pub(super) fn assert_no_raw_controls(text: &str) {
    assert!(
        !text.chars().any(char::is_control),
        "raw control text was rendered: {text:?}"
    );
    assert!(!text.contains('\x1b'), "raw ESC was rendered: {text:?}");
    assert!(
        !text.contains('\u{009b}'),
        "raw C1 CSI was rendered: {text:?}"
    );
}

pub(super) fn menu_entry_mnemonic(label: &str) -> Option<char> {
    let open = label.rfind('(')?;
    let close = label[open..].find(')')?.saturating_add(open);
    let mnemonic = label[open + 1..close].chars().next()?;
    Some(mnemonic.to_ascii_lowercase())
}
