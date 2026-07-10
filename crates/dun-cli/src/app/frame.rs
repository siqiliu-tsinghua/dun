use crate::*;

impl AppState {
    pub(crate) fn buffer_views(&self) -> Vec<BufferView<'_>> {
        self.buffers
            .iter()
            .map(|buffer| {
                let search_matches = buffer
                    .search
                    .as_ref()
                    .map(|search| search.matches.as_slice())
                    .unwrap_or(&[]);
                let active_search_match = buffer
                    .search
                    .as_ref()
                    .and_then(|search| search.active_index);
                BufferView::scrolled_xy(
                    buffer.id,
                    &buffer.buffer,
                    buffer.first_line,
                    buffer.first_column,
                )
                .with_first_visual_row(buffer.first_visual_row)
                .with_search(search_matches, active_search_match)
                .with_wrap(buffer.word_wrap)
                .with_highlight_spans(
                    buffer
                        .highlight
                        .as_ref()
                        .filter(|highlight| highlight.revision == buffer.buffer.revision())
                        .map(|highlight| highlight.spans.as_slice())
                        .unwrap_or(&[]),
                )
            })
            .collect()
    }

    pub(crate) fn refresh_search_caches(&mut self) {
        for buffer in &mut self.buffers {
            buffer.refresh_search_cache();
        }
    }

    pub(crate) const fn mouse_enabled(&self) -> bool {
        self.mouse_enabled
    }

    pub(crate) fn sync_view_for_area(&mut self, area: Rect) {
        self.workspace_area = area;
        self.refresh_search_caches();
        let Some(context) = self.focused_buffer_view_context(area) else {
            return;
        };
        let Some(buffer) = self.buffer_state_mut(context.buffer_id) else {
            return;
        };

        buffer.ensure_cursor_visible(context.body_height, context.body_width);
    }
}
