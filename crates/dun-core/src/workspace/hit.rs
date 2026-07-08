use super::model::SplitPathStep;
use super::split::split_rects;
use super::*;

impl Workspace {
    pub fn split_at(&self, area: Rect, x: u16, y: u16) -> Option<SplitDragHandle> {
        let mut path = Vec::new();
        split_at_node(&self.root, area, x, y, &mut path)
    }
}

fn split_at_node(
    node: &LayoutNode,
    area: Rect,
    x: u16,
    y: u16,
    path: &mut Vec<SplitPathStep>,
) -> Option<SplitDragHandle> {
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
    if first_rect.contains(x, y) {
        path.push(SplitPathStep::First);
        if let Some(handle) = split_at_node(first, first_rect, x, y, path) {
            return Some(handle);
        }
        path.pop();
    }
    if second_rect.contains(x, y) {
        path.push(SplitPathStep::Second);
        if let Some(handle) = split_at_node(second, second_rect, x, y, path) {
            return Some(handle);
        }
        path.pop();
    }
    if split_boundary_contains(*axis, first_rect, second_rect, x, y) {
        return Some(SplitDragHandle { path: path.clone() });
    }

    None
}

fn split_boundary_contains(axis: Axis, first: Rect, second: Rect, x: u16, y: u16) -> bool {
    match axis {
        Axis::Horizontal => {
            y >= first.y
                && y < first.bottom()
                && (x == first.right().saturating_sub(1) || x == second.x)
        }
        Axis::Vertical => {
            x >= first.x
                && x < first.right()
                && (y == first.bottom().saturating_sub(1) || y == second.y)
        }
    }
}
