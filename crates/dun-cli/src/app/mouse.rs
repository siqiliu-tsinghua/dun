use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MouseDragState {
    Selection {
        buffer_id: BufferId,
        anchor: Position,
    },
    Split {
        handle: SplitDragHandle,
    },
    Scrollbar {
        buffer_id: BufferId,
    },
}
