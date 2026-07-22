use dun_core::Rect;
use dun_term::AmbiguousWidth;

use crate::render::chrome::sanitize_chrome_text;
use crate::{UiOverlay, UiShell, display_width};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OverlayLayout {
    pub(crate) rect: Rect,
    pub(crate) list_start_row: u16,
    pub(crate) list_rows: usize,
}

pub(crate) fn overlay_layout(
    shell: &UiShell,
    overlay: &UiOverlay,
    area: Rect,
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
        shell.profile.ambiguous_width,
    )
}

#[allow(clippy::too_many_arguments)]
fn overlay_layout_for_content(
    overlay: &UiOverlay,
    title: &str,
    lines: &[String],
    input: Option<&str>,
    buttons: &[String],
    list: &[String],
    area: Rect,
    mode: AmbiguousWidth,
) -> Option<OverlayLayout> {
    if area.width < 12 || area.height < 5 {
        return None;
    }

    let mut content_width = display_width(title, mode).saturating_add(4);
    for line in lines {
        content_width = content_width.max(display_width(line, mode));
    }
    if let Some(input) = input {
        content_width = content_width.max(display_width(input, mode).max(32));
    }
    for button in buttons {
        content_width = content_width.max(display_width(button, mode));
    }
    for entry in list {
        content_width = content_width.max(display_width(entry, mode));
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
    let rect = Rect::new(
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
