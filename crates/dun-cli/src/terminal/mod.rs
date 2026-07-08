mod action;
mod lifecycle;
mod sgr;

pub(crate) use action::RuntimeAction;
pub(crate) use lifecycle::TerminalGuard;
#[cfg(test)]
pub(crate) use sgr::rewrite_16_color_sgr;
pub(crate) use sgr::{TerminalColorRewrite, TerminalWriter};
