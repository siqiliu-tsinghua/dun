use super::support::*;

#[test]
fn shell_resolves_keymap_commands() {
    let shell = UiShell::default();
    let sequence = KeySequence::from_str("Ctrl+S").unwrap();

    assert_eq!(
        shell.command_for_sequence(&sequence),
        Some(&EditorCommand::File(FileCommand::Save))
    );
}

#[test]
fn shell_resolves_single_strokes_and_describes_workspace() {
    let shell = UiShell::default();
    let stroke = KeyStroke::from_str("F1").unwrap();

    assert_eq!(
        shell.command_for_stroke(stroke),
        Some(&EditorCommand::App(AppCommand::Help))
    );

    let description = shell.describe_workspace(&Workspace::new_untitled());
    assert!(description.contains("theme=msedit"));
    assert!(description.contains("windows=1"));
    assert!(description.contains("border=┌──┐"));
}

#[test]
fn frame_contains_menu_status_and_sanitized_buffer_content() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "safe\x1b]0;x\x07");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();

    let frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    assert_eq!(frame.menu.items[0].label, "File");
    assert_eq!(frame.menu.items[1].label, "Edit");
    assert_eq!(frame.menu.items[2].label, "View");
    assert_eq!(frame.menu.items[3].label, "Help");
    assert_eq!(frame.status.focused_window, WindowId(1));
    assert_eq!(frame.windows.len(), 1);
    assert_eq!(frame.windows[0].body[0].as_plain_text(), "safe␛]0;x␇");
    assert!(frame.windows[0].body[0].has_non_text_segments());
    assert_eq!(frame.windows[0].gutter_width, 2);
    assert_eq!(
        frame.windows[0].gutter,
        vec![UiGutterLine {
            y: 1,
            label: "1 ".to_string(),
        }]
    );
    assert_eq!(frame.windows[0].cursor, Some(UiCursor { x: 3, y: 1 }));
}

#[test]
fn frame_renders_wrapped_lines_with_plain_gutter() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcd efghij");
    let buffer_view = BufferView::new(BufferId(1), &buffer).with_wrap(true);
    let shell = UiShell::default();

    let frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 12, 6), &[buffer_view]);

    assert_eq!(frame.windows[0].gutter[0].label, "1 ");
    assert_eq!(frame.windows[0].gutter[1].label, "  ");
    assert_eq!(frame.windows[0].body[0].as_plain_text(), "abcd efg");
    assert_eq!(frame.windows[0].body[1].as_plain_text(), "hij");
}

#[test]
fn frame_maps_buffer_cursor_to_window_body() {
    let workspace = Workspace::new_untitled();
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abc\né");
    buffer.set_cursor(dun_core::Position::new(1, 2)).unwrap();
    let buffer_view = BufferView::new(BufferId(1), &buffer);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    assert_eq!(frame.windows[0].cursor, Some(UiCursor { x: 4, y: 2 }));
}

#[test]
fn frame_maps_cursor_after_wide_utf8_text() {
    let workspace = Workspace::new_untitled();
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "中x");
    buffer
        .set_cursor(dun_core::Position::new(0, "中".len()))
        .unwrap();
    let buffer_view = BufferView::new(BufferId(1), &buffer);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    assert_eq!(frame.windows[0].cursor, Some(UiCursor { x: 5, y: 1 }));
}

#[test]
fn frame_maps_buffer_selection_to_window_body() {
    let workspace = Workspace::new_untitled();
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abc\n中x");
    buffer
        .select(Position::new(1, 0), Position::new(1, "中".len()))
        .unwrap();
    let buffer_view = BufferView::new(BufferId(1), &buffer);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    assert_eq!(
        frame.windows[0].selection,
        vec![UiSelectionLine {
            y: 2,
            start_x: 3,
            end_x: 5,
        }]
    );
}

#[test]
fn frame_maps_horizontal_scroll_to_body_cursor_and_selection() {
    let workspace = Workspace::new_untitled();
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdef");
    buffer.set_cursor(Position::new(0, 4)).unwrap();
    buffer
        .select(Position::new(0, 2), Position::new(0, 5))
        .unwrap();
    let buffer_view = BufferView::scrolled_xy(BufferId(1), &buffer, 0, 2);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    assert_eq!(frame.windows[0].body[0].as_plain_text(), "cdef");
    assert_eq!(frame.windows[0].cursor, Some(UiCursor { x: 6, y: 1 }));
    assert_eq!(
        frame.windows[0].selection,
        vec![UiSelectionLine {
            y: 1,
            start_x: 3,
            end_x: 6,
        }]
    );
}

#[test]
fn frame_maps_wrapped_selection_to_visual_rows() {
    let workspace = Workspace::new_untitled();
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdefghijklmnop");
    buffer
        .select(Position::new(0, 2), Position::new(0, 14))
        .unwrap();
    let buffer_view = BufferView::new(BufferId(1), &buffer).with_wrap(true);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 12, 6), &[buffer_view]);

    assert_eq!(frame.windows[0].body[0].as_plain_text(), "abcdefgh");
    assert_eq!(frame.windows[0].body[1].as_plain_text(), "ijklmnop");
    assert_eq!(
        frame.windows[0].selection,
        vec![
            UiSelectionLine {
                y: 1,
                start_x: 5,
                end_x: 11,
            },
            UiSelectionLine {
                y: 2,
                start_x: 3,
                end_x: 9,
            },
        ]
    );
}

#[test]
fn frame_starts_wrapped_body_at_visual_row_offset() {
    let workspace = Workspace::new_untitled();
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdefghijklmnop");
    buffer.set_cursor(Position::new(0, 10)).unwrap();
    buffer
        .select(Position::new(0, 2), Position::new(0, 14))
        .unwrap();
    let buffer_view = BufferView::new(BufferId(1), &buffer)
        .with_first_visual_row(1)
        .with_wrap(true);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 12, 6), &[buffer_view]);

    assert_eq!(frame.windows[0].body[0].as_plain_text(), "ijklmnop");
    assert_eq!(frame.windows[0].cursor, Some(UiCursor { x: 9, y: 1 }));
    assert_eq!(
        frame.windows[0].selection,
        vec![UiSelectionLine {
            y: 1,
            start_x: 3,
            end_x: 9,
        }]
    );
}

#[test]
fn frame_maps_search_matches_to_window_body() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "one two one");
    let matches = buffer.find_all("one");
    let buffer_view = BufferView::new(BufferId(1), &buffer).with_search(&matches, Some(0));

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    assert_eq!(
        frame.windows[0].search_matches,
        vec![
            UiSearchMatchLine {
                y: 1,
                start_x: 3,
                end_x: 6,
                active: true,
            },
            UiSearchMatchLine {
                y: 1,
                start_x: 11,
                end_x: 14,
                active: false,
            },
        ]
    );
}

#[test]
fn frame_maps_wrapped_search_matches_to_visual_rows() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdefghijklmnop");
    let matches = buffer.find_all("efghij");
    let buffer_view = BufferView::new(BufferId(1), &buffer)
        .with_wrap(true)
        .with_search(&matches, Some(0));

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 12, 6), &[buffer_view]);

    assert_eq!(
        frame.windows[0].search_matches,
        vec![
            UiSearchMatchLine {
                y: 1,
                start_x: 7,
                end_x: 11,
                active: true,
            },
            UiSearchMatchLine {
                y: 2,
                start_x: 3,
                end_x: 5,
                active: true,
            },
        ]
    );
}

#[test]
fn frame_clips_search_matches_by_horizontal_scroll() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdef");
    let matches = buffer.find_all("cd");
    let buffer_view =
        BufferView::scrolled_xy(BufferId(1), &buffer, 0, 2).with_search(&matches, Some(0));

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    assert_eq!(
        frame.windows[0].search_matches,
        vec![UiSearchMatchLine {
            y: 1,
            start_x: 3,
            end_x: 5,
            active: true,
        }]
    );
}

#[test]
fn frame_keeps_wide_char_horizontal_viewport_on_utf8_boundaries() {
    let workspace = Workspace::new_untitled();
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "中abc");
    buffer
        .select(Position::new(0, 0), Position::new(0, "中".len()))
        .unwrap();
    let matches = buffer.find_all("中");
    let buffer_view =
        BufferView::scrolled_xy(BufferId(1), &buffer, 0, 1).with_search(&matches, Some(0));

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 10, 4), &[buffer_view]);

    assert_eq!(frame.windows[0].body[0].as_plain_text(), "abc");
    assert!(frame.windows[0].selection.is_empty());
    assert!(frame.windows[0].search_matches.is_empty());
    assert_eq!(
        frame.windows[0].horizontal_edges,
        vec![UiHorizontalEdgeLine {
            y: 1,
            left: true,
            right: false,
        }]
    );
}

#[test]
fn frame_reports_vertical_scrollbar_for_scrolled_buffer() {
    let workspace = Workspace::new_untitled();
    let buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, "1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
    let buffer_view = BufferView::scrolled(BufferId(1), &buffer, 3);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 6), &[buffer_view]);

    assert_eq!(
        frame.windows[0].scrollbar,
        Some(UiScrollbar { y: 2, height: 2 })
    );
}

#[test]
fn frame_reports_horizontal_edge_indicators() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "0123456789");
    let buffer_view = BufferView::scrolled_xy(BufferId(1), &buffer, 0, 2);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 10, 4), &[buffer_view]);

    assert_eq!(
        frame.windows[0].horizontal_edges,
        vec![UiHorizontalEdgeLine {
            y: 1,
            left: true,
            right: true,
        }]
    );
}

#[test]
fn frame_maps_scrolled_line_number_gutter() {
    let workspace = Workspace::new_untitled();
    let buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, "1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
    let buffer_view = BufferView::scrolled(BufferId(1), &buffer, 8);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 6), &[buffer_view]);

    assert_eq!(frame.windows[0].gutter_width, 3);
    assert_eq!(
        frame.windows[0].gutter,
        vec![
            UiGutterLine {
                y: 1,
                label: " 9 ".to_string(),
            },
            UiGutterLine {
                y: 2,
                label: "10 ".to_string(),
            },
        ]
    );
}

#[test]
fn narrow_window_omits_wide_gutter_to_keep_body_columns() {
    let workspace = Workspace::new_untitled();
    let text = (0..1000).map(|_| "x").collect::<Vec<_>>().join("\n");
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
    let buffer_view = BufferView::new(BufferId(1), &buffer);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 8, 6), &[buffer_view]);

    assert_eq!(frame.windows[0].gutter_width, 0);
    assert!(frame.windows[0].gutter.is_empty());
    assert_eq!(frame.windows[0].body[0].as_plain_text(), "x");
    assert_eq!(frame.windows[0].cursor, Some(UiCursor { x: 1, y: 1 }));
}

#[test]
fn tiny_windows_have_no_body_gutter_or_cursor() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hidden");
    let buffer_view = BufferView::new(BufferId(1), &buffer);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 4, 2), &[buffer_view]);

    assert!(frame.windows[0].body.is_empty());
    assert_eq!(frame.windows[0].gutter_width, 0);
    assert!(frame.windows[0].gutter.is_empty());
    assert_eq!(frame.windows[0].cursor, None);
}

#[test]
fn status_text_is_clipped_by_display_width() {
    let text = status_text_for_width(
        "日志服务-error.log*",
        "Ln 100/200 Col 42 | utf-8/256",
        12,
        '…',
    );

    assert!(display_width(&text) <= 12);
    assert_eq!(text.chars().last(), Some('…'));

    let text = status_text_for_width("file", "Ln 1", 12, '…');

    assert_eq!(display_width(&text), 12);
    assert!(text.starts_with("file"));
    assert!(text.ends_with("Ln 1"));
}

#[test]
fn window_title_is_clipped_by_display_width() {
    let mut workspace = Workspace::new_untitled();
    workspace.window_mut(WindowId(1)).unwrap().title = "日志服务-error.log".to_string();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    let title = window_title_for_width(&shell, &frame.windows[0], 8);

    assert!(display_width(&title) <= 8);
    assert_eq!(
        title.chars().last(),
        Some(shell.glyphs.indicators.truncation)
    );
}

#[test]
fn frame_uses_tiled_workspace_rectangles() {
    let mut workspace = Workspace::new_untitled();
    workspace.split_focused(Axis::Horizontal).unwrap();

    let first = TextBuffer::from_text_with_kind(BufferKind::Untitled, "left");
    let second = TextBuffer::from_text_with_kind(BufferKind::Untitled, "right");
    let buffers = [
        BufferView::new(BufferId(1), &first),
        BufferView::new(BufferId(2), &second),
    ];

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 20), &buffers);

    assert_eq!(frame.windows.len(), 2);
    assert_eq!(frame.windows[0].rect, Rect::new(0, 0, 40, 20));
    assert_eq!(frame.windows[1].rect, Rect::new(40, 0, 40, 20));
    assert!(frame.windows[1].focused);
}

#[test]
fn collapsed_window_has_no_body_lines() {
    let mut workspace = Workspace::new_untitled();
    workspace.collapse_focused().unwrap();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "hidden");
    let buffer_view = BufferView::new(BufferId(1), &buffer);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    assert!(frame.windows[0].collapsed);
    assert!(frame.windows[0].body.is_empty());
}

#[test]
fn dirty_and_readonly_flags_follow_buffer_state() {
    let workspace = Workspace::new_untitled();
    let mut buffer = TextBuffer::from_text_with_kind(BufferKind::ReadOnly, "locked");
    buffer.set_cursor(Position::new(0, 0)).unwrap();
    let buffer_view = BufferView::new(BufferId(1), &buffer);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    assert!(frame.windows[0].read_only);
    assert!(!frame.windows[0].dirty);
}

#[test]
fn menu_exposes_help_and_quit_commands() {
    let menu = UiShell::default().menu_bar(None);
    let commands = menu
        .items
        .iter()
        .flat_map(|item| item.entries.iter().map(|entry| &entry.command))
        .collect::<Vec<_>>();

    assert!(
        commands
            .iter()
            .any(|command| **command == EditorCommand::App(AppCommand::Help))
    );
    assert!(
        commands
            .iter()
            .any(|command| **command == EditorCommand::App(AppCommand::StatusHistory))
    );
    assert!(
        commands
            .iter()
            .any(|command| **command == EditorCommand::App(AppCommand::SearchResults))
    );
    assert!(
        commands
            .iter()
            .any(|command| **command == EditorCommand::App(AppCommand::ReloadConfig))
    );
    assert!(
        commands
            .iter()
            .any(|command| **command == EditorCommand::App(AppCommand::ConfigDiagnostics))
    );
    assert!(
        commands
            .iter()
            .any(|command| **command == EditorCommand::App(AppCommand::Quit))
    );
}

#[test]
fn menu_mnemonics_are_unique_within_each_menu() {
    let menu = UiShell::default().menu_bar(None);

    for item in menu.items {
        let mut seen = Vec::new();
        for entry in item.entries {
            let Some(mnemonic) = menu_entry_mnemonic(entry.label) else {
                continue;
            };
            assert!(
                !seen.contains(&mnemonic),
                "{} menu repeats mnemonic {mnemonic} in {}",
                item.label,
                entry.label
            );
            seen.push(mnemonic);
        }
    }
}

#[test]
fn frame_maps_plugin_highlight_spans_to_window_body() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "fn main() {}\nbody");
    let highlights = [
        BufferHighlightSpan {
            line: 0,
            start_column: 0,
            end_column: 2,
            class: HighlightClass::Keyword,
        },
        BufferHighlightSpan {
            line: 5,
            start_column: 0,
            end_column: 1,
            class: HighlightClass::Comment,
        },
    ];
    let buffer_view = BufferView::new(BufferId(1), &buffer).with_highlight_spans(&highlights);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    assert_eq!(
        frame.windows[0].highlights,
        vec![UiHighlightLine {
            y: 1,
            start_x: 3,
            end_x: 5,
            class: HighlightClass::Keyword,
        }],
        "visible span maps beside the gutter; the off-screen line is dropped"
    );
}

#[test]
fn frame_clips_highlight_spans_to_horizontal_scroll() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdef");
    let highlights = [BufferHighlightSpan {
        line: 0,
        start_column: 0,
        end_column: 4,
        class: HighlightClass::StringLiteral,
    }];
    let buffer_view =
        BufferView::scrolled_xy(BufferId(1), &buffer, 0, 2).with_highlight_spans(&highlights);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    assert_eq!(
        frame.windows[0].highlights,
        vec![UiHighlightLine {
            y: 1,
            start_x: 3,
            end_x: 5,
            class: HighlightClass::StringLiteral,
        }],
        "columns before the horizontal scroll origin are clipped"
    );
}

#[test]
fn frame_maps_wrapped_highlight_spans_to_visual_rows() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdefghijkl");
    let highlights = [BufferHighlightSpan {
        line: 0,
        start_column: 6,
        end_column: 10,
        class: HighlightClass::Number,
    }];
    let buffer_view = BufferView::new(BufferId(1), &buffer)
        .with_wrap(true)
        .with_highlight_spans(&highlights);

    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 12, 6), &[buffer_view]);

    assert_eq!(
        frame.windows[0].highlights,
        vec![
            UiHighlightLine {
                y: 1,
                start_x: 9,
                end_x: 11,
                class: HighlightClass::Number,
            },
            UiHighlightLine {
                y: 2,
                start_x: 3,
                end_x: 5,
                class: HighlightClass::Number,
            },
        ],
        "a span crossing the wrap boundary produces one segment per visual row"
    );
}
