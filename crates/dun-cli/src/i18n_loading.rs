use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use dun_config::{MAX_CATALOG_FILE_BYTES, TextCatalog, locale_candidates, parse_catalog};
use dun_term::EncodingProfile;

use crate::config_loading::{ConfigSource, default_config_path, installed_share_dir_for_exe};

/// The catalog to install plus an optional diagnostic. Absence of
/// translations is never a diagnostic — only a present-but-unusable file is.
pub(crate) struct LoadedCatalog {
    pub(crate) catalog: TextCatalog,
    pub(crate) diagnostic: Option<String>,
}

impl LoadedCatalog {
    fn english() -> Self {
        Self {
            catalog: TextCatalog::empty(),
            diagnostic: None,
        }
    }

    fn failed(diagnostic: String) -> Self {
        Self {
            catalog: TextCatalog::empty(),
            diagnostic: Some(diagnostic),
        }
    }
}

/// Load UI translations for the current locale (docs/i18n.md): first
/// nonempty of `LC_ALL`/`LC_MESSAGES`/`LANG`, resource files in the
/// directories `i18n_search_dirs` names. ASCII terminals stay English — the
/// sanitizer would only escape non-ASCII labels into unreadable chrome.
pub(crate) fn load_ui_catalog(source: &ConfigSource, encoding: EncodingProfile) -> LoadedCatalog {
    if matches!(encoding, EncodingProfile::Ascii) {
        return LoadedCatalog::english();
    }
    load_from_dirs(
        &i18n_search_dirs(source),
        &locale_value().unwrap_or_default(),
    )
}

/// Pure worker for `load_ui_catalog`, separated for tests. A directory with
/// no file for this locale is skipped; the first directory that has one ends
/// the search, including when that file turns out to be broken — a shared
/// installation must not silently paper over the catalog you installed
/// yourself.
pub(crate) fn load_from_dirs(dirs: &[PathBuf], raw_locale: &str) -> LoadedCatalog {
    for dir in dirs {
        if let Some(loaded) = catalog_in_dir(dir, raw_locale) {
            return loaded;
        }
    }
    LoadedCatalog::english()
}

/// Where catalogs are looked for, most specific first: the `i18n/` directory
/// beside the active config file, then the installation's shared directory.
/// Empty when config is disabled — `--no-config` means built-in everything,
/// and that includes the built-in language.
pub(crate) fn i18n_search_dirs(source: &ConfigSource) -> Vec<PathBuf> {
    let mut dirs = Vec::with_capacity(2);
    if matches!(source, ConfigSource::Disabled) {
        return dirs;
    }
    if let Some(dir) = i18n_dir_for_source(source) {
        dirs.push(dir);
    }
    if let Some(shared) = shared_i18n_dir() {
        if !dirs.contains(&shared) {
            dirs.push(shared);
        }
    }
    dirs
}

/// The `i18n/` directory belonging to a config source, or `None` when
/// config is disabled (`--no-config` means built-in everything).
pub(crate) fn i18n_dir_for_source(source: &ConfigSource) -> Option<PathBuf> {
    let config_path = match source {
        ConfigSource::Disabled => return None,
        ConfigSource::Explicit(path)
        | ConfigSource::Environment(path)
        | ConfigSource::DefaultFile(path) => path.clone(),
        ConfigSource::BuiltInDefaults => default_config_path()?,
    };
    Some(config_path.parent()?.join("i18n"))
}

/// The installation's shared catalog directory, for a system-wide install
/// where the per-user config directory is not where the files went.
fn shared_i18n_dir() -> Option<PathBuf> {
    shared_i18n_dir_for_exe(&env::current_exe().ok()?)
}

/// The catalogs of the installation the running binary belongs to — the
/// `i18n/` directory of the same `share/dun` that holds the installed
/// configuration, so the two always travel together.
pub(crate) fn shared_i18n_dir_for_exe(exe: &Path) -> Option<PathBuf> {
    Some(installed_share_dir_for_exe(exe)?.join("i18n"))
}

/// The search path as the Config Diagnostics window reports it.
pub(crate) fn i18n_search_text(source: &ConfigSource) -> String {
    let dirs = i18n_search_dirs(source);
    if dirs.is_empty() {
        return "disabled (--no-config)".to_string();
    }
    let mut text = String::new();
    for dir in dirs {
        if !text.is_empty() {
            text.push_str(", ");
        }
        text.push_str(&dir.display().to_string());
    }
    text
}

pub(crate) fn locale_value() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(env::var_os)
        .map(|value| value.to_string_lossy().into_owned())
        .find(|value| !value.is_empty())
}

/// One directory's answer for this locale, or `None` when it has no file
/// for any candidate. Tries the candidates most specific first and stops at
/// the first one that exists: a broken `zh-CN.conf` reports its error
/// instead of silently masking itself behind `zh.conf`.
fn catalog_in_dir(dir: &Path, raw_locale: &str) -> Option<LoadedCatalog> {
    for lang in locale_candidates(raw_locale) {
        let path = dir.join(format!("{lang}.conf"));
        if !path.exists() {
            continue;
        }
        let text = match read_capped(&path) {
            Ok(text) => text,
            Err(error) => {
                return Some(LoadedCatalog::failed(format!(
                    "i18n: {}: {error}",
                    path.display()
                )));
            }
        };
        return Some(match parse_catalog(&text, &lang) {
            Ok(catalog) => LoadedCatalog {
                catalog,
                diagnostic: None,
            },
            Err(error) => LoadedCatalog::failed(format!("i18n: {}: {error}", path.display())),
        });
    }
    None
}

/// Single-directory entry point kept for the tests that predate the search
/// path; `load_from_dirs` is what the editor calls.
#[cfg(test)]
pub(crate) fn catalog_from_dir(dir: &Path, raw_locale: &str) -> LoadedCatalog {
    catalog_in_dir(dir, raw_locale).unwrap_or_else(LoadedCatalog::english)
}

/// Read at most the catalog cap; a translation file is untrusted input and
/// must not be able to make the editor slurp an arbitrarily large file.
fn read_capped(path: &Path) -> std::io::Result<String> {
    let file = File::open(path)?;
    let mut text = String::new();
    file.take(MAX_CATALOG_FILE_BYTES as u64 + 1)
        .read_to_string(&mut text)?;
    if text.len() > MAX_CATALOG_FILE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("file exceeds {MAX_CATALOG_FILE_BYTES} byte cap"),
        ));
    }
    Ok(text)
}
