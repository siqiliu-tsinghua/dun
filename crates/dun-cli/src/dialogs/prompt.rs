use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PromptState {
    pub(crate) kind: PromptKind,
    pub(crate) input: LineInput,
    pub(crate) preview: Option<PromptPreviewState>,
    pub(crate) history_index: Option<usize>,
    pub(crate) history_draft: String,
    pub(crate) completion: Option<PromptCompletionState>,
}

impl PromptState {
    pub(crate) fn new(
        kind: PromptKind,
        input: String,
        preview: Option<PromptPreviewState>,
    ) -> Self {
        Self {
            kind,
            input: LineInput::new(input),
            preview,
            history_index: None,
            history_draft: String::new(),
            completion: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn status_text(&self) -> String {
        let label = match self.kind {
            PromptKind::CommandLine => "Command: ",
            PromptKind::Find => "Find: ",
            PromptKind::ReplaceFind => "Find to replace: ",
            PromptKind::ReplaceWith => "Replace with: ",
            PromptKind::GoToLine => "Go To Line: ",
            PromptKind::RunCommand => "Run Command: ",
        };
        format!("{label}{}", self.input.as_str())
    }

    pub(crate) fn detach_history(&mut self) {
        if self.kind == PromptKind::CommandLine {
            self.history_index = None;
            self.history_draft.clear();
        }
    }

    pub(crate) fn clear_completion(&mut self) {
        self.completion = None;
    }

    pub(crate) fn next_completion_replacement(
        &mut self,
        input: &str,
        forward: bool,
    ) -> Option<String> {
        let completion = self.completion.as_mut()?;
        let next_index = if completion.active_index.is_none() && input == completion.base_input {
            if forward {
                0
            } else {
                completion.candidates.len().saturating_sub(1)
            }
        } else {
            let current_index = completion
                .active_index
                .or_else(|| {
                    completion
                        .candidates
                        .iter()
                        .position(|candidate| candidate.replacement == input)
                })
                .filter(|_| {
                    completion
                        .candidates
                        .iter()
                        .any(|candidate| candidate.replacement == input)
                })?;
            wrapping_index(
                current_index,
                completion.candidates.len(),
                if forward { 1 } else { -1 },
            )
        };
        completion.active_index = Some(next_index);
        Some(completion.candidates[next_index].replacement.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PromptCompletionState {
    pub(crate) base_input: String,
    pub(crate) candidates: Vec<CommandCompletionCandidate>,
    pub(crate) active_index: Option<usize>,
}

impl PromptCompletionState {
    pub(crate) fn new(base_input: String, candidates: Vec<CommandCompletionCandidate>) -> Self {
        Self {
            base_input,
            candidates,
            active_index: None,
        }
    }

    pub(crate) fn status_text(&self, catalog: &TextCatalog) -> String {
        let list = self
            .candidates
            .iter()
            .map(|candidate| candidate.display.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if let Some(index) = self.active_index {
            ui_text::tr_fmt(
                catalog,
                ui_text::STATUS_COMPLETION_SELECTED,
                &[
                    &(index + 1).to_string(),
                    &self.candidates.len().to_string(),
                    &list,
                ],
            )
        } else {
            ui_text::tr_fmt(catalog, ui_text::STATUS_COMPLETION_CANDIDATES, &[&list])
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PromptPreviewState {
    pub(crate) buffer_id: BufferId,
    pub(crate) cursor: Position,
    pub(crate) selection: Option<Selection>,
    pub(crate) search: Option<BufferSearchState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PromptKind {
    CommandLine,
    Find,
    ReplaceFind,
    ReplaceWith,
    GoToLine,
    RunCommand,
}

impl PromptKind {
    pub(crate) const fn history_kind(self) -> Option<PromptHistoryKind> {
        match self {
            Self::CommandLine => Some(PromptHistoryKind::CommandLine),
            Self::RunCommand => Some(PromptHistoryKind::RunCommand),
            Self::Find | Self::ReplaceFind | Self::ReplaceWith | Self::GoToLine => None,
        }
    }

    /// The prefix a prompt's status messages carry, e.g. "Find: no matches for
    /// foo". It is not drawn in the modal — the modal has a title — so it has
    /// to read as a sentence opener on its own. "Replace Find:" did not: the
    /// status line said "Replace Find: type to search", which is not English.
    pub(crate) fn label(self, catalog: &TextCatalog) -> &str {
        let key = match self {
            Self::CommandLine => ui_text::PROMPT_COMMAND_LABEL,
            Self::Find => ui_text::PROMPT_FIND_LABEL,
            Self::ReplaceFind => ui_text::PROMPT_REPLACE_FIND_LABEL,
            Self::ReplaceWith => ui_text::PROMPT_REPLACE_WITH_LABEL,
            Self::GoToLine => ui_text::PROMPT_GO_TO_LINE_LABEL,
            Self::RunCommand => ui_text::PROMPT_RUN_COMMAND_LABEL,
        };
        ui_text::tr(catalog, key)
    }

    pub(crate) fn name(self, catalog: &TextCatalog) -> &str {
        let key = match self {
            Self::CommandLine => ui_text::PROMPT_COMMAND_NAME,
            Self::Find => ui_text::PROMPT_FIND_NAME,
            Self::ReplaceFind | Self::ReplaceWith => ui_text::PROMPT_REPLACE_TITLE,
            Self::GoToLine => ui_text::PROMPT_GO_TO_LINE_NAME,
            Self::RunCommand => ui_text::PROMPT_RUN_COMMAND_NAME,
        };
        ui_text::tr(catalog, key)
    }

    pub(crate) const fn is_replace(self) -> bool {
        matches!(self, Self::ReplaceFind | Self::ReplaceWith)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PromptHistoryKind {
    CommandLine,
    RunCommand,
}
