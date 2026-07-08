#![allow(unused_imports)]

use super::support::*;

#[test]
fn from_path_opens_utf8_file_path() {
    let path = temp_file_path("open.txt");
    std::fs::write(&path, "one\r\ntwo").unwrap();

    let app = AppState::from_path(Some(path.clone())).unwrap();
    let state = app.buffer_state(BufferId(1)).unwrap();

    assert_eq!(state.path.as_ref(), Some(&path));
    assert_eq!(state.encoding, FileTextEncoding::Utf8);
    assert_eq!(state.buffer.line(0), Some("one"));
    assert_eq!(state.buffer.line(1), Some("two"));
    assert_eq!(state.buffer.line_ending(), dun_core::LineEnding::CrLf);
    assert!(!state.buffer.is_dirty());
    assert_eq!(
        app.workspace.focused_window().unwrap().title,
        title_for_path(&path)
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn stable_file_read_validation_accepts_unchanged_file() {
    let path = temp_file_path("stable-read.txt");
    std::fs::write(&path, "stable").unwrap();
    let metadata = std::fs::metadata(&path).unwrap();
    let snapshot = FileReadSnapshot::from_metadata(&metadata);

    validate_stable_file_read(&path, snapshot, metadata.len()).unwrap();

    let _ = std::fs::remove_file(path);
}

#[test]
fn stable_file_read_validation_rejects_truncated_file() {
    let path = temp_file_path("truncated-read.txt");
    std::fs::write(&path, "stable").unwrap();
    let metadata = std::fs::metadata(&path).unwrap();
    let snapshot = FileReadSnapshot::from_metadata(&metadata);
    std::fs::write(&path, "x").unwrap();

    let error = validate_stable_file_read(&path, snapshot, metadata.len()).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "file changed while reading; retry open");

    let _ = std::fs::remove_file(path);
}

#[test]
fn stable_file_read_validation_rejects_deleted_file() {
    let path = temp_file_path("deleted-read.txt");
    std::fs::write(&path, "stable").unwrap();
    let metadata = std::fs::metadata(&path).unwrap();
    let snapshot = FileReadSnapshot::from_metadata(&metadata);
    std::fs::remove_file(&path).unwrap();

    let error = validate_stable_file_read(&path, snapshot, metadata.len()).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "file changed while reading; retry open");
}

#[cfg(unix)]
#[test]
fn stable_file_read_validation_rejects_same_size_replacement() {
    let path = temp_file_path("replaced-read.txt");
    let replacement = temp_file_path("replaced-read-next.txt");
    std::fs::write(&path, "aaaa").unwrap();
    let metadata = std::fs::metadata(&path).unwrap();
    let snapshot = FileReadSnapshot::from_metadata(&metadata);
    std::fs::write(&replacement, "bbbb").unwrap();
    std::fs::rename(&replacement, &path).unwrap();

    let error = validate_stable_file_read(&path, snapshot, metadata.len()).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "file changed while reading; retry open");

    let _ = std::fs::remove_file(path);
}

#[test]
fn invalid_utf8_file_path_opens_read_only_fallback() {
    let path = temp_file_path("invalid.txt");
    std::fs::write(&path, [b'o', b'k', 0xff, b'\n', b'\\', b'\t', 0xe4]).unwrap();

    let app = AppState::from_path(Some(path.clone())).unwrap();
    let state = app.buffer_state(BufferId(1)).unwrap();
    let window = app.workspace.focused_window().unwrap();

    assert!(state.buffer.is_read_only());
    assert_eq!(state.buffer.kind(), BufferKind::ReadOnly);
    assert_eq!(state.encoding, FileTextEncoding::EscapedBytes);
    assert_eq!(state.buffer.to_text(), "ok\\xFF\n\\\\\\x09\\xE4");
    assert_eq!(state.path.as_ref(), Some(&path));
    assert_eq!(window.buffer_kind, BufferKind::ReadOnly);
    assert_eq!(app.focused_buffer_status(), "[Escaped Bytes]");
    assert_eq!(app.focused_file_status(), bracket(&title_for_path(&path)));
    assert!(app.focused_detail_status().contains("[Escaped Bytes]"));
    assert_eq!(
        app.status_message,
        Some(format!(
            "Opened {} read-only: non-UTF-8 bytes shown as escapes",
            path.display()
        ))
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn invalid_utf8_fallback_preserves_valid_unicode_segments() {
    let path = temp_file_path("invalid-with-unicode.txt");
    std::fs::write(&path, [b'a', 0xe4, 0xb8, 0xad, 0xff, b'b']).unwrap();

    let app = AppState::from_path(Some(path.clone())).unwrap();
    let state = app.buffer_state(BufferId(1)).unwrap();

    assert!(state.buffer.is_read_only());
    assert_eq!(state.encoding, FileTextEncoding::EscapedBytes);
    assert_eq!(state.buffer.to_text(), "a中\\xFFb");

    let _ = std::fs::remove_file(path);
}

#[test]
fn save_rejects_read_only_invalid_utf8_fallback() {
    let path = temp_file_path("invalid-save.txt");
    std::fs::write(&path, [0xff, b'a']).unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();

    app.handle_command(&EditorCommand::File(FileCommand::Save));

    assert_eq!(
        app.status_message,
        Some("Save failed: focused buffer is read-only".to_string())
    );
    assert_eq!(std::fs::read(&path).unwrap(), vec![0xff, b'a']);

    let _ = std::fs::remove_file(path);
}

#[test]
fn save_as_rejects_read_only_invalid_utf8_fallback() {
    let path = temp_file_path("invalid-save-as.txt");
    let target = temp_file_path("invalid-save-as-target.txt");
    std::fs::write(&path, [0xff, b'a']).unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();

    app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
    send_text(&mut app, &target.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(
        app.status_message,
        Some("Save As failed: focused buffer is read-only".to_string())
    );
    assert!(!target.exists());

    let _ = std::fs::remove_file(path);
}

#[test]
fn file_at_editable_soft_limit_is_accepted() {
    let path = temp_file_path("soft-limit-ok.txt");
    std::fs::write(&path, "abcd").unwrap();
    let config = config_with_editable_file_soft_limit(4);

    let app = AppState::from_config_path(config, Some(path.clone())).unwrap();

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "abcd"
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn file_over_editable_soft_limit_is_rejected_before_editing() {
    let path = temp_file_path("soft-limit-large.txt");
    std::fs::write(&path, "abcd").unwrap();
    let config = config_with_editable_file_soft_limit(3);

    let error = match AppState::from_config_path(config, Some(path.clone())) {
        Ok(_) => panic!("file above editable soft limit should be rejected"),
        Err(error) => error,
    };

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("too large for editable mode"));
    assert!(error.to_string().contains("3 byte soft limit"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn open_prompt_reports_file_over_editable_soft_limit() {
    let path = temp_file_path("prompt-soft-limit-large.txt");
    std::fs::write(&path, "abcd").unwrap();
    let mut app = AppState::from_config(config_with_editable_file_soft_limit(3));

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &path.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "");
    assert_eq!(state.path, None);
    let status = app.status_message.as_deref().unwrap_or_default();
    assert!(status.starts_with("Open failed: "));
    assert!(status.contains("too large for editable mode"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn open_prompt_reports_missing_file_with_path() {
    let path = temp_file_path("missing-open.txt");
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &path.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    assert_eq!(
        app.status_message,
        Some(format!("Open failed: {}: not found", path.display()))
    );
    assert!(app.file_dialog.is_some());
    assert!(
        app.active_overlay()
            .unwrap()
            .lines
            .iter()
            .any(|line| line.contains("Open failed:"))
    );
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");
}

#[test]
fn open_command_reports_directory_path() {
    let path = temp_file_path("open-dir");
    std::fs::create_dir(&path).unwrap();
    let mut app = AppState::new();

    app.run_open_command(&[path.to_string_lossy().into_owned()]);

    assert_eq!(
        app.status_message,
        Some(format!(
            "Open failed: {}: path is a directory",
            path.display()
        ))
    );
    assert_eq!(app.buffer_state(BufferId(1)).unwrap().buffer.to_text(), "");

    let _ = std::fs::remove_dir(path);
}

#[test]
fn save_command_writes_focused_file_buffer() {
    let path = temp_file_path("save.txt");
    std::fs::write(&path, "old").unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
    app.handle_text_input('!');
    app.handle_command(&EditorCommand::File(FileCommand::Save));

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old!");
    assert!(!state.buffer.is_dirty());
    assert_eq!(
        app.status_message,
        Some(format!("Saved {}", path.display()))
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn save_refuses_external_file_change() {
    let path = temp_file_path("save-external-change.txt");
    std::fs::write(&path, "old").unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();
    std::fs::write(&path, "external change").unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
    app.handle_text_input('!');
    app.handle_command(&EditorCommand::File(FileCommand::Save));

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "external change");
    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|status| status.contains("file changed on disk"))
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn reload_command_refreshes_focused_file_buffer() {
    let path = temp_file_path("reload-file.txt");
    std::fs::write(&path, "old").unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();
    std::fs::write(&path, "new").unwrap();

    app.handle_command(&EditorCommand::File(FileCommand::Reload));

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "new"
    );
    assert_eq!(
        app.status_message,
        Some(format!("Reloaded {}", path.display()))
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn save_command_cleans_atomic_temp_file() {
    let path = temp_file_path("atomic-save.txt");
    std::fs::write(&path, "old").unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
    app.handle_text_input('!');
    app.handle_command(&EditorCommand::File(FileCommand::Save));

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old!");
    assert!(atomic_temp_files_for(&path).is_empty());

    let _ = std::fs::remove_file(path);
}

#[test]
fn open_cleans_stale_atomic_save_temp_file() {
    let path = temp_file_path("stale-open-cleanup.txt");
    let stale_temp = write_atomic_temp_file_for(&path, 0, "stale");
    std::fs::write(&path, "current").unwrap();

    let app = AppState::from_path(Some(path.clone())).unwrap();

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "current"
    );
    assert!(!stale_temp.exists());
    assert_eq!(
        app.status_message,
        Some(format!(
            "Opened {}; cleaned 1 stale save temp file(s)",
            path.display()
        ))
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn open_reports_newer_atomic_save_recovery_temp_file() {
    let path = temp_file_path("recovery-open-warning.txt");
    std::fs::write(&path, "current").unwrap();
    let recovery_temp = write_newer_atomic_temp_file_for(&path, "recovered");

    let app = AppState::from_path(Some(path.clone())).unwrap();

    assert_eq!(
        app.buffer_state(BufferId(1)).unwrap().buffer.to_text(),
        "current"
    );
    assert!(recovery_temp.exists());
    let status = app.status_message.as_deref().unwrap_or_default();
    assert!(status.starts_with(&format!(
        "Opened {}; recovery temp file found: ",
        path.display()
    )));
    assert!(status.contains(&recovery_temp.display().to_string()));

    let _ = std::fs::remove_file(recovery_temp);
    let _ = std::fs::remove_file(path);
}

#[test]
fn save_cleans_stale_atomic_save_temp_file() {
    let path = temp_file_path("stale-save-cleanup.txt");
    std::fs::write(&path, "old").unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();
    let stale_temp = write_atomic_temp_file_for(&path, 0, "stale");
    make_destination_at_least_as_new_as(&path, &stale_temp, "old");
    app.buffer_state_mut(BufferId(1)).unwrap().file_snapshot = current_file_snapshot(&path).ok();

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
    app.handle_text_input('!');
    app.handle_command(&EditorCommand::File(FileCommand::Save));

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old!");
    assert!(!stale_temp.exists());
    assert_eq!(
        app.status_message,
        Some(format!(
            "Saved {}; cleaned 1 stale save temp file(s)",
            path.display()
        ))
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn save_preserves_newer_atomic_save_recovery_temp_file() {
    let path = temp_file_path("recovery-save-warning.txt");
    std::fs::write(&path, "old").unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();
    let recovery_temp = write_newer_atomic_temp_file_for(&path, "recovered");

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
    app.handle_text_input('!');
    app.handle_command(&EditorCommand::File(FileCommand::Save));

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old!");
    assert!(recovery_temp.exists());
    let status = app.status_message.as_deref().unwrap_or_default();
    assert!(status.starts_with(&format!(
        "Saved {}; recovery temp file found: ",
        path.display()
    )));
    assert!(status.contains(&recovery_temp.display().to_string()));

    let _ = std::fs::remove_file(recovery_temp);
    let _ = std::fs::remove_file(path);
}

#[test]
fn save_rejects_read_only_target_without_replacing_it() {
    let path = temp_file_path("readonly-save.txt");
    std::fs::write(&path, "old").unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
    app.handle_text_input('!');
    set_path_readonly(&path, true);

    app.handle_command(&EditorCommand::File(FileCommand::Save));

    set_path_readonly(&path, false);
    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old");
    assert!(state.buffer.is_dirty());
    assert_eq!(
        app.status_message,
        Some(format!(
            "Save failed: {}: destination is read-only",
            path.display()
        ))
    );
    assert!(atomic_temp_files_for(&path).is_empty());

    let _ = std::fs::remove_file(path);
}

#[cfg(unix)]
#[test]
fn save_through_symlink_preserves_link_and_updates_target() {
    let target = temp_file_path("atomic-symlink-target.txt");
    let link = temp_file_path("atomic-symlink-link.txt");
    std::fs::write(&target, "old").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let mut app = AppState::from_path(Some(link.clone())).unwrap();

    app.handle_command(&EditorCommand::Edit(EditCommand::MoveLineEnd));
    app.handle_text_input('!');
    app.handle_command(&EditorCommand::File(FileCommand::Save));

    assert_eq!(std::fs::read_to_string(&target).unwrap(), "old!");
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        app.status_message,
        Some(format!("Saved {}", link.display()))
    );

    let _ = std::fs::remove_file(link);
    let _ = std::fs::remove_file(target);
}

#[test]
fn save_without_path_reports_status_message() {
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Save));

    assert_eq!(
        app.status_message,
        Some("Save failed: focused buffer has no file path".to_string())
    );
}

#[test]
fn new_command_clears_loaded_file_metadata() {
    let path = temp_file_path("new.txt");
    std::fs::write(&path, "loaded").unwrap();
    let mut app = AppState::from_path(Some(path.clone())).unwrap();

    app.handle_command(&EditorCommand::File(FileCommand::New));

    let state = app.buffer_state(BufferId(1)).unwrap();
    let window = app.workspace.focused_window().unwrap();
    assert_eq!(state.path, None);
    assert_eq!(state.buffer.kind(), dun_core::BufferKind::Untitled);
    assert_eq!(state.buffer.to_text(), "");
    assert_eq!(window.title, "Untitled");
    assert_eq!(window.buffer_kind, dun_core::BufferKind::Untitled);

    let _ = std::fs::remove_file(path);
}

#[test]
fn open_command_uses_prompt_to_load_file() {
    let path = temp_file_path("prompt-open.txt");
    std::fs::write(&path, "opened").unwrap();
    let mut app = AppState::new();

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    assert_eq!(app.prompt_status_text(), Some("Open: ".to_string()));

    send_text(&mut app, &path.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "opened");
    assert_eq!(state.path.as_ref(), Some(&path));
    assert_eq!(
        app.status_message,
        Some(format!("Opened {}", path.display()))
    );
    assert_eq!(app.prompt_status_text(), None);

    let _ = std::fs::remove_file(path);
}

#[test]
fn save_as_prompt_writes_and_attaches_path() {
    let path = temp_file_path("prompt-save-as.txt");
    let mut app = AppState::new();
    app.handle_text_input('x');

    app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
    assert_eq!(app.prompt_status_text(), Some("Save As: ".to_string()));

    send_text(&mut app, &path.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    let window = app.workspace.focused_window().unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
    assert_eq!(state.path.as_ref(), Some(&path));
    assert_eq!(state.buffer.kind(), dun_core::BufferKind::File);
    assert!(!state.buffer.is_dirty());
    assert_eq!(window.buffer_kind, dun_core::BufferKind::File);
    assert_eq!(window.title, title_for_path(&path));

    let _ = std::fs::remove_file(path);
}

#[test]
fn save_as_reports_missing_parent_directory() {
    let parent = temp_file_path("missing-save-parent");
    let path = parent.join("out.txt");
    let mut app = AppState::new();
    app.handle_text_input('x');

    app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
    send_text(&mut app, &path.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(
        app.status_message,
        Some(format!(
            "Save As failed: {}: parent directory does not exist",
            path.display()
        ))
    );
    assert!(state.buffer.is_dirty());
    assert_eq!(state.path, None);
    assert!(!path.exists());
}

#[test]
fn save_as_reports_directory_destination() {
    let path = temp_file_path("save-as-dir");
    std::fs::create_dir(&path).unwrap();
    let mut app = AppState::new();
    app.handle_text_input('x');

    app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
    send_text(&mut app, &path.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(
        app.status_message,
        Some(format!(
            "Save As failed: {}: destination is a directory",
            path.display()
        ))
    );
    assert!(state.buffer.is_dirty());
    assert_eq!(state.path, None);

    let _ = std::fs::remove_dir(path);
}

#[test]
fn open_command_can_discard_dirty_buffer_before_prompt() {
    let path = temp_file_path("confirm-open.txt");
    std::fs::write(&path, "opened").unwrap();
    let mut app = AppState::new();
    app.handle_text_input('x');

    app.handle_command(&EditorCommand::File(FileCommand::Open));
    assert_eq!(
        app.confirm_status_text(),
        Some("Unsaved changes in Untitled: Save(s) Discard(d) Cancel(c)".to_string())
    );

    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Char('d'), CrosstermKeyModifiers::NONE),
    );
    assert_eq!(app.prompt_status_text(), Some("Open: ".to_string()));

    send_text(&mut app, &path.to_string_lossy());
    handle_key_event(
        &mut app,
        CrosstermKeyEvent::new(CrosstermKeyCode::Enter, CrosstermKeyModifiers::NONE),
    );

    let state = app.buffer_state(BufferId(1)).unwrap();
    assert_eq!(state.buffer.to_text(), "opened");
    assert_eq!(state.path.as_ref(), Some(&path));

    let _ = std::fs::remove_file(path);
}

#[test]
#[ignore]
fn large_file_perf_baseline_open_search_scroll_and_render() {
    let path = temp_file_path("large-file-perf.log");
    let fixture = write_large_file_perf_fixture(&path, large_file_perf_target_bytes());
    eprintln!(
        "large_file_perf fixture: bytes={} lines={} error_lines={}",
        fixture.bytes, fixture.lines, fixture.error_lines
    );

    let config = config_with_editable_file_soft_limit(fixture.bytes as u64);
    let mut app = measure_large_file_perf("startup_open", || {
        AppState::from_config_path(config, Some(path.clone())).unwrap()
    });
    let buffer_id = app.focused_buffer_id().unwrap();
    let line_count = app.buffer_state(buffer_id).unwrap().buffer.line_count();
    assert_eq!(line_count, fixture.lines);

    let sparse_matches = measure_large_file_perf("find_all_sparse_match", || {
        app.buffer_state(buffer_id)
            .unwrap()
            .buffer
            .find_all("ERROR service=api")
    });
    assert_eq!(sparse_matches.len(), fixture.error_lines);

    let missing_matches = measure_large_file_perf("find_all_missing_match", || {
        app.buffer_state(buffer_id)
            .unwrap()
            .buffer
            .find_all("needle-that-does-not-exist")
    });
    assert!(missing_matches.is_empty());

    let last_line = fixture.lines.saturating_sub(1);
    app.focused_buffer_mut()
        .unwrap()
        .buffer
        .set_cursor(Position::new(last_line, 0))
        .unwrap();
    measure_large_file_perf("sync_view_to_eof", || {
        app.sync_view_for_area(Rect::new(0, 0, 120, 40));
    });
    assert!(app.buffer_state(buffer_id).unwrap().first_line > 0);

    let buffer_views = app.buffer_views();
    let ui_frame = measure_large_file_perf("ui_frame_visible_window", || {
        app.shell
            .frame_for_workspace(&app.workspace, app.workspace_area, &buffer_views)
    });
    assert!(!ui_frame.windows[0].body.is_empty());

    let backend = TestBackend::new(120, 42);
    let mut terminal = Terminal::new(backend).unwrap();
    measure_large_file_perf("ratatui_draw_visible_window", || {
        terminal
            .draw(|frame| app.shell.render(frame, &ui_frame))
            .unwrap();
    });

    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore]
fn large_file_perf_long_line_display_cap() {
    let line_bytes = large_line_perf_target_bytes();
    let long_line = "x".repeat(line_bytes);
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, &long_line);
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let workspace = Workspace::new_untitled();
    let shell = UiShell::default();

    let ui_frame = measure_large_file_perf("ui_frame_long_line_display_cap", || {
        shell.frame_for_workspace(&workspace, Rect::new(0, 0, 120, 8), &[buffer_view])
    });

    let line = &ui_frame.windows[0].body[0];
    assert!(line.truncated);
    assert_eq!(
        line.bytes_consumed,
        Limits::default().line_display_soft_limit_bytes
    );

    let backend = TestBackend::new(120, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    measure_large_file_perf("ratatui_draw_long_line_display_cap", || {
        terminal
            .draw(|frame| shell.render(frame, &ui_frame))
            .unwrap();
    });
}
