use super::TextKey;

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

pub(crate) const STATUS_PANE_COLLAPSED: TextKey = (
    "status.pane.collapsed",
    "Pane is collapsed; expand it to edit",
);
pub(crate) const STATUS_PANE_COLLAPSED_WITH_KEY: TextKey = (
    "status.pane.collapsed-with-key",
    "Pane is collapsed; expand it to edit ({})",
);

// Window-layout status messages.
pub(crate) const WINDOW_UNTITLED: TextKey = ("window.untitled", "Untitled");
pub(crate) const WINDOW_UNTITLED_NUMBERED: TextKey = ("window.untitled-numbered", "Untitled-{}");
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

#[cfg(test)]
pub(crate) const ALL: &[TextKey] = &[
    STATUS_WORKSPACE_ERROR_CANNOT_CLOSE_LAST,
    STATUS_WORKSPACE_ERROR_CANNOT_COLLAPSE_LAST,
    STATUS_WORKSPACE_ERROR_FOCUS_MISSING,
    STATUS_WORKSPACE_ERROR_NO_NEIGHBOR,
    STATUS_WORKSPACE_ERROR_NO_RESIZABLE_SPLIT,
    STATUS_WORKSPACE_ERROR_WINDOW_MISSING,
    STATUS_PANE_COLLAPSED,
    STATUS_PANE_COLLAPSED_WITH_KEY,
    WINDOW_UNTITLED,
    WINDOW_UNTITLED_NUMBERED,
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
