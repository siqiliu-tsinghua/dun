use crate::*;

impl AppState {
    pub(crate) fn reset_focused_to_untitled(&mut self) {
        let Ok(window) = self.workspace.focused_window() else {
            return;
        };
        let window_id = window.id;
        let buffer_id = window.buffer_id;

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = BufferState::new(buffer_id, TextBuffer::new_untitled());
        }
        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = "Untitled".to_string();
            window.buffer_kind = dun_core::BufferKind::Untitled;
        }
        self.set_status("New untitled buffer");
    }

    pub(crate) fn open_file_path(&mut self, path: PathBuf) -> io::Result<()> {
        let loaded = load_text_buffer(&path, self.limits.editable_file_soft_limit_bytes)
            .map_err(|error| path_io_error(&path, error))?;
        let temp_report = reconcile_atomic_save_temp_files(&path);
        self.replace_focused_buffer_with_file(path, loaded, temp_report);
        Ok(())
    }

    fn replace_focused_buffer_with_file(
        &mut self,
        path: PathBuf,
        loaded: LoadedTextBuffer,
        temp_report: AtomicTempReconcileReport,
    ) {
        let Ok(window) = self.workspace.focused_window() else {
            return;
        };
        let window_id = window.id;
        let buffer_id = window.buffer_id;
        let title = title_for_path(&path);
        let kind = loaded.buffer.kind();
        let encoding = loaded.encoding;

        if let Some(state) = self.buffer_state_mut(buffer_id) {
            *state = BufferState::from_file(buffer_id, path.clone(), loaded);
        } else {
            self.buffers
                .push(BufferState::from_file(buffer_id, path.clone(), loaded));
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = title;
            window.buffer_kind = kind;
        }

        let status = opened_file_status(&path, encoding);
        self.set_status(status_with_atomic_temp_report(status, &temp_report));
    }

    pub(crate) fn save_focused_buffer(&mut self) -> io::Result<()> {
        let buffer_id = self
            .focused_buffer_id()
            .ok_or_else(|| io::Error::other("focused window is missing"))?;
        self.save_buffer(buffer_id).map(|_| ())
    }

    pub(crate) fn save_buffer(&mut self, buffer_id: BufferId) -> io::Result<PathBuf> {
        let (path, text) = {
            let buffer = self
                .buffer_state(buffer_id)
                .ok_or_else(|| io::Error::other("buffer is missing"))?;
            if buffer.buffer.is_read_only() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer is read-only",
                ));
            }
            if !buffer.encoding.is_save_safe() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer encoding is not save-safe",
                ));
            }
            let path = buffer.path.clone().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer has no file path",
                )
            })?;
            validate_save_snapshot(buffer.file_snapshot, &path)?;
            (path, buffer.buffer.to_text())
        };

        let report =
            atomic_write_text_file(&path, &text).map_err(|error| path_io_error(&path, error))?;

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            buffer.buffer.mark_saved();
            buffer.file_snapshot = current_file_snapshot(&path).ok();
        }
        self.set_status(status_with_atomic_temp_report(
            format!("Saved {}", path.display()),
            &report.temp_reconcile,
        ));
        Ok(path)
    }

    pub(crate) fn save_focused_buffer_as(&mut self, path: PathBuf) -> io::Result<()> {
        let window = self
            .workspace
            .focused_window()
            .map_err(|_| io::Error::other("focused window is missing"))?;
        let window_id = window.id;
        let buffer_id = window.buffer_id;
        let text = {
            let buffer = self
                .buffer_state(buffer_id)
                .ok_or_else(|| io::Error::other("focused buffer is missing"))?;
            if buffer.buffer.is_read_only() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer is read-only",
                ));
            }
            if !buffer.encoding.is_save_safe() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer encoding is not save-safe",
                ));
            }
            buffer.buffer.to_text()
        };

        let report =
            atomic_write_text_file(&path, &text).map_err(|error| path_io_error(&path, error))?;

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            buffer.path = Some(path.clone());
            buffer.encoding = FileTextEncoding::Utf8;
            buffer.file_snapshot = current_file_snapshot(&path).ok();
            buffer.buffer.set_kind(dun_core::BufferKind::File);
            buffer.buffer.mark_saved();
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = title_for_path(&path);
            window.buffer_kind = dun_core::BufferKind::File;
        }

        self.set_status(status_with_atomic_temp_report(
            format!("Saved {}", path.display()),
            &report.temp_reconcile,
        ));
        Ok(())
    }

    pub(crate) fn reload_focused_buffer(&mut self) -> io::Result<()> {
        let window = self
            .workspace
            .focused_window()
            .map_err(|_| io::Error::other("focused window is missing"))?;
        let window_id = window.id;
        let buffer_id = window.buffer_id;
        let (
            path,
            cursor,
            first_line,
            first_visual_row,
            first_column,
            word_wrap,
            visible_whitespace,
            bookmarks,
        ) = {
            let buffer = self
                .buffer_state(buffer_id)
                .ok_or_else(|| io::Error::other("focused buffer is missing"))?;
            let path = buffer.path.clone().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "focused buffer has no file path",
                )
            })?;
            (
                path,
                buffer.buffer.cursor_position(),
                buffer.first_line,
                buffer.first_visual_row,
                buffer.first_column,
                buffer.word_wrap,
                buffer.visible_whitespace,
                buffer.bookmarks.clone(),
            )
        };

        let loaded = load_text_buffer(&path, self.limits.editable_file_soft_limit_bytes)
            .map_err(|error| path_io_error(&path, error))?;
        let temp_report = reconcile_atomic_save_temp_files(&path);
        let title = title_for_path(&path);
        let kind = loaded.buffer.kind();
        let encoding = loaded.encoding;
        let mut reloaded = BufferState::from_file(buffer_id, path.clone(), loaded);
        reloaded.word_wrap = word_wrap;
        reloaded.visible_whitespace = visible_whitespace;
        reloaded.bookmarks = bookmarks;
        reloaded.normalize_bookmarks();
        let line = cursor
            .line
            .min(reloaded.buffer.line_count().saturating_sub(1));
        let column = reloaded.clamp_column_to_line(line, cursor.column);
        let _ = reloaded.buffer.set_cursor(Position::new(line, column));
        reloaded.first_line = first_line.min(reloaded.buffer.line_count().saturating_sub(1));
        reloaded.first_column = if reloaded.word_wrap { 0 } else { first_column };
        reloaded.first_visual_row = if reloaded.word_wrap {
            first_visual_row
        } else {
            0
        };

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            *buffer = reloaded;
        } else {
            self.buffers.push(reloaded);
        }

        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = title;
            window.buffer_kind = kind;
        }

        let status = reloaded_file_status(&path, encoding);
        self.set_status(status_with_atomic_temp_report(status, &temp_report));
        Ok(())
    }
}
