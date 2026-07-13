use dun_config::parse_catalog;

use super::support::*;

fn shell_with_zh_menus() -> UiShell {
    UiShell {
        catalog: parse_catalog(
            "menu.file = 文件\nmenu.file.new = 新建\nmenu.view.scroll-left = 左移视图\n",
            "zh-CN",
        )
        .expect("reference translation parses"),
        ..UiShell::default()
    }
}

#[test]
fn empty_catalog_keeps_english_labels_borrowed() {
    let shell = UiShell::default();
    let menu = shell.menu_bar(None);
    assert_eq!(menu.items[0].label, "File");
    assert_eq!(menu.items[0].entries[0].label, "New (N)");
    assert!(matches!(menu.items[0].label, std::borrow::Cow::Borrowed(_)));
}

#[test]
fn translated_labels_compose_the_english_mnemonic() {
    let shell = shell_with_zh_menus();
    let menu = shell.menu_bar(None);
    assert_eq!(menu.items[0].label, "文件 (F)");
    assert_eq!(menu.items[0].entries[0].label, "新建 (N)");
    // Punctuation mnemonics survive translation too.
    let scroll_left = &menu.items[2].entries[8].label;
    assert_eq!(scroll_left.as_ref(), "左移视图 ([)");
    // Untranslated siblings keep their English labels.
    assert_eq!(menu.items[1].label, "Edit");
    assert_eq!(menu.items[0].entries[1].label, "Open... (O)");
}

#[test]
fn mnemonics_keep_working_on_translated_labels() {
    let shell = shell_with_zh_menus();
    // Top level: "文件 (F)" must still answer to F, and the others to their
    // English first letters.
    assert_eq!(shell.menu_index_for_mnemonic('f'), Some(0));
    assert_eq!(shell.menu_index_for_mnemonic('F'), Some(0));
    assert_eq!(shell.menu_index_for_mnemonic('e'), Some(1));
    // Dropdown: "新建 (N)" answers to N.
    assert_eq!(shell.menu_entry_index_for_mnemonic(0, 'n'), Some(0));
    assert_eq!(shell.menu_entry_mnemonic(0, 0), Some('N'));
    // The first CJK char of a translated label is not a mnemonic.
    assert_eq!(shell.menu_index_for_mnemonic('文'), None);
}

#[test]
fn every_menu_key_in_the_reference_translation_exists() {
    // Guard against key drift between menu_bar() and i18n/zh-CN.conf: a
    // translated file must translate every menu it names, and translating
    // every key must translate every visible label.
    let text = include_str!("../../../../i18n/zh-CN.conf");
    let shell = UiShell {
        catalog: parse_catalog(text, "zh-CN").expect("shipped file parses"),
        ..UiShell::default()
    };
    let menu = shell.menu_bar(None);
    for item in &menu.items {
        assert!(
            matches!(item.label, std::borrow::Cow::Owned(_)),
            "menu label `{}` has no translation in i18n/zh-CN.conf",
            item.label
        );
        for entry in &item.entries {
            assert!(
                matches!(entry.label, std::borrow::Cow::Owned(_)),
                "entry label `{}` has no translation in i18n/zh-CN.conf",
                entry.label
            );
        }
    }
}
