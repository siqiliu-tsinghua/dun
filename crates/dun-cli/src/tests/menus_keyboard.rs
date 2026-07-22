#![allow(unused_imports)]

use super::support::*;

fn key(code: TerminalKeyCode) -> TerminalKeyEvent {
    TerminalKeyEvent::new(code, TerminalKeyModifiers::NONE)
}

fn alt(ch: char) -> TerminalKeyEvent {
    TerminalKeyEvent::new(TerminalKeyCode::Char(ch), TerminalKeyModifiers::ALT)
}

fn ctrl(ch: char) -> TerminalKeyEvent {
    TerminalKeyEvent::new(TerminalKeyCode::Char(ch), TerminalKeyModifiers::CONTROL)
}

/// Every dropdown entry advertises a mnemonic in its label ("Open... (O)"), so
/// a bare letter must run it. The runtime used to handle only Esc, the arrows,
/// Enter, and Alt+letter while a menu was open, which left every one of those
/// parenthesised letters a promise the editor never kept.
#[test]
fn a_bare_letter_runs_the_menu_entry_that_advertises_it() {
    let mut app = AppState::new();

    handle_key_event(&mut app, alt('f'));
    assert_eq!(app.active_menu, Some(0), "Alt+F opens the File menu");

    // File -> "Open... (O)".
    handle_key_event(&mut app, key(TerminalKeyCode::Char('o')));

    assert!(app.active_menu.is_none(), "the menu closes once it runs");
    assert!(
        app.file_dialog.is_some(),
        "pressing O must open the Open dialog"
    );
}

#[test]
fn the_mnemonic_is_case_insensitive_and_ignores_unmatched_keys() {
    let mut app = AppState::new();

    handle_key_event(&mut app, alt('v'));
    // View has no "J" entry: an unmatched key leaves the menu open rather than
    // silently swallowing the keypress or closing on it.
    handle_key_event(&mut app, key(TerminalKeyCode::Char('j')));
    assert_eq!(app.active_menu, Some(2));

    // View -> "Split Horizontal (H)", matched case-insensitively.
    handle_key_event(&mut app, key(TerminalKeyCode::Char('H')));
    assert!(app.active_menu.is_none());
    assert_eq!(app.workspace.window_count(), 2, "the split ran");
}

#[test]
fn collapse_and_expand_are_reachable_from_view_menu_mnemonics() {
    let mut app = AppState::new();
    // The only window refuses to collapse, so give the room somewhere to go.
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));

    handle_key_event(&mut app, alt('v'));
    handle_key_event(&mut app, key(TerminalKeyCode::Char('m')));
    assert!(app.workspace.focused_window().unwrap().collapsed);

    handle_key_event(&mut app, alt('v'));
    handle_key_event(&mut app, key(TerminalKeyCode::Char('p')));
    assert!(!app.workspace.focused_window().unwrap().collapsed);
}

#[test]
fn collapse_and_expand_are_reachable_from_default_keybindings() {
    let mut app = AppState::new();
    // The only window refuses to collapse, so give the room somewhere to go.
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));

    handle_key_event(&mut app, ctrl('x'));
    handle_key_event(&mut app, key(TerminalKeyCode::Char('m')));
    assert!(app.workspace.focused_window().unwrap().collapsed);

    handle_key_event(&mut app, ctrl('x'));
    handle_key_event(&mut app, key(TerminalKeyCode::Char('p')));
    assert!(!app.workspace.focused_window().unwrap().collapsed);
}

/// The 40 dead menu keys were declarations the runtime never honoured; deriving
/// from the labels also catches a missing or ambiguous declaration itself.
#[test]
fn every_menu_entry_has_a_unique_mnemonic() {
    let shell = UiShell::default();

    for menu_index in 0..shell.menu_count() {
        let entry_count = shell
            .menu_entry_count(menu_index)
            .expect("an enumerated menu must exist");
        let mut seen_mnemonics = Vec::new();

        for entry_index in 0..entry_count {
            let mnemonic = shell
                .menu_entry_mnemonic(menu_index, entry_index)
                .unwrap_or_else(|| {
                    panic!("menu {menu_index} entry {entry_index} has no derivable mnemonic")
                });
            assert!(
                !seen_mnemonics
                    .iter()
                    .any(|seen: &char| seen.eq_ignore_ascii_case(&mnemonic)),
                "menu {menu_index} entry {entry_index} duplicates mnemonic `{mnemonic}`"
            );
            seen_mnemonics.push(mnemonic);
        }
    }
}

/// Bare letters once ignored all 40 advertised mnemonics; every derived letter
/// must resolve to the exact entry that declares it. The two tests above keep
/// the dispatch path covered end to end without executing Quit or Shell Escape.
#[test]
fn every_menu_mnemonic_dispatches_its_own_entry() {
    let shell = UiShell::default();

    for menu_index in 0..shell.menu_count() {
        let entry_count = shell
            .menu_entry_count(menu_index)
            .expect("an enumerated menu must exist");
        for entry_index in 0..entry_count {
            let mnemonic = shell
                .menu_entry_mnemonic(menu_index, entry_index)
                .expect("the mnemonic declaration contract is checked separately");
            assert_eq!(
                shell.menu_entry_index_for_mnemonic(menu_index, mnemonic),
                Some(entry_index),
                "menu {menu_index} mnemonic `{mnemonic}` does not dispatch entry {entry_index}"
            );
        }
    }
}

/// `file.close` used to dispatch `window.close`, leaving one command listed
/// twice and the file-close command absent; menu commands must be one-to-one.
#[test]
fn no_two_menu_entries_dispatch_the_same_command() {
    let shell = UiShell::default();
    let mut commands = Vec::new();

    for menu_index in 0..shell.menu_count() {
        let entry_count = shell
            .menu_entry_count(menu_index)
            .expect("an enumerated menu must exist");
        for entry_index in 0..entry_count {
            let command = shell
                .menu_entry_command(menu_index, entry_index)
                .expect("an enumerated menu entry must have a command");
            for (seen_menu, seen_entry, seen_command) in &commands {
                assert_ne!(
                    seen_command,
                    &command,
                    "menu {seen_menu} entry {seen_entry} and menu {menu_index} entry \
                     {entry_index} both dispatch `{}`",
                    command_id(&command)
                );
            }
            commands.push((menu_index, entry_index, command));
        }
    }
}

/// Status messages were written into the frame and then unconditionally
/// overwritten by the buffer readout whenever no modal was open, so every
/// command's feedback was invisible in normal editing. They must survive the
/// frame that follows the command, and give the status line back on the next
/// keypress.
#[test]
fn a_status_message_survives_until_the_next_keypress() {
    let mut app = AppState::new();

    // Nothing to undo: the refusal is the only feedback the user gets.
    app.handle_command(&EditorCommand::Edit(EditCommand::Undo));
    assert!(
        app.status_message.is_some(),
        "the command reports why it did nothing"
    );

    handle_key_event(&mut app, key(TerminalKeyCode::Char('x')));
    assert_eq!(
        app.status_message, None,
        "the next keypress hands the status line back to the buffer readout"
    );
}
