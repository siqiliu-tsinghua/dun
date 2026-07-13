//! Fixed UI chrome text with catalog keys (docs/i18n.md, slice 3).
//!
//! Every translatable dialog/overlay/window-title string is declared here
//! once as a `(catalog key, English default)` pair, so call sites cannot
//! invent keys ad hoc and the translation-completeness test can enumerate
//! the full set (`ALL`).

use dun_config::TextCatalog;

pub(crate) type TextKey = (&'static str, &'static str);

// Prompt modal titles.
pub(crate) const PROMPT_COMMAND_TITLE: TextKey = ("prompt.command.title", "Command");
pub(crate) const PROMPT_FIND_TITLE: TextKey = ("prompt.find.title", "Find");
pub(crate) const PROMPT_REPLACE_TITLE: TextKey = ("prompt.replace.title", "Replace");
pub(crate) const PROMPT_GO_TO_LINE_TITLE: TextKey = ("prompt.go-to-line.title", "Go To Line");
pub(crate) const PROMPT_RUN_COMMAND_TITLE: TextKey = ("prompt.run-command.title", "Run Command");

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

/// Every key above, for the translation-completeness test.
#[cfg(test)]
pub(crate) const ALL: &[TextKey] = &[
    PROMPT_COMMAND_TITLE,
    PROMPT_FIND_TITLE,
    PROMPT_REPLACE_TITLE,
    PROMPT_GO_TO_LINE_TITLE,
    PROMPT_RUN_COMMAND_TITLE,
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
