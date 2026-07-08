use crate::buffer::{BufferId, BufferKind};

mod focus;
mod hit;
mod model;
mod resize;
mod split;

pub use model::{
    Axis, Direction, LayoutNode, Rect, SplitDragHandle, WindowId, WindowKind, WindowLayout,
    WindowState, Workspace, WorkspaceError,
};

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
            next_window_id: 2,
            next_buffer_id: 2,
        }
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn focused_window(&self) -> Result<&WindowState, WorkspaceError> {
        self.window(self.focused)
    }

    pub fn window(&self, id: WindowId) -> Result<&WindowState, WorkspaceError> {
        self.windows
            .iter()
            .find(|window| window.id == id)
            .ok_or(WorkspaceError::WindowMissing)
    }

    pub fn window_mut(&mut self, id: WindowId) -> Result<&mut WindowState, WorkspaceError> {
        self.windows
            .iter_mut()
            .find(|window| window.id == id)
            .ok_or(WorkspaceError::WindowMissing)
    }

    pub fn collapse_focused(&mut self) -> Result<(), WorkspaceError> {
        self.window_mut(self.focused)?.collapsed = true;
        Ok(())
    }

    pub fn expand_focused(&mut self) -> Result<(), WorkspaceError> {
        self.window_mut(self.focused)?.collapsed = false;
        Ok(())
    }

    pub fn toggle_focused_collapse(&mut self) -> Result<bool, WorkspaceError> {
        let window = self.window_mut(self.focused)?;
        window.collapsed = !window.collapsed;
        Ok(window.collapsed)
    }
}

#[cfg(test)]
mod tests;
