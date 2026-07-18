#![cfg(unix)]
#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[test]
fn microsoft_edit_cli_reference_contract_when_available() -> io::Result<()> {
    let Some(edit) = command_on_path("edit") else {
        eprintln!("skipping Microsoft Edit CLI reference test: edit is not on PATH");
        return Ok(());
    };

    let output = Command::new(edit)
        .arg("--help")
        .stdin(Stdio::null())
        .output()?;
    let help = String::from_utf8_lossy(&output.stdout);
    // An `edit` on PATH that is not Microsoft Edit — e.g. FreeBSD's `/usr/bin/edit`,
    // which is `ee` — is not our reference: skip rather than fail against it.
    if !output.status.success() || !help.contains("Usage: edit") {
        eprintln!(
            "skipping Microsoft Edit CLI reference test: `edit` on PATH is not Microsoft Edit"
        );
        return Ok(());
    }
    assert!(
        help.contains("-h, --help") && help.contains("-v, --version"),
        "edit help did not expose help/version options\n{help}"
    );
    assert!(
        help.contains("FILE[:LINE[:COLUMN]]"),
        "edit help did not expose file:line:column argument\n{help}"
    );

    let output = Command::new(dun_binary()).arg("--help").output()?;
    assert!(
        output.status.success(),
        "dun --help failed with status {:?}",
        output.status
    );
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(
        help.contains("dun [OPTIONS] [--] [PATH]"),
        "dun help did not contain expected usage\n{help}"
    );
    assert!(
        help.contains("-h, --help") && help.contains("-V, --version"),
        "dun help did not expose help/version options\n{help}"
    );

    Ok(())
}

#[test]
fn microsoft_edit_static_menu_reference_matches_expected_groups_and_shortcuts() -> io::Result<()> {
    let Some(root) = reference_root() else {
        eprintln!("skipping Microsoft Edit static menu reference test: reference/msedit missing");
        return Ok(());
    };
    let menubar = fs::read_to_string(root.join("crates/edit/src/bin/edit/draw_menubar.rs"))?;
    let localization = fs::read_to_string(root.join("i18n/edit.toml"))?;

    for marker in [
        "menubar_menu_begin(loc(LocId::File), 'F')",
        "menubar_menu_begin(loc(LocId::Edit), 'E')",
        "menubar_menu_begin(loc(LocId::View), 'V')",
        "menubar_menu_begin(loc(LocId::Help), 'H')",
        "menubar_menu_button(loc(LocId::FileNew), 'N', kbmod::CTRL | vk::N)",
        "menubar_menu_button(loc(LocId::FileOpen), 'O', kbmod::CTRL | vk::O)",
        "menubar_menu_button(loc(LocId::FileSave), 'S', kbmod::CTRL | vk::S)",
        "menubar_menu_button(loc(LocId::FileSaveAs), 'A', vk::NULL)",
        "menubar_menu_button(loc(LocId::FileClose), 'C', kbmod::CTRL | vk::W)",
        "menubar_menu_button(loc(LocId::FileExit), 'X', kbmod::CTRL | vk::Q)",
        "menubar_menu_button(loc(LocId::EditUndo), 'U', kbmod::CTRL | vk::Z)",
        "menubar_menu_button(loc(LocId::EditRedo), 'R', kbmod::CTRL | vk::Y)",
        "menubar_menu_button(loc(LocId::EditCut), 'T', kbmod::CTRL | vk::X)",
        "menubar_menu_button(loc(LocId::EditCopy), 'C', kbmod::CTRL | vk::C)",
        "menubar_menu_button(loc(LocId::EditPaste), 'P', kbmod::CTRL | vk::V)",
        "menubar_menu_button(loc(LocId::EditFind), 'F', kbmod::CTRL | vk::F)",
        "menubar_menu_button(loc(LocId::EditReplace), 'L', kbmod::CTRL | vk::R)",
        "menubar_menu_button(loc(LocId::EditSelectAll), 'A', kbmod::CTRL | vk::A)",
        "menubar_menu_button(loc(LocId::ViewFocusStatusbar), 'S', vk::NULL)",
        "menubar_menu_button(loc(LocId::ViewGoToFile), 'F', kbmod::CTRL | vk::P)",
        "menubar_menu_button(loc(LocId::FileGoto), 'G', kbmod::CTRL | vk::G)",
        "menubar_menu_checkbox(loc(LocId::ViewWordWrap), 'W', kbmod::ALT | vk::Z",
        "menubar_menu_button(loc(LocId::HelpAbout), 'A', vk::NULL)",
    ] {
        assert!(
            menubar.contains(marker),
            "Microsoft Edit menu source missing marker: {marker}"
        );
    }

    for marker in [
        "[File]",
        "en = \"File\"",
        "[FileOpen]",
        "[Edit]",
        "en = \"Edit\"",
        "[View]",
        "en = \"View\"",
        "[Help]",
        "en = \"Help\"",
    ] {
        assert!(
            localization.contains(marker),
            "Microsoft Edit localization missing marker: {marker}"
        );
    }

    Ok(())
}

#[test]
fn microsoft_edit_static_statusbar_reference_matches_expected_fields() -> io::Result<()> {
    let Some(root) = reference_root() else {
        eprintln!(
            "skipping Microsoft Edit static statusbar reference test: reference/msedit missing"
        );
        return Ok(());
    };
    let statusbar = fs::read_to_string(root.join("crates/edit/src/bin/edit/draw_statusbar.rs"))?;

    for marker in [
        "ctx.table_begin(\"statusbar\")",
        "state.menubar_color_bg",
        "state.menubar_color_fg",
        "\"language\"",
        "tb.language().map_or(\"Plain Text\"",
        "ctx.button(\"newline\", if tb.is_crlf() { \"CRLF\" } else { \"LF\" }",
        "ctx.button(\"encoding\", tb.encoding()",
        "\"indentation\"",
        "ctx.label(\n            \"location\"",
        "tb.cursor_logical_pos().y + 1",
        "tb.cursor_logical_pos().x + 1",
        "ctx.label(\"dirty\", \"*\")",
        "\"filename\", filename",
        "Overflow::TruncateMiddle",
    ] {
        assert!(
            statusbar.contains(marker),
            "Microsoft Edit statusbar source missing marker: {marker}"
        );
    }

    Ok(())
}

#[test]
fn microsoft_edit_static_color_and_terminal_setup_reference_is_documented() -> io::Result<()> {
    let Some(root) = reference_root() else {
        eprintln!("skipping Microsoft Edit static color reference test: reference/msedit missing");
        return Ok(());
    };
    let main = fs::read_to_string(root.join("crates/edit/src/bin/edit/main.rs"))?;
    let state = fs::read_to_string(root.join("crates/edit/src/bin/edit/state.rs"))?;

    for marker in [
        "state.menubar_color_bg = tui.indexed(IndexedColor::Background).oklab_blend",
        "IndexedColor::BrightBlue",
        "state.menubar_color_fg = tui.contrasted(state.menubar_color_bg)",
        "tui.set_floater_default_bg(floater_bg)",
        "tui.set_modal_default_bg(floater_bg)",
        "\\x1b[?1049h\\x1b[?1002;1006;2004h\\x1b[?1036h",
        "\\x1b]4;0;?;1;?",
        "\\x1b]10;?\\x07\\x1b]11;?\\x07",
        "\\r\u{2026}\\x1b[6n",
        "\\x1b[c",
        "tui.setup_indexed_colors(indexed_colors)",
    ] {
        assert!(
            main.contains(marker),
            "Microsoft Edit main source missing marker: {marker}"
        );
    }

    for marker in [
        "pub menubar_color_bg: StraightRgba",
        "pub menubar_color_fg: StraightRgba",
        "pub wants_file_picker: StateFilePicker",
        "pub wants_search: StateSearch",
        "pub wants_statusbar_focus: bool",
        "pub wants_encoding_picker: bool",
    ] {
        assert!(
            state.contains(marker),
            "Microsoft Edit state source missing marker: {marker}"
        );
    }

    Ok(())
}

fn dun_binary() -> &'static OsStr {
    OsStr::new(env!("CARGO_BIN_EXE_dun"))
}

fn command_on_path(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    env::split_paths(&paths)
        .map(|path| path.join(name))
        .find(|path| path.is_file())
}

fn reference_root() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("reference/msedit");
    root.is_dir().then_some(root)
}
