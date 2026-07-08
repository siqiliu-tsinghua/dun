use dun_core::{Position, TextBuffer};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(crate) fn decimal_digits(mut value: usize) -> usize {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}

pub(crate) fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

pub(crate) fn visible_whitespace_text(
    line: &str,
    show_whitespace: bool,
    ascii_only: bool,
) -> String {
    if !show_whitespace {
        return line.to_string();
    }

    let mut text = visible_whitespace_prefix_text(line, ascii_only);
    if ascii_only {
        text.push('$');
    } else {
        text.push('¶');
    }
    text
}

pub(crate) fn visible_whitespace_prefix_text(line: &str, ascii_only: bool) -> String {
    let mut text = String::with_capacity(line.len());
    for ch in line.chars() {
        match ch {
            ' ' if ascii_only => text.push('.'),
            ' ' => text.push('·'),
            '\t' if ascii_only => text.push('>'),
            '\t' => text.push('→'),
            _ => text.push(ch),
        }
    }
    text
}

pub(crate) fn wrap_line_segments(line: &str, width: usize) -> Vec<&str> {
    let width = width.max(1);
    if line.is_empty() {
        return vec![""];
    }

    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut column = 0usize;
    for (index, ch) in line.char_indices() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
        if column > 0 && column.saturating_add(ch_width) > width {
            segments.push(&line[start..index]);
            start = index;
            column = 0;
        }
        column = column.saturating_add(ch_width);
    }

    segments.push(&line[start..]);
    segments
}

pub(crate) fn fit_text_to_width(text: &str, max_width: usize, truncation: char) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let truncation_width = UnicodeWidthChar::width(truncation).unwrap_or(1);
    if truncation_width > max_width {
        return String::new();
    }

    let body_width = max_width - truncation_width;
    let mut out = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > body_width {
            break;
        }
        out.push(ch);
        width += ch_width;
    }
    out.push(truncation);
    out
}

pub(crate) fn status_text_for_width(
    left: &str,
    right: &str,
    width: usize,
    truncation: char,
) -> String {
    if width == 0 {
        return String::new();
    }

    let right_width = display_width(right);
    if !right.is_empty() && width >= right_width.saturating_add(2) {
        let left_width = width - right_width - 1;
        let left = fit_text_to_width(left, left_width, truncation);
        let gap = width.saturating_sub(display_width(&left).saturating_add(right_width));
        let mut out = left;
        out.push_str(&" ".repeat(gap));
        out.push_str(right);
        return out;
    }

    fit_text_to_width(left, width, truncation)
}

pub(crate) fn buffer_end_position(buffer: &TextBuffer) -> Position {
    let last_line = buffer.line_count().saturating_sub(1);
    let last_column = buffer.line(last_line).map(str::len).unwrap_or(0);
    Position::new(last_line, last_column)
}
