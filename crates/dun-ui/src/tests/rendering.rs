use super::support::*;

#[test]
fn short_dropdown_keeps_selected_menu_entry_visible() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let last_entry = shell.menu_entry_count(2).unwrap() - 1;
    let ui_frame = shell.frame_for_workspace_with_menu_selection(
        &workspace,
        Rect::new(0, 0, 80, 10),
        &[buffer_view],
        Some(MenuSelection::with_entry(2, last_entry)),
    );
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| shell.render(frame, &ui_frame))
        .unwrap();

    let snapshot = terminal_text_snapshot(terminal.backend().buffer(), 80, 12);
    assert!(snapshot.contains("Reload Config"));
    assert!(!snapshot.contains("Split Horizontal"));
    assert!(snapshot.contains(vertical_overflow_up(&shell)));
}

#[test]
fn ratatui_renderer_draws_frame_without_panicking() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hello\nworld");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 60, 8), &[buffer_view]);
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| shell.render(frame, &ui_frame))
        .unwrap();
}

#[test]
fn ratatui_renderer_draws_tiny_tiled_frame_without_panicking() {
    let mut workspace = Workspace::new_untitled();
    workspace.split_focused(Axis::Horizontal).unwrap();
    let first = TextBuffer::from_text_with_kind(BufferKind::Untitled, "left");
    let second = TextBuffer::from_text_with_kind(BufferKind::Untitled, "right");
    let buffers = [
        BufferView::new(BufferId(1), &first),
        BufferView::new(BufferId(2), &second),
    ];
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 8, 2), &buffers);
    let backend = TestBackend::new(8, 4);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| shell.render(frame, &ui_frame))
        .unwrap();
}

#[test]
fn ratatui_renderer_draws_active_submenu() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace_with_menu(
        &workspace,
        Rect::new(0, 0, 80, 12),
        &[buffer_view],
        Some(0),
    );
    let backend = TestBackend::new(80, 14);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| shell.render(frame, &ui_frame))
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("File"));
    assert!(rendered.contains("Save As"));
    assert!(rendered.contains("Quit"));
}

#[test]
fn ratatui_renderer_draws_view_output_commands_in_submenu() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace_with_menu_selection(
        &workspace,
        Rect::new(0, 0, 90, 36),
        &[buffer_view],
        Some(MenuSelection::menu_only(2)),
    );
    let backend = TestBackend::new(90, 38);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| shell.render(frame, &ui_frame))
        .unwrap();

    let snapshot = terminal_text_snapshot(terminal.backend().buffer(), 90, 38);
    assert!(snapshot.contains("View"));
    assert!(snapshot.contains("Outline"));
    assert!(snapshot.contains("Search Results"));
    assert!(snapshot.contains("Output Index"));
    assert!(snapshot.contains("Output Summary"));
    assert!(snapshot.contains("Output Status"));
    assert!(snapshot.contains("Output Stdout"));
    assert!(snapshot.contains("Output Stdout Body"));
    assert!(snapshot.contains("Output Stderr"));
    assert!(snapshot.contains("Output Stderr Body"));
    assert!(snapshot.contains("Output Truncated"));
    assert!(snapshot.contains("Output Next Match"));
    assert!(snapshot.contains("Output Previous Match"));
    assert!(snapshot.contains("Output Next Section"));
    assert!(snapshot.contains("Output Previous Section"));
    assert!(snapshot.contains("Output Only Stdout"));
    assert!(snapshot.contains("Output Only Stderr"));
    assert!(snapshot.contains("Output Save"));
    assert!(snapshot.contains("Output Clear"));
}

#[test]
fn ratatui_text_snapshot_covers_menu_window_and_status_layout() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "alpha\nbeta");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 40, 6), &[buffer_view]);
    let backend = TestBackend::new(40, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| shell.render(frame, &ui_frame))
        .unwrap();

    let snapshot = terminal_text_snapshot(terminal.backend().buffer(), 40, 8);
    assert!(snapshot.contains("File"));
    assert!(snapshot.contains("alpha"));
    assert!(snapshot.contains("1 window"));
}

#[test]
fn ratatui_renderer_draws_prompt_overlay() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let mut ui_frame =
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);
    ui_frame.overlay = Some(UiOverlay::prompt("Go To Line", "12", 2));
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| shell.render(frame, &ui_frame))
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Go To Line"));
    assert!(rendered.contains("12"));
}

#[test]
fn ratatui_renderer_draws_file_dialog_overlay() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let mut ui_frame =
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 90, 14), &[buffer_view]);
    ui_frame.overlay = Some(UiOverlay::file_dialog(
        "Open",
        vec![
            "Directory: /tmp".to_string(),
            "Select a file or type a path.".to_string(),
        ],
        "/tmp/al",
        7,
        vec!["[D] logs/".to_string(), "    alpha.log".to_string()],
        Some(1),
        vec!["Enter  Tab complete  Up/Down select  Esc cancel".to_string()],
    ));
    let backend = TestBackend::new(90, 16);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| shell.render(frame, &ui_frame))
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Open"));
    assert!(rendered.contains("Directory: /tmp"));
    assert!(rendered.contains("/tmp/al"));
    assert!(rendered.contains("logs"));
    assert!(rendered.contains("alpha.log"));
}

#[test]
fn ratatui_renderer_draws_modal_list_overflow_indicators() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let mut ui_frame =
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 60, 10), &[buffer_view]);
    ui_frame.overlay = Some(
        UiOverlay::message(
            "Switch Buffer",
            vec!["Showing 5-7 of 20".to_string()],
            vec![],
        )
        .with_list(
            vec![
                "  buffer-05".to_string(),
                "> buffer-06".to_string(),
                "  buffer-07".to_string(),
            ],
            Some(1),
            32,
        )
        .with_list_overflow(true, true),
    );
    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| shell.render(frame, &ui_frame))
        .unwrap();

    let snapshot = terminal_text_snapshot(terminal.backend().buffer(), 60, 12);
    assert!(snapshot.contains(vertical_overflow_up(&shell)));
    assert!(snapshot.contains(vertical_overflow_down(&shell)));
    assert!(snapshot.contains("> buffer-06"));
}

#[test]
fn ratatui_renderer_draws_viewport_polish_markers() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(
        BufferKind::Untitled,
        "0123456789\nline2\nline3\nline4\nline5\nline6\nline7\nline8",
    );
    let buffer_view = BufferView::scrolled_xy(BufferId(1), &buffer, 2, 2);
    let shell = UiShell::default();
    let ui_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 20, 6), &[buffer_view]);
    let backend = TestBackend::new(20, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| shell.render(frame, &ui_frame))
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains('‹'));
    assert!(rendered.contains('█'));
}
