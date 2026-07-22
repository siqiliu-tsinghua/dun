use crate::files::{
    PathErrorDetail, common_entry_prefix, ensure_trailing_separator, expand_user_path,
    file_dialog_context, file_dialog_list_message, is_completable_file_dialog_entry,
    list_file_dialog_entries,
};
use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileDialogMessage {
    NoMatches,
    NoVisibleMatches,
    NoMatchesForPrefix(String),
    OnlyHiddenFiltered,
    DirectoryEmpty,
    CannotList {
        directory: String,
        detail: PathErrorDetail,
    },
    HiddenFiles {
        shown: bool,
    },
    ConfirmOverwrite(String),
    /// Already-rendered text composed elsewhere *with* a catalog — the
    /// open/save failure status that `app/file_dialogs.rs` copies into the
    /// dialog. This is the one honest escape hatch; do not use it for
    /// anything a variant can express.
    Text(String),
}

impl FileDialogMessage {
    pub(crate) fn render(&self, catalog: &TextCatalog) -> String {
        match self {
            Self::NoMatches => {
                ui_text::tr(catalog, ui_text::STATUS_FILE_DIALOG_NO_MATCHES).to_string()
            }
            Self::NoVisibleMatches => {
                ui_text::tr(catalog, ui_text::STATUS_FILE_DIALOG_NO_VISIBLE_MATCHES).to_string()
            }
            Self::NoMatchesForPrefix(prefix) => ui_text::tr_fmt(
                catalog,
                ui_text::STATUS_FILE_DIALOG_NO_MATCHES_FOR_PREFIX,
                &[prefix],
            ),
            Self::OnlyHiddenFiltered => {
                ui_text::tr(catalog, ui_text::STATUS_FILE_DIALOG_ONLY_HIDDEN_FILTERED).to_string()
            }
            Self::DirectoryEmpty => {
                ui_text::tr(catalog, ui_text::STATUS_FILE_DIALOG_DIRECTORY_EMPTY).to_string()
            }
            Self::CannotList { directory, detail } => {
                let detail = detail.render(catalog);
                ui_text::tr_fmt(
                    catalog,
                    ui_text::STATUS_FILE_DIALOG_CANNOT_LIST,
                    &[directory, &detail],
                )
            }
            Self::HiddenFiles { shown } => {
                let key = if *shown {
                    ui_text::STATUS_FILE_DIALOG_HIDDEN_SHOWN
                } else {
                    ui_text::STATUS_FILE_DIALOG_HIDDEN_HIDDEN
                };
                ui_text::tr(catalog, key).to_string()
            }
            Self::ConfirmOverwrite(path) => ui_text::tr_fmt(
                catalog,
                ui_text::STATUS_FILE_DIALOG_CONFIRM_OVERWRITE,
                &[path],
            ),
            Self::Text(text) => text.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileDialogState {
    pub(crate) kind: FileDialogKind,
    pub(crate) input: LineInput,
    pub(crate) entries: Vec<FileDialogEntry>,
    pub(crate) selected_index: Option<usize>,
    pub(crate) scroll_offset: usize,
    pub(crate) show_hidden: bool,
    pub(crate) selection_touched: bool,
    pub(crate) message: Option<FileDialogMessage>,
    pub(crate) after_success: Option<PendingAction>,
    pub(crate) overwrite_path: Option<PathBuf>,
}

impl FileDialogState {
    pub(crate) fn new(
        kind: FileDialogKind,
        input: String,
        after_success: Option<PendingAction>,
    ) -> Self {
        let mut state = Self {
            kind,
            input: LineInput::new(input),
            entries: Vec::new(),
            selected_index: None,
            scroll_offset: 0,
            show_hidden: false,
            selection_touched: false,
            message: None,
            after_success,
            overwrite_path: None,
        };
        state.refresh_entries();
        state
    }

    #[cfg(test)]
    pub(crate) fn status_text(&self) -> String {
        let label = match self.kind {
            FileDialogKind::Open => "Open: ",
            FileDialogKind::SaveAs => "Save As: ",
        };
        format!("{label}{}", self.input.as_str())
    }

    pub(crate) fn overlay(
        &self,
        keymap: &FileDialogKeymap,
        catalog: &TextCatalog,
        mode: AmbiguousWidth,
    ) -> UiOverlay {
        let context = file_dialog_context(self.input.as_str());
        let hidden_state = if self.show_hidden {
            ui_text::tr(catalog, ui_text::DIALOG_HIDDEN_SHOWN)
        } else if context.prefix.starts_with('.') {
            ui_text::tr(catalog, ui_text::DIALOG_HIDDEN_BY_PREFIX)
        } else {
            ui_text::tr(catalog, ui_text::DIALOG_HIDDEN_HIDDEN)
        };
        let hidden_key = file_dialog_action_key_text(keymap, FileDialogAction::ToggleHidden);
        let entry_count = self.entries.iter().filter(|entry| !entry.is_parent).count();
        let mut lines = vec![
            ui_text::tr_fmt(
                catalog,
                ui_text::DIALOG_LOOK_IN,
                &[&context.directory.display().to_string()],
            ),
            format!("{}:", self.kind.input_label(catalog)),
            self.message
                .as_ref()
                .map(|message| message.render(catalog))
                .unwrap_or_else(|| self.kind.help_text(entry_count, catalog)),
            ui_text::tr_fmt(
                catalog,
                ui_text::DIALOG_HIDDEN,
                &[hidden_state, &hidden_key],
            ),
        ];
        if self.entries.len() > FILE_DIALOG_VISIBLE_ENTRIES {
            if let Some((start, end, _)) = self.visible_entry_range() {
                lines.push(ui_text::tr_fmt(
                    catalog,
                    ui_text::DIALOG_SHOWING_MATCHES,
                    &[
                        &(start + 1).to_string(),
                        &end.to_string(),
                        &self.entries.len().to_string(),
                    ],
                ));
            }
        }

        let (list, selected) = self.visible_entry_texts(catalog);
        let mut overlay = UiOverlay::file_dialog(
            self.kind.name(catalog),
            lines,
            self.input.as_str().to_string(),
            self.input.cursor_display_column(mode),
            list,
            selected,
            vec![file_dialog_shortcuts_text(keymap, catalog)],
        );
        if let Some((start, end, _)) = self.visible_entry_range() {
            overlay = overlay.with_list_overflow(start > 0, end < self.entries.len());
        }
        overlay
    }

    pub(crate) fn refresh_entries(&mut self) {
        let context = file_dialog_context(self.input.as_str());
        match list_file_dialog_entries(&context, self.show_hidden) {
            Ok(listing) => {
                self.message = file_dialog_list_message(
                    &context,
                    &listing.entries,
                    self.show_hidden,
                    listing.hidden_filtered,
                );
                self.entries = listing.entries;
                self.selected_index = if self.entries.is_empty() {
                    None
                } else {
                    Some(0)
                };
                self.scroll_offset = 0;
                self.selection_touched = false;
            }
            Err(error) => {
                self.entries.clear();
                self.selected_index = None;
                self.scroll_offset = 0;
                self.selection_touched = false;
                self.message = Some(FileDialogMessage::CannotList {
                    directory: context.directory.display().to_string(),
                    detail: PathErrorDetail::classify(&error),
                });
            }
        }
    }

    pub(crate) fn visible_entry_texts(
        &self,
        catalog: &TextCatalog,
    ) -> (Vec<String>, Option<usize>) {
        let Some((start, end, selected)) = self.visible_entry_range() else {
            return (
                vec![ui_text::tr(catalog, ui_text::DIALOG_NO_MATCHES).to_string()],
                None,
            );
        };
        let list = self.entries[start..end]
            .iter()
            .map(|entry| entry.display_text(catalog))
            .collect::<Vec<_>>();
        let selected = if (start..end).contains(&selected) {
            Some(selected - start)
        } else {
            None
        };
        (list, selected)
    }

    pub(crate) fn visible_entry_range(&self) -> Option<(usize, usize, usize)> {
        if self.entries.is_empty() {
            return None;
        }

        let selected = self
            .selected_index
            .filter(|index| *index < self.entries.len())
            .unwrap_or_else(|| self.scroll_offset.min(self.entries.len().saturating_sub(1)));
        let start = self.scroll_offset.min(self.max_scroll_offset());
        let end = start
            .saturating_add(FILE_DIALOG_VISIBLE_ENTRIES)
            .min(self.entries.len());
        Some((start, end, selected))
    }

    pub(crate) fn entry_index_for_visible_index(&self, visible_index: usize) -> Option<usize> {
        let (start, end, _) = self.visible_entry_range()?;
        let index = start.saturating_add(visible_index);
        (index < end).then_some(index)
    }

    pub(crate) fn max_scroll_offset(&self) -> usize {
        self.entries
            .len()
            .saturating_sub(FILE_DIALOG_VISIBLE_ENTRIES)
    }

    pub(crate) fn ensure_selection_visible(&mut self) {
        let Some(selected) = self.selected_index else {
            self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
            return;
        };

        if selected < self.scroll_offset {
            self.scroll_offset = selected;
        } else {
            let visible_end = self
                .scroll_offset
                .saturating_add(FILE_DIALOG_VISIBLE_ENTRIES);
            if selected >= visible_end {
                self.scroll_offset = selected
                    .saturating_add(1)
                    .saturating_sub(FILE_DIALOG_VISIBLE_ENTRIES);
            }
        }
        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
    }

    pub(crate) fn clamp_selection_to_visible_range(&mut self) {
        if self.entries.is_empty() {
            self.selected_index = None;
            self.scroll_offset = 0;
            return;
        }

        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
        let start = self.scroll_offset;
        let end = start
            .saturating_add(FILE_DIALOG_VISIBLE_ENTRIES)
            .min(self.entries.len());
        let selected = self.selected_index.unwrap_or(start);
        self.selected_index = Some(selected.clamp(start, end.saturating_sub(1)));
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        if self.entries.is_empty() {
            self.selected_index = None;
            self.message = Some(FileDialogMessage::NoMatches);
            return;
        }

        let current = self.selected_index.unwrap_or(0);
        self.selected_index = Some(wrapping_index(current, self.entries.len(), delta));
        self.ensure_selection_visible();
        self.selection_touched = true;
        self.message = None;
    }

    pub(crate) fn page_selection(&mut self, delta: isize) {
        if self.entries.is_empty() {
            self.selected_index = None;
            self.scroll_offset = 0;
            self.message = Some(FileDialogMessage::NoMatches);
            return;
        }

        let current = self.selected_index.unwrap_or(0);
        let page = FILE_DIALOG_VISIBLE_ENTRIES.saturating_sub(1).max(1) as isize;
        let next = current
            .saturating_add_signed(delta.saturating_mul(page))
            .min(self.entries.len().saturating_sub(1));
        self.selected_index = Some(next);
        self.ensure_selection_visible();
        self.selection_touched = true;
        self.message = None;
    }

    pub(crate) fn scroll(&mut self, delta: isize) {
        if self.entries.is_empty() {
            self.selected_index = None;
            self.scroll_offset = 0;
            return;
        }

        self.scroll_offset = self
            .scroll_offset
            .saturating_add_signed(delta)
            .min(self.max_scroll_offset());
        self.clamp_selection_to_visible_range();
        self.selection_touched = true;
        self.message = None;
    }

    pub(crate) fn move_input_left(&mut self) {
        self.input.move_left();
        self.message = None;
    }

    pub(crate) fn move_input_right(&mut self) {
        self.input.move_right();
        self.message = None;
    }

    pub(crate) fn move_input_start(&mut self) {
        self.input.move_start();
        self.message = None;
    }

    pub(crate) fn move_input_end(&mut self) {
        self.input.move_end();
        self.message = None;
    }

    pub(crate) fn insert_char(&mut self, ch: char) {
        self.overwrite_path = None;
        self.input.insert_char(ch);
        self.refresh_entries();
    }

    pub(crate) fn insert_text(&mut self, text: &str) {
        self.overwrite_path = None;
        self.input.insert_str(text);
        self.refresh_entries();
    }

    pub(crate) fn delete_backward(&mut self) {
        self.overwrite_path = None;
        self.input.delete_backward();
        self.refresh_entries();
    }

    pub(crate) fn delete_forward(&mut self) {
        self.overwrite_path = None;
        self.input.delete_forward();
        self.refresh_entries();
    }

    pub(crate) fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.refresh_entries();
        self.message = Some(FileDialogMessage::HiddenFiles {
            shown: self.show_hidden,
        });
    }

    pub(crate) fn complete(&mut self, forward: bool) {
        if self.entries.is_empty() {
            self.message = Some(FileDialogMessage::NoMatches);
            return;
        }

        let context = file_dialog_context(self.input.as_str());
        let completion_indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| is_completable_file_dialog_entry(entry, &context.prefix))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        if completion_indices.is_empty() {
            self.message = Some(FileDialogMessage::NoMatches);
            return;
        }

        if completion_indices.len() == 1 {
            self.apply_entry(completion_indices[0]);
            return;
        }

        if let Some(prefix) = common_entry_prefix(&self.entries, &context.prefix) {
            if prefix.len() > context.prefix.len() {
                self.input
                    .set_text(format!("{}{}", context.base_input, prefix));
                self.refresh_entries();
                return;
            }
        }

        let selected = self
            .selected_index
            .filter(|index| completion_indices.contains(index))
            .or_else(|| {
                if forward {
                    completion_indices.first().copied()
                } else {
                    completion_indices.last().copied()
                }
            });
        let Some(selected) = selected else {
            return;
        };
        self.apply_entry(selected);
    }

    pub(crate) fn submit(&mut self) -> FileDialogSubmit {
        let input = self.input.as_str().trim().to_string();

        // The list selection is checked before the empty-input bail: picking a
        // file with the arrows leaves the name field empty, so bailing first
        // meant Enter on an arrow-selected file cancelled the dialog instead of
        // opening it, and the branch below was unreachable in the one case it
        // exists for.
        if self.kind == FileDialogKind::Open {
            if let Some(index) = self
                .selected_index
                .filter(|index| *index < self.entries.len())
            {
                let entry = self.entries[index].clone();
                let context = file_dialog_context(self.input.as_str());
                let should_use_selected = self.selection_touched
                    || entry.name == context.prefix
                    || entry.is_parent && context.prefix == "..";
                if should_use_selected {
                    if entry.is_dir {
                        self.apply_entry(index);
                        return FileDialogSubmit::ContinueEditing;
                    }
                    return FileDialogSubmit::Path(entry.path);
                }
            }

            if input.is_empty() {
                return FileDialogSubmit::Cancel;
            }

            let path = expand_user_path(&input);
            if path.is_dir() {
                self.input
                    .set_text(ensure_trailing_separator(input.to_string()));
                self.refresh_entries();
                return FileDialogSubmit::ContinueEditing;
            }
            return FileDialogSubmit::Path(path);
        }

        if input.is_empty() {
            return FileDialogSubmit::Cancel;
        }

        let path = expand_user_path(&input);
        if self.kind.confirms_overwrite()
            && path.exists()
            && !path.is_dir()
            && self.overwrite_path.as_ref() != Some(&path)
        {
            self.overwrite_path = Some(path.clone());
            self.message = Some(FileDialogMessage::ConfirmOverwrite(
                path.display().to_string(),
            ));
            return FileDialogSubmit::ContinueEditing;
        }

        FileDialogSubmit::Path(path)
    }

    pub(crate) fn click_visible_entry(&mut self, visible_index: usize) -> FileDialogSubmit {
        let Some(index) = self.entry_index_for_visible_index(visible_index) else {
            return FileDialogSubmit::ContinueEditing;
        };
        let Some(entry) = self.entries.get(index).cloned() else {
            return FileDialogSubmit::ContinueEditing;
        };

        self.selected_index = Some(index);
        self.selection_touched = true;
        match self.kind {
            FileDialogKind::Open => {
                if entry.is_dir {
                    self.apply_entry(index);
                    FileDialogSubmit::ContinueEditing
                } else {
                    FileDialogSubmit::Path(entry.path)
                }
            }
            FileDialogKind::SaveAs => {
                self.apply_entry(index);
                FileDialogSubmit::ContinueEditing
            }
        }
    }

    pub(crate) fn apply_entry(&mut self, index: usize) {
        let Some(entry) = self.entries.get(index) else {
            return;
        };
        self.overwrite_path = None;
        self.input.set_text(entry.input.clone());
        self.refresh_entries();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileDialogKind {
    Open,
    SaveAs,
}

impl FileDialogKind {
    pub(crate) fn name(self, catalog: &TextCatalog) -> &str {
        match self {
            Self::Open => ui_text::tr(catalog, ui_text::DIALOG_OPEN_TITLE),
            Self::SaveAs => ui_text::tr(catalog, ui_text::DIALOG_SAVE_AS_TITLE),
        }
    }

    pub(crate) fn input_label(self, catalog: &TextCatalog) -> &str {
        match self {
            Self::Open => ui_text::tr(catalog, ui_text::DIALOG_OPEN_INPUT_LABEL),
            Self::SaveAs => ui_text::tr(catalog, ui_text::DIALOG_SAVE_AS_INPUT_LABEL),
        }
    }

    pub(crate) const fn confirms_overwrite(self) -> bool {
        matches!(self, Self::SaveAs)
    }

    /// English keeps its singular/plural branching; a translation supplies
    /// one `{}` template used for every count.
    pub(crate) fn help_text(self, entry_count: usize, catalog: &TextCatalog) -> String {
        let key = match self {
            Self::Open => ui_text::DIALOG_OPEN_HELP,
            Self::SaveAs => ui_text::DIALOG_SAVE_AS_HELP,
        };
        if let Some(template) = ui_text::tr_template(catalog, key, 1) {
            return ui_text::substitute(template, &[&entry_count.to_string()]);
        }
        let noun = if entry_count == 1 { "entry" } else { "entries" };
        match self {
            Self::Open => format!("Select a file or type a path. {entry_count} {noun}."),
            Self::SaveAs => format!("Type the destination path. {entry_count} {noun}."),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileDialogEntry {
    pub(crate) name: String,
    pub(crate) input: String,
    pub(crate) path: PathBuf,
    pub(crate) is_dir: bool,
    pub(crate) is_parent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileDialogListing {
    pub(crate) entries: Vec<FileDialogEntry>,
    pub(crate) hidden_filtered: usize,
}

impl FileDialogEntry {
    pub(crate) fn display_text(&self, catalog: &TextCatalog) -> String {
        if self.is_parent {
            ui_text::tr(catalog, ui_text::DIALOG_PARENT_DIR).to_string()
        } else if self.is_dir {
            format!("[DIR]  {}/", self.name)
        } else {
            format!("       {}", self.name)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileDialogSubmit {
    Cancel,
    ContinueEditing,
    Path(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileDialogContext {
    pub(crate) base_input: String,
    pub(crate) prefix: String,
    pub(crate) directory: PathBuf,
}
