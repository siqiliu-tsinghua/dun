mod action;
mod clipboard;
mod event_loop;
mod input;
mod lifecycle;
mod profile;
mod sgr;
mod shell;

pub(crate) use action::RuntimeAction;
pub(crate) use clipboard::osc52_copy_sequence;
pub(crate) use event_loop::run_event_loop;
pub(crate) use input::{
    handle_key_event, handle_mouse_event, key_stroke_from_crossterm, text_input_from_crossterm,
};
pub(crate) use lifecycle::{TerminalGuard, install_panic_terminal_restore};
pub(crate) use profile::detect_terminal_profile;
#[cfg(test)]
pub(crate) use sgr::rewrite_16_color_sgr;
pub(crate) use sgr::{TerminalColorRewrite, TerminalWriter};
pub(crate) use shell::{
    command_run_status, duration_status_text, exit_status_text, handle_runtime_action,
    run_command_capture,
};
