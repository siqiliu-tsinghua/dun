use crate::buffer::{BufferId, BufferKind};

use super::model::{DEFAULT_SPLIT_RATIO, MAX_SPLIT_RATIO, MIN_SPLIT_RATIO};
use super::*;

impl Workspace {
    pub fn split_focused(&mut self, axis: Axis) -> Result<WindowId, WorkspaceError> {
        if !self.root.contains(self.focused) {
            return Err(WorkspaceError::FocusMissing);
        }

        let old = self.focused;
        let new = self.create_untitled_window();
        let replacement = LayoutNode::Split {
            axis,
            ratio: DEFAULT_SPLIT_RATIO,
            first: Box::new(LayoutNode::Leaf(old)),
            second: Box::new(LayoutNode::Leaf(new)),
        };

        if !self.root.replace_leaf(old, replacement) {
            return Err(WorkspaceError::FocusMissing);
        }

        self.focused = new;
        Ok(new)
    }

    pub fn close_focused(&mut self) -> Result<WindowId, WorkspaceError> {
        if self.windows.len() <= 1 {
            return Err(WorkspaceError::CannotCloseLastWindow);
        }

        let closing = self.focused;
        let Some((new_root, next_focus)) = remove_leaf(&self.root, closing) else {
            return Err(WorkspaceError::FocusMissing);
        };

        self.root = new_root.ok_or(WorkspaceError::CannotCloseLastWindow)?;
        self.windows.retain(|window| window.id != closing);
        self.focused = next_focus;
        Ok(next_focus)
    }

    pub fn resolved_layout(&self, area: Rect) -> Vec<WindowLayout> {
        let mut out = Vec::with_capacity(self.windows.len());
        self.root.resolved(area, &mut out);
        out
    }

    fn create_untitled_window(&mut self) -> WindowId {
        let window_id = WindowId(self.next_window_id);
        let buffer_id = BufferId(self.next_buffer_id);
        self.next_window_id += 1;
        self.next_buffer_id += 1;

        self.windows.push(WindowState {
            id: window_id,
            title: format!("Untitled-{}", window_id.0),
            kind: WindowKind::Edit,
            collapsed: false,
            buffer_id,
            buffer_kind: BufferKind::Untitled,
        });

        window_id
    }
}

impl LayoutNode {
    fn resolved(&self, area: Rect, out: &mut Vec<WindowLayout>) {
        match self {
            Self::Leaf(id) => out.push(WindowLayout {
                id: *id,
                rect: area,
            }),
            Self::Split {
                axis,
                ratio,
                first,
                second,
            } => {
                let (first_rect, second_rect) = split_rects(area, *axis, *ratio);
                first.resolved(first_rect, out);
                second.resolved(second_rect, out);
            }
        }
    }
}

fn remove_leaf(node: &LayoutNode, target: WindowId) -> Option<(Option<LayoutNode>, WindowId)> {
    match node {
        LayoutNode::Leaf(id) if *id == target => Some((None, *id)),
        LayoutNode::Leaf(_) => None,
        LayoutNode::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            if let Some((new_first, candidate)) = remove_leaf(first, target) {
                return Some(match new_first {
                    Some(first) => (
                        Some(LayoutNode::Split {
                            axis: *axis,
                            ratio: *ratio,
                            first: Box::new(first),
                            second: second.clone(),
                        }),
                        candidate,
                    ),
                    None => {
                        let sibling = (**second).clone();
                        let candidate = sibling.first_leaf();
                        (Some(sibling), candidate)
                    }
                });
            }

            if let Some((new_second, candidate)) = remove_leaf(second, target) {
                return Some(match new_second {
                    Some(second) => (
                        Some(LayoutNode::Split {
                            axis: *axis,
                            ratio: *ratio,
                            first: first.clone(),
                            second: Box::new(second),
                        }),
                        candidate,
                    ),
                    None => {
                        let sibling = (**first).clone();
                        let candidate = sibling.first_leaf();
                        (Some(sibling), candidate)
                    }
                });
            }

            None
        }
    }
}

fn split_dimension(total: u16, ratio: u16) -> u16 {
    if total < 2 {
        return total;
    }

    let raw = ((total as u32 * ratio.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO) as u32) / 1000)
        .min(total as u32) as u16;
    raw.clamp(1, total - 1)
}

pub(super) fn split_rects(area: Rect, axis: Axis, ratio: u16) -> (Rect, Rect) {
    let ratio = ratio.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO);
    match axis {
        Axis::Horizontal => {
            let first_width = split_dimension(area.width, ratio);
            let second_width = area.width.saturating_sub(first_width);
            (
                Rect::new(area.x, area.y, first_width, area.height),
                Rect::new(
                    area.x.saturating_add(first_width),
                    area.y,
                    second_width,
                    area.height,
                ),
            )
        }
        Axis::Vertical => {
            let first_height = split_dimension(area.height, ratio);
            let second_height = area.height.saturating_sub(first_height);
            (
                Rect::new(area.x, area.y, area.width, first_height),
                Rect::new(
                    area.x,
                    area.y.saturating_add(first_height),
                    area.width,
                    second_height,
                ),
            )
        }
    }
}
