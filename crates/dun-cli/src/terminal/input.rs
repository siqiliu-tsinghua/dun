use crate::*;

pub(crate) fn handle_mouse_event(app: &mut AppState, event: CrosstermMouseEvent) {
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
            CrosstermMouseEventKind::Down(CrosstermMouseButton::Left) => {
                app.handle_buffer_switcher_mouse_down(event.column, event.row);
            }
            CrosstermMouseEventKind::Down(CrosstermMouseButton::Right) => {
                app.note_right_click_paste();
            }
            CrosstermMouseEventKind::ScrollUp => {
                app.scroll_buffer_switcher(-1);
            }
            CrosstermMouseEventKind::ScrollDown => {
                app.scroll_buffer_switcher(1);
            }
            CrosstermMouseEventKind::ScrollLeft | CrosstermMouseEventKind::ScrollRight => {}
            CrosstermMouseEventKind::Up(CrosstermMouseButton::Left) => {
                app.handle_mouse_up();
            }
            _ => {}
        }
        return;
    }

    if app.file_dialog.is_some() {
        match event.kind {
            CrosstermMouseEventKind::Down(CrosstermMouseButton::Left) => {
                app.handle_file_dialog_mouse_down(event.column, event.row);
            }
            CrosstermMouseEventKind::Down(CrosstermMouseButton::Right) => {
                app.note_right_click_paste();
            }
            CrosstermMouseEventKind::ScrollUp => {
                app.scroll_file_dialog(-1);
            }
            CrosstermMouseEventKind::ScrollDown => {
                app.scroll_file_dialog(1);
            }
            CrosstermMouseEventKind::ScrollLeft | CrosstermMouseEventKind::ScrollRight => {}
            CrosstermMouseEventKind::Up(CrosstermMouseButton::Left) => {
                app.handle_mouse_up();
            }
            _ => {}
        }
        return;
    }

    match event.kind {
        CrosstermMouseEventKind::Down(CrosstermMouseButton::Left) => {
            app.handle_mouse_down(event.column, event.row);
        }
        CrosstermMouseEventKind::Down(CrosstermMouseButton::Right) => {
            app.note_right_click_paste();
        }
        CrosstermMouseEventKind::Drag(CrosstermMouseButton::Left) => {
            app.handle_mouse_drag(event.column, event.row);
        }
        CrosstermMouseEventKind::ScrollUp => {
            app.handle_mouse_scroll(
                event.column,
                event.row,
                -(EDITOR_MOUSE_WHEEL_LINES as isize),
            );
        }
        CrosstermMouseEventKind::ScrollDown => {
            app.handle_mouse_scroll(event.column, event.row, EDITOR_MOUSE_WHEEL_LINES as isize);
        }
        CrosstermMouseEventKind::ScrollLeft => {
            app.scroll_focused_columns(-1);
        }
        CrosstermMouseEventKind::ScrollRight => {
            app.scroll_focused_columns(1);
        }
        CrosstermMouseEventKind::Up(CrosstermMouseButton::Left) => {
            app.handle_mouse_up();
        }
        _ => {}
    }
}

pub(crate) fn handle_key_event(app: &mut AppState, event: CrosstermKeyEvent) {
    if matches!(event.kind, CrosstermKeyEventKind::Release) {
        return;
    }

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

    let Some(stroke) = key_stroke_from_crossterm(event) else {
        return;
    };

    if app.handle_auxiliary_enter_key_stroke(stroke) {
        return;
    }

    if app.handle_key_stroke(stroke) {
        return;
    }

    if app.handle_selection_key_stroke(stroke) {
        return;
    }

    if app.handle_auxiliary_window_key_stroke(stroke) {
        return;
    }

    if handle_menu_mnemonic_key_event(app, event) {
        return;
    }

    if let Some(ch) = text_input_from_crossterm(event) {
        app.handle_text_input(ch);
    }
}

fn handle_active_menu_key_event(app: &mut AppState, event: CrosstermKeyEvent) {
    match event.code {
        CrosstermKeyCode::Esc => app.clear_active_menu(),
        CrosstermKeyCode::Left => {
            app.move_active_menu(-1);
        }
        CrosstermKeyCode::Right => {
            app.move_active_menu(1);
        }
        CrosstermKeyCode::Up => {
            app.move_active_menu_entry(-1);
        }
        CrosstermKeyCode::Down => {
            app.move_active_menu_entry(1);
        }
        CrosstermKeyCode::Enter => {
            app.dispatch_active_menu_entry();
        }
        CrosstermKeyCode::Char(ch) if event.modifiers.contains(CrosstermKeyModifiers::ALT) => {
            if let Some(menu_index) = app.shell.menu_index_for_mnemonic(ch) {
                app.open_keyboard_menu(menu_index);
            }
        }
        _ => {}
    }
}

fn handle_menu_mnemonic_key_event(app: &mut AppState, event: CrosstermKeyEvent) -> bool {
    if !event.modifiers.contains(CrosstermKeyModifiers::ALT)
        || event.modifiers.contains(CrosstermKeyModifiers::CONTROL)
    {
        return false;
    }
    let CrosstermKeyCode::Char(ch) = event.code else {
        return false;
    };
    let Some(menu_index) = app.shell.menu_index_for_mnemonic(ch) else {
        return false;
    };

    app.pending_keys.clear();
    app.open_keyboard_menu(menu_index);
    true
}

pub(crate) fn key_stroke_from_crossterm(event: CrosstermKeyEvent) -> Option<KeyStroke> {
    let modifiers = key_modifiers_from_crossterm(event.modifiers);
    let key = match event.code {
        CrosstermKeyCode::Backspace => Key::Backspace,
        CrosstermKeyCode::Enter => Key::Enter,
        CrosstermKeyCode::Left => Key::Left,
        CrosstermKeyCode::Right => Key::Right,
        CrosstermKeyCode::Up => Key::Up,
        CrosstermKeyCode::Down => Key::Down,
        CrosstermKeyCode::Home => Key::Home,
        CrosstermKeyCode::End => Key::End,
        CrosstermKeyCode::PageUp => Key::PageUp,
        CrosstermKeyCode::PageDown => Key::PageDown,
        CrosstermKeyCode::Tab => Key::Tab,
        CrosstermKeyCode::BackTab => Key::BackTab,
        CrosstermKeyCode::Delete => Key::Delete,
        CrosstermKeyCode::Insert => Key::Insert,
        CrosstermKeyCode::F(number) => Key::F(number),
        CrosstermKeyCode::Char(ch) => Key::Char(normalize_event_char(ch, modifiers)),
        CrosstermKeyCode::Esc => Key::Esc,
        _ => return None,
    };

    Some(KeyStroke::new(key, modifiers))
}

fn key_modifiers_from_crossterm(modifiers: CrosstermKeyModifiers) -> KeyModifiers {
    KeyModifiers {
        shift: modifiers.contains(CrosstermKeyModifiers::SHIFT),
        ctrl: modifiers.contains(CrosstermKeyModifiers::CONTROL),
        alt: modifiers.contains(CrosstermKeyModifiers::ALT),
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

pub(crate) fn text_input_from_crossterm(event: CrosstermKeyEvent) -> Option<char> {
    let modifiers = key_modifiers_from_crossterm(event.modifiers);
    if modifiers.ctrl || modifiers.alt {
        return None;
    }

    match event.code {
        CrosstermKeyCode::Char(ch) if !ch.is_control() => Some(ch),
        _ => None,
    }
}
