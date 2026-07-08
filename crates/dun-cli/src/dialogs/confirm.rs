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
    CloseWindow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CopyTextError {
    MissingBuffer,
    NoSelection,
    Buffer(BufferError),
}
