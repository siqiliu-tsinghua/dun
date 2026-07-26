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
    let scroll_left = &menu.items[2]
        .entries
        .iter()
        .find(|entry| entry.command == EditorCommand::Edit(dun_core::EditCommand::ScrollLeft))
        .expect("View menu contains Scroll Left")
        .label;
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
    // Guard against key drift between menu_bar() and i18n/zh-Hans.conf: a
    // translated file must translate every menu it names, and translating
    // every key must translate every visible label.
    let text = include_str!("../../../../i18n/zh-Hans.conf");
    let shell = UiShell {
        catalog: parse_catalog(text, "zh-Hans").expect("shipped file parses"),
        ..UiShell::default()
    };
    let menu = shell.menu_bar(None);
    for item in &menu.items {
        assert!(
            matches!(item.label, std::borrow::Cow::Owned(_)),
            "menu label `{}` has no translation in i18n/zh-Hans.conf",
            item.label
        );
        for entry in &item.entries {
            assert!(
                matches!(entry.label, std::borrow::Cow::Owned(_)),
                "entry label `{}` has no translation in i18n/zh-Hans.conf",
                entry.label
            );
        }
    }
}

#[test]
fn translated_menus_never_overflow_a_narrow_terminal() {
    // Translated labels are often wider than English ("新建" costs four
    // cells before its mnemonic) and a translation value can be far longer
    // than the English label ever suggested. Nothing may leak past the
    // terminal edge, and the cut must land on display-width boundaries
    // (user-raised risk 2026-07-13).
    let shell = UiShell {
        catalog: parse_catalog(
            "menu.file = 文件\nmenu.file.new = 新建一个非常非常非常非常长的空缓冲区\nmenu.view = 视图与窗口管理\n",
            "zh-CN",
        )
        .expect("parses"),
        ..UiShell::default()
    };
    let (width, height) = (34u16, 12u16);
    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let frame = shell.frame_for_workspace_with_menu_selection(
        &workspace,
        Rect::new(0, 0, width, height),
        &[buffer_view],
        Some(MenuSelection::menu_only(0)),
    );
    let mut surface = crate::surface::Surface::new(width, height, shell.theme.palette.editor);
    crate::render::surface_frame::render_ui_frame_to_surface(&mut surface, &shell, &frame);

    for y in 0..height {
        let row = surface.row_text(y);
        assert!(
            display_width(row.trim_end(), shell.profile.ambiguous_width) <= width as usize,
            "row {y} overflows {width} cols: {row:?}"
        );
    }
    let all: String = (0..height).map(|y| surface.row_text(y)).collect();
    let truncation = shell.glyphs.indicators.truncation;
    assert!(
        all.contains(truncation),
        "an over-long label must truncate visibly"
    );
    assert!(
        all.contains("新建一个"),
        "the truncated label must keep its head"
    );
}

#[test]
fn rightmost_translated_menu_shifts_on_screen_instead_of_vanishing() {
    // At 26 columns the translated menu bar cuts off before 帮助, but its
    // mnemonic still opens it; the dropdown must shift left onto the screen
    // rather than being clamped into nothing.
    let text = include_str!("../../../../i18n/zh-Hans.conf");
    let shell = UiShell {
        catalog: parse_catalog(text, "zh-Hans").expect("shipped file parses"),
        ..UiShell::default()
    };
    let area = Rect::new(0, 0, 26, 10);
    let menu = shell.menu_bar(Some(MenuSelection::menu_only(3)));
    let rect = clamp_menu_rect(dropdown_rect_for_menu(&shell, &menu, 3).unwrap(), area)
        .expect("the dropdown must survive clamping");
    assert!(
        rect.x.saturating_add(rect.width) <= area.width,
        "dropdown {rect:?} must sit fully inside {area:?}"
    );

    let workspace = Workspace::new_untitled();
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, "body");
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let frame = shell.frame_for_workspace_with_menu_selection(
        &workspace,
        area,
        &[buffer_view],
        Some(MenuSelection::menu_only(3)),
    );
    let mut surface = crate::surface::Surface::new(26, 10, shell.theme.palette.editor);
    crate::render::surface_frame::render_ui_frame_to_surface(&mut surface, &shell, &frame);
    let dropdown_rows: String = (1..10).map(|y| surface.row_text(y)).collect();
    assert!(
        dropdown_rows.contains("帮助 ("),
        "the shifted dropdown must actually render its entry"
    );

    // Hit testing shares the shifted geometry: a click inside the visible
    // dropdown runs the entry.
    assert_eq!(
        shell.menu_entry_command_at_in_area(
            MenuSelection::menu_only(3),
            rect.x + 2,
            rect.y + 1,
            area
        ),
        Some(EditorCommand::App(AppCommand::Help))
    );
}
