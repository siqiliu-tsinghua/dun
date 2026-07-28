use crate::*;
use dun_ui::{EditorLineDisplay, FoldSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferViewContext {
    pub(crate) buffer_id: BufferId,
    pub(crate) body_height: usize,
    pub(crate) body_width: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BufferHighlight {
    pub(crate) revision: u64,
    pub(crate) spans: Vec<BufferHighlightSpan>,
}

pub(crate) struct BufferState {
    pub(crate) id: BufferId,
    pub(crate) buffer: TextBuffer,
    pub(crate) path: Option<PathBuf>,
    pub(crate) encoding: FileTextEncoding,
    pub(crate) file_snapshot: Option<FileReadSnapshot>,
    pub(crate) first_line: usize,
    pub(crate) first_visual_row: usize,
    pub(crate) first_column: usize,
    pub(crate) folds: FoldSet,
    pub(crate) search: Option<BufferSearchState>,
    pub(crate) word_wrap: bool,
    pub(crate) visible_whitespace: bool,
    pub(crate) highlight: Option<BufferHighlight>,
}

impl BufferState {
    pub(crate) fn new(id: BufferId, buffer: TextBuffer) -> Self {
        Self {
            id,
            buffer,
            path: None,
            encoding: FileTextEncoding::Utf8,
            file_snapshot: None,
            first_line: 0,
            first_visual_row: 0,
            first_column: 0,
            folds: FoldSet::empty(),
            search: None,
            word_wrap: false,
            visible_whitespace: false,
            highlight: None,
        }
    }

    pub(crate) fn from_file(id: BufferId, path: PathBuf, loaded: LoadedTextBuffer) -> Self {
        Self {
            id,
            buffer: loaded.buffer,
            path: Some(path),
            encoding: loaded.encoding,
            file_snapshot: loaded.snapshot,
            first_line: 0,
            first_visual_row: 0,
            first_column: 0,
            folds: FoldSet::empty(),
            search: None,
            word_wrap: false,
            visible_whitespace: false,
            highlight: None,
        }
    }

    pub(crate) fn set_search(
        &mut self,
        spec: SearchSpec,
        matches: Vec<SearchMatch>,
        active_index: Option<usize>,
    ) {
        let active_index = active_index.filter(|index| *index < matches.len());
        self.search = Some(BufferSearchState {
            spec,
            matches,
            revision: self.buffer.revision(),
            active_index,
        });
    }

    pub(crate) fn refresh_search_cache(&mut self) {
        if let Some(search) = &mut self.search {
            search.refresh(&self.buffer);
        }
    }

    pub(crate) fn search_status(&self) -> Option<String> {
        let search = self.search.as_ref()?;
        (search.revision == self.buffer.revision()).then(|| search.status_text())
    }

    pub(crate) fn keep_cursor_inside_visible_lines(&mut self, body_height: usize) {
        if body_height == 0 {
            return;
        }

        let line_map = EditorLineDisplay::new(self.buffer.line_count(), &self.folds);
        let cursor = self.buffer.cursor_position();
        let Some(first_row) = line_map.placement_for_source_line(self.first_line) else {
            return;
        };
        let last_row = first_row
            .saturating_add(body_height.saturating_sub(1))
            .min(line_map.visible_row_count().saturating_sub(1));
        let cursor_row = line_map
            .placement_for_source_line(cursor.line)
            .unwrap_or(last_row);
        let target_row = cursor_row.clamp(first_row, last_row);
        let target_line = line_map
            .source_anchor_for_visible_row(target_row)
            .unwrap_or(cursor.line);
        if target_line == cursor.line {
            return;
        }

        let target_column = self.clamp_column_to_line(target_line, cursor.column);
        let _ = self
            .buffer
            .set_cursor(Position::new(target_line, target_column));
    }

    pub(crate) fn clamp_column_to_line(&self, line_index: usize, target_column: usize) -> usize {
        let Some(line) = self.buffer.line(line_index) else {
            return 0;
        };
        let mut column = target_column.min(line.len());
        while !line.is_char_boundary(column) {
            column -= 1;
        }
        column
    }
}
