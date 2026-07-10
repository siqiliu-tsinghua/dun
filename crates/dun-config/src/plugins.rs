use std::path::PathBuf;

pub(crate) const DEFAULT_PLUGIN_TIMEOUT_MS: u64 = 2_000;
pub(crate) const DEFAULT_PLUGIN_MAX_FRAME_BYTES: usize = 256 * 1024;
pub(crate) const PLUGIN_TRUST_VALUES: &str = "`pure-sandbox` or `user-trusted-external`";
pub(crate) const PLUGIN_ROLE_VALUES: &str =
    "`syntax-highlight`, `log-filter`, `text-transform`, or `config-helper`";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginEntry {
    pub id: String,
    pub command: PathBuf,
    pub trust: PluginTrust,
    pub roles: Vec<PluginRole>,
    pub timeout_ms: u64,
    pub max_frame_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginTrust {
    PureSandbox,
    UserTrustedExternal,
}

impl PluginTrust {
    pub(crate) fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "pure-sandbox" => Some(Self::PureSandbox),
            "user-trusted-external" => Some(Self::UserTrustedExternal),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginRole {
    SyntaxHighlight,
    LogFilter,
    TextTransform,
    ConfigHelper,
}

impl PluginRole {
    pub(crate) fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "syntax-highlight" => Some(Self::SyntaxHighlight),
            "log-filter" => Some(Self::LogFilter),
            "text-transform" => Some(Self::TextTransform),
            "config-helper" => Some(Self::ConfigHelper),
            _ => None,
        }
    }

    pub(crate) const fn as_config_value(self) -> &'static str {
        match self {
            Self::SyntaxHighlight => "syntax-highlight",
            Self::LogFilter => "log-filter",
            Self::TextTransform => "text-transform",
            Self::ConfigHelper => "config-helper",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginConfigError {
    MissingCommand(String),
    MissingTrust(String),
    MissingRoles(String),
    TimeoutZero(String),
    MaxFrameBytesZero(String),
    DuplicateRole { id: String, role: PluginRole },
}

pub(crate) fn validate_plugin_entries(entries: &[PluginEntry]) -> Result<(), PluginConfigError> {
    for entry in entries {
        if entry.command.as_os_str().is_empty() {
            return Err(PluginConfigError::MissingCommand(entry.id.clone()));
        }
        if entry.roles.is_empty() {
            return Err(PluginConfigError::MissingRoles(entry.id.clone()));
        }
        if entry.timeout_ms == 0 {
            return Err(PluginConfigError::TimeoutZero(entry.id.clone()));
        }
        if entry.max_frame_bytes == 0 {
            return Err(PluginConfigError::MaxFrameBytesZero(entry.id.clone()));
        }
        for (index, role) in entry.roles.iter().copied().enumerate() {
            if entry.roles[..index].contains(&role) {
                return Err(PluginConfigError::DuplicateRole {
                    id: entry.id.clone(),
                    role,
                });
            }
        }
    }

    Ok(())
}

pub(crate) struct PluginEntryDraft {
    pub(crate) id: String,
    pub(crate) command: PathBuf,
    pub(crate) trust: Option<PluginTrust>,
    pub(crate) roles: Vec<PluginRole>,
    pub(crate) timeout_ms: u64,
    pub(crate) max_frame_bytes: usize,
}

impl PluginEntryDraft {
    pub(crate) fn new(id: String) -> Self {
        Self {
            id,
            command: PathBuf::new(),
            trust: None,
            roles: Vec::new(),
            timeout_ms: DEFAULT_PLUGIN_TIMEOUT_MS,
            max_frame_bytes: DEFAULT_PLUGIN_MAX_FRAME_BYTES,
        }
    }

    pub(crate) fn into_entry(self) -> Result<PluginEntry, PluginConfigError> {
        let trust = self
            .trust
            .ok_or_else(|| PluginConfigError::MissingTrust(self.id.clone()))?;
        Ok(PluginEntry {
            id: self.id,
            command: self.command,
            trust,
            roles: self.roles,
            timeout_ms: self.timeout_ms,
            max_frame_bytes: self.max_frame_bytes,
        })
    }
}

impl From<PluginEntry> for PluginEntryDraft {
    fn from(entry: PluginEntry) -> Self {
        Self {
            id: entry.id,
            command: entry.command,
            trust: Some(entry.trust),
            roles: entry.roles,
            timeout_ms: entry.timeout_ms,
            max_frame_bytes: entry.max_frame_bytes,
        }
    }
}
