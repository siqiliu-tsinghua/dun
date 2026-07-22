#![allow(unused_imports)]

use super::support::*;

#[test]
fn open_dialog_reuses_recent_successful_directory() {
    let directory = temp_file_path("open-dialog-recent");
    let path = directory.join("recent.txt");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(&path, "opened").unwrap();
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &path.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );
    app.handle_command(&EditorCommand::File(FileCommand::Open));

    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Open: {}/", directory.display()))
    );

    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn open_dialog_enters_directory_path() {
    let path = temp_file_path("open-dialog-dir");
    std::fs::create_dir(&path).unwrap();
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &path.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Open: {}/", path.display()))
    );
    assert_eq!(app.status_message, None);
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");

    let _ = std::fs::remove_dir(path);
}

#[test]
fn open_dialog_tab_completes_unique_file_path() {
    let directory = temp_file_path("open-dialog-tab");
    let path = directory.join("alpha.log");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(&path, "opened").unwrap();
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/al", directory.display()));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Open: {}", path.display()))
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "opened");
    assert_eq!(state.path.as_ref(), Some(&path));

    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn file_dialog_path_input_cursor_edits_middle_of_path() {
    let directory = temp_file_path("file-dialog-cursor");
    std::fs::create_dir(&directory).unwrap();
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/ab", directory.display()));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::NONE),
    );
    send_text(&mut app, "X");
    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Open: {}/aXb", directory.display()))
    );
    assert_eq!(
        app.file_dialog
            .as_ref()
            .map(|dialog| dialog.input.cursor_index),
        Some(format!("{}/aX", directory.display()).len())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Backspace, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Open: {}/ab", directory.display()))
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Delete, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Open: {}/a", directory.display()))
    );

    let _ = std::fs::remove_dir(directory);
}

#[test]
fn file_dialog_path_input_home_end_and_utf8_cursor_are_safe() {
    let directory = temp_file_path("file-dialog-home-end");
    std::fs::create_dir(&directory).unwrap();
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
    send_text(&mut app, &format!("{}/中b", directory.display()));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Left, CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Backspace, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Save As: {}/b", directory.display()))
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Home, CrosstermKeyModifiers::NONE),
    );
    send_text(&mut app, "~");
    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Save As: ~{}/b", directory.display()))
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::End, CrosstermKeyModifiers::NONE),
    );
    send_text(&mut app, "!");
    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Save As: ~{}/b!", directory.display()))
    );

    let _ = std::fs::remove_dir(directory);
}

#[test]
fn open_dialog_down_enter_opens_selected_file() {
    let directory = temp_file_path("open-dialog-select");
    let first = directory.join("a.txt");
    let second = directory.join("b.txt");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(&first, "first").unwrap();
    std::fs::write(&second, "second").unwrap();
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/", directory.display()));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "second");
    assert_eq!(state.path.as_ref(), Some(&second));

    let _ = std::fs::remove_file(first);
    let _ = std::fs::remove_file(second);
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn open_dialog_page_keys_move_selection_and_scroll() {
    let directory = temp_file_path("open-dialog-page");
    std::fs::create_dir(&directory).unwrap();
    for index in 0..20 {
        std::fs::write(
            directory.join(format!("item{index:02}.txt")),
            format!("item{index:02}"),
        )
        .unwrap();
    }
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/", directory.display()));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::PageDown, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.file_dialog
            .as_ref()
            .and_then(|dialog| dialog.selected_index),
        Some(11)
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::PageDown, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.file_dialog
            .as_ref()
            .and_then(|dialog| dialog.selected_index),
        Some(20)
    );
    assert_eq!(
        app.file_dialog.as_ref().map(|dialog| dialog.scroll_offset),
        Some(9)
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::PageUp, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.file_dialog
            .as_ref()
            .and_then(|dialog| dialog.selected_index),
        Some(9)
    );
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::PageUp, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.file_dialog
            .as_ref()
            .and_then(|dialog| dialog.selected_index),
        Some(0)
    );
    assert_eq!(
        app.file_dialog.as_ref().map(|dialog| dialog.scroll_offset),
        Some(0)
    );

    for index in 0..20 {
        let _ = std::fs::remove_file(directory.join(format!("item{index:02}.txt")));
    }
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn file_dialog_parent_entry_is_first_and_enters_parent_directory() {
    let directory = temp_file_path("file-dialog-parent");
    let child = directory.join("child");
    std::fs::create_dir(&directory).unwrap();
    std::fs::create_dir(&child).unwrap();
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/", child.display()));

    let dialog = app.file_dialog.as_ref().unwrap();
    assert_eq!(
        dialog.entries.first().map(|entry| entry.name.as_str()),
        Some("..")
    );
    assert_eq!(
        dialog.entries.first().map(|entry| entry.is_parent),
        Some(true)
    );

    app.click_file_dialog_visible_index(0);

    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Open: {}/", directory.display()))
    );
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");

    let _ = std::fs::remove_dir(child);
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn file_dialog_hides_dotfiles_until_prefix_or_toggle() {
    let directory = temp_file_path("file-dialog-hidden");
    let hidden = directory.join(".secret");
    let visible = directory.join("visible.txt");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(&hidden, "hidden").unwrap();
    std::fs::write(&visible, "visible").unwrap();
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/", directory.display()));

    let entry_names = app
        .file_dialog
        .as_ref()
        .unwrap()
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();
    assert!(entry_names.contains(&".."));
    assert!(entry_names.contains(&"visible.txt"));
    assert!(!entry_names.contains(&".secret"));

    send_text(&mut app, ".");
    let entry_names = app
        .file_dialog
        .as_ref()
        .unwrap()
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();
    assert!(entry_names.contains(&".secret"));

    let mut app = AppState::new();
    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/", directory.display()));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('h'), CrosstermKeyModifiers::CONTROL),
    );

    let dialog = app.file_dialog.as_ref().unwrap();
    let entry_names = dialog
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();
    assert!(dialog.show_hidden);
    assert!(entry_names.contains(&".secret"));
    assert_eq!(
        dialog.message,
        Some(FileDialogMessage::HiddenFiles { shown: true })
    );

    let _ = std::fs::remove_file(hidden);
    let _ = std::fs::remove_file(visible);
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn file_dialog_overlay_exposes_msedit_like_dialog_fields() {
    let directory = temp_file_path("file-dialog-overlay");
    let file = directory.join("alpha.log");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(&file, "alpha").unwrap();
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/", directory.display()));

    let overlay = app.active_overlay().expect("file dialog overlay");
    assert_eq!(overlay.title, "Open");
    assert_eq!(
        overlay.input.as_deref(),
        Some(format!("{}/", directory.display()).as_str())
    );
    assert!(
        overlay
            .lines
            .iter()
            .any(|line| line.starts_with("Look in: "))
    );
    assert!(overlay.lines.iter().any(|line| line == "File name:"));
    assert!(
        overlay
            .lines
            .iter()
            .any(|line| line.starts_with("Hidden: "))
    );
    assert!(
        overlay
            .list
            .iter()
            .any(|line| line == "[..] Parent directory")
    );
    assert!(overlay.list.iter().any(|line| line.contains("alpha.log")));
    assert!(
        overlay
            .buttons
            .iter()
            .any(|line| line.contains("[Enter] OK"))
    );
    assert_eq!(overlay.min_width, 60);

    let _ = std::fs::remove_file(file);
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn file_dialog_uses_configured_modal_keybindings() {
    let mut config = Config::default();
    config.file_dialog_keys.set_action_binding(
        FileDialogAction::ToggleHidden,
        Some(KeyStroke::plain(Key::F(8))),
    );
    let mut app = AppState::from_config(config);

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('h'), CrosstermKeyModifiers::CONTROL),
    );
    assert!(!app.file_dialog.as_ref().unwrap().show_hidden);

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::F(8), CrosstermKeyModifiers::NONE),
    );
    assert!(app.file_dialog.as_ref().unwrap().show_hidden);

    let help = help_text(
        &app.shell.keymap,
        &app.file_dialog_keys,
        &app.shell.catalog,
        app.shell.profile.ambiguous_width,
    );
    assert!(help.contains("F8"));
    assert!(help.contains("file_dialog.toggle_hidden"));
}

#[test]
fn file_dialog_overlay_reports_scroll_overflow() {
    let directory = temp_file_path("open-dialog-overflow");
    std::fs::create_dir(&directory).unwrap();
    for index in 0..14 {
        std::fs::write(directory.join(format!("item{index:02}.txt")), "x").unwrap();
    }
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/", directory.display()));
    let overlay = app.active_overlay().expect("file dialog overlay");
    assert!(!overlay.list_has_more_above);
    assert!(overlay.list_has_more_below);

    app.scroll_file_dialog(2);
    let overlay = app.active_overlay().expect("file dialog overlay");
    assert!(overlay.list_has_more_above);
    assert!(overlay.list_has_more_below);

    for index in 0..14 {
        let _ = std::fs::remove_file(directory.join(format!("item{index:02}.txt")));
    }
    let _ = std::fs::remove_dir(directory);
}

#[test]
fn save_as_dialog_requires_second_enter_before_overwrite() {
    let path = temp_file_path("save-as-overwrite.txt");
    std::fs::write(&path, "old").unwrap();
    let mut app = AppState::new();
    app.handle_text_input('x');

    app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
    send_text(&mut app, &path.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old");
    assert!(app.file_dialog.is_some());
    assert!(
        app.active_overlay()
            .unwrap()
            .lines
            .iter()
            .any(|line| line.contains("Replace existing file"))
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().path.as_ref(),
        Some(&path)
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn save_as_dialog_tab_completes_directory_before_save() {
    let parent = temp_file_path("save-as-dialog-tab");
    let directory = parent.join("nested");
    let path = directory.join("out.txt");
    std::fs::create_dir(&parent).unwrap();
    std::fs::create_dir(&directory).unwrap();
    let mut app = AppState::new();
    app.handle_text_input('x');

    app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
    send_text(&mut app, &format!("{}/nes", parent.display()));
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Tab, CrosstermKeyModifiers::NONE),
    );
    assert_eq!(
        app.prompt_status_text(),
        Some(format!("Save As: {}/", directory.display()))
    );

    send_text(&mut app, "out.txt");
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
    assert_eq!(state.path.as_ref(), Some(&path));
    assert!(!state.buffer.is_dirty());

    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir(directory);
    let _ = std::fs::remove_dir(parent);
}

/// Picking a file with the arrow keys leaves the name field empty -- a fresh
/// Open dialog starts with no text at all and simply lists the working
/// directory. `submit` used to bail on an empty input *before* it looked at the
/// list selection, so Enter on an arrow-selected file cancelled the dialog, and
/// the selection branch underneath was unreachable in the one case it exists
/// for. Reproduce that state exactly: entries listed, selection moved, input
/// still empty.
#[test]
fn submitting_an_arrow_selected_file_with_an_empty_name_field_opens_it() {
    let directory = temp_file_path("open-dialog-empty-input");
    std::fs::create_dir(&directory).unwrap();
    let target = directory.join("bbb.txt");
    std::fs::write(directory.join("aaa.txt"), "AAA").unwrap();
    std::fs::write(&target, "BBB").unwrap();

    // List the directory, then clear the name field the way a fresh dialog is,
    // keeping the entries and moving the selection onto bbb.txt.
    let mut dialog = FileDialogState::new(
        FileDialogKind::Open,
        format!("{}/", directory.display()),
        None,
    );
    dialog.input.set_text(String::new());
    dialog.move_selection(1);
    dialog.move_selection(1);

    let selected = dialog.entries[dialog.selected_index.unwrap()].clone();
    assert_eq!(selected.name, "bbb.txt", "the arrows landed on bbb.txt");
    assert!(dialog.input.as_str().is_empty(), "the name field is empty");

    match dialog.submit() {
        FileDialogSubmit::Path(path) => assert_eq!(path, target),
        other => panic!("expected the selected file, got {other:?}"),
    }

    let _ = std::fs::remove_file(directory.join("aaa.txt"));
    let _ = std::fs::remove_file(target);
    let _ = std::fs::remove_dir(directory);
}

/// The same path must still navigate into a directory rather than "opening" it.
#[test]
fn submitting_an_arrow_selected_directory_navigates_into_it() {
    let directory = temp_file_path("open-dialog-empty-input-dir");
    let nested = directory.join("sub");
    std::fs::create_dir(&directory).unwrap();
    std::fs::create_dir(&nested).unwrap();

    let mut dialog = FileDialogState::new(
        FileDialogKind::Open,
        format!("{}/", directory.display()),
        None,
    );
    dialog.input.set_text(String::new());
    dialog.move_selection(1);

    assert!(matches!(dialog.submit(), FileDialogSubmit::ContinueEditing));

    let _ = std::fs::remove_dir(nested);
    let _ = std::fs::remove_dir(directory);
}

/// Enter with nothing typed and nothing selected still dismisses the dialog.
#[test]
fn submitting_an_untouched_empty_dialog_still_cancels() {
    let directory = temp_file_path("open-dialog-untouched");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("aaa.txt"), "AAA").unwrap();

    let mut dialog = FileDialogState::new(
        FileDialogKind::Open,
        format!("{}/", directory.display()),
        None,
    );
    dialog.input.set_text(String::new());

    assert!(matches!(dialog.submit(), FileDialogSubmit::Cancel));

    let _ = std::fs::remove_file(directory.join("aaa.txt"));
    let _ = std::fs::remove_dir(directory);
}

/// The end-to-end path through the key handler, with a directory typed in.
#[test]
fn arrow_selecting_a_file_and_pressing_enter_opens_it() {
    let directory = temp_file_path("open-dialog-arrow-select");
    std::fs::create_dir(&directory).unwrap();
    let target = directory.join("bbb.txt");
    std::fs::write(directory.join("aaa.txt"), "AAA").unwrap();
    std::fs::write(&target, "BBB").unwrap();

    let mut app = AppState::new();
    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{}/", directory.display()));

    for _ in 0..2 {
        handle_key_event(
            &mut app,
            CrosstermKeyEvent::new(CrosstermKeyCode::Down, CrosstermKeyModifiers::NONE),
        );
    }
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert!(app.file_dialog.is_none(), "the dialog closed");
    let focused = app
        .focused_buffer_id()
        .and_then(|id| app.buffer_state(id))
        .expect("a focused buffer");
    assert_eq!(focused.buffer.to_text(), "BBB");

    let _ = std::fs::remove_file(directory.join("aaa.txt"));
    let _ = std::fs::remove_file(target);
    let _ = std::fs::remove_dir(directory);
}
