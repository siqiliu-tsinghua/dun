use super::support::*;

#[test]
fn parses_single_key_stroke_with_modifiers() {
    let stroke = KeyStroke::from_str("Ctrl+Alt+Q").unwrap();

    assert_eq!(
        stroke,
        KeyStroke::new(
            Key::Char('q'),
            KeyModifiers {
                ctrl: true,
                alt: true,
                shift: false,
            },
        )
    );
}

#[test]
fn parses_key_sequence() {
    let sequence = KeySequence::from_str("Ctrl+X, V").unwrap();

    assert_eq!(sequence.strokes.len(), 2);
    assert_eq!(sequence.strokes[0].key, Key::Char('x'));
    assert_eq!(sequence.strokes[1].key, Key::Char('v'));
}

#[test]
fn parses_special_keys() {
    assert_eq!(
        KeyStroke::from_str("Alt+Shift+Left").unwrap(),
        KeyStroke::new(Key::Left, KeyModifiers::ALT_SHIFT)
    );
    assert_eq!(
        KeyStroke::from_str("F12").unwrap(),
        KeyStroke::plain(Key::F(12))
    );
    assert_eq!(
        KeyStroke::from_str("Esc").unwrap(),
        KeyStroke::plain(Key::Esc)
    );
}

#[test]
fn rejects_unknown_key_names() {
    assert_eq!(
        KeyStroke::from_str("Ctrl+Hyper"),
        Err(KeyParseError::UnknownKey("Hyper".to_string()))
    );
}

#[test]
fn keymap_finds_bound_command() {
    let keymap = Keymap::default_editor();
    let sequence = KeySequence::from_str("Ctrl+S").unwrap();

    assert_eq!(
        keymap.command_for_sequence(&sequence),
        Some(&EditorCommand::File(FileCommand::Save))
    );

    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+G").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::GoToLine))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("PageDown").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::MovePageDown))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Shift+PageDown").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::ExtendSelectionPageDown))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,]").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::ScrollRight))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,.").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,F").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::ToggleFold))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,A").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::UnfoldAll))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,K").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::ToggleBookmark))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,N").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::NextBookmark))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,L").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::PreviousBookmark))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+Right").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::MoveWordRight))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+Delete").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::DeleteWordForward))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+Shift+Left").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::ExtendSelectionWordLeft))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,Ctrl+C").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::CopyExternal))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,Ctrl+V").unwrap()),
        Some(&EditorCommand::Edit(EditCommand::PasteExternal))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("F2").unwrap()),
        Some(&EditorCommand::App(AppCommand::StatusHistory))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("F5").unwrap()),
        Some(&EditorCommand::App(AppCommand::ReloadConfig))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("F6").unwrap()),
        Some(&EditorCommand::App(AppCommand::ConfigDiagnostics))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,O").unwrap()),
        Some(&EditorCommand::App(AppCommand::RunCommand))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,S").unwrap()),
        Some(&EditorCommand::App(AppCommand::ShellEscape))
    );
    assert_eq!(
        keymap.sequence_for_command(&EditorCommand::File(FileCommand::Save)),
        Some(&sequence)
    );
}

/// The dead menu mnemonics showed that a declared shortcut can silently name
/// nothing runnable; every compiled-in binding must stay on the command surface.
#[test]
fn every_default_keybinding_names_a_real_command() {
    let keymap = Keymap::default_editor();

    for binding in &keymap.bindings {
        let id = command_id(&binding.command);
        assert!(
            ALL_COMMAND_IDS.contains(&id),
            "default binding `{}` names unlisted command `{id}`",
            binding.sequence
        );
    }
}

#[test]
fn default_keymap_has_mac_friendly_window_aliases() {
    let keymap = Keymap::default_editor();

    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,Left").unwrap()),
        Some(&EditorCommand::Window(WindowCommand::FocusLeft))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Alt+Left").unwrap()),
        Some(&EditorCommand::Window(WindowCommand::FocusLeft))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Ctrl+X,Shift+Right").unwrap()),
        Some(&EditorCommand::Window(WindowCommand::ResizeRight))
    );
    assert_eq!(
        keymap.command_for_sequence(&KeySequence::from_str("Alt+Shift+Right").unwrap()),
        Some(&EditorCommand::Window(WindowCommand::ResizeRight))
    );
    assert_eq!(
        keymap.sequence_for_command(&EditorCommand::Window(WindowCommand::FocusLeft)),
        Some(&KeySequence::from_str("Ctrl+X,Left").unwrap())
    );
}

#[test]
fn keymap_reports_sequence_prefixes() {
    let keymap = Keymap::default_editor();

    assert!(keymap.has_sequence_prefix(&KeySequence::from_str("Ctrl+X").unwrap()));
    assert!(!keymap.has_sequence_prefix(&KeySequence::from_str("Ctrl+X,H").unwrap()));
    assert!(!keymap.has_sequence_prefix(&KeySequence::from_str("Alt+Left").unwrap()));
}

#[test]
fn keymap_rejects_duplicate_bindings() {
    let keymap = Keymap {
        bindings: vec![
            KeyBinding::new("Ctrl+S", EditorCommand::File(FileCommand::Save)),
            KeyBinding::new("Ctrl+S", EditorCommand::File(FileCommand::SaveAs)),
        ],
    };

    assert_eq!(
        keymap.validate(),
        Err(KeymapError::DuplicateBinding {
            sequence: KeySequence::from_str("Ctrl+S").unwrap(),
            bound: "file.save",
            rebound: "file.save_as",
        })
    );
}

#[test]
fn file_dialog_keymap_finds_bound_actions() {
    let keymap = FileDialogKeymap::default_file_dialog();

    assert_eq!(
        keymap.action_for_stroke(KeyStroke::from_str("Ctrl+H").unwrap()),
        Some(FileDialogAction::ToggleHidden)
    );
    assert_eq!(
        keymap.stroke_for_action(FileDialogAction::MoveInputStart),
        Some(KeyStroke::from_str("Home").unwrap())
    );
    assert_eq!(
        file_dialog_action_id(FileDialogAction::DeleteForward),
        "file_dialog.delete_forward"
    );
    assert_eq!(
        file_dialog_action_from_id("delete-forward"),
        Ok(FileDialogAction::DeleteForward)
    );
}

#[test]
fn file_dialog_keymap_rejects_duplicate_bindings() {
    let keymap = FileDialogKeymap {
        bindings: vec![
            FileDialogKeyBinding::new("Esc", FileDialogAction::Cancel),
            FileDialogKeyBinding::new("Esc", FileDialogAction::Submit),
        ],
    };

    assert_eq!(
        keymap.validate(),
        Err(FileDialogKeymapError::DuplicateBinding {
            stroke: KeyStroke::from_str("Esc").unwrap(),
            bound: "file_dialog.cancel",
            rebound: "file_dialog.submit",
        })
    );
}

#[test]
fn key_sequences_have_stable_display_text() {
    assert_eq!(
        KeySequence::from_str("Ctrl+X,H").unwrap().to_string(),
        "Ctrl+X,H"
    );
    assert_eq!(
        KeySequence::from_str("Alt+Shift+Left").unwrap().to_string(),
        "Alt+Shift+Left"
    );
}

/// `file.close` was once an alias for `window.close`; exhaustive canonical
/// round-tripping catches two ids that parse to the same command.
#[test]
fn all_command_ids_round_trip() {
    const EDITOR_COMMAND_VARIANT_COUNT: usize = 7 + 51 + 17 + 18;

    assert_eq!(ALL_COMMAND_IDS.len(), EDITOR_COMMAND_VARIANT_COUNT);

    let mut unique_ids = ALL_COMMAND_IDS.to_vec();
    unique_ids.sort_unstable();
    unique_ids.dedup();
    assert_eq!(
        unique_ids.len(),
        ALL_COMMAND_IDS.len(),
        "ALL_COMMAND_IDS contains duplicates"
    );

    for &id in ALL_COMMAND_IDS {
        let command = command_from_id(id)
            .unwrap_or_else(|error| panic!("listed command id `{id}` does not parse: {error:?}"));
        assert_eq!(
            command_id(&command),
            id,
            "listed command id `{id}` parses to a different canonical command"
        );
    }
}

/// Plugin actions carry a `plugin_id`/`action_id` pair but are not
/// user-bindable: every instance collapses to one generic `command_id`, that id
/// is deliberately absent from `ALL_COMMAND_IDS`, and it does not round-trip
/// back through `command_from_id` (a user keymap can never name one).
#[test]
fn plugin_action_has_a_generic_non_bindable_id() {
    let action = EditorCommand::PluginAction {
        plugin_id: "log-filter".into(),
        action_id: "run".into(),
        kind: dun_core::PluginActionKind::Surface,
    };
    let other = EditorCommand::PluginAction {
        plugin_id: "notes".into(),
        action_id: "clear".into(),
        kind: dun_core::PluginActionKind::Execute,
    };

    assert_eq!(command_id(&action), "plugin.action");
    assert_eq!(command_id(&other), command_id(&action));

    assert!(!ALL_COMMAND_IDS.contains(&"plugin.action"));
    assert_eq!(
        command_from_id("plugin.action"),
        Err(CommandParseError::UnknownCommand(
            "plugin.action".to_string()
        ))
    );
}

#[test]
fn command_id_aliases_and_unknown_ids_are_preserved() {
    assert_eq!(
        command_from_id("app.reload_config"),
        Ok(EditorCommand::App(AppCommand::ReloadConfig))
    );
    assert_eq!(
        command_from_id("app.config_diagnostics"),
        Ok(EditorCommand::App(AppCommand::ConfigDiagnostics))
    );
    assert_eq!(
        command_from_id("edit.move-word-right"),
        Ok(EditorCommand::Edit(EditCommand::MoveWordRight))
    );
    assert_eq!(
        command_from_id("edit.move_document_start"),
        Ok(EditorCommand::Edit(EditCommand::MoveDocumentStart))
    );
    assert_eq!(
        command_from_id("edit.move_document_end"),
        Ok(EditorCommand::Edit(EditCommand::MoveDocumentEnd))
    );
    assert_eq!(
        command_from_id("edit.extend_selection_page_down"),
        Ok(EditorCommand::Edit(EditCommand::ExtendSelectionPageDown))
    );
    assert_eq!(
        command_from_id("edit.scroll_right"),
        Ok(EditorCommand::Edit(EditCommand::ScrollRight))
    );
    assert_eq!(
        command_from_id("edit.delete_word_backward"),
        Ok(EditorCommand::Edit(EditCommand::DeleteWordBackward))
    );
    assert_eq!(
        command_from_id("edit.copy_external"),
        Ok(EditorCommand::Edit(EditCommand::CopyExternal))
    );
    assert_eq!(
        command_from_id("edit.paste_external"),
        Ok(EditorCommand::Edit(EditCommand::PasteExternal))
    );
    assert!(command_from_id("app.command_output_clear").is_err());
    assert!(command_from_id("app.command_output_save").is_err());
    assert_eq!(
        command_from_id("app.config_diagnostics_keymap"),
        Ok(EditorCommand::App(AppCommand::ConfigDiagnosticsKeymap))
    );
    assert_eq!(
        command_from_id("app.config_diagnostics_file_dialog_keymap"),
        Ok(EditorCommand::App(
            AppCommand::ConfigDiagnosticsFileDialogKeymap
        ))
    );
    assert!(command_from_id("app.outline").is_err());
    assert_eq!(
        command_from_id("app.search_results"),
        Ok(EditorCommand::App(AppCommand::SearchResults))
    );
    assert_eq!(
        command_from_id("app.nope"),
        Err(CommandParseError::UnknownCommand("app.nope".to_string()))
    );
}
