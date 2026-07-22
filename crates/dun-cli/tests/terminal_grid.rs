#![cfg(unix)]
#![forbid(unsafe_code)]

mod support;

use std::io;
use std::time::Duration;

use dun_term::AmbiguousWidth;
use support::pty::{CTRL_Q, TerminalCase, command_on_path, pty_test_guard, run_dun_in_pty};
use support::terminal_grid::{
    GridRect, TerminalColor, assert_line_contains, assert_text_at, find_border_box,
    find_border_boxes, parse_terminal_grid,
};

#[test]
fn terminal_grid_parses_sgr_snapshot_text_and_attributes() -> io::Result<()> {
    let grid = parse_terminal_grid(
        "\x1b[7;1mFile\x1b[0m\n\x1b[38;5;196mX",
        10,
        2,
        AmbiguousWidth::Narrow,
        None,
    );

    assert_text_at(&grid, 0, 0, "File");
    let hotkey = grid.cell(0, 0).expect("hotkey cell");
    assert!(hotkey.style.reverse);
    assert!(hotkey.style.bold);
    assert_eq!(
        grid.cell(1, 0).expect("indexed color cell").style.fg,
        TerminalColor::Indexed(196)
    );

    Ok(())
}

#[test]
fn terminal_grid_applies_raw_csi_cursor_moves_and_erases() -> io::Result<()> {
    let grid = parse_terminal_grid(
        "\x1b[2J\x1b[3;4Halpha\x1b[1;1HTop\x1b[4;1H\x1b[31mbeta\x1b[0m\x1b[4;6H!",
        12,
        5,
        AmbiguousWidth::Narrow,
        None,
    );

    assert_text_at(&grid, 0, 0, "Top");
    assert_text_at(&grid, 2, 3, "alpha");
    assert_text_at(&grid, 3, 0, "beta !");
    assert_line_contains(&grid, 2, "alpha");
    assert_eq!(
        grid.cell(3, 0).expect("ansi color cell").style.fg,
        TerminalColor::Ansi(1)
    );
    assert_eq!(
        grid.cell(3, 5).expect("reset color cell").style.fg,
        TerminalColor::Default
    );
    assert_eq!(grid.cursor.map(|cursor| (cursor.x, cursor.y)), Some((6, 3)));

    Ok(())
}

#[test]
fn terminal_grid_handles_wide_chars_tabs_and_crlf() -> io::Result<()> {
    let grid = parse_terminal_grid("中\tX\r\nY", 12, 2, AmbiguousWidth::Narrow, None);

    assert_text_at(&grid, 0, 0, "中");
    assert_eq!(
        grid.cell(0, 1).expect("wide char continuation").ch,
        ' ',
        "wide character continuation cell should remain blank"
    );
    assert_text_at(&grid, 0, 8, "X");
    assert_text_at(&grid, 1, 0, "Y");
    assert_eq!(grid.cursor.map(|cursor| (cursor.x, cursor.y)), Some((1, 1)));

    Ok(())
}

#[test]
fn terminal_grid_resets_sgr_attributes_selectively() -> io::Result<()> {
    let grid = parse_terminal_grid(
        "\x1b[1;4;7;31;48;5;18mA\x1b[22;24;27;39;49mB",
        4,
        1,
        AmbiguousWidth::Narrow,
        None,
    );

    let styled = grid.cell(0, 0).expect("styled cell").style;
    assert!(styled.bold);
    assert!(styled.underline);
    assert!(styled.reverse);
    assert_eq!(styled.fg, TerminalColor::Ansi(1));
    assert_eq!(styled.bg, TerminalColor::Indexed(18));

    let reset = grid.cell(0, 1).expect("reset cell").style;
    assert!(!reset.bold);
    assert!(!reset.underline);
    assert!(!reset.reverse);
    assert_eq!(reset.fg, TerminalColor::Default);
    assert_eq!(reset.bg, TerminalColor::Default);

    Ok(())
}

#[test]
fn terminal_grid_restores_saved_cursor_positions() -> io::Result<()> {
    let grid = parse_terminal_grid("A\x1b7\x1b[3;5HB\x1b8C", 8, 4, AmbiguousWidth::Narrow, None);

    assert_text_at(&grid, 0, 0, "AC");
    assert_text_at(&grid, 2, 4, "B");
    assert_eq!(grid.cursor.map(|cursor| (cursor.x, cursor.y)), Some((2, 0)));

    Ok(())
}

#[test]
fn terminal_grid_finds_narrow_unicode_and_ascii_border_boxes() -> io::Result<()> {
    let unicode = parse_terminal_grid(
        "menu\n┌─ Title ┐\n│ body   │\n│        │\n└────────┘\n",
        10,
        5,
        AmbiguousWidth::Narrow,
        None,
    );
    assert_eq!(unicode.ambiguous_width, AmbiguousWidth::Narrow);
    assert_eq!(
        find_border_box(&unicode),
        Some(GridRect {
            row: 1,
            col: 0,
            width: 10,
            height: 4,
        })
    );
    assert_eq!(unicode.cell(1, 0).expect("top-left glyph head").ch, '┌');
    assert_eq!(unicode.cell(1, 9).expect("top-right glyph head").ch, '┐');
    assert_eq!(unicode.cell(4, 0).expect("bottom-left glyph head").ch, '└');
    assert_eq!(unicode.cell(4, 9).expect("bottom-right glyph head").ch, '┘');
    for col in 1..9 {
        assert_eq!(
            unicode.cell(4, col).expect("bottom-border glyph head").ch,
            '─'
        );
    }

    let ascii = parse_terminal_grid(
        "left right\n+- A ++- B +\n|    ||    |\n+----++----+\n",
        12,
        4,
        AmbiguousWidth::Narrow,
        None,
    );
    assert_eq!(
        find_border_boxes(&ascii),
        vec![
            GridRect {
                row: 1,
                col: 0,
                width: 6,
                height: 3,
            },
            GridRect {
                row: 1,
                col: 6,
                width: 6,
                height: 3,
            },
        ]
    );

    Ok(())
}

#[test]
fn terminal_grid_finds_wide_unicode_border_box_by_physical_cells() -> io::Result<()> {
    let grid = parse_terminal_grid(
        "┌Title   ┐\n│ body   │\n└────┘\n",
        12,
        3,
        AmbiguousWidth::Wide,
        None,
    );

    assert_eq!(grid.ambiguous_width, AmbiguousWidth::Wide);
    assert_eq!(
        find_border_box(&grid),
        Some(GridRect {
            row: 0,
            col: 0,
            width: 12,
            height: 3,
        })
    );
    assert_eq!(grid.cell(0, 0).expect("top-left glyph head").ch, '┌');
    assert_eq!(grid.cell(0, 10).expect("top-right glyph head").ch, '┐');
    assert_eq!(grid.cell(1, 0).expect("left-side glyph head").ch, '│');
    assert_eq!(grid.cell(1, 10).expect("right-side glyph head").ch, '│');
    assert_eq!(grid.cell(2, 0).expect("bottom-left glyph head").ch, '└');
    for col in [2, 4, 6, 8] {
        assert_eq!(grid.cell(2, col).expect("bottom-border glyph head").ch, '─');
    }
    assert_eq!(grid.cell(2, 10).expect("bottom-right glyph head").ch, '┘');
    assert_eq!(grid.cell(2, 11).expect("corner continuation cell").ch, ' ');

    Ok(())
}

#[test]
fn pty_harness_answers_the_probe_without_the_narrow_fallback() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY probe test: expect(1) is not on PATH");
        return Ok(());
    };
    let case = utf8_probe_case("narrow probe response");
    let run = run_dun_in_pty(&expect, case, &[], "Untitled", CTRL_Q)?;
    let probe_elapsed = run
        .probe_elapsed
        .expect("expect observed the startup probe");
    eprintln!("default Narrow PTY responder elapsed: {probe_elapsed:?}");

    assert!(run.status.success(), "PTY run failed\n{}", run.output);
    assert!(
        probe_elapsed < Duration::from_millis(500),
        "default Narrow responder paid the 500 ms fallback: {:?}",
        probe_elapsed
    );
    assert_eq!(
        find_border_box(&run.terminal_grid_for_case(case)),
        Some(GridRect {
            row: 1,
            col: 0,
            width: 80,
            height: 22,
        })
    );

    Ok(())
}

#[test]
fn pty_harness_can_answer_the_probe_as_wide() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY probe test: expect(1) is not on PATH");
        return Ok(());
    };
    let case = utf8_probe_case("wide probe response").with_wide_ambiguous_width();
    let run = run_dun_in_pty(&expect, case, &[], "Untitled", CTRL_Q)?;
    let probe_elapsed = run
        .probe_elapsed
        .expect("expect observed the startup probe");
    eprintln!("dedicated Wide PTY responder elapsed: {probe_elapsed:?}");

    assert!(run.status.success(), "PTY run failed\n{}", run.output);
    assert!(
        probe_elapsed < Duration::from_millis(500),
        "Wide responder paid the 500 ms fallback: {:?}",
        probe_elapsed
    );
    let grid = run.terminal_grid_for_case(case);
    assert_eq!(grid.ambiguous_width, AmbiguousWidth::Wide);
    assert_eq!(
        find_border_box(&grid),
        Some(GridRect {
            row: 1,
            col: 0,
            width: 80,
            height: 22,
        })
    );

    Ok(())
}

#[test]
fn pty_harness_retains_the_no_response_narrow_fallback() -> io::Result<()> {
    let _guard = pty_test_guard();
    let Some(expect) = command_on_path("expect") else {
        eprintln!("skipping PTY probe test: expect(1) is not on PATH");
        return Ok(());
    };
    let case = utf8_probe_case("no probe response").without_ambiguous_width_probe_response();
    let run = run_dun_in_pty(&expect, case, &[], "Untitled", CTRL_Q)?;
    let probe_elapsed = run
        .probe_elapsed
        .expect("expect observed the startup probe");
    eprintln!("no-response Narrow fallback elapsed: {probe_elapsed:?}");

    assert!(run.status.success(), "PTY run failed\n{}", run.output);
    assert!(
        probe_elapsed >= Duration::from_millis(500),
        "no-response case did not reach the 500 ms fallback: {:?}",
        probe_elapsed
    );
    assert_eq!(
        find_border_box(&run.terminal_grid_for_case(case)),
        Some(GridRect {
            row: 1,
            col: 0,
            width: 80,
            height: 22,
        })
    );

    Ok(())
}

fn utf8_probe_case(name: &'static str) -> TerminalCase {
    TerminalCase::new(
        name,
        "xterm-256color",
        "en_US.UTF-8",
        "en_US.UTF-8",
        false,
        "UTF-8/256",
    )
}
