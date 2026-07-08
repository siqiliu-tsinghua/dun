use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use dun_config::{Config, parse_config};

pub(crate) const DUN_CONFIG_ENV: &str = "DUN_CONFIG";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConfigLoadRequest {
    explicit_path: Option<PathBuf>,
    no_config: bool,
}

impl ConfigLoadRequest {
    pub(crate) const fn new(explicit_path: Option<PathBuf>, no_config: bool) -> Self {
        Self {
            explicit_path,
            no_config,
        }
    }

    #[cfg(test)]
    pub(crate) fn explicit(path: PathBuf) -> Self {
        Self::new(Some(path), false)
    }

    pub(crate) fn diagnostics_text(&self) -> String {
        if self.no_config {
            return "--no-config".to_string();
        }

        match &self.explicit_path {
            Some(path) => format!("--config {}", path.display()),
            None => "discovery (DUN_CONFIG, XDG_CONFIG_HOME, HOME)".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoadedConfig {
    pub(crate) config: Config,
    pub(crate) source: ConfigSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ConfigSource {
    Disabled,
    Explicit(PathBuf),
    Environment(PathBuf),
    DefaultFile(PathBuf),
    BuiltInDefaults,
}

impl ConfigSource {
    pub(crate) fn status_text(&self) -> String {
        match self {
            Self::Disabled => "Config reloaded from built-in defaults (--no-config)".to_string(),
            Self::Explicit(path) => format!("Config reloaded from {}", path.display()),
            Self::Environment(path) => {
                format!("Config reloaded from {DUN_CONFIG_ENV}={}", path.display())
            }
            Self::DefaultFile(path) => format!("Config reloaded from {}", path.display()),
            Self::BuiltInDefaults => "Config reloaded from built-in defaults".to_string(),
        }
    }

    pub(crate) fn diagnostics_text(&self) -> String {
        match self {
            Self::Disabled => "disabled (--no-config)".to_string(),
            Self::Explicit(path) => format!("explicit file ({})", path.display()),
            Self::Environment(path) => format!("{DUN_CONFIG_ENV} ({})", path.display()),
            Self::DefaultFile(path) => format!("default file ({})", path.display()),
            Self::BuiltInDefaults => "built-in defaults".to_string(),
        }
    }
}

#[cfg(test)]
pub(crate) fn load_startup_config(
    explicit_path: Option<&Path>,
    no_config: bool,
) -> io::Result<Config> {
    let request = ConfigLoadRequest::new(explicit_path.map(Path::to_path_buf), no_config);
    load_config(&request).map(|loaded| loaded.config)
}

pub(crate) fn load_config(request: &ConfigLoadRequest) -> io::Result<LoadedConfig> {
    if request.no_config {
        return Ok(LoadedConfig {
            config: Config::default(),
            source: ConfigSource::Disabled,
        });
    }

    if let Some(path) = &request.explicit_path {
        return Ok(LoadedConfig {
            config: read_config_file(path)?,
            source: ConfigSource::Explicit(path.clone()),
        });
    }

    if let Some(path) = env_config_path() {
        return Ok(LoadedConfig {
            config: read_config_file(&path)?,
            source: ConfigSource::Environment(path),
        });
    }

    if let Some(path) = default_config_path() {
        if path.exists() {
            return Ok(LoadedConfig {
                config: read_config_file(&path)?,
                source: ConfigSource::DefaultFile(path),
            });
        }
    }

    Ok(LoadedConfig {
        config: Config::default(),
        source: ConfigSource::BuiltInDefaults,
    })
}

fn read_config_file(path: &Path) -> io::Result<Config> {
    let text = fs::read_to_string(path)
        .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))?;
    parse_config(&text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {error}", path.display()),
        )
    })
}

pub(crate) fn env_config_path() -> Option<PathBuf> {
    env::var_os(DUN_CONFIG_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub(crate) fn default_config_path() -> Option<PathBuf> {
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(config_home).join("dun").join("config"));
    }

    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("dun")
                .join("config")
        })
}
