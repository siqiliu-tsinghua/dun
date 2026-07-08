use std::io;
use std::time::Duration;

use crossterm::event::{self, Event};
use dun_core::Rect;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use super::{
    TerminalColorRewrite, TerminalGuard, TerminalWriter, handle_key_event, handle_mouse_event,
    handle_runtime_action,
};
use crate::AppState;

pub(crate) fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<TerminalWriter>>,
    app: &mut AppState,
    guard: &mut TerminalGuard,
    color_rewrite: &TerminalColorRewrite,
) -> io::Result<()> {
    while !app.should_quit {
        guard.set_mouse_enabled(app.mouse_enabled())?;
        color_rewrite.set_profile(app.shell.profile);
        terminal.draw(|frame| {
            let area = frame.area();
            let workspace_area = Rect::new(0, 0, area.width, area.height.saturating_sub(2));
            app.sync_view_for_area(workspace_area);
            let buffer_views = app.buffer_views();
            let mut ui_frame = app.shell.frame_for_workspace_with_menu_selection(
                &app.workspace,
                workspace_area,
                &buffer_views,
                app.menu_selection(),
            );
            if let Some(message) = &app.status_message {
                ui_frame.status.left = message.clone();
            } else {
                ui_frame.status.left = app.focused_buffer_status();
            }
            if app.prompt.is_none()
                && app.file_dialog.is_none()
                && app.buffer_switcher.is_none()
                && app.confirm.is_none()
                && app.replace_confirm.is_none()
            {
                ui_frame.status.left = format!(
                    "{} {}",
                    app.focused_buffer_status(),
                    app.focused_detail_status()
                );
            }
            ui_frame.status.right = app.focused_file_status();
            ui_frame.overlay = app.active_overlay();
            app.shell.render(frame, &ui_frame);
        })?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(event) => handle_key_event(app, event),
                Event::Paste(text) => app.handle_paste(&text),
                Event::Mouse(event) => handle_mouse_event(app, event),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if let Some(action) = app.take_runtime_action() {
            handle_runtime_action(action, terminal, app, guard)?;
        }
    }

    Ok(())
}
