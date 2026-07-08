use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SearchSpec {
    pub(crate) input: String,
    pub(crate) query: String,
    pub(crate) options: SearchOptions,
}

impl SearchSpec {
    pub(crate) fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        let mut options = SearchOptions::default();

        if let Some(rest) = trimmed.strip_prefix('/') {
            let (flags, query) = rest
                .find(char::is_whitespace)
                .map(|index| (&rest[..index], rest[index..].trim_start()))
                .unwrap_or((rest, ""));
            if !flags.is_empty()
                && !query.is_empty()
                && flags
                    .chars()
                    .all(|ch| matches!(ch, 'i' | 'I' | 'c' | 'C' | 'w' | 'W'))
            {
                for flag in flags.chars() {
                    match flag.to_ascii_lowercase() {
                        'i' => options.case_sensitive = false,
                        'c' => options.case_sensitive = true,
                        'w' => options.whole_word = true,
                        _ => {}
                    }
                }
                return Self {
                    input: trimmed.to_string(),
                    query: query.to_string(),
                    options,
                };
            }
        }

        Self {
            input: trimmed.to_string(),
            query: trimmed.to_string(),
            options,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    pub(crate) fn display(&self) -> String {
        let mut flags = Vec::new();
        if !self.options.case_sensitive {
            flags.push("ignore-case");
        }
        if self.options.whole_word {
            flags.push("whole-word");
        }
        if flags.is_empty() {
            self.query.clone()
        } else {
            format!("{} ({})", self.query, flags.join(", "))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SearchSelection {
    pub(crate) index: usize,
    pub(crate) wrapped: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BufferSearchState {
    pub(crate) spec: SearchSpec,
    pub(crate) matches: Vec<SearchMatch>,
    pub(crate) revision: u64,
    pub(crate) active_index: Option<usize>,
}

impl BufferSearchState {
    pub(crate) fn refresh(&mut self, buffer: &TextBuffer) {
        if self.revision == buffer.revision() {
            self.active_index = current_match_selection(buffer, &self.matches)
                .map(|selection| selection.index)
                .or_else(|| {
                    self.active_index
                        .filter(|index| *index < self.matches.len())
                });
            return;
        }

        let previous_active = self.active_index;
        self.matches = buffer.find_all_with_options(&self.spec.query, self.spec.options);
        self.revision = buffer.revision();
        self.active_index = current_match_selection(buffer, &self.matches)
            .map(|selection| selection.index)
            .or_else(|| previous_active.filter(|index| *index < self.matches.len()));
    }

    pub(crate) fn status_text(&self) -> String {
        match (self.matches.len(), self.active_index) {
            (0, _) => "Find 0".to_string(),
            (total, Some(index)) => format!("Find {}/{total}", index + 1),
            (total, None) => format!("Find {total}"),
        }
    }
}
