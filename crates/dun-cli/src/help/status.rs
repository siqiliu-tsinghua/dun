use crate::*;

pub(crate) fn replacement_status_text<'a>(
    catalog: &'a TextCatalog,
    replacement: &'a str,
) -> &'a str {
    if replacement.is_empty() {
        ui_text::tr(catalog, ui_text::STATUS_REPLACEMENT_EMPTY)
    } else {
        replacement
    }
}

pub(crate) fn selection_status(buffer: &TextBuffer, mode: AmbiguousWidth) -> Option<String> {
    let range = buffer.selection_range()?;
    if range.is_empty() {
        return None;
    }

    if range.start.line == range.end.line {
        let line = buffer.line(range.start.line)?;
        let selected = &line[range.start.column..range.end.column];
        return Some(format!("Sel {}c", str_width(selected, mode)));
    }

    Some(format!("Sel {}L", range.end.line - range.start.line + 1))
}

pub(crate) fn scroll_status(
    buffer: &BufferState,
    context: Option<BufferViewContext>,
    mode: AmbiguousWidth,
) -> String {
    let total = buffer.buffer.line_count().max(1);
    let context = context.unwrap_or(BufferViewContext {
        buffer_id: buffer.id,
        body_height: 1,
        body_width: 1,
    });
    if buffer.word_wrap {
        let total_rows = buffer.wrapped_total_visual_rows(context.body_width.max(1), mode);
        let start_row = buffer
            .wrapped_top_visual_row(context.body_width.max(1), mode)
            .min(total_rows.saturating_sub(1));
        let end_row = start_row
            .saturating_add(context.body_height.max(1))
            .min(total_rows);
        let line = buffer.first_line.min(total.saturating_sub(1)) + 1;
        return format!(
            "View V{}-{end_row}/{total_rows} L{line} wrap",
            start_row + 1
        );
    }

    let height = context.body_height.max(1);
    let start = buffer.first_line.min(total.saturating_sub(1));
    let end = start.saturating_add(height).min(total);
    let max_column = buffer.max_line_display_width(mode);
    let body_width = context.body_width;
    if buffer.first_column == 0 && (body_width == 0 || max_column <= body_width) {
        format!("View {}-{end}/{total}", start + 1)
    } else {
        let column_start = buffer.first_column + 1;
        let column_end = buffer
            .first_column
            .saturating_add(body_width.max(1))
            .min(max_column.max(column_start));
        format!(
            "View {}-{end}/{total} X{}-{}/{}",
            start + 1,
            column_start,
            column_end,
            max_column.max(1)
        )
    }
}

pub(crate) fn buffer_disk_state(buffer: &BufferState) -> Option<&'static str> {
    let path = buffer.path.as_ref()?;
    let snapshot = buffer.file_snapshot?;
    match current_file_snapshot(path) {
        Ok(current) if current == snapshot => None,
        Ok(_) => Some("Disk Changed"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Some("Disk Missing"),
        Err(_) => Some("Disk ?"),
    }
}

pub(crate) fn bracket(text: &str) -> String {
    format!("[{text}]")
}

pub(crate) const fn file_encoding_status(encoding: FileTextEncoding) -> &'static str {
    match encoding {
        FileTextEncoding::Utf8 => "UTF-8",
        FileTextEncoding::EscapedBytes => "Escaped Bytes",
    }
}

pub(crate) const fn line_ending_status(line_ending: LineEnding) -> &'static str {
    match line_ending {
        LineEnding::Lf => "LF",
        LineEnding::CrLf => "CRLF",
    }
}

pub(crate) fn terminal_profile_status(profile: TerminalProfile) -> String {
    format!(
        "{}/{}",
        encoding_status(profile.encoding),
        color_status(profile.colors)
    )
}

pub(crate) fn env_config_path_text() -> String {
    env_config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(unset)".to_string())
}

pub(crate) fn default_config_path_text() -> String {
    default_config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(unavailable)".to_string())
}

pub(crate) const fn encoding_status(encoding: EncodingProfile) -> &'static str {
    match encoding {
        EncodingProfile::Utf8 => "UTF-8",
        EncodingProfile::Ascii => "ASCII",
    }
}

pub(crate) const fn color_status(colors: ColorProfile) -> &'static str {
    match colors {
        ColorProfile::Color256 => "256c",
        ColorProfile::Color16 => "16c",
        ColorProfile::Mono => "mono",
    }
}

pub(crate) fn buffer_error_text(catalog: &TextCatalog, error: BufferError) -> &str {
    let key = match error {
        BufferError::InvalidPosition(_) => ui_text::STATUS_BUFFER_ERROR_INVALID_POSITION,
        BufferError::InvalidRange(_) => ui_text::STATUS_BUFFER_ERROR_INVALID_RANGE,
        BufferError::ReadOnly => ui_text::STATUS_BUFFER_ERROR_READ_ONLY,
    };
    ui_text::tr(catalog, key)
}

pub(crate) fn workspace_error_text(catalog: &TextCatalog, error: WorkspaceError) -> &str {
    let key = match error {
        WorkspaceError::CannotCloseLastWindow => ui_text::STATUS_WORKSPACE_ERROR_CANNOT_CLOSE_LAST,
        WorkspaceError::CannotCollapseLastWindow => {
            ui_text::STATUS_WORKSPACE_ERROR_CANNOT_COLLAPSE_LAST
        }
        WorkspaceError::FocusMissing => ui_text::STATUS_WORKSPACE_ERROR_FOCUS_MISSING,
        WorkspaceError::NoNeighbor => ui_text::STATUS_WORKSPACE_ERROR_NO_NEIGHBOR,
        WorkspaceError::NoResizableSplit => ui_text::STATUS_WORKSPACE_ERROR_NO_RESIZABLE_SPLIT,
        WorkspaceError::WindowMissing => ui_text::STATUS_WORKSPACE_ERROR_WINDOW_MISSING,
    };
    ui_text::tr(catalog, key)
}

pub(crate) fn axis_name(catalog: &TextCatalog, axis: Axis) -> &str {
    let key = match axis {
        Axis::Horizontal => ui_text::STATUS_WINDOW_AXIS_HORIZONTAL,
        Axis::Vertical => ui_text::STATUS_WINDOW_AXIS_VERTICAL,
    };
    ui_text::tr(catalog, key)
}
