use dun_core::Rect;

use crate::render::chrome::sanitize_chrome_text;
use crate::{UiShell, UiWindow, fit_text_to_width};

pub(crate) fn offset_rect(rect: Rect, origin: Rect) -> Rect {
    Rect::new(
        origin.x.saturating_add(rect.x),
        origin.y.saturating_add(rect.y),
        rect.width.min(origin.width.saturating_sub(rect.x)),
        rect.height.min(origin.height.saturating_sub(rect.y)),
    )
}

pub(crate) fn window_title_for_width(
    shell: &UiShell,
    window: &UiWindow,
    max_width: usize,
) -> String {
    let mut title = String::new();
    title.push(' ');
    if window.focused {
        title.push(shell.glyphs.indicators.focused);
        title.push(' ');
    }
    title.push_str(&sanitize_chrome_text(shell, &window.title));
    if window.dirty {
        title.push(' ');
        title.push(shell.glyphs.indicators.dirty);
    }
    if window.read_only {
        title.push(' ');
        title.push(shell.glyphs.indicators.read_only);
    }
    if window.collapsed {
        title.push(' ');
        title.push(shell.glyphs.indicators.collapsed);
    }
    title.push(' ');

    fit_text_to_width(&title, max_width, shell.glyphs.indicators.truncation)
}
