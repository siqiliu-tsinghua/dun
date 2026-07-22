#![allow(unused_imports)]

use super::support::*;

#[test]
fn sgr_rewriter_converts_ansi_palette_codes_to_legacy_codes() {
    let mut pending = Vec::new();
    let output = rewrite_16_color_sgr(b"\x1b[38;5;7;48;5;4mX\x1b[38;5;15;48;5;8mY", &mut pending);

    assert_eq!(
        String::from_utf8(output).unwrap(),
        "\x1b[37;44mX\x1b[97;100mY"
    );
    assert!(pending.is_empty());
}

#[test]
fn sgr_rewriter_preserves_split_sequences_until_complete() {
    let mut pending = Vec::new();

    assert_eq!(rewrite_16_color_sgr(b"\x1b[38;5", &mut pending), b"");
    assert_eq!(
        String::from_utf8(rewrite_16_color_sgr(b";11m!", &mut pending)).unwrap(),
        "\x1b[93m!"
    );
    assert!(pending.is_empty());
}

#[test]
fn sgr_rewriter_leaves_non_sgr_csi_sequences_unchanged() {
    let mut pending = Vec::new();
    let output = rewrite_16_color_sgr(b"\x1b[?25l\x1b[2;3H", &mut pending);

    assert_eq!(String::from_utf8(output).unwrap(), "\x1b[?25l\x1b[2;3H");
    assert!(pending.is_empty());
}

#[test]
fn sgr_rewriter_preserves_invalid_non_csi_and_oversized_pending_sequences() {
    let mut pending = Vec::new();

    assert_eq!(
        String::from_utf8(rewrite_16_color_sgr(b"\x1b[38;5;xm", &mut pending)).unwrap(),
        "\x1b[38;5;xm"
    );
    assert_eq!(
        String::from_utf8(rewrite_16_color_sgr(b"\x1b[m", &mut pending)).unwrap(),
        "\x1b[0m"
    );
    assert_eq!(
        String::from_utf8(rewrite_16_color_sgr(b"\x1b[;38;5;2m", &mut pending)).unwrap(),
        "\x1b[0;32m"
    );
    assert_eq!(
        String::from_utf8(rewrite_16_color_sgr(b"\x1b[38;5;16m", &mut pending)).unwrap(),
        "\x1b[38;5;16m"
    );
    assert_eq!(
        String::from_utf8(rewrite_16_color_sgr(b"\x1b]0;title\x07", &mut pending)).unwrap(),
        "\x1b]0;title\x07"
    );

    let oversized = {
        let mut input = b"\x1b[".to_vec();
        input.extend(std::iter::repeat(b'1').take(1025));
        input
    };
    let output = rewrite_16_color_sgr(&oversized, &mut pending);
    assert_eq!(output, oversized);
    assert!(pending.is_empty());
}

#[test]
fn terminal_color_rewrite_tracks_color_profile() {
    let rewrite = TerminalColorRewrite::new(TerminalProfile::utf8_16());

    assert!(rewrite.is_enabled());
    rewrite.set_profile(TerminalProfile::utf8_256());
    assert!(!rewrite.is_enabled());
    rewrite.set_profile(TerminalProfile::ascii_mono());
    assert!(!rewrite.is_enabled());
    rewrite.set_profile(TerminalProfile::ascii_16());
    assert!(rewrite.is_enabled());
}

#[test]
fn translates_ctrl_q_to_config_key_stroke() {
    let event = TerminalKeyEvent::new(TerminalKeyCode::Char('q'), TerminalKeyModifiers::CONTROL);

    assert_eq!(
        key_stroke_from_event(event),
        Some(KeyStroke::new(Key::Char('q'), KeyModifiers::CTRL))
    );
}

#[test]
fn translates_shifted_arrow_keys() {
    let event = TerminalKeyEvent::new(TerminalKeyCode::Left, TerminalKeyModifiers::SHIFT);

    assert_eq!(
        key_stroke_from_event(event),
        Some(KeyStroke::new(Key::Left, KeyModifiers::SHIFT))
    );
}

#[test]
fn translates_common_modified_terminal_keys() {
    assert_eq!(
        key_stroke_from_event(TerminalKeyEvent::new(
            TerminalKeyCode::Home,
            TerminalKeyModifiers::CONTROL,
        )),
        Some(KeyStroke::new(Key::Home, KeyModifiers::CTRL))
    );
    assert_eq!(
        key_stroke_from_event(TerminalKeyEvent::new(
            TerminalKeyCode::End,
            TerminalKeyModifiers::CONTROL,
        )),
        Some(KeyStroke::new(Key::End, KeyModifiers::CTRL))
    );
    assert_eq!(
        key_stroke_from_event(TerminalKeyEvent::new(
            TerminalKeyCode::F(3),
            TerminalKeyModifiers::SHIFT,
        )),
        Some(KeyStroke::new(Key::F(3), KeyModifiers::SHIFT))
    );
    assert_eq!(
        key_stroke_from_event(TerminalKeyEvent::new(
            TerminalKeyCode::Left,
            TerminalKeyModifiers::SHIFT | TerminalKeyModifiers::CONTROL,
        )),
        Some(KeyStroke::new(
            Key::Left,
            KeyModifiers {
                shift: true,
                ctrl: true,
                alt: false,
            },
        ))
    );
}

#[test]
fn translates_common_unmodified_terminal_keys() {
    let cases = [
        (TerminalKeyCode::Backspace, Key::Backspace),
        (TerminalKeyCode::Enter, Key::Enter),
        (TerminalKeyCode::Right, Key::Right),
        (TerminalKeyCode::Up, Key::Up),
        (TerminalKeyCode::Down, Key::Down),
        (TerminalKeyCode::PageUp, Key::PageUp),
        (TerminalKeyCode::PageDown, Key::PageDown),
        (TerminalKeyCode::Tab, Key::Tab),
        (TerminalKeyCode::BackTab, Key::BackTab),
        (TerminalKeyCode::Delete, Key::Delete),
        (TerminalKeyCode::Insert, Key::Insert),
        (TerminalKeyCode::Esc, Key::Esc),
    ];

    for (code, expected_key) in cases {
        assert_eq!(
            key_stroke_from_event(TerminalKeyEvent::new(code, TerminalKeyModifiers::NONE)),
            Some(KeyStroke::new(expected_key, KeyModifiers::NONE))
        );
    }

    assert_eq!(
        key_stroke_from_event(TerminalKeyEvent::new(
            TerminalKeyCode::Null,
            TerminalKeyModifiers::NONE,
        )),
        None
    );
}

#[test]
fn shifted_ascii_key_strokes_are_normalized_by_case() {
    assert_eq!(
        key_stroke_from_event(TerminalKeyEvent::new(
            TerminalKeyCode::Char('a'),
            TerminalKeyModifiers::SHIFT,
        )),
        Some(KeyStroke::new(Key::Char('A'), KeyModifiers::SHIFT))
    );
    assert_eq!(
        key_stroke_from_event(TerminalKeyEvent::new(
            TerminalKeyCode::Char('A'),
            TerminalKeyModifiers::NONE,
        )),
        Some(KeyStroke::new(Key::Char('a'), KeyModifiers::NONE))
    );
}

#[test]
fn shell_escape_command_requests_runtime_action() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::ShellEscape));

    assert_eq!(app.take_runtime_action(), Some(RuntimeAction::ShellEscape));
    assert_eq!(app.status_message, Some("Shell escape".to_string()));
}

#[test]
fn terminal_text_input_ignores_control_shortcuts() {
    let plain = TerminalKeyEvent::new(TerminalKeyCode::Char('x'), TerminalKeyModifiers::NONE);
    let control = TerminalKeyEvent::new(TerminalKeyCode::Char('x'), TerminalKeyModifiers::CONTROL);
    let alt = TerminalKeyEvent::new(TerminalKeyCode::Char('x'), TerminalKeyModifiers::ALT);
    let control_character =
        TerminalKeyEvent::new(TerminalKeyCode::Char('\n'), TerminalKeyModifiers::NONE);

    assert_eq!(text_input_from_event(plain), Some('x'));
    assert_eq!(text_input_from_event(control), None);
    assert_eq!(text_input_from_event(alt), None);
    assert_eq!(text_input_from_event(control_character), None);
}

#[test]
fn key_release_events_are_ignored_before_text_input() {
    let mut app = AppState::new();
    let release = TerminalKeyEvent::new_with_kind(
        TerminalKeyCode::Char('x'),
        TerminalKeyModifiers::NONE,
        TerminalKeyEventKind::Release,
    );

    handle_key_event(&mut app, release);

    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");
}

#[test]
fn keyboard_menu_navigation_uses_terminal_key_events() {
    let mut app = AppState::new();
    let file_menu = app.shell.menu_index_for_mnemonic('f').unwrap();
    let edit_menu = app.shell.menu_index_for_mnemonic('e').unwrap();

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('f'), TerminalKeyModifiers::ALT),
    );
    assert_eq!(app.active_menu, Some(file_menu));
    assert_eq!(app.active_menu_entry, Some(0));

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Right, TerminalKeyModifiers::NONE),
    );
    assert_ne!(app.active_menu, Some(file_menu));

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('e'), TerminalKeyModifiers::ALT),
    );
    assert_eq!(app.active_menu, Some(edit_menu));

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Down, TerminalKeyModifiers::NONE),
    );
    assert_eq!(app.active_menu_entry, Some(1));

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Up, TerminalKeyModifiers::NONE),
    );
    assert_eq!(app.active_menu_entry, Some(0));

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Esc, TerminalKeyModifiers::NONE),
    );
    assert_eq!(app.active_menu, None);
}

#[test]
fn keyboard_menu_enter_dispatches_selected_entry() {
    let mut app = AppState::new();
    let help_menu = app.shell.menu_index_for_mnemonic('h').unwrap();
    app.open_keyboard_menu(help_menu);

    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );

    assert_eq!(app.active_menu, None);
    assert_eq!(
        app.workspace.focused_window().unwrap().kind,
        WindowKind::Help
    );
}
