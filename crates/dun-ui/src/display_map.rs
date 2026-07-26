use dun_core::{DisplaySanitizer, SanitizedLine};
use dun_term::{AmbiguousWidth, GlyphSet, char_width, str_width};

/// The authoritative mapping between editor source bytes and terminal cells.
///
/// Source positions remain UTF-8 byte boundaries. Display positions account
/// for sanitizer expansion, ambiguous-width policy, and optional whitespace
/// markers without changing the source coordinates reported by the sanitizer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorTextDisplay {
    sanitizer: DisplaySanitizer,
    ambiguous_width: AmbiguousWidth,
    glyphs: GlyphSet,
    visible_whitespace: bool,
}

impl EditorTextDisplay {
    pub const fn new(
        sanitizer: DisplaySanitizer,
        ambiguous_width: AmbiguousWidth,
        glyphs: GlyphSet,
        visible_whitespace: bool,
    ) -> Self {
        Self {
            sanitizer,
            ambiguous_width,
            glyphs,
            visible_whitespace,
        }
    }

    pub const fn visible_whitespace(self) -> bool {
        self.visible_whitespace
    }

    pub const fn ambiguous_width(self) -> AmbiguousWidth {
        self.ambiguous_width
    }

    pub const fn ascii_only(self) -> bool {
        self.sanitizer.ascii_only
    }

    pub fn sanitize_line(self, line: &str) -> SanitizedLine {
        self.sanitize_source(line, true)
    }

    pub fn sanitize_wrapped_segment(self, segment: WrappedSegment<'_>) -> SanitizedLine {
        self.sanitize_source(segment.source(), segment.logical_end)
    }

    /// Returns the display column at `source_byte`, or `None` when the byte is
    /// not a UTF-8 boundary in `line`.
    pub fn source_byte_to_display_column(self, line: &str, source_byte: usize) -> Option<usize> {
        let prefix = line.get(..source_byte)?;
        Some(self.source_text_width(prefix))
    }

    /// Maps a display column to the nearest source boundary on its right.
    /// Columns occupied by the logical EOL marker map to `line.len()`.
    pub fn display_column_to_source_byte(self, line: &str, target: usize) -> usize {
        if target == 0 {
            return 0;
        }

        let mut width = 0usize;
        for (index, ch) in line.char_indices() {
            let next_index = index.saturating_add(ch.len_utf8());
            width = width.saturating_add(self.source_char_width(ch));
            if width >= target {
                return next_index;
            }
        }

        line.len()
    }

    /// The display width of a whole logical line, used for horizontal scroll
    /// extents and edge indicators.
    ///
    /// With visible whitespace off this is the raw unicode width — a single
    /// native pass, escape-ignorant — matching the pre-seam `display_width`
    /// exactly (control/invisible bytes still measure at their unicode width
    /// here, as they did before the seam, even though the within-line column
    /// mapping below sanitizes them). Keeping the fast path is what stops a
    /// multi-megabyte line from making every frame's edge computation walk it
    /// character by character.
    pub fn line_display_width(self, line: &str) -> usize {
        if self.visible_whitespace {
            self.source_text_width(line)
                .saturating_add(self.output_char_width(self.glyphs.whitespace.end_of_line))
        } else {
            str_width(line, self.ambiguous_width)
        }
    }

    pub fn source_char_display_width(self, ch: char) -> usize {
        self.source_char_width(ch)
    }

    pub fn advance_wrapped_position(
        self,
        row: usize,
        column: usize,
        next_width: usize,
        body_width: usize,
    ) -> (usize, usize) {
        let body_width = body_width.max(1);
        if column > 0 && column.saturating_add(next_width) > body_width {
            (row.saturating_add(1), next_width)
        } else {
            (row, column.saturating_add(next_width))
        }
    }

    pub fn wrapped_segments(self, line: &str, width: usize) -> WrappedSegments<'_> {
        WrappedSegments {
            display: self,
            line,
            width: width.max(1),
            next_source: 0,
            emitted_any: false,
            suffix_pending: false,
            done: false,
        }
    }

    pub fn wrapped_row_count(self, line: &str, width: usize) -> usize {
        self.wrapped_segments(line, width).count().max(1)
    }

    pub fn wrapped_row_column_for_source_byte(
        self,
        line: &str,
        source_byte: usize,
        width: usize,
    ) -> Option<(usize, usize)> {
        line.get(..source_byte)?;
        let mut end_candidate = None;

        for (row, segment) in self.wrapped_segments(line, width).enumerate() {
            if source_byte == segment.start_byte {
                return Some((row, 0));
            }
            if source_byte < segment.end_byte {
                let local_byte = source_byte.saturating_sub(segment.start_byte);
                let column = self.source_byte_to_display_column(segment.source(), local_byte)?;
                return Some((row, column));
            }
            if source_byte == segment.end_byte {
                let column = self.source_text_width(segment.source());
                if segment.logical_end {
                    return Some((row, column));
                }
                end_candidate = Some((row, column));
            }
        }

        end_candidate
    }

    pub fn source_byte_for_wrapped_row_column(
        self,
        line: &str,
        target_row: usize,
        target_column: usize,
        width: usize,
    ) -> usize {
        let Some(segment) = self.wrapped_segments(line, width).nth(target_row) else {
            return line.len();
        };
        let local_byte = self.display_column_to_source_byte(segment.source(), target_column);
        segment.start_byte.saturating_add(local_byte)
    }

    fn sanitize_source(self, source: &str, logical_end: bool) -> SanitizedLine {
        let suffix =
            (self.visible_whitespace && logical_end).then_some(self.glyphs.whitespace.end_of_line);
        self.sanitizer
            .sanitize_line_mapped(source, |ch| self.whitespace_mapping(ch), suffix)
    }

    fn source_text_width(self, text: &str) -> usize {
        text.chars().fold(0usize, |width, ch| {
            width.saturating_add(self.source_char_width(ch))
        })
    }

    fn source_char_width(self, ch: char) -> usize {
        let mapped = self.whitespace_mapping(ch).unwrap_or(ch);
        self.output_char_width(mapped)
    }

    fn output_char_width(self, ch: char) -> usize {
        if self.sanitizer.renders_unchanged(ch) {
            return char_width(ch, self.ambiguous_width).unwrap_or(0);
        }

        let mut encoded = [0; 4];
        let source = ch.encode_utf8(&mut encoded);
        let sanitizer = if self.sanitizer.ascii_only {
            DisplaySanitizer::unlimited_ascii()
        } else {
            DisplaySanitizer::unlimited_utf8()
        };
        sanitizer
            .sanitize_line(source)
            .segments
            .iter()
            .flat_map(|segment| segment.text.chars())
            .fold(0usize, |width, rendered| {
                width.saturating_add(char_width(rendered, self.ambiguous_width).unwrap_or(0))
            })
    }

    const fn whitespace_mapping(self, ch: char) -> Option<char> {
        if !self.visible_whitespace {
            return None;
        }
        match ch {
            ' ' => Some(self.glyphs.whitespace.space),
            '\t' => Some(self.glyphs.whitespace.tab),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WrappedSegment<'a> {
    line: &'a str,
    start_byte: usize,
    end_byte: usize,
    logical_end: bool,
}

impl<'a> WrappedSegment<'a> {
    pub fn source(self) -> &'a str {
        &self.line[self.start_byte..self.end_byte]
    }

    pub const fn start_byte(self) -> usize {
        self.start_byte
    }

    pub const fn end_byte(self) -> usize {
        self.end_byte
    }
}

#[derive(Clone, Debug)]
pub struct WrappedSegments<'a> {
    display: EditorTextDisplay,
    line: &'a str,
    width: usize,
    next_source: usize,
    emitted_any: bool,
    suffix_pending: bool,
    done: bool,
}

impl<'a> Iterator for WrappedSegments<'a> {
    type Item = WrappedSegment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.next_source == self.line.len() {
            if !self.emitted_any || self.suffix_pending {
                self.emitted_any = true;
                self.suffix_pending = false;
                self.done = true;
                return Some(WrappedSegment {
                    line: self.line,
                    start_byte: self.line.len(),
                    end_byte: self.line.len(),
                    logical_end: true,
                });
            }
            self.done = true;
            return None;
        }

        let start_byte = self.next_source;
        let mut end_byte = start_byte;
        let mut column = 0usize;
        for (offset, ch) in self.line[start_byte..].char_indices() {
            let index = start_byte.saturating_add(offset);
            let ch_width = self.display.source_char_width(ch);
            let (next_row, next_column) = self
                .display
                .advance_wrapped_position(0, column, ch_width, self.width);
            if next_row > 0 {
                break;
            }
            column = next_column;
            end_byte = index.saturating_add(ch.len_utf8());
        }

        self.next_source = end_byte;
        self.emitted_any = true;
        let source_complete = end_byte == self.line.len();
        let logical_end = if source_complete {
            if self.display.visible_whitespace {
                let suffix_width = self
                    .display
                    .output_char_width(self.display.glyphs.whitespace.end_of_line);
                if column > 0 && column.saturating_add(suffix_width) > self.width {
                    self.suffix_pending = true;
                    false
                } else {
                    self.done = true;
                    true
                }
            } else {
                self.done = true;
                true
            }
        } else {
            false
        };

        Some(WrappedSegment {
            line: self.line,
            start_byte,
            end_byte,
            logical_end,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(mode: AmbiguousWidth, visible_whitespace: bool) -> EditorTextDisplay {
        EditorTextDisplay::new(
            DisplaySanitizer::unlimited_utf8(),
            mode,
            GlyphSet::unicode_single_line(),
            visible_whitespace,
        )
    }

    #[test]
    fn visible_whitespace_coordinates_are_hard_coded_for_both_width_modes() {
        let line = "中 \t";
        let narrow = display(AmbiguousWidth::Narrow, true);
        let wide = display(AmbiguousWidth::Wide, true);

        assert_eq!(narrow.sanitize_line(line).as_plain_text(), "中·→¶");
        assert_eq!(narrow.source_byte_to_display_column(line, 0), Some(0));
        assert_eq!(narrow.source_byte_to_display_column(line, 3), Some(2));
        assert_eq!(narrow.source_byte_to_display_column(line, 4), Some(3));
        assert_eq!(narrow.source_byte_to_display_column(line, 5), Some(4));
        assert_eq!(narrow.line_display_width(line), 5);

        assert_eq!(wide.sanitize_line(line).as_plain_text(), "中·→¶");
        assert_eq!(wide.source_byte_to_display_column(line, 0), Some(0));
        assert_eq!(wide.source_byte_to_display_column(line, 3), Some(2));
        assert_eq!(wide.source_byte_to_display_column(line, 4), Some(4));
        assert_eq!(wide.source_byte_to_display_column(line, 5), Some(6));
        assert_eq!(wide.line_display_width(line), 8);
    }

    #[test]
    fn display_columns_round_trip_to_utf8_source_boundaries() {
        let line = "aé中";
        let display = display(AmbiguousWidth::Narrow, false);
        let expected = [(0, 0), (1, 1), (3, 2), (6, 4)];

        for column in 0..=display.line_display_width(line) {
            let source = display.display_column_to_source_byte(line, column);
            assert!(line.is_char_boundary(source));
        }
        for (source, column) in expected {
            assert_eq!(
                display.source_byte_to_display_column(line, source),
                Some(column)
            );
            assert_eq!(display.display_column_to_source_byte(line, column), source);
        }
    }

    #[test]
    fn wrapped_row_and_column_map_to_source_and_logical_eol() {
        let line = "中ab";
        let display = display(AmbiguousWidth::Narrow, true);

        assert_eq!(display.wrapped_row_count(line, 2), 3);
        assert_eq!(display.source_byte_for_wrapped_row_column(line, 0, 1, 2), 3);
        assert_eq!(display.source_byte_for_wrapped_row_column(line, 1, 1, 2), 4);
        assert_eq!(
            display.source_byte_for_wrapped_row_column(line, 2, 0, 2),
            line.len()
        );
        assert_eq!(display.display_column_to_source_byte("x", 2), 1);
    }

    #[test]
    fn within_line_char_mapping_is_escape_aware() {
        let display = display(AmbiguousWidth::Narrow, false);

        // The within-line column mapping sanitizes escapes: ESC renders as the
        // one-cell control picture "␛", a tag-block char as the nine-cell
        // "<U+E0041>", a zero-width space as the eight-cell "<U+200B>".
        // Expectations are hard-coded, never derived from the renderer.
        assert_eq!(display.source_char_display_width('\u{1b}'), 1);
        assert_eq!(display.source_char_display_width('\u{e0041}'), 9);
        assert_eq!(display.source_char_display_width('a'), 1);
        // "a" (1) + U+200B rendered "<U+200B>" (8) + "b" (1) = 10, at end byte 5.
        assert_eq!(
            display.source_byte_to_display_column("a\u{200b}b", 5),
            Some(10)
        );
    }

    #[test]
    fn line_display_width_is_raw_unicode_width_when_markers_are_off() {
        let display = display(AmbiguousWidth::Narrow, false);

        // The aggregate line width (scroll extent) stays escape-ignorant to
        // preserve the pre-seam behavior and its single-pass speed: a
        // zero-width space contributes 0, not the 8 cells its escape renders as
        // in the within-line mapping above. `中` is two cells; `a`/`b` one each.
        assert_eq!(display.line_display_width("a\u{200b}b"), 2);
        assert_eq!(display.line_display_width("中ab"), 4);
        assert_eq!(display.line_display_width(""), 0);
    }

    #[test]
    fn default_off_output_matches_the_existing_sanitizer() {
        let sanitizer = DisplaySanitizer::utf8(4);
        let corpus = ["中x", "a\tb", "safe\u{1b}", "abcdefgh"];

        for mode in [AmbiguousWidth::Narrow, AmbiguousWidth::Wide] {
            let display =
                EditorTextDisplay::new(sanitizer, mode, GlyphSet::unicode_single_line(), false);
            for line in corpus {
                assert_eq!(display.sanitize_line(line), sanitizer.sanitize_line(line));
            }
        }
    }
}
