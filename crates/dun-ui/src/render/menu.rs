use ratatui::buffer::Buffer;
use ratatui::layout::Rect as TuiRect;
use ratatui::prelude::{Frame, Line, Span};
use ratatui::widgets::Paragraph;

use crate::render::chrome::{
    render_border, render_vertical_overflow_indicators, sanitize_chrome_text, to_ratatui_style,
};
use crate::{MenuBar, MenuEntry, UiShell, display_width, fit_text_to_width, status_text_for_width};

pub(crate) fn render_menu(frame: &mut Frame<'_>, shell: &UiShell, menu: &MenuBar, area: TuiRect) {
    let mut spans = Vec::new();
    spans.push(Span::styled(
        " ",
        to_ratatui_style(shell.theme.palette.menu_text),
    ));

    for (index, item) in menu.items.iter().enumerate() {
        let active = menu.active.map(|selection| selection.menu_index) == Some(index);
        let item_style = if active {
            to_ratatui_style(shell.theme.palette.menu_active)
        } else {
            to_ratatui_style(shell.theme.palette.menu_text)
        };
        let hotkey_style = if active {
            to_ratatui_style(shell.theme.palette.menu_active_hotkey)
        } else {
            to_ratatui_style(shell.theme.palette.menu_hotkey)
        };
        spans.push(Span::styled(" ", item_style));
        let mut chars = item.label.chars();
        if let Some(first) = chars.next() {
            spans.push(Span::styled(first.to_string(), hotkey_style));
            spans.push(Span::styled(chars.collect::<String>(), item_style));
        }
        spans.push(Span::styled(" ", item_style));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(to_ratatui_style(shell.theme.palette.menu_bar)),
        area,
    );
}

pub(crate) fn render_active_menu(
    buffer: &mut Buffer,
    shell: &UiShell,
    menu: &MenuBar,
    area: TuiRect,
) {
    let Some(active) = menu.active else {
        return;
    };
    let Some(item) = menu.items.get(active.menu_index) else {
        return;
    };
    let Some(rect) = dropdown_rect_for_menu(shell, menu, active.menu_index) else {
        return;
    };
    let Some(rect) = clamp_menu_rect(rect, area) else {
        return;
    };

    let background = to_ratatui_style(shell.theme.palette.menu_panel);
    for y in rect.y..rect.y.saturating_add(rect.height) {
        for x in rect.x..rect.x.saturating_add(rect.width) {
            buffer[(x, y)].set_char(' ').set_style(background);
        }
    }
    render_border(
        buffer,
        rect,
        shell.glyphs.border,
        to_ratatui_style(shell.theme.palette.menu_panel_border),
    );

    let content_width = rect.width.saturating_sub(4) as usize;
    let max_rows = rect.height.saturating_sub(2) as usize;
    let Some((start, end)) =
        menu_visible_entry_range(item.entries.len(), active.entry_index, max_rows)
    else {
        return;
    };
    render_vertical_overflow_indicators(
        buffer,
        shell,
        rect,
        start > 0,
        end < item.entries.len(),
        to_ratatui_style(shell.theme.palette.menu_panel_border),
    );
    for (visible_index, entry) in item.entries[start..end].iter().enumerate() {
        let index = start + visible_index;
        let y = rect.y + 1 + visible_index as u16;
        let text = menu_entry_text(shell, entry, content_width);
        let style = if active.entry_index == Some(index) {
            shell.theme.palette.menu_active
        } else {
            shell.theme.palette.menu_panel_text
        };
        buffer.set_string(rect.x + 2, y, text, to_ratatui_style(style));
    }
}

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
) -> Option<TuiRect> {
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

    Some(TuiRect::new(start, 1, width.max(3), height.max(3)))
}

pub(crate) fn clamp_menu_rect(rect: TuiRect, area: TuiRect) -> Option<TuiRect> {
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

    (width >= 3 && height >= 3).then_some(TuiRect::new(x, y, width, height))
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
