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
