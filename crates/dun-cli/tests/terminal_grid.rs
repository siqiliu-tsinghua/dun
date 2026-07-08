#![cfg(unix)]
#![forbid(unsafe_code)]

mod support;

use std::io;

use support::terminal_grid::{TerminalColor, parse_terminal_grid};

#[test]
fn terminal_grid_parses_sgr_snapshot_text_and_attributes() -> io::Result<()> {
    let grid = parse_terminal_grid("\x1b[7;1mFile\x1b[0m\n\x1b[38;5;196mX", 10, 2, None);

    assert_eq!(grid.text_at(0, 0, 4), "File");
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

    assert_eq!(grid.text_at(0, 0, 3), "Top");
    assert_eq!(grid.text_at(2, 3, 5), "alpha");
    assert_eq!(grid.text_at(3, 0, 6), "beta !");
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
