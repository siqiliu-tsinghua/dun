use super::support::*;

#[test]
fn shell_applies_configured_terminal_fallbacks() {
    let config = Config {
        terminal: TerminalOverrides {
            encoding: Some(EncodingProfile::Ascii),
            colors: Some(ColorProfile::Color16),
        },
        ..Config::default()
    };

    let shell = UiShell::from_config(&config, TerminalProfile::default());

    assert_eq!(shell.profile, TerminalProfile::ascii_16());
    assert_eq!(shell.glyphs, GlyphSet::ascii());
    assert_eq!(shell.theme.colors, ColorProfile::Color16);
    assert!(shell.display_sanitizer.ascii_only);
}

#[test]
fn status_chrome_sanitizes_terminal_control_payloads() {
    let shell = UiShell::default();
    let status = StatusBar {
        left: "Opened \x1b]0;owned\x07.log".to_string(),
        right: "Ln 1 \x1b[31mred\x1b[0m".to_string(),
        focused_window: WindowId(1),
    };

    let text = sanitized_status_text_for_width(&shell, &status, 80);

    assert_no_raw_controls(&text);
    assert!(text.contains("␛]0;owned␇"));
    assert!(text.contains("␛[31mred␛[0m"));
}

#[test]
fn window_title_sanitizes_terminal_control_payloads() {
    let mut workspace = Workspace::new_untitled();
    workspace.window_mut(WindowId(1)).unwrap().title = "evil\x1b]0;owned\x07.log".to_string();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let frame = shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 10), &[buffer_view]);

    let title = window_title_for_width(&shell, &frame.windows[0], 40);

    assert_no_raw_controls(&title);
    assert!(title.contains("evil␛]0;owned␇.log"));
}

#[test]
fn ascii_chrome_sanitization_stays_ascii() {
    let config = Config {
        terminal: TerminalOverrides {
            encoding: Some(EncodingProfile::Ascii),
            colors: Some(ColorProfile::Color16),
        },
        ..Config::default()
    };
    let shell = UiShell::from_config(&config, TerminalProfile::default());
    let status = StatusBar {
        left: "打开 \x1b[2J".to_string(),
        right: "\u{009b}31m".to_string(),
        focused_window: WindowId(1),
    };
    let text = sanitized_status_text_for_width(&shell, &status, 80);

    assert_no_raw_controls(&text);
    assert!(text.is_ascii());
    assert!(text.contains("\\u{6253}\\u{5f00} ^[[2J"));
    assert!(text.contains("<U+009B>31m"));
}

#[test]
fn ascii_renderer_keeps_menu_dialog_scrollbar_and_edges_ascii() {
    let config = Config {
        terminal: TerminalOverrides {
            encoding: Some(EncodingProfile::Ascii),
            colors: Some(ColorProfile::Color16),
        },
        ..Config::default()
    };
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(
        BufferKind::Untitled,
        "0123456789\nline2\nline3\nline4\nline5\nline6",
    );
    let buffer_view = BufferView::scrolled_xy(BufferId(1), &buffer, 2, 2);
    let shell = UiShell::from_config(&config, TerminalProfile::default());
    let mut ui_frame =
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 24, 8), &[buffer_view]);
    ui_frame.overlay = Some(
        UiOverlay::message("List", vec!["Showing 2-3 of 9".to_string()], vec![])
            .with_list(vec!["a".to_string(), "b".to_string()], Some(0), 16)
            .with_list_overflow(true, true),
    );

    let snapshot = render_frame_text(&shell, &ui_frame, 24, 10);
    assert!(snapshot.is_ascii());
    assert!(snapshot.contains('^'));
    assert!(snapshot.contains('v'));
    assert!(!snapshot.contains('↑'));
    assert!(!snapshot.contains('↓'));
    assert!(!snapshot.contains('█'));
    assert!(!snapshot.contains('‹'));
    assert!(!snapshot.contains('›'));
}

#[test]
fn surface_renderer_does_not_emit_raw_controls_from_untrusted_text() {
    let mut workspace = Workspace::new_untitled();
    workspace.window_mut(WindowId(1)).unwrap().title = "title\x1b]0;owned\x07".to_string();
    let buffer = TextBuffer::from_text_with_kind(
        BufferKind::Untitled,
        "body\x1b[31mred\x1b[0m\n\u{009b}clear",
    );
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let mut ui_frame =
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 80, 8), &[buffer_view]);
    ui_frame.status.left = "Opened \x1b]52;c;SGVsbG8=\x07".to_string();
    ui_frame.status.right = "Ln \x1b[2J".to_string();

    let rendered = render_frame_text(&shell, &ui_frame, 80, 10);
    assert_no_raw_controls(&rendered);
    assert!(rendered.contains("␛]0;owned␇"));
    assert!(rendered.contains("␛[31mred␛[0m"));
    assert!(rendered.contains("<U+009B>clear"));
    assert!(rendered.contains("␛]52;c;SGVsbG8=␇"));
}
