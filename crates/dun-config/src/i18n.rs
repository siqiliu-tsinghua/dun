//! UI text catalog: external per-language resource files (`docs/i18n.md`).
//!
//! English lives in the binary as `&'static str` defaults; a catalog holds
//! the loaded translations for one language. Parsing is pure — file
//! discovery and locale detection belong to the caller (`dun-cli`).

use std::collections::HashMap;
use std::fmt;

use dun_core::DisplaySanitizer;

/// Whole-file cap checked by the caller before parsing.
pub const MAX_CATALOG_FILE_BYTES: usize = 64 * 1024;
/// Per-value cap: UI labels are short; anything longer is a mistake.
pub const MAX_CATALOG_VALUE_BYTES: usize = 256;

/// Loaded translations for one language. Empty means built-in English.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextCatalog {
    lang: Option<String>,
    entries: HashMap<String, String>,
}

impl TextCatalog {
    pub fn empty() -> Self {
        Self::default()
    }

    /// The language tag this catalog was loaded for, if any.
    pub fn lang(&self) -> Option<&str> {
        self.lang.as_deref()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}

/// Locale candidates for a raw locale value, most specific first:
/// `zh_CN.UTF-8` → `["zh-CN", "zh-Hans", "zh"]`. `C`, `POSIX`, and
/// empty select none.
pub fn locale_candidates(raw_locale: &str) -> Vec<String> {
    let base = raw_locale
        .split(['.', '@'])
        .next()
        .unwrap_or_default()
        .trim();
    if base.is_empty() || base.eq_ignore_ascii_case("c") || base.eq_ignore_ascii_case("posix") {
        return Vec::new();
    }

    let mut parts = base.splitn(2, ['_', '-']);
    let primary = parts.next().unwrap_or_default().to_ascii_lowercase();
    if primary.is_empty() || !primary.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return Vec::new();
    }

    let region = match parts.next() {
        Some(region) => {
            let region = region.to_ascii_uppercase();
            if region.is_empty() || !region.chars().all(|ch| ch.is_ascii_alphanumeric()) {
                return vec![primary];
            }
            Some(region)
        }
        None => None,
    };

    let mut candidates = Vec::with_capacity(3);
    if let Some(region) = region.as_deref() {
        candidates.push(format!("{primary}-{region}"));
    }
    if let Some(script) = locale_script(&primary, region.as_deref()) {
        candidates.push(format!("{primary}-{script}"));
    }
    candidates.push(primary);
    candidates
}

fn locale_script(language: &str, region: Option<&str>) -> Option<&'static str> {
    match (language, region) {
        ("zh", None | Some("CN" | "SG" | "MY")) => Some("Hans"),
        ("zh", Some("TW" | "HK" | "MO")) => Some("Hant"),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogParseError {
    Line { line_number: usize, text: String },
}

impl CatalogParseError {
    fn line(line_number: usize, text: impl Into<String>) -> Self {
        Self::Line {
            line_number,
            text: text.into(),
        }
    }
}

impl fmt::Display for CatalogParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Line { line_number, text } => write!(formatter, "line {line_number}: {text}"),
        }
    }
}

/// Parse one language resource file. Rejects the whole file on the first
/// violation: a translation that fails the safety rules must not half-load.
pub fn parse_catalog(input: &str, lang: &str) -> Result<TextCatalog, CatalogParseError> {
    let mut entries = HashMap::new();

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(CatalogParseError::line(
                line_number,
                "expected `key = value` entry",
            ));
        };
        let key = raw_key.trim();
        let value = raw_value.trim();

        if key.is_empty()
            || !key.chars().all(|ch| {
                ch.is_ascii_lowercase()
                    || ch.is_ascii_digit()
                    || ch == '.'
                    || ch == '-'
                    || ch == '_'
            })
        {
            return Err(CatalogParseError::line(
                line_number,
                format!("invalid key `{key}`; keys are lowercase [a-z0-9._-]"),
            ));
        }
        if value.is_empty() {
            return Err(CatalogParseError::line(
                line_number,
                format!("empty value for `{key}`; delete the line to keep English"),
            ));
        }
        if value.len() > MAX_CATALOG_VALUE_BYTES {
            return Err(CatalogParseError::line(
                line_number,
                format!(
                    "value for `{key}` is {} bytes; cap is {MAX_CATALOG_VALUE_BYTES}",
                    value.len()
                ),
            ));
        }
        if !is_plain_display_text(value) {
            return Err(CatalogParseError::line(
                line_number,
                format!("value for `{key}` contains control, bidi, or invisible characters"),
            ));
        }

        // Unknown keys are accepted so newer translation files keep working
        // on older binaries; duplicates follow config semantics (last wins).
        entries.insert(key.to_string(), value.to_string());
    }

    Ok(TextCatalog {
        lang: Some(lang.to_string()),
        entries,
    })
}

/// The safety oracle is the display sanitizer itself: a value is accepted
/// only if sanitizing changes nothing, so this rule can never drift from
/// what the renderer escapes.
fn is_plain_display_text(value: &str) -> bool {
    let sanitized = DisplaySanitizer::unlimited_utf8().sanitize_line(value);
    !sanitized.has_non_text_segments() && sanitized.as_plain_text() == value
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(index) => &line[..index],
        None => line,
    }
}
