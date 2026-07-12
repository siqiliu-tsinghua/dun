use super::model::SplitPathStep;
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
fn window_at_returns_window_containing_point() {
    let mut workspace = Workspace::new_untitled();
    let left = workspace.focused;
    let right = workspace.split_focused(Axis::Horizontal).unwrap();

    assert_eq!(workspace.window_at(area(), 10, 10), Some(left));
    assert_eq!(workspace.window_at(area(), 60, 10), Some(right));
    assert_eq!(workspace.window_at(area(), 100, 10), None);
}

#[test]
fn focus_at_updates_focused_window_for_point() {
    let mut workspace = Workspace::new_untitled();
    let left = workspace.focused;
    workspace.split_focused(Axis::Horizontal).unwrap();

    assert_eq!(workspace.focus_at(area(), 10, 10), Some(left));
    assert_eq!(workspace.focused, left);
    assert_eq!(workspace.focus_at(area(), 100, 10), None);
    assert_eq!(workspace.focused, left);
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
fn resize_focused_changes_vertical_split_ratio_from_both_children() {
    let mut workspace = Workspace::new_untitled();
    let first = workspace.focused;
    let second = workspace.split_focused(Axis::Vertical).unwrap();

    workspace.focused = first;
    assert_eq!(workspace.resize_focused(Direction::Down).unwrap(), 550);
    assert_eq!(layout_rect(&workspace, first), Rect::new(0, 0, 100, 22));
    assert_eq!(layout_rect(&workspace, second), Rect::new(0, 22, 100, 18));

    workspace.focused = second;
    assert_eq!(workspace.resize_focused(Direction::Up).unwrap(), 500);
    assert_eq!(layout_rect(&workspace, first), Rect::new(0, 0, 100, 20));
    assert_eq!(layout_rect(&workspace, second), Rect::new(0, 20, 100, 20));
}

#[test]
fn split_at_returns_handle_for_split_boundary() {
    let mut workspace = Workspace::new_untitled();
    workspace.split_focused(Axis::Horizontal).unwrap();

    assert!(workspace.split_at(area(), 49, 10).is_some());
    assert!(workspace.split_at(area(), 50, 10).is_some());
    assert!(workspace.split_at(area(), 10, 10).is_none());
}

#[test]
fn resize_split_to_moves_split_boundary_to_coordinate() {
    let mut workspace = Workspace::new_untitled();
    let first = workspace.focused;
    let second = workspace.split_focused(Axis::Horizontal).unwrap();
    let handle = workspace.split_at(area(), 50, 10).unwrap();

    assert_eq!(workspace.resize_split_to(&handle, area(), 75, 10), Ok(750));

    assert_eq!(layout_rect(&workspace, first), Rect::new(0, 0, 75, 40));
    assert_eq!(layout_rect(&workspace, second), Rect::new(75, 0, 25, 40));
}

#[test]
fn resize_split_to_clamps_to_supported_ratio_range() {
    let mut workspace = Workspace::new_untitled();
    let first = workspace.focused;
    let second = workspace.split_focused(Axis::Horizontal).unwrap();
    let handle = workspace.split_at(area(), 50, 10).unwrap();

    assert_eq!(workspace.resize_split_to(&handle, area(), 99, 10), Ok(900));

    assert_eq!(layout_rect(&workspace, first), Rect::new(0, 0, 90, 40));
    assert_eq!(layout_rect(&workspace, second), Rect::new(90, 0, 10, 40));
}

#[test]
fn resize_split_to_handles_vertical_tiny_and_invalid_paths() {
    let mut workspace = Workspace::new_untitled();
    workspace.split_focused(Axis::Vertical).unwrap();
    let handle = workspace.split_at(area(), 10, 20).unwrap();

    assert_eq!(
        workspace.resize_split_to(&handle, Rect::new(0, 0, 100, 1), 10, 0),
        Ok(500)
    );

    let invalid_handle = SplitDragHandle {
        path: vec![SplitPathStep::First],
    };
    assert_eq!(
        workspace.resize_split_to(&invalid_handle, area(), 10, 10),
        Err(WorkspaceError::NoResizableSplit)
    );
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
fn workspace_reports_focus_missing_for_corrupt_focus() {
    let mut workspace = Workspace::new_untitled();
    workspace.split_focused(Axis::Horizontal).unwrap();
    workspace.focused = WindowId(999);

    assert_eq!(
        workspace.split_focused(Axis::Horizontal),
        Err(WorkspaceError::FocusMissing)
    );
    assert_eq!(
        workspace.resize_focused(Direction::Right),
        Err(WorkspaceError::FocusMissing)
    );
    assert_eq!(
        workspace.rotate_focused_split(),
        Err(WorkspaceError::FocusMissing)
    );
    assert_eq!(workspace.close_focused(), Err(WorkspaceError::FocusMissing));
    assert_eq!(workspace.only_focused(), Err(WorkspaceError::FocusMissing));
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
fn only_focused_keeps_the_focused_window_and_returns_the_others() {
    let mut workspace = Workspace::new_untitled();
    let first = workspace.focused;
    let second = workspace.split_focused(Axis::Horizontal).unwrap();
    let third = workspace.split_focused(Axis::Vertical).unwrap();
    workspace.focused = second;
    workspace.collapse_focused().unwrap();

    let removed = workspace.only_focused().unwrap();

    assert_eq!(workspace.window_count(), 1);
    assert_eq!(workspace.focused, second);
    assert_eq!(workspace.windows[0].id, second);
    assert_eq!(workspace.root, LayoutNode::Leaf(second));
    assert!(!workspace.focused_window().unwrap().collapsed);
    assert_eq!(
        removed.iter().map(|window| window.id).collect::<Vec<_>>(),
        vec![first, third]
    );
}

#[test]
fn only_focused_leaves_a_single_window_workspace_unchanged() {
    let mut workspace = Workspace::new_untitled();
    let before = workspace.clone();

    assert_eq!(workspace.only_focused(), Ok(Vec::new()));
    assert_eq!(workspace, before);
}

#[test]
fn collapse_expand_and_toggle_update_focused_window_state() {
    let mut workspace = Workspace::new_untitled();
    // Collapsing needs somewhere for the room to go: the only window refuses.
    workspace.split_focused(Axis::Horizontal).unwrap();

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

#[test]
fn resolved_layout_keeps_split_children_visible_when_possible() {
    let mut workspace = Workspace::new_untitled();
    let first = workspace.focused;
    let second = workspace.split_focused(Axis::Horizontal).unwrap();
    workspace.focused = second;
    workspace.resize_focused_by(Direction::Left, 500).unwrap();

    let layouts = workspace.resolved_layout(Rect::new(0, 0, 3, 2));

    assert_eq!(
        layouts
            .iter()
            .find(|layout| layout.id == first)
            .unwrap()
            .rect,
        Rect::new(0, 0, 1, 2)
    );
    assert_eq!(
        layouts
            .iter()
            .find(|layout| layout.id == second)
            .unwrap()
            .rect,
        Rect::new(1, 0, 2, 2)
    );
}

/// Collapsing the only window hid the editor body while keystrokes kept editing
/// the buffer behind it -- the user blind-typed into a file they could not see.
/// Collapsing exists to give room to the other panes; with none, it only takes.
#[test]
fn the_only_window_cannot_be_collapsed() {
    let mut workspace = Workspace::new_untitled();

    assert_eq!(
        workspace.collapse_focused(),
        Err(WorkspaceError::CannotCollapseLastWindow)
    );
    assert_eq!(
        workspace.toggle_focused_collapse(),
        Err(WorkspaceError::CannotCollapseLastWindow)
    );
    assert!(!workspace.focused_window().unwrap().collapsed);
    assert!(!workspace.focused_is_collapsed());
}
