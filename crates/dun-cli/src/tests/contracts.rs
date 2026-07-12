use super::support::*;
use dun_config::ALL_COMMAND_IDS;
use std::collections::BTreeSet;

/// The rejected advertised `theme = mono` setting was stale user-facing text;
/// this catches the same declaration drift in the in-editor help command list.
#[test]
fn every_help_entry_names_a_real_command() {
    let help = help_text(
        &Keymap::default_editor(),
        &FileDialogKeymap::default_file_dialog(),
    );
    let command_help = help
        .split_once("\nPrompts\n")
        .map(|(commands, _)| commands)
        .expect("help command sections must precede the prompt documentation");
    let help_ids = command_help
        .lines()
        .filter_map(|line| {
            let (_, bracketed_id) = line.rsplit_once('[')?;
            bracketed_id.strip_suffix(']')
        })
        .collect::<Vec<_>>();

    assert!(!help_ids.is_empty(), "help must list editor commands");
    for id in help_ids {
        assert!(
            ALL_COMMAND_IDS.contains(&id),
            "help names command `{id}` outside ALL_COMMAND_IDS"
        );
    }
}

const PROMPT_ONLY_COMMANDS: &[&str] = &[
    // The one-way collapse action has no default binding or menu entry.
    "window.collapse",
    // The one-way expand action has no default binding or menu entry.
    "window.expand",
    // The close-other-windows action has no default binding or menu entry.
    "window.only",
    // Clipboard diagnostics are exposed only through the command prompt.
    "app.config_diagnostics_clipboard",
    // File-dialog keymap diagnostics are exposed only through the command prompt.
    "app.config_diagnostics_file_dialog_keymap",
    // Input diagnostics are exposed only through the command prompt.
    "app.config_diagnostics_input",
    // Editor keymap diagnostics are exposed only through the command prompt.
    "app.config_diagnostics_keymap",
    // Limit diagnostics are exposed only through the command prompt.
    "app.config_diagnostics_limits",
    // Path diagnostics are exposed only through the command prompt.
    "app.config_diagnostics_paths",
    // Config-source diagnostics are exposed only through the command prompt.
    "app.config_diagnostics_source",
    // Summary diagnostics are exposed only through the command prompt.
    "app.config_diagnostics_summary",
    // Terminal diagnostics are exposed only through the command prompt.
    "app.config_diagnostics_terminal",
];

/// The command prompt made every id technically runnable while concealing the
/// same discoverability failure as the 40 dead menu mnemonics; exceptions must
/// remain explicit and exact.
#[test]
fn every_command_is_reachable_without_typing_its_id() {
    let keymap = Keymap::default_editor();
    let mut reachable = keymap
        .bindings
        .iter()
        .map(|binding| command_id(&binding.command))
        .collect::<BTreeSet<_>>();
    let shell = UiShell::default();

    for menu_index in 0..shell.menu_count() {
        let entry_count = shell
            .menu_entry_count(menu_index)
            .expect("an enumerated menu must exist");
        for entry_index in 0..entry_count {
            let command = shell
                .menu_entry_command(menu_index, entry_index)
                .expect("an enumerated menu entry must have a command");
            reachable.insert(command_id(&command));
        }
    }

    let prompt_only = PROMPT_ONLY_COMMANDS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        prompt_only.len(),
        PROMPT_ONLY_COMMANDS.len(),
        "PROMPT_ONLY_COMMANDS contains duplicates"
    );
    for &id in PROMPT_ONLY_COMMANDS {
        assert!(
            ALL_COMMAND_IDS.contains(&id),
            "prompt-only allowlist names unknown command `{id}`"
        );
        assert!(
            !reachable.contains(id),
            "`{id}` is now discoverable; remove it from PROMPT_ONLY_COMMANDS"
        );
    }

    let unreachable = ALL_COMMAND_IDS
        .iter()
        .copied()
        .filter(|id| !reachable.contains(id))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        unreachable, prompt_only,
        "commands without a default keybinding or menu entry must be explicitly allowlisted"
    );
}
