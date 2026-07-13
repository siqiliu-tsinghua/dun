use dun_config::TextCatalog;

use crate::*;

pub(crate) fn help_buffer(
    keymap: &Keymap,
    file_dialog_keys: &FileDialogKeymap,
    catalog: &TextCatalog,
) -> TextBuffer {
    let text = help_text(keymap, file_dialog_keys, catalog);
    TextBuffer::from_text_with_kind(BufferKind::ReadOnly, &text)
}

/// One fixed help row: a literal key-cap column (never translated — key
/// names are key names in every language) plus a translatable description.
struct HelpRow {
    keys: &'static str,
    key: &'static str,
    english: &'static str,
}

const fn row(keys: &'static str, key: &'static str, english: &'static str) -> HelpRow {
    HelpRow { keys, key, english }
}

/// Fixed help sections that are not generated from a keymap. The catalog
/// key of every title and row lives here so the translation-completeness
/// test can enumerate them.
const FIXED_SECTIONS: &[(&str, &str, &[HelpRow])] = &[
    (
        "help.section.prompts",
        "Prompts",
        &[
            row("Enter", "help.prompts.submit", "Submit prompt"),
            row("Esc", "help.prompts.cancel", "Cancel prompt"),
            row("Backspace", "help.prompts.edit", "Edit prompt input"),
            row("Up/Down", "help.prompts.history", "Command history"),
        ],
    ),
    (
        "help.section.command-prompt",
        "Command Prompt",
        &[
            row(
                "plugin",
                "help.command-prompt.plugin",
                "Report syntax-highlight plugin state",
            ),
            row(
                "plugin load",
                "help.command-prompt.plugin-load",
                "Enable lazy launch on the next edit",
            ),
            row(
                "plugin unload",
                "help.command-prompt.plugin-unload",
                "Stop and disable the highlight host",
            ),
        ],
    ),
    (
        "help.section.selection",
        "Selection",
        &[
            row(
                "Shift+Arrow",
                "help.selection.arrow",
                "Extend selection by character or line",
            ),
            row(
                "Shift+Home/End",
                "help.selection.line-edge",
                "Extend selection to line edge",
            ),
            row(
                "Shift+PageUp/Down",
                "help.selection.page",
                "Extend selection by visible page",
            ),
            row(
                "Ctrl+Shift+Arrow",
                "help.selection.word",
                "Extend selection by word when delivered",
            ),
        ],
    ),
    (
        "help.section.navigation",
        "Navigation",
        &[
            row(
                "PageUp/PageDown",
                "help.navigation.page",
                "Move by visible page",
            ),
            row(
                "Ctrl+Home/End",
                "help.navigation.document",
                "Move to document start/end",
            ),
            row(
                "F3/Shift+F3",
                "help.navigation.find-repeat",
                "Repeat find forward/backward",
            ),
        ],
    ),
];

const MOUSE_ROWS: &[HelpRow] = &[
    row(
        "Mouse click",
        "help.file-dialogs.mouse-click",
        "Select list entry when mouse is enabled",
    ),
    row(
        "Mouse wheel",
        "help.file-dialogs.mouse-wheel",
        "Scroll list when mouse is enabled",
    ),
];

const MENU_ROWS: &[HelpRow] = &[
    row(
        "Alt+F/E/V/H",
        "help.menus.open",
        "Open File/Edit/View/Help menu",
    ),
    row("Left/Right", "help.menus.switch", "Switch open menu"),
    row("Up/Down", "help.menus.move", "Move menu selection"),
    row("Enter", "help.menus.run", "Run selected menu command"),
    row("Esc", "help.menus.close", "Close open menu"),
];

const NOTE_KEYS: &[(&str, &str)] = &[
    (
        "help.notes.command-prompt",
        "Type commands in the command prompt to list command-line actions.",
    ),
    (
        "help.notes.read-only",
        "Help opens as a read-only tiled window.",
    ),
    (
        "help.notes.dirty-confirm",
        "Dirty buffers ask for confirmation before changes are discarded.",
    ),
];

const FILE_DIALOG_HELP: &[(FileDialogAction, &str)] = &[
    (FileDialogAction::Submit, "Open/save selected path"),
    (FileDialogAction::Cancel, "Cancel dialog"),
    (FileDialogAction::CompleteForward, "Complete path"),
    (FileDialogAction::CompleteBackward, "Complete path backward"),
    (FileDialogAction::ToggleHidden, "Toggle hidden files"),
    (FileDialogAction::MoveSelectionUp, "Move file selection up"),
    (
        FileDialogAction::MoveSelectionDown,
        "Move file selection down",
    ),
    (FileDialogAction::PageSelectionUp, "Page file selection up"),
    (
        FileDialogAction::PageSelectionDown,
        "Page file selection down",
    ),
    (FileDialogAction::MoveInputLeft, "Move path cursor left"),
    (FileDialogAction::MoveInputRight, "Move path cursor right"),
    (
        FileDialogAction::MoveInputStart,
        "Move path cursor to start",
    ),
    (FileDialogAction::MoveInputEnd, "Move path cursor to end"),
    (
        FileDialogAction::DeleteBackward,
        "Delete previous path character",
    ),
    (FileDialogAction::DeleteForward, "Delete path character"),
];

fn tr<'a>(catalog: &'a TextCatalog, key: &str, english: &'static str) -> &'a str {
    catalog.get(key).unwrap_or(english)
}

/// Pad by display width, not char count: a translated key column ("(未绑定)")
/// is wider than its char count says, and `{:<15}` would misalign every
/// description after it.
fn pad_to_display_width(text: &str, width: usize) -> String {
    let pad = width.saturating_sub(UnicodeWidthStr::width(text));
    let mut out = String::with_capacity(text.len() + pad);
    out.push_str(text);
    out.extend(std::iter::repeat_n(' ', pad));
    out
}

pub(crate) fn help_text(
    keymap: &Keymap,
    file_dialog_keys: &FileDialogKeymap,
    catalog: &TextCatalog,
) -> String {
    let mut out = String::from(tr(catalog, "help.title", "Dun Help"));
    out.push_str("\n\n");

    for (index, section) in HELP_SECTIONS.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(tr(catalog, section.key, section.title));
        out.push('\n');

        for command in section.commands {
            push_help_command(
                &mut out,
                keymap,
                catalog,
                &command.command,
                command.description,
            );
        }
    }

    for (title_key, title, rows) in FIXED_SECTIONS {
        out.push('\n');
        out.push_str(tr(catalog, title_key, title));
        out.push('\n');
        for fixed_row in *rows {
            push_fixed_row(&mut out, catalog, fixed_row);
        }
    }

    out.push('\n');
    out.push_str(tr(catalog, "help.section.file-dialogs", "File Dialogs"));
    out.push('\n');
    for (action, description) in FILE_DIALOG_HELP {
        push_file_dialog_help(&mut out, file_dialog_keys, catalog, *action, description);
    }
    for fixed_row in MOUSE_ROWS {
        push_fixed_row(&mut out, catalog, fixed_row);
    }

    out.push('\n');
    out.push_str(tr(catalog, "help.section.menus", "Menus"));
    out.push('\n');
    for fixed_row in MENU_ROWS {
        push_fixed_row(&mut out, catalog, fixed_row);
    }

    out.push('\n');
    out.push_str(tr(catalog, "help.section.notes", "Notes"));
    out.push('\n');
    for (key, english) in NOTE_KEYS {
        out.push_str("  ");
        out.push_str(tr(catalog, key, english));
        out.push('\n');
    }

    out
}

/// Every catalog key the help window can look up, paired with its English
/// default. This is the enumeration the translation-completeness test walks;
/// keep it in sync by construction — it derives from the same tables
/// `help_text` renders from.
#[cfg(test)]
pub(crate) fn help_translation_keys() -> Vec<(String, &'static str)> {
    let mut keys: Vec<(String, &'static str)> = vec![
        ("help.title".to_string(), "Dun Help"),
        ("help.unbound".to_string(), "(unbound)"),
        ("help.section.file-dialogs".to_string(), "File Dialogs"),
        ("help.section.menus".to_string(), "Menus"),
        ("help.section.notes".to_string(), "Notes"),
    ];
    for section in HELP_SECTIONS {
        keys.push((section.key.to_string(), section.title));
        for command in section.commands {
            keys.push((
                format!("help.command.{}", command_id(&command.command)),
                command.description,
            ));
        }
    }
    for (title_key, title, rows) in FIXED_SECTIONS {
        keys.push((title_key.to_string(), title));
        for fixed_row in *rows {
            keys.push((fixed_row.key.to_string(), fixed_row.english));
        }
    }
    for fixed_row in MOUSE_ROWS.iter().chain(MENU_ROWS) {
        keys.push((fixed_row.key.to_string(), fixed_row.english));
    }
    for (key, english) in NOTE_KEYS {
        keys.push((key.to_string(), english));
    }
    for (action, description) in FILE_DIALOG_HELP {
        keys.push((
            format!("help.command.{}", file_dialog_action_id(*action)),
            description,
        ));
    }
    keys
}

fn push_fixed_row(out: &mut String, catalog: &TextCatalog, fixed_row: &HelpRow) {
    out.push_str(&format!(
        "  {} {}\n",
        pad_to_display_width(fixed_row.keys, 17),
        tr(catalog, fixed_row.key, fixed_row.english)
    ));
}

fn push_help_command(
    out: &mut String,
    keymap: &Keymap,
    catalog: &TextCatalog,
    command: &EditorCommand,
    description: &'static str,
) {
    let sequence = keymap
        .sequence_for_command(command)
        .map(ToString::to_string)
        .unwrap_or_else(|| tr(catalog, "help.unbound", "(unbound)").to_string());
    let id = command_id(command);
    out.push_str(&format!(
        "  {} {} [{id}]\n",
        pad_to_display_width(&sequence, 15),
        tr(catalog, &format!("help.command.{id}"), description)
    ));
}

fn push_file_dialog_help(
    out: &mut String,
    keymap: &FileDialogKeymap,
    catalog: &TextCatalog,
    action: FileDialogAction,
    description: &'static str,
) {
    let sequence = file_dialog_action_key_text(keymap, action);
    let id = file_dialog_action_id(action);
    out.push_str(&format!(
        "  {} {} [{id}]\n",
        pad_to_display_width(&sequence, 15),
        tr(catalog, &format!("help.command.{id}"), description)
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

pub(crate) fn file_dialog_shortcuts_text(
    keymap: &FileDialogKeymap,
    catalog: &TextCatalog,
) -> String {
    format!(
        "[{}] {}  [{}] {}  [{}] {}  [{}] {}",
        file_dialog_action_key_text(keymap, FileDialogAction::Submit),
        ui_text::tr(catalog, ui_text::DIALOG_SHORTCUT_OK),
        file_dialog_action_key_text(keymap, FileDialogAction::CompleteForward),
        ui_text::tr(catalog, ui_text::DIALOG_SHORTCUT_COMPLETE),
        file_dialog_action_key_text(keymap, FileDialogAction::ToggleHidden),
        ui_text::tr(catalog, ui_text::DIALOG_SHORTCUT_HIDDEN),
        file_dialog_action_key_text(keymap, FileDialogAction::Cancel),
        ui_text::tr(catalog, ui_text::DIALOG_SHORTCUT_CANCEL),
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
    key: &'static str,
    title: &'static str,
    commands: &'static [HelpCommand],
}

struct HelpCommand {
    command: EditorCommand,
    description: &'static str,
}

const HELP_SECTIONS: &[HelpSection] = &[
    HelpSection {
        key: "help.section.app",
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
        key: "help.section.file",
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
                description: "Close the focused file",
            },
        ],
    },
    HelpSection {
        key: "help.section.edit",
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
        key: "help.section.windows",
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
                command: EditorCommand::Window(WindowCommand::Only),
                description: "Close every window except the focused one",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::ToggleCollapse),
                description: "Collapse or expand focused pane",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::Collapse),
                description: "Collapse focused pane",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::Expand),
                description: "Expand focused pane",
            },
            HelpCommand {
                command: EditorCommand::Window(WindowCommand::Close),
                description: "Close focused window",
            },
        ],
    },
];
