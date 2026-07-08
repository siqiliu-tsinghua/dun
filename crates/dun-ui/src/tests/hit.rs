use super::support::*;

#[test]
fn menu_hit_tests_map_columns_and_dropdown_rows() {
    let shell = UiShell::default();

    assert_eq!(shell.menu_index_at_column(2), Some(0));
    assert_eq!(shell.menu_index_at_column(8), Some(1));
    assert_eq!(shell.menu_index_at_column(20), Some(3));
    assert_eq!(shell.menu_index_at_column(0), None);
    assert_eq!(
        shell.menu_entry_command_at(0, 2, 2),
        Some(EditorCommand::File(FileCommand::New))
    );
    assert_eq!(
        shell.menu_entry_command_at(3, 20, 2),
        Some(EditorCommand::App(AppCommand::Help))
    );
    assert_eq!(shell.menu_entry_command_at(3, 0, 2), None);
}

#[test]
fn hit_test_maps_body_click_to_buffer_position() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcd");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();

    let hit = shell
        .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view], 5, 1)
        .unwrap();

    assert_eq!(hit.window_id, WindowId(1));
    assert_eq!(hit.buffer_id, BufferId(1));
    assert_eq!(hit.target, UiMouseTarget::Body(Position::new(0, 2)));
}

#[test]
fn hit_test_maps_wide_character_click_to_valid_utf8_boundary() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "中x");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();

    let hit = shell
        .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view], 4, 1)
        .unwrap();

    assert_eq!(
        hit.target,
        UiMouseTarget::Body(Position::new(0, "中".len()))
    );
}

#[test]
fn hit_test_separates_window_chrome_and_gutter() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcd");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();

    let chrome = shell
        .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view], 0, 0)
        .unwrap();
    let gutter = shell
        .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view], 1, 1)
        .unwrap();

    assert_eq!(chrome.target, UiMouseTarget::Chrome);
    assert_eq!(gutter.target, UiMouseTarget::Gutter);
}

#[test]
fn hit_test_maps_empty_body_area_to_buffer_end() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcd");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();

    let hit = shell
        .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view], 10, 5)
        .unwrap();

    assert_eq!(hit.target, UiMouseTarget::Body(Position::new(0, 4)));
}

#[test]
fn hit_test_accounts_for_horizontal_scroll() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdef");
    let buffer_view = BufferView::scrolled_xy(BufferId(1), &buffer, 0, 2);
    let shell = UiShell::default();

    let hit = shell
        .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view], 3, 1)
        .unwrap();

    assert_eq!(hit.target, UiMouseTarget::Body(Position::new(0, 2)));
}

#[test]
fn hit_test_maps_scrollbar_click_to_target_line() {
    let workspace = Workspace::new_untitled();
    let buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, "1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();

    let hit = shell
        .hit_test_workspace(&workspace, Rect::new(0, 0, 80, 6), &[buffer_view], 79, 3)
        .unwrap();

    assert_eq!(
        hit.target,
        UiMouseTarget::Scrollbar {
            first_line: 4,
            first_visual_row: 0,
        }
    );
}

#[test]
fn scrolled_dropdown_hit_test_tracks_visible_entry_range() {
    let shell = UiShell::default();
    let menu = shell.menu_bar(None);
    let last_entry = shell.menu_entry_count(2).unwrap() - 1;
    let active = MenuSelection::with_entry(2, last_entry);
    let area = Rect::new(0, 0, 80, 10);
    let rect = clamp_menu_rect(
        dropdown_rect_for_menu(&shell, &menu, 2).unwrap(),
        TuiRect::new(area.x, area.y, area.width, area.height),
    )
    .unwrap();
    let (start, _) =
        menu_visible_entry_range(last_entry + 1, active.entry_index, rect.height as usize - 2)
            .unwrap();
    let row = rect.y + 1 + (last_entry - start) as u16;

    assert_eq!(
        shell.menu_entry_command_at_in_area(active, rect.x + 2, row, area),
        Some(EditorCommand::App(AppCommand::ReloadConfig))
    );
}

#[test]
fn overlay_hit_test_maps_file_dialog_list_rows() {
    let shell = UiShell::default();
    let overlay = UiOverlay::file_dialog(
        "Open",
        vec![
            "Directory: /tmp".to_string(),
            "Select a file or type a path.".to_string(),
        ],
        "/tmp/",
        5,
        vec!["[D] logs/".to_string(), "    alpha.log".to_string()],
        Some(0),
        vec!["Enter  Tab complete  Up/Down select  Esc cancel".to_string()],
    );
    let area = Rect::new(0, 0, 90, 16);

    assert_eq!(shell.hit_test_overlay_list(&overlay, area, 20, 8), Some(0));
    assert_eq!(shell.hit_test_overlay_list(&overlay, area, 20, 9), Some(1));
    assert_eq!(shell.hit_test_overlay_list(&overlay, area, 20, 7), None);
}
