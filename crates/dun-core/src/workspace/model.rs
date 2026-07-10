use crate::buffer::{BufferId, BufferKind};

pub(super) const DEFAULT_SPLIT_RATIO: u16 = 500;
pub(super) const MIN_SPLIT_RATIO: u16 = 100;
pub(super) const MAX_SPLIT_RATIO: u16 = 900;
pub(super) const DEFAULT_RESIZE_STEP: u16 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Split available width into left and right children.
    Horizontal,
    /// Split available height into top and bottom children.
    Vertical,
}

impl Axis {
    pub(super) fn toggled(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn right(self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub const fn bottom(self) -> u16 {
        self.y.saturating_add(self.height)
    }

    pub const fn contains(self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    pub(super) fn center_x(self) -> i32 {
        self.x as i32 * 2 + self.width as i32
    }

    pub(super) fn center_y(self) -> i32 {
        self.y as i32 * 2 + self.height as i32
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutNode {
    Leaf(WindowId),
    Split {
        axis: Axis,
        ratio: u16,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

impl LayoutNode {
    pub(super) fn contains(&self, target: WindowId) -> bool {
        match self {
            Self::Leaf(id) => *id == target,
            Self::Split { first, second, .. } => first.contains(target) || second.contains(target),
        }
    }

    pub(super) fn first_leaf(&self) -> WindowId {
        match self {
            Self::Leaf(id) => *id,
            Self::Split { first, .. } => first.first_leaf(),
        }
    }

    pub(super) fn replace_leaf(&mut self, target: WindowId, replacement: LayoutNode) -> bool {
        match self {
            Self::Leaf(id) if *id == target => {
                *self = replacement;
                true
            }
            Self::Leaf(_) => false,
            Self::Split { first, second, .. } => {
                first.replace_leaf(target, replacement.clone())
                    || second.replace_leaf(target, replacement)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowKind {
    Edit,
    ReadOnly,
    CommandOutput,
    ConfigDiagnostics,
    Help,
    Outline,
    SearchResults,
    StatusHistory,
    Prompt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowState {
    pub id: WindowId,
    pub title: String,
    pub kind: WindowKind,
    pub collapsed: bool,
    pub buffer_id: BufferId,
    pub buffer_kind: BufferKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowLayout {
    pub id: WindowId,
    pub rect: Rect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitDragHandle {
    pub(super) path: Vec<SplitPathStep>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SplitPathStep {
    First,
    Second,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceError {
    CannotCloseLastWindow,
    FocusMissing,
    NoNeighbor,
    NoResizableSplit,
    WindowMissing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub root: LayoutNode,
    pub focused: WindowId,
    pub windows: Vec<WindowState>,
    pub(super) next_window_id: u64,
    pub(super) next_buffer_id: u64,
}
