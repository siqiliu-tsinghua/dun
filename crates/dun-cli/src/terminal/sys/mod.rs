#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub(crate) use unix::{Readiness, Terminal};
