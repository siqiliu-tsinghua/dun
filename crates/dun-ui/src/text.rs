use dun_core::{Position, TextBuffer};
use dun_term::{AmbiguousWidth, char_width, str_width};

pub(crate) fn decimal_digits(mut value: usize) -> usize {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}

pub(crate) fn display_width(text: &str, mode: AmbiguousWidth) -> usize {
    str_width(text, mode)
}

pub(crate) fn wrap_line_segments(line: &str, width: usize, mode: AmbiguousWidth) -> Vec<&str> {
    let width = width.max(1);
    if line.is_empty() {
        return vec![""];
    }

    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut column = 0usize;
    for (index, ch) in line.char_indices() {
        let ch_width = char_width(ch, mode).unwrap_or(0).max(1);
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

pub(crate) fn fit_text_to_width(
    text: &str,
    max_width: usize,
    truncation: char,
    mode: AmbiguousWidth,
) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text, mode) <= max_width {
        return text.to_string();
    }

    let truncation_width = char_width(truncation, mode).unwrap_or(1);
    if truncation_width > max_width {
        return String::new();
    }

    let body_width = max_width - truncation_width;
    let mut out = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_width = char_width(ch, mode).unwrap_or(0);
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
    mode: AmbiguousWidth,
) -> String {
    if width == 0 {
        return String::new();
    }

    let right_width = display_width(right, mode);
    if !right.is_empty() && width >= right_width.saturating_add(2) {
        let left_width = width - right_width - 1;
        let left = fit_text_to_width(left, left_width, truncation, mode);
        let gap = width.saturating_sub(display_width(&left, mode).saturating_add(right_width));
        let mut out = left;
        out.push_str(&" ".repeat(gap));
        out.push_str(right);
        return out;
    }

    fit_text_to_width(left, width, truncation, mode)
}

pub(crate) fn buffer_end_position(buffer: &TextBuffer) -> Position {
    let last_line = buffer.line_count().saturating_sub(1);
    let last_column = buffer.line(last_line).map(str::len).unwrap_or(0);
    Position::new(last_line, last_column)
}

#[cfg(test)]
mod tests {
    use super::{display_width, fit_text_to_width, wrap_line_segments};
    use dun_term::AmbiguousWidth;

    #[test]
    fn wide_text_helpers_measure_fit_and_wrap_ambiguous_glyphs() {
        let text = "───";

        assert_eq!(display_width(text, AmbiguousWidth::Narrow), 3);
        assert_eq!(display_width(text, AmbiguousWidth::Wide), 6);
        assert_eq!(
            fit_text_to_width(text, 4, '…', AmbiguousWidth::Narrow),
            text
        );
        assert_eq!(fit_text_to_width(text, 4, '…', AmbiguousWidth::Wide), "─…");
        assert_eq!(
            wrap_line_segments(text, 4, AmbiguousWidth::Narrow),
            vec![text]
        );
        assert_eq!(
            wrap_line_segments(text, 4, AmbiguousWidth::Wide),
            vec!["──", "─"]
        );
    }
}
