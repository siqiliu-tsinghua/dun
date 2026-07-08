#![cfg(unix)]
#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::time::Duration;

mod support;

use support::pty::temp_path;
use support::tmux::{TmuxCapture, TmuxSession, tmux_test_guard};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
const INTERACTION_TIMEOUT: Duration = Duration::from_secs(3);

#[test]
fn tmux_grid_renders_baseline_layout_80x24() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let Some(session) =
        TmuxSession::start_dun("baseline-80x24", 80, 24, &[OsStr::new("--no-config")])?
    else {
        return Ok(());
    };

    let screen = session.capture_until_contains("Untitled", STARTUP_TIMEOUT)?;

    assert_eq!(
        screen.lines.len(),
        24,
        "unexpected screen height\n{}",
        screen.text
    );
    assert_line_contains(&screen, 0, "File");
    assert_line_contains(&screen, 0, "Edit");
    assert_line_contains(&screen, 0, "View");
    assert_line_contains(&screen, 0, "Help");
    assert_line_contains(&screen, 1, "┌─ ◆ Untitled");
    assert_line_contains(&screen, 22, "└");
    assert_line_contains(&screen, 23, "[Plain Text]");
    assert_line_contains(&screen, 23, "1:1");
    assert_line_contains(&screen, 23, "[Untitled]");

    Ok(())
}

#[test]
fn tmux_grid_respects_larger_fixed_pane_dimensions() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let Some(session) =
        TmuxSession::start_dun("baseline-100x30", 100, 30, &[OsStr::new("--no-config")])?
    else {
        return Ok(());
    };

    let screen = session.capture_until_contains("Untitled", STARTUP_TIMEOUT)?;

    assert_eq!(
        screen.lines.len(),
        30,
        "unexpected screen height\n{}",
        screen.text
    );
    assert_eq!(
        screen.line(1).chars().count(),
        100,
        "top border should span the fixed pane width\n{}",
        screen.line(1)
    );
    assert_eq!(
        screen.line(28).chars().count(),
        100,
        "bottom border should span the fixed pane width\n{}",
        screen.line(28)
    );
    assert_line_contains(&screen, 29, "[Plain Text]");
    assert_line_contains(&screen, 29, "[Untitled]");

    Ok(())
}

#[test]
fn tmux_grid_command_prompt_can_split_window() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let Some(session) =
        TmuxSession::start_dun("split-window", 80, 24, &[OsStr::new("--no-config")])?
    else {
        return Ok(());
    };
    session.capture_until_contains("Untitled", STARTUP_TIMEOUT)?;

    session.send_keys(&["C-p", "window.split_horizontal", "Enter"])?;
    let screen = session.capture_until_contains("Untitled-2", INTERACTION_TIMEOUT)?;
    let screen = session
        .capture_stable(Duration::from_millis(500))
        .unwrap_or(screen);

    assert_eq!(
        count_char(screen.line(1), '┌'),
        2,
        "split top border should have two panes\n{}",
        screen.line(1)
    );
    assert_eq!(
        count_char(screen.line(1), '┐'),
        2,
        "split top border should have two panes\n{}",
        screen.line(1)
    );
    assert_eq!(
        screen.line(2).matches("│1│").count(),
        2,
        "both split panes should render a gutter/body edge\n{}",
        screen.line(2)
    );
    assert_line_contains(&screen, 1, "Untitled-2");
    assert_line_contains(&screen, 23, "[Untitled-2]");

    Ok(())
}

#[test]
fn tmux_grid_ascii_16_fallback_uses_ascii_chrome_and_no_256_sgr() -> io::Result<()> {
    let _guard = tmux_test_guard();
    let config_path = temp_path("dun-tmux-ascii-16", "conf");
    fs::write(
        &config_path,
        "terminal.encoding = ascii\nterminal.colors = 16\n",
    )?;

    let args = [OsStr::new("--config"), config_path.as_os_str()];
    let session = TmuxSession::start_dun("ascii-16", 100, 24, &args);
    let Some(session) = session? else {
        let _ = fs::remove_file(&config_path);
        return Ok(());
    };

    let screen = session.capture_until_contains("Untitled", STARTUP_TIMEOUT)?;
    let sgr = session.capture_sgr()?;
    let _ = fs::remove_file(&config_path);

    assert_line_contains(&screen, 1, "+- * Untitled");
    assert_line_contains(&screen, 23, "ASCII/16");
    assert_no_unicode_box_drawing(&screen);
    assert!(
        !sgr.text.contains("38;5;") && !sgr.text.contains("48;5;"),
        "16-color fallback should not emit 256-color SGR\n{}",
        sgr.text
    );

    Ok(())
}

fn assert_line_contains(screen: &TmuxCapture, row: usize, needle: &str) {
    assert!(
        screen.line(row).contains(needle),
        "row {row} did not contain {needle:?}\nrow: {:?}\nfull screen:\n{}",
        screen.line(row),
        screen.text
    );
}

fn assert_no_unicode_box_drawing(screen: &TmuxCapture) {
    for ch in ['┌', '┐', '└', '┘', '─', '│', '◆'] {
        assert!(
            !screen.text.contains(ch),
            "ASCII fallback rendered Unicode box drawing {ch:?}\n{}",
            screen.text
        );
    }
}

fn count_char(input: &str, needle: char) -> usize {
    input.chars().filter(|ch| *ch == needle).count()
}
