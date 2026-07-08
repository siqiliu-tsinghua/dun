use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LineInput {
    pub(crate) text: String,
    pub(crate) cursor_index: usize,
}

impl LineInput {
    pub(crate) fn new(text: String) -> Self {
        Self {
            cursor_index: text.len(),
            text,
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }

    pub(crate) fn set_text(&mut self, text: String) {
        self.cursor_index = text.len();
        self.text = text;
    }

    pub(crate) fn cursor_display_column(&self) -> usize {
        self.text
            .get(..self.cursor_index)
            .map(UnicodeWidthStr::width)
            .unwrap_or_else(|| UnicodeWidthStr::width(self.text.as_str()))
    }

    pub(crate) fn move_left(&mut self) {
        if self.cursor_index > 0 {
            self.cursor_index = previous_char_boundary(&self.text, self.cursor_index);
        }
    }

    pub(crate) fn move_right(&mut self) {
        if self.cursor_index < self.text.len() {
            self.cursor_index = next_char_boundary(&self.text, self.cursor_index);
        }
    }

    pub(crate) fn move_start(&mut self) {
        self.cursor_index = 0;
    }

    pub(crate) fn move_end(&mut self) {
        self.cursor_index = self.text.len();
    }

    pub(crate) fn insert_char(&mut self, ch: char) {
        self.text.insert(self.cursor_index, ch);
        self.cursor_index += ch.len_utf8();
    }

    pub(crate) fn insert_str(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.text.insert_str(self.cursor_index, text);
        self.cursor_index += text.len();
    }

    pub(crate) fn delete_backward(&mut self) {
        if self.cursor_index == 0 {
            return;
        }

        let start = previous_char_boundary(&self.text, self.cursor_index);
        self.text.drain(start..self.cursor_index);
        self.cursor_index = start;
    }

    pub(crate) fn delete_forward(&mut self) {
        if self.cursor_index >= self.text.len() {
            return;
        }

        let end = next_char_boundary(&self.text, self.cursor_index);
        self.text.drain(self.cursor_index..end);
    }
}
