use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use dun_config::{Config, TextCatalog, parse_config_overlay};

use crate::ui_text;

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
    /// Where the *user* layer came from; `BuiltInDefaults` means there was
    /// no user file, not that nothing was loaded — see `base`.
    pub(crate) source: ConfigSource,
    /// The installed configuration the user layer was applied on top of,
    /// when there is one.
    pub(crate) base: Option<PathBuf>,
    /// An installed configuration that exists but could not be used. It is
    /// a machine-wide file: reporting it must not stop the editor, because
    /// one broken file would otherwise lock out every user of the machine.
    pub(crate) base_diagnostic: Option<String>,
}

impl LoadedConfig {
    /// What to name as the source in a status message. With no user file
    /// but an installed one, "built-in defaults" would be untrue — the
    /// installed file is where the settings came from.
    pub(crate) fn status_source(&self) -> ConfigSource {
        match (&self.source, &self.base) {
            (ConfigSource::BuiltInDefaults, Some(base)) => ConfigSource::DefaultFile(base.clone()),
            (source, _) => source.clone(),
        }
    }
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
    pub(crate) fn status_text(&self, catalog: &TextCatalog) -> String {
        match self {
            Self::Disabled => {
                ui_text::tr(catalog, ui_text::STATUS_CONFIG_RELOADED_DISABLED).to_string()
            }
            Self::Explicit(path) | Self::DefaultFile(path) => ui_text::tr_fmt(
                catalog,
                ui_text::STATUS_CONFIG_RELOADED_PATH,
                &[&path.display().to_string()],
            ),
            Self::Environment(path) => ui_text::tr_fmt(
                catalog,
                ui_text::STATUS_CONFIG_RELOADED_ENVIRONMENT,
                &[DUN_CONFIG_ENV, &path.display().to_string()],
            ),
            Self::BuiltInDefaults => {
                ui_text::tr(catalog, ui_text::STATUS_CONFIG_RELOADED_DEFAULTS).to_string()
            }
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

/// Two layers, in this order: the installed configuration that came with
/// the binary, then one user file on top of it. Keys the user file sets
/// win; keys it leaves alone keep the installed value, and failing that the
/// built-in default. `--no-config` disables both — it means built-in
/// everything.
pub(crate) fn load_config(request: &ConfigLoadRequest) -> io::Result<LoadedConfig> {
    load_config_from(request, installed_config_path())
}

/// The worker, with the installed layer's path passed in: `current_exe()`
/// cannot be moved in a test, and the layering rules are what need testing.
pub(crate) fn load_config_from(
    request: &ConfigLoadRequest,
    installed_path: Option<PathBuf>,
) -> io::Result<LoadedConfig> {
    if request.no_config {
        return Ok(LoadedConfig {
            config: Config::default(),
            source: ConfigSource::Disabled,
            base: None,
            base_diagnostic: None,
        });
    }

    let installed = load_installed_config(installed_path);
    let base_config = installed.config;
    let base = installed.path;
    let base_diagnostic = installed.diagnostic;

    if let Some(path) = &request.explicit_path {
        return Ok(LoadedConfig {
            config: read_config_file_onto(base_config, path)?,
            source: ConfigSource::Explicit(path.clone()),
            base,
            base_diagnostic,
        });
    }

    if let Some(path) = env_config_path() {
        return Ok(LoadedConfig {
            config: read_config_file_onto(base_config, &path)?,
            source: ConfigSource::Environment(path),
            base,
            base_diagnostic,
        });
    }

    if let Some(path) = default_config_path() {
        if path.exists() {
            return Ok(LoadedConfig {
                config: read_config_file_onto(base_config, &path)?,
                source: ConfigSource::DefaultFile(path),
                base,
                base_diagnostic,
            });
        }
    }

    Ok(LoadedConfig {
        config: base_config,
        source: ConfigSource::BuiltInDefaults,
        base,
        base_diagnostic,
    })
}

/// The installed layer: `<bin>/../share/dun/config`, absent on a machine
/// where nobody installed one.
struct InstalledConfig {
    config: Config,
    path: Option<PathBuf>,
    diagnostic: Option<String>,
}

fn load_installed_config(installed_path: Option<PathBuf>) -> InstalledConfig {
    let none = InstalledConfig {
        config: Config::default(),
        path: None,
        diagnostic: None,
    };
    let Some(path) = installed_path else {
        return none;
    };
    if !path.exists() {
        return none;
    }
    match read_config_file_onto(Config::default(), &path) {
        Ok(config) => InstalledConfig {
            config,
            path: Some(path),
            diagnostic: None,
        },
        // Deliberately not an error: this file belongs to whoever installed
        // dun, and a machine-wide mistake must not stop every user's editor.
        // The user's own file still applies, on built-in defaults.
        Err(error) => InstalledConfig {
            diagnostic: Some(format!("config: {error}")),
            ..none
        },
    }
}

/// `<bin>/../share/dun` for the running executable, so nothing has to be
/// compiled in and the same binary works wherever it was put:
/// `/opt/dun/bin/dun` reads `/opt/dun/share/dun`, `/usr/bin/dun` reads
/// `/usr/share/dun`, `~/.local/bin/dun` reads `~/.local/share/dun`.
pub(crate) fn installed_share_dir_for_exe(exe: &Path) -> Option<PathBuf> {
    Some(exe.parent()?.parent()?.join("share").join("dun"))
}

pub(crate) fn installed_share_dir() -> Option<PathBuf> {
    installed_share_dir_for_exe(&env::current_exe().ok()?)
}

pub(crate) fn installed_config_path() -> Option<PathBuf> {
    Some(installed_share_dir()?.join("config"))
}

pub(crate) fn installed_config_path_text() -> String {
    match installed_config_path() {
        Some(path) if path.exists() => path.display().to_string(),
        Some(path) => format!("{} (absent)", path.display()),
        None => "(unknown)".to_string(),
    }
}

fn read_config_file_onto(base: Config, path: &Path) -> io::Result<Config> {
    let text = fs::read_to_string(path)
        .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))?;
    parse_config_overlay(base, &text).map_err(|error| {
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
