use std::borrow::Cow;

use dun_core::EditorCommand;

use crate::hit::entry_mnemonic;
use crate::{MenuBar, MenuEntry, MenuItem, MenuSelection, UiShell};

impl UiShell {
    /// Compose a translated top-level label. The mnemonic letter always
    /// comes from the compiled English label ("File" -> F), so keyboard
    /// navigation is identical in every language (docs/i18n.md).
    fn menu_label(&self, key: &str, english: &'static str) -> Cow<'static, str> {
        match self.catalog.get(key) {
            None => Cow::Borrowed(english),
            Some(base) => {
                let mnemonic = english.chars().next().unwrap_or('?');
                Cow::Owned(format!("{base} ({mnemonic})"))
            }
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
        let entry = |key: &str, english: &'static str, command: EditorCommand| {
            MenuEntry::new(self.entry_label(key, english), command)
        };

        MenuBar {
            active,
            items: vec![
                MenuItem::new(
                    self.menu_label("menu.file", "File"),
                    vec![
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
                    ],
                ),
                MenuItem::new(
                    self.menu_label("menu.edit", "Edit"),
                    vec![
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
                    ],
                ),
                MenuItem::new(
                    self.menu_label("menu.view", "View"),
                    vec![
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
                    ],
                ),
                MenuItem::new(
                    self.menu_label("menu.help", "Help"),
                    vec![entry(
                        "menu.help.help",
                        "Help (H)",
                        EditorCommand::App(dun_core::AppCommand::Help),
                    )],
                ),
            ],
        }
    }
}
