use super::TextKey;

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

// Helper-window body text.
pub(crate) const WINDOW_CONFIG_DIAGNOSTICS_HEADING: TextKey = (
    "window.config-diagnostics.heading",
    "Dun Config Diagnostics",
);
pub(crate) const WINDOW_CONFIG_DIAGNOSTICS_SECTION_SUMMARY: TextKey =
    ("window.config-diagnostics.section.summary", "Summary");
pub(crate) const WINDOW_CONFIG_DIAGNOSTICS_SECTION_PATHS: TextKey =
    ("window.config-diagnostics.section.paths", "Paths");
pub(crate) const WINDOW_CONFIG_DIAGNOSTICS_SECTION_SOURCE: TextKey =
    ("window.config-diagnostics.section.source", "Source");
pub(crate) const WINDOW_CONFIG_DIAGNOSTICS_SECTION_TERMINAL: TextKey =
    ("window.config-diagnostics.section.terminal", "Terminal");
pub(crate) const WINDOW_CONFIG_DIAGNOSTICS_SECTION_INPUT: TextKey =
    ("window.config-diagnostics.section.input", "Input");
pub(crate) const WINDOW_CONFIG_DIAGNOSTICS_SECTION_CLIPBOARD: TextKey =
    ("window.config-diagnostics.section.clipboard", "Clipboard");
pub(crate) const WINDOW_CONFIG_DIAGNOSTICS_SECTION_LIMITS: TextKey =
    ("window.config-diagnostics.section.limits", "Limits");
pub(crate) const WINDOW_CONFIG_DIAGNOSTICS_SECTION_KEYMAP: TextKey =
    ("window.config-diagnostics.section.keymap", "Keymap");
pub(crate) const WINDOW_CONFIG_DIAGNOSTICS_SECTION_FILE_DIALOG_KEYMAP: TextKey = (
    "window.config-diagnostics.section.file-dialog-keymap",
    "File Dialog Keymap",
);
pub(crate) const WINDOW_STATUS_HISTORY_HEADING: TextKey =
    ("window.status-history.heading", "Dun Status History");
pub(crate) const WINDOW_STATUS_HISTORY_LEVEL_INFO: TextKey =
    ("window.status-history.level.info", "info");
pub(crate) const WINDOW_STATUS_HISTORY_LEVEL_ERROR: TextKey =
    ("window.status-history.level.error", "error");
pub(crate) const WINDOW_STATUS_HISTORY_EMPTY: TextKey =
    ("window.status-history.empty", "No status messages yet.");

#[cfg(test)]
pub(super) const ALL: &[TextKey] = &[
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
    WINDOW_CONFIG_DIAGNOSTICS_HEADING,
    WINDOW_CONFIG_DIAGNOSTICS_SECTION_SUMMARY,
    WINDOW_CONFIG_DIAGNOSTICS_SECTION_PATHS,
    WINDOW_CONFIG_DIAGNOSTICS_SECTION_SOURCE,
    WINDOW_CONFIG_DIAGNOSTICS_SECTION_TERMINAL,
    WINDOW_CONFIG_DIAGNOSTICS_SECTION_INPUT,
    WINDOW_CONFIG_DIAGNOSTICS_SECTION_CLIPBOARD,
    WINDOW_CONFIG_DIAGNOSTICS_SECTION_LIMITS,
    WINDOW_CONFIG_DIAGNOSTICS_SECTION_KEYMAP,
    WINDOW_CONFIG_DIAGNOSTICS_SECTION_FILE_DIALOG_KEYMAP,
    WINDOW_STATUS_HISTORY_HEADING,
    WINDOW_STATUS_HISTORY_LEVEL_INFO,
    WINDOW_STATUS_HISTORY_LEVEL_ERROR,
    WINDOW_STATUS_HISTORY_EMPTY,
];
