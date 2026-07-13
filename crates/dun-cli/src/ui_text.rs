//! Fixed UI chrome and status text with catalog keys (docs/i18n.md, slices 3–4).
//!
//! Every translatable dialog/overlay/window-title/status string is declared
//! here once as a `(catalog key, English default)` pair, so call sites cannot
//! invent keys ad hoc and the translation-completeness test can enumerate the
//! full set (`ALL`).

use dun_config::TextCatalog;

pub(crate) type TextKey = (&'static str, &'static str);

// Prompt modal titles.
pub(crate) const PROMPT_COMMAND_TITLE: TextKey = ("prompt.command.title", "Command");
pub(crate) const PROMPT_FIND_TITLE: TextKey = ("prompt.find.title", "Find");
pub(crate) const PROMPT_REPLACE_TITLE: TextKey = ("prompt.replace.title", "Replace");
pub(crate) const PROMPT_GO_TO_LINE_TITLE: TextKey = ("prompt.go-to-line.title", "Go To Line");
pub(crate) const PROMPT_RUN_COMMAND_TITLE: TextKey = ("prompt.run-command.title", "Run Command");

// Prompt-kind sentence openers and names used by status messages.
pub(crate) const PROMPT_COMMAND_LABEL: TextKey = ("prompt.command.label", "Command: ");
pub(crate) const PROMPT_COMMAND_NAME: TextKey = ("prompt.command.name", "Command");
pub(crate) const PROMPT_FIND_LABEL: TextKey = ("prompt.find.label", "Find: ");
pub(crate) const PROMPT_FIND_NAME: TextKey = ("prompt.find.name", "Find");
pub(crate) const PROMPT_REPLACE_FIND_LABEL: TextKey =
    ("prompt.replace-find.label", "Find to replace: ");
pub(crate) const PROMPT_REPLACE_WITH_LABEL: TextKey =
    ("prompt.replace-with.label", "Replace with: ");
pub(crate) const PROMPT_GO_TO_LINE_LABEL: TextKey = ("prompt.go-to-line.label", "Go To Line: ");
pub(crate) const PROMPT_GO_TO_LINE_NAME: TextKey = ("prompt.go-to-line.name", "Go To Line");
pub(crate) const PROMPT_RUN_COMMAND_LABEL: TextKey = ("prompt.run-command.label", "Run Command: ");
pub(crate) const PROMPT_RUN_COMMAND_NAME: TextKey = ("prompt.run-command.name", "Run Command");

// Unsaved-changes confirmation. The button letters (s)/(d)/(c) are the
// actual keys the dialog answers to and are appended by the code — a
// translation supplies only the word.
pub(crate) const CONFIRM_UNSAVED_TITLE: TextKey = ("confirm.unsaved.title", "Unsaved Changes");
pub(crate) const CONFIRM_UNSAVED_BODY: TextKey = ("confirm.unsaved.body", "Unsaved changes in {}");
pub(crate) const CONFIRM_SAVE: TextKey = ("confirm.button.save", "Save");
pub(crate) const CONFIRM_DISCARD: TextKey = ("confirm.button.discard", "Discard");
pub(crate) const CONFIRM_CANCEL: TextKey = ("confirm.button.cancel", "Cancel");

// Replace confirmation.
pub(crate) const CONFIRM_REPLACE_TITLE: TextKey = ("confirm.replace.title", "Confirm Replace");
pub(crate) const CONFIRM_REPLACE_FIND: TextKey = ("confirm.replace.find", "Find: {}");
pub(crate) const CONFIRM_REPLACE_WITH: TextKey = ("confirm.replace.with", "Replace with: {}");
pub(crate) const CONFIRM_REPLACE: TextKey = ("confirm.button.replace", "Replace");
pub(crate) const CONFIRM_SKIP: TextKey = ("confirm.button.skip", "Skip");
pub(crate) const CONFIRM_ALL: TextKey = ("confirm.button.all", "All");
pub(crate) const CONFIRM_MATCH_OF: TextKey = ("confirm.replace.match-of", "Match {}/{}");
pub(crate) const CONFIRM_MATCH_TOTAL: TextKey = ("confirm.replace.match-total", "Match {}");
pub(crate) const CONFIRM_MATCH_NONE: TextKey = ("confirm.replace.match-none", "Match -");
pub(crate) const CONFIRM_PROGRESS: TextKey =
    ("confirm.replace.progress", "; replaced {}, skipped {}");

// Buffer switcher.
pub(crate) const SWITCHER_TITLE: TextKey = ("switcher.title", "Switch Buffer");
pub(crate) const SWITCHER_OPEN_BUFFERS: TextKey = ("switcher.open-buffers", "Open buffers: {}");
pub(crate) const SWITCHER_SHOWING: TextKey = ("switcher.showing", "Showing {}-{} of {} buffers");
pub(crate) const SWITCHER_HINT_MOVE: TextKey = ("switcher.hint.move", "[Up/Down PgUp/PgDn] Move");
pub(crate) const SWITCHER_HINT_ACTIONS: TextKey = (
    "switcher.hint.actions",
    "[Home/End] First/Last  [Enter] Switch  [Esc] Cancel",
);

// File dialogs. The help lines keep English singular/plural branching in
// code; a translation supplies one template for all counts.
pub(crate) const DIALOG_OPEN_TITLE: TextKey = ("dialog.open.title", "Open");
pub(crate) const DIALOG_SAVE_AS_TITLE: TextKey = ("dialog.save-as.title", "Save As");
pub(crate) const DIALOG_OPEN_INPUT_LABEL: TextKey = ("dialog.open.input-label", "File name");
pub(crate) const DIALOG_SAVE_AS_INPUT_LABEL: TextKey = ("dialog.save-as.input-label", "Save as");
pub(crate) const DIALOG_LOOK_IN: TextKey = ("dialog.look-in", "Look in: {}");
pub(crate) const DIALOG_OPEN_HELP: TextKey = (
    "dialog.open.help",
    "Select a file or type a path. {} entries.",
);
pub(crate) const DIALOG_SAVE_AS_HELP: TextKey = (
    "dialog.save-as.help",
    "Type the destination path. {} entries.",
);
pub(crate) const DIALOG_HIDDEN: TextKey = ("dialog.hidden", "Hidden: {} ({})");
pub(crate) const DIALOG_HIDDEN_SHOWN: TextKey = ("dialog.hidden.shown", "shown");
pub(crate) const DIALOG_HIDDEN_HIDDEN: TextKey = ("dialog.hidden.hidden", "hidden");
pub(crate) const DIALOG_HIDDEN_BY_PREFIX: TextKey =
    ("dialog.hidden.shown-by-prefix", "shown by prefix");
pub(crate) const DIALOG_SHOWING_MATCHES: TextKey =
    ("dialog.showing-matches", "Showing {}-{} of {} matches");
pub(crate) const DIALOG_PARENT_DIR: TextKey = ("dialog.parent-directory", "[..] Parent directory");
pub(crate) const DIALOG_NO_MATCHES: TextKey = ("dialog.no-matches-row", "(no matches)");
pub(crate) const DIALOG_SHORTCUT_OK: TextKey = ("dialog.shortcut.ok", "OK");
pub(crate) const DIALOG_SHORTCUT_COMPLETE: TextKey = ("dialog.shortcut.complete", "Complete");
pub(crate) const DIALOG_SHORTCUT_HIDDEN: TextKey = ("dialog.shortcut.hidden", "Hidden");
pub(crate) const DIALOG_SHORTCUT_CANCEL: TextKey = ("dialog.shortcut.cancel", "Cancel");

// Helper-window titles (translated when the window opens).
pub(crate) const WINDOW_HELP_TITLE: TextKey = ("window.help.title", "Help");
pub(crate) const WINDOW_CONFIG_DIAGNOSTICS_TITLE: TextKey =
    ("window.config-diagnostics.title", "Config Diagnostics");
pub(crate) const WINDOW_STATUS_HISTORY_TITLE: TextKey =
    ("window.status-history.title", "Status History");
pub(crate) const WINDOW_SEARCH_RESULTS_TITLE: TextKey =
    ("window.search-results.title", "Search Results");
pub(crate) const WINDOW_COMMAND_OUTPUT_TITLE: TextKey =
    ("window.command-output.title", "Command Output");

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

// Command-line, configuration, plugin, and command-run status messages.
pub(crate) const STATUS_COMMAND_CANCELLED: TextKey =
    ("status.command.cancelled", "Command cancelled");
pub(crate) const STATUS_COMMAND_UNKNOWN: TextKey =
    ("status.command.unknown", "Unknown command: {}");
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
pub(crate) const STATUS_SHELL_ESCAPE: TextKey = ("status.shell.escape", "Shell escape");
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
pub(crate) const STATUS_MOVE_LINE_BUFFER_MISSING: TextKey = (
    "status.move-line.buffer-missing",
    "Move line failed: focused buffer is missing",
);
pub(crate) const STATUS_INDENT_BUFFER_MISSING: TextKey = (
    "status.indent.buffer-missing",
    "Indent failed: focused buffer is missing",
);
pub(crate) const STATUS_OUTDENT_BUFFER_MISSING: TextKey = (
    "status.outdent.buffer-missing",
    "Outdent failed: focused buffer is missing",
);
pub(crate) const STATUS_TRIM_BUFFER_MISSING: TextKey = (
    "status.trim.buffer-missing",
    "Trim failed: focused buffer is missing",
);
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
pub(crate) const STATUS_REDO_BUFFER_MISSING: TextKey = (
    "status.redo.buffer-missing",
    "Redo failed: focused buffer is missing",
);
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
pub(crate) const STATUS_CUT_BUFFER_MISSING: TextKey = (
    "status.cut.buffer-missing",
    "Cut failed: focused buffer is missing",
);
pub(crate) const STATUS_CUT_READ_ONLY: TextKey =
    ("status.cut.read-only", "Cut failed: buffer is read-only");
pub(crate) const STATUS_CUT_NO_SELECTION: TextKey =
    ("status.cut.no-selection", "Cut: no selection");
pub(crate) const STATUS_CUT_SELECTION: TextKey = ("status.cut.selection", "Cut selection");
pub(crate) const STATUS_PASTE_EMPTY: TextKey = (
    "status.paste.empty",
    "Paste: internal clipboard empty; use terminal paste",
);
pub(crate) const STATUS_PASTE_BUFFER_MISSING: TextKey = (
    "status.paste.buffer-missing",
    "Paste failed: focused buffer is missing",
);
pub(crate) const STATUS_PASTE_SELECTION: TextKey = ("status.paste.selection", "Pasted selection");
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
pub(crate) const STATUS_REPLACE_DONE: TextKey = (
    "status.replace.done",
    "Replace done: {} replaced, {} skipped",
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

/// Every key above, for the translation-completeness test.
#[cfg(test)]
pub(crate) const ALL: &[TextKey] = &[
    PROMPT_COMMAND_TITLE,
    PROMPT_FIND_TITLE,
    PROMPT_REPLACE_TITLE,
    PROMPT_GO_TO_LINE_TITLE,
    PROMPT_RUN_COMMAND_TITLE,
    PROMPT_COMMAND_LABEL,
    PROMPT_COMMAND_NAME,
    PROMPT_FIND_LABEL,
    PROMPT_FIND_NAME,
    PROMPT_REPLACE_FIND_LABEL,
    PROMPT_REPLACE_WITH_LABEL,
    PROMPT_GO_TO_LINE_LABEL,
    PROMPT_GO_TO_LINE_NAME,
    PROMPT_RUN_COMMAND_LABEL,
    PROMPT_RUN_COMMAND_NAME,
    CONFIRM_UNSAVED_TITLE,
    CONFIRM_UNSAVED_BODY,
    CONFIRM_SAVE,
    CONFIRM_DISCARD,
    CONFIRM_CANCEL,
    CONFIRM_REPLACE_TITLE,
    CONFIRM_REPLACE_FIND,
    CONFIRM_REPLACE_WITH,
    CONFIRM_REPLACE,
    CONFIRM_SKIP,
    CONFIRM_ALL,
    CONFIRM_MATCH_OF,
    CONFIRM_MATCH_TOTAL,
    CONFIRM_MATCH_NONE,
    CONFIRM_PROGRESS,
    SWITCHER_TITLE,
    SWITCHER_OPEN_BUFFERS,
    SWITCHER_SHOWING,
    SWITCHER_HINT_MOVE,
    SWITCHER_HINT_ACTIONS,
    DIALOG_OPEN_TITLE,
    DIALOG_SAVE_AS_TITLE,
    DIALOG_OPEN_INPUT_LABEL,
    DIALOG_SAVE_AS_INPUT_LABEL,
    DIALOG_LOOK_IN,
    DIALOG_OPEN_HELP,
    DIALOG_SAVE_AS_HELP,
    DIALOG_HIDDEN,
    DIALOG_HIDDEN_SHOWN,
    DIALOG_HIDDEN_HIDDEN,
    DIALOG_HIDDEN_BY_PREFIX,
    DIALOG_SHOWING_MATCHES,
    DIALOG_PARENT_DIR,
    DIALOG_NO_MATCHES,
    DIALOG_SHORTCUT_OK,
    DIALOG_SHORTCUT_COMPLETE,
    DIALOG_SHORTCUT_HIDDEN,
    DIALOG_SHORTCUT_CANCEL,
    WINDOW_HELP_TITLE,
    WINDOW_CONFIG_DIAGNOSTICS_TITLE,
    WINDOW_STATUS_HISTORY_TITLE,
    WINDOW_SEARCH_RESULTS_TITLE,
    WINDOW_COMMAND_OUTPUT_TITLE,
    STATUS_SWITCHER_ONLY_ONE,
    STATUS_SWITCHER_OPENED,
    STATUS_SWITCHER_CANCELLED,
    STATUS_SWITCHER_NO_BUFFERS,
    STATUS_SWITCHER_BUFFER_MISSING,
    STATUS_SWITCHER_SWITCHED,
    STATUS_SWITCHER_NO_WINDOW,
    STATUS_COMMAND_CANCELLED,
    STATUS_COMMAND_UNKNOWN,
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
    STATUS_PLUGIN_NOT_CONFIGURED,
    STATUS_PLUGIN_IS_LOADED,
    STATUS_PLUGIN_IS_UNLOADED,
    STATUS_PLUGIN_UNLOADED,
    STATUS_PLUGIN_LOADED,
    STATUS_PLUGIN_USAGE,
    STATUS_PLUGIN_FAILED,
    STATUS_THEME_CHANGED,
    STATUS_OPEN_DIRTY,
    STATUS_OPEN_FAILED,
    STATUS_SAVE_AS_FAILED,
    STATUS_SAVE_FAILED,
    STATUS_RELOAD_FAILED,
    STATUS_CONFIG_RELOAD_FAILED,
    STATUS_SHELL_ESCAPE,
    STATUS_RUN_FOCUSED_WINDOW_MISSING,
    STATUS_RUN_OUTPUT_WINDOW_MISSING,
    STATUS_RUN_RUNNING,
    STATUS_RUN_FAILED,
    STATUS_COPY_LINE_BUFFER_MISSING,
    STATUS_COPY_LINE_COPIED,
    STATUS_DELETE_LINE_BUFFER_MISSING,
    STATUS_MOVE_LINE_BUFFER_MISSING,
    STATUS_INDENT_BUFFER_MISSING,
    STATUS_OUTDENT_BUFFER_MISSING,
    STATUS_TRIM_BUFFER_MISSING,
    STATUS_WRAP_BUFFER_MISSING,
    STATUS_WRAP_ON,
    STATUS_WRAP_OFF,
    STATUS_UNDO_BUFFER_MISSING,
    STATUS_REDO_BUFFER_MISSING,
    STATUS_SCROLL_LEFT,
    STATUS_SCROLL_RIGHT,
    STATUS_SCROLL_LEFT_EDGE,
    STATUS_SCROLL_RIGHT_EDGE,
    STATUS_COPY_COPIED,
    STATUS_COPY_BUFFER_MISSING,
    STATUS_COPY_NO_SELECTION,
    STATUS_EXTERNAL_COPY_BUFFER_MISSING,
    STATUS_EXTERNAL_COPY_NO_SELECTION,
    STATUS_EXTERNAL_COPY_DISABLED,
    STATUS_EXTERNAL_COPY_TOO_LARGE,
    STATUS_EXTERNAL_COPY_COPIED,
    STATUS_CUT_BUFFER_MISSING,
    STATUS_CUT_READ_ONLY,
    STATUS_CUT_NO_SELECTION,
    STATUS_CUT_SELECTION,
    STATUS_PASTE_EMPTY,
    STATUS_PASTE_BUFFER_MISSING,
    STATUS_PASTE_SELECTION,
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
    STATUS_AUX_FOCUSED_WINDOW_MISSING,
    STATUS_AUX_WINDOW_MISSING,
    STATUS_HELP_OPENED,
    STATUS_HELP_FOCUSED_WINDOW_MISSING,
    STATUS_HELP_WINDOW_MISSING,
    STATUS_CONFIG_DIAGNOSTICS_OPENED,
    STATUS_CONFIG_DIAGNOSTICS_FOCUSED_WINDOW_MISSING,
    STATUS_CONFIG_DIAGNOSTICS_WINDOW_MISSING,
    STATUS_CONFIG_DIAGNOSTICS_BUFFER_MISSING,
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
    STATUS_FIND_BUFFER_MISSING,
    STATUS_FIND_NO_MATCHES,
    STATUS_FIND_MATCH,
    STATUS_FIND_MATCH_WRAPPED,
    STATUS_FIND_NO_QUERY,
    STATUS_REPLACE_NO_QUERY,
    STATUS_REPLACE_BUFFER_MISSING,
    STATUS_REPLACE_CANCELLED,
    STATUS_REPLACE_NO_MATCHES,
    STATUS_REPLACE_DONE,
    STATUS_REPLACE_ALL_NO_QUERY,
    STATUS_REPLACE_ALL_BUFFER_MISSING,
    STATUS_REPLACE_ALL_NO_MATCHES,
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
];

/// Translate a fixed string.
pub(crate) fn tr(catalog: &TextCatalog, key: TextKey) -> &str {
    catalog.get(key.0).unwrap_or(key.1)
}

/// Translate a `{}`-template and substitute arguments left to right. A
/// translated template whose placeholder count does not match the argument
/// count is ignored in favor of the English template: a translation mistake
/// must never drop or duplicate runtime values.
pub(crate) fn tr_fmt(catalog: &TextCatalog, key: TextKey, args: &[&str]) -> String {
    let template = catalog
        .get(key.0)
        .filter(|translated| placeholder_count(translated) == args.len())
        .unwrap_or(key.1);
    substitute(template, args)
}

/// The validated translated template for a key, when the caller needs its
/// own English fallback (e.g. English singular/plural branching).
pub(crate) fn tr_template(catalog: &TextCatalog, key: TextKey, arg_count: usize) -> Option<&str> {
    catalog
        .get(key.0)
        .filter(|translated| placeholder_count(translated) == arg_count)
}

pub(crate) fn placeholder_count(template: &str) -> usize {
    template.matches("{}").count()
}

pub(crate) fn substitute(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    for arg in args {
        match rest.split_once("{}") {
            Some((head, tail)) => {
                out.push_str(head);
                out.push_str(arg);
                rest = tail;
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}
