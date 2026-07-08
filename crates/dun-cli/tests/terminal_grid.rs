#![cfg(unix)]
#![forbid(unsafe_code)]

mod support;

use std::io;

use support::terminal_grid::{
    GridRect, TerminalColor, assert_line_contains, assert_text_at, find_border_box,
    find_border_boxes, parse_terminal_grid,
};

#[test]
fn terminal_grid_parses_sgr_snapshot_text_and_attributes() -> io::Result<()> {
    let grid = parse_terminal_grid("\x1b[7;1mFile\x1b[0m\n\x1b[38;5;196mX", 10, 2, None);

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
    let grid = parse_terminal_grid("中\tX\r\nY", 12, 2, None);

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
    let grid = parse_terminal_grid("\x1b[1;4;7;31;48;5;18mA\x1b[22;24;27;39;49mB", 4, 1, None);

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
    let grid = parse_terminal_grid("A\x1b7\x1b[3;5HB\x1b8C", 8, 4, None);

    assert_text_at(&grid, 0, 0, "AC");
    assert_text_at(&grid, 2, 4, "B");
    assert_eq!(grid.cursor.map(|cursor| (cursor.x, cursor.y)), Some((2, 0)));

    Ok(())
}

#[test]
fn terminal_grid_finds_unicode_and_ascii_border_boxes() -> io::Result<()> {
    let unicode = parse_terminal_grid(
        "menu\n┌─ Title ┐\n│ body   │\n│        │\n└────────┘\n",
        10,
        5,
        None,
    );
    assert_eq!(
        find_border_box(&unicode),
        Some(GridRect {
            row: 1,
            col: 0,
            width: 10,
            height: 4,
        })
    );

    let ascii = parse_terminal_grid(
        "left right\n+- A ++- B +\n|    ||    |\n+----++----+\n",
        12,
        4,
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
