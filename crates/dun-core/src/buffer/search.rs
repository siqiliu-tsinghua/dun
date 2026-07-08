use super::edit::{end_position_after_text, normalize_edit_text};
use super::*;

impl TextBuffer {
    pub fn find_all(&self, query: &str) -> Vec<SearchMatch> {
        self.find_all_with_options(query, SearchOptions::default())
    }

    pub fn find_all_with_options(&self, query: &str, options: SearchOptions) -> Vec<SearchMatch> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut matches = Vec::new();
        for (line_index, line) in self.lines.iter().enumerate() {
            for (column, end_column) in search_line_matches(line, query, options) {
                matches.push(SearchMatch {
                    range: TextRange::new(
                        Position::new(line_index, column),
                        Position::new(line_index, end_column),
                    ),
                });
            }
        }
        matches
    }

    pub fn replace_all(&mut self, query: &str, new_text: &str) -> Result<usize, BufferError> {
        self.replace_all_with_options(query, new_text, SearchOptions::default())
    }

    pub fn replace_all_with_options(
        &mut self,
        query: &str,
        new_text: &str,
        options: SearchOptions,
    ) -> Result<usize, BufferError> {
        self.ensure_editable()?;
        self.break_undo_merge();
        if query.is_empty() {
            return Ok(0);
        }

        let matches = self.find_all_with_options(query, options);
        if matches.is_empty() {
            return Ok(0);
        }

        let new_text = normalize_edit_text(new_text);
        let before_cursor = self.cursor.position;
        let before_selection = self.selection;
        let mut replacements = Vec::with_capacity(matches.len());
        for item in &matches {
            replacements.push((item.range, self.text_in_range(item.range)?));
        }

        let mut edits = Vec::with_capacity(replacements.len());
        for (range, old_text) in replacements.into_iter().rev() {
            self.replace_range_inner(range, &new_text)?;
            edits.push(TextEdit::Replace {
                range,
                old_text,
                new_text: new_text.clone(),
            });
        }

        let after_cursor = matches
            .first()
            .map(|item| end_position_after_text(item.range.start, &new_text))
            .unwrap_or(before_cursor);
        self.cursor = Cursor::new(after_cursor);
        self.selection = None;
        self.undo_stack.push(EditTransaction {
            edits,
            before_cursor,
            after_cursor,
            before_selection,
            after_selection: None,
            merge_kind: EditMergeKind::None,
        });
        self.redo_stack.clear();
        self.bump_revision();

        Ok(matches.len())
    }
}

fn search_line_matches(line: &str, query: &str, options: SearchOptions) -> Vec<(usize, usize)> {
    if options.case_sensitive {
        return line
            .match_indices(query)
            .filter_map(|(start, text)| {
                let end = start + text.len();
                search_options_accept_match(line, start, end, options).then_some((start, end))
            })
            .collect();
    }

    let query = query.to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let (folded, byte_map) = folded_line_with_byte_map(line);
    folded
        .match_indices(&query)
        .filter_map(|(folded_start, text)| {
            let folded_end = folded_start + text.len();
            let start = *byte_map.get(folded_start)?;
            let end = *byte_map.get(folded_end)?;
            search_options_accept_match(line, start, end, options).then_some((start, end))
        })
        .collect()
}

fn folded_line_with_byte_map(line: &str) -> (String, Vec<usize>) {
    let mut folded = String::new();
    let mut byte_map = Vec::new();
    byte_map.push(0);

    for (start, ch) in line.char_indices() {
        let end = start + ch.len_utf8();
        let lower = ch.to_lowercase().collect::<String>();
        for offset in 0..lower.len() {
            if offset == 0 {
                byte_map.push(start);
            } else {
                byte_map.push(end);
            }
        }
        folded.push_str(&lower);
        if lower.is_empty() {
            byte_map.push(end);
        } else if let Some(last) = byte_map.last_mut() {
            *last = end;
        }
    }

    if byte_map.len() < folded.len() + 1 {
        byte_map.resize(folded.len() + 1, line.len());
    }
    if let Some(last) = byte_map.last_mut() {
        *last = line.len();
    }

    (folded, byte_map)
}

fn search_options_accept_match(
    line: &str,
    start: usize,
    end: usize,
    options: SearchOptions,
) -> bool {
    if !options.whole_word {
        return true;
    }

    !previous_char_is_word(line, start) && !next_char_is_word(line, end)
}

fn previous_char_is_word(line: &str, index: usize) -> bool {
    line.get(..index)
        .and_then(|prefix| prefix.chars().next_back())
        .is_some_and(is_search_word_char)
}

fn next_char_is_word(line: &str, index: usize) -> bool {
    line.get(index..)
        .and_then(|suffix| suffix.chars().next())
        .is_some_and(is_search_word_char)
}

fn is_search_word_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}
