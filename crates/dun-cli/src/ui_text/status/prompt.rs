use super::TextKey;

// Prompt and command-completion status messages.
pub(crate) const STATUS_PROMPT_TYPE_TO_SEARCH: TextKey =
    ("status.prompt.type-to-search", "{}type to search");
pub(crate) const STATUS_PROMPT_BUFFER_MISSING: TextKey = (
    "status.prompt.buffer-missing",
    "{}focused buffer is missing",
);
pub(crate) const STATUS_PROMPT_NO_MATCHES: TextKey =
    ("status.prompt.no-matches", "{}no matches for {}");
pub(crate) const STATUS_PROMPT_MATCH: TextKey = ("status.prompt.match", "{}{}/{} {}");
pub(crate) const STATUS_PROMPT_CANCELLED: TextKey = ("status.prompt.cancelled", "{} cancelled");
pub(crate) const STATUS_COMPLETION_CURSOR_END: TextKey = (
    "status.completion.cursor-end",
    "Command completion: move cursor to end",
);
pub(crate) const STATUS_COMPLETION_NO_MATCHES: TextKey = (
    "status.completion.no-matches",
    "Command completion: no matches",
);
pub(crate) const STATUS_COMPLETION_READY: TextKey =
    ("status.completion.ready", "Command completion");
pub(crate) const STATUS_COMPLETION_MATCHES: TextKey = (
    "status.completion.matches",
    "Command completion: {} matches",
);
pub(crate) const STATUS_COMPLETION_CANDIDATES: TextKey =
    ("status.completion.candidates", "Command completion: {}");
pub(crate) const STATUS_COMPLETION_SELECTED: TextKey =
    ("status.completion.selected", "Command completion: {}/{} {}");

#[cfg(test)]
pub(crate) const ALL: &[TextKey] = &[
    STATUS_PROMPT_TYPE_TO_SEARCH,
    STATUS_PROMPT_BUFFER_MISSING,
    STATUS_PROMPT_NO_MATCHES,
    STATUS_PROMPT_MATCH,
    STATUS_PROMPT_CANCELLED,
    STATUS_COMPLETION_CURSOR_END,
    STATUS_COMPLETION_NO_MATCHES,
    STATUS_COMPLETION_READY,
    STATUS_COMPLETION_MATCHES,
    STATUS_COMPLETION_CANDIDATES,
    STATUS_COMPLETION_SELECTED,
];
