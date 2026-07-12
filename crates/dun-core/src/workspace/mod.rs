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

    /// Collapse the focused pane to its title bar.
    ///
    /// Refuses on the only window. Collapsing exists to give room to the other
    /// panes; with none there is nothing to give it to, and all it achieves is
    /// to make the editor body vanish.
    ///
    /// A collapsed pane shows no body, so nothing may be *edited* through it —
    /// see `AppState::focused_pane_is_collapsed`. The pane stays focusable so
    /// that `expand` and `toggle_collapse` have something to act on.
    pub fn collapse_focused(&mut self) -> Result<(), WorkspaceError> {
        if self.windows.len() <= 1 {
            return Err(WorkspaceError::CannotCollapseLastWindow);
        }

        self.window_mut(self.focused)?.collapsed = true;
        Ok(())
    }

    /// Expand the focused pane. Reports whether it was actually collapsed, so
    /// the caller does not announce work it did not do -- the pane that was
    /// already open has nothing to expand.
    pub fn expand_focused(&mut self) -> Result<bool, WorkspaceError> {
        let window = self.window_mut(self.focused)?;
        let was_collapsed = window.collapsed;
        window.collapsed = false;
        Ok(was_collapsed)
    }

    pub fn toggle_focused_collapse(&mut self) -> Result<bool, WorkspaceError> {
        if self.window(self.focused)?.collapsed {
            self.expand_focused()?;
            return Ok(false);
        }

        self.collapse_focused()?;
        Ok(true)
    }

    pub fn focused_is_collapsed(&self) -> bool {
        self.window(self.focused)
            .map(|window| window.collapsed)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests;
