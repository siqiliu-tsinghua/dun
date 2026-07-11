use dun_core::Rect;

use crate::render::chrome::sanitize_chrome_text;
use crate::{MenuBar, MenuEntry, UiShell, display_width, fit_text_to_width, status_text_for_width};

pub(crate) fn menu_item_column_range(menu: &MenuBar, index: usize) -> Option<(u16, u16)> {
    let mut x = 1usize;
    for (candidate, item) in menu.items.iter().enumerate() {
        let end = x.saturating_add(display_width(item.label).saturating_add(2));
        if candidate == index {
            return Some((
                x.min(u16::MAX as usize) as u16,
                end.min(u16::MAX as usize) as u16,
            ));
        }
        x = end;
    }

    None
}

pub(crate) fn dropdown_rect_for_menu(
    shell: &UiShell,
    menu: &MenuBar,
    index: usize,
) -> Option<Rect> {
    let item = menu.items.get(index)?;
    let (start, _) = menu_item_column_range(menu, index)?;
    let content_width = item
        .entries
        .iter()
        .map(|entry| menu_entry_width(shell, entry))
        .max()
        .unwrap_or(1)
        .max(display_width(item.label));
    let width = content_width.saturating_add(4).min(u16::MAX as usize) as u16;
    let height = item.entries.len().saturating_add(2).min(u16::MAX as usize) as u16;

    Some(Rect::new(start, 1, width.max(3), height.max(3)))
}

pub(crate) fn clamp_menu_rect(rect: Rect, area: Rect) -> Option<Rect> {
    if area.width == 0 || area.height <= 1 {
        return None;
    }

    let x = rect
        .x
        .min(area.x.saturating_add(area.width).saturating_sub(1));
    let y = rect
        .y
        .min(area.y.saturating_add(area.height).saturating_sub(1));
    let width = rect
        .width
        .min(area.x.saturating_add(area.width).saturating_sub(x));
    let height = rect
        .height
        .min(area.y.saturating_add(area.height).saturating_sub(y));

    (width >= 3 && height >= 3).then_some(Rect::new(x, y, width, height))
}

pub(crate) fn menu_visible_entry_range(
    total: usize,
    selected: Option<usize>,
    max_rows: usize,
) -> Option<(usize, usize)> {
    if total == 0 || max_rows == 0 {
        return None;
    }

    let max_rows = max_rows.min(total);
    let selected = selected.unwrap_or(0).min(total - 1);
    let mut start = 0usize;
    if selected >= max_rows {
        start = selected.saturating_add(1).saturating_sub(max_rows);
    }
    start = start.min(total.saturating_sub(max_rows));
    Some((start, start.saturating_add(max_rows).min(total)))
}

fn menu_entry_width(shell: &UiShell, entry: &MenuEntry) -> usize {
    let label_width = display_width(entry.label);
    let shortcut_width = shell
        .keymap
        .sequence_for_command(&entry.command)
        .map(|shortcut| display_width(&shortcut.to_string()))
        .unwrap_or(0);
    if shortcut_width == 0 {
        label_width
    } else {
        label_width.saturating_add(1).saturating_add(shortcut_width)
    }
}

pub(crate) fn menu_entry_text(shell: &UiShell, entry: &MenuEntry, width: usize) -> String {
    let shortcut = shell
        .keymap
        .sequence_for_command(&entry.command)
        .map(ToString::to_string)
        .unwrap_or_default();
    let label = sanitize_chrome_text(shell, entry.label);
    let shortcut = sanitize_chrome_text(shell, &shortcut);

    if shortcut.is_empty() {
        return fit_text_to_width(&label, width, shell.glyphs.indicators.truncation);
    }

    status_text_for_width(&label, &shortcut, width, shell.glyphs.indicators.truncation)
}
