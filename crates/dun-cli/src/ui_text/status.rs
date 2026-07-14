use super::TextKey;

// Buffer-switcher status messages.
pub(crate) const STATUS_SWITCHER_ONLY_ONE: TextKey = (
    "status.buffer-switcher.only-one",
    "Buffer switcher: only one buffer",
);
pub(crate) const STATUS_SWITCHER_OPENED: TextKey =
    ("status.buffer-switcher.opened", "Switch buffer");
pub(crate) const STATUS_SWITCHER_CANCELLED: TextKey = (
    "status.buffer-switcher.cancelled",
    "Switch buffer cancelled",
);
pub(crate) const STATUS_SWITCHER_NO_BUFFERS: TextKey = (
    "status.buffer-switcher.no-buffers",
    "Switch buffer failed: no buffers",
);
pub(crate) const STATUS_SWITCHER_BUFFER_MISSING: TextKey = (
    "status.buffer-switcher.buffer-missing",
    "Switch buffer failed: buffer is missing",
);
pub(crate) const STATUS_SWITCHER_SWITCHED: TextKey =
    ("status.buffer-switcher.switched", "Switched to {}");
pub(crate) const STATUS_SWITCHER_NO_WINDOW: TextKey = (
    "status.buffer-switcher.no-window",
    "Switch buffer failed: {} has no window",
);

// Shared helper fragments used inside complete status templates.
pub(crate) const STATUS_REPLACEMENT_EMPTY: TextKey = ("status.replacement.empty", "<empty>");
pub(crate) const STATUS_BUFFER_ERROR_INVALID_POSITION: TextKey =
    ("status.buffer-error.invalid-position", "invalid position");
pub(crate) const STATUS_BUFFER_ERROR_INVALID_RANGE: TextKey =
    ("status.buffer-error.invalid-range", "invalid range");
pub(crate) const STATUS_BUFFER_ERROR_READ_ONLY: TextKey =
    ("status.buffer-error.read-only", "buffer is read-only");
pub(crate) const STATUS_WORKSPACE_ERROR_CANNOT_CLOSE_LAST: TextKey = (
    "status.workspace-error.cannot-close-last-window",
    "cannot close the last window",
);
pub(crate) const STATUS_WORKSPACE_ERROR_CANNOT_COLLAPSE_LAST: TextKey = (
    "status.workspace-error.cannot-collapse-last-window",
    "cannot collapse the only window",
);
pub(crate) const STATUS_WORKSPACE_ERROR_FOCUS_MISSING: TextKey = (
    "status.workspace-error.focus-missing",
    "focused window is missing",
);
pub(crate) const STATUS_WORKSPACE_ERROR_NO_NEIGHBOR: TextKey =
    ("status.workspace-error.no-neighbor", "no neighboring pane");
pub(crate) const STATUS_WORKSPACE_ERROR_NO_RESIZABLE_SPLIT: TextKey = (
    "status.workspace-error.no-resizable-split",
    "no matching split",
);
pub(crate) const STATUS_WORKSPACE_ERROR_WINDOW_MISSING: TextKey =
    ("status.workspace-error.window-missing", "window is missing");

// Command-line, configuration, plugin, and command-run status messages.
pub(crate) const STATUS_COMMAND_CANCELLED: TextKey =
    ("status.command.cancelled", "Command cancelled");
pub(crate) const STATUS_COMMAND_UNKNOWN: TextKey =
    ("status.command.unknown", "Unknown command: {}");
pub(crate) const STATUS_COMMAND_PARSE_FAILED: TextKey =
    ("status.command.parse-failed", "Command failed: {}");
pub(crate) const STATUS_COMMAND_PARSE_TRAILING_ESCAPE: TextKey =
    ("status.command.parse.trailing-escape", "trailing escape");
pub(crate) const STATUS_COMMAND_PARSE_UNCLOSED_QUOTE: TextKey =
    ("status.command.parse.unclosed-quote", "unclosed quote");
pub(crate) const STATUS_COMMAND_LINE_HELP: TextKey = (
    "status.command-line.help",
    "Commands: {}, or any command id such as {}",
);
pub(crate) const STATUS_COMMAND_THEME_ARITY: TextKey = (
    "status.command.theme-arity",
    "Command failed: theme expects zero or one theme name",
);
pub(crate) const STATUS_COMMAND_RUN_ARITY: TextKey = (
    "status.command.run-arity",
    "Command failed: run expects zero args or one quoted command",
);
pub(crate) const STATUS_COMMAND_RESULTS_ARITY: TextKey = (
    "status.command.results-arity",
    "Command failed: results expects zero args or one match number",
);
pub(crate) const STATUS_COMMAND_OPEN_ARITY: TextKey = (
    "status.command.open-arity",
    "Command failed: open expects zero or one path",
);
pub(crate) const STATUS_COMMAND_SAVE_ARITY: TextKey = (
    "status.command.save-arity",
    "Command failed: save expects zero or one path",
);
pub(crate) const STATUS_COMMAND_SAVE_AS_ARITY: TextKey = (
    "status.command.save-as-arity",
    "Command failed: save-as expects zero or one path",
);
pub(crate) const STATUS_COMMAND_NO_ARGUMENTS: TextKey = (
    "status.command.no-arguments",
    "Command failed: {} expects no arguments",
);
pub(crate) const STATUS_COMMAND_FIND_ARITY: TextKey = (
    "status.command.find-arity",
    "Command failed: find expects zero or one query",
);
pub(crate) const STATUS_COMMAND_REPLACE_ARITY: TextKey = (
    "status.command.replace-arity",
    "Command failed: replace expects query and replacement, or all query replacement",
);
pub(crate) const STATUS_COMMAND_GO_TO_LINE_ARITY: TextKey = (
    "status.command.go-to-line-arity",
    "Command failed: go-to-line expects one line number",
);
pub(crate) const STATUS_COMMAND_CONFIG_SECTION: TextKey = (
    "status.command.config-section",
    "Command failed: config expects one of {}",
);
pub(crate) const STATUS_COMMAND_CONFIG_SECTION_ARITY: TextKey = (
    "status.command.config-section-arity",
    "Command failed: config expects zero args or one of {}",
);
pub(crate) const STATUS_PLUGIN_NOT_CONFIGURED: TextKey = (
    "status.plugin.not-configured",
    "No syntax-highlight plugin configured",
);
pub(crate) const STATUS_PLUGIN_IS_LOADED: TextKey =
    ("status.plugin.is-loaded", "Plugin {} is loaded");
pub(crate) const STATUS_PLUGIN_IS_UNLOADED: TextKey =
    ("status.plugin.is-unloaded", "Plugin {} is unloaded");
pub(crate) const STATUS_PLUGIN_UNLOADED: TextKey = ("status.plugin.unloaded", "Plugin {} unloaded");
pub(crate) const STATUS_PLUGIN_LOADED: TextKey = (
    "status.plugin.loaded",
    "Plugin {} loaded (starts on the next edit)",
);
pub(crate) const STATUS_PLUGIN_USAGE: TextKey =
    ("status.plugin.usage", "Usage: plugin [load|unload]");
pub(crate) const STATUS_PLUGIN_FAILED: TextKey = ("status.plugin.failed", "Plugin {} failed: {}");
pub(crate) const STATUS_THEME_CHANGED: TextKey = ("status.theme.changed", "Theme: {}");
pub(crate) const STATUS_THEME_CURRENT: TextKey = ("status.theme.current", "Theme: {} ({})");
pub(crate) const STATUS_THEME_UNKNOWN: TextKey = (
    "status.theme.unknown",
    "Theme failed: unknown theme {}; expected {}",
);
pub(crate) const STATUS_OPEN_DIRTY: TextKey = (
    "status.open.dirty",
    "Open failed: focused buffer has unsaved changes",
);
pub(crate) const STATUS_OPEN_FAILED: TextKey = ("status.open.failed", "Open failed: {}");
pub(crate) const STATUS_SAVE_AS_FAILED: TextKey = ("status.save-as.failed", "Save As failed: {}");
pub(crate) const STATUS_SAVE_FAILED: TextKey = ("status.save.failed", "Save failed: {}");
pub(crate) const STATUS_RELOAD_FAILED: TextKey = ("status.reload.failed", "Reload failed: {}");
pub(crate) const STATUS_CONFIG_RELOAD_FAILED: TextKey =
    ("status.config.reload-failed", "Config reload failed: {}");
pub(crate) const STATUS_CONFIG_RELOADED_DISABLED: TextKey = (
    "status.config.reloaded-disabled",
    "Config reloaded from built-in defaults (--no-config)",
);
pub(crate) const STATUS_CONFIG_RELOADED_PATH: TextKey =
    ("status.config.reloaded-path", "Config reloaded from {}");
pub(crate) const STATUS_CONFIG_RELOADED_ENVIRONMENT: TextKey = (
    "status.config.reloaded-environment",
    "Config reloaded from {}={}",
);
pub(crate) const STATUS_CONFIG_RELOADED_DEFAULTS: TextKey = (
    "status.config.reloaded-defaults",
    "Config reloaded from built-in defaults",
);
pub(crate) const STATUS_SHELL_ESCAPE: TextKey = ("status.shell.escape", "Shell escape");
pub(crate) const STATUS_SHELL_RETURNED: TextKey = ("status.shell.returned", "Shell returned {}");
pub(crate) const STATUS_RUN_FOCUSED_WINDOW_MISSING: TextKey = (
    "status.run.focused-window-missing",
    "Run command failed: focused window is missing",
);
pub(crate) const STATUS_RUN_OUTPUT_WINDOW_MISSING: TextKey = (
    "status.run.output-window-missing",
    "Run command failed: output window is missing",
);
pub(crate) const STATUS_RUN_RUNNING: TextKey = ("status.run.running", "Running command: {}");
pub(crate) const STATUS_RUN_FAILED: TextKey = ("status.run.failed", "Run command failed: {}");
pub(crate) const STATUS_RUN_RETURNED: TextKey =
    ("status.run.returned", "Command returned {} in {}");
pub(crate) const STATUS_RUN_RETURNED_TRUNCATED: TextKey = (
    "status.run.returned-truncated",
    "Command returned {} in {}; output truncated",
);
pub(crate) const STATUS_RUN_TIMED_OUT: TextKey = (
    "status.run.timed-out",
    "Command timed out after {} and was killed",
);
pub(crate) const STATUS_RUN_TIMED_OUT_TRUNCATED: TextKey = (
    "status.run.timed-out-truncated",
    "Command timed out after {} and was killed; output truncated",
);
pub(crate) const STATUS_RUN_EXIT: TextKey = ("status.run.exit", "exit {}");
pub(crate) const STATUS_RUN_TERMINATED: TextKey = ("status.run.terminated", "terminated");

// Editing and clipboard status messages.
pub(crate) const STATUS_COPY_LINE_BUFFER_MISSING: TextKey = (
    "status.copy-line.buffer-missing",
    "Copy line failed: focused buffer is missing",
);
pub(crate) const STATUS_COPY_LINE_COPIED: TextKey = ("status.copy-line.copied", "Copied line");
pub(crate) const STATUS_DELETE_LINE_BUFFER_MISSING: TextKey = (
    "status.delete-line.buffer-missing",
    "Delete line failed: focused buffer is missing",
);
pub(crate) const STATUS_DELETE_LINE_FAILED: TextKey =
    ("status.delete-line.failed", "Delete line failed: {}");
pub(crate) const STATUS_MOVE_LINE_BUFFER_MISSING: TextKey = (
    "status.move-line.buffer-missing",
    "Move line failed: focused buffer is missing",
);
pub(crate) const STATUS_MOVE_LINE_FAILED: TextKey =
    ("status.move-line.failed", "Move line failed: {}");
pub(crate) const STATUS_INDENT_BUFFER_MISSING: TextKey = (
    "status.indent.buffer-missing",
    "Indent failed: focused buffer is missing",
);
pub(crate) const STATUS_INDENT_FAILED: TextKey = ("status.indent.failed", "Indent failed: {}");
pub(crate) const STATUS_OUTDENT_BUFFER_MISSING: TextKey = (
    "status.outdent.buffer-missing",
    "Outdent failed: focused buffer is missing",
);
pub(crate) const STATUS_OUTDENT_FAILED: TextKey = ("status.outdent.failed", "Outdent failed: {}");
pub(crate) const STATUS_TRIM_BUFFER_MISSING: TextKey = (
    "status.trim.buffer-missing",
    "Trim failed: focused buffer is missing",
);
pub(crate) const STATUS_TRIM_FAILED: TextKey = ("status.trim.failed", "Trim failed: {}");
pub(crate) const STATUS_WRAP_BUFFER_MISSING: TextKey = (
    "status.wrap.buffer-missing",
    "Wrap failed: focused buffer is missing",
);
pub(crate) const STATUS_WRAP_ON: TextKey = ("status.wrap.on", "Word wrap on");
pub(crate) const STATUS_WRAP_OFF: TextKey = ("status.wrap.off", "Word wrap off");
pub(crate) const STATUS_UNDO_BUFFER_MISSING: TextKey = (
    "status.undo.buffer-missing",
    "Undo failed: focused buffer is missing",
);
pub(crate) const STATUS_UNDO_FAILED: TextKey = ("status.undo.failed", "Undo failed: {}");
pub(crate) const STATUS_REDO_BUFFER_MISSING: TextKey = (
    "status.redo.buffer-missing",
    "Redo failed: focused buffer is missing",
);
pub(crate) const STATUS_REDO_FAILED: TextKey = ("status.redo.failed", "Redo failed: {}");
pub(crate) const STATUS_SCROLL_LEFT: TextKey = ("status.scroll.left", "Scrolled left to column {}");
pub(crate) const STATUS_SCROLL_RIGHT: TextKey =
    ("status.scroll.right", "Scrolled right to column {}");
pub(crate) const STATUS_SCROLL_LEFT_EDGE: TextKey =
    ("status.scroll.left-edge", "Already at left edge");
pub(crate) const STATUS_SCROLL_RIGHT_EDGE: TextKey =
    ("status.scroll.right-edge", "Already at right edge");
pub(crate) const STATUS_COPY_COPIED: TextKey = ("status.copy.copied", "Copied selection");
pub(crate) const STATUS_COPY_BUFFER_MISSING: TextKey = (
    "status.copy.buffer-missing",
    "Copy failed: focused buffer is missing",
);
pub(crate) const STATUS_COPY_NO_SELECTION: TextKey =
    ("status.copy.no-selection", "Copy: no selection");
pub(crate) const STATUS_COPY_FAILED: TextKey = ("status.copy.failed", "Copy failed: {}");
pub(crate) const STATUS_EXTERNAL_COPY_BUFFER_MISSING: TextKey = (
    "status.external-copy.buffer-missing",
    "External copy failed: focused buffer is missing",
);
pub(crate) const STATUS_EXTERNAL_COPY_NO_SELECTION: TextKey = (
    "status.external-copy.no-selection",
    "External copy: no selection",
);
pub(crate) const STATUS_EXTERNAL_COPY_DISABLED: TextKey = (
    "status.external-copy.disabled",
    "External copy disabled: copied selection internally",
);
pub(crate) const STATUS_EXTERNAL_COPY_TOO_LARGE: TextKey = (
    "status.external-copy.too-large",
    "External copy failed: selection is {} bytes; limit is {}",
);
pub(crate) const STATUS_EXTERNAL_COPY_COPIED: TextKey = (
    "status.external-copy.copied",
    "Copied selection to external clipboard",
);
pub(crate) const STATUS_EXTERNAL_COPY_FAILED: TextKey =
    ("status.external-copy.failed", "External copy failed: {}");
pub(crate) const STATUS_CUT_BUFFER_MISSING: TextKey = (
    "status.cut.buffer-missing",
    "Cut failed: focused buffer is missing",
);
pub(crate) const STATUS_CUT_READ_ONLY: TextKey =
    ("status.cut.read-only", "Cut failed: buffer is read-only");
pub(crate) const STATUS_CUT_NO_SELECTION: TextKey =
    ("status.cut.no-selection", "Cut: no selection");
pub(crate) const STATUS_CUT_SELECTION: TextKey = ("status.cut.selection", "Cut selection");
pub(crate) const STATUS_CUT_FAILED: TextKey = ("status.cut.failed", "Cut failed: {}");
pub(crate) const STATUS_PASTE_EMPTY: TextKey = (
    "status.paste.empty",
    "Paste: internal clipboard empty; use terminal paste",
);
pub(crate) const STATUS_PASTE_BUFFER_MISSING: TextKey = (
    "status.paste.buffer-missing",
    "Paste failed: focused buffer is missing",
);
pub(crate) const STATUS_PASTE_SELECTION: TextKey = ("status.paste.selection", "Pasted selection");
pub(crate) const STATUS_PASTE_FAILED: TextKey = ("status.paste.failed", "Paste failed: {}");
pub(crate) const STATUS_PASTE_IGNORED_CONFIRMATION: TextKey = (
    "status.paste.ignored-confirmation",
    "Paste ignored during confirmation",
);
pub(crate) const STATUS_PASTE_IGNORED_REPLACE: TextKey = (
    "status.paste.ignored-replace-confirmation",
    "Paste ignored during replace confirmation",
);
pub(crate) const STATUS_PASTE_IGNORED_SWITCHER: TextKey = (
    "status.paste.ignored-buffer-switcher",
    "Paste ignored during buffer switcher",
);
pub(crate) const STATUS_PASTE_WAITING: TextKey = (
    "status.paste.waiting",
    "Paste: waiting for terminal bracketed paste data",
);
pub(crate) const STATUS_PANE_COLLAPSED: TextKey = (
    "status.pane.collapsed",
    "Pane is collapsed; expand it to edit",
);
pub(crate) const STATUS_PANE_COLLAPSED_WITH_KEY: TextKey = (
    "status.pane.collapsed-with-key",
    "Pane is collapsed; expand it to edit ({})",
);

// File-dialog and file-operation status messages.
pub(crate) const STATUS_UNSAVED_CANCELLED: TextKey =
    ("status.unsaved.cancelled", "Unsaved changes cancelled");
pub(crate) const STATUS_DIALOG_CANCELLED: TextKey = ("status.dialog.cancelled", "{} cancelled");
pub(crate) const STATUS_FILE_NEW_UNTITLED: TextKey =
    ("status.file.new-untitled", "New untitled buffer");
pub(crate) const STATUS_SAVE_NO_CHANGES: TextKey =
    ("status.save.no-changes", "No changes to save in {}");
pub(crate) const STATUS_OPEN_OPENED: TextKey = ("status.open.opened", "Opened {}");
pub(crate) const STATUS_OPEN_OPENED_ESCAPED: TextKey = (
    "status.open.opened-escaped-bytes",
    "Opened {} read-only: non-UTF-8 bytes shown as escapes",
);
pub(crate) const STATUS_RELOAD_RELOADED: TextKey = ("status.reload.reloaded", "Reloaded {}");
pub(crate) const STATUS_RELOAD_RELOADED_ESCAPED: TextKey = (
    "status.reload.reloaded-escaped-bytes",
    "Reloaded {} read-only: non-UTF-8 bytes shown as escapes",
);
pub(crate) const STATUS_SAVE_SAVED: TextKey = ("status.save.saved", "Saved {}");
pub(crate) const STATUS_ATOMIC_CLEANED: TextKey = (
    "status.atomic-temp.cleaned",
    "cleaned {} stale save temp file(s)",
);
pub(crate) const STATUS_ATOMIC_CLEAN_FAILED: TextKey = (
    "status.atomic-temp.clean-failed",
    "failed to clean {} save temp file(s)",
);
pub(crate) const STATUS_ATOMIC_RECOVERY_FOUND: TextKey = (
    "status.atomic-temp.recovery-found",
    "recovery temp file found: {}",
);
pub(crate) const STATUS_ATOMIC_RECOVERY_FOUND_MANY: TextKey = (
    "status.atomic-temp.recovery-found-many",
    "{} recovery temp file(s) found; first: {}",
);

// Helper-window status messages.
pub(crate) const STATUS_AUX_FOCUSED_WINDOW_MISSING: TextKey = (
    "status.aux-window.focused-window-missing",
    "{} failed: focused window is missing",
);
pub(crate) const STATUS_AUX_WINDOW_MISSING: TextKey = (
    "status.aux-window.window-missing",
    "{} failed: window is missing",
);
pub(crate) const STATUS_HELP_OPENED: TextKey = ("status.help.opened", "Help");
pub(crate) const STATUS_HELP_FOCUSED_WINDOW_MISSING: TextKey = (
    "status.help.focused-window-missing",
    "Help failed: focused window is missing",
);
pub(crate) const STATUS_HELP_WINDOW_MISSING: TextKey = (
    "status.help.window-missing",
    "Help failed: help window is missing",
);
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_OPENED: TextKey =
    ("status.config-diagnostics.opened", "Config diagnostics");
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_FOCUSED_WINDOW_MISSING: TextKey = (
    "status.config-diagnostics.focused-window-missing",
    "Config diagnostics failed: focused window is missing",
);
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_WINDOW_MISSING: TextKey = (
    "status.config-diagnostics.window-missing",
    "Config diagnostics failed: diagnostics window is missing",
);
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_BUFFER_MISSING: TextKey = (
    "status.config-diagnostics.buffer-missing",
    "Config diagnostics failed: diagnostics buffer is missing",
);
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_SECTION_NOT_FOUND: TextKey = (
    "status.config-diagnostics.section-not-found",
    "Config diagnostics: {} section not found",
);
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_SECTION: TextKey = (
    "status.config-diagnostics.section",
    "Config diagnostics: {}",
);
pub(crate) const STATUS_HISTORY_OPENED: TextKey = ("status.history.opened", "Status history");
pub(crate) const STATUS_HISTORY_FOCUSED_WINDOW_MISSING: TextKey = (
    "status.history.focused-window-missing",
    "Status history failed: focused window is missing",
);
pub(crate) const STATUS_HISTORY_WINDOW_MISSING: TextKey = (
    "status.history.window-missing",
    "Status history failed: status window is missing",
);

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

// Window-layout status messages.
pub(crate) const STATUS_WINDOW_SPLIT_HORIZONTAL: TextKey =
    ("status.window.split-horizontal", "Split horizontally");
pub(crate) const STATUS_WINDOW_SPLIT_VERTICAL: TextKey =
    ("status.window.split-vertical", "Split vertically");
pub(crate) const STATUS_WINDOW_SPLITS_EVEN: TextKey =
    ("status.window.splits-even", "Splits are already even");
pub(crate) const STATUS_WINDOW_EQUALIZED: TextKey =
    ("status.window.equalized", "Equalized {} split(s)");
pub(crate) const STATUS_WINDOW_COLLAPSED: TextKey = ("status.window.collapsed", "Collapsed pane");
pub(crate) const STATUS_WINDOW_EXPANDED: TextKey = ("status.window.expanded", "Expanded pane");
pub(crate) const STATUS_WINDOW_ALREADY_EXPANDED: TextKey =
    ("status.window.already-expanded", "Pane is already expanded");
pub(crate) const STATUS_WINDOW_FOCUSED_LEFT: TextKey =
    ("status.window.focused-left", "Focused left");
pub(crate) const STATUS_WINDOW_FOCUSED_RIGHT: TextKey =
    ("status.window.focused-right", "Focused right");
pub(crate) const STATUS_WINDOW_FOCUSED_UP: TextKey = ("status.window.focused-up", "Focused up");
pub(crate) const STATUS_WINDOW_FOCUSED_DOWN: TextKey =
    ("status.window.focused-down", "Focused down");
pub(crate) const STATUS_WINDOW_RESIZED_LEFT: TextKey =
    ("status.window.resized-left", "Resized left");
pub(crate) const STATUS_WINDOW_RESIZED_RIGHT: TextKey =
    ("status.window.resized-right", "Resized right");
pub(crate) const STATUS_WINDOW_RESIZED_UP: TextKey = ("status.window.resized-up", "Resized up");
pub(crate) const STATUS_WINDOW_RESIZED_DOWN: TextKey =
    ("status.window.resized-down", "Resized down");
pub(crate) const STATUS_WINDOW_CLOSED_ITEM: TextKey = ("status.window.closed-item", "Closed {}");
pub(crate) const STATUS_WINDOW_CLOSED: TextKey = ("status.window.closed", "Closed window");
pub(crate) const STATUS_WINDOW_ONLY_ONE: TextKey =
    ("status.window.only-one", "Already the only window");
pub(crate) const STATUS_WINDOW_CLOSED_OTHERS: TextKey =
    ("status.window.closed-others", "Closed {} other window(s)");
pub(crate) const STATUS_WINDOW_AXIS_HORIZONTAL: TextKey =
    ("status.window.axis.horizontal", "horizontal");
pub(crate) const STATUS_WINDOW_AXIS_VERTICAL: TextKey = ("status.window.axis.vertical", "vertical");
pub(crate) const STATUS_WINDOW_ROTATED: TextKey =
    ("status.window.rotated", "Rotated focused split to {}");
pub(crate) const STATUS_WINDOW_ROTATE_FAILED: TextKey =
    ("status.window.rotate-failed", "Rotate split failed: {}");
pub(crate) const STATUS_WINDOW_COLLAPSE_FAILED: TextKey =
    ("status.window.collapse-failed", "Collapse failed: {}");
pub(crate) const STATUS_WINDOW_EXPAND_FAILED: TextKey =
    ("status.window.expand-failed", "Expand failed: {}");
pub(crate) const STATUS_WINDOW_TOGGLE_COLLAPSE_FAILED: TextKey = (
    "status.window.toggle-collapse-failed",
    "Toggle collapse failed: {}",
);
pub(crate) const STATUS_WINDOW_FOCUS_LEFT_FAILED: TextKey =
    ("status.window.focus-left-failed", "Focus left failed: {}");
pub(crate) const STATUS_WINDOW_FOCUS_RIGHT_FAILED: TextKey =
    ("status.window.focus-right-failed", "Focus right failed: {}");
pub(crate) const STATUS_WINDOW_FOCUS_UP_FAILED: TextKey =
    ("status.window.focus-up-failed", "Focus up failed: {}");
pub(crate) const STATUS_WINDOW_FOCUS_DOWN_FAILED: TextKey =
    ("status.window.focus-down-failed", "Focus down failed: {}");
pub(crate) const STATUS_WINDOW_RESIZE_LEFT_FAILED: TextKey =
    ("status.window.resize-left-failed", "Resize left failed: {}");
pub(crate) const STATUS_WINDOW_RESIZE_RIGHT_FAILED: TextKey = (
    "status.window.resize-right-failed",
    "Resize right failed: {}",
);
pub(crate) const STATUS_WINDOW_RESIZE_UP_FAILED: TextKey =
    ("status.window.resize-up-failed", "Resize up failed: {}");
pub(crate) const STATUS_WINDOW_RESIZE_DOWN_FAILED: TextKey =
    ("status.window.resize-down-failed", "Resize down failed: {}");
pub(crate) const STATUS_WINDOW_SPLIT_FAILED: TextKey =
    ("status.window.split-failed", "Split failed: {}");
pub(crate) const STATUS_WINDOW_CLOSE_FAILED: TextKey =
    ("status.window.close-failed", "Close failed: {}");
pub(crate) const STATUS_WINDOW_ONLY_FAILED: TextKey =
    ("status.window.only-failed", "Only window failed: {}");

/// Every status key above, for the translation-completeness test.
#[cfg(test)]
pub(super) const ALL: &[TextKey] = &[
    STATUS_SWITCHER_ONLY_ONE,
    STATUS_SWITCHER_OPENED,
    STATUS_SWITCHER_CANCELLED,
    STATUS_SWITCHER_NO_BUFFERS,
    STATUS_SWITCHER_BUFFER_MISSING,
    STATUS_SWITCHER_SWITCHED,
    STATUS_SWITCHER_NO_WINDOW,
    STATUS_REPLACEMENT_EMPTY,
    STATUS_BUFFER_ERROR_INVALID_POSITION,
    STATUS_BUFFER_ERROR_INVALID_RANGE,
    STATUS_BUFFER_ERROR_READ_ONLY,
    STATUS_WORKSPACE_ERROR_CANNOT_CLOSE_LAST,
    STATUS_WORKSPACE_ERROR_CANNOT_COLLAPSE_LAST,
    STATUS_WORKSPACE_ERROR_FOCUS_MISSING,
    STATUS_WORKSPACE_ERROR_NO_NEIGHBOR,
    STATUS_WORKSPACE_ERROR_NO_RESIZABLE_SPLIT,
    STATUS_WORKSPACE_ERROR_WINDOW_MISSING,
    STATUS_COMMAND_CANCELLED,
    STATUS_COMMAND_UNKNOWN,
    STATUS_COMMAND_PARSE_FAILED,
    STATUS_COMMAND_PARSE_TRAILING_ESCAPE,
    STATUS_COMMAND_PARSE_UNCLOSED_QUOTE,
    STATUS_COMMAND_LINE_HELP,
    STATUS_COMMAND_THEME_ARITY,
    STATUS_COMMAND_RUN_ARITY,
    STATUS_COMMAND_RESULTS_ARITY,
    STATUS_COMMAND_OPEN_ARITY,
    STATUS_COMMAND_SAVE_ARITY,
    STATUS_COMMAND_SAVE_AS_ARITY,
    STATUS_COMMAND_NO_ARGUMENTS,
    STATUS_COMMAND_FIND_ARITY,
    STATUS_COMMAND_REPLACE_ARITY,
    STATUS_COMMAND_GO_TO_LINE_ARITY,
    STATUS_COMMAND_CONFIG_SECTION,
    STATUS_COMMAND_CONFIG_SECTION_ARITY,
    STATUS_PLUGIN_NOT_CONFIGURED,
    STATUS_PLUGIN_IS_LOADED,
    STATUS_PLUGIN_IS_UNLOADED,
    STATUS_PLUGIN_UNLOADED,
    STATUS_PLUGIN_LOADED,
    STATUS_PLUGIN_USAGE,
    STATUS_PLUGIN_FAILED,
    STATUS_THEME_CHANGED,
    STATUS_THEME_CURRENT,
    STATUS_THEME_UNKNOWN,
    STATUS_OPEN_DIRTY,
    STATUS_OPEN_FAILED,
    STATUS_SAVE_AS_FAILED,
    STATUS_SAVE_FAILED,
    STATUS_RELOAD_FAILED,
    STATUS_CONFIG_RELOAD_FAILED,
    STATUS_CONFIG_RELOADED_DISABLED,
    STATUS_CONFIG_RELOADED_PATH,
    STATUS_CONFIG_RELOADED_ENVIRONMENT,
    STATUS_CONFIG_RELOADED_DEFAULTS,
    STATUS_SHELL_ESCAPE,
    STATUS_SHELL_RETURNED,
    STATUS_RUN_FOCUSED_WINDOW_MISSING,
    STATUS_RUN_OUTPUT_WINDOW_MISSING,
    STATUS_RUN_RUNNING,
    STATUS_RUN_FAILED,
    STATUS_RUN_RETURNED,
    STATUS_RUN_RETURNED_TRUNCATED,
    STATUS_RUN_TIMED_OUT,
    STATUS_RUN_TIMED_OUT_TRUNCATED,
    STATUS_RUN_EXIT,
    STATUS_RUN_TERMINATED,
    STATUS_COPY_LINE_BUFFER_MISSING,
    STATUS_COPY_LINE_COPIED,
    STATUS_DELETE_LINE_BUFFER_MISSING,
    STATUS_DELETE_LINE_FAILED,
    STATUS_MOVE_LINE_BUFFER_MISSING,
    STATUS_MOVE_LINE_FAILED,
    STATUS_INDENT_BUFFER_MISSING,
    STATUS_INDENT_FAILED,
    STATUS_OUTDENT_BUFFER_MISSING,
    STATUS_OUTDENT_FAILED,
    STATUS_TRIM_BUFFER_MISSING,
    STATUS_TRIM_FAILED,
    STATUS_WRAP_BUFFER_MISSING,
    STATUS_WRAP_ON,
    STATUS_WRAP_OFF,
    STATUS_UNDO_BUFFER_MISSING,
    STATUS_UNDO_FAILED,
    STATUS_REDO_BUFFER_MISSING,
    STATUS_REDO_FAILED,
    STATUS_SCROLL_LEFT,
    STATUS_SCROLL_RIGHT,
    STATUS_SCROLL_LEFT_EDGE,
    STATUS_SCROLL_RIGHT_EDGE,
    STATUS_COPY_COPIED,
    STATUS_COPY_BUFFER_MISSING,
    STATUS_COPY_NO_SELECTION,
    STATUS_COPY_FAILED,
    STATUS_EXTERNAL_COPY_BUFFER_MISSING,
    STATUS_EXTERNAL_COPY_NO_SELECTION,
    STATUS_EXTERNAL_COPY_DISABLED,
    STATUS_EXTERNAL_COPY_TOO_LARGE,
    STATUS_EXTERNAL_COPY_COPIED,
    STATUS_EXTERNAL_COPY_FAILED,
    STATUS_CUT_BUFFER_MISSING,
    STATUS_CUT_READ_ONLY,
    STATUS_CUT_NO_SELECTION,
    STATUS_CUT_SELECTION,
    STATUS_CUT_FAILED,
    STATUS_PASTE_EMPTY,
    STATUS_PASTE_BUFFER_MISSING,
    STATUS_PASTE_SELECTION,
    STATUS_PASTE_FAILED,
    STATUS_PASTE_IGNORED_CONFIRMATION,
    STATUS_PASTE_IGNORED_REPLACE,
    STATUS_PASTE_IGNORED_SWITCHER,
    STATUS_PASTE_WAITING,
    STATUS_PANE_COLLAPSED,
    STATUS_PANE_COLLAPSED_WITH_KEY,
    STATUS_UNSAVED_CANCELLED,
    STATUS_DIALOG_CANCELLED,
    STATUS_FILE_NEW_UNTITLED,
    STATUS_SAVE_NO_CHANGES,
    STATUS_OPEN_OPENED,
    STATUS_OPEN_OPENED_ESCAPED,
    STATUS_RELOAD_RELOADED,
    STATUS_RELOAD_RELOADED_ESCAPED,
    STATUS_SAVE_SAVED,
    STATUS_ATOMIC_CLEANED,
    STATUS_ATOMIC_CLEAN_FAILED,
    STATUS_ATOMIC_RECOVERY_FOUND,
    STATUS_ATOMIC_RECOVERY_FOUND_MANY,
    STATUS_AUX_FOCUSED_WINDOW_MISSING,
    STATUS_AUX_WINDOW_MISSING,
    STATUS_HELP_OPENED,
    STATUS_HELP_FOCUSED_WINDOW_MISSING,
    STATUS_HELP_WINDOW_MISSING,
    STATUS_CONFIG_DIAGNOSTICS_OPENED,
    STATUS_CONFIG_DIAGNOSTICS_FOCUSED_WINDOW_MISSING,
    STATUS_CONFIG_DIAGNOSTICS_WINDOW_MISSING,
    STATUS_CONFIG_DIAGNOSTICS_BUFFER_MISSING,
    STATUS_CONFIG_DIAGNOSTICS_SECTION_NOT_FOUND,
    STATUS_CONFIG_DIAGNOSTICS_SECTION,
    STATUS_HISTORY_OPENED,
    STATUS_HISTORY_FOCUSED_WINDOW_MISSING,
    STATUS_HISTORY_WINDOW_MISSING,
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
    STATUS_WINDOW_SPLIT_HORIZONTAL,
    STATUS_WINDOW_SPLIT_VERTICAL,
    STATUS_WINDOW_SPLITS_EVEN,
    STATUS_WINDOW_EQUALIZED,
    STATUS_WINDOW_COLLAPSED,
    STATUS_WINDOW_EXPANDED,
    STATUS_WINDOW_ALREADY_EXPANDED,
    STATUS_WINDOW_FOCUSED_LEFT,
    STATUS_WINDOW_FOCUSED_RIGHT,
    STATUS_WINDOW_FOCUSED_UP,
    STATUS_WINDOW_FOCUSED_DOWN,
    STATUS_WINDOW_RESIZED_LEFT,
    STATUS_WINDOW_RESIZED_RIGHT,
    STATUS_WINDOW_RESIZED_UP,
    STATUS_WINDOW_RESIZED_DOWN,
    STATUS_WINDOW_CLOSED_ITEM,
    STATUS_WINDOW_CLOSED,
    STATUS_WINDOW_ONLY_ONE,
    STATUS_WINDOW_CLOSED_OTHERS,
    STATUS_WINDOW_AXIS_HORIZONTAL,
    STATUS_WINDOW_AXIS_VERTICAL,
    STATUS_WINDOW_ROTATED,
    STATUS_WINDOW_ROTATE_FAILED,
    STATUS_WINDOW_COLLAPSE_FAILED,
    STATUS_WINDOW_EXPAND_FAILED,
    STATUS_WINDOW_TOGGLE_COLLAPSE_FAILED,
    STATUS_WINDOW_FOCUS_LEFT_FAILED,
    STATUS_WINDOW_FOCUS_RIGHT_FAILED,
    STATUS_WINDOW_FOCUS_UP_FAILED,
    STATUS_WINDOW_FOCUS_DOWN_FAILED,
    STATUS_WINDOW_RESIZE_LEFT_FAILED,
    STATUS_WINDOW_RESIZE_RIGHT_FAILED,
    STATUS_WINDOW_RESIZE_UP_FAILED,
    STATUS_WINDOW_RESIZE_DOWN_FAILED,
    STATUS_WINDOW_SPLIT_FAILED,
    STATUS_WINDOW_CLOSE_FAILED,
    STATUS_WINDOW_ONLY_FAILED,
];
