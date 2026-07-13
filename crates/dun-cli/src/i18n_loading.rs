use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use dun_config::{MAX_CATALOG_FILE_BYTES, TextCatalog, locale_candidates, parse_catalog};
use dun_term::EncodingProfile;

use crate::config_loading::{ConfigSource, default_config_path};

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
/// nonempty of `LC_ALL`/`LC_MESSAGES`/`LANG`, resource files in `i18n/`
/// next to the active config file. ASCII terminals stay English — the
/// sanitizer would only escape non-ASCII labels into unreadable chrome.
pub(crate) fn load_ui_catalog(source: &ConfigSource, encoding: EncodingProfile) -> LoadedCatalog {
    if matches!(encoding, EncodingProfile::Ascii) {
        return LoadedCatalog::english();
    }
    let Some(dir) = i18n_dir_for_source(source) else {
        return LoadedCatalog::english();
    };
    catalog_from_dir(&dir, &locale_value().unwrap_or_default())
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

fn locale_value() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(env::var_os)
        .map(|value| value.to_string_lossy().into_owned())
        .find(|value| !value.is_empty())
}

/// Pure worker for `load_ui_catalog`, separated for tests. Tries the
/// locale's candidate files most specific first and stops at the first one
/// that exists: a broken `zh-CN.conf` reports its error instead of silently
/// masking itself behind `zh.conf`.
pub(crate) fn catalog_from_dir(dir: &Path, raw_locale: &str) -> LoadedCatalog {
    for lang in locale_candidates(raw_locale) {
        let path = dir.join(format!("{lang}.conf"));
        if !path.exists() {
            continue;
        }
        let text = match read_capped(&path) {
            Ok(text) => text,
            Err(error) => {
                return LoadedCatalog::failed(format!("i18n: {}: {error}", path.display()));
            }
        };
        return match parse_catalog(&text, &lang) {
            Ok(catalog) => LoadedCatalog {
                catalog,
                diagnostic: None,
            },
            Err(error) => LoadedCatalog::failed(format!("i18n: {}: {error}", path.display())),
        };
    }
    LoadedCatalog::english()
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
