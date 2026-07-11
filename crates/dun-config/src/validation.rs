use crate::{
    ConfigError, FileDialogKeymapError, KeyParseError, KeymapError, LimitsError, PluginConfigError,
};

pub(crate) fn config_error_text(error: &ConfigError) -> String {
    match error {
        ConfigError::Keymap(error) => format!("invalid keymap: {}", keymap_error_text(error)),
        ConfigError::FileDialogKeymap(error) => {
            format!(
                "invalid file dialog keymap: {}",
                file_dialog_keymap_error_text(error)
            )
        }
        ConfigError::Limits(error) => format!("invalid limits: {}", limits_error_text(*error)),
        ConfigError::Plugins(error) => {
            format!("invalid plugin configuration: {}", plugin_error_text(error))
        }
    }
}

fn plugin_error_text(error: &PluginConfigError) -> String {
    match error {
        PluginConfigError::MissingCommand(id) => {
            format!("plugin `{id}` requires a nonempty command")
        }
        PluginConfigError::MissingTrust(id) => {
            format!("plugin `{id}` requires an explicit trust value")
        }
        PluginConfigError::MissingRoles(id) => {
            format!("plugin `{id}` requires at least one role")
        }
        PluginConfigError::TimeoutZero(id) => {
            format!("plugin `{id}` timeout must be greater than zero milliseconds")
        }
        PluginConfigError::MaxFrameBytesZero(id) => {
            format!("plugin `{id}` frame byte limit must be greater than zero")
        }
        PluginConfigError::DuplicateRole { id, role } => format!(
            "plugin `{id}` has duplicate role `{}`",
            role.as_config_value()
        ),
    }
}

fn keymap_error_text(error: &KeymapError) -> String {
    match error {
        KeymapError::DuplicateBinding {
            sequence,
            bound,
            rebound,
        } => format!(
            "`{sequence}` is bound to both `{bound}` and `{rebound}`; \
             unbind one with `key.{bound} = none`"
        ),
        KeymapError::EmptySequence => "empty key sequence".to_string(),
    }
}

fn file_dialog_keymap_error_text(error: &FileDialogKeymapError) -> String {
    match error {
        FileDialogKeymapError::DuplicateBinding {
            stroke,
            bound,
            rebound,
        } => format!(
            "`{stroke}` is bound to both `{bound}` and `{rebound}`; \
             unbind one with `key.{bound} = none`"
        ),
    }
}

fn limits_error_text(error: LimitsError) -> &'static str {
    match error {
        LimitsError::EditableFileSoftLimitZero => {
            "editable file soft limit must be greater than zero"
        }
        LimitsError::LineDisplaySoftLimitZero => {
            "line display soft limit must be greater than zero"
        }
        LimitsError::RunCommandTimeoutZero => {
            "run command timeout must be greater than zero milliseconds"
        }
    }
}

pub(crate) fn key_parse_error_text(error: &KeyParseError) -> String {
    match error {
        KeyParseError::EmptySequence => "empty sequence".to_string(),
        KeyParseError::EmptyStroke => "empty stroke".to_string(),
        KeyParseError::MissingKey => "missing key".to_string(),
        KeyParseError::DuplicateModifier(modifier) => {
            format!("duplicate modifier `{modifier}`")
        }
        KeyParseError::UnknownModifier(modifier) => format!("unknown modifier `{modifier}`"),
        KeyParseError::UnknownKey(key) => format!("unknown key `{key}`"),
        KeyParseError::InvalidFunctionKey(key) => format!("invalid function key `{key}`"),
    }
}
