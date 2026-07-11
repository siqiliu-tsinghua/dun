use ratatui::layout::Rect as TuiRect;

use crate::render::chrome::{vertical_overflow_down, vertical_overflow_up};
use crate::render::menu::{
    clamp_menu_rect, dropdown_rect_for_menu, menu_entry_text, menu_visible_entry_range,
};
use crate::render::status::sanitized_status_text_for_width;
use crate::render::surface_draw::{draw_border, draw_overflow_indicators};
use crate::surface::Surface;
use crate::{MenuBar, StatusBar, UiShell};

pub(crate) fn draw_status(
    surface: &mut Surface,
    shell: &UiShell,
    status: &StatusBar,
    area: TuiRect,
) {
    let style = shell.theme.palette.status_bar;
    let text = sanitized_status_text_for_width(shell, status, area.width as usize);

    surface.fill_rect(area.x, area.y, area.width, area.height, ' ', style);
    surface.set_text(area.x, area.y, &text, style);
}

pub(crate) fn draw_menu_bar(surface: &mut Surface, shell: &UiShell, menu: &MenuBar, area: TuiRect) {
    surface.fill_rect(
        area.x,
        area.y,
        area.width,
        area.height,
        ' ',
        shell.theme.palette.menu_bar,
    );

    let mut x = area.x;
    x = x.saturating_add(surface.set_text(x, area.y, " ", shell.theme.palette.menu_text));

    for (index, item) in menu.items.iter().enumerate() {
        let active = menu.active.map(|selection| selection.menu_index) == Some(index);
        let item_style = if active {
            shell.theme.palette.menu_active
        } else {
            shell.theme.palette.menu_text
        };
        let hotkey_style = if active {
            shell.theme.palette.menu_active_hotkey
        } else {
            shell.theme.palette.menu_hotkey
        };

        x = x.saturating_add(surface.set_text(x, area.y, " ", item_style));
        let mut chars = item.label.chars();
        if let Some(first) = chars.next() {
            let first = first.to_string();
            x = x.saturating_add(surface.set_text(x, area.y, &first, hotkey_style));
            let rest = chars.collect::<String>();
            x = x.saturating_add(surface.set_text(x, area.y, &rest, item_style));
        }
        x = x.saturating_add(surface.set_text(x, area.y, " ", item_style));
    }
}

pub(crate) fn draw_active_menu(
    surface: &mut Surface,
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

    surface.fill_rect(
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        ' ',
        shell.theme.palette.menu_panel,
    );
    draw_border(
        surface,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        shell.glyphs.border,
        shell.theme.palette.menu_panel_border,
    );

    let content_width = rect.width.saturating_sub(4) as usize;
    let max_rows = rect.height.saturating_sub(2) as usize;
    let Some((start, end)) =
        menu_visible_entry_range(item.entries.len(), active.entry_index, max_rows)
    else {
        return;
    };
    draw_overflow_indicators(
        surface,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        vertical_overflow_up(shell),
        vertical_overflow_down(shell),
        start > 0,
        end < item.entries.len(),
        shell.theme.palette.menu_panel_border,
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
        surface.set_text(rect.x + 2, y, &text, style);
    }
}

#[cfg(test)]
mod tests {
    use dun_core::{BufferId, BufferKind, Rect, TextBuffer, Workspace};
    use dun_term::{Style, StyleAttrs, TerminalColor};
    use ratatui::layout::Rect as TuiRect;

    use super::{draw_active_menu, draw_menu_bar, draw_status};
    use crate::render::menu::{clamp_menu_rect, dropdown_rect_for_menu};
    use crate::surface::Surface;
    use crate::{BufferView, MenuSelection, UiFrame, UiShell};

    const INITIAL_STYLE: Style = Style::new(
        TerminalColor::Indexed(254),
        TerminalColor::Indexed(255),
        StyleAttrs::BOLD_REVERSE,
    );

    fn fixture(
        shell: &UiShell,
        width: u16,
        height: u16,
        active_menu: Option<MenuSelection>,
    ) -> UiFrame {
        let workspace = Workspace::new_untitled();
        let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
        let buffer_view = BufferView::new(BufferId(1), &buffer);
        shell.frame_for_workspace_with_menu_selection(
            &workspace,
            Rect::new(0, 0, width, height),
            &[buffer_view],
            active_menu,
        )
    }

    #[test]
    fn status_line_fills_row_and_draws_text() {
        let shell = UiShell::default();
        let frame = fixture(&shell, 12, 3, None);
        let mut surface = Surface::new(12, 1, INITIAL_STYLE);

        draw_status(
            &mut surface,
            &shell,
            &frame.status,
            TuiRect::new(0, 0, 12, 1),
        );

        assert_eq!(surface.row_text(0), "1 window(s) ");
        assert!((0..12).all(|x| {
            surface.cell(x, 0).map(|cell| cell.style) == Some(shell.theme.palette.status_bar)
        }));
    }

    #[test]
    fn menu_bar_draws_items_with_hotkey_style() {
        let shell = UiShell::default();
        let frame = fixture(&shell, 40, 6, None);
        let mut surface = Surface::new(40, 1, INITIAL_STYLE);

        draw_menu_bar(&mut surface, &shell, &frame.menu, TuiRect::new(0, 0, 40, 1));

        assert!(surface.row_text(0).starts_with("  File  Edit  View  Help "));
        assert_eq!(surface.cell(2, 0).unwrap().symbol, "F");
        assert_eq!(
            surface.cell(2, 0).map(|cell| cell.style),
            Some(shell.theme.palette.menu_hotkey)
        );
        assert_eq!(surface.cell(3, 0).unwrap().symbol, "i");
        assert_eq!(
            surface.cell(3, 0).map(|cell| cell.style),
            Some(shell.theme.palette.menu_text)
        );
        assert_eq!(
            surface.cell(39, 0).map(|cell| cell.style),
            Some(shell.theme.palette.menu_bar)
        );
    }

    #[test]
    fn menu_bar_marks_active_item() {
        let shell = UiShell::default();
        let frame = fixture(&shell, 40, 6, Some(MenuSelection::menu_only(0)));
        let mut surface = Surface::new(40, 1, INITIAL_STYLE);

        draw_menu_bar(&mut surface, &shell, &frame.menu, TuiRect::new(0, 0, 40, 1));

        assert_eq!(
            surface.cell(2, 0).map(|cell| cell.style),
            Some(shell.theme.palette.menu_active_hotkey)
        );
        assert_eq!(
            surface.cell(3, 0).map(|cell| cell.style),
            Some(shell.theme.palette.menu_active)
        );
    }

    #[test]
    fn active_menu_draws_panel_border_and_entries() {
        let shell = UiShell::default();
        let frame = fixture(&shell, 80, 12, Some(MenuSelection::menu_only(0)));
        let area = TuiRect::new(0, 0, 80, 14);
        let rect = clamp_menu_rect(
            dropdown_rect_for_menu(&shell, &frame.menu, 0).unwrap(),
            area,
        )
        .unwrap();
        let mut surface = Surface::new(80, 14, INITIAL_STYLE);

        draw_active_menu(&mut surface, &shell, &frame.menu, area);

        assert_eq!(
            surface.cell(rect.x, rect.y).unwrap().symbol,
            shell.glyphs.border.top_left.to_string()
        );
        assert_eq!(
            surface
                .cell(rect.x + rect.width - 1, rect.y)
                .unwrap()
                .symbol,
            shell.glyphs.border.top_right.to_string()
        );
        assert_eq!(
            surface
                .cell(rect.x, rect.y + rect.height - 1)
                .unwrap()
                .symbol,
            shell.glyphs.border.bottom_left.to_string()
        );
        assert_eq!(
            surface
                .cell(rect.x + rect.width - 1, rect.y + rect.height - 1)
                .unwrap()
                .symbol,
            shell.glyphs.border.bottom_right.to_string()
        );
        assert_eq!(
            surface.cell(rect.x, rect.y).map(|cell| cell.style),
            Some(shell.theme.palette.menu_panel_border)
        );
        assert!(surface.row_text(rect.y + 1).contains("New (N)"));
        assert_eq!(
            surface.cell(rect.x + 2, rect.y + 1).map(|cell| cell.style),
            Some(shell.theme.palette.menu_panel_text)
        );
    }

    #[test]
    fn active_menu_shows_overflow_indicator_when_truncated() {
        let shell = UiShell::default();
        let last_entry = shell.menu_entry_count(2).unwrap() - 1;
        let frame = fixture(
            &shell,
            80,
            10,
            Some(MenuSelection::with_entry(2, last_entry)),
        );
        let area = TuiRect::new(0, 0, 80, 12);
        let rect = clamp_menu_rect(
            dropdown_rect_for_menu(&shell, &frame.menu, 2).unwrap(),
            area,
        )
        .unwrap();
        let indicator_x = rect.x + rect.width - 2;
        let mut surface = Surface::new(80, 12, INITIAL_STYLE);

        draw_active_menu(&mut surface, &shell, &frame.menu, area);

        assert_eq!(
            surface.cell(indicator_x, rect.y).unwrap().symbol,
            crate::vertical_overflow_up(&shell).to_string()
        );
        assert_eq!(
            surface.cell(indicator_x, rect.y).map(|cell| cell.style),
            Some(shell.theme.palette.menu_panel_border)
        );
        assert!((0..surface.height()).any(|y| surface.row_text(y).contains("Reload Config")));
    }

    #[test]
    fn active_menu_absent_when_no_selection() {
        let shell = UiShell::default();
        let frame = fixture(&shell, 40, 6, None);
        let mut surface = Surface::new(40, 8, INITIAL_STYLE);
        surface.fill_rect(0, 0, 40, 8, '.', INITIAL_STYLE);
        let original = surface.clone();

        draw_active_menu(&mut surface, &shell, &frame.menu, TuiRect::new(0, 0, 40, 8));

        assert_eq!(surface, original);
    }
}
