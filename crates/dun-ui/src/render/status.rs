use ratatui::layout::Rect as TuiRect;
use ratatui::prelude::Frame;
use ratatui::widgets::Paragraph;

use crate::render::chrome::{sanitize_chrome_text, to_ratatui_style};
use crate::{StatusBar, UiShell, status_text_for_width};

pub(crate) fn render_status(
    frame: &mut Frame<'_>,
    shell: &UiShell,
    status: &StatusBar,
    area: TuiRect,
) {
    let text = sanitized_status_text_for_width(shell, status, area.width as usize);

    frame.render_widget(
        Paragraph::new(text).style(to_ratatui_style(shell.theme.palette.status_bar)),
        area,
    );
}

pub(crate) fn sanitized_status_text_for_width(
    shell: &UiShell,
    status: &StatusBar,
    width: usize,
) -> String {
    let left = sanitize_chrome_text(shell, &status.left);
    let right = sanitize_chrome_text(shell, &status.right);
    status_text_for_width(&left, &right, width, shell.glyphs.indicators.truncation)
}
