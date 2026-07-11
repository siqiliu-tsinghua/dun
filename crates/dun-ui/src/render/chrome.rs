use crate::UiShell;

pub(crate) fn vertical_overflow_up(shell: &UiShell) -> char {
    if shell.profile.supports_unicode_glyphs() {
        '↑'
    } else {
        '^'
    }
}

pub(crate) fn vertical_overflow_down(shell: &UiShell) -> char {
    if shell.profile.supports_unicode_glyphs() {
        '↓'
    } else {
        'v'
    }
}

pub(crate) fn sanitize_chrome_text(shell: &UiShell, text: &str) -> String {
    shell.display_sanitizer.sanitize_line(text).as_plain_text()
}
