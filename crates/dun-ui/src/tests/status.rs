use dun_term::{Style, StyleAttrs, TerminalColor};

use super::support::*;
use crate::render::surface_layers::draw_status;
use crate::surface::Surface;

const INITIAL_STYLE: Style = Style::new(
    TerminalColor::Indexed(254),
    TerminalColor::Indexed(255),
    StyleAttrs::BOLD_REVERSE,
);

fn status(plugin: Option<PluginIndicator>) -> StatusBar {
    StatusBar {
        left: "left".to_string(),
        right: "right".to_string(),
        plugin,
        focused_window: WindowId(1),
    }
}

fn draw(shell: &UiShell, status: &StatusBar, width: u16) -> Surface {
    let mut surface = Surface::new(width, 1, INITIAL_STYLE);
    draw_status(&mut surface, shell, status, Rect::new(0, 0, width, 1));
    surface
}

#[test]
fn status_without_plugin_indicator_renders_the_existing_row_unchanged() {
    let shell = UiShell::default();
    let surface = draw(&shell, &status(None), 20);

    assert_eq!(surface.row_text(0), "left           right");
    assert!((0..20).all(|x| {
        surface.cell(x, 0).map(|cell| cell.style) == Some(shell.theme.palette.status_bar)
    }));
}

#[test]
fn alert_plugin_indicator_swaps_the_theme_warning_style() {
    let shell = UiShell::default();
    let surface = draw(
        &shell,
        &status(Some(PluginIndicator {
            text: "[demo idle]".to_string(),
            alert: true,
        })),
        20,
    );
    let warning = shell.theme.palette.warning;
    let expected = Style {
        fg: warning.bg,
        bg: warning.fg,
        attrs: warning.attrs,
    };

    assert!(surface.row_text(0).ends_with("[demo idle]"));
    assert!((0..9).all(|x| {
        surface.cell(x, 0).map(|cell| cell.style) == Some(shell.theme.palette.status_bar)
    }));
    assert!((9..20).all(|x| surface.cell(x, 0).map(|cell| cell.style) == Some(expected)));
}

#[test]
fn normal_plugin_indicator_uses_status_text_style() {
    let shell = UiShell::default();
    let surface = draw(
        &shell,
        &status(Some(PluginIndicator {
            text: "[demo]".to_string(),
            alert: false,
        })),
        20,
    );

    assert!(surface.row_text(0).ends_with("[demo]"));
    assert!((14..20).all(|x| {
        surface.cell(x, 0).map(|cell| cell.style) == Some(shell.theme.palette.status_text)
    }));
    assert!((0..14).all(|x| {
        surface.cell(x, 0).map(|cell| cell.style) == Some(shell.theme.palette.status_bar)
    }));
}

#[test]
fn plugin_indicator_is_dropped_when_the_status_row_is_too_narrow() {
    let shell = UiShell::default();
    let without_indicator = draw(&shell, &status(None), 4);
    let with_indicator = draw(
        &shell,
        &status(Some(PluginIndicator {
            text: "[very-long-plugin]".to_string(),
            alert: true,
        })),
        4,
    );

    assert_eq!(with_indicator, without_indicator);
}
