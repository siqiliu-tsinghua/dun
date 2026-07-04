use crate::buffer::{BufferId, BufferKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowKind {
    Edit,
    ReadOnly,
    Help,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub root: LayoutNode,
    pub focused: WindowId,
    pub windows: Vec<WindowState>,
}

impl Workspace {
    pub fn new_untitled() -> Self {
        let window_id = WindowId(1);
        let buffer_id = BufferId(1);

        Self {
            root: LayoutNode::Leaf(window_id),
            focused: window_id,
            windows: vec![WindowState {
                id: window_id,
                title: "Untitled".to_string(),
                kind: WindowKind::Edit,
                collapsed: false,
                buffer_id,
                buffer_kind: BufferKind::Untitled,
            }],
        }
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }
}
