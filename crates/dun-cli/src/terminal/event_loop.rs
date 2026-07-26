use std::io;
use std::time::{Duration, Instant};

use dun_core::Rect;

use super::shell::PendingOsc52Read;
use super::{
    EventReader, SurfaceBackend, Terminal, TerminalColorRewrite, TerminalGuard, handle_key_event,
    handle_mouse_event, handle_runtime_action, vt::event::Event,
};
use crate::AppState;

const EVENT_POLL_SLICE: Duration = Duration::from_millis(250);

pub(crate) fn run_event_loop(
    backend: &mut SurfaceBackend,
    app: &mut AppState,
    terminal: &Terminal,
    event_reader: &mut EventReader,
    guard: &mut TerminalGuard,
    color_rewrite: &TerminalColorRewrite,
) -> io::Result<()> {
    let mut pending_osc52_read = None;

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

        if let Some(pending) = &pending_osc52_read {
            if poll_pending_osc52_read(event_reader, app, pending)? {
                pending_osc52_read = None;
            }
        } else if let Some(event) = event_reader.next_event(EVENT_POLL_SLICE)? {
            match event {
                Event::Key(event) => handle_key_event(app, event),
                Event::Paste(text) => app.handle_paste(&text),
                Event::Osc52Clipboard(_) => {}
                Event::Mouse(event) => handle_mouse_event(app, event),
                Event::Resize(_, _) => backend.clear()?,
            }
        }

        if let Some(action) = app.take_runtime_action() {
            if let Some(pending) = handle_runtime_action(action, backend, app, guard, event_reader)?
            {
                pending_osc52_read = Some(pending);
            }
        }

        app.pump_plugins();
    }

    Ok(())
}

fn poll_pending_osc52_read(
    event_reader: &mut EventReader,
    app: &mut AppState,
    pending: &PendingOsc52Read,
) -> io::Result<bool> {
    let remaining = pending.deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        event_reader.cancel_osc52_query();
        app.complete_external_paste_timeout();
        return Ok(true);
    }

    let wait = EVENT_POLL_SLICE.min(remaining);
    if let Some(text) = event_reader.next_osc52_response(wait)? {
        event_reader.cancel_osc52_query();
        app.complete_external_paste(text);
        return Ok(true);
    }

    if Instant::now() >= pending.deadline {
        event_reader.cancel_osc52_query();
        app.complete_external_paste_timeout();
        return Ok(true);
    }

    // The response-only reader disarms at each caller deadline. Re-arm for
    // the next bounded slice while preserving the one absolute 500 ms limit.
    event_reader.begin_osc52_query(pending.max_bytes);
    Ok(false)
}
