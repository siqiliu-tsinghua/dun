use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

mod cursor;
mod edit;
mod line_ops;
mod model;
mod search;
mod selection;
mod undo;

pub use model::{
    BufferError, BufferId, BufferKind, Cursor, EditMergeKind, EditTransaction, LineEnding,
    Position, SearchMatch, SearchOptions, Selection, TextBuffer, TextEdit, TextRange,
};

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new_untitled()
    }
}

impl TextBuffer {
    pub fn new_untitled() -> Self {
        Self::from_parts(BufferKind::Untitled, vec![String::new()], LineEnding::Lf)
    }

    pub fn from_text(text: &str) -> Self {
        Self::from_text_with_kind(BufferKind::File, text)
    }

    pub fn from_text_with_kind(kind: BufferKind, text: &str) -> Self {
        let line_ending = detect_line_ending(text);
        let lines = parse_lines(text, line_ending);
        Self::from_parts(kind, lines, line_ending)
    }

    pub fn kind(&self) -> BufferKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: BufferKind) {
        self.kind = kind;
    }

    pub fn is_read_only(&self) -> bool {
        matches!(self.kind, BufferKind::ReadOnly)
    }

    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(String::as_str)
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn to_text(&self) -> String {
        self.lines.join(self.line_ending.as_str())
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_dirty(&self) -> bool {
        if let Some(dirty) = self.dirty_cache.get() {
            return dirty;
        }
        let dirty = self.current_fingerprint() != self.saved_fingerprint;
        self.dirty_cache.set(Some(dirty));
        dirty
    }

    pub fn mark_saved(&mut self) {
        self.saved_fingerprint = self.current_fingerprint();
        self.dirty_cache.set(Some(false));
    }

    pub(super) fn from_parts(
        kind: BufferKind,
        mut lines: Vec<String>,
        line_ending: LineEnding,
    ) -> Self {
        if lines.is_empty() {
            lines.push(String::new());
        }

        let mut buffer = Self {
            kind,
            lines,
            line_ending,
            cursor: Cursor::default(),
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            revision: 0,
            saved_fingerprint: 0,
            dirty_cache: std::cell::Cell::new(Some(false)),
        };
        buffer.saved_fingerprint = buffer.current_fingerprint();
        buffer
    }

    pub(super) fn ensure_editable(&self) -> Result<(), BufferError> {
        if self.is_read_only() {
            Err(BufferError::ReadOnly)
        } else {
            Ok(())
        }
    }

    pub(super) fn validate_position(&self, position: Position) -> Result<(), BufferError> {
        let Some(line) = self.lines.get(position.line) else {
            return Err(BufferError::InvalidPosition(position));
        };

        if position.column > line.len() || !line.is_char_boundary(position.column) {
            return Err(BufferError::InvalidPosition(position));
        }

        Ok(())
    }

    pub(super) fn validate_range(&self, range: TextRange) -> Result<(), BufferError> {
        if range.start > range.end {
            return Err(BufferError::InvalidRange(range));
        }

        self.validate_position(range.start)
            .map_err(|_| BufferError::InvalidRange(range))?;
        self.validate_position(range.end)
            .map_err(|_| BufferError::InvalidRange(range))?;
        Ok(())
    }

    fn current_fingerprint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.lines.hash(&mut hasher);
        self.line_ending.hash(&mut hasher);
        hasher.finish()
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
        self.dirty_cache.set(None);
    }
}

fn detect_line_ending(text: &str) -> LineEnding {
    match text.find('\n') {
        Some(index) if index > 0 && text.as_bytes()[index - 1] == b'\r' => LineEnding::CrLf,
        _ => LineEnding::Lf,
    }
}

fn parse_lines(text: &str, line_ending: LineEnding) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    text.split('\n')
        .map(|line| match line_ending {
            LineEnding::Lf => line.to_string(),
            LineEnding::CrLf => line.strip_suffix('\r').unwrap_or(line).to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
