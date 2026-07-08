#![allow(unused_imports)]

pub(super) use crate::*;
pub(super) use dun_config::{ColorProfile, EncodingProfile, KeySequence, TerminalOverrides};
pub(super) use dun_core::{
    AppCommand, Axis, BufferId, BufferKind, FileCommand, Position, Rect, WindowId,
};
pub(super) use ratatui::Terminal;
pub(super) use ratatui::backend::TestBackend;
pub(super) use ratatui::buffer::Buffer;
pub(super) use ratatui::layout::Rect as TuiRect;
pub(super) use std::str::FromStr;

pub(super) fn terminal_text_snapshot(buffer: &Buffer, width: u16, height: u16) -> String {
    let mut out = String::new();
    for y in 0..height {
        for x in 0..width {
            out.push_str(buffer[(x, y)].symbol());
        }
        if y + 1 < height {
            out.push('\n');
        }
    }
    out
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
