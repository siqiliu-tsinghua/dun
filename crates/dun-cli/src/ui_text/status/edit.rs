use super::TextKey;

pub(crate) const STATUS_BUFFER_ERROR_INVALID_POSITION: TextKey =
    ("status.buffer-error.invalid-position", "invalid position");
pub(crate) const STATUS_BUFFER_ERROR_INVALID_RANGE: TextKey =
    ("status.buffer-error.invalid-range", "invalid range");
pub(crate) const STATUS_BUFFER_ERROR_READ_ONLY: TextKey =
    ("status.buffer-error.read-only", "buffer is read-only");

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
pub(crate) const STATUS_WHITESPACE_BUFFER_MISSING: TextKey = (
    "status.whitespace.buffer-missing",
    "Whitespace failed: focused buffer is missing",
);
pub(crate) const STATUS_WHITESPACE_ON: TextKey = ("status.whitespace.on", "Visible whitespace on");
pub(crate) const STATUS_WHITESPACE_OFF: TextKey =
    ("status.whitespace.off", "Visible whitespace off");
pub(crate) const STATUS_DETAIL_WHITESPACE: TextKey = ("status.detail.whitespace", "Whitespace");
pub(crate) const STATUS_BOOKMARK_BUFFER_MISSING: TextKey = (
    "status.bookmark.buffer-missing",
    "Bookmark failed: focused buffer is missing",
);
pub(crate) const STATUS_BOOKMARK_ADDED: TextKey = ("status.bookmark.added", "Bookmarked line {}");
pub(crate) const STATUS_BOOKMARK_REMOVED: TextKey =
    ("status.bookmark.removed", "Removed bookmark at line {}");
pub(crate) const STATUS_BOOKMARK_NONE: TextKey = ("status.bookmark.none", "Bookmark: none set");
pub(crate) const STATUS_BOOKMARK_LINE: TextKey = ("status.bookmark.line", "Bookmark: line {}");
pub(crate) const STATUS_DETAIL_BOOKMARK: TextKey = ("status.detail.bookmark", "Mark");
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

#[cfg(test)]
pub(crate) const ALL: &[TextKey] = &[
    STATUS_BUFFER_ERROR_INVALID_POSITION,
    STATUS_BUFFER_ERROR_INVALID_RANGE,
    STATUS_BUFFER_ERROR_READ_ONLY,
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
    STATUS_WHITESPACE_BUFFER_MISSING,
    STATUS_WHITESPACE_ON,
    STATUS_WHITESPACE_OFF,
    STATUS_DETAIL_WHITESPACE,
    STATUS_BOOKMARK_BUFFER_MISSING,
    STATUS_BOOKMARK_ADDED,
    STATUS_BOOKMARK_REMOVED,
    STATUS_BOOKMARK_NONE,
    STATUS_BOOKMARK_LINE,
    STATUS_DETAIL_BOOKMARK,
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
];
