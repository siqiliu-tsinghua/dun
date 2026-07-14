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

pub(crate) const STATUS_OPEN_DIRTY: TextKey = (
    "status.open.dirty",
    "Open failed: focused buffer has unsaved changes",
);
pub(crate) const STATUS_OPEN_FAILED: TextKey = ("status.open.failed", "Open failed: {}");
pub(crate) const STATUS_SAVE_AS_FAILED: TextKey = ("status.save-as.failed", "Save As failed: {}");
pub(crate) const STATUS_SAVE_FAILED: TextKey = ("status.save.failed", "Save failed: {}");
pub(crate) const STATUS_RELOAD_FAILED: TextKey = ("status.reload.failed", "Reload failed: {}");

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
pub(crate) const STATUS_PATH_ERROR_FRAME: TextKey = ("status.path-error.frame", "{}: {}");
pub(crate) const STATUS_PATH_ERROR_NOT_FOUND: TextKey =
    ("status.path-error.not-found", "not found");
pub(crate) const STATUS_PATH_ERROR_PERMISSION_DENIED: TextKey =
    ("status.path-error.permission-denied", "permission denied");
pub(crate) const STATUS_PATH_ERROR_PARENT_MISSING: TextKey = (
    "status.path-error.parent-missing",
    "parent directory does not exist",
);
pub(crate) const STATUS_PATH_ERROR_DESTINATION_READ_ONLY: TextKey = (
    "status.path-error.destination-read-only",
    "destination is read-only",
);
pub(crate) const STATUS_FILE_DIALOG_NO_MATCHES: TextKey =
    ("status.file-dialog.no-matches", "No matches");
pub(crate) const STATUS_FILE_DIALOG_NO_VISIBLE_MATCHES: TextKey = (
    "status.file-dialog.no-visible-matches",
    "No visible matches; type . or toggle hidden files",
);
pub(crate) const STATUS_FILE_DIALOG_NO_MATCHES_FOR_PREFIX: TextKey = (
    "status.file-dialog.no-matches-for-prefix",
    "No matches for `{}`; `..` goes up",
);
pub(crate) const STATUS_FILE_DIALOG_ONLY_HIDDEN_FILTERED: TextKey = (
    "status.file-dialog.only-hidden-filtered",
    "Only hidden entries are filtered; type . or toggle hidden files",
);
pub(crate) const STATUS_FILE_DIALOG_DIRECTORY_EMPTY: TextKey = (
    "status.file-dialog.directory-empty",
    "Directory is empty; `..` goes up",
);
pub(crate) const STATUS_FILE_DIALOG_CANNOT_LIST: TextKey =
    ("status.file-dialog.cannot-list", "Cannot list {}: {}");
pub(crate) const STATUS_FILE_DIALOG_HIDDEN_SHOWN: TextKey =
    ("status.file-dialog.hidden-shown", "Hidden files shown");
pub(crate) const STATUS_FILE_DIALOG_HIDDEN_HIDDEN: TextKey =
    ("status.file-dialog.hidden-hidden", "Hidden files hidden");
pub(crate) const STATUS_FILE_DIALOG_CONFIRM_OVERWRITE: TextKey = (
    "status.file-dialog.confirm-overwrite",
    "Replace existing file {}? Press Enter again.",
);

#[cfg(test)]
pub(crate) const ALL: &[TextKey] = &[
    STATUS_SWITCHER_ONLY_ONE,
    STATUS_SWITCHER_OPENED,
    STATUS_SWITCHER_CANCELLED,
    STATUS_SWITCHER_NO_BUFFERS,
    STATUS_SWITCHER_BUFFER_MISSING,
    STATUS_SWITCHER_SWITCHED,
    STATUS_SWITCHER_NO_WINDOW,
    STATUS_OPEN_DIRTY,
    STATUS_OPEN_FAILED,
    STATUS_SAVE_AS_FAILED,
    STATUS_SAVE_FAILED,
    STATUS_RELOAD_FAILED,
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
    STATUS_PATH_ERROR_FRAME,
    STATUS_PATH_ERROR_NOT_FOUND,
    STATUS_PATH_ERROR_PERMISSION_DENIED,
    STATUS_PATH_ERROR_PARENT_MISSING,
    STATUS_PATH_ERROR_DESTINATION_READ_ONLY,
    STATUS_FILE_DIALOG_NO_MATCHES,
    STATUS_FILE_DIALOG_NO_VISIBLE_MATCHES,
    STATUS_FILE_DIALOG_NO_MATCHES_FOR_PREFIX,
    STATUS_FILE_DIALOG_ONLY_HIDDEN_FILTERED,
    STATUS_FILE_DIALOG_DIRECTORY_EMPTY,
    STATUS_FILE_DIALOG_CANNOT_LIST,
    STATUS_FILE_DIALOG_HIDDEN_SHOWN,
    STATUS_FILE_DIALOG_HIDDEN_HIDDEN,
    STATUS_FILE_DIALOG_CONFIRM_OVERWRITE,
];
