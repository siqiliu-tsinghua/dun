use crate::*;

pub(crate) fn help_buffer(keymap: &Keymap, file_dialog_keys: &FileDialogKeymap) -> TextBuffer {
    let text = help_text(keymap, file_dialog_keys);
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, &text)
}

pub(crate) fn help_text(keymap: &Keymap, file_dialog_keys: &FileDialogKeymap) -> String {
    let mut out = String::from("Dun Help\n\n");

    for (index, section) in HELP_SECTIONS.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(section.title);
        out.push('\n');

        for command in section.commands {
            push_help_command(&mut out, keymap, &command.command, command.description);
        }
    }

    out.push_str(
        "\nPrompts\n  Enter           Submit prompt\n  Esc             Cancel prompt\n  Backspace       Edit prompt input\n  Up/Down         Command history\n\n",
    );
    out.push_str(
        "Command Prompt\n  plugin          Report syntax-highlight plugin state\n  plugin load     Enable lazy launch on the next edit\n  plugin unload   Stop and disable the highlight host\n\n",
    );
    out.push_str(
        "Selection\n  Shift+Arrow       Extend selection by character or line\n  Shift+Home/End    Extend selection to line edge\n  Shift+PageUp/Down Extend selection by visible page\n  Ctrl+Shift+Arrow  Extend selection by word when delivered\n\n",
    );
    out.push_str(
        "Navigation\n  PageUp/PageDown   Move by visible page\n  Ctrl+Home/End     Move to document start/end\n  F3/Shift+F3       Repeat find forward/backward\n\n",
    );
    out.push_str("File Dialogs\n");
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::Submit,
        "Open/save selected path",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::Cancel,
        "Cancel dialog",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::CompleteForward,
        "Complete path",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::CompleteBackward,
        "Complete path backward",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::ToggleHidden,
        "Toggle hidden files",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveSelectionUp,
        "Move file selection up",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveSelectionDown,
        "Move file selection down",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::PageSelectionUp,
        "Page file selection up",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::PageSelectionDown,
        "Page file selection down",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveInputLeft,
        "Move path cursor left",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveInputRight,
        "Move path cursor right",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveInputStart,
        "Move path cursor to start",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::MoveInputEnd,
        "Move path cursor to end",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::DeleteBackward,
        "Delete previous path character",
    );
    push_file_dialog_help(
        &mut out,
        file_dialog_keys,
        FileDialogAction::DeleteForward,
        "Delete path character",
    );
    out.push_str(
        "  Mouse click     Select list entry when mouse is enabled\n  Mouse wheel     Scroll list when mouse is enabled\n\n",
    );
    out.push_str(
        "Menus\n  Alt+F/E/V/H     Open File/Edit/View/Help menu\n  Left/Right      Switch open menu\n  Up/Down         Move menu selection\n  Enter           Run selected menu command\n  Esc             Close open menu\n\n",
    );
    out.push_str(
        "Notes\n  Type commands in the command prompt to list command-line actions.\n  Help opens as a read-only tiled window.\n  Dirty buffers ask for confirmation before changes are discarded.\n",
    );

    out
}

fn push_help_command(
    out: &mut String,
    keymap: &Keymap,
    command: &EditorCommand,
    description: &str,
) {
    let sequence = keymap
        .sequence_for_command(command)
        .map(ToString::to_string)
        .unwrap_or_else(|| "(unbound)".to_string());
    out.push_str(&format!(
        "  {sequence:<15} {description} [{}]\n",
        command_id(command)
    ));
}

fn push_file_dialog_help(
    out: &mut String,
    keymap: &FileDialogKeymap,
    action: FileDialogAction,
    description: &str,
) {
    let sequence = file_dialog_action_key_text(keymap, action);
    out.push_str(&format!(
        "  {sequence:<15} {description} [{}]\n",
        file_dialog_action_id(action)
    ));
}

pub(crate) fn file_dialog_action_key_text(
    keymap: &FileDialogKeymap,
    action: FileDialogAction,
) -> String {
    keymap
        .stroke_for_action(action)
        .map(|stroke| stroke.to_string())
        .unwrap_or_else(|| "(unbound)".to_string())
}

pub(crate) fn file_dialog_shortcuts_text(keymap: &FileDialogKeymap) -> String {
    format!(
        "[{}] OK  [{}] Complete  [{}] Hidden  [{}] Cancel",
        file_dialog_action_key_text(keymap, FileDialogAction::Submit),
        file_dialog_action_key_text(keymap, FileDialogAction::CompleteForward),
        file_dialog_action_key_text(keymap, FileDialogAction::ToggleHidden),
        file_dialog_action_key_text(keymap, FileDialogAction::Cancel),
    )
}

pub(crate) fn important_config_diagnostic_commands() -> &'static [EditorCommand] {
    &[
        EditorCommand::App(AppCommand::CommandLine),
        EditorCommand::App(AppCommand::Help),
        EditorCommand::App(AppCommand::ReloadConfig),
        EditorCommand::App(AppCommand::ConfigDiagnostics),
        EditorCommand::App(AppCommand::RunCommand),
        EditorCommand::File(FileCommand::Open),
        EditorCommand::File(FileCommand::Save),
        EditorCommand::Edit(EditCommand::Find),
        EditorCommand::Edit(EditCommand::CopyExternal),
        EditorCommand::Window(WindowCommand::SplitHorizontal),
        EditorCommand::Window(WindowCommand::FocusLeft),
    ]
}

struct HelpSection {
    title: &'static str,
    commands: &'static [HelpCommand],
}

struct HelpCommand {
    command: EditorCommand,
    description: &'static str,
}

const HELP_SECTIONS: &[HelpSection] = &[
    HelpSection {
        title: "App",
        commands: &[
            HelpCommand {
                command: EditorCommand::App(AppCommand::Help),
                description: "Help",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::StatusHistory),
                description: "Status history",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::CommandLine),
                description: "Command line",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::ConfigDiagnostics),
                description: "Config diagnostics",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::SearchResults),
                description: "List current search results",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::ReloadConfig),
                description: "Reload config",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::RunCommand),
                description: "Run command to read-only output",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::ShellEscape),
                description: "Open interactive shell and return",
            },
            HelpCommand {
                command: EditorCommand::App(AppCommand::Quit),
                description: "Quit",
            },
        ],
    },
    HelpSection {
        title: "File",
        commands: &[
            HelpCommand {
                command: EditorCommand::File(FileCommand::New),
                description: "New untitled buffer",
            },
            HelpCommand {
                command: EditorCommand::File(FileCommand::Open),
                description: "Open file",
            },
            HelpCommand {
                command: EditorCommand::File(FileCommand::SwitchBuffer),
                description: "Switch open buffer",
            },
            HelpCommand {
                command: EditorCommand::File(FileCommand::Save),
                description: "Save",
            },
            HelpCommand {
                command: EditorCommand::File(FileCommand::SaveAs),
                description: "Save as",
            },
            HelpCommand {
                command: EditorCommand::File(FileCommand::Reload),
                description: "Reload from disk",
            },
            HelpCommand {
                command: EditorCommand::File(FileCommand::Close),
                description: "Close focused file/window",
            },
        ],
    },
    HelpSection {
        title: "Edit",
        commands: &[
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveLeft),
                description: "Move left",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveRight),
                description: "Move right",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveUp),
                description: "Move up",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveDown),
                description: "Move down",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MovePageUp),
                description: "Move page up",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MovePageDown),
                description: "Move page down",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveDocumentStart),
                description: "Move to document start",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveDocumentEnd),
                description: "Move to document end",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ScrollLeft),
                description: "Scroll view left",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ScrollRight),
                description: "Scroll view right",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveWordLeft),
                description: "Move word left",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveWordRight),
                description: "Move word right",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveLineStart),
                description: "Move to line start",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveLineEnd),
                description: "Move to line end",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ExtendSelectionPageUp),
                description: "Extend selection page up",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ExtendSelectionPageDown),
                description: "Extend selection page down",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ExtendSelectionWordLeft),
                description: "Extend selection word left",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ExtendSelectionWordRight),
                description: "Extend selection word right",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::InsertNewline),
                description: "Insert newline",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::DeleteBackward),
                description: "Delete backward",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::DeleteForward),
                description: "Delete forward",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::DeleteWordBackward),
                description: "Delete word backward",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::DeleteWordForward),
                description: "Delete word forward",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Undo),
                description: "Undo",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Redo),
                description: "Redo",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Cut),
                description: "Cut selection to internal clipboard",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Copy),
                description: "Copy selection to internal clipboard",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::CopyExternal),
                description: "Copy selection through OSC 52 when enabled",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Paste),
                description: "Paste internal clipboard",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::SelectAll),
                description: "Select all",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::SelectLine),
                description: "Select current line",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::CopyLine),
                description: "Copy current line",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::DeleteLine),
                description: "Delete current line",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveLineUp),
                description: "Move current line up",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::MoveLineDown),
                description: "Move current line down",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::IndentLine),
                description: "Indent selected/current lines",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::OutdentLine),
                description: "Outdent selected/current lines",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::TrimTrailingWhitespace),
                description: "Trim trailing whitespace",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::ToggleWordWrap),
                description: "Toggle word wrap",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Find),
                description: "Find",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::FindNext),
                description: "Find next",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::FindPrevious),
                description: "Find previous",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::Replace),
                description: "Replace current or next match",
            },
            HelpCommand {
                command: EditorCommand::Edit(EditCommand::GoToLine),
                description: "Go to line",
            },
        ],
    },
    HelpSection {
        title: "Windows",
        commands: &[
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::SplitHorizontal),
                description: "Split horizontally",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::SplitVertical),
                description: "Split vertically",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::FocusLeft),
                description: "Focus left",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::FocusRight),
                description: "Focus right",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::FocusUp),
                description: "Focus up",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::FocusDown),
                description: "Focus down",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::ResizeLeft),
                description: "Resize left",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::ResizeRight),
                description: "Resize right",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::ResizeUp),
                description: "Resize up",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::ResizeDown),
                description: "Resize down",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::Equalize),
                description: "Equalize splits",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::RotateSplit),
                description: "Rotate focused split",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::ToggleCollapse),
                description: "Collapse or expand focused pane",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::Close),
                description: "Close focused window",
            },
        ],
    },
];
