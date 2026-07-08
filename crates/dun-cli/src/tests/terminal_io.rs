#![allow(unused_imports)]

use super::support::*;

#[test]
fn sgr_rewriter_converts_crossterm_ansi_palette_codes_to_legacy_codes() {
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
fn translates_ctrl_q_to_config_key_stroke() {
    let event = CrosstermKeyEvent::new(CrosstermKeyCode::Char('q'), CrosstermKeyModifiers::CONTROL);

    assert_eq!(
        key_stroke_from_crossterm(event),
        Some(KeyStroke::new(Key::Char('q'), KeyModifiers::CTRL))
    );
}

#[test]
fn translates_shifted_arrow_keys() {
    let event = CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::SHIFT);

    assert_eq!(
        key_stroke_from_crossterm(event),
        Some(KeyStroke::new(Key::Left, KeyModifiers::SHIFT))
    );
}

#[test]
fn translates_common_modified_terminal_keys() {
    assert_eq!(
        key_stroke_from_crossterm(CrosstermKeyEvent::new(
            CrosstermKeyCode::Home,
            CrosstermKeyModifiers::CONTROL,
        )),
        Some(KeyStroke::new(Key::Home, KeyModifiers::CTRL))
    );
    assert_eq!(
        key_stroke_from_crossterm(CrosstermKeyEvent::new(
            CrosstermKeyCode::End,
            CrosstermKeyModifiers::CONTROL,
        )),
        Some(KeyStroke::new(Key::End, KeyModifiers::CTRL))
    );
    assert_eq!(
        key_stroke_from_crossterm(CrosstermKeyEvent::new(
            CrosstermKeyCode::F(3),
            CrosstermKeyModifiers::SHIFT,
        )),
        Some(KeyStroke::new(Key::F(3), KeyModifiers::SHIFT))
    );
    assert_eq!(
        key_stroke_from_crossterm(CrosstermKeyEvent::new(
            CrosstermKeyCode::Left,
            CrosstermKeyModifiers::SHIFT | CrosstermKeyModifiers::CONTROL,
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
fn shell_escape_command_requests_runtime_action() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::App(AppCommand::ShellEscape));

    assert_eq!(app.take_runtime_action(), Some(RuntimeAction::ShellEscape));
    assert_eq!(app.status_message, Some("Shell escape".to_string()));
}

#[test]
fn crossterm_text_input_ignores_control_shortcuts() {
    let plain = CrosstermKeyEvent::new(CrosstermKeyCode::Char('x'), CrosstermKeyModifiers::NONE);
    let control =
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('x'), CrosstermKeyModifiers::CONTROL);

    assert_eq!(text_input_from_crossterm(plain), Some('x'));
    assert_eq!(text_input_from_crossterm(control), None);
}
