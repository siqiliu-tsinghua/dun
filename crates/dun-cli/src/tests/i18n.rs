use std::fs;
use std::path::PathBuf;

use super::support::*;
use crate::config_loading::ConfigSource;
use crate::i18n_loading::{catalog_from_dir, i18n_dir_for_source};

fn temp_i18n_dir(name: &str) -> PathBuf {
    let dir = temp_file_path(name);
    fs::create_dir_all(&dir).expect("creates temp i18n dir");
    dir
}

#[test]
fn picks_the_most_specific_locale_candidate() {
    let dir = temp_i18n_dir("i18n-specific");
    fs::write(dir.join("zh-CN.conf"), "menu.file = 文件\n").unwrap();
    fs::write(dir.join("zh.conf"), "menu.file = 檔案\n").unwrap();

    let loaded = catalog_from_dir(&dir, "zh_CN.UTF-8");
    assert_eq!(loaded.catalog.get("menu.file"), Some("文件"));
    assert_eq!(loaded.catalog.lang(), Some("zh-CN"));
    assert!(loaded.diagnostic.is_none());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn falls_back_to_the_primary_subtag_file() {
    let dir = temp_i18n_dir("i18n-primary");
    fs::write(dir.join("zh.conf"), "menu.file = 檔案\n").unwrap();

    let loaded = catalog_from_dir(&dir, "zh_TW.Big5");
    assert_eq!(loaded.catalog.get("menu.file"), Some("檔案"));
    assert_eq!(loaded.catalog.lang(), Some("zh"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn missing_files_and_c_locale_stay_english_without_diagnostics() {
    let dir = temp_i18n_dir("i18n-missing");

    let loaded = catalog_from_dir(&dir, "de_DE.UTF-8");
    assert!(loaded.catalog.is_empty());
    assert!(loaded.diagnostic.is_none());

    let loaded = catalog_from_dir(&dir, "C");
    assert!(loaded.catalog.is_empty());
    assert!(loaded.diagnostic.is_none());

    let loaded = catalog_from_dir(&dir.join("does-not-exist"), "de_DE.UTF-8");
    assert!(loaded.catalog.is_empty());
    assert!(loaded.diagnostic.is_none());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn broken_specific_file_reports_instead_of_masking_itself() {
    let dir = temp_i18n_dir("i18n-broken");
    // The specific file smuggles a bidi override; the primary file is fine.
    fs::write(dir.join("zh-CN.conf"), "menu.file = a\u{202e}b\n").unwrap();
    fs::write(dir.join("zh.conf"), "menu.file = 檔案\n").unwrap();

    let loaded = catalog_from_dir(&dir, "zh_CN.UTF-8");
    assert!(loaded.catalog.is_empty(), "broken file must not half-load");
    let diagnostic = loaded.diagnostic.expect("reports the broken file");
    assert!(diagnostic.contains("zh-CN.conf"), "{diagnostic}");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn oversized_file_is_rejected_before_parsing() {
    let dir = temp_i18n_dir("i18n-oversized");
    let mut text = String::from("menu.file = ok\n");
    while text.len() <= dun_config::MAX_CATALOG_FILE_BYTES {
        text.push_str("# padding comment line to exceed the file cap\n");
    }
    fs::write(dir.join("ja.conf"), &text).unwrap();

    let loaded = catalog_from_dir(&dir, "ja_JP.UTF-8");
    assert!(loaded.catalog.is_empty());
    assert!(loaded.diagnostic.expect("reports the cap").contains("cap"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn i18n_dir_follows_the_config_source() {
    let config = PathBuf::from("/home/u/.config/dun/config");
    for source in [
        ConfigSource::Explicit(config.clone()),
        ConfigSource::Environment(config.clone()),
        ConfigSource::DefaultFile(config.clone()),
    ] {
        assert_eq!(
            i18n_dir_for_source(&source),
            Some(PathBuf::from("/home/u/.config/dun/i18n"))
        );
    }
    assert_eq!(i18n_dir_for_source(&ConfigSource::Disabled), None);
}
