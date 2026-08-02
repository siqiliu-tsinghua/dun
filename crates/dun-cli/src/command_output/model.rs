use std::ffi::OsString;
use std::process::ExitStatus;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandRunResult {
    pub(crate) command: String,
    pub(crate) shell: OsString,
    pub(crate) status: ExitStatus,
    pub(crate) elapsed: Duration,
    pub(crate) timed_out: bool,
    pub(crate) background_processes_killed: bool,
    pub(crate) stdout: CapturedCommandStream,
    pub(crate) stderr: CapturedCommandStream,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CapturedCommandStream {
    pub(crate) bytes: Vec<u8>,
    pub(crate) truncated: bool,
}
