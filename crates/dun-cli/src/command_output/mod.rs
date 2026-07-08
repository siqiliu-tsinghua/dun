mod format;
mod model;

pub(crate) const COMMAND_OUTPUT_STREAM_SOFT_LIMIT_BYTES: usize = 512 * 1024;

pub(crate) use format::{command_output_buffer, command_output_empty_buffer, command_output_text};
pub(crate) use model::{CapturedCommandStream, CommandOutputSection, CommandRunResult};
