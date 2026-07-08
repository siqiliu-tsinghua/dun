use ratatui::layout::{Position as TuiPosition, Rect as TuiRect};
use ratatui::prelude::Frame;
use ratatui::widgets::Block;

use crate::render::chrome::{
    render_border, render_vertical_overflow_indicators, sanitize_chrome_text, to_ratatui_style,
};
use crate::{UiOverlay, UiShell, display_width, fit_text_to_width};

pub(crate) fn render_overlay(
    frame: &mut Frame<'_>,
    shell: &UiShell,
    overlay: &UiOverlay,
    area: TuiRect,
) {
    if area.width < 12 || area.height < 5 {
        return;
    }

    let scrim = to_ratatui_style(shell.theme.palette.modal_scrim);
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            frame.buffer_mut()[(x, y)].set_style(scrim);
        }
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
    let Some(layout) = overlay_layout_for_content(
        overlay,
        &title,
        &lines,
        input.as_deref(),
        &buttons,
        &list,
        area,
    ) else {
        return;
    };
    let rect = layout.rect;

    frame.render_widget(
        Block::default().style(to_ratatui_style(shell.theme.palette.modal)),
        rect,
    );
    render_border(
        frame.buffer_mut(),
        rect,
        shell.glyphs.border,
        to_ratatui_style(shell.theme.palette.modal_border),
    );

    if rect.width > 6 {
        let title_width = rect.width.saturating_sub(4) as usize;
        let title = fit_text_to_width(
            &format!(" {title} "),
            title_width,
            shell.glyphs.indicators.truncation,
        );
        frame.buffer_mut().set_string(
            rect.x + 2,
            rect.y,
            title,
            to_ratatui_style(shell.theme.palette.modal_text),
        );
    }

    let mut row = rect.y + 1;
    let inner_width = rect.width.saturating_sub(4) as usize;
    for line in lines {
        if row >= rect.y + rect.height - 1 {
            break;
        }
        let text = fit_text_to_width(&line, inner_width, shell.glyphs.indicators.truncation);
        frame.buffer_mut().set_string(
            rect.x + 2,
            row,
            text,
            to_ratatui_style(shell.theme.palette.modal_text),
        );
        row += 1;
    }

    if let Some(input) = input {
        if row < rect.y + rect.height - 1 {
            let input_style = to_ratatui_style(shell.theme.palette.modal_input);
            for x in (rect.x + 2)..rect.x.saturating_add(rect.width).saturating_sub(2) {
                frame.buffer_mut()[(x, row)]
                    .set_char(' ')
                    .set_style(input_style);
            }
            let text = fit_text_to_width(&input, inner_width, shell.glyphs.indicators.truncation);
            frame
                .buffer_mut()
                .set_string(rect.x + 2, row, text, input_style);
            if let Some(cursor_column) = overlay.cursor_column {
                let x = rect
                    .x
                    .saturating_add(2)
                    .saturating_add(cursor_column.min(inner_width.saturating_sub(1)) as u16);
                frame.set_cursor_position(TuiPosition::new(x, row));
            }
            row += 1;
        }
    }

    for (index, entry) in list.into_iter().enumerate() {
        if row >= rect.y + rect.height - 1 {
            break;
        }
        let style = if Some(index) == overlay.selected_list_index {
            to_ratatui_style(shell.theme.palette.modal_input)
        } else {
            to_ratatui_style(shell.theme.palette.modal_text)
        };
        if Some(index) == overlay.selected_list_index {
            for x in (rect.x + 2)..rect.x.saturating_add(rect.width).saturating_sub(2) {
                frame.buffer_mut()[(x, row)].set_char(' ').set_style(style);
            }
        }
        let text = fit_text_to_width(&entry, inner_width, shell.glyphs.indicators.truncation);
        frame.buffer_mut().set_string(rect.x + 2, row, text, style);
        row += 1;
    }
    render_vertical_overflow_indicators(
        frame.buffer_mut(),
        shell,
        rect,
        overlay.list_has_more_above,
        overlay.list_has_more_below,
        to_ratatui_style(shell.theme.palette.modal_border),
    );

    for button in buttons {
        if row >= rect.y + rect.height - 1 {
            break;
        }
        let text = fit_text_to_width(&button, inner_width, shell.glyphs.indicators.truncation);
        let x = rect
            .x
            .saturating_add(rect.width.saturating_sub(display_width(&text) as u16) / 2);
        frame.buffer_mut().set_string(
            x,
            row,
            text,
            to_ratatui_style(shell.theme.palette.modal_text),
        );
        row += 1;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OverlayLayout {
    pub(crate) rect: TuiRect,
    pub(crate) list_start_row: u16,
    pub(crate) list_rows: usize,
}

pub(crate) fn overlay_layout(
    shell: &UiShell,
    overlay: &UiOverlay,
    area: TuiRect,
) -> Option<OverlayLayout> {
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

    overlay_layout_for_content(
        overlay,
        &title,
        &lines,
        input.as_deref(),
        &buttons,
        &list,
        area,
    )
}

fn overlay_layout_for_content(
    overlay: &UiOverlay,
    title: &str,
    lines: &[String],
    input: Option<&str>,
    buttons: &[String],
    list: &[String],
    area: TuiRect,
) -> Option<OverlayLayout> {
    if area.width < 12 || area.height < 5 {
        return None;
    }

    let mut content_width = display_width(title).saturating_add(4);
    for line in lines {
        content_width = content_width.max(display_width(line));
    }
    if let Some(input) = input {
        content_width = content_width.max(display_width(input).max(32));
    }
    for button in buttons {
        content_width = content_width.max(display_width(button));
    }
    for entry in list {
        content_width = content_width.max(display_width(entry));
    }

    let width = content_width
        .saturating_add(4)
        .max(overlay.min_width as usize)
        .min(area.width as usize) as u16;
    let content_rows = lines
        .len()
        .saturating_add(usize::from(input.is_some()))
        .saturating_add(list.len())
        .saturating_add(buttons.len())
        .max(1);
    let height = content_rows
        .saturating_add(2)
        .max(4)
        .min(area.height as usize) as u16;
    let rect = TuiRect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );

    let bottom = rect.y.saturating_add(rect.height).saturating_sub(1);
    let mut row = rect.y.saturating_add(1);
    for _ in lines {
        if row >= bottom {
            break;
        }
        row = row.saturating_add(1);
    }
    if input.is_some() && row < bottom {
        row = row.saturating_add(1);
    }

    let list_start_row = row;
    let mut list_rows = 0;
    for _ in list {
        if row >= bottom {
            break;
        }
        list_rows += 1;
        row = row.saturating_add(1);
    }

    Some(OverlayLayout {
        rect,
        list_start_row,
        list_rows,
    })
}
