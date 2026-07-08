use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BufferSwitcherState {
    pub(crate) selected_index: usize,
    pub(crate) scroll_offset: usize,
}

impl BufferSwitcherState {
    pub(crate) fn new(selected_index: usize, total: usize) -> Self {
        let mut state = Self {
            selected_index: selected_index.min(total.saturating_sub(1)),
            scroll_offset: 0,
        };
        state.ensure_selected_visible(total);
        state
    }

    pub(crate) fn selected_index(&self, total: usize) -> Option<usize> {
        (total > 0).then_some(self.selected_index.min(total - 1))
    }

    pub(crate) fn move_selection(&mut self, delta: isize, total: usize) {
        if total == 0 {
            self.selected_index = 0;
            self.scroll_offset = 0;
            return;
        }

        self.selected_index = if delta < 0 {
            self.selected_index.saturating_sub(delta.unsigned_abs())
        } else {
            self.selected_index
                .saturating_add(delta as usize)
                .min(total - 1)
        };
        self.ensure_selected_visible(total);
    }

    pub(crate) fn page_selection(&mut self, delta: isize, total: usize) {
        let step = BUFFER_SWITCHER_VISIBLE_ENTRIES.saturating_sub(1).max(1) as isize;
        self.move_selection(delta.saturating_mul(step), total);
    }

    pub(crate) fn select_first(&mut self, total: usize) {
        if total == 0 {
            self.selected_index = 0;
            self.scroll_offset = 0;
            return;
        }

        self.selected_index = 0;
        self.ensure_selected_visible(total);
    }

    pub(crate) fn select_last(&mut self, total: usize) {
        if total == 0 {
            self.selected_index = 0;
            self.scroll_offset = 0;
            return;
        }

        self.selected_index = total - 1;
        self.ensure_selected_visible(total);
    }

    pub(crate) fn select_visible_index(
        &mut self,
        visible_index: usize,
        total: usize,
    ) -> Option<usize> {
        let index = self.scroll_offset.saturating_add(visible_index);
        if index < total {
            self.selected_index = index;
            self.ensure_selected_visible(total);
            Some(index)
        } else {
            None
        }
    }

    pub(crate) fn visible_entry_texts(
        &self,
        entries: &[BufferSwitcherEntry],
    ) -> (Vec<String>, Option<usize>) {
        let Some((start, end, selected)) = self.visible_entry_range(entries.len()) else {
            return (vec!["(no buffers)".to_string()], None);
        };
        let list = entries[start..end]
            .iter()
            .map(|entry| entry.text.clone())
            .collect::<Vec<_>>();
        let selected = if (start..end).contains(&selected) {
            Some(selected - start)
        } else {
            None
        };
        (list, selected)
    }

    pub(crate) fn visible_entry_range(&self, total: usize) -> Option<(usize, usize, usize)> {
        if total == 0 {
            return None;
        }

        let selected = self.selected_index.min(total - 1);
        let start = self.scroll_offset.min(total - 1);
        let end = start
            .saturating_add(BUFFER_SWITCHER_VISIBLE_ENTRIES)
            .min(total);
        Some((start, end, selected))
    }

    pub(crate) fn ensure_selected_visible(&mut self, total: usize) {
        if total == 0 {
            self.selected_index = 0;
            self.scroll_offset = 0;
            return;
        }

        self.selected_index = self.selected_index.min(total - 1);
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index
            >= self
                .scroll_offset
                .saturating_add(BUFFER_SWITCHER_VISIBLE_ENTRIES)
        {
            self.scroll_offset = self
                .selected_index
                .saturating_sub(BUFFER_SWITCHER_VISIBLE_ENTRIES.saturating_sub(1));
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BufferSwitcherEntry {
    pub(crate) buffer_id: BufferId,
    pub(crate) text: String,
}
