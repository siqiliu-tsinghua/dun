#![forbid(unsafe_code)]

pub mod buffer;
pub mod command;
pub mod display;
pub mod file_text;
pub mod workspace;

pub use buffer::{
    BufferError, BufferId, BufferKind, Cursor, EditTransaction, LineEnding, Position, SearchMatch,
    SearchOptions, Selection, TextBuffer, TextEdit, TextRange,
};
pub use command::{AppCommand, EditCommand, EditorCommand, FileCommand, WindowCommand};
pub use display::{DisplayClass, DisplaySanitizer, DisplaySegment, SanitizedLine};
pub use file_text::{DecodedFileText, FileTextEncoding, decode_file_text};
pub use workspace::{
    Axis, Direction, LayoutNode, Rect, SplitDragHandle, WindowId, WindowKind, WindowLayout,
    WindowState, Workspace, WorkspaceError,
};
