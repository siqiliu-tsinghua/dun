use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConfirmState {
    pub(crate) action: PendingAction,
    pub(crate) buffer_id: BufferId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReplaceConfirmState {
    pub(crate) buffer_id: BufferId,
    pub(crate) spec: SearchSpec,
    pub(crate) replacement: String,
    pub(crate) replaced: usize,
    pub(crate) skipped: usize,
    pub(crate) skipped_in_cycle: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PendingAction {
    Quit,
    New,
    OpenPrompt,
    ReloadBuffer,
    /// `file.close` — close the focused *file*. Never refuses: the last window
    /// falls back to an empty untitled buffer.
    CloseFile,
    /// `window.close` — close the focused *pane*. Refuses on the last window.
    CloseWindow,
    /// `window.only` — close every other window, keeping this one. Carries the
    /// target because the confirm dialog refocuses the dirty buffer's window.
    OnlyWindow(WindowId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CopyTextError {
    MissingBuffer,
    NoSelection,
    Buffer(BufferError),
}
