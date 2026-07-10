use dun_core::EditorCommand;

use crate::{MenuBar, MenuEntry, MenuItem, MenuSelection, UiShell};

impl UiShell {
    pub(crate) fn menu_bar(&self, active: Option<MenuSelection>) -> MenuBar {
        MenuBar {
            active,
            items: vec![
                MenuItem::new(
                    "File",
                    vec![
                        MenuEntry::new("New (N)", EditorCommand::File(dun_core::FileCommand::New)),
                        MenuEntry::new(
                            "Open... (O)",
                            EditorCommand::File(dun_core::FileCommand::Open),
                        ),
                        MenuEntry::new(
                            "Switch Buffer (B)",
                            EditorCommand::File(dun_core::FileCommand::SwitchBuffer),
                        ),
                        MenuEntry::new(
                            "Save (S)",
                            EditorCommand::File(dun_core::FileCommand::Save),
                        ),
                        MenuEntry::new(
                            "Save As... (A)",
                            EditorCommand::File(dun_core::FileCommand::SaveAs),
                        ),
                        MenuEntry::new(
                            "Reload (E)",
                            EditorCommand::File(dun_core::FileCommand::Reload),
                        ),
                        MenuEntry::new(
                            "Close (C)",
                            EditorCommand::File(dun_core::FileCommand::Close),
                        ),
                        MenuEntry::new(
                            "Run Command (R)",
                            EditorCommand::App(dun_core::AppCommand::RunCommand),
                        ),
                        MenuEntry::new(
                            "Shell Escape (H)",
                            EditorCommand::App(dun_core::AppCommand::ShellEscape),
                        ),
                        MenuEntry::new("Quit (Q)", EditorCommand::App(dun_core::AppCommand::Quit)),
                    ],
                ),
                MenuItem::new(
                    "Edit",
                    vec![
                        MenuEntry::new(
                            "Undo (U)",
                            EditorCommand::Edit(dun_core::EditCommand::Undo),
                        ),
                        MenuEntry::new(
                            "Redo (R)",
                            EditorCommand::Edit(dun_core::EditCommand::Redo),
                        ),
                        MenuEntry::new("Cut (T)", EditorCommand::Edit(dun_core::EditCommand::Cut)),
                        MenuEntry::new(
                            "Copy (C)",
                            EditorCommand::Edit(dun_core::EditCommand::Copy),
                        ),
                        MenuEntry::new(
                            "Copy External (X)",
                            EditorCommand::Edit(dun_core::EditCommand::CopyExternal),
                        ),
                        MenuEntry::new(
                            "Paste (P)",
                            EditorCommand::Edit(dun_core::EditCommand::Paste),
                        ),
                        MenuEntry::new(
                            "Select All (A)",
                            EditorCommand::Edit(dun_core::EditCommand::SelectAll),
                        ),
                        MenuEntry::new(
                            "Select Line (L)",
                            EditorCommand::Edit(dun_core::EditCommand::SelectLine),
                        ),
                        MenuEntry::new(
                            "Copy Line (Y)",
                            EditorCommand::Edit(dun_core::EditCommand::CopyLine),
                        ),
                        MenuEntry::new(
                            "Delete Line (K)",
                            EditorCommand::Edit(dun_core::EditCommand::DeleteLine),
                        ),
                        MenuEntry::new(
                            "Indent Line (I)",
                            EditorCommand::Edit(dun_core::EditCommand::IndentLine),
                        ),
                        MenuEntry::new(
                            "Outdent Line (O)",
                            EditorCommand::Edit(dun_core::EditCommand::OutdentLine),
                        ),
                        MenuEntry::new(
                            "Trim Whitespace (W)",
                            EditorCommand::Edit(dun_core::EditCommand::TrimTrailingWhitespace),
                        ),
                        MenuEntry::new(
                            "Find (F)",
                            EditorCommand::Edit(dun_core::EditCommand::Find),
                        ),
                        MenuEntry::new(
                            "Find Next (N)",
                            EditorCommand::Edit(dun_core::EditCommand::FindNext),
                        ),
                        MenuEntry::new(
                            "Replace (B)",
                            EditorCommand::Edit(dun_core::EditCommand::Replace),
                        ),
                        MenuEntry::new(
                            "Go To Line (G)",
                            EditorCommand::Edit(dun_core::EditCommand::GoToLine),
                        ),
                    ],
                ),
                MenuItem::new(
                    "View",
                    vec![
                        MenuEntry::new(
                            "Split Horizontal (H)",
                            EditorCommand::Window(dun_core::WindowCommand::SplitHorizontal),
                        ),
                        MenuEntry::new(
                            "Split Vertical (V)",
                            EditorCommand::Window(dun_core::WindowCommand::SplitVertical),
                        ),
                        MenuEntry::new(
                            "Equalize (E)",
                            EditorCommand::Window(dun_core::WindowCommand::Equalize),
                        ),
                        MenuEntry::new(
                            "Toggle Collapse (C)",
                            EditorCommand::Window(dun_core::WindowCommand::ToggleCollapse),
                        ),
                        MenuEntry::new(
                            "Word Wrap (Z)",
                            EditorCommand::Edit(dun_core::EditCommand::ToggleWordWrap),
                        ),
                        MenuEntry::new(
                            "Visible Whitespace (.)",
                            EditorCommand::Edit(dun_core::EditCommand::ToggleVisibleWhitespace),
                        ),
                        MenuEntry::new(
                            "Toggle Bookmark (M)",
                            EditorCommand::Edit(dun_core::EditCommand::ToggleBookmark),
                        ),
                        MenuEntry::new(
                            "Next Bookmark (N)",
                            EditorCommand::Edit(dun_core::EditCommand::NextBookmark),
                        ),
                        MenuEntry::new(
                            "Previous Bookmark (P)",
                            EditorCommand::Edit(dun_core::EditCommand::PreviousBookmark),
                        ),
                        MenuEntry::new(
                            "Scroll Left ([)",
                            EditorCommand::Edit(dun_core::EditCommand::ScrollLeft),
                        ),
                        MenuEntry::new(
                            "Scroll Right (])",
                            EditorCommand::Edit(dun_core::EditCommand::ScrollRight),
                        ),
                        MenuEntry::new(
                            "Close Window (X)",
                            EditorCommand::Window(dun_core::WindowCommand::Close),
                        ),
                        MenuEntry::new(
                            "Outline (/)",
                            EditorCommand::App(dun_core::AppCommand::Outline),
                        ),
                        MenuEntry::new(
                            "Search Results (W)",
                            EditorCommand::App(dun_core::AppCommand::SearchResults),
                        ),
                        MenuEntry::new(
                            "Status History (S)",
                            EditorCommand::App(dun_core::AppCommand::StatusHistory),
                        ),
                        MenuEntry::new(
                            "Config Diagnostics (D)",
                            EditorCommand::App(dun_core::AppCommand::ConfigDiagnostics),
                        ),
                        MenuEntry::new(
                            "Reload Config (R)",
                            EditorCommand::App(dun_core::AppCommand::ReloadConfig),
                        ),
                    ],
                ),
                MenuItem::new(
                    "Help",
                    vec![MenuEntry::new(
                        "Help (H)",
                        EditorCommand::App(dun_core::AppCommand::Help),
                    )],
                ),
            ],
        }
    }
}
