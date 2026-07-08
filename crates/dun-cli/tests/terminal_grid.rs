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
