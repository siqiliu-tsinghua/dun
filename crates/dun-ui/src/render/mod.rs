pub(crate) mod chrome;
pub(crate) mod menu;
pub(crate) mod overlay;
pub(crate) mod status;
#[allow(dead_code)]
pub(crate) mod surface_draw;
#[allow(dead_code)]
pub(crate) mod surface_frame;
#[allow(dead_code)]
pub(crate) mod surface_layers;
#[allow(dead_code)]
pub(crate) mod surface_window;
pub(crate) mod window;

use ratatui::layout::Rect as TuiRect;
use ratatui::prelude::Frame;

use crate::render::chrome::render_background;
use crate::render::menu::{render_active_menu, render_menu};
use crate::render::overlay::render_overlay;
use crate::render::status::render_status;
use crate::render::window::render_window;
use crate::{UiFrame, UiShell};

pub fn render_ui_frame(frame: &mut Frame<'_>, shell: &UiShell, ui_frame: &UiFrame) {
    let area = frame.area();
    render_background(frame, area, shell.theme.palette.editor);

    if area.height == 0 || area.width == 0 {
        return;
    }

    let menu_area = TuiRect::new(area.x, area.y, area.width, 1);
    render_menu(frame, shell, &ui_frame.menu, menu_area);

    if area.height == 1 {
        return;
    }

    let status_area = TuiRect::new(area.x, area.y + area.height - 1, area.width, 1);
    render_status(frame, shell, &ui_frame.status, status_area);

    if area.height <= 2 {
        return;
    }

    let workspace_area = TuiRect::new(area.x, area.y + 1, area.width, area.height - 2);
    for window in &ui_frame.windows {
        render_window(frame, shell, window, workspace_area);
    }

    render_active_menu(frame.buffer_mut(), shell, &ui_frame.menu, area);
    if let Some(overlay) = &ui_frame.overlay {
        render_overlay(frame, shell, overlay, area);
    }
}
