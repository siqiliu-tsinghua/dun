#![cfg(unix)]
#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::time::Duration;

mod support;

use support::pty::{microsoft_edit_on_path, temp_path};
use support::terminal_grid::TerminalGrid;
use support::tmux::{TmuxSession, tmux_test_guard};

const DIFF_COLS: u16 = 80;
const DIFF_ROWS: u16 = 24;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
const STABLE_TIMEOUT: Duration = Duration::from_millis(600);

#[derive(Clone, Copy, Debug)]
struct DiffCase<'a> {
    name: &'a str,
    keys: &'a [&'a str],
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EditorProjection {
    body: Vec<String>,
    cursor: Option<(u16, u16)>,
}

#[test]
fn microsoft_edit_diff_open_file_and_basic_cursor_motion_when_available() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let Some(edit) = microsoft_edit_on_path() else {
        eprintln!(
            "skipping Microsoft Edit differential test: no Microsoft Edit on PATH \
             (a non-Edit `edit`, e.g. FreeBSD's `ee`, does not count)"
        );
        return Ok(());
    };

    for case in [
        DiffCase {
            name: "Right Right",
            keys: &["Right", "Right"],
        },
        DiffCase {
            name: "End",
            keys: &["End"],
        },
        DiffCase {
            name: "Down Up",
            keys: &["Down", "Up"],
        },
        DiffCase {
            name: "Down Right",
            keys: &["Down", "Right"],
        },
    ] {
        run_diff_case(edit.as_os_str(), case)?;
    }

    Ok(())
}

fn run_diff_case(edit: &OsStr, case: DiffCase<'_>) -> io::Result<()> {
    let file_path = temp_path("dun-msedit-diff", "txt");
    fs::write(&file_path, "alpha\nbeta\ngamma\n")?;
    let expected = ["alpha", "beta", "gamma"];

    let dun = TmuxSession::start_dun(
        &format!("msedit-diff-dun-{}", sanitize_label(case.name)),
        DIFF_COLS,
        DIFF_ROWS,
        &[OsStr::new("--no-config"), file_path.as_os_str()],
    );
    let edit = TmuxSession::start_executable(
        &format!("msedit-diff-edit-{}", sanitize_label(case.name)),
        DIFF_COLS,
        DIFF_ROWS,
        edit,
        &[file_path.as_os_str()],
    );

    let Some(dun) = dun? else {
        let _ = fs::remove_file(&file_path);
        return Ok(());
    };
    let Some(edit) = edit? else {
        let _ = fs::remove_file(&file_path);
        return Ok(());
    };

    wait_for_editor_text(&dun)?;
    wait_for_editor_text(&edit)?;
    assert_projected_editors_match(
        &format!("{} initial open", case.name),
        &dun,
        &edit,
        &expected,
    )?;

    dun.send_keys(case.keys)?;
    edit.send_keys(case.keys)?;
    assert_projected_editors_match(&format!("after {}", case.name), &dun, &edit, &expected)?;

    let _ = fs::remove_file(&file_path);

    Ok(())
}

fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn wait_for_editor_text(session: &TmuxSession) -> io::Result<()> {
    session.capture_until_contains("alpha", STARTUP_TIMEOUT)?;
    let _ = session.capture_stable(STABLE_TIMEOUT)?;
    Ok(())
}

fn assert_projected_editors_match(
    case: &str,
    dun: &TmuxSession,
    edit: &TmuxSession,
    expected_lines: &[&str],
) -> io::Result<()> {
    let _ = dun.capture_stable(STABLE_TIMEOUT)?;
    let _ = edit.capture_stable(STABLE_TIMEOUT)?;

    let dun_grid = dun.capture_grid()?;
    let edit_grid = edit.capture_grid()?;
    let dun_projection = project_editor_body("dun", &dun_grid, expected_lines)?;
    let edit_projection = project_editor_body("edit", &edit_grid, expected_lines)?;

    assert_eq!(
        dun_projection,
        edit_projection,
        "{case} projection mismatch\n{}",
        projection_diff_dump(&dun_projection, &edit_projection)
    );

    Ok(())
}

fn project_editor_body(
    label: &str,
    grid: &TerminalGrid,
    expected_lines: &[&str],
) -> io::Result<EditorProjection> {
    let mut rows = Vec::new();
    let mut body = Vec::new();
    let mut search_start = 0u16;

    for (index, expected) in expected_lines.iter().enumerate() {
        let line_no = index + 1;
        let Some((row, body_col, text)) =
            find_numbered_body_line(grid, search_start, line_no, expected)
        else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "{label} projection could not find body line {line_no} matching {expected:?}\n{}",
                    grid_dump(grid)
                ),
            ));
        };
        search_start = row.saturating_add(1);
        rows.push((row, body_col));
        body.push(text);
    }

    let cursor = grid.cursor.and_then(|cursor| {
        rows.iter()
            .enumerate()
            .find(|(_, (row, body_col))| cursor.y == *row && cursor.x >= *body_col)
            .map(|(index, (_, body_col))| (cursor.x - *body_col, index as u16))
    });

    Ok(EditorProjection { body, cursor })
}

fn find_numbered_body_line(
    grid: &TerminalGrid,
    search_start: u16,
    line_no: usize,
    expected: &str,
) -> Option<(u16, u16, String)> {
    let line_no = line_no.to_string();
    for row in search_start..grid.height {
        let line = grid.line_text(row);
        let Some(separator_col) = numbered_line_separator_col(&line, &line_no) else {
            continue;
        };
        for body_col in body_col_candidates(&line, separator_col) {
            let text = normalized_body_text(grid, row, body_col);
            if text == expected {
                return Some((row, body_col, text));
            }
        }
    }

    None
}

fn numbered_line_separator_col(line: &str, line_no: &str) -> Option<u16> {
    let chars = line.chars().collect::<Vec<_>>();
    let max_scan = chars.len().min(12);
    for separator in 0..max_scan {
        if !is_vertical_separator(chars[separator]) {
            continue;
        }
        let left = (0..separator)
            .rfind(|index| is_vertical_separator(chars[*index]))
            .map_or(0, |index| index + 1);
        let segment = chars[left..separator].iter().collect::<String>();
        if segment.trim() == line_no {
            return Some(separator as u16);
        }
    }

    None
}

fn body_col_candidates(line: &str, separator_col: u16) -> Vec<u16> {
    let chars = line.chars().collect::<Vec<_>>();
    let first = separator_col.saturating_add(1);
    let mut candidates = vec![first];
    if chars.get(first as usize) == Some(&' ') {
        candidates.push(first.saturating_add(1));
    }
    candidates
}

fn normalized_body_text(grid: &TerminalGrid, row: u16, body_col: u16) -> String {
    let mut text = grid.text_at(row, body_col, grid.width.saturating_sub(body_col));
    loop {
        text = text.trim_end().to_string();
        if text.chars().last().is_some_and(is_trailing_editor_chrome) {
            let _ = text.pop();
            continue;
        }
        return text.trim_end().to_string();
    }
}

fn is_vertical_separator(ch: char) -> bool {
    matches!(ch, '│' | '|')
}

fn is_trailing_editor_chrome(ch: char) -> bool {
    matches!(ch, '│' | '|' | '█' | '▅' | '░' | '▒' | '▓')
}

fn projection_diff_dump(dun: &EditorProjection, edit: &EditorProjection) -> String {
    let mut dump = String::from("body projection:\n");
    let lines = dun.body.len().max(edit.body.len());
    for index in 0..lines {
        let dun_line = dun.body.get(index).map(String::as_str).unwrap_or("");
        let edit_line = edit.body.get(index).map(String::as_str).unwrap_or("");
        dump.push_str(&format!(
            "{:>3}: dun  {:<40} | edit {}\n",
            index + 1,
            quoted(dun_line),
            quoted(edit_line)
        ));
    }
    dump.push_str(&format!(
        "cursor: dun {:?} | edit {:?}",
        dun.cursor, edit.cursor
    ));
    dump
}

fn grid_dump(grid: &TerminalGrid) -> String {
    (0..grid.height)
        .map(|row| format!("{row:>3}: {}", grid.line_text(row)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn quoted(value: &str) -> String {
    format!("{value:?}")
}
