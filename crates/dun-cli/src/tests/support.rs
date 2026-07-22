#![allow(unused_imports)]

pub(super) use crate::*;
pub(super) use dun_core::TextRange;
pub(super) use std::ffi::OsString;
pub(super) use std::io::Write;
pub(super) use std::path::{Path, PathBuf};
pub(super) use std::str::FromStr;
pub(super) use std::time::{Duration, Instant};

pub(super) fn left_click(column: u16, row: u16) -> TerminalMouseEvent {
    TerminalMouseEvent {
        kind: TerminalMouseEventKind::Down(TerminalMouseButton::Left),
        column,
        row,
        modifiers: TerminalKeyModifiers::NONE,
    }
}

pub(super) fn right_click(column: u16, row: u16) -> TerminalMouseEvent {
    TerminalMouseEvent {
        kind: TerminalMouseEventKind::Down(TerminalMouseButton::Right),
        column,
        row,
        modifiers: TerminalKeyModifiers::NONE,
    }
}

pub(super) fn left_drag(column: u16, row: u16) -> TerminalMouseEvent {
    TerminalMouseEvent {
        kind: TerminalMouseEventKind::Drag(TerminalMouseButton::Left),
        column,
        row,
        modifiers: TerminalKeyModifiers::NONE,
    }
}

pub(super) fn left_up(column: u16, row: u16) -> TerminalMouseEvent {
    TerminalMouseEvent {
        kind: TerminalMouseEventKind::Up(TerminalMouseButton::Left),
        column,
        row,
        modifiers: TerminalKeyModifiers::NONE,
    }
}

pub(super) fn scroll_down(column: u16, row: u16) -> TerminalMouseEvent {
    TerminalMouseEvent {
        kind: TerminalMouseEventKind::ScrollDown,
        column,
        row,
        modifiers: TerminalKeyModifiers::NONE,
    }
}

pub(super) fn scroll_up(column: u16, row: u16) -> TerminalMouseEvent {
    TerminalMouseEvent {
        kind: TerminalMouseEventKind::ScrollUp,
        column,
        row,
        modifiers: TerminalKeyModifiers::NONE,
    }
}

pub(super) fn config_with_editable_file_soft_limit(limit: u64) -> Config {
    Config {
        limits: Limits {
            editable_file_soft_limit_bytes: limit,
            ..Limits::default()
        },
        ..Config::default()
    }
}

pub(super) fn app_from_config_path(path: PathBuf) -> AppState {
    let request = ConfigLoadRequest::explicit(path);
    let loaded_config = load_config(&request).unwrap();
    AppState::from_loaded_config(request, loaded_config)
}

pub(super) fn help_command_line(text: &str) -> &str {
    text.lines()
        .find(|line| line.contains("Help [app.help]"))
        .expect("help command line should be present")
}

pub(super) fn keymap_command_line<'a>(text: &'a str, command: &str) -> &'a str {
    text.lines()
        .find(|line| line.contains(command))
        .expect("keymap command line should be present")
}

pub(super) fn temp_file_path(name: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "dun-cli-test-{}-{unique}-{name}",
        std::process::id()
    ))
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LargeFilePerfFixture {
    pub(super) bytes: usize,
    pub(super) lines: usize,
    pub(super) error_lines: usize,
}

pub(super) fn large_file_perf_target_bytes() -> usize {
    perf_env_usize("DUN_PERF_LARGE_FILE_BYTES").unwrap_or(8 * 1024 * 1024)
}

pub(super) fn large_line_perf_target_bytes() -> usize {
    perf_env_usize("DUN_PERF_LONG_LINE_BYTES").unwrap_or(512 * 1024)
}

pub(super) fn perf_env_usize(name: &str) -> Option<usize> {
    let value = std::env::var(name).ok()?.parse().ok()?;
    (value > 0).then_some(value)
}

pub(super) fn write_large_file_perf_fixture(
    path: &Path,
    target_bytes: usize,
) -> LargeFilePerfFixture {
    let mut file = std::fs::File::create(path).unwrap();
    let mut bytes = 0;
    let mut lines = 0;
    let mut error_lines = 0;

    while bytes < target_bytes {
        if lines > 0 {
            file.write_all(b"\n").unwrap();
            bytes += 1;
        }

        let line = if lines % 257 == 0 {
            error_lines += 1;
            format!(
                "ERROR service=api shard={:04} request_id={:08x} message=slow backend response",
                lines % 4096,
                lines
            )
        } else {
            format!(
                "INFO service=api shard={:04} request_id={:08x} message=heartbeat ok",
                lines % 4096,
                lines
            )
        };
        file.write_all(line.as_bytes()).unwrap();
        bytes += line.len();
        lines += 1;
    }

    LargeFilePerfFixture {
        bytes,
        lines,
        error_lines,
    }
}

pub(super) fn measure_large_file_perf<T>(label: &str, action: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let output = action();
    let elapsed = started.elapsed();
    eprintln!("large_file_perf {label}: {} ms", elapsed.as_millis());
    output
}

pub(super) fn atomic_temp_files_for(path: &Path) -> Vec<PathBuf> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().unwrap_or_default();
    let mut prefix = OsString::from(".");
    prefix.push(file_name);
    prefix.push(".dun-save-");
    let prefix = prefix.to_string_lossy();

    std::fs::read_dir(directory)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(&*prefix))
        .map(|entry| entry.path())
        .collect()
}

pub(super) fn write_atomic_temp_file_for(path: &Path, attempt: u32, contents: &str) -> PathBuf {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().unwrap_or_default();
    let temp_path = atomic_temp_path(directory, file_name, attempt);
    std::fs::write(&temp_path, contents).unwrap();
    temp_path
}

pub(super) fn write_newer_atomic_temp_file_for(path: &Path, contents: &str) -> PathBuf {
    for attempt in 0..100 {
        std::thread::sleep(Duration::from_millis(10));
        let temp_path = write_atomic_temp_file_for(path, attempt, contents);
        if file_modified(&temp_path) > file_modified(path) {
            return temp_path;
        }
        let _ = std::fs::remove_file(&temp_path);
    }

    panic!("could not create atomic temp file newer than destination");
}

pub(super) fn make_destination_at_least_as_new_as(path: &Path, other: &Path, contents: &str) {
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(10));
        std::fs::write(path, contents).unwrap();
        if file_modified(path) >= file_modified(other) {
            return;
        }
    }

    panic!("could not make destination at least as new as comparison file");
}

pub(super) fn file_modified(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}

pub(super) fn set_path_readonly(path: &Path, readonly: bool) {
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_readonly(readonly);
    std::fs::set_permissions(path, permissions).unwrap();
}

pub(super) fn send_text(app: &mut AppState, text: &str) {
    for ch in text.chars() {
        handle_key_event(
            app,
            TerminalKeyEvent::new(TerminalKeyCode::Char(ch), TerminalKeyModifiers::NONE),
        );
    }
}

pub(super) fn submit_command_line(app: &mut AppState, text: &str) {
    app.handle_command(&EditorCommand::App(AppCommand::CommandLine));
    send_text(app, text);
    handle_key_event(
        app,
        TerminalKeyEvent::new(TerminalKeyCode::Enter, TerminalKeyModifiers::NONE),
    );
}

pub(super) fn file_dialog_list_point(app: &AppState, visible_index: usize) -> (u16, u16) {
    let overlay = app.active_overlay().expect("file dialog overlay");
    let area = app.overlay_area();
    for y in 0..area.height {
        if app.shell.hit_test_overlay_list(&overlay, area, 20, y) == Some(visible_index) {
            return (20, y);
        }
    }

    panic!("visible file dialog row {visible_index} was not hittable");
}

pub(super) fn app_with_text(text: &str) -> AppState {
    let mut app = AppState::new();
    app.buffers[0].buffer = TextBuffer::from_text_with_kind(dun_core::BufferKind::Untitled, text);
    app
}
