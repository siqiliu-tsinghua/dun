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
        format!("{}{}", self.kind.label(), self.input.as_str())
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

    pub(crate) fn status_text(&self) -> String {
        let list = self
            .candidates
            .iter()
            .map(|candidate| candidate.display.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if let Some(index) = self.active_index {
            format!(
                "Command completion: {}/{} {}",
                index + 1,
                self.candidates.len(),
                list
            )
        } else {
            format!("Command completion: {list}")
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
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::CommandLine => "Command: ",
            Self::Find => "Find: ",
            Self::ReplaceFind => "Find to replace: ",
            Self::ReplaceWith => "Replace with: ",
            Self::GoToLine => "Go To Line: ",
            Self::RunCommand => "Run Command: ",
        }
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::CommandLine => "Command",
            Self::Find => "Find",
            Self::ReplaceFind | Self::ReplaceWith => "Replace",
            Self::GoToLine => "Go To Line",
            Self::RunCommand => "Run Command",
        }
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
