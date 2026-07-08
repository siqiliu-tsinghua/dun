use crate::*;

impl AppState {
    pub(crate) fn focused_path_text(&self) -> String {
        let Some(buffer_id) = self.focused_buffer_id() else {
            return String::new();
        };

        self.path_text_for_buffer(buffer_id)
    }

    pub(crate) fn path_text_for_buffer(&self, buffer_id: BufferId) -> String {
        self.buffer_state(buffer_id)
            .and_then(|buffer| buffer.path.as_ref())
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub(crate) fn buffer_display_name(&self, buffer_id: BufferId) -> String {
        if let Some(name) = self
            .buffer_state(buffer_id)
            .and_then(|buffer| buffer.path.as_ref())
            .map(|path| title_for_path(path))
        {
            return name;
        }

        self.workspace
            .windows
            .iter()
            .find(|window| window.buffer_id == buffer_id)
            .map(|window| window.title.clone())
            .unwrap_or_else(|| format!("Buffer {}", buffer_id.0))
    }

    pub(crate) fn focused_buffer_status(&self) -> String {
        let Ok(window) = self.workspace.focused_window() else {
            return "[No Window]".to_string();
        };

        let Some(buffer) = self.buffer_state(window.buffer_id) else {
            return format!("[{}]", window.title);
        };

        let mode = if buffer.encoding == FileTextEncoding::EscapedBytes {
            "Escaped Bytes"
        } else if buffer.buffer.is_read_only() {
            "Read Only"
        } else {
            "Plain Text"
        };
        let dirty = if buffer.buffer.is_dirty() { "*" } else { "" };

        format!("[{mode}{dirty}]")
    }

    pub(crate) fn focused_detail_status(&self) -> String {
        let profile = terminal_profile_status(self.shell.profile);
        let window = self.focused_window_status();
        let Some(buffer_id) = self.focused_buffer_id() else {
            return format!("[Ln -] [{profile}] [{window}]");
        };
        let Some(buffer) = self.buffer_state(buffer_id) else {
            return format!("[Ln -] [{profile}] [{window}]");
        };

        let position = buffer.buffer.cursor_position();
        let column = buffer
            .buffer
            .line(position.line)
            .and_then(|line| line.get(..position.column))
            .map(|prefix| UnicodeWidthStr::width(prefix) + 1)
            .unwrap_or(1);

        let mut parts = vec![
            bracket(line_ending_status(buffer.buffer.line_ending())),
            bracket(file_encoding_status(buffer.encoding)),
            bracket("Spaces:4"),
            format!("{}:{}", position.line + 1, column),
            bracket(&scroll_status(
                buffer,
                self.focused_buffer_view_context(self.workspace_area),
            )),
            bracket(&profile),
            bracket(&window),
        ];
        if let Some(state) = buffer_disk_state(buffer) {
            parts.insert(4, bracket(state));
        }
        if buffer.word_wrap {
            parts.insert(4, bracket("Wrap"));
        }
        if buffer.visible_whitespace {
            parts.insert(4, bracket("Whitespace"));
        }
        if buffer.bookmarks.contains(&position.line) {
            parts.insert(4, bracket("Mark"));
        }
        if let Some(selection) = selection_status(&buffer.buffer) {
            parts.insert(4, bracket(&selection));
        }
        if let Some(search) = buffer.search_status() {
            parts.insert(4, bracket(&search));
        }

        parts.join(" ")
    }

    pub(crate) fn focused_file_status(&self) -> String {
        let Some(buffer_id) = self.focused_buffer_id() else {
            return "[No file]".to_string();
        };

        let name = self.buffer_display_name(buffer_id);
        bracket(&name)
    }

    fn focused_window_status(&self) -> String {
        let total = self.workspace.window_count();
        let Some(index) = self
            .workspace
            .windows
            .iter()
            .position(|window| window.id == self.workspace.focused)
            .map(|index| index + 1)
        else {
            return format!("Win -/{total}");
        };

        format!("Win {index}/{total}")
    }
}
