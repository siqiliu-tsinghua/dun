use super::support::*;
use crate::render::surface_overlay::draw_overlay;
use crate::surface::Surface;

fn dialog_overlay() -> UiOverlay {
    UiOverlay {
        title: "Open".to_string(),
        lines: vec!["Look in: /tmp".to_string()],
        input: Some(String::new()),
        cursor_column: Some(0),
        list: vec!["[DIR]  crates/".to_string(), "AGENTS.md".to_string()],
        selected_list_index: Some(0),
        list_has_more_above: false,
        list_has_more_below: false,
        buttons: vec!["[Esc] Cancel".to_string()],
        min_width: 40,
    }
}

/// The modal must blank the editor text underneath it. Restyling the region
/// without overwriting the symbols leaves the buffer showing through wherever
/// the modal's own text does not reach, which is how the file dialogs once
/// rendered a mangled mix of dialog and document.
#[test]
fn modal_body_blanks_the_editor_text_underneath() {
    let shell = UiShell::default();
    let area = Rect::new(0, 0, 60, 18);
    let mut surface = Surface::new(area.width, area.height, shell.theme.palette.editor);

    // Fill the whole area with buffer text, as the window layer would.
    for y in 0..area.height {
        surface.set_text(
            0,
            y,
            &"editor text that must not bleed through ".repeat(2)[..60],
            shell.theme.palette.editor,
        );
    }

    let overlay = dialog_overlay();
    draw_overlay(&mut surface, &shell, &overlay, area).expect("overlay fits this area");

    let rect = crate::render::overlay::overlay_layout(&shell, &overlay, area)
        .expect("overlay fits this area")
        .rect;

    // Every interior cell is either the modal's own text or a blank; none of
    // it is left over from the editor beneath.
    for y in rect.y + 1..rect.y + rect.height - 1 {
        // Border glyphs are multi-byte, so walk chars rather than bytes.
        let interior = surface
            .row_text(y)
            .chars()
            .skip(rect.x as usize + 1)
            .take(rect.width as usize - 2)
            .collect::<String>();
        assert!(
            !interior.contains("bleed"),
            "editor text bled through the modal on row {y}: {interior:?}"
        );
    }
}
