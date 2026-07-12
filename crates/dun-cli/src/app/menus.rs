use crate::*;

impl AppState {
    pub(crate) fn menu_selection(&self) -> Option<MenuSelection> {
        self.active_menu.map(|menu_index| MenuSelection {
            menu_index,
            entry_index: self.active_menu_entry,
        })
    }

    pub(crate) fn clear_active_menu(&mut self) {
        self.active_menu = None;
        self.active_menu_entry = None;
    }

    fn first_menu_entry(&self, menu_index: usize) -> Option<usize> {
        self.shell
            .menu_entry_count(menu_index)
            .filter(|count| *count > 0)
            .map(|_| 0)
    }

    pub(crate) fn open_mouse_menu(&mut self, menu_index: usize) {
        if self.active_menu == Some(menu_index) {
            self.clear_active_menu();
        } else {
            self.active_menu = Some(menu_index);
            self.active_menu_entry = None;
        }
    }

    pub(crate) fn open_keyboard_menu(&mut self, menu_index: usize) {
        self.active_menu = Some(menu_index);
        self.active_menu_entry = self.first_menu_entry(menu_index);
    }

    pub(crate) fn move_active_menu(&mut self, delta: isize) -> bool {
        let Some(current) = self.active_menu else {
            return false;
        };
        let menu_count = self.shell.menu_count();
        if menu_count == 0 {
            self.clear_active_menu();
            return true;
        }

        let next = wrapping_index(current, menu_count, delta);
        let keep_entry_selected = self.active_menu_entry.is_some();
        self.active_menu = Some(next);
        self.active_menu_entry = if keep_entry_selected {
            self.first_menu_entry(next)
        } else {
            None
        };
        true
    }

    pub(crate) fn move_active_menu_entry(&mut self, delta: isize) -> bool {
        let Some(menu_index) = self.active_menu else {
            return false;
        };
        let Some(count) = self.shell.menu_entry_count(menu_index) else {
            return false;
        };
        if count == 0 {
            self.active_menu_entry = None;
            return true;
        }

        let current = self.active_menu_entry.unwrap_or(0);
        self.active_menu_entry = Some(wrapping_index(current, count, delta));
        true
    }

    pub(crate) fn dispatch_active_menu_entry(&mut self) -> bool {
        let Some(menu_index) = self.active_menu else {
            return false;
        };
        let entry_index = self.active_menu_entry.unwrap_or(0);
        self.dispatch_menu_entry(menu_index, entry_index)
    }

    /// Runs the open dropdown's entry whose label advertises `ch` in trailing
    /// parens ("Open... (O)"). Returns false when nothing matches, so an
    /// unrelated key leaves the menu open instead of silently closing it.
    pub(crate) fn dispatch_active_menu_mnemonic(&mut self, ch: char) -> bool {
        let Some(menu_index) = self.active_menu else {
            return false;
        };
        let Some(entry_index) = self.shell.menu_entry_index_for_mnemonic(menu_index, ch) else {
            return false;
        };
        self.dispatch_menu_entry(menu_index, entry_index)
    }

    fn dispatch_menu_entry(&mut self, menu_index: usize, entry_index: usize) -> bool {
        let Some(command) = self.shell.menu_entry_command(menu_index, entry_index) else {
            return false;
        };

        self.clear_active_menu();
        self.pending_keys.clear();
        self.handle_command(&command);
        true
    }
}
