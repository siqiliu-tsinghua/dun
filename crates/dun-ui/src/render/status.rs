use crate::render::chrome::sanitize_chrome_text;
use crate::{StatusBar, UiShell, status_text_for_width};

pub(crate) fn sanitized_status_text_for_width(
    shell: &UiShell,
    status: &StatusBar,
    width: usize,
) -> String {
    let left = sanitize_chrome_text(shell, &status.left);
    let right = sanitize_chrome_text(shell, &status.right);
    status_text_for_width(
        &left,
        &right,
        width,
        shell.glyphs.indicators.truncation,
        shell.profile.ambiguous_width,
    )
}
