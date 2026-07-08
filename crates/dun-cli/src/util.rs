pub(crate) const FILE_DIALOG_VISIBLE_ENTRIES: usize = 12;
pub(crate) const BUFFER_SWITCHER_VISIBLE_ENTRIES: usize = 12;
pub(crate) const EDITOR_MOUSE_WHEEL_LINES: usize = 3;
pub(crate) const MIN_BODY_COLUMNS_WITH_GUTTER: u16 = 4;
pub(crate) const EDITOR_INDENT: &str = "    ";
pub(crate) const STATUS_HISTORY_LIMIT: usize = 128;
pub(crate) const COMMAND_HISTORY_LIMIT: usize = 128;

pub(crate) fn wrapping_index(index: usize, len: usize, delta: isize) -> usize {
    debug_assert!(len > 0);
    let len = len as isize;
    (index as isize).saturating_add(delta).rem_euclid(len) as usize
}
