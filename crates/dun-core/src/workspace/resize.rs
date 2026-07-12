use super::model::{
    DEFAULT_RESIZE_STEP, DEFAULT_SPLIT_RATIO, MAX_SPLIT_RATIO, MIN_SPLIT_RATIO, SplitPathStep,
};
use super::split::split_rects;
use super::*;

impl Workspace {
    pub fn resize_focused(&mut self, direction: Direction) -> Result<u16, WorkspaceError> {
        self.resize_focused_by(direction, DEFAULT_RESIZE_STEP)
    }

    pub fn resize_focused_by(
        &mut self,
        direction: Direction,
        step: u16,
    ) -> Result<u16, WorkspaceError> {
        match resize_node(&mut self.root, self.focused, direction, step) {
            SearchOutcome::Applied(ratio) => Ok(ratio),
            SearchOutcome::Contains => Err(WorkspaceError::NoResizableSplit),
            SearchOutcome::Miss => Err(WorkspaceError::FocusMissing),
        }
    }

    /// Reset every split to an even share. Returns how many splits were
    /// actually off-balance, so the caller does not announce work it did not
    /// do: a single window has no splits, and evenly-split panes are already
    /// equal.
    pub fn equalize(&mut self) -> usize {
        equalize_node(&mut self.root)
    }

    pub fn rotate_focused_split(&mut self) -> Result<Axis, WorkspaceError> {
        match rotate_node(&mut self.root, self.focused) {
            RotateOutcome::Rotated(axis) => Ok(axis),
            RotateOutcome::Contains => Err(WorkspaceError::NoResizableSplit),
            RotateOutcome::Miss => Err(WorkspaceError::FocusMissing),
        }
    }

    pub fn resize_split_to(
        &mut self,
        handle: &SplitDragHandle,
        area: Rect,
        x: u16,
        y: u16,
    ) -> Result<u16, WorkspaceError> {
        let Some((node, rect)) = split_node_mut_at_path(&mut self.root, &handle.path, area) else {
            return Err(WorkspaceError::NoResizableSplit);
        };
        let LayoutNode::Split { axis, ratio, .. } = node else {
            return Err(WorkspaceError::NoResizableSplit);
        };

        *ratio = ratio_for_split_position(*axis, rect, x, y);
        Ok(*ratio)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchOutcome {
    Applied(u16),
    Contains,
    Miss,
}

fn resize_node(
    node: &mut LayoutNode,
    target: WindowId,
    direction: Direction,
    step: u16,
) -> SearchOutcome {
    match node {
        LayoutNode::Leaf(id) if *id == target => SearchOutcome::Contains,
        LayoutNode::Leaf(_) => SearchOutcome::Miss,
        LayoutNode::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            match resize_node(first, target, direction, step) {
                SearchOutcome::Applied(ratio) => return SearchOutcome::Applied(ratio),
                SearchOutcome::Contains => {
                    return resize_at_split(*axis, ratio, true, direction, step);
                }
                SearchOutcome::Miss => {}
            }

            match resize_node(second, target, direction, step) {
                SearchOutcome::Applied(ratio) => SearchOutcome::Applied(ratio),
                SearchOutcome::Contains => resize_at_split(*axis, ratio, false, direction, step),
                SearchOutcome::Miss => SearchOutcome::Miss,
            }
        }
    }
}

fn resize_at_split(
    axis: Axis,
    ratio: &mut u16,
    in_first: bool,
    direction: Direction,
    step: u16,
) -> SearchOutcome {
    let delta = match (axis, in_first, direction) {
        (Axis::Horizontal, true, Direction::Right) | (Axis::Vertical, true, Direction::Down) => {
            step as i32
        }
        (Axis::Horizontal, false, Direction::Left) | (Axis::Vertical, false, Direction::Up) => {
            -(step as i32)
        }
        _ => return SearchOutcome::Contains,
    };

    let next = (*ratio as i32 + delta).clamp(MIN_SPLIT_RATIO as i32, MAX_SPLIT_RATIO as i32);
    *ratio = next as u16;
    SearchOutcome::Applied(*ratio)
}

/// Returns the number of splits that were not already even.
fn equalize_node(node: &mut LayoutNode) -> usize {
    match node {
        LayoutNode::Leaf(_) => 0,
        LayoutNode::Split {
            ratio,
            first,
            second,
            ..
        } => {
            let changed = usize::from(*ratio != DEFAULT_SPLIT_RATIO);
            *ratio = DEFAULT_SPLIT_RATIO;
            changed + equalize_node(first) + equalize_node(second)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RotateOutcome {
    Rotated(Axis),
    Contains,
    Miss,
}

fn rotate_node(node: &mut LayoutNode, target: WindowId) -> RotateOutcome {
    match node {
        LayoutNode::Leaf(id) if *id == target => RotateOutcome::Contains,
        LayoutNode::Leaf(_) => RotateOutcome::Miss,
        LayoutNode::Split {
            axis,
            first,
            second,
            ..
        } => {
            match rotate_node(first, target) {
                RotateOutcome::Rotated(axis) => return RotateOutcome::Rotated(axis),
                RotateOutcome::Contains => {
                    *axis = axis.toggled();
                    return RotateOutcome::Rotated(*axis);
                }
                RotateOutcome::Miss => {}
            }

            match rotate_node(second, target) {
                RotateOutcome::Rotated(axis) => RotateOutcome::Rotated(axis),
                RotateOutcome::Contains => {
                    *axis = axis.toggled();
                    RotateOutcome::Rotated(*axis)
                }
                RotateOutcome::Miss => RotateOutcome::Miss,
            }
        }
    }
}

fn split_node_mut_at_path<'a>(
    node: &'a mut LayoutNode,
    path: &[SplitPathStep],
    area: Rect,
) -> Option<(&'a mut LayoutNode, Rect)> {
    let Some((step, rest)) = path.split_first() else {
        return Some((node, area));
    };
    let LayoutNode::Split {
        axis,
        ratio,
        first,
        second,
    } = node
    else {
        return None;
    };
    let (first_rect, second_rect) = split_rects(area, *axis, *ratio);

    match step {
        SplitPathStep::First => split_node_mut_at_path(first, rest, first_rect),
        SplitPathStep::Second => split_node_mut_at_path(second, rest, second_rect),
    }
}

fn ratio_for_split_position(axis: Axis, area: Rect, x: u16, y: u16) -> u16 {
    let (total, offset) = match axis {
        Axis::Horizontal => (area.width, x.saturating_sub(area.x)),
        Axis::Vertical => (area.height, y.saturating_sub(area.y)),
    };
    if total < 2 {
        return DEFAULT_SPLIT_RATIO;
    }

    let offset = offset.clamp(1, total - 1);
    ((offset as u32 * 1000) / total as u32).clamp(MIN_SPLIT_RATIO as u32, MAX_SPLIT_RATIO as u32)
        as u16
}
