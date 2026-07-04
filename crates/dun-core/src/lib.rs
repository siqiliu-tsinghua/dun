#![forbid(unsafe_code)]

pub mod buffer;
pub mod command;
pub mod workspace;

pub use buffer::{BufferId, BufferKind};
pub use command::{AppCommand, EditCommand, EditorCommand, FileCommand, WindowCommand};
pub use workspace::{
    Axis, Direction, LayoutNode, Rect, WindowId, WindowKind, WindowLayout, WindowState, Workspace,
    WorkspaceError,
};
