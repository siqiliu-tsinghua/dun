use std::io;
use std::time::Duration;

use dun_core::Rect;

use super::{
    EventReader, SurfaceBackend, Terminal, TerminalColorRewrite, TerminalGuard, handle_key_event,
    handle_mouse_event, handle_runtime_action, vt::event::Event,
};
use crate::AppState;

pub(crate) fn run_event_loop(
    backend: &mut SurfaceBackend,
    app: &mut AppState,
    terminal: &Terminal,
    event_reader: &mut EventReader,
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

        if let Some(event) = event_reader.next_event(Duration::from_millis(250))? {
            match event {
                Event::Key(event) => handle_key_event(app, event),
                Event::Paste(text) => app.handle_paste(&text),
                Event::Osc52Clipboard(_) => {}
                Event::Mouse(event) => handle_mouse_event(app, event),
                Event::Resize(_, _) => backend.clear()?,
            }
        }

        if let Some(action) = app.take_runtime_action() {
            handle_runtime_action(action, backend, app, guard)?;
        }

        app.pump_plugins();
    }

    Ok(())
}
