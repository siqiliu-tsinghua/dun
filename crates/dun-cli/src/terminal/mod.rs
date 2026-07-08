mod action;
mod input;
mod lifecycle;
mod sgr;
mod shell;

pub(crate) use action::RuntimeAction;
pub(crate) use input::{
    handle_key_event, handle_mouse_event, key_stroke_from_crossterm, text_input_from_crossterm,
};
pub(crate) use lifecycle::TerminalGuard;
#[cfg(test)]
pub(crate) use sgr::rewrite_16_color_sgr;
pub(crate) use sgr::{TerminalColorRewrite, TerminalWriter};
pub(crate) use shell::{
    command_run_status, duration_status_text, exit_status_text, handle_runtime_action,
    run_command_capture,
};
