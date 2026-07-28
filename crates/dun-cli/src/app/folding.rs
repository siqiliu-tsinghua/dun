use crate::*;
use dun_core::FoldRange;

impl AppState {
    pub(super) fn toggle_fold(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_FOLD_BUFFER_MISSING).to_string(),
            );
            return;
        };

        if let Some(selection) = buffer.buffer.selection_range() {
            let start_line = selection.start.line;
            let end_line_exclusive =
                if selection.end.column == 0 && selection.end.line > start_line {
                    selection.end.line
                } else {
                    selection.end.line.saturating_add(1)
                }
                .min(buffer.buffer.line_count());
            let line_count = end_line_exclusive.saturating_sub(start_line);
            if line_count < 2 {
                self.set_status(
                    ui_text::tr(
                        &self.shell.catalog,
                        ui_text::STATUS_FOLD_SELECTION_TOO_SHORT,
                    )
                    .to_string(),
                );
                return;
            }

            buffer
                .buffer
                .insert_fold(FoldRange::new(start_line, end_line_exclusive));
            let _ = buffer.buffer.set_cursor(Position::new(start_line, 0));
            self.set_status(ui_text::tr_fmt(
                &self.shell.catalog,
                ui_text::STATUS_FOLD_FOLDED,
                &[&line_count.to_string()],
            ));
            return;
        }

        let cursor_line = buffer.buffer.cursor_position().line;
        if buffer.buffer.remove_fold_at(cursor_line) {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_FOLD_UNFOLDED).to_string(),
            );
        } else {
            self.set_status(
                ui_text::tr(
                    &self.shell.catalog,
                    ui_text::STATUS_FOLD_SELECTION_TOO_SHORT,
                )
                .to_string(),
            );
        }
    }

    pub(super) fn unfold_all(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_FOLD_BUFFER_MISSING).to_string(),
            );
            return;
        };

        let fold_count = buffer.buffer.folds().ranges().len();
        if fold_count == 0 {
            self.set_status(
                ui_text::tr(&self.shell.catalog, ui_text::STATUS_FOLD_NOTHING_TO_UNFOLD)
                    .to_string(),
            );
            return;
        }

        buffer.buffer.clear_folds();
        self.set_status(ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_FOLD_ALL_UNFOLDED,
            &[&fold_count.to_string()],
        ));
    }

    pub(super) fn expand_focused_fold_at_cursor(&mut self) {
        let Some(buffer) = self.focused_buffer_mut() else {
            return;
        };
        let cursor_line = buffer.buffer.cursor_position().line;
        buffer.buffer.remove_fold_at(cursor_line);
    }
}
