use super::TextKey;

// Command-line, configuration, plugin, and command-run status messages.
pub(crate) const STATUS_COMMAND_CANCELLED: TextKey =
    ("status.command.cancelled", "Command cancelled");
pub(crate) const STATUS_COMMAND_UNKNOWN: TextKey =
    ("status.command.unknown", "Unknown command: {}");
pub(crate) const STATUS_COMMAND_PARSE_FAILED: TextKey =
    ("status.command.parse-failed", "Command failed: {}");
pub(crate) const STATUS_COMMAND_PARSE_TRAILING_ESCAPE: TextKey =
    ("status.command.parse.trailing-escape", "trailing escape");
pub(crate) const STATUS_COMMAND_PARSE_UNCLOSED_QUOTE: TextKey =
    ("status.command.parse.unclosed-quote", "unclosed quote");
pub(crate) const STATUS_COMMAND_LINE_HELP: TextKey = (
    "status.command-line.help",
    "Commands: {}, or any command id such as {}",
);
pub(crate) const STATUS_COMMAND_THEME_ARITY: TextKey = (
    "status.command.theme-arity",
    "Command failed: theme expects zero or one theme name",
);
pub(crate) const STATUS_COMMAND_RUN_ARITY: TextKey = (
    "status.command.run-arity",
    "Command failed: run expects zero args or one quoted command",
);
pub(crate) const STATUS_COMMAND_RESULTS_ARITY: TextKey = (
    "status.command.results-arity",
    "Command failed: results expects zero args or one match number",
);
pub(crate) const STATUS_COMMAND_OPEN_ARITY: TextKey = (
    "status.command.open-arity",
    "Command failed: open expects zero or one path",
);
pub(crate) const STATUS_COMMAND_SAVE_ARITY: TextKey = (
    "status.command.save-arity",
    "Command failed: save expects zero or one path",
);
pub(crate) const STATUS_COMMAND_SAVE_AS_ARITY: TextKey = (
    "status.command.save-as-arity",
    "Command failed: save-as expects zero or one path",
);
pub(crate) const STATUS_COMMAND_NO_ARGUMENTS: TextKey = (
    "status.command.no-arguments",
    "Command failed: {} expects no arguments",
);
pub(crate) const STATUS_COMMAND_FIND_ARITY: TextKey = (
    "status.command.find-arity",
    "Command failed: find expects zero or one query",
);
pub(crate) const STATUS_COMMAND_REPLACE_ARITY: TextKey = (
    "status.command.replace-arity",
    "Command failed: replace expects query and replacement, or all query replacement",
);
pub(crate) const STATUS_COMMAND_GO_TO_LINE_ARITY: TextKey = (
    "status.command.go-to-line-arity",
    "Command failed: go-to-line expects one line number",
);
pub(crate) const STATUS_COMMAND_CONFIG_SECTION: TextKey = (
    "status.command.config-section",
    "Command failed: config expects one of {}",
);
pub(crate) const STATUS_COMMAND_CONFIG_SECTION_ARITY: TextKey = (
    "status.command.config-section-arity",
    "Command failed: config expects zero args or one of {}",
);
pub(crate) const STATUS_PLUGIN_NOT_CONFIGURED: TextKey =
    ("status.plugin.not-configured", "No plugin host configured");
pub(crate) const STATUS_PLUGIN_IS_LOADED: TextKey =
    ("status.plugin.is-loaded", "Plugin {} is loaded");
pub(crate) const STATUS_PLUGIN_IS_UNLOADED: TextKey =
    ("status.plugin.is-unloaded", "Plugin {} is unloaded");
pub(crate) const STATUS_PLUGIN_UNLOADED: TextKey = ("status.plugin.unloaded", "Plugin {} unloaded");
pub(crate) const STATUS_PLUGIN_LOADED: TextKey = (
    "status.plugin.loaded",
    "Plugin {} loaded (starts on the next edit)",
);
pub(crate) const STATUS_PLUGIN_LOADED_EAGER: TextKey =
    ("status.plugin.loaded-eager", "Plugin {} loaded");
pub(crate) const STATUS_PLUGIN_USAGE: TextKey = (
    "status.plugin.usage",
    "Usage: plugin [load|unload] [plugin-id]",
);
pub(crate) const STATUS_PLUGIN_UNKNOWN_ID: TextKey =
    ("status.plugin.unknown-id", "No plugin named {}");
pub(crate) const STATUS_PLUGIN_FAILED: TextKey = ("status.plugin.failed", "Plugin {} failed: {}");
pub(crate) const STATUS_PLUGIN_WINDOW_LIMIT: TextKey = (
    "status.plugin.window-limit",
    "Plugin {} reached its window limit",
);
pub(crate) const STATUS_THEME_CHANGED: TextKey = ("status.theme.changed", "Theme: {}");
pub(crate) const STATUS_THEME_CURRENT: TextKey = ("status.theme.current", "Theme: {} ({})");
pub(crate) const STATUS_THEME_UNKNOWN: TextKey = (
    "status.theme.unknown",
    "Theme failed: unknown theme {}; expected {}",
);
pub(crate) const STATUS_CONFIG_RELOAD_FAILED: TextKey =
    ("status.config.reload-failed", "Config reload failed: {}");
pub(crate) const STATUS_CONFIG_RELOADED_DISABLED: TextKey = (
    "status.config.reloaded-disabled",
    "Config reloaded from built-in defaults (--no-config)",
);
pub(crate) const STATUS_CONFIG_RELOADED_PATH: TextKey =
    ("status.config.reloaded-path", "Config reloaded from {}");
pub(crate) const STATUS_CONFIG_RELOADED_ENVIRONMENT: TextKey = (
    "status.config.reloaded-environment",
    "Config reloaded from {}={}",
);
pub(crate) const STATUS_CONFIG_RELOADED_DEFAULTS: TextKey = (
    "status.config.reloaded-defaults",
    "Config reloaded from built-in defaults",
);
pub(crate) const STATUS_SHELL_ESCAPE: TextKey = ("status.shell.escape", "Shell escape");
pub(crate) const STATUS_SHELL_RETURNED: TextKey = ("status.shell.returned", "Shell returned {}");
pub(crate) const STATUS_RUN_FOCUSED_WINDOW_MISSING: TextKey = (
    "status.run.focused-window-missing",
    "Run command failed: focused window is missing",
);
pub(crate) const STATUS_RUN_OUTPUT_WINDOW_MISSING: TextKey = (
    "status.run.output-window-missing",
    "Run command failed: output window is missing",
);
pub(crate) const STATUS_RUN_RUNNING: TextKey = ("status.run.running", "Running command: {}");
pub(crate) const STATUS_RUN_FAILED: TextKey = ("status.run.failed", "Run command failed: {}");
pub(crate) const STATUS_RUN_RETURNED: TextKey =
    ("status.run.returned", "Command returned {} in {}");
pub(crate) const STATUS_RUN_RETURNED_TRUNCATED: TextKey = (
    "status.run.returned-truncated",
    "Command returned {} in {}; output truncated",
);
pub(crate) const STATUS_RUN_TIMED_OUT: TextKey = (
    "status.run.timed-out",
    "Command timed out after {} and was killed",
);
pub(crate) const STATUS_RUN_TIMED_OUT_TRUNCATED: TextKey = (
    "status.run.timed-out-truncated",
    "Command timed out after {} and was killed; output truncated",
);
pub(crate) const STATUS_RUN_EXIT: TextKey = ("status.run.exit", "exit {}");
pub(crate) const STATUS_RUN_TERMINATED: TextKey = ("status.run.terminated", "terminated");

// Helper-window status messages.
pub(crate) const STATUS_AUX_FOCUSED_WINDOW_MISSING: TextKey = (
    "status.aux-window.focused-window-missing",
    "{} failed: focused window is missing",
);
pub(crate) const STATUS_AUX_WINDOW_MISSING: TextKey = (
    "status.aux-window.window-missing",
    "{} failed: window is missing",
);
pub(crate) const STATUS_HELP_OPENED: TextKey = ("status.help.opened", "Help");
pub(crate) const STATUS_HELP_FOCUSED_WINDOW_MISSING: TextKey = (
    "status.help.focused-window-missing",
    "Help failed: focused window is missing",
);
pub(crate) const STATUS_HELP_WINDOW_MISSING: TextKey = (
    "status.help.window-missing",
    "Help failed: help window is missing",
);
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_OPENED: TextKey =
    ("status.config-diagnostics.opened", "Config diagnostics");
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_FOCUSED_WINDOW_MISSING: TextKey = (
    "status.config-diagnostics.focused-window-missing",
    "Config diagnostics failed: focused window is missing",
);
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_WINDOW_MISSING: TextKey = (
    "status.config-diagnostics.window-missing",
    "Config diagnostics failed: diagnostics window is missing",
);
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_BUFFER_MISSING: TextKey = (
    "status.config-diagnostics.buffer-missing",
    "Config diagnostics failed: diagnostics buffer is missing",
);
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_SECTION_NOT_FOUND: TextKey = (
    "status.config-diagnostics.section-not-found",
    "Config diagnostics: {} section not found",
);
pub(crate) const STATUS_CONFIG_DIAGNOSTICS_SECTION: TextKey = (
    "status.config-diagnostics.section",
    "Config diagnostics: {}",
);
pub(crate) const STATUS_HISTORY_OPENED: TextKey = ("status.history.opened", "Status history");
pub(crate) const STATUS_HISTORY_FOCUSED_WINDOW_MISSING: TextKey = (
    "status.history.focused-window-missing",
    "Status history failed: focused window is missing",
);
pub(crate) const STATUS_HISTORY_WINDOW_MISSING: TextKey = (
    "status.history.window-missing",
    "Status history failed: status window is missing",
);

#[cfg(test)]
#[rustfmt::skip]
pub(crate) const ALL: &[TextKey] = &[
    STATUS_COMMAND_CANCELLED, STATUS_COMMAND_UNKNOWN, STATUS_COMMAND_PARSE_FAILED, STATUS_COMMAND_PARSE_TRAILING_ESCAPE,
    STATUS_COMMAND_PARSE_UNCLOSED_QUOTE, STATUS_COMMAND_LINE_HELP, STATUS_COMMAND_THEME_ARITY, STATUS_COMMAND_RUN_ARITY,
    STATUS_COMMAND_RESULTS_ARITY, STATUS_COMMAND_OPEN_ARITY, STATUS_COMMAND_SAVE_ARITY, STATUS_COMMAND_SAVE_AS_ARITY,
    STATUS_COMMAND_NO_ARGUMENTS, STATUS_COMMAND_FIND_ARITY, STATUS_COMMAND_REPLACE_ARITY, STATUS_COMMAND_GO_TO_LINE_ARITY,
    STATUS_COMMAND_CONFIG_SECTION, STATUS_COMMAND_CONFIG_SECTION_ARITY, STATUS_PLUGIN_NOT_CONFIGURED, STATUS_PLUGIN_IS_LOADED,
    STATUS_PLUGIN_IS_UNLOADED, STATUS_PLUGIN_UNLOADED, STATUS_PLUGIN_LOADED, STATUS_PLUGIN_LOADED_EAGER,
    STATUS_PLUGIN_USAGE, STATUS_PLUGIN_UNKNOWN_ID, STATUS_PLUGIN_FAILED, STATUS_PLUGIN_WINDOW_LIMIT, STATUS_THEME_CHANGED,
    STATUS_THEME_CURRENT, STATUS_THEME_UNKNOWN,
    STATUS_CONFIG_RELOAD_FAILED, STATUS_CONFIG_RELOADED_DISABLED, STATUS_CONFIG_RELOADED_PATH, STATUS_CONFIG_RELOADED_ENVIRONMENT,
    STATUS_CONFIG_RELOADED_DEFAULTS, STATUS_SHELL_ESCAPE, STATUS_SHELL_RETURNED, STATUS_RUN_FOCUSED_WINDOW_MISSING,
    STATUS_RUN_OUTPUT_WINDOW_MISSING, STATUS_RUN_RUNNING, STATUS_RUN_FAILED, STATUS_RUN_RETURNED,
    STATUS_RUN_RETURNED_TRUNCATED, STATUS_RUN_TIMED_OUT, STATUS_RUN_TIMED_OUT_TRUNCATED, STATUS_RUN_EXIT,
    STATUS_RUN_TERMINATED, STATUS_AUX_FOCUSED_WINDOW_MISSING, STATUS_AUX_WINDOW_MISSING, STATUS_HELP_OPENED,
    STATUS_HELP_FOCUSED_WINDOW_MISSING, STATUS_HELP_WINDOW_MISSING, STATUS_CONFIG_DIAGNOSTICS_OPENED, STATUS_CONFIG_DIAGNOSTICS_FOCUSED_WINDOW_MISSING,
    STATUS_CONFIG_DIAGNOSTICS_WINDOW_MISSING, STATUS_CONFIG_DIAGNOSTICS_BUFFER_MISSING, STATUS_CONFIG_DIAGNOSTICS_SECTION_NOT_FOUND, STATUS_CONFIG_DIAGNOSTICS_SECTION,
    STATUS_HISTORY_OPENED, STATUS_HISTORY_FOCUSED_WINDOW_MISSING, STATUS_HISTORY_WINDOW_MISSING,
];
