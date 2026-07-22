//! Character display width under a terminal's ambiguous-width reading.
//!
//! `unicode-width` ships both readings: `width`/`width` is the Western/default
//! reading (Ambiguous = 1) and `width_cjk` is the East-Asian reading
//! (Ambiguous = 2). These two functions are the single place in the workspace
//! that calls `unicode-width` directly; every layout and render path routes
//! through them so a run uses one consistent width model.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::profile::AmbiguousWidth;

/// Display columns for one character under `mode`. `None` for control
/// characters (as `unicode-width` reports), matching the crate's contract.
pub fn char_width(ch: char, mode: AmbiguousWidth) -> Option<usize> {
    match mode {
        AmbiguousWidth::Narrow => UnicodeWidthChar::width(ch),
        AmbiguousWidth::Wide => UnicodeWidthChar::width_cjk(ch),
    }
}

/// Display columns for a string under `mode`.
pub fn str_width(text: &str, mode: AmbiguousWidth) -> usize {
    match mode {
        AmbiguousWidth::Narrow => UnicodeWidthStr::width(text),
        AmbiguousWidth::Wide => UnicodeWidthStr::width_cjk(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambiguous_glyphs_are_narrow_or_wide_by_mode() {
        // U+2500 box-drawing and U+25C6 diamond are East Asian Ambiguous.
        for ch in ['\u{2500}', '\u{25C6}'] {
            assert_eq!(char_width(ch, AmbiguousWidth::Narrow), Some(1));
            assert_eq!(char_width(ch, AmbiguousWidth::Wide), Some(2));
        }
    }

    #[test]
    fn ascii_and_wide_chars_are_stable_across_modes() {
        // ASCII is always 1; a genuinely Wide CJK char is always 2.
        for mode in [AmbiguousWidth::Narrow, AmbiguousWidth::Wide] {
            assert_eq!(char_width('a', mode), Some(1));
            assert_eq!(char_width('\u{4e2d}', mode), Some(2)); // 中
        }
    }

    #[test]
    fn str_width_sums_under_the_mode() {
        // "─中a": ambiguous + wide + ascii.
        let text = "\u{2500}\u{4e2d}a";
        assert_eq!(str_width(text, AmbiguousWidth::Narrow), 1 + 2 + 1);
        assert_eq!(str_width(text, AmbiguousWidth::Wide), 2 + 2 + 1);
    }
}
