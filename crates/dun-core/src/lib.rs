#![forbid(unsafe_code)]

pub mod buffer;
pub mod command;
pub mod display;
pub mod workspace;

pub use buffer::{
    BufferError, BufferId, BufferKind, Cursor, EditTransaction, LineEnding, Position, SearchMatch,
    Selection, TextBuffer, TextEdit, TextRange,
};
pub use command::{AppCommand, EditCommand, EditorCommand, FileCommand, WindowCommand};
pub use display::{DisplayClass, DisplaySanitizer, DisplaySegment, SanitizedLine};
pub use workspace::{
    Axis, Direction, LayoutNode, Rect, WindowId, WindowKind, WindowLayout, WindowState, Workspace,
    WorkspaceError,
};
