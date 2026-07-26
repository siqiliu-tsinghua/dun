#![allow(unused_imports)]

use super::support::*;
use dun_config::TerminalOverrides;

const STANDARD_WIDTH: u16 = 80;
const STANDARD_HEIGHT: u16 = 24;
const BLEED_UNDERLAY: &str = "\
UNDERLAY row 01: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 02: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 03: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 04: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 05: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 06: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 07: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 08: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 09: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 10: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 11: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 12: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 13: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 14: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 15: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 16: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 17: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 18: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 19: modal backgrounds must erase every one of these glyphs completely.
UNDERLAY row 20: modal backgrounds must erase every one of these glyphs completely.";

/// Compare `actual` against `src/tests/snapshots/<name>.txt`.
/// With `UPDATE_SNAPSHOTS=1` set, rewrite the file instead of asserting.
pub(super) fn assert_snapshot(name: &str, actual: &str) {
    assert!(
        !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'),
        "snapshot names must contain only ASCII letters, digits, `_`, and `-`"
    );
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/snapshots");
    let path = directory.join(format!("{name}.txt"));
    let actual = single_final_newline(actual);

    if std::env::var("UPDATE_SNAPSHOTS").as_deref() == Ok("1") {
        std::fs::create_dir_all(&directory).unwrap_or_else(|error| {
            panic!(
                "could not create snapshot directory {}: {error}",
                directory.display()
            )
        });
        std::fs::write(&path, actual).unwrap_or_else(|error| {
            panic!("could not update snapshot {}: {error}", path.display())
        });
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "missing or unreadable snapshot {}: {error}\n\
             rerun with UPDATE_SNAPSHOTS=1 and review the resulting git diff",
            path.display()
        )
    });
    if expected != actual {
        panic!(
            "snapshot `{name}` changed\n{}\n\
             rerun with UPDATE_SNAPSHOTS=1 and review the resulting git diff",
            snapshot_diff(&expected, &actual)
        );
    }
}

fn single_final_newline(text: &str) -> String {
    let mut normalized = text.trim_end_matches('\n').to_string();
    normalized.push('\n');
    normalized
}

fn snapshot_diff(expected: &str, actual: &str) -> String {
    let expected_lines = expected.split_terminator('\n').collect::<Vec<_>>();
    let actual_lines = actual.split_terminator('\n').collect::<Vec<_>>();
    let line_count = expected_lines.len().max(actual_lines.len());
    let first_difference = (0..line_count)
        .find(|index| expected_lines.get(*index) != actual_lines.get(*index))
        .unwrap_or(0);
    let start = first_difference.saturating_sub(2);
    let end = first_difference.saturating_add(3).min(line_count);
    let mut diff = format!("first differing line: {}\n", first_difference + 1);
    for index in start..end {
        let marker = if index == first_difference { '>' } else { ' ' };
        diff.push_str(&format!(
            "{marker} expected {:>3}: {:?}\n",
            index + 1,
            expected_lines.get(index).copied().unwrap_or("<missing>")
        ));
        diff.push_str(&format!(
            "{marker}   actual {:>3}: {:?}\n",
            index + 1,
            actual_lines.get(index).copied().unwrap_or("<missing>")
        ));
    }
    diff.pop();
    diff
}

fn fixed_config(theme: ThemeName, encoding: EncodingProfile, colors: ColorProfile) -> Config {
    Config {
        theme,
        terminal: TerminalOverrides {
            encoding: Some(encoding),
            colors: Some(colors),
            ambiguous_width: None,
        },
        ..Config::default()
    }
}

fn fixed_app() -> AppState {
    AppState::from_config(fixed_config(
        ThemeName::Dun,
        EncodingProfile::Utf8,
        ColorProfile::Color256,
    ))
}

fn fixed_app_with_text(text: &str) -> AppState {
    let mut app = fixed_app();
    app.buffers[0].buffer = TextBuffer::from_text_with_kind(dun_core::BufferKind::Untitled, text);
    app
}

/// Assemble a frame exactly like `run_event_loop`, redact unstable text, and
/// render the glyph and style snapshot without allocating a terminal.
fn app_snapshot(
    app: &mut AppState,
    width: u16,
    height: u16,
    redactions: &[(&str, &str)],
) -> String {
    let workspace_area = Rect::new(0, 0, width, height.saturating_sub(2));
    app.sync_view_for_area(workspace_area);
    let mode = app.shell.profile.ambiguous_width;
    let buffer_views = app.buffer_views();
    let mut frame = app.shell.frame_for_workspace_with_menu_selection(
        &app.workspace,
        workspace_area,
        &buffer_views,
        app.menu_selection(),
    );
    let modal_open = app.prompt.is_some()
        || app.file_dialog.is_some()
        || app.buffer_switcher.is_some()
        || app.confirm.is_some()
        || app.replace_confirm.is_some();
    frame.status.left = match &app.status_message {
        Some(message) => message.clone(),
        None if modal_open => app.focused_buffer_status(),
        None => format!(
            "{} {}",
            app.focused_buffer_status(),
            app.focused_detail_status()
        ),
    };
    frame.status.right = app.focused_file_status();
    frame.status.plugin = app.plugin_indicator();
    frame.overlay = app.active_overlay();
    redact_frame_text(&mut frame, redactions, mode);

    dun_ui::frame_snapshot(&app.shell, &frame, width, height)
}

fn redact_frame_text(
    frame: &mut dun_ui::UiFrame,
    redactions: &[(&str, &str)],
    mode: AmbiguousWidth,
) {
    redact_text(&mut frame.status.left, redactions);
    redact_text(&mut frame.status.right, redactions);
    if let Some(plugin) = &mut frame.status.plugin {
        redact_text(&mut plugin.text, redactions);
    }

    for window in &mut frame.windows {
        redact_text(&mut window.title, redactions);
        for gutter in &mut window.gutter {
            redact_text(&mut gutter.label, redactions);
        }
        for line in &mut window.body {
            for segment in &mut line.segments {
                redact_text(&mut segment.text, redactions);
            }
        }
    }

    let Some(overlay) = &mut frame.overlay else {
        return;
    };
    redact_text(&mut overlay.title, redactions);
    for line in &mut overlay.lines {
        redact_text(line, redactions);
    }
    for entry in &mut overlay.list {
        redact_text(entry, redactions);
    }
    for button in &mut overlay.buttons {
        redact_text(button, redactions);
    }
    if let Some(input) = &mut overlay.input {
        if let Some(cursor_column) = overlay.cursor_column {
            let mut prefix = display_prefix(input, cursor_column, mode);
            redact_text(&mut prefix, redactions);
            overlay.cursor_column = Some(str_width(prefix.as_str(), mode));
        }
        redact_text(input, redactions);
    }
}

fn redact_text(text: &mut String, redactions: &[(&str, &str)]) {
    for (needle, replacement) in redactions {
        assert!(!needle.is_empty(), "a snapshot redaction needle is empty");
        if text.contains(needle) {
            *text = text.replace(needle, replacement);
        }
    }
}

fn display_prefix(text: &str, columns: usize, mode: AmbiguousWidth) -> String {
    let mut prefix = String::new();
    let mut width = 0usize;
    for ch in text.chars() {
        let width_for_char = char_width(ch, mode).unwrap_or(0);
        if width.saturating_add(width_for_char) > columns {
            break;
        }
        prefix.push(ch);
        width = width.saturating_add(width_for_char);
    }
    prefix
}

struct SnapshotDirectory {
    path: PathBuf,
}

impl SnapshotDirectory {
    fn new(name: &str) -> Self {
        let path = temp_file_path(name);
        std::fs::create_dir(&path).unwrap();
        std::fs::create_dir(path.join("docs")).unwrap();
        std::fs::write(path.join("alpha.txt"), "alpha fixture\n").unwrap();
        std::fs::write(path.join("beta.log"), "beta fixture\n").unwrap();
        Self { path }
    }

    fn path_text(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
}

impl Drop for SnapshotDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Protects the default 80x24 resting screen and its complete style map.
#[test]
fn startup_80x24_snapshot() {
    let mut app = fixed_app();

    assert_snapshot(
        "startup_80x24",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

#[test]
fn wide_80x24_snapshot() {
    let text = "◆".repeat(36);
    let mut config = fixed_config(
        ThemeName::Dun,
        EncodingProfile::Utf8,
        ColorProfile::Color256,
    );
    config.terminal.ambiguous_width = Some(AmbiguousWidth::Wide);
    let mut app = AppState::from_config(config);
    app.buffers[0].buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, &text);
    app.buffers[0]
        .buffer
        .set_cursor(Position::new(0, text.len()))
        .unwrap();

    assert_snapshot(
        "wide_80x24",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects the full per-buffer marker trail, including an empty line's EOL
/// marker and the status-detail bracket.
#[test]
fn visible_whitespace_snapshot() {
    let mut app = fixed_app_with_text("alpha beta\n\tindented\n\nend ");
    app.handle_command(&EditorCommand::Edit(EditCommand::ToggleVisibleWhitespace));
    app.status_message = None;

    assert_snapshot(
        "visible_whitespace",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects Open's modal blanking: real editor text must not bleed through.
#[test]
fn open_dialog_snapshot() {
    let directory = SnapshotDirectory::new("snapshot-open-dialog");
    let directory_text = directory.path_text();
    let mut app = fixed_app_with_text(BLEED_UNDERLAY);
    app.handle_command(&EditorCommand::File(FileCommand::Open));
    send_text(&mut app, &format!("{directory_text}/"));

    assert_snapshot(
        "open_dialog",
        &app_snapshot(
            &mut app,
            STANDARD_WIDTH,
            STANDARD_HEIGHT,
            &[(&directory_text, "<TMP>")],
        ),
    );
}

/// Protects Save As layout, full-path redaction, input styling, and cursor.
#[test]
fn save_as_dialog_snapshot() {
    let directory = SnapshotDirectory::new("snapshot-save-as-dialog");
    let directory_text = directory.path_text();
    let mut app = fixed_app_with_text("save this deterministic fixture");
    app.handle_command(&EditorCommand::File(FileCommand::SaveAs));
    send_text(&mut app, &format!("{directory_text}/draft.txt"));

    assert_snapshot(
        "save_as_dialog",
        &app_snapshot(
            &mut app,
            STANDARD_WIDTH,
            STANDARD_HEIGHT,
            &[(&directory_text, "<TMP>")],
        ),
    );
}

/// Protects the unsaved-changes modal and its three explicit choices.
#[test]
fn confirm_unsaved_snapshot() {
    let mut app = fixed_app();
    app.handle_paste("unsaved editor contents beneath the confirmation");
    app.handle_command(&EditorCommand::File(FileCommand::New));

    assert_snapshot(
        "confirm_unsaved",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects buffer-switcher entries, selection, and modal controls.
#[test]
fn buffer_switcher_snapshot() {
    let mut app = fixed_app_with_text("first buffer");
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    app.buffer_state_mut(BufferId(2)).unwrap().buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, "second buffer");
    app.handle_command(&EditorCommand::File(FileCommand::SwitchBuffer));

    assert_snapshot(
        "buffer_switcher",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects the live Find prompt, preview selection, and active-match style.
#[test]
fn find_prompt_snapshot() {
    let mut app = fixed_app_with_text("zero needle two needle\nsecond line");
    app.handle_command(&EditorCommand::Edit(EditCommand::Find));
    send_text(&mut app, "needle");

    assert_snapshot(
        "find_prompt",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects the Go To Line prompt title, input row, and cursor placement.
#[test]
fn go_to_line_prompt_snapshot() {
    let text = (1..=30)
        .map(|line| format!("fixed line {line:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = fixed_app_with_text(&text);
    app.handle_command(&EditorCommand::Edit(EditCommand::GoToLine));
    send_text(&mut app, "12");

    assert_snapshot(
        "go_to_line_prompt",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects File dropdown blanking over real text, the same class as modals.
#[test]
fn file_menu_open_snapshot() {
    let mut app = fixed_app_with_text(BLEED_UNDERLAY);
    handle_key_event(
        &mut app,
        TerminalKeyEvent::new(TerminalKeyCode::Char('f'), TerminalKeyModifiers::ALT),
    );

    assert_snapshot(
        "file_menu_open",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects Dun's warm focused pane and cool-haze unfocused pane distinction.
#[test]
fn split_two_panes_snapshot() {
    let mut app = fixed_app_with_text("unfocused pane recedes into haze\nleft detail");
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    app.buffer_state_mut(BufferId(2)).unwrap().buffer = TextBuffer::from_text_with_kind(
        BufferKind::Untitled,
        "focused pane stays warm\nright detail",
    );
    app.status_message = None;

    assert_snapshot(
        "split_two_panes",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects the tiled, read-only Help screen and the restored whitespace row.
#[test]
fn help_screen_snapshot() {
    let mut app = fixed_app();
    app.handle_command(&EditorCommand::App(AppCommand::Help));
    let help = app.focused_buffer_mut().expect("focused Help buffer");
    let line = (0..help.buffer.line_count())
        .find(|&line| {
            help.buffer
                .line(line)
                .is_some_and(|text| text.contains("edit.toggle_visible_whitespace"))
        })
        .expect("visible-whitespace Help row");
    help.buffer.set_cursor(Position::new(line, 0)).unwrap();

    assert_snapshot(
        "help_screen",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects the read-only Search Results pane and deterministic match rows.
#[test]
fn search_results_snapshot() {
    let mut app = fixed_app_with_text("alpha\nbeta alpha\ngamma\n");
    submit_command_line(&mut app, "find alpha");
    submit_command_line(&mut app, "results");

    assert_snapshot(
        "search_results",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects status-history levels, ordering, and read-only pane rendering.
#[test]
fn status_history_snapshot() {
    let mut app = fixed_app();
    app.set_status("Opened sample.txt");
    app.set_status("Save failed: disk full");
    app.handle_command(&EditorCommand::App(AppCommand::StatusHistory));

    assert_snapshot(
        "status_history",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects command feedback: "Nothing to undo" must remain visible.
#[test]
fn status_after_failed_command_snapshot() {
    let mut app = fixed_app();
    app.handle_command(&EditorCommand::Edit(EditCommand::Undo));

    assert_snapshot(
        "status_after_failed_command",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects the Dun theme's complete startup palette assignment.
#[test]
fn theme_dun_snapshot() {
    let mut app = AppState::from_config(fixed_config(
        ThemeName::Dun,
        EncodingProfile::Utf8,
        ColorProfile::Color256,
    ));

    assert_snapshot(
        "theme_dun",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects the MS-DOS Editor-inspired startup palette assignment.
#[test]
fn theme_msedit_snapshot() {
    let mut app = AppState::from_config(fixed_config(
        ThemeName::MsEdit,
        EncodingProfile::Utf8,
        ColorProfile::Color256,
    ));

    assert_snapshot(
        "theme_msedit",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects the Turbo Vision-inspired startup palette assignment.
#[test]
fn theme_turbo_snapshot() {
    let mut app = AppState::from_config(fixed_config(
        ThemeName::Turbo,
        EncodingProfile::Utf8,
        ColorProfile::Color256,
    ));

    assert_snapshot(
        "theme_turbo",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects the dark theme's startup palette assignment.
#[test]
fn theme_dark_snapshot() {
    let mut app = AppState::from_config(fixed_config(
        ThemeName::Dark,
        EncodingProfile::Utf8,
        ColorProfile::Color256,
    ));

    assert_snapshot(
        "theme_dark",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects the forced 16-color profile, ANSI color spelling, and glyphs.
#[test]
fn fallback_16_color_snapshot() {
    let mut app = AppState::from_config(fixed_config(
        ThemeName::Dun,
        EncodingProfile::Utf8,
        ColorProfile::Color16,
    ));

    assert_snapshot(
        "fallback_16_color",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects forced ASCII/mono fallback glyphs and attribute-only styling.
#[test]
fn fallback_mono_snapshot() {
    let mut app = AppState::from_config(fixed_config(
        ThemeName::Dun,
        EncodingProfile::Ascii,
        ColorProfile::Mono,
    ));

    assert_snapshot(
        "fallback_mono",
        &app_snapshot(&mut app, STANDARD_WIDTH, STANDARD_HEIGHT, &[]),
    );
}

/// Protects the 40x10 layout where tiny panes drop gutters and clip titles.
#[test]
fn narrow_40x10_snapshot() {
    let mut app = fixed_app_with_text("wide pane\nkeeps context");
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    app.handle_command(&EditorCommand::Window(WindowCommand::SplitHorizontal));
    let many_lines = (1..=10_000)
        .map(|line| format!("narrow body {line:05}"))
        .collect::<Vec<_>>()
        .join("\n");
    app.buffer_state_mut(BufferId(3)).unwrap().buffer =
        TextBuffer::from_text_with_kind(BufferKind::Untitled, &many_lines);
    app.status_message = None;

    assert_snapshot("narrow_40x10", &app_snapshot(&mut app, 40, 10, &[]));
}
