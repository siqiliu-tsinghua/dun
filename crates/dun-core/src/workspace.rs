use crate::buffer::{BufferId, BufferKind};

const DEFAULT_SPLIT_RATIO: u16 = 500;
const MIN_SPLIT_RATIO: u16 = 100;
const MAX_SPLIT_RATIO: u16 = 900;
const DEFAULT_RESIZE_STEP: u16 = 50;

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
    fn toggled(self) -> Self {
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

    fn center_x(self) -> i32 {
        self.x as i32 * 2 + self.width as i32
    }

    fn center_y(self) -> i32 {
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
    fn contains(&self, target: WindowId) -> bool {
        match self {
            Self::Leaf(id) => *id == target,
            Self::Split { first, second, .. } => first.contains(target) || second.contains(target),
        }
    }

    fn first_leaf(&self) -> WindowId {
        match self {
            Self::Leaf(id) => *id,
            Self::Split { first, .. } => first.first_leaf(),
        }
    }

    fn replace_leaf(&mut self, target: WindowId, replacement: LayoutNode) -> bool {
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
                let ratio = (*ratio).clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO);
                match axis {
                    Axis::Horizontal => {
                        let first_width = ((area.width as u32 * ratio as u32) / 1000) as u16;
                        let first_width = first_width.min(area.width);
                        let second_width = area.width.saturating_sub(first_width);

                        first.resolved(Rect::new(area.x, area.y, first_width, area.height), out);
                        second.resolved(
                            Rect::new(
                                area.x.saturating_add(first_width),
                                area.y,
                                second_width,
                                area.height,
                            ),
                            out,
                        );
                    }
                    Axis::Vertical => {
                        let first_height = ((area.height as u32 * ratio as u32) / 1000) as u16;
                        let first_height = first_height.min(area.height);
                        let second_height = area.height.saturating_sub(first_height);

                        first.resolved(Rect::new(area.x, area.y, area.width, first_height), out);
                        second.resolved(
                            Rect::new(
                                area.x,
                                area.y.saturating_add(first_height),
                                area.width,
                                second_height,
                            ),
                            out,
                        );
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowKind {
    Edit,
    ReadOnly,
    ConfigDiagnostics,
    Help,
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
    next_window_id: u64,
    next_buffer_id: u64,
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

    pub fn focus_direction(
        &mut self,
        direction: Direction,
        area: Rect,
    ) -> Result<WindowId, WorkspaceError> {
        let layouts = self.resolved_layout(area);
        let Some(current) = layouts.iter().find(|layout| layout.id == self.focused) else {
            return Err(WorkspaceError::FocusMissing);
        };

        let next = layouts
            .iter()
            .filter(|layout| layout.id != self.focused)
            .filter_map(|layout| {
                neighbor_score(direction, current.rect, layout.rect).map(|score| (score, layout.id))
            })
            .min_by_key(|(score, _)| *score)
            .map(|(_, id)| id)
            .ok_or(WorkspaceError::NoNeighbor)?;

        self.focused = next;
        Ok(next)
    }

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

    pub fn equalize(&mut self) {
        equalize_node(&mut self.root);
    }

    pub fn rotate_focused_split(&mut self) -> Result<Axis, WorkspaceError> {
        match rotate_node(&mut self.root, self.focused) {
            RotateOutcome::Rotated(axis) => Ok(axis),
            RotateOutcome::Contains => Err(WorkspaceError::NoResizableSplit),
            RotateOutcome::Miss => Err(WorkspaceError::FocusMissing),
        }
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

fn neighbor_score(direction: Direction, current: Rect, candidate: Rect) -> Option<(u16, u16)> {
    match direction {
        Direction::Left if candidate.right() <= current.x => Some((
            current.x.saturating_sub(candidate.right()),
            center_delta(current.center_y(), candidate.center_y()),
        )),
        Direction::Right if candidate.x >= current.right() => Some((
            candidate.x.saturating_sub(current.right()),
            center_delta(current.center_y(), candidate.center_y()),
        )),
        Direction::Up if candidate.bottom() <= current.y => Some((
            current.y.saturating_sub(candidate.bottom()),
            center_delta(current.center_x(), candidate.center_x()),
        )),
        Direction::Down if candidate.y >= current.bottom() => Some((
            candidate.y.saturating_sub(current.bottom()),
            center_delta(current.center_x(), candidate.center_x()),
        )),
        _ => None,
    }
}

fn center_delta(a: i32, b: i32) -> u16 {
    a.abs_diff(b).min(u16::MAX as u32) as u16
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

fn equalize_node(node: &mut LayoutNode) {
    match node {
        LayoutNode::Leaf(_) => {}
        LayoutNode::Split {
            ratio,
            first,
            second,
            ..
        } => {
            *ratio = DEFAULT_SPLIT_RATIO;
            equalize_node(first);
            equalize_node(second);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::new(0, 0, 100, 40)
    }

    fn layout_rect(workspace: &Workspace, id: WindowId) -> Rect {
        workspace
            .resolved_layout(area())
            .into_iter()
            .find(|layout| layout.id == id)
            .expect("window should be in resolved layout")
            .rect
    }

    #[test]
    fn new_workspace_starts_with_one_untitled_window() {
        let workspace = Workspace::new_untitled();

        assert_eq!(workspace.window_count(), 1);
        assert_eq!(workspace.focused, WindowId(1));
        assert_eq!(workspace.root, LayoutNode::Leaf(WindowId(1)));
        assert_eq!(workspace.focused_window().unwrap().title, "Untitled");
    }

    #[test]
    fn split_horizontal_creates_right_hand_focused_window() {
        let mut workspace = Workspace::new_untitled();
        let first = workspace.focused;

        let second = workspace.split_focused(Axis::Horizontal).unwrap();

        assert_eq!(workspace.window_count(), 2);
        assert_eq!(workspace.focused, second);
        assert_eq!(layout_rect(&workspace, first), Rect::new(0, 0, 50, 40));
        assert_eq!(layout_rect(&workspace, second), Rect::new(50, 0, 50, 40));
    }

    #[test]
    fn split_vertical_creates_lower_focused_window() {
        let mut workspace = Workspace::new_untitled();
        let first = workspace.focused;

        let second = workspace.split_focused(Axis::Vertical).unwrap();

        assert_eq!(workspace.focused, second);
        assert_eq!(layout_rect(&workspace, first), Rect::new(0, 0, 100, 20));
        assert_eq!(layout_rect(&workspace, second), Rect::new(0, 20, 100, 20));
    }

    #[test]
    fn focus_direction_selects_geometric_neighbor() {
        let mut workspace = Workspace::new_untitled();
        let left = workspace.focused;
        let right = workspace.split_focused(Axis::Horizontal).unwrap();

        assert_eq!(
            workspace.focus_direction(Direction::Left, area()).unwrap(),
            left
        );
        assert_eq!(
            workspace.focus_direction(Direction::Right, area()).unwrap(),
            right
        );
    }

    #[test]
    fn focus_direction_reports_missing_neighbor() {
        let mut workspace = Workspace::new_untitled();

        assert_eq!(
            workspace.focus_direction(Direction::Left, area()),
            Err(WorkspaceError::NoNeighbor)
        );
    }

    #[test]
    fn resize_focused_changes_nearest_split_ratio() {
        let mut workspace = Workspace::new_untitled();
        let first = workspace.focused;
        let second = workspace.split_focused(Axis::Horizontal).unwrap();

        workspace.focused = first;
        assert_eq!(workspace.resize_focused(Direction::Right).unwrap(), 550);
        assert_eq!(layout_rect(&workspace, first), Rect::new(0, 0, 55, 40));
        assert_eq!(layout_rect(&workspace, second), Rect::new(55, 0, 45, 40));

        workspace.focused = second;
        assert_eq!(workspace.resize_focused(Direction::Left).unwrap(), 500);
        assert_eq!(layout_rect(&workspace, first), Rect::new(0, 0, 50, 40));
        assert_eq!(layout_rect(&workspace, second), Rect::new(50, 0, 50, 40));
    }

    #[test]
    fn resize_focused_reports_when_no_matching_edge_exists() {
        let mut workspace = Workspace::new_untitled();
        workspace.split_focused(Axis::Horizontal).unwrap();

        assert_eq!(
            workspace.resize_focused(Direction::Down),
            Err(WorkspaceError::NoResizableSplit)
        );
    }

    #[test]
    fn close_focused_removes_window_and_repairs_tree() {
        let mut workspace = Workspace::new_untitled();
        let first = workspace.focused;
        let second = workspace.split_focused(Axis::Horizontal).unwrap();

        assert_eq!(workspace.close_focused().unwrap(), first);

        assert_eq!(workspace.window_count(), 1);
        assert_eq!(workspace.focused, first);
        assert_eq!(workspace.root, LayoutNode::Leaf(first));
        assert!(workspace.window(second).is_err());
    }

    #[test]
    fn close_last_window_is_rejected() {
        let mut workspace = Workspace::new_untitled();

        assert_eq!(
            workspace.close_focused(),
            Err(WorkspaceError::CannotCloseLastWindow)
        );
        assert_eq!(workspace.window_count(), 1);
    }

    #[test]
    fn close_nested_window_promotes_sibling_subtree() {
        let mut workspace = Workspace::new_untitled();
        let left = workspace.focused;
        let right = workspace.split_focused(Axis::Horizontal).unwrap();
        let lower_right = workspace.split_focused(Axis::Vertical).unwrap();

        assert_eq!(workspace.focused, lower_right);
        assert_eq!(workspace.close_focused().unwrap(), right);

        assert_eq!(workspace.window_count(), 2);
        assert_eq!(workspace.focused, right);
        assert_eq!(layout_rect(&workspace, left), Rect::new(0, 0, 50, 40));
        assert_eq!(layout_rect(&workspace, right), Rect::new(50, 0, 50, 40));
    }

    #[test]
    fn collapse_expand_and_toggle_update_focused_window_state() {
        let mut workspace = Workspace::new_untitled();

        workspace.collapse_focused().unwrap();
        assert!(workspace.focused_window().unwrap().collapsed);

        workspace.expand_focused().unwrap();
        assert!(!workspace.focused_window().unwrap().collapsed);

        assert!(workspace.toggle_focused_collapse().unwrap());
        assert!(!workspace.toggle_focused_collapse().unwrap());
    }

    #[test]
    fn equalize_resets_all_split_ratios() {
        let mut workspace = Workspace::new_untitled();
        let first = workspace.focused;
        workspace.split_focused(Axis::Horizontal).unwrap();
        workspace.focused = first;
        workspace.resize_focused_by(Direction::Right, 200).unwrap();

        assert_eq!(layout_rect(&workspace, first), Rect::new(0, 0, 70, 40));

        workspace.equalize();

        assert_eq!(layout_rect(&workspace, first), Rect::new(0, 0, 50, 40));
    }

    #[test]
    fn rotate_focused_split_toggles_nearest_parent_axis() {
        let mut workspace = Workspace::new_untitled();
        let first = workspace.focused;
        let second = workspace.split_focused(Axis::Horizontal).unwrap();

        workspace.focused = first;
        assert_eq!(workspace.rotate_focused_split().unwrap(), Axis::Vertical);
        assert_eq!(layout_rect(&workspace, first), Rect::new(0, 0, 100, 20));
        assert_eq!(layout_rect(&workspace, second), Rect::new(0, 20, 100, 20));
    }
}
