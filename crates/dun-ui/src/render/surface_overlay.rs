use ratatui::layout::Rect as TuiRect;

use crate::render::chrome::{sanitize_chrome_text, vertical_overflow_down, vertical_overflow_up};
use crate::render::overlay::overlay_layout;
use crate::render::surface_draw::{draw_border, draw_overflow_indicators};
use crate::surface::Surface;
use crate::{UiOverlay, UiShell, display_width, fit_text_to_width};

pub(crate) fn draw_overlay(
    surface: &mut Surface,
    shell: &UiShell,
    overlay: &UiOverlay,
    area: TuiRect,
) -> Option<(u16, u16)> {
    if area.width < 12 || area.height < 5 {
        return None;
    }

    for row in area.y..area.y.saturating_add(area.height) {
        surface.style_run(area.x, row, area.width, shell.theme.palette.modal_scrim);
    }

    let title = sanitize_chrome_text(shell, &overlay.title);
    let lines = overlay
        .lines
        .iter()
        .map(|line| sanitize_chrome_text(shell, line))
        .collect::<Vec<_>>();
    let input = overlay
        .input
        .as_ref()
        .map(|input| sanitize_chrome_text(shell, input));
    let buttons = overlay
        .buttons
        .iter()
        .map(|button| sanitize_chrome_text(shell, button))
        .collect::<Vec<_>>();
    let list = overlay
        .list
        .iter()
        .map(|entry| sanitize_chrome_text(shell, entry))
        .collect::<Vec<_>>();
    let rect = overlay_layout(shell, overlay, area)?.rect;

    for row in rect.y..rect.y.saturating_add(rect.height) {
        surface.style_run(rect.x, row, rect.width, shell.theme.palette.modal);
    }
    draw_border(
        surface,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        shell.glyphs.border,
        shell.theme.palette.modal_border,
    );

    if rect.width > 6 {
        let title_width = rect.width.saturating_sub(4) as usize;
        let title = fit_text_to_width(
            &format!(" {title} "),
            title_width,
            shell.glyphs.indicators.truncation,
        );
        surface.set_text(rect.x + 2, rect.y, &title, shell.theme.palette.modal_text);
    }

    let mut cursor = None;
    let mut row = rect.y + 1;
    let inner_width = rect.width.saturating_sub(4) as usize;
    for line in lines {
        if row >= rect.y + rect.height - 1 {
            break;
        }
        let text = fit_text_to_width(&line, inner_width, shell.glyphs.indicators.truncation);
        surface.set_text(rect.x + 2, row, &text, shell.theme.palette.modal_text);
        row += 1;
    }

    if let Some(input) = input {
        if row < rect.y + rect.height - 1 {
            let input_style = shell.theme.palette.modal_input;
            surface.fill_rect(
                rect.x + 2,
                row,
                rect.width.saturating_sub(4),
                1,
                ' ',
                input_style,
            );
            let text = fit_text_to_width(&input, inner_width, shell.glyphs.indicators.truncation);
            surface.set_text(rect.x + 2, row, &text, input_style);
            if let Some(cursor_column) = overlay.cursor_column {
                let x = rect
                    .x
                    .saturating_add(2)
                    .saturating_add(cursor_column.min(inner_width.saturating_sub(1)) as u16);
                cursor = Some((x, row));
            }
            row += 1;
        }
    }

    for (index, entry) in list.into_iter().enumerate() {
        if row >= rect.y + rect.height - 1 {
            break;
        }
        let style = if Some(index) == overlay.selected_list_index {
            shell.theme.palette.modal_input
        } else {
            shell.theme.palette.modal_text
        };
        if Some(index) == overlay.selected_list_index {
            surface.fill_rect(rect.x + 2, row, rect.width.saturating_sub(4), 1, ' ', style);
        }
        let text = fit_text_to_width(&entry, inner_width, shell.glyphs.indicators.truncation);
        surface.set_text(rect.x + 2, row, &text, style);
        row += 1;
    }
    draw_overflow_indicators(
        surface,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        vertical_overflow_up(shell),
        vertical_overflow_down(shell),
        overlay.list_has_more_above,
        overlay.list_has_more_below,
        shell.theme.palette.modal_border,
    );

    for button in buttons {
        if row >= rect.y + rect.height - 1 {
            break;
        }
        let text = fit_text_to_width(&button, inner_width, shell.glyphs.indicators.truncation);
        let x = rect
            .x
            .saturating_add(rect.width.saturating_sub(display_width(&text) as u16) / 2);
        surface.set_text(x, row, &text, shell.theme.palette.modal_text);
        row += 1;
    }

    cursor
}
