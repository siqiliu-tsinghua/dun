use crate::i18n::{MAX_CATALOG_VALUE_BYTES, TextCatalog, locale_candidates, parse_catalog};

#[test]
fn empty_catalog_translates_nothing() {
    let catalog = TextCatalog::empty();
    assert!(catalog.is_empty());
    assert_eq!(catalog.lang(), None);
    assert_eq!(catalog.get("menu.file"), None);
}

#[test]
fn locale_candidates_cover_region_and_primary() {
    assert_eq!(locale_candidates("zh_CN.UTF-8"), vec!["zh-CN", "zh"]);
    assert_eq!(locale_candidates("zh-cn"), vec!["zh-CN", "zh"]);
    assert_eq!(locale_candidates("de_DE@euro"), vec!["de-DE", "de"]);
    assert_eq!(locale_candidates("ja"), vec!["ja"]);
    assert_eq!(locale_candidates("JA.eucJP"), vec!["ja"]);
}

#[test]
fn locale_candidates_reject_c_posix_and_junk() {
    assert!(locale_candidates("").is_empty());
    assert!(locale_candidates("C").is_empty());
    assert!(locale_candidates("C.UTF-8").is_empty());
    assert!(locale_candidates("POSIX").is_empty());
    assert!(locale_candidates("123").is_empty());
    assert!(locale_candidates("..").is_empty());
}

#[test]
fn parses_comments_blanks_and_last_wins() {
    let catalog = parse_catalog(
        "# reference translation\n\nmenu.file = 文件 # trailing note\nmenu.file = 檔案\n",
        "zh-CN",
    )
    .expect("parses");
    assert_eq!(catalog.lang(), Some("zh-CN"));
    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog.get("menu.file"), Some("檔案"));
}

#[test]
fn accepts_unknown_keys_for_forward_compatibility() {
    let catalog = parse_catalog("some.future.key = text\n", "de").expect("parses");
    assert_eq!(catalog.get("some.future.key"), Some("text"));
}

#[test]
fn rejects_missing_equals_bad_keys_and_empty_values() {
    assert!(parse_catalog("menu.file\n", "zh-CN").is_err());
    assert!(parse_catalog("Menu.File = x\n", "zh-CN").is_err());
    assert!(parse_catalog("menu file = x\n", "zh-CN").is_err());
    assert!(parse_catalog("menu.file =\n", "zh-CN").is_err());
    assert!(parse_catalog("menu.file = # only a comment\n", "zh-CN").is_err());
}

#[test]
fn rejects_oversized_values() {
    let value = "х".repeat(MAX_CATALOG_VALUE_BYTES);
    let input = format!("menu.file = {value}\n");
    assert!(parse_catalog(&input, "uk").is_err());
}

#[test]
fn rejects_control_escape_bidi_and_invisible_values() {
    // The oracle is the display sanitizer: anything it would escape is
    // rejected whole-file. One representative per class the sanitizer
    // hardening covered.
    let hostile = [
        "menu.file = a\u{0007}b\n",    // C0 control (BEL)
        "menu.file = a\u{001b}[31m\n", // ESC
        "menu.file = a\u{0085}b\n",    // C1 control
        "menu.file = a\tb\n",          // tab is a control in a label
        "menu.file = a\u{202e}b\n",    // bidi override (Trojan Source)
        "menu.file = a\u{200b}b\n",    // zero-width space
        "menu.file = a\u{feff}b\n",    // BOM / zero-width no-break space
    ];
    for input in hostile {
        assert!(
            parse_catalog(input, "zh-CN").is_err(),
            "must reject {input:?}"
        );
    }
}

#[test]
fn error_reports_line_number() {
    let error =
        parse_catalog("menu.file = ok\nbroken line\n", "zh-CN").expect_err("rejects line 2");
    assert!(error.to_string().starts_with("line 2:"), "{error}");
}
