use std::fmt;
use std::str::FromStr;

use dun_term::{ColorProfile, EncodingProfile, ThemeName};

use crate::keys::normalize_token;
use crate::plugins::{
    PLUGIN_ROLE_VALUES, PLUGIN_TRUST_VALUES, PluginEntryDraft, PluginRole, PluginTrust,
};
use crate::validation::{config_error_text, key_parse_error_text};
use crate::{Config, ConfigError, KeySequence, command_from_id, file_dialog_action_from_id};

pub fn parse_config(input: &str) -> Result<Config, ConfigParseError> {
    parse_config_overlay(Config::default(), input)
}

pub fn parse_config_overlay(mut config: Config, input: &str) -> Result<Config, ConfigParseError> {
    let mut plugin_entries = std::mem::take(&mut config.plugins)
        .into_iter()
        .map(PluginEntryDraft::from)
        .collect::<Vec<_>>();

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(ConfigParseError::line(
                line_number,
                "expected `key = value` entry",
            ));
        };
        apply_config_entry(
            &mut config,
            &mut plugin_entries,
            raw_key.trim(),
            raw_value.trim(),
            line_number,
        )?;
    }

    config.plugins = plugin_entries
        .into_iter()
        .map(PluginEntryDraft::into_entry)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| ConfigParseError::global(config_error_text(&ConfigError::from(error))))?;

    config
        .validate()
        .map_err(|error| ConfigParseError::global(config_error_text(&error)))?;
    Ok(config)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigParseError {
    pub line: Option<usize>,
    pub message: String,
}

impl ConfigParseError {
    fn line(line: usize, message: impl Into<String>) -> Self {
        Self {
            line: Some(line),
            message: message.into(),
        }
    }

    fn global(message: impl Into<String>) -> Self {
        Self {
            line: None,
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "line {line}: {}", self.message),
            None => write!(formatter, "{}", self.message),
        }
    }
}

impl std::error::Error for ConfigParseError {}

fn strip_comment(line: &str) -> &str {
    line.split_once('#')
        .map(|(before_comment, _)| before_comment)
        .unwrap_or(line)
}

fn apply_config_entry(
    config: &mut Config,
    plugin_entries: &mut Vec<PluginEntryDraft>,
    raw_key: &str,
    raw_value: &str,
    line_number: usize,
) -> Result<(), ConfigParseError> {
    if raw_key.is_empty() {
        return Err(ConfigParseError::line(line_number, "empty config key"));
    }

    let value = unquote_value(raw_value);

    if let Some(plugin_key) = raw_key.strip_prefix("plugin.") {
        apply_plugin_entry(plugin_entries, plugin_key, value, line_number)?;
        return Ok(());
    }

    let key = normalize_config_key(raw_key);

    match key.as_str() {
        "theme" => {
            config.theme = parse_theme_name(value)
                .ok_or_else(|| ConfigParseError::line(line_number, "unknown theme name"))?;
        }
        "terminal.encoding" => {
            config.terminal.encoding = parse_encoding_profile(value)
                .map(Some)
                .ok_or_else(|| ConfigParseError::line(line_number, "unknown terminal encoding"))?;
        }
        "terminal.colors" | "terminal.color" => {
            config.terminal.colors = parse_color_profile(value)
                .map(Some)
                .ok_or_else(|| ConfigParseError::line(line_number, "unknown terminal colors"))?;
        }
        "mouse.enabled" | "input.mouse" => {
            config.mouse.enabled = parse_bool(value)
                .ok_or_else(|| ConfigParseError::line(line_number, "expected true or false"))?;
        }
        "clipboard.osc52.enabled" | "clipboard.osc52" => {
            config.clipboard.osc52.enabled = parse_bool(value)
                .ok_or_else(|| ConfigParseError::line(line_number, "expected true or false"))?;
        }
        "clipboard.osc52.max_bytes" => {
            let value = parse_byte_count(value, line_number)?;
            config.clipboard.osc52.max_bytes = usize::try_from(value).map_err(|_| {
                ConfigParseError::line(line_number, "OSC 52 byte limit does not fit this platform")
            })?;
        }
        "limits.run_command_timeout_ms" => {
            config.limits.run_command_timeout_ms = value.parse().map_err(|_| {
                ConfigParseError::line(line_number, "expected a timeout in milliseconds")
            })?;
        }
        "limits.editable_file_soft_limit_bytes" => {
            config.limits.editable_file_soft_limit_bytes = parse_byte_count(value, line_number)?;
        }
        "limits.line_display_soft_limit_bytes" => {
            let value = parse_byte_count(value, line_number)?;
            config.limits.line_display_soft_limit_bytes = usize::try_from(value).map_err(|_| {
                ConfigParseError::line(
                    line_number,
                    "line display soft limit does not fit this platform",
                )
            })?;
        }
        _ if key.starts_with("key.file_dialog.") => {
            apply_file_dialog_key_binding(
                config,
                &key["key.file_dialog.".len()..],
                value,
                line_number,
            )?;
        }
        _ if key.starts_with("file_dialog.key.") => {
            apply_file_dialog_key_binding(
                config,
                &key["file_dialog.key.".len()..],
                value,
                line_number,
            )?;
        }
        _ if key.starts_with("key.") => {
            apply_key_binding(config, &key["key.".len()..], value, line_number)?;
        }
        _ if key.starts_with("keybinding.") => {
            apply_key_binding(config, &key["keybinding.".len()..], value, line_number)?;
        }
        _ => {
            return Err(ConfigParseError::line(
                line_number,
                format!("unknown config key `{raw_key}`"),
            ));
        }
    }

    Ok(())
}

fn apply_plugin_entry(
    entries: &mut Vec<PluginEntryDraft>,
    plugin_key: &str,
    value: &str,
    line_number: usize,
) -> Result<(), ConfigParseError> {
    let Some((id, field)) = plugin_key.rsplit_once('.') else {
        return Err(ConfigParseError::line(
            line_number,
            "expected `plugin.<id>.<field>` config key",
        ));
    };
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(ConfigParseError::line(
            line_number,
            format!("invalid plugin id `{id}`; expected only `[a-z0-9-]` characters"),
        ));
    }

    let entry = match entries.iter().position(|entry| entry.id == id) {
        Some(index) => &mut entries[index],
        None => {
            entries.push(PluginEntryDraft::new(id.to_string()));
            entries.last_mut().expect("plugin entry was just inserted")
        }
    };

    match field {
        "command" => entry.command = value.into(),
        "trust" => {
            entry.trust = Some(PluginTrust::from_config_value(value).ok_or_else(|| {
                ConfigParseError::line(
                    line_number,
                    format!(
                        "unknown plugin trust `{value}`; allowed values are {PLUGIN_TRUST_VALUES}"
                    ),
                )
            })?);
        }
        "roles" => {
            let roles = if value.is_empty() {
                Vec::new()
            } else {
                value
                    .split(',')
                    .map(|raw_role| {
                        let role = raw_role.trim();
                        PluginRole::from_config_value(role).ok_or_else(|| {
                            ConfigParseError::line(
                                line_number,
                                format!(
                                    "unknown plugin role `{role}`; allowed values are {PLUGIN_ROLE_VALUES}"
                                ),
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };
            entry.roles = roles;
        }
        "timeout_ms" => {
            entry.timeout_ms = value.parse().map_err(|_| {
                ConfigParseError::line(line_number, "expected a plugin timeout in milliseconds")
            })?;
        }
        "max_frame_bytes" => {
            let value = parse_byte_count(value, line_number)?;
            entry.max_frame_bytes = usize::try_from(value).map_err(|_| {
                ConfigParseError::line(
                    line_number,
                    "plugin frame byte limit does not fit this platform",
                )
            })?;
        }
        _ => {
            return Err(ConfigParseError::line(
                line_number,
                format!("unknown plugin config field `{field}`"),
            ));
        }
    }

    Ok(())
}

fn apply_key_binding(
    config: &mut Config,
    command_id: &str,
    value: &str,
    line_number: usize,
) -> Result<(), ConfigParseError> {
    let command = command_from_id(command_id).map_err(|_| {
        ConfigParseError::line(line_number, format!("unknown command id `{command_id}`"))
    })?;

    let sequence = match normalize_token(value).as_str() {
        "none" | "disabled" | "unbind" => None,
        _ => Some(KeySequence::from_str(value).map_err(|error| {
            ConfigParseError::line(
                line_number,
                format!("invalid key sequence: {}", key_parse_error_text(&error)),
            )
        })?),
    };

    config.keybindings.set_command_binding(command, sequence);
    Ok(())
}

fn apply_file_dialog_key_binding(
    config: &mut Config,
    action_id: &str,
    value: &str,
    line_number: usize,
) -> Result<(), ConfigParseError> {
    let action = file_dialog_action_from_id(action_id).map_err(|_| {
        ConfigParseError::line(
            line_number,
            format!("unknown file dialog action `{action_id}`"),
        )
    })?;

    let stroke = match normalize_token(value).as_str() {
        "none" | "disabled" | "unbind" => None,
        _ => {
            let sequence = KeySequence::from_str(value).map_err(|error| {
                ConfigParseError::line(
                    line_number,
                    format!("invalid key sequence: {}", key_parse_error_text(&error)),
                )
            })?;
            if sequence.strokes.len() != 1 {
                return Err(ConfigParseError::line(
                    line_number,
                    "file dialog keybindings must use a single key stroke",
                ));
            }
            Some(sequence.strokes[0])
        }
    };

    config.file_dialog_keys.set_action_binding(action, stroke);
    Ok(())
}

fn normalize_config_key(input: &str) -> String {
    input
        .trim()
        .chars()
        .map(|ch| match ch {
            '-' | ' ' => '_',
            _ => ch.to_ascii_lowercase(),
        })
        .collect()
}

fn unquote_value(input: &str) -> &str {
    let trimmed = input.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
        {
            return &trimmed[1..trimmed.len() - 1];
        }
    }

    trimmed
}

fn parse_theme_name(input: &str) -> Option<ThemeName> {
    match normalize_token(input).as_str() {
        "msedit" | "microsoftedit" => Some(ThemeName::MsEdit),
        "turbo" | "turbovision" => Some(ThemeName::Turbo),
        "dark" => Some(ThemeName::Dark),
        "dun" => Some(ThemeName::Dun),
        "solarizeddark" => Some(ThemeName::SolarizedDark),
        "solarizedlight" => Some(ThemeName::SolarizedLight),
        _ => None,
    }
}

fn parse_encoding_profile(input: &str) -> Option<EncodingProfile> {
    match normalize_token(input).as_str() {
        "utf8" => Some(EncodingProfile::Utf8),
        "ascii" => Some(EncodingProfile::Ascii),
        _ => None,
    }
}

fn parse_color_profile(input: &str) -> Option<ColorProfile> {
    match normalize_token(input).as_str() {
        "256" | "256color" | "color256" => Some(ColorProfile::Color256),
        "16" | "16color" | "color16" | "ansi" => Some(ColorProfile::Color16),
        "mono" | "monochrome" | "none" | "off" => Some(ColorProfile::Mono),
        _ => None,
    }
}

fn parse_bool(input: &str) -> Option<bool> {
    match normalize_token(input).as_str() {
        "true" | "yes" | "on" | "1" | "enabled" => Some(true),
        "false" | "no" | "off" | "0" | "disabled" => Some(false),
        _ => None,
    }
}

fn parse_byte_count(input: &str, line_number: usize) -> Result<u64, ConfigParseError> {
    let normalized: String = input
        .trim()
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_')
        .flat_map(char::to_lowercase)
        .collect();

    let digit_count = normalized
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .count();
    if digit_count == 0 {
        return Err(ConfigParseError::line(
            line_number,
            "expected byte count such as `1048576` or `16 MiB`",
        ));
    }

    let number = normalized[..digit_count].parse::<u64>().map_err(|_| {
        ConfigParseError::line(line_number, "byte count is outside the supported range")
    })?;
    let suffix = &normalized[digit_count..];
    let multiplier = match suffix {
        "" | "b" | "byte" | "bytes" => 1,
        "k" | "kb" | "kib" => 1024,
        "m" | "mb" | "mib" => 1024 * 1024,
        "g" | "gb" | "gib" => 1024 * 1024 * 1024,
        _ => {
            return Err(ConfigParseError::line(
                line_number,
                format!("unknown byte-count suffix `{suffix}`"),
            ));
        }
    };

    number.checked_mul(multiplier).ok_or_else(|| {
        ConfigParseError::line(line_number, "byte count is outside the supported range")
    })
}
