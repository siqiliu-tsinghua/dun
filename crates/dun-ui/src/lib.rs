#![forbid(unsafe_code)]

mod frame;
mod hit;
mod model;
mod render;
mod shell;
mod text;

pub use model::{
    BufferView, MenuBar, MenuEntry, MenuItem, MenuSelection, StatusBar, UiCursor, UiFrame,
    UiGutterLine, UiHorizontalEdgeLine, UiMouseHit, UiMouseTarget, UiOverlay, UiScrollbar,
    UiSearchMatchLine, UiSelectionLine, UiWindow,
};
#[cfg(test)]
pub(crate) use render::chrome::{vertical_overflow_down, vertical_overflow_up};
pub(crate) use render::menu::{
    clamp_menu_rect, dropdown_rect_for_menu, menu_item_column_range, menu_visible_entry_range,
};
pub(crate) use render::overlay::overlay_layout;
pub use render::render_ui_frame;
#[cfg(test)]
pub(crate) use render::status::sanitized_status_text_for_width;
#[cfg(test)]
pub(crate) use render::window::window_title_for_width;
pub use shell::UiShell;
pub(crate) use text::{
    buffer_end_position, decimal_digits, display_width, fit_text_to_width, status_text_for_width,
    visible_whitespace_prefix_text, visible_whitespace_text, wrap_line_segments,
};

#[cfg(test)]
mod tests;
