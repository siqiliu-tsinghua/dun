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
        let title = ui_text::tr(&self.shell.catalog, ui_text::WINDOW_UNTITLED).to_string();
        if let Ok(window) = self.workspace.window_mut(window_id) {
            window.title = title;
            window.buffer_kind = dun_core::BufferKind::Untitled;
        }
        self.set_status(
            ui_text::tr(&self.shell.catalog, ui_text::STATUS_FILE_NEW_UNTITLED).to_string(),
        );
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

        let status = opened_file_status(&self.shell.catalog, &path, encoding);
        let status = status_with_atomic_temp_report(&self.shell.catalog, status, &temp_report);
        self.set_status(status);
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

            // Saving an unmodified buffer used to rewrite the file anyway. The
            // atomic save writes a temp file and renames it over the original,
            // so every idle Ctrl+S replaced the inode and bumped the mtime for
            // nothing -- a pointless write on the remote boxes this editor is
            // for, and a lie to anything watching the file.
            //
            // This sits *after* the read-only, encoding and no-path refusals:
            // those carry a reason the user needs, and answering a read-only
            // buffer with "no changes to save" would swap it for a misleading
            // one. It also cannot lose a rescue path -- validate_save_snapshot
            // already refuses a plain Save when the file vanished from under a
            // clean buffer, so there was nothing to rescue.
            if !buffer.buffer.is_dirty() {
                self.set_status(ui_text::tr_fmt(
                    &self.shell.catalog,
                    ui_text::STATUS_SAVE_NO_CHANGES,
                    &[&path.display().to_string()],
                ));
                return Ok(path);
            }

            validate_save_snapshot(buffer.file_snapshot, &path)?;
            (path, buffer.buffer.to_text())
        };

        let report =
            atomic_write_text_file(&path, &text).map_err(|error| path_io_error(&path, error))?;

        if let Some(buffer) = self.buffer_state_mut(buffer_id) {
            buffer.buffer.mark_saved();
            buffer.file_snapshot = current_file_snapshot(&path).ok();
        }
        let status = ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_SAVE_SAVED,
            &[&path.display().to_string()],
        );
        let status =
            status_with_atomic_temp_report(&self.shell.catalog, status, &report.temp_reconcile);
        self.set_status(status);
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

        let status = ui_text::tr_fmt(
            &self.shell.catalog,
            ui_text::STATUS_SAVE_SAVED,
            &[&path.display().to_string()],
        );
        let status =
            status_with_atomic_temp_report(&self.shell.catalog, status, &report.temp_reconcile);
        self.set_status(status);
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

        let status = reloaded_file_status(&self.shell.catalog, &path, encoding);
        let status = status_with_atomic_temp_report(&self.shell.catalog, status, &temp_report);
        self.set_status(status);
        Ok(())
    }
}
