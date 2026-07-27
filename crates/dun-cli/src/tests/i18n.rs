use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use dun_plugin::json::{self, Json};
use dun_plugin::{GrantedCapabilities, PluginMenu, Role, TrustClass};

use super::support::*;
use crate::config_loading::ConfigSource;
use crate::i18n_loading::{catalog_from_dir, i18n_dir_for_source};
use crate::plugins::PluginHost;

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

fn accepted_plugin_menu_host() -> PluginHost {
    let payload = json::obj([
        ("top_label", json::obj([("en_US", json::str("Tools"))])),
        (
            "items",
            Json::Arr(vec![json::obj([
                ("label", json::obj([("en_US", json::str("Run"))])),
                ("action_id", json::str("run")),
            ])]),
        ),
    ]);
    let menu = PluginMenu::from_payload(&payload).expect("valid menu payload");
    let granted =
        GrantedCapabilities::for_roles(&[Role::LogFilter], TrustClass::UserTrustedExternal);
    let (mut host, _messages, events) = PluginHost::for_tests_granted("tools", granted);
    events
        .send(HostEvent::Started {
            menu: Some(menu),
            keybinding: None,
        })
        .unwrap();
    assert!(host.poll().is_empty(), "handshake events are absorbed");
    host
}

fn rendered_top_level_menu_labels(app: &AppState) -> Vec<String> {
    let buffer_views = app.buffer_views();
    app.shell
        .frame_for_workspace(&app.workspace, Rect::new(0, 0, 120, 20), &buffer_views)
        .menu
        .items
        .into_iter()
        .map(|item| item.label.into_owned())
        .collect()
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

/// The script step is the whole reason a Singaporean reader stops getting
/// English, and the whole reason a Taipei reader does not get Simplified
/// characters. Pin both halves: every Simplified locale reaches `zh-Hans`,
/// every Traditional locale reaches `zh-Hant`, and neither ever reaches the
/// other — a regression in the script table would otherwise show up as
/// nothing worse than "the wrong Chinese", which no other test would catch.
#[test]
fn shipped_script_catalogs_route_each_locale_to_its_own_script() {
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
        assert_eq!(loaded.catalog.lang(), Some("zh-Hant"), "{raw_locale}");
        // Not merely Traditional characters: Taiwan says 檔案 for "File",
        // where the mainland says 文件. A character conversion of zh-Hans
        // would put 文件 here and pass every other check.
        assert_eq!(
            loaded.catalog.get("menu.file"),
            Some("檔案"),
            "{raw_locale}"
        );
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
        AmbiguousWidth::Narrow,
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
        str_width(&line[..start], AmbiguousWidth::Narrow)
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
fn shipped_zh_config_diagnostics_jump_finds_the_translated_heading() {
    let mut app = AppState::new();
    app.shell.catalog = shipped_zh_catalog();

    app.jump_config_diagnostics_section(ConfigDiagnosticsSection::FileDialogKeymap);

    let window = app.workspace.focused_window().unwrap();
    assert_eq!(window.kind, WindowKind::ConfigDiagnostics);
    let buffer = app.buffer_state(window.buffer_id).unwrap();
    assert_eq!(
        buffer.buffer.line(buffer.buffer.cursor_position().line),
        Some("文件对话框快捷键")
    );
}

#[test]
fn shipped_zh_config_diagnostics_body_has_no_english_headings() {
    let mut app = AppState::new();
    app.shell.catalog = shipped_zh_catalog();

    app.open_config_diagnostics_screen();

    let window = app.workspace.focused_window().unwrap();
    let text = app.buffer_state(window.buffer_id).unwrap().buffer.to_text();
    for translated in [
        "Dun 配置诊断",
        "摘要",
        "路径",
        "来源",
        "终端",
        "输入",
        "剪贴板",
        "限制",
        "快捷键",
        "文件对话框快捷键",
    ] {
        assert!(
            text.lines().any(|line| line == translated),
            "missing translated heading `{translated}`"
        );
    }
    for english in [
        "Dun Config Diagnostics",
        "Summary",
        "Paths",
        "Source",
        "Terminal",
        "Input",
        "Clipboard",
        "Limits",
        "Keymap",
        "File Dialog Keymap",
    ] {
        assert!(
            !text.contains(english),
            "English heading leaked into translated diagnostics: `{english}`"
        );
    }
}

#[test]
fn empty_catalog_keeps_diagnostic_window_body_headings_exactly_english() {
    let mut app = AppState::new();

    app.open_config_diagnostics_screen();

    let window = app.workspace.focused_window().unwrap();
    let text = app.buffer_state(window.buffer_id).unwrap().buffer.to_text();
    let expected = [
        "Dun Config Diagnostics",
        "Summary",
        "Paths",
        "Source",
        "Terminal",
        "Input",
        "Clipboard",
        "Limits",
        "Keymap",
        "File Dialog Keymap",
    ];
    let actual = text
        .lines()
        .filter(|line| expected.contains(line))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);

    assert_eq!(
        AppState::new().status_history_text(),
        "Dun Status History\n\nNo status messages yet.\n"
    );
}

/// The severity tags are catalog-driven now, so the English defaults need a
/// test of their own: `StatusLevel::label()` used to hold them and is gone.
#[test]
fn empty_catalog_keeps_status_history_levels_exactly_english() {
    let mut app = AppState::new();
    app.set_status("Opened sample.txt");
    app.set_status("Save failed: disk full");

    let text = app.status_history_text();

    assert!(text.contains("[info] Opened sample.txt"), "{text}");
    assert!(text.contains("[error] Save failed: disk full"), "{text}");
}

#[test]
fn shipped_zh_status_history_body_translates_heading_and_levels() {
    let mut app = AppState::new();
    app.shell.catalog = shipped_zh_catalog();
    app.set_status("Opened sample.txt");
    app.set_status("Save failed: disk full");

    let text = app.status_history_text();

    assert!(text.starts_with("Dun 状态历史\n\n"));
    assert!(text.contains("[信息] Opened sample.txt"));
    assert!(text.contains("[错误] Save failed: disk full"));
    assert!(!text.contains("Dun Status History"));
    assert!(!text.contains("[info]"));
    assert!(!text.contains("[error]"));
}

/// The empty-history line is the one full sentence in this window that brief
/// 054 did not cover; it is catalog-driven now like the rest of the body.
#[test]
fn shipped_zh_status_history_translates_the_empty_message() {
    let mut app = AppState::new();
    app.shell.catalog = shipped_zh_catalog();

    let text = app.status_history_text();

    assert_eq!(text, "Dun 状态历史\n\n暂无状态消息。\n");
    assert!(!text.contains("No status messages yet."));
}

#[test]
fn plugin_menu_resolution_preserves_builtin_labels_and_indices() {
    const BUILT_IN_MENU_COUNT: usize = 4;

    for path in shipped_catalog_files() {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let lang = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_else(|| panic!("{} has no UTF-8 language tag", path.display()));
        let catalog = dun_config::parse_catalog(&text, lang)
            .unwrap_or_else(|error| panic!("{} does not parse: {error}", path.display()));

        let mut before = AppState::new();
        before.shell.catalog = catalog.clone();
        let before_labels = rendered_top_level_menu_labels(&before);
        assert_eq!(before_labels.len(), BUILT_IN_MENU_COUNT);
        for (index, mnemonic) in ['F', 'E', 'V', 'H'].into_iter().enumerate() {
            assert_eq!(
                before.shell.menu_index_for_mnemonic(mnemonic),
                Some(index),
                "{}: built-in mnemonic {mnemonic} moved before plugin resolution",
                path.display()
            );
        }

        let mut after = AppState::new();
        after.shell.catalog = catalog;
        after.plugin_hosts = PluginHosts::for_tests(vec![accepted_plugin_menu_host()]);
        after.refresh_plugin_contributions();
        let after_labels = rendered_top_level_menu_labels(&after);

        assert_eq!(
            after_labels[..BUILT_IN_MENU_COUNT],
            before_labels,
            "{}: adding an accepted plugin must leave the built-in prefix byte-identical",
            path.display()
        );
        assert_eq!(after_labels[BUILT_IN_MENU_COUNT], "Tools");
        for (index, mnemonic) in ['F', 'E', 'V', 'H'].into_iter().enumerate() {
            assert_eq!(
                after.shell.menu_index_for_mnemonic(mnemonic),
                Some(index),
                "{}: built-in mnemonic {mnemonic} moved after plugin resolution",
                path.display()
            );
        }
    }
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
                // A translation may use indexed {0}/{1} to reorder arguments
                // into its own word order; it may not skip one, invent one, or
                // mix the two forms.
                let valid = crate::ui_text::indexed_template_is_valid(translated, expected);
                let actual = crate::ui_text::placeholder_count(translated);
                (!valid).then(|| match english {
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
    english.handle_prompt_key_event(TerminalKeyEvent::new(
        TerminalKeyCode::Esc,
        TerminalKeyModifiers::NONE,
    ));
    assert_eq!(english.status_message.as_deref(), Some("Find cancelled"));

    let mut chinese = AppState::new();
    chinese.shell.catalog = shipped_zh_catalog();
    chinese.handle_command(&EditorCommand::Edit(EditCommand::Find));
    chinese.handle_prompt_key_event(TerminalKeyEvent::new(
        TerminalKeyCode::Esc,
        TerminalKeyModifiers::NONE,
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
        TerminalKeyEvent::new(TerminalKeyCode::Char('h'), TerminalKeyModifiers::CONTROL);

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
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
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
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
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

/// Japanese and Korean are verb-final and Russian word order is free, so a
/// translation of a multi-argument template must be able to put the arguments
/// where its grammar wants them. Positional `{}` cannot express that — this is
/// what indexed `{N}` is for, and what the validator has to police.
#[test]
fn indexed_placeholders_let_a_translation_reorder_arguments() {
    use crate::ui_text::{indexed_template_is_valid, placeholder_count, substitute};

    // Positional: English order, filled left to right (unchanged behaviour).
    assert_eq!(
        substitute("Find: {}/{} for {}", &["2", "7", "fn"]),
        "Find: 2/7 for fn"
    );

    // Indexed: the query comes first, the counts after — the whole point.
    assert_eq!(
        substitute("検索：{2} — {0}/{1} 件目", &["2", "7", "fn"]),
        "検索：fn — 2/7 件目"
    );
    // An index may repeat.
    assert_eq!(substitute("{0} → {0}", &["x"]), "x → x");

    // Arity is the highest index plus one, so the validator compares like
    // with like across the two forms.
    assert_eq!(placeholder_count("{}/{} {}"), 3);
    assert_eq!(placeholder_count("{2} {0} {1}"), 3);

    // What must be rejected: a skipped argument (silently drops a runtime
    // value), an out-of-range index, and a mix of the two forms.
    assert!(indexed_template_is_valid("{2} {0} {1}", 3));
    assert!(!indexed_template_is_valid("{0} {2}", 3), "skips index 1");
    assert!(
        !indexed_template_is_valid("{0} {1} {9}", 3),
        "index out of range"
    );
    assert!(
        !indexed_template_is_valid("{0} and {}", 2),
        "mixes the two forms"
    );
    assert!(
        !indexed_template_is_valid("{}/{}", 3),
        "wrong positional arity"
    );
}

/// A translation that breaks the placeholder rules must degrade to English,
/// never to nonsense or to a dropped value.
#[test]
fn a_broken_indexed_template_falls_back_to_english() {
    let catalog = dun_config::parse_catalog(
        // Skips {1}: the total would silently vanish from the status line.
        "status.replace.match-of = 一致 {0}\n",
        "test",
    )
    .expect("parses");
    assert_eq!(
        crate::ui_text::tr_fmt(&catalog, crate::ui_text::CONFIRM_MATCH_OF, &["2", "7"]),
        "Match 2/7"
    );
}
