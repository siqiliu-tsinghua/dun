use super::TextKey;

// Command Output buffer content.
pub(crate) const COMMAND_OUTPUT_TITLE: TextKey = ("command-output.title", "Dun Command Output");
pub(crate) const COMMAND_OUTPUT_COMMAND: TextKey = ("command-output.command", "Command: {}");
pub(crate) const COMMAND_OUTPUT_SHELL: TextKey = ("command-output.shell", "Shell: {}");
pub(crate) const COMMAND_OUTPUT_STATUS: TextKey = ("command-output.status", "Status: {}");
pub(crate) const COMMAND_OUTPUT_STATUS_TIMED_OUT: TextKey = (
    "command-output.status.timed-out",
    "timed out; process killed",
);
pub(crate) const COMMAND_OUTPUT_ELAPSED: TextKey = ("command-output.elapsed", "Elapsed: {}");
pub(crate) const COMMAND_OUTPUT_LIMIT: TextKey =
    ("command-output.limit", "Limit: {} bytes per stream");
pub(crate) const COMMAND_OUTPUT_STDOUT: TextKey = ("command-output.stdout", "Stdout: {}");
pub(crate) const COMMAND_OUTPUT_STDOUT_LINES: TextKey =
    ("command-output.stdout-lines", "Stdout Lines: {}");
pub(crate) const COMMAND_OUTPUT_STDERR: TextKey = ("command-output.stderr", "Stderr: {}");
pub(crate) const COMMAND_OUTPUT_STDERR_LINES: TextKey =
    ("command-output.stderr-lines", "Stderr Lines: {}");
pub(crate) const COMMAND_OUTPUT_TRUNCATED: TextKey = ("command-output.truncated", "Truncated: {}");
pub(crate) const COMMAND_OUTPUT_YES: TextKey = ("command-output.yes", "yes");
pub(crate) const COMMAND_OUTPUT_NO: TextKey = ("command-output.no", "no");
pub(crate) const COMMAND_OUTPUT_STREAM_COMPLETE: TextKey =
    ("command-output.stream.complete", "{} bytes, complete");
pub(crate) const COMMAND_OUTPUT_STREAM_TRUNCATED: TextKey =
    ("command-output.stream.truncated", "{} bytes, truncated");
pub(crate) const COMMAND_OUTPUT_STDOUT_SECTION: TextKey =
    ("command-output.stdout-section", "--- stdout ({}) ---");
pub(crate) const COMMAND_OUTPUT_STDERR_SECTION: TextKey =
    ("command-output.stderr-section", "--- stderr ({}) ---");
pub(crate) const COMMAND_OUTPUT_EMPTY: TextKey = ("command-output.empty", "(empty)");
pub(crate) const COMMAND_OUTPUT_TRUNCATED_MARKER: TextKey =
    ("command-output.truncated-marker", "[truncated]");

#[cfg(test)]
pub(crate) const ALL: &[TextKey] = &[
    COMMAND_OUTPUT_TITLE,
    COMMAND_OUTPUT_COMMAND,
    COMMAND_OUTPUT_SHELL,
    COMMAND_OUTPUT_STATUS,
    COMMAND_OUTPUT_STATUS_TIMED_OUT,
    COMMAND_OUTPUT_ELAPSED,
    COMMAND_OUTPUT_LIMIT,
    COMMAND_OUTPUT_STDOUT,
    COMMAND_OUTPUT_STDOUT_LINES,
    COMMAND_OUTPUT_STDERR,
    COMMAND_OUTPUT_STDERR_LINES,
    COMMAND_OUTPUT_TRUNCATED,
    COMMAND_OUTPUT_YES,
    COMMAND_OUTPUT_NO,
    COMMAND_OUTPUT_STREAM_COMPLETE,
    COMMAND_OUTPUT_STREAM_TRUNCATED,
    COMMAND_OUTPUT_STDOUT_SECTION,
    COMMAND_OUTPUT_STDERR_SECTION,
    COMMAND_OUTPUT_EMPTY,
    COMMAND_OUTPUT_TRUNCATED_MARKER,
];
