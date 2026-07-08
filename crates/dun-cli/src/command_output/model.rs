use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandOutputSection {
    Stdout,
    Stderr,
}

impl CommandOutputSection {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }

    pub(crate) const fn view_title(self) -> &'static str {
        match self {
            Self::Stdout => "Command Output Stdout",
            Self::Stderr => "Command Output Stderr",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandRunResult {
    pub(crate) command: String,
    pub(crate) shell: OsString,
    pub(crate) status: ExitStatus,
    pub(crate) elapsed: Duration,
    pub(crate) stdout: CapturedCommandStream,
    pub(crate) stderr: CapturedCommandStream,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CapturedCommandStream {
    pub(crate) bytes: Vec<u8>,
    pub(crate) truncated: bool,
}
