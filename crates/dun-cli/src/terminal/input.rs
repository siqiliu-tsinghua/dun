use crate::*;

pub(crate) fn handle_mouse_event(app: &mut AppState, event: TerminalMouseEvent) {
    if !app.mouse_enabled()
        || app.prompt.is_some()
        || app.confirm.is_some()
        || app.replace_confirm.is_some()
    {
        app.handle_mouse_up();
        return;
    }

    if app.buffer_switcher.is_some() {
        match event.kind {
            TerminalMouseEventKind::Down(TerminalMouseButton::Left) => {
                app.handle_buffer_switcher_mouse_down(event.column, event.row);
            }
            TerminalMouseEventKind::Down(TerminalMouseButton::Right) => {
                app.note_right_click_paste();
            }
            TerminalMouseEventKind::ScrollUp => {
                app.scroll_buffer_switcher(-1);
            }
            TerminalMouseEventKind::ScrollDown => {
                app.scroll_buffer_switcher(1);
            }
            TerminalMouseEventKind::ScrollLeft | TerminalMouseEventKind::ScrollRight => {}
            TerminalMouseEventKind::Up(TerminalMouseButton::Left) => {
                app.handle_mouse_up();
            }
            _ => {}
        }
        return;
    }

    if app.file_dialog.is_some() {
        match event.kind {
            TerminalMouseEventKind::Down(TerminalMouseButton::Left) => {
                app.handle_file_dialog_mouse_down(event.column, event.row);
            }
            TerminalMouseEventKind::Down(TerminalMouseButton::Right) => {
                app.note_right_click_paste();
            }
            TerminalMouseEventKind::ScrollUp => {
                app.scroll_file_dialog(-1);
            }
            TerminalMouseEventKind::ScrollDown => {
                app.scroll_file_dialog(1);
            }
            TerminalMouseEventKind::ScrollLeft | TerminalMouseEventKind::ScrollRight => {}
            TerminalMouseEventKind::Up(TerminalMouseButton::Left) => {
                app.handle_mouse_up();
            }
            _ => {}
        }
        return;
    }

    match event.kind {
        TerminalMouseEventKind::Down(TerminalMouseButton::Left) => {
            app.handle_mouse_down(event.column, event.row);
        }
        TerminalMouseEventKind::Down(TerminalMouseButton::Right) => {
            app.note_right_click_paste();
        }
        TerminalMouseEventKind::Drag(TerminalMouseButton::Left) => {
            app.handle_mouse_drag(event.column, event.row);
        }
        TerminalMouseEventKind::ScrollUp => {
            app.handle_mouse_scroll(
                event.column,
                event.row,
                -(EDITOR_MOUSE_WHEEL_LINES as isize),
            );
        }
        TerminalMouseEventKind::ScrollDown => {
            app.handle_mouse_scroll(event.column, event.row, EDITOR_MOUSE_WHEEL_LINES as isize);
        }
        TerminalMouseEventKind::ScrollLeft => {
            app.scroll_focused_columns(-1);
        }
        TerminalMouseEventKind::ScrollRight => {
            app.scroll_focused_columns(1);
        }
        TerminalMouseEventKind::Up(TerminalMouseButton::Left) => {
            app.handle_mouse_up();
        }
        _ => {}
    }
}

pub(crate) fn handle_key_event(app: &mut AppState, event: TerminalKeyEvent) {
    if matches!(event.kind, TerminalKeyEventKind::Release) {
        return;
    }

    // A status message lives until the next keypress: the frame drawn right
    // after the command that set it shows it, and then the user's next key
    // hands the status line back to the buffer readout. Without an expiry the
    // message would pin the status line permanently.
    app.status_message = None;

    if app.active_menu.is_some() {
        handle_active_menu_key_event(app, event);
        return;
    }

    if app.handle_confirm_key_event(event) {
        return;
    }

    if app.handle_replace_confirm_key_event(event) {
        return;
    }

    if app.handle_buffer_switcher_key_event(event) {
        return;
    }

    if app.handle_file_dialog_key_event(event) {
        return;
    }

    if app.handle_prompt_key_event(event) {
        return;
    }

    let Some(stroke) = key_stroke_from_event(event) else {
        return;
    };

    if app.handle_auxiliary_enter_key_stroke(stroke) {
        return;
    }

    if app.handle_auxiliary_window_key_stroke(stroke) {
        return;
    }

    if app.handle_key_stroke(stroke) {
        return;
    }

    if app.handle_selection_key_stroke(stroke) {
        return;
    }

    if handle_menu_mnemonic_key_event(app, event) {
        return;
    }

    if let Some(ch) = text_input_from_event(event) {
        app.handle_text_input(ch);
    }
}

fn handle_active_menu_key_event(app: &mut AppState, event: TerminalKeyEvent) {
    match event.code {
        TerminalKeyCode::Esc => app.clear_active_menu(),
        TerminalKeyCode::Left => {
            app.move_active_menu(-1);
        }
        TerminalKeyCode::Right => {
            app.move_active_menu(1);
        }
        TerminalKeyCode::Up => {
            app.move_active_menu_entry(-1);
        }
        TerminalKeyCode::Down => {
            app.move_active_menu_entry(1);
        }
        TerminalKeyCode::Enter => {
            app.dispatch_active_menu_entry();
        }
        TerminalKeyCode::Char(ch) if event.modifiers.contains(TerminalKeyModifiers::ALT) => {
            if let Some(menu_index) = app.shell.menu_index_for_mnemonic(ch) {
                app.open_keyboard_menu(menu_index);
            }
        }
        // A bare letter runs the entry that advertises it in its label
        // ("Open... (O)"), the way every other menu-driven editor behaves.
        TerminalKeyCode::Char(ch) if !event.modifiers.contains(TerminalKeyModifiers::CONTROL) => {
            app.dispatch_active_menu_mnemonic(ch);
        }
        _ => {}
    }
}

fn handle_menu_mnemonic_key_event(app: &mut AppState, event: TerminalKeyEvent) -> bool {
    if !event.modifiers.contains(TerminalKeyModifiers::ALT)
        || event.modifiers.contains(TerminalKeyModifiers::CONTROL)
    {
        return false;
    }
    let TerminalKeyCode::Char(ch) = event.code else {
        return false;
    };
    let Some(menu_index) = app.shell.menu_index_for_mnemonic(ch) else {
        return false;
    };

    app.pending_keys.clear();
    app.open_keyboard_menu(menu_index);
    true
}

pub(crate) fn key_stroke_from_event(event: TerminalKeyEvent) -> Option<KeyStroke> {
    let modifiers = key_modifiers_from_event(event.modifiers);
    let key = match event.code {
        TerminalKeyCode::Backspace => Key::Backspace,
        TerminalKeyCode::Enter => Key::Enter,
        TerminalKeyCode::Left => Key::Left,
        TerminalKeyCode::Right => Key::Right,
        TerminalKeyCode::Up => Key::Up,
        TerminalKeyCode::Down => Key::Down,
        TerminalKeyCode::Home => Key::Home,
        TerminalKeyCode::End => Key::End,
        TerminalKeyCode::PageUp => Key::PageUp,
        TerminalKeyCode::PageDown => Key::PageDown,
        TerminalKeyCode::Tab => Key::Tab,
        TerminalKeyCode::BackTab => Key::BackTab,
        TerminalKeyCode::Delete => Key::Delete,
        TerminalKeyCode::Insert => Key::Insert,
        TerminalKeyCode::F(number) => Key::F(number),
        TerminalKeyCode::Char(ch) => Key::Char(normalize_event_char(ch, modifiers)),
        TerminalKeyCode::Esc => Key::Esc,
        _ => return None,
    };

    Some(KeyStroke::new(key, modifiers))
}

fn key_modifiers_from_event(modifiers: TerminalKeyModifiers) -> KeyModifiers {
    KeyModifiers {
        shift: modifiers.contains(TerminalKeyModifiers::SHIFT),
        ctrl: modifiers.contains(TerminalKeyModifiers::CONTROL),
        alt: modifiers.contains(TerminalKeyModifiers::ALT),
    }
}

fn normalize_event_char(ch: char, modifiers: KeyModifiers) -> char {
    if ch.is_ascii_alphabetic() && modifiers.shift {
        ch.to_ascii_uppercase()
    } else if ch.is_ascii_alphabetic() {
        ch.to_ascii_lowercase()
    } else {
        ch
    }
}

pub(crate) fn text_input_from_event(event: TerminalKeyEvent) -> Option<char> {
    let modifiers = key_modifiers_from_event(event.modifiers);
    if modifiers.ctrl || modifiers.alt {
        return None;
    }

    match event.code {
        TerminalKeyCode::Char(ch) if !ch.is_control() => Some(ch),
        _ => None,
    }
}
