use std::borrow::Cow;

use dun_core::EditorCommand;

use crate::hit::entry_mnemonic;
use crate::{MenuBar, MenuEntry, MenuItem, MenuSelection, UiShell};

/// One dropdown entry: its catalog key, its compiled English label (which
/// carries the mnemonic in trailing parens), and the command it runs.
struct EntrySpec {
    key: &'static str,
    english: &'static str,
    command: EditorCommand,
}

/// One top-level menu. The English label's first letter is its mnemonic.
struct MenuSpec {
    key: &'static str,
    english: &'static str,
    entries: &'static [EntrySpec],
}

const fn entry(key: &'static str, english: &'static str, command: EditorCommand) -> EntrySpec {
    EntrySpec {
        key,
        english,
        command,
    }
}

const FILE_ENTRIES: &[EntrySpec] = &[
    entry(
        "menu.file.new",
        "New (N)",
        EditorCommand::File(dun_core::FileCommand::New),
    ),
    entry(
        "menu.file.open",
        "Open... (O)",
        EditorCommand::File(dun_core::FileCommand::Open),
    ),
    entry(
        "menu.file.switch-buffer",
        "Switch Buffer (B)",
        EditorCommand::File(dun_core::FileCommand::SwitchBuffer),
    ),
    entry(
        "menu.file.save",
        "Save (S)",
        EditorCommand::File(dun_core::FileCommand::Save),
    ),
    entry(
        "menu.file.save-as",
        "Save As... (A)",
        EditorCommand::File(dun_core::FileCommand::SaveAs),
    ),
    entry(
        "menu.file.reload",
        "Reload (E)",
        EditorCommand::File(dun_core::FileCommand::Reload),
    ),
    entry(
        "menu.file.close",
        "Close (C)",
        EditorCommand::File(dun_core::FileCommand::Close),
    ),
    entry(
        "menu.file.run-command",
        "Run Command (R)",
        EditorCommand::App(dun_core::AppCommand::RunCommand),
    ),
    entry(
        "menu.file.shell-escape",
        "Shell Escape (H)",
        EditorCommand::App(dun_core::AppCommand::ShellEscape),
    ),
    entry(
        "menu.file.quit",
        "Quit (Q)",
        EditorCommand::App(dun_core::AppCommand::Quit),
    ),
];

const EDIT_ENTRIES: &[EntrySpec] = &[
    entry(
        "menu.edit.undo",
        "Undo (U)",
        EditorCommand::Edit(dun_core::EditCommand::Undo),
    ),
    entry(
        "menu.edit.redo",
        "Redo (R)",
        EditorCommand::Edit(dun_core::EditCommand::Redo),
    ),
    entry(
        "menu.edit.cut",
        "Cut (T)",
        EditorCommand::Edit(dun_core::EditCommand::Cut),
    ),
    entry(
        "menu.edit.copy",
        "Copy (C)",
        EditorCommand::Edit(dun_core::EditCommand::Copy),
    ),
    entry(
        "menu.edit.copy-external",
        "Copy External (X)",
        EditorCommand::Edit(dun_core::EditCommand::CopyExternal),
    ),
    entry(
        "menu.edit.paste-external",
        "Paste External (E)",
        EditorCommand::Edit(dun_core::EditCommand::PasteExternal),
    ),
    entry(
        "menu.edit.paste",
        "Paste (P)",
        EditorCommand::Edit(dun_core::EditCommand::Paste),
    ),
    entry(
        "menu.edit.select-all",
        "Select All (A)",
        EditorCommand::Edit(dun_core::EditCommand::SelectAll),
    ),
    entry(
        "menu.edit.select-line",
        "Select Line (L)",
        EditorCommand::Edit(dun_core::EditCommand::SelectLine),
    ),
    entry(
        "menu.edit.copy-line",
        "Copy Line (Y)",
        EditorCommand::Edit(dun_core::EditCommand::CopyLine),
    ),
    entry(
        "menu.edit.delete-line",
        "Delete Line (K)",
        EditorCommand::Edit(dun_core::EditCommand::DeleteLine),
    ),
    entry(
        "menu.edit.indent-line",
        "Indent Line (I)",
        EditorCommand::Edit(dun_core::EditCommand::IndentLine),
    ),
    entry(
        "menu.edit.outdent-line",
        "Outdent Line (O)",
        EditorCommand::Edit(dun_core::EditCommand::OutdentLine),
    ),
    entry(
        "menu.edit.trim-whitespace",
        "Trim Whitespace (W)",
        EditorCommand::Edit(dun_core::EditCommand::TrimTrailingWhitespace),
    ),
    entry(
        "menu.edit.find",
        "Find (F)",
        EditorCommand::Edit(dun_core::EditCommand::Find),
    ),
    entry(
        "menu.edit.find-next",
        "Find Next (N)",
        EditorCommand::Edit(dun_core::EditCommand::FindNext),
    ),
    entry(
        "menu.edit.replace",
        "Replace (B)",
        EditorCommand::Edit(dun_core::EditCommand::Replace),
    ),
    entry(
        "menu.edit.go-to-line",
        "Go To Line (G)",
        EditorCommand::Edit(dun_core::EditCommand::GoToLine),
    ),
];

const VIEW_ENTRIES: &[EntrySpec] = &[
    entry(
        "menu.view.split-horizontal",
        "Split Horizontal (H)",
        EditorCommand::Window(dun_core::WindowCommand::SplitHorizontal),
    ),
    entry(
        "menu.view.split-vertical",
        "Split Vertical (V)",
        EditorCommand::Window(dun_core::WindowCommand::SplitVertical),
    ),
    entry(
        "menu.view.equalize",
        "Equalize (E)",
        EditorCommand::Window(dun_core::WindowCommand::Equalize),
    ),
    entry(
        "menu.view.only-window",
        "Only Window (O)",
        EditorCommand::Window(dun_core::WindowCommand::Only),
    ),
    entry(
        "menu.view.toggle-collapse",
        "Toggle Collapse (C)",
        EditorCommand::Window(dun_core::WindowCommand::ToggleCollapse),
    ),
    entry(
        "menu.view.collapse",
        "Collapse (M)",
        EditorCommand::Window(dun_core::WindowCommand::Collapse),
    ),
    entry(
        "menu.view.expand",
        "Expand (P)",
        EditorCommand::Window(dun_core::WindowCommand::Expand),
    ),
    entry(
        "menu.view.word-wrap",
        "Word Wrap (Z)",
        EditorCommand::Edit(dun_core::EditCommand::ToggleWordWrap),
    ),
    entry(
        "menu.view.visible-whitespace",
        "Visible Whitespace (.)",
        EditorCommand::Edit(dun_core::EditCommand::ToggleVisibleWhitespace),
    ),
    entry(
        "menu.view.toggle-bookmark",
        "Toggle Bookmark (K)",
        EditorCommand::Edit(dun_core::EditCommand::ToggleBookmark),
    ),
    entry(
        "menu.view.next-bookmark",
        "Next Bookmark (N)",
        EditorCommand::Edit(dun_core::EditCommand::NextBookmark),
    ),
    entry(
        "menu.view.previous-bookmark",
        "Previous Bookmark (L)",
        EditorCommand::Edit(dun_core::EditCommand::PreviousBookmark),
    ),
    entry(
        "menu.view.scroll-left",
        "Scroll Left ([)",
        EditorCommand::Edit(dun_core::EditCommand::ScrollLeft),
    ),
    entry(
        "menu.view.scroll-right",
        "Scroll Right (])",
        EditorCommand::Edit(dun_core::EditCommand::ScrollRight),
    ),
    entry(
        "menu.view.close-window",
        "Close Window (X)",
        EditorCommand::Window(dun_core::WindowCommand::Close),
    ),
    entry(
        "menu.view.search-results",
        "Search Results (W)",
        EditorCommand::App(dun_core::AppCommand::SearchResults),
    ),
    entry(
        "menu.view.status-history",
        "Status History (S)",
        EditorCommand::App(dun_core::AppCommand::StatusHistory),
    ),
    entry(
        "menu.view.config-diagnostics",
        "Config Diagnostics (D)",
        EditorCommand::App(dun_core::AppCommand::ConfigDiagnostics),
    ),
    entry(
        "menu.view.reload-config",
        "Reload Config (R)",
        EditorCommand::App(dun_core::AppCommand::ReloadConfig),
    ),
];

const HELP_ENTRIES: &[EntrySpec] = &[entry(
    "menu.help.help",
    "Help (H)",
    EditorCommand::App(dun_core::AppCommand::Help),
)];

/// The menu, declared once. `menu_bar` renders it and
/// `menu_translation_keys` enumerates it, so a key cannot exist in one
/// and be missing from the other (that drift is what let a shipped
/// translation lose a menu label without any test noticing).
const MENUS: &[MenuSpec] = &[
    MenuSpec {
        key: "menu.file",
        english: "File",
        entries: FILE_ENTRIES,
    },
    MenuSpec {
        key: "menu.edit",
        english: "Edit",
        entries: EDIT_ENTRIES,
    },
    MenuSpec {
        key: "menu.view",
        english: "View",
        entries: VIEW_ENTRIES,
    },
    MenuSpec {
        key: "menu.help",
        english: "Help",
        entries: HELP_ENTRIES,
    },
];

/// Derive the invariant mnemonic for an English top-level menu label.
///
/// The source-label rule accepts only an ASCII letter in the first position
/// and normalizes it to uppercase. It differs from rendered-label matching
/// because a translated label carries this English mnemonic in trailing
/// parentheses rather than at the start of its translated base text.
pub fn english_menu_mnemonic(label: &str) -> Option<char> {
    let mnemonic = label.chars().next()?;
    mnemonic
        .is_ascii_alphabetic()
        .then(|| mnemonic.to_ascii_uppercase())
}

/// Compose the rendered form of a translated top-level menu label.
///
/// This rule appends the mnemonic derived from the English source label;
/// rendered-label matching differs because it must read that suffix before
/// falling back to the first character of an untranslated label.
pub fn compose_translated_menu_label(base: &str, mnemonic: char) -> String {
    format!("{base} ({mnemonic})")
}

/// Iterate the invariant top-level mnemonics derived directly from `MENUS`.
///
/// Built-in source labels use their first ASCII letter, while rendered-label
/// matching must prefer the appended suffix on translated labels. A label
/// without a valid source mnemonic is therefore a bug in the built-in table.
pub fn built_in_menu_mnemonics() -> impl Iterator<Item = char> {
    MENUS.iter().map(|menu| {
        english_menu_mnemonic(menu.english)
            .expect("built-in menu labels must start with an ASCII letter")
    })
}

/// Every catalog key the menu bar can look up, with its English default.
/// The translation-completeness test walks this so a new language file cannot
/// ship with untranslated menus.
pub fn menu_translation_keys() -> Vec<(&'static str, &'static str)> {
    let mut keys = Vec::new();
    for menu in MENUS {
        keys.push((menu.key, menu.english));
        for entry in menu.entries {
            keys.push((entry.key, entry.english));
        }
    }
    keys
}

impl UiShell {
    /// Compose a translated top-level label. The mnemonic letter always
    /// comes from the compiled English label ("File" -> F), so keyboard
    /// navigation is identical in every language (docs/i18n.md).
    fn menu_label(&self, key: &str, english: &'static str) -> Cow<'static, str> {
        match self.catalog.get(key) {
            None => Cow::Borrowed(english),
            // No usable mnemonic means the built-in table is wrong, which
            // `built_in_top_level_mnemonics_are_unique_ascii_letters` catches
            // at test time. Do NOT panic here: this runs on every frame, and
            // killing a live editing session over a cosmetic table mistake is
            // a far worse outcome than showing one menu untranslated. Falling
            // back to the English label also keeps the menu reachable, since
            // matching falls back to a label's first character.
            Some(base) => match english_menu_mnemonic(english) {
                Some(mnemonic) => Cow::Owned(compose_translated_menu_label(base, mnemonic)),
                None => Cow::Borrowed(english),
            },
        }
    }

    /// Compose a translated dropdown label, keeping the English trailing
    /// "(M)" mnemonic: "New (N)" + "新建" -> "新建 (N)".
    fn entry_label(&self, key: &str, english: &'static str) -> Cow<'static, str> {
        match self.catalog.get(key) {
            None => Cow::Borrowed(english),
            Some(base) => match entry_mnemonic(english) {
                Some(mnemonic) => Cow::Owned(format!("{base} ({mnemonic})")),
                None => Cow::Owned(base.to_string()),
            },
        }
    }

    pub(crate) fn menu_bar(&self, active: Option<MenuSelection>) -> MenuBar {
        let mut items: Vec<MenuItem> = MENUS
            .iter()
            .map(|menu| {
                MenuItem::new(
                    self.menu_label(menu.key, menu.english),
                    menu.entries
                        .iter()
                        .map(|entry| {
                            MenuEntry::new(
                                self.entry_label(entry.key, entry.english),
                                entry.command.clone(),
                            )
                        })
                        .collect(),
                )
            })
            .collect();
        // Plugin menus are pre-resolved by the caller; they trail the built-in
        // menus so their menu indices are stable for dispatch and hit testing.
        items.extend(self.plugin_menu_items.iter().cloned());
        MenuBar { active, items }
    }
}
