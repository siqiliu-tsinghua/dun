use super::support::*;

fn body_text(shell: &UiShell, buffer: &TextBuffer, visible: bool) -> Vec<String> {
    let workspace = Workspace::new_untitled();
    let buffer_view = BufferView::new(BufferId(1), buffer).with_visible_whitespace(visible);
    shell
        .frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view])
        .windows[0]
        .body
        .iter()
        .map(|line| line.as_plain_text())
        .collect()
}

#[test]
fn utf8_markers_cover_space_tab_logical_eol_and_empty_line() {
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "a \tb\n");

    assert_eq!(
        body_text(&UiShell::default(), &buffer, true),
        ["a·→b¶", "¶"]
    );
}

#[test]
fn ascii_markers_cover_space_tab_logical_eol_and_empty_line() {
    let config = Config {
        terminal: TerminalOverrides {
            encoding: Some(EncodingProfile::Ascii),
            ..TerminalOverrides::default()
        },
        ..Config::default()
    };
    let shell = UiShell::from_config(&config, TerminalProfile::default());
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "a \tb\n");

    assert_eq!(body_text(&shell, &buffer, true), ["a.>b$", "$"]);
}

#[test]
fn visible_whitespace_defaults_off_without_changing_rendered_bytes() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "a \tb\n");
    let shell = UiShell::default();
    let default_view = BufferView::new(BufferId(1), &buffer);
    let explicit_off = BufferView::new(BufferId(1), &buffer).with_visible_whitespace(false);

    let default_frame =
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[default_view]);
    let explicit_frame =
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[explicit_off]);

    assert_eq!(default_frame, explicit_frame);
    assert_eq!(
        default_frame.windows[0]
            .body
            .iter()
            .map(|line| line.as_plain_text())
            .collect::<Vec<_>>(),
        ["a ␉b", ""]
    );
}

#[test]
fn bookmark_gutter_marks_one_and_two_digit_logical_lines_without_widening() {
    let workspace = Workspace::new_untitled();
    let text = (1..=12)
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
    let bookmarks = [0, 9];
    let buffer_view = BufferView::new(BufferId(1), &buffer).with_bookmarks(&bookmarks);
    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 80, 14), &[buffer_view]);
    let window = &frame.windows[0];

    assert_eq!(window.geometry.gutter.width, 3);
    assert_eq!(window.gutter[0].label, " 1*");
    assert!(window.gutter[0].marked);
    assert_eq!(window.gutter[9].label, "10*");
    assert!(window.gutter[9].marked);
    assert_eq!(
        window.geometry.body.x, 4,
        "bookmark must not widen the gutter"
    );
}

#[test]
fn bookmark_gutter_uses_the_profile_glyph_in_wide_mode() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "one");
    let bookmarks = [0];
    let buffer_view = BufferView::new(BufferId(1), &buffer).with_bookmarks(&bookmarks);
    let mut shell = UiShell::default();
    shell.profile.ambiguous_width = dun_term::AmbiguousWidth::Wide;

    let frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 5), &[buffer_view]);

    assert_eq!(frame.windows[0].geometry.gutter.width, 3);
    assert_eq!(frame.windows[0].gutter[0].label, "1*");
    assert!(frame.windows[0].gutter[0].marked);
}

#[test]
fn bookmark_marks_only_the_first_wrapped_row_even_from_a_continuation_viewport() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "abcdefghijklmnop\nz");
    let bookmarks = [0, 1];
    let shell = UiShell::default();
    let top_view = BufferView::new(BufferId(1), &buffer)
        .with_wrap(true)
        .with_bookmarks(&bookmarks);
    let top_frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 12, 6), &[top_view]);

    assert_eq!(top_frame.windows[0].gutter[0].label, "1*");
    assert!(top_frame.windows[0].gutter[0].marked);
    assert_eq!(top_frame.windows[0].gutter[1].label, "  ");
    assert!(!top_frame.windows[0].gutter[1].marked);

    let continuation_view = BufferView::new(BufferId(1), &buffer)
        .with_first_visual_row(1)
        .with_wrap(true)
        .with_bookmarks(&bookmarks);
    let continuation_frame =
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 12, 6), &[continuation_view]);

    assert_eq!(continuation_frame.windows[0].gutter[0].label, "  ");
    assert!(!continuation_frame.windows[0].gutter[0].marked);
    assert_eq!(continuation_frame.windows[0].gutter[1].label, "2*");
    assert!(continuation_frame.windows[0].gutter[1].marked);
}

#[test]
fn narrow_windows_omit_bookmark_gutters_with_large_line_counts() {
    let workspace = Workspace::new_untitled();
    let text = (0..1000).map(|_| "x").collect::<Vec<_>>().join("\n");
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
    let bookmarks = [0, 999];
    let buffer_view = BufferView::new(BufferId(1), &buffer).with_bookmarks(&bookmarks);
    let frame =
        UiShell::default().frame_for_workspace(&workspace, Rect::new(0, 0, 8, 6), &[buffer_view]);

    assert_eq!(frame.windows[0].geometry.gutter.width, 0);
    assert!(frame.windows[0].gutter.is_empty());
    assert_eq!(frame.windows[0].geometry.body.x, 1);
}

#[test]
fn surface_keeps_bookmark_marker_and_unmarked_separator_in_the_edge_cell() {
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "a\nb");
    let bookmarks = [0];
    let buffer_view = BufferView::new(BufferId(1), &buffer).with_bookmarks(&bookmarks);
    let shell = UiShell::default();
    let frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 20, 4), &[buffer_view]);
    let mut surface = crate::surface::Surface::new(20, 6, shell.theme.palette.editor);

    crate::render::surface_frame::render_ui_frame_to_surface(&mut surface, &shell, &frame);

    assert_eq!(
        surface.cell(2, 2).unwrap().symbol,
        "*",
        "the separator must not overwrite a marked gutter edge"
    );
    assert_eq!(
        surface.cell(2, 3).unwrap().symbol,
        shell.glyphs.border.vertical.to_string(),
        "an unmarked gutter edge still draws the separator"
    );
}
