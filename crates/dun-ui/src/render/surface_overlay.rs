use dun_core::Rect as TuiRect;

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

    let border_columns = shell.border_columns();
    let panel_inset = border_columns.saturating_add(1);
    let panel_padding = panel_inset.saturating_mul(2);
    let top_inset = border_columns.max(2);
    let top_padding = top_inset.saturating_mul(2);

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

    // The modal body must blank what is underneath it. `style_run` only
    // restyles cells, so the editor text below would keep showing through
    // wherever the modal's own text does not reach.
    surface.fill_rect(
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        ' ',
        shell.theme.palette.modal,
    );
    draw_border(
        surface,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        shell.glyphs.border,
        shell.theme.palette.modal_border,
    );

    if rect.width > top_padding.saturating_add(2) {
        let title_width = rect.width.saturating_sub(top_padding) as usize;
        let title = fit_text_to_width(
            &format!(" {title} "),
            title_width,
            shell.glyphs.indicators.truncation,
            shell.profile.ambiguous_width,
        );
        surface.set_text(
            rect.x.saturating_add(top_inset),
            rect.y,
            &title,
            shell.theme.palette.modal_text,
        );
    }

    let mut cursor = None;
    let mut row = rect.y + 1;
    let inner_width = rect.width.saturating_sub(panel_padding) as usize;
    for line in lines {
        if row >= rect.y + rect.height - 1 {
            break;
        }
        let text = fit_text_to_width(
            &line,
            inner_width,
            shell.glyphs.indicators.truncation,
            shell.profile.ambiguous_width,
        );
        surface.set_text(
            rect.x.saturating_add(panel_inset),
            row,
            &text,
            shell.theme.palette.modal_text,
        );
        row += 1;
    }

    if let Some(input) = input {
        if row < rect.y + rect.height - 1 {
            let input_style = shell.theme.palette.modal_input;
            surface.fill_rect(
                rect.x.saturating_add(panel_inset),
                row,
                rect.width.saturating_sub(panel_padding),
                1,
                ' ',
                input_style,
            );
            let text = fit_text_to_width(
                &input,
                inner_width,
                shell.glyphs.indicators.truncation,
                shell.profile.ambiguous_width,
            );
            surface.set_text(rect.x.saturating_add(panel_inset), row, &text, input_style);
            if let Some(cursor_column) = overlay.cursor_column {
                let x = rect
                    .x
                    .saturating_add(panel_inset)
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
            surface.fill_rect(
                rect.x.saturating_add(panel_inset),
                row,
                rect.width.saturating_sub(panel_padding),
                1,
                ' ',
                style,
            );
        }
        let text = fit_text_to_width(
            &entry,
            inner_width,
            shell.glyphs.indicators.truncation,
            shell.profile.ambiguous_width,
        );
        surface.set_text(rect.x.saturating_add(panel_inset), row, &text, style);
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
        shell.glyphs.border.vertical,
        shell.theme.palette.modal_border,
    );

    for button in buttons {
        if row >= rect.y + rect.height - 1 {
            break;
        }
        let text = fit_text_to_width(
            &button,
            inner_width,
            shell.glyphs.indicators.truncation,
            shell.profile.ambiguous_width,
        );
        let x = rect.x.saturating_add(
            rect.width
                .saturating_sub(display_width(&text, shell.profile.ambiguous_width) as u16)
                / 2,
        );
        surface.set_text(x, row, &text, shell.theme.palette.modal_text);
        row += 1;
    }

    cursor
}
