#![allow(unused_imports)]

use super::support::*;

fn key(code: CrosstermKeyCode) -> CrosstermKeyEvent {
    CrosstermKeyEvent::new(code, CrosstermKeyModifiers::NONE)
}

fn alt(ch: char) -> CrosstermKeyEvent {
    CrosstermKeyEvent::new(CrosstermKeyCode::Char(ch), CrosstermKeyModifiers::ALT)
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
    handle_key_event(&mut app, key(CrosstermKeyCode::Char('o')));

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
    handle_key_event(&mut app, key(CrosstermKeyCode::Char('j')));
    assert_eq!(app.active_menu, Some(2));

    // View -> "Split Horizontal (H)", matched case-insensitively.
    handle_key_event(&mut app, key(CrosstermKeyCode::Char('H')));
    assert!(app.active_menu.is_none());
    assert_eq!(app.workspace.window_count(), 2, "the split ran");
}

/// Mnemonics have to be unique within a menu, or a letter would be ambiguous
/// and the later entry unreachable from the keyboard. Resolving each expected
/// letter back through the lookup proves both that it exists and that it wins.
#[test]
fn every_menu_entry_has_a_unique_mnemonic() {
    let shell = UiShell::default();
    assert_eq!(shell.menu_count(), MENU_MNEMONICS.len());

    for (menu_index, mnemonics) in MENU_MNEMONICS.iter().enumerate() {
        assert_eq!(
            shell.menu_entry_count(menu_index),
            Some(mnemonics.len()),
            "menu {menu_index} gained or lost entries without updating its mnemonics"
        );

        for (entry_index, mnemonic) in mnemonics.iter().enumerate() {
            let resolved = shell
                .menu_entry_index_for_mnemonic(menu_index, *mnemonic)
                .unwrap_or_else(|| panic!("menu {menu_index}: no entry advertises `{mnemonic}`"));
            assert_eq!(
                resolved, entry_index,
                "menu {menu_index}: `{mnemonic}` resolves to entry {resolved}, not the entry \
                 that advertises it ({entry_index}) -- duplicate mnemonic"
            );
        }
    }
}

/// The mnemonics as written in `dun-ui`'s menu labels.
const MENU_MNEMONICS: [&[char]; 4] = [
    &['N', 'O', 'B', 'S', 'A', 'E', 'C', 'R', 'H', 'Q'],
    &[
        'U', 'R', 'T', 'C', 'X', 'P', 'A', 'L', 'Y', 'K', 'I', 'O', 'W', 'F', 'N', 'B', 'G',
    ],
    &['H', 'V', 'E', 'C', 'Z', '[', ']', 'X', 'W', 'S', 'D', 'R'],
    &['H'],
];

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

    handle_key_event(&mut app, key(CrosstermKeyCode::Char('x')));
    assert_eq!(
        app.status_message, None,
        "the next keypress hands the status line back to the buffer readout"
    );
}
