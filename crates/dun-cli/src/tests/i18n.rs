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

#[test]
fn shipped_zh_catalog_translates_the_whole_help_window() {
    let text = include_str!("../../../../i18n/zh-CN.conf");
    let catalog = dun_config::parse_catalog(text, "zh-CN").expect("shipped file parses");

    let missing: Vec<String> = crate::help::content::help_translation_keys()
        .into_iter()
        .filter(|(key, _)| catalog.get(key).is_none())
        .map(|(key, english)| format!("{key} = {english}"))
        .collect();
    assert!(
        missing.is_empty(),
        "untranslated help keys:\n{}",
        missing.join("\n")
    );

    let help = crate::help::content::help_text(
        &dun_config::Keymap::default_editor(),
        &dun_config::FileDialogKeymap::default_file_dialog(),
        &catalog,
    );
    assert!(help.contains("帮助"), "translated help must render zh text");
    assert!(
        !help.contains("Move left"),
        "English descriptions leaked into translated help"
    );
    // Command ids stay untranslated: they are what the command prompt takes.
    assert!(help.contains("[edit.move_left]"));

    // Column alignment must hold by display width, not char count: the
    // translated "(未绑定)" key column is 10 cells wide but only 6 chars,
    // so char-count padding would shift its description right.
    let description_column = |line: &str, description: &str| {
        let start = line.find(description).expect("description in line");
        unicode_width::UnicodeWidthStr::width(&line[..start])
    };
    let bound = help
        .lines()
        .find(|line| line.contains("[app.help]"))
        .expect("bound command row");
    let unbound = help
        .lines()
        .find(|line| line.contains("[app.search_results]"))
        .expect("unbound command row");
    assert!(
        unbound.contains("(未绑定)"),
        "fixture expects an unbound row"
    );
    assert_eq!(
        description_column(bound, "帮助"),
        description_column(unbound, "列出当前搜索结果"),
        "description columns must align across ASCII and wide key columns"
    );
}

#[test]
fn shipped_zh_catalog_translates_all_dialog_chrome() {
    let text = include_str!("../../../../i18n/zh-CN.conf");
    let catalog = dun_config::parse_catalog(text, "zh-CN").expect("shipped file parses");

    let missing: Vec<String> = crate::ui_text::ALL
        .iter()
        .filter(|(key, _)| catalog.get(key).is_none())
        .map(|(key, english)| format!("{key} = {english}"))
        .collect();
    assert!(
        missing.is_empty(),
        "untranslated dialog keys:\n{}",
        missing.join("\n")
    );

    // Translated templates must keep the exact placeholder count of their
    // English default — mismatches silently fall back to English, so a
    // shipped translation with a mismatch is a bug, not a preference.
    let mismatched: Vec<String> = crate::ui_text::ALL
        .iter()
        .filter_map(|(key, english)| {
            let translated = catalog.get(key)?;
            (crate::ui_text::placeholder_count(translated)
                != crate::ui_text::placeholder_count(english))
            .then(|| format!("{key}: `{english}` vs `{translated}`"))
        })
        .collect();
    assert!(
        mismatched.is_empty(),
        "placeholder count mismatches:\n{}",
        mismatched.join("\n")
    );
}

#[test]
fn tr_fmt_substitutes_and_survives_broken_templates() {
    let catalog = dun_config::parse_catalog(
        "confirm.unsaved.body = {} 有未保存的更改\nconfirm.replace.match-of = 匹配太少\n",
        "zh-CN",
    )
    .expect("parses");

    // Correct placeholder count: translated template is used, args in order.
    assert_eq!(
        crate::ui_text::tr_fmt(&catalog, crate::ui_text::CONFIRM_UNSAVED_BODY, &["a.txt"]),
        "a.txt 有未保存的更改"
    );
    // Placeholder count mismatch (translation lost its {}s): the English
    // template wins so no runtime value is dropped.
    assert_eq!(
        crate::ui_text::tr_fmt(&catalog, crate::ui_text::CONFIRM_MATCH_OF, &["2", "5"]),
        "Match 2/5"
    );
    // Untranslated key: English template with substitution.
    assert_eq!(
        crate::ui_text::tr_fmt(&catalog, crate::ui_text::SWITCHER_OPEN_BUFFERS, &["3"]),
        "Open buffers: 3"
    );
}
