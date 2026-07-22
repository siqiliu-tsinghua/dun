use std::io;
use std::time::Duration;

use crossterm::event as crossterm_event;
use dun_core::Rect;

use super::{
    SurfaceBackend, Terminal, TerminalColorRewrite, TerminalGuard, handle_key_event,
    handle_mouse_event, handle_runtime_action,
    vt::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
        MouseEventKind,
    },
};
use crate::AppState;

pub(crate) fn run_event_loop(
    backend: &mut SurfaceBackend,
    app: &mut AppState,
    terminal: &Terminal,
    guard: &mut TerminalGuard,
    color_rewrite: &TerminalColorRewrite,
) -> io::Result<()> {
    while !app.should_quit {
        guard.set_mouse_enabled(app.mouse_enabled())?;
        color_rewrite.set_profile(app.shell.profile);
        let (width, height) = terminal.size()?;
        let workspace_area = Rect::new(0, 0, width, height.saturating_sub(2));
        app.sync_view_for_area(workspace_area);
        let buffer_views = app.buffer_views();
        let mut ui_frame = app.shell.frame_for_workspace_with_menu_selection(
            &app.workspace,
            workspace_area,
            &buffer_views,
            app.menu_selection(),
        );
        // A status message outranks the idle buffer readout. It used to be
        // written here and then unconditionally overwritten below, so every
        // command's feedback -- "only one buffer", "Config reloaded", "Theme
        // failed" -- was set, recorded in the history, and never shown.
        let modal_open = app.prompt.is_some()
            || app.file_dialog.is_some()
            || app.buffer_switcher.is_some()
            || app.confirm.is_some()
            || app.replace_confirm.is_some();
        ui_frame.status.left = match &app.status_message {
            Some(message) => message.clone(),
            None if modal_open => app.focused_buffer_status(),
            None => format!(
                "{} {}",
                app.focused_buffer_status(),
                app.focused_detail_status()
            ),
        };
        ui_frame.status.right = app.focused_file_status();
        ui_frame.status.plugin = app.plugin_indicator();
        ui_frame.overlay = app.active_overlay();
        backend.draw(&app.shell, &ui_frame, width, height)?;

        #[cfg(any(debug_assertions, feature = "test-panic-hook"))]
        if std::env::var_os("DUN_TEST_PANIC").is_some() {
            panic!("DUN_TEST_PANIC");
        }

        if crossterm_event::poll(Duration::from_millis(250))? {
            if let Some(event) = event_from_crossterm(crossterm_event::read()?) {
                match event {
                    Event::Key(event) => handle_key_event(app, event),
                    Event::Paste(text) => app.handle_paste(&text),
                    Event::Mouse(event) => handle_mouse_event(app, event),
                    Event::Resize(_, _) => backend.clear()?,
                }
            }
        }

        if let Some(action) = app.take_runtime_action() {
            handle_runtime_action(action, backend, app, guard)?;
        }

        app.pump_plugins();
    }

    Ok(())
}

fn event_from_crossterm(event: crossterm_event::Event) -> Option<Event> {
    match event {
        crossterm_event::Event::Key(event) => Some(Event::Key(KeyEvent::new_with_kind(
            key_code_from_crossterm(event.code),
            key_modifiers_from_crossterm(event.modifiers),
            key_event_kind_from_crossterm(event.kind),
        ))),
        crossterm_event::Event::Mouse(event) => Some(Event::Mouse(MouseEvent {
            kind: mouse_event_kind_from_crossterm(event.kind),
            column: event.column,
            row: event.row,
            modifiers: key_modifiers_from_crossterm(event.modifiers),
        })),
        crossterm_event::Event::Paste(text) => Some(Event::Paste(text)),
        crossterm_event::Event::Resize(width, height) => Some(Event::Resize(width, height)),
        crossterm_event::Event::FocusGained | crossterm_event::Event::FocusLost => None,
    }
}

fn key_code_from_crossterm(code: crossterm_event::KeyCode) -> KeyCode {
    match code {
        crossterm_event::KeyCode::Backspace => KeyCode::Backspace,
        crossterm_event::KeyCode::Enter => KeyCode::Enter,
        crossterm_event::KeyCode::Left => KeyCode::Left,
        crossterm_event::KeyCode::Right => KeyCode::Right,
        crossterm_event::KeyCode::Up => KeyCode::Up,
        crossterm_event::KeyCode::Down => KeyCode::Down,
        crossterm_event::KeyCode::Home => KeyCode::Home,
        crossterm_event::KeyCode::End => KeyCode::End,
        crossterm_event::KeyCode::PageUp => KeyCode::PageUp,
        crossterm_event::KeyCode::PageDown => KeyCode::PageDown,
        crossterm_event::KeyCode::Tab => KeyCode::Tab,
        crossterm_event::KeyCode::BackTab => KeyCode::BackTab,
        crossterm_event::KeyCode::Delete => KeyCode::Delete,
        crossterm_event::KeyCode::Insert => KeyCode::Insert,
        crossterm_event::KeyCode::F(number) => KeyCode::F(number),
        crossterm_event::KeyCode::Char(ch) => KeyCode::Char(ch),
        crossterm_event::KeyCode::Esc => KeyCode::Esc,
        crossterm_event::KeyCode::Null => KeyCode::Null,
        _ => KeyCode::Null,
    }
}

fn key_modifiers_from_crossterm(modifiers: crossterm_event::KeyModifiers) -> KeyModifiers {
    let mut owned = KeyModifiers::NONE;
    if modifiers.contains(crossterm_event::KeyModifiers::SHIFT) {
        owned |= KeyModifiers::SHIFT;
    }
    if modifiers.contains(crossterm_event::KeyModifiers::CONTROL) {
        owned |= KeyModifiers::CONTROL;
    }
    if modifiers.contains(crossterm_event::KeyModifiers::ALT) {
        owned |= KeyModifiers::ALT;
    }
    owned
}

fn key_event_kind_from_crossterm(kind: crossterm_event::KeyEventKind) -> KeyEventKind {
    match kind {
        crossterm_event::KeyEventKind::Press => KeyEventKind::Press,
        crossterm_event::KeyEventKind::Repeat => KeyEventKind::Repeat,
        crossterm_event::KeyEventKind::Release => KeyEventKind::Release,
    }
}

fn mouse_button_from_crossterm(button: crossterm_event::MouseButton) -> MouseButton {
    match button {
        crossterm_event::MouseButton::Left => MouseButton::Left,
        crossterm_event::MouseButton::Middle => MouseButton::Middle,
        crossterm_event::MouseButton::Right => MouseButton::Right,
    }
}

fn mouse_event_kind_from_crossterm(kind: crossterm_event::MouseEventKind) -> MouseEventKind {
    match kind {
        crossterm_event::MouseEventKind::Down(button) => {
            MouseEventKind::Down(mouse_button_from_crossterm(button))
        }
        crossterm_event::MouseEventKind::Up(button) => {
            MouseEventKind::Up(mouse_button_from_crossterm(button))
        }
        crossterm_event::MouseEventKind::Drag(button) => {
            MouseEventKind::Drag(mouse_button_from_crossterm(button))
        }
        crossterm_event::MouseEventKind::Moved => MouseEventKind::Moved,
        crossterm_event::MouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
        crossterm_event::MouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
        crossterm_event::MouseEventKind::ScrollLeft => MouseEventKind::ScrollLeft,
        crossterm_event::MouseEventKind::ScrollRight => MouseEventKind::ScrollRight,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_preserves_each_key_event_kind() {
        assert_eq!(
            event_from_crossterm(crossterm_event::Event::Key(
                crossterm_event::KeyEvent::new_with_kind(
                    crossterm_event::KeyCode::Enter,
                    crossterm_event::KeyModifiers::NONE,
                    crossterm_event::KeyEventKind::Press,
                ),
            )),
            Some(Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
            }))
        );
        assert_eq!(
            event_from_crossterm(crossterm_event::Event::Key(
                crossterm_event::KeyEvent::new_with_kind(
                    crossterm_event::KeyCode::Enter,
                    crossterm_event::KeyModifiers::NONE,
                    crossterm_event::KeyEventKind::Repeat,
                ),
            )),
            Some(Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Repeat,
            }))
        );
        assert_eq!(
            event_from_crossterm(crossterm_event::Event::Key(
                crossterm_event::KeyEvent::new_with_kind(
                    crossterm_event::KeyCode::Enter,
                    crossterm_event::KeyModifiers::NONE,
                    crossterm_event::KeyEventKind::Release,
                ),
            )),
            Some(Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Release,
            }))
        );
    }

    #[test]
    fn adapter_keeps_supported_modifiers_and_drops_super() {
        assert_eq!(
            event_from_crossterm(crossterm_event::Event::Key(crossterm_event::KeyEvent::new(
                crossterm_event::KeyCode::Char('x'),
                crossterm_event::KeyModifiers::SHIFT
                    | crossterm_event::KeyModifiers::CONTROL
                    | crossterm_event::KeyModifiers::ALT
                    | crossterm_event::KeyModifiers::SUPER,
            ),)),
            Some(Event::Key(KeyEvent {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT,
                kind: KeyEventKind::Press,
            }))
        );
    }

    #[test]
    fn adapter_maps_unknown_key_codes_to_null() {
        assert_eq!(
            event_from_crossterm(crossterm_event::Event::Key(crossterm_event::KeyEvent::new(
                crossterm_event::KeyCode::CapsLock,
                crossterm_event::KeyModifiers::NONE,
            ),)),
            Some(Event::Key(KeyEvent {
                code: KeyCode::Null,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
            }))
        );
    }

    #[test]
    fn adapter_maps_every_mouse_event_kind() {
        let cases = [
            (
                crossterm_event::Event::Mouse(crossterm_event::MouseEvent {
                    kind: crossterm_event::MouseEventKind::Down(crossterm_event::MouseButton::Left),
                    column: 3,
                    row: 5,
                    modifiers: crossterm_event::KeyModifiers::SHIFT,
                }),
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: 3,
                    row: 5,
                    modifiers: KeyModifiers::SHIFT,
                }),
            ),
            (
                crossterm_event::Event::Mouse(crossterm_event::MouseEvent {
                    kind: crossterm_event::MouseEventKind::Up(crossterm_event::MouseButton::Middle),
                    column: 3,
                    row: 5,
                    modifiers: crossterm_event::KeyModifiers::SHIFT,
                }),
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Up(MouseButton::Middle),
                    column: 3,
                    row: 5,
                    modifiers: KeyModifiers::SHIFT,
                }),
            ),
            (
                crossterm_event::Event::Mouse(crossterm_event::MouseEvent {
                    kind: crossterm_event::MouseEventKind::Drag(
                        crossterm_event::MouseButton::Right,
                    ),
                    column: 3,
                    row: 5,
                    modifiers: crossterm_event::KeyModifiers::SHIFT,
                }),
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Drag(MouseButton::Right),
                    column: 3,
                    row: 5,
                    modifiers: KeyModifiers::SHIFT,
                }),
            ),
            (
                crossterm_event::Event::Mouse(crossterm_event::MouseEvent {
                    kind: crossterm_event::MouseEventKind::Moved,
                    column: 3,
                    row: 5,
                    modifiers: crossterm_event::KeyModifiers::SHIFT,
                }),
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Moved,
                    column: 3,
                    row: 5,
                    modifiers: KeyModifiers::SHIFT,
                }),
            ),
            (
                crossterm_event::Event::Mouse(crossterm_event::MouseEvent {
                    kind: crossterm_event::MouseEventKind::ScrollUp,
                    column: 3,
                    row: 5,
                    modifiers: crossterm_event::KeyModifiers::SHIFT,
                }),
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollUp,
                    column: 3,
                    row: 5,
                    modifiers: KeyModifiers::SHIFT,
                }),
            ),
            (
                crossterm_event::Event::Mouse(crossterm_event::MouseEvent {
                    kind: crossterm_event::MouseEventKind::ScrollDown,
                    column: 3,
                    row: 5,
                    modifiers: crossterm_event::KeyModifiers::SHIFT,
                }),
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: 3,
                    row: 5,
                    modifiers: KeyModifiers::SHIFT,
                }),
            ),
            (
                crossterm_event::Event::Mouse(crossterm_event::MouseEvent {
                    kind: crossterm_event::MouseEventKind::ScrollLeft,
                    column: 3,
                    row: 5,
                    modifiers: crossterm_event::KeyModifiers::SHIFT,
                }),
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollLeft,
                    column: 3,
                    row: 5,
                    modifiers: KeyModifiers::SHIFT,
                }),
            ),
            (
                crossterm_event::Event::Mouse(crossterm_event::MouseEvent {
                    kind: crossterm_event::MouseEventKind::ScrollRight,
                    column: 3,
                    row: 5,
                    modifiers: crossterm_event::KeyModifiers::SHIFT,
                }),
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollRight,
                    column: 3,
                    row: 5,
                    modifiers: KeyModifiers::SHIFT,
                }),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(event_from_crossterm(input), Some(expected));
        }
    }

    #[test]
    fn adapter_maps_paste_and_resize_and_drops_focus() {
        assert_eq!(
            event_from_crossterm(crossterm_event::Event::Paste("text".to_string())),
            Some(Event::Paste("text".to_string()))
        );
        assert_eq!(
            event_from_crossterm(crossterm_event::Event::Resize(80, 24)),
            Some(Event::Resize(80, 24))
        );
        assert_eq!(
            event_from_crossterm(crossterm_event::Event::FocusGained),
            None
        );
        assert_eq!(
            event_from_crossterm(crossterm_event::Event::FocusLost),
            None
        );
    }
}
