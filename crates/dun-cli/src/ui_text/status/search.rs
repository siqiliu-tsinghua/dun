use super::TextKey;

// Shared helper fragments used inside complete status templates.
pub(crate) const STATUS_REPLACEMENT_EMPTY: TextKey = ("status.replacement.empty", "<empty>");

// Find, replace, search-results, and go-to-line status messages.
pub(crate) const STATUS_FIND_BUFFER_MISSING: TextKey = (
    "status.find.buffer-missing",
    "Find: focused buffer is missing",
);
pub(crate) const STATUS_FIND_NO_MATCHES: TextKey =
    ("status.find.no-matches", "Find: no matches for {}");
pub(crate) const STATUS_FIND_MATCH: TextKey = ("status.find.match", "Find: {}/{} {}");
pub(crate) const STATUS_FIND_MATCH_WRAPPED: TextKey =
    ("status.find.match-wrapped", "Find: {}/{} {} (wrapped)");
pub(crate) const STATUS_FIND_NO_QUERY: TextKey = ("status.find.no-query", "Find: no query");
pub(crate) const STATUS_REPLACE_NO_QUERY: TextKey =
    ("status.replace.no-query", "Replace: no query");
pub(crate) const STATUS_REPLACE_BUFFER_MISSING: TextKey = (
    "status.replace.buffer-missing",
    "Replace: focused buffer is missing",
);
pub(crate) const STATUS_REPLACE_CANCELLED: TextKey = (
    "status.replace.cancelled",
    "Replace cancelled: {} replaced, {} skipped",
);
pub(crate) const STATUS_REPLACE_NO_MATCHES: TextKey =
    ("status.replace.no-matches", "Replace: no matches for {}");
pub(crate) const STATUS_REPLACE_FAILED: TextKey = ("status.replace.failed", "Replace failed: {}");
pub(crate) const STATUS_REPLACE_DONE: TextKey = (
    "status.replace.done",
    "Replace done: {} replaced, {} skipped",
);
pub(crate) const STATUS_REPLACE_CONFIRM: TextKey =
    ("status.replace.confirm", "Replace confirm: {}/{} {} -> {}");
pub(crate) const STATUS_REPLACE_APPLIED_NEXT: TextKey = (
    "status.replace.applied-next",
    "Replace: {}/{} {} -> {}; next {}/{}",
);
pub(crate) const STATUS_REPLACE_APPLIED_NEXT_WRAPPED: TextKey = (
    "status.replace.applied-next-wrapped",
    "Replace: {}/{} {} -> {} (wrapped); next {}/{}",
);
pub(crate) const STATUS_REPLACE_APPLIED_DONE: TextKey = (
    "status.replace.applied-done",
    "Replace: {}/{} {} -> {}; no matches left",
);
pub(crate) const STATUS_REPLACE_APPLIED_DONE_WRAPPED: TextKey = (
    "status.replace.applied-done-wrapped",
    "Replace: {}/{} {} -> {} (wrapped); no matches left",
);
pub(crate) const STATUS_REPLACE_ALL_NO_QUERY: TextKey =
    ("status.replace-all.no-query", "Replace All: no query");
pub(crate) const STATUS_REPLACE_ALL_BUFFER_MISSING: TextKey = (
    "status.replace-all.buffer-missing",
    "Replace All: focused buffer is missing",
);
pub(crate) const STATUS_REPLACE_ALL_NO_MATCHES: TextKey = (
    "status.replace-all.no-matches",
    "Replace All: no matches for {}",
);
pub(crate) const STATUS_REPLACE_ALL_FAILED: TextKey =
    ("status.replace-all.failed", "Replace All failed: {}");
pub(crate) const STATUS_REPLACE_ALL_APPLIED: TextKey =
    ("status.replace-all.applied", "Replace All: {} {} -> {}");
pub(crate) const STATUS_REPLACE_ALL_APPLIED_REMAINING: TextKey = (
    "status.replace-all.applied-remaining",
    "Replace All: {} {} -> {}; {} matches remain",
);
pub(crate) const STATUS_RESULTS_FOCUSED_BUFFER_MISSING: TextKey = (
    "status.search-results.focused-buffer-missing",
    "Search Results: focused buffer is missing",
);
pub(crate) const STATUS_RESULTS_NO_QUERY: TextKey =
    ("status.search-results.no-query", "Search Results: no query");
pub(crate) const STATUS_RESULTS_NO_MATCHES: TextKey = (
    "status.search-results.no-matches",
    "Search Results: no matches for {}",
);
pub(crate) const STATUS_RESULTS_SOURCE_BUFFER_MISSING: TextKey = (
    "status.search-results.source-buffer-missing",
    "Search Results: source buffer is missing",
);
pub(crate) const STATUS_RESULTS_MATCHES: TextKey = (
    "status.search-results.matches",
    "Search Results: {} match(es)",
);
pub(crate) const STATUS_RESULTS_MATCH_NUMBER_EXPECTED: TextKey = (
    "status.search-results.match-number-expected",
    "Search Results: match number expected",
);
pub(crate) const STATUS_RESULTS_OUT_OF_RANGE: TextKey = (
    "status.search-results.out-of-range",
    "Search Results: match {} out of range",
);
pub(crate) const STATUS_RESULTS_SOURCE_WINDOW_MISSING: TextKey = (
    "status.search-results.source-window-missing",
    "Search Results: source window is missing",
);
pub(crate) const STATUS_RESULTS_SELECTED: TextKey =
    ("status.search-results.selected", "Search Results: {}/{} {}");
pub(crate) const STATUS_LIST_NO_ENTRIES: TextKey = ("status.list.no-entries", "{}: no entries");
pub(crate) const STATUS_LIST_SELECTED: TextKey = ("status.list.selected", "{}: selected {}/{}");
pub(crate) const STATUS_GO_TO_LINE_INVALID: TextKey = (
    "status.go-to-line.invalid",
    "Go to line failed: invalid line number {}",
);
pub(crate) const STATUS_GO_TO_LINE_STARTS_AT_ONE: TextKey = (
    "status.go-to-line.starts-at-one",
    "Go to line failed: line numbers start at 1",
);
pub(crate) const STATUS_GO_TO_LINE_BUFFER_MISSING: TextKey = (
    "status.go-to-line.buffer-missing",
    "Go to line failed: focused buffer is missing",
);
pub(crate) const STATUS_GO_TO_LINE_PAST_END: TextKey = (
    "status.go-to-line.past-end",
    "Go to line failed: line {} is past end ({} lines)",
);
pub(crate) const STATUS_GO_TO_LINE_MOVED: TextKey = ("status.go-to-line.moved", "Go to line: {}");
pub(crate) const STATUS_GO_TO_LINE_FAILED: TextKey =
    ("status.go-to-line.failed", "Go to line failed: {}");

#[cfg(test)]
pub(crate) const ALL: &[TextKey] = &[
    STATUS_REPLACEMENT_EMPTY,
    STATUS_FIND_BUFFER_MISSING,
    STATUS_FIND_NO_MATCHES,
    STATUS_FIND_MATCH,
    STATUS_FIND_MATCH_WRAPPED,
    STATUS_FIND_NO_QUERY,
    STATUS_REPLACE_NO_QUERY,
    STATUS_REPLACE_BUFFER_MISSING,
    STATUS_REPLACE_CANCELLED,
    STATUS_REPLACE_NO_MATCHES,
    STATUS_REPLACE_FAILED,
    STATUS_REPLACE_DONE,
    STATUS_REPLACE_CONFIRM,
    STATUS_REPLACE_APPLIED_NEXT,
    STATUS_REPLACE_APPLIED_NEXT_WRAPPED,
    STATUS_REPLACE_APPLIED_DONE,
    STATUS_REPLACE_APPLIED_DONE_WRAPPED,
    STATUS_REPLACE_ALL_NO_QUERY,
    STATUS_REPLACE_ALL_BUFFER_MISSING,
    STATUS_REPLACE_ALL_NO_MATCHES,
    STATUS_REPLACE_ALL_FAILED,
    STATUS_REPLACE_ALL_APPLIED,
    STATUS_REPLACE_ALL_APPLIED_REMAINING,
    STATUS_RESULTS_FOCUSED_BUFFER_MISSING,
    STATUS_RESULTS_NO_QUERY,
    STATUS_RESULTS_NO_MATCHES,
    STATUS_RESULTS_SOURCE_BUFFER_MISSING,
    STATUS_RESULTS_MATCHES,
    STATUS_RESULTS_MATCH_NUMBER_EXPECTED,
    STATUS_RESULTS_OUT_OF_RANGE,
    STATUS_RESULTS_SOURCE_WINDOW_MISSING,
    STATUS_RESULTS_SELECTED,
    STATUS_LIST_NO_ENTRIES,
    STATUS_LIST_SELECTED,
    STATUS_GO_TO_LINE_INVALID,
    STATUS_GO_TO_LINE_STARTS_AT_ONE,
    STATUS_GO_TO_LINE_BUFFER_MISSING,
    STATUS_GO_TO_LINE_PAST_END,
    STATUS_GO_TO_LINE_MOVED,
    STATUS_GO_TO_LINE_FAILED,
];
