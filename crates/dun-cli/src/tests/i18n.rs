use std::collections::BTreeMap;
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

fn shipped_i18n_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../i18n")
}

fn shipped_zh_catalog() -> dun_config::TextCatalog {
    dun_config::parse_catalog(include_str!("../../../../i18n/zh-Hans.conf"), "zh-Hans")
        .expect("shipped file parses")
}

fn translation_defaults() -> BTreeMap<String, &'static str> {
    let mut defaults = BTreeMap::new();
    for &(key, english) in crate::ui_text::ALL {
        assert!(
            defaults.insert(key.to_string(), english).is_none(),
            "duplicate translation key: {key}"
        );
    }
    for (key, english) in crate::help::content::help_translation_keys() {
        assert!(
            defaults.insert(key.clone(), english).is_none(),
            "duplicate translation key: {key}"
        );
    }
    // The menu keys live in dun-ui and were the one surface this validator
    // could not see: a shipped translation could drop a menu label — the most
    // visible text in the editor — and only a hardcoded zh-Hans test would
    // notice, so any *new* language would ship with unvalidated menus.
    for (key, english) in dun_ui::menu_translation_keys() {
        assert!(
            defaults.insert(key.to_string(), english).is_none(),
            "duplicate translation key: {key}"
        );
    }
    defaults
}

fn shipped_catalog_files() -> Vec<PathBuf> {
    let dir = shipped_i18n_dir();
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()))
        .map(|entry| entry.expect("reads shipped i18n directory entry").path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("conf"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no shipped i18n/*.conf files found");
    files
}

fn menu_uses_translation_key(key: &str) -> bool {
    const MARKER: &str = "DUN_MENU_TRANSLATION_PROBE_9A7E";

    let mut app = AppState::new();
    app.shell.catalog = dun_config::parse_catalog(&format!("{key} = {MARKER}\n"), "test")
        .expect("probe catalog parses");
    let buffer_views = app.buffer_views();
    let frame =
        app.shell
            .frame_for_workspace(&app.workspace, Rect::new(0, 0, 80, 20), &buffer_views);
    frame.menu.items.iter().any(|item| {
        item.label.contains(MARKER)
            || item
                .entries
                .iter()
                .any(|entry| entry.label.contains(MARKER))
    })
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
fn shipped_script_catalog_resolves_simplified_but_not_traditional_chinese() {
    let dir = shipped_i18n_dir();
    for raw_locale in ["zh_CN.UTF-8", "zh_SG.UTF-8", "zh_MY.UTF-8", "zh"] {
        let loaded = catalog_from_dir(&dir, raw_locale);
        assert_eq!(loaded.catalog.lang(), Some("zh-Hans"), "{raw_locale}");
        assert_eq!(
            loaded.catalog.get("menu.file"),
            Some("文件"),
            "{raw_locale}"
        );
        assert!(loaded.diagnostic.is_none(), "{raw_locale}");
    }

    for raw_locale in ["zh_TW.UTF-8", "zh_HK.UTF-8", "zh_MO.UTF-8"] {
        let loaded = catalog_from_dir(&dir, raw_locale);
        assert!(loaded.catalog.is_empty(), "{raw_locale}");
        assert!(loaded.diagnostic.is_none(), "{raw_locale}");
    }
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
    let catalog = shipped_zh_catalog();

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
fn every_shipped_translation_is_valid_and_complete() {
    const DESTRUCTIVE_ACTION_KEYS: [&str; 3] = [
        "confirm.button.save",
        "confirm.button.discard",
        "confirm.button.cancel",
    ];

    let defaults = translation_defaults();
    for path in shipped_catalog_files() {
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        assert!(
            bytes.len() <= dun_config::MAX_CATALOG_FILE_BYTES,
            "{} is {} bytes; catalog cap is {} bytes",
            path.display(),
            bytes.len(),
            dun_config::MAX_CATALOG_FILE_BYTES
        );
        let text = std::str::from_utf8(&bytes)
            .unwrap_or_else(|error| panic!("{} is not UTF-8: {error}", path.display()));
        let lang = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_else(|| panic!("{} has no UTF-8 language tag", path.display()));
        let catalog = dun_config::parse_catalog(text, lang)
            .unwrap_or_else(|error| panic!("{} does not parse: {error}", path.display()));

        let missing: Vec<String> = defaults
            .iter()
            .filter(|(key, _)| catalog.get(key).is_none())
            .map(|(key, english)| format!("{key} = {english}"))
            .collect();
        assert!(
            missing.is_empty(),
            "{} is missing translation keys:\n{}",
            path.display(),
            missing.join("\n")
        );

        let mut unknown: Vec<&str> = catalog
            .keys()
            .filter(|key| !defaults.contains_key(*key) && !menu_uses_translation_key(key))
            .collect();
        unknown.sort_unstable();
        assert!(
            unknown.is_empty(),
            "{} has unknown translation keys:\n{}",
            path.display(),
            unknown.join("\n")
        );

        let mismatched: Vec<String> = catalog
            .keys()
            .filter_map(|key| {
                let translated = catalog.get(key).expect("enumerated catalog key exists");
                let english = defaults.get(key).copied();
                // Menu labels have no templates; their compiled English
                // labels are rendered directly with a separate mnemonic.
                let expected = english.map(crate::ui_text::placeholder_count).unwrap_or(0);
                let actual = crate::ui_text::placeholder_count(translated);
                (actual != expected).then(|| match english {
                    Some(english) => format!(
                        "{key}: expected {expected}, got {actual}; `{english}` vs `{translated}`"
                    ),
                    None => format!(
                        "{key}: expected {expected}, got {actual}; menu translation `{translated}`"
                    ),
                })
            })
            .collect();
        assert!(
            mismatched.is_empty(),
            "{} has placeholder count mismatches:\n{}",
            path.display(),
            mismatched.join("\n")
        );

        let labels = DESTRUCTIVE_ACTION_KEYS.map(|key| {
            let value = catalog.get(key).unwrap_or_else(|| {
                panic!(
                    "{} is missing destructive-action key `{key}`",
                    path.display()
                )
            });
            assert!(
                !value.is_empty(),
                "{} has an empty destructive-action label for `{key}`",
                path.display()
            );
            (key, value)
        });
        for left in 0..labels.len() {
            for right in left + 1..labels.len() {
                assert_ne!(
                    labels[left].1,
                    labels[right].1,
                    "{} destructive-action labels must be pairwise distinct; `{}` and `{}` are both `{}`",
                    path.display(),
                    labels[left].0,
                    labels[right].0,
                    labels[left].1
                );
            }
        }
    }
}

#[test]
fn ui_text_keys_are_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for (key, _) in crate::ui_text::ALL {
        assert!(seen.insert(*key), "duplicate ui text key: {key}");
    }
}

#[test]
fn prompt_cancel_status_uses_the_catalog_and_keeps_exact_english() {
    let mut english = AppState::new();
    english.handle_command(&EditorCommand::Edit(EditCommand::Find));
    english.handle_prompt_key_event(CrosstermKeyEvent::new(
        CrosstermKeyCode::Esc,
        CrosstermKeyModifiers::NONE,
    ));
    assert_eq!(english.status_message.as_deref(), Some("Find cancelled"));

    let mut chinese = AppState::new();
    chinese.shell.catalog = shipped_zh_catalog();
    chinese.handle_command(&EditorCommand::Edit(EditCommand::Find));
    chinese.handle_prompt_key_event(CrosstermKeyEvent::new(
        CrosstermKeyCode::Esc,
        CrosstermKeyModifiers::NONE,
    ));
    assert_eq!(chinese.status_message.as_deref(), Some("已取消查找"));
}

#[test]
fn empty_find_status_uses_the_catalog_and_keeps_exact_english() {
    let mut english = AppState::new();
    english.repeat_find(SearchDirection::Forward);
    assert_eq!(english.status_message.as_deref(), Some("Find: no query"));

    let mut chinese = AppState::new();
    chinese.shell.catalog = shipped_zh_catalog();
    chinese.repeat_find(SearchDirection::Forward);
    assert_eq!(
        chinese.status_message.as_deref(),
        Some("查找：没有查询内容")
    );
}

#[test]
fn window_status_uses_the_catalog_and_keeps_exact_english() {
    let mut english = AppState::new();
    english.handle_command(&EditorCommand::Window(WindowCommand::Equalize));
    assert_eq!(
        english.status_message.as_deref(),
        Some("Splits are already even")
    );

    let mut chinese = AppState::new();
    chinese.shell.catalog = shipped_zh_catalog();
    chinese.handle_command(&EditorCommand::Window(WindowCommand::Equalize));
    assert_eq!(chinese.status_message.as_deref(), Some("拆分已经均匀"));
}

#[test]
fn workspace_error_helper_uses_the_catalog_and_keeps_exact_english() {
    let mut english = AppState::new();
    english.sync_view_for_area(Rect::new(0, 0, 80, 20));
    english.handle_command(&EditorCommand::Window(WindowCommand::FocusLeft));
    assert_eq!(
        english.status_message.as_deref(),
        Some("Focus left failed: no neighboring pane")
    );

    let mut chinese = AppState::new();
    chinese.shell.catalog = shipped_zh_catalog();
    chinese.sync_view_for_area(Rect::new(0, 0, 80, 20));
    chinese.handle_command(&EditorCommand::Window(WindowCommand::FocusLeft));
    assert_eq!(
        chinese.status_message.as_deref(),
        Some("焦点左移失败：没有相邻窗格")
    );
}

#[test]
fn opened_file_helper_uses_the_catalog_and_keeps_exact_english() {
    let path = temp_file_path("i18n-opened-file.txt");
    fs::write(&path, "translated open\n").unwrap();

    let mut english = AppState::new();
    english.open_file_path(path.clone()).unwrap();
    assert_eq!(
        english.status_message,
        Some(format!("Opened {}", path.display()))
    );

    let mut chinese = AppState::new();
    chinese.shell.catalog = shipped_zh_catalog();
    chinese.open_file_path(path.clone()).unwrap();
    assert_eq!(
        chinese.status_message,
        Some(format!("已打开 {}", path.display()))
    );

    fs::remove_file(path).unwrap();
}

#[test]
fn file_dialog_message_renders_through_the_catalog_after_a_real_interaction() {
    let toggle_hidden =
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('h'), CrosstermKeyModifiers::CONTROL);

    let mut english = AppState::new();
    english.handle_command(&EditorCommand::File(FileCommand::Open));
    english.handle_file_dialog_key_event(toggle_hidden);
    assert!(
        english
            .active_overlay()
            .expect("English file dialog")
            .lines
            .iter()
            .any(|line| line == "Hidden files shown")
    );

    let mut chinese = AppState::new();
    chinese.shell.catalog = shipped_zh_catalog();
    chinese.handle_command(&EditorCommand::File(FileCommand::Open));
    chinese.handle_file_dialog_key_event(toggle_hidden);
    assert!(
        chinese
            .active_overlay()
            .expect("Chinese file dialog")
            .lines
            .iter()
            .any(|line| line == "已显示隐藏文件")
    );
}

#[test]
fn missing_path_error_uses_the_catalog_and_keeps_exact_english() {
    let path = temp_file_path("i18n-missing-open.txt");

    let mut english = AppState::new();
    english.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut english, &path.to_string_lossy());
    handle_key_event(
        &mut english,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );
    let english_status = format!("Open failed: {}: not found", path.display());
    assert_eq!(
        english.status_message.as_deref(),
        Some(english_status.as_str())
    );
    assert!(
        english
            .active_overlay()
            .expect("English file dialog remains open")
            .lines
            .iter()
            .any(|line| line == &english_status)
    );

    let mut chinese = AppState::new();
    chinese.shell.catalog = shipped_zh_catalog();
    chinese.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut chinese, &path.to_string_lossy());
    handle_key_event(
        &mut chinese,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );
    let chinese_status = format!("打开失败：{}：未找到", path.display());
    assert_eq!(
        chinese.status_message.as_deref(),
        Some(chinese_status.as_str())
    );
    assert!(
        chinese
            .active_overlay()
            .expect("Chinese file dialog remains open")
            .lines
            .iter()
            .any(|line| line == &chinese_status)
    );
}

#[test]
fn untitled_window_titles_use_the_catalog_at_startup_and_after_split() {
    let mut app = AppState::from_config_with_catalog(Config::default(), shipped_zh_catalog());
    assert_eq!(app.workspace.focused_window().unwrap().title, "无标题");

    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    assert_eq!(app.workspace.focused_window().unwrap().title, "无标题-2");
}

#[test]
fn command_output_buffer_content_uses_the_catalog() {
    let mut app = AppState::new();
    app.shell.catalog = shipped_zh_catalog();

    app.run_external_command_to_buffer("printf dun-i18n");

    let window = app.workspace.focused_window().unwrap();
    let text = app.buffer_state(window.buffer_id).unwrap().buffer.to_text();
    assert!(text.starts_with("Dun 命令输出\n\n命令：printf dun-i18n\n"));
    assert!(text.contains("状态：退出码 0\n"));
    assert!(text.contains("标准输出：8 字节，完整\n"));
    assert!(text.contains("已截断：否\n"));
    assert!(text.contains("--- 标准输出（8 字节，完整） ---\ndun-i18n\n"));
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

/// The one path-error consumer that does *not* go through the catalog: an
/// `io::Error` that escapes to the CLI's own `eprintln!` before there is an
/// editor at all (`dun /file/that/cannot/be/opened`). Its text comes from
/// `Display`, so `Display` needs a fence of its own — without this, changing
/// the separator or a detail word passes the whole suite.
#[test]
fn cli_path_error_display_is_pinned_english() {
    use std::io::ErrorKind;

    let error = crate::files::path_io_error(
        std::path::Path::new("/tmp/no-such-file"),
        std::io::Error::from(ErrorKind::NotFound),
    );
    assert_eq!(error.to_string(), "/tmp/no-such-file: not found");

    let error = crate::files::path_io_error(
        std::path::Path::new(""),
        std::io::Error::from(ErrorKind::PermissionDenied),
    );
    assert_eq!(error.to_string(), "(empty path): permission denied");
}
