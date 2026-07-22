use dun_term::{BorderGlyphs, Style};

use crate::surface::Surface;

pub(crate) fn draw_border(
    surface: &mut Surface,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    glyphs: BorderGlyphs,
    style: Style,
) {
    if width == 0 || height == 0 {
        return;
    }

    let glyph_width = surface.glyph_width(glyphs.vertical);
    if glyph_width == 1 {
        let right = x.saturating_add(width.saturating_sub(1));
        let bottom = y.saturating_add(height.saturating_sub(1));

        if width == 1 || height == 1 {
            for row in y..=bottom {
                for column in x..=right {
                    set_char(surface, column, row, glyphs.horizontal, style);
                }
            }
            return;
        }

        set_char(surface, x, y, glyphs.top_left, style);
        set_char(surface, right, y, glyphs.top_right, style);
        set_char(surface, x, bottom, glyphs.bottom_left, style);
        set_char(surface, right, bottom, glyphs.bottom_right, style);

        for column in x.saturating_add(1)..right {
            set_char(surface, column, y, glyphs.horizontal, style);
            set_char(surface, column, bottom, glyphs.horizontal, style);
        }

        for row in y.saturating_add(1)..bottom {
            set_char(surface, x, row, glyphs.vertical, style);
            set_char(surface, right, row, glyphs.vertical, style);
        }
        return;
    }

    let glyph_width = u16::try_from(glyph_width).unwrap_or(1);
    let bottom = y.saturating_add(height.saturating_sub(1));
    if height == 1 || width < glyph_width.saturating_mul(2) {
        let complete_glyphs = width / glyph_width;
        let residual = width % glyph_width;
        for row in y..=bottom {
            for slot in 0..complete_glyphs {
                let column = x.saturating_add(slot.saturating_mul(glyph_width));
                set_char(surface, column, row, glyphs.horizontal, style);
            }
            let residual_start = x.saturating_add(complete_glyphs.saturating_mul(glyph_width));
            for offset in 0..residual {
                set_char(
                    surface,
                    residual_start.saturating_add(offset),
                    row,
                    ' ',
                    style,
                );
            }
        }
        return;
    }

    let right = x.saturating_add(width.saturating_sub(glyph_width));
    set_char(surface, x, y, glyphs.top_left, style);
    set_char(surface, right, y, glyphs.top_right, style);
    set_char(surface, x, bottom, glyphs.bottom_left, style);
    set_char(surface, right, bottom, glyphs.bottom_right, style);

    let interior_width = width.saturating_sub(glyph_width.saturating_mul(2));
    let complete_glyphs = interior_width / glyph_width;
    for slot in 0..complete_glyphs {
        let column = x
            .saturating_add(glyph_width)
            .saturating_add(slot.saturating_mul(glyph_width));
        set_char(surface, column, y, glyphs.horizontal, style);
        set_char(surface, column, bottom, glyphs.horizontal, style);
    }

    let residual_start = x
        .saturating_add(glyph_width)
        .saturating_add(complete_glyphs.saturating_mul(glyph_width));
    for offset in 0..interior_width % glyph_width {
        let column = residual_start.saturating_add(offset);
        set_char(surface, column, y, ' ', style);
        set_char(surface, column, bottom, ' ', style);
    }

    for row in y.saturating_add(1)..bottom {
        set_char(surface, x, row, glyphs.vertical, style);
        set_char(surface, right, row, glyphs.vertical, style);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_overflow_indicators(
    surface: &mut Surface,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    up: char,
    down: char,
    has_above: bool,
    has_below: bool,
    border: char,
    style: Style,
) {
    if width < 4 || height < 3 {
        return;
    }

    let border_width = u16::try_from(surface.glyph_width(border)).unwrap_or(1);
    let interior_width = width.saturating_sub(border_width.saturating_mul(2));
    let complete_slots = interior_width / border_width;
    if complete_slots == 0 {
        return;
    }
    let column = x.saturating_add(border_width).saturating_add(
        complete_slots
            .saturating_sub(1)
            .saturating_mul(border_width),
    );

    if has_above {
        set_char(surface, column, y, up, style);
    }
    if has_below {
        let bottom = y.saturating_add(height).saturating_sub(1);
        set_char(surface, column, bottom, down, style);
    }
}

fn set_char(surface: &mut Surface, x: u16, y: u16, glyph: char, style: Style) {
    let mut buffer = [0; 4];
    surface.set_text(x, y, glyph.encode_utf8(&mut buffer), style);
}

#[cfg(test)]
mod tests {
    use dun_core::Rect;
    use dun_term::{
        AmbiguousWidth, AnsiColor, BorderGlyphs, GlyphSet, Style, TerminalColor, char_width,
    };

    use super::{draw_border, draw_overflow_indicators};
    use crate::render::menu::{clamp_menu_rect, dropdown_rect_for_menu};
    use crate::render::surface_layers::draw_active_menu;
    use crate::surface::Surface;
    use crate::{MenuSelection, UiShell};

    const FILL_STYLE: Style = Style::plain(TerminalColor::Default, TerminalColor::Default);
    const DRAW_STYLE: Style = Style::plain(
        TerminalColor::Ansi(AnsiColor::White),
        TerminalColor::Ansi(AnsiColor::Blue),
    );
    const GLYPHS: BorderGlyphs = BorderGlyphs {
        top_left: 'A',
        top_right: 'B',
        bottom_left: 'C',
        bottom_right: 'D',
        horizontal: '-',
        vertical: '|',
        left_tee: '+',
        right_tee: '+',
        top_tee: '+',
        bottom_tee: '+',
        cross: '+',
    };

    fn filled_surface(width: u16, height: u16) -> Surface {
        let mut surface = Surface::new(width, height, FILL_STYLE);
        surface.fill_rect(0, 0, width, height, '.', FILL_STYLE);
        surface
    }

    #[test]
    fn border_box_draws_corners_and_edges() {
        let mut surface = filled_surface(6, 5);

        draw_border(&mut surface, 1, 1, 4, 3, GLYPHS, DRAW_STYLE);

        assert_eq!(surface.row_text(0), "......");
        assert_eq!(surface.row_text(1), ".A--B.");
        assert_eq!(surface.row_text(2), ".|..|.");
        assert_eq!(surface.row_text(3), ".C--D.");
        assert_eq!(surface.row_text(4), "......");
        assert_eq!(
            surface.cell(2, 2).map(|cell| cell.symbol.as_str()),
            Some(".")
        );
    }

    #[test]
    fn border_single_row_fills_horizontal() {
        let mut surface = filled_surface(6, 3);

        draw_border(&mut surface, 1, 1, 4, 1, GLYPHS, DRAW_STYLE);

        assert_eq!(surface.row_text(0), "......");
        assert_eq!(surface.row_text(1), ".----.");
        assert_eq!(surface.row_text(2), "......");
    }

    #[test]
    fn border_single_column_fills_horizontal() {
        let mut surface = filled_surface(5, 5);

        draw_border(&mut surface, 2, 1, 1, 3, GLYPHS, DRAW_STYLE);

        assert_eq!(surface.row_text(0), ".....");
        assert_eq!(surface.row_text(1), "..-..");
        assert_eq!(surface.row_text(2), "..-..");
        assert_eq!(surface.row_text(3), "..-..");
        assert_eq!(surface.row_text(4), ".....");
    }

    #[test]
    fn border_zero_size_draws_nothing() {
        let mut surface = filled_surface(4, 3);
        let original = surface.clone();

        draw_border(&mut surface, 1, 1, 0, 2, GLYPHS, DRAW_STYLE);
        draw_border(&mut surface, 1, 1, 2, 0, GLYPHS, DRAW_STYLE);

        assert_eq!(surface, original);
    }

    #[test]
    fn border_cells_carry_style() {
        let mut surface = filled_surface(4, 3);

        draw_border(&mut surface, 0, 0, 4, 3, GLYPHS, DRAW_STYLE);

        assert_eq!(surface.cell(0, 0).map(|cell| cell.style), Some(DRAW_STYLE));
        assert_eq!(surface.cell(1, 1).map(|cell| cell.style), Some(FILL_STYLE));
    }

    #[test]
    fn wide_border_tiles_eighty_columns_without_overwrite_or_overflow() {
        let glyphs = GlyphSet::unicode_single_line().border;
        let mut surface =
            Surface::new(80, 3, FILL_STYLE).with_ambiguous_width(AmbiguousWidth::Wide);
        surface.fill_rect(0, 0, 80, 3, '.', FILL_STYLE);

        draw_border(&mut surface, 0, 0, 80, 3, glyphs, DRAW_STYLE);

        assert_eq!(
            surface.cell(0, 0).unwrap().symbol,
            glyphs.top_left.to_string()
        );
        assert_eq!(
            surface.cell(78, 0).unwrap().symbol,
            glyphs.top_right.to_string()
        );
        let horizontal = glyphs.horizontal.to_string();
        assert_eq!(
            (2..78)
                .filter(|column| surface.cell(*column, 0).unwrap().symbol == horizontal)
                .count(),
            38
        );
        for column in 0..80 {
            let cell = surface.cell(column, 0).unwrap();
            assert_eq!(cell.wide_continuation, column % 2 == 1, "column {column}");
            assert_eq!(cell.style, DRAW_STYLE, "column {column}");
        }
        assert!(surface.cell(80, 0).is_none());
    }

    #[test]
    fn wide_border_places_odd_residual_before_right_corner() {
        let glyphs = GlyphSet::unicode_single_line().border;
        let mut surface =
            Surface::new(79, 3, FILL_STYLE).with_ambiguous_width(AmbiguousWidth::Wide);
        surface.fill_rect(0, 0, 79, 3, '.', FILL_STYLE);

        draw_border(&mut surface, 0, 0, 79, 3, glyphs, DRAW_STYLE);

        assert_eq!(
            surface.cell(77, 0).unwrap().symbol,
            glyphs.top_right.to_string()
        );
        assert!(surface.cell(78, 0).unwrap().wide_continuation);
        assert_eq!(
            surface
                .cell(76, 0)
                .map(|cell| { (cell.symbol.as_str(), cell.style, cell.wide_continuation,) }),
            Some((" ", DRAW_STYLE, false))
        );
        let horizontal = glyphs.horizontal.to_string();
        assert_eq!(
            (0..79)
                .filter(|column| surface.cell(*column, 0).unwrap().symbol == horizontal)
                .count(),
            37
        );
        assert!(surface.cell(79, 0).is_none());
    }

    #[test]
    fn wide_overflow_indicator_replaces_last_complete_horizontal_slot() {
        let glyphs = GlyphSet::unicode_single_line().border;
        let mut surface =
            Surface::new(80, 3, FILL_STYLE).with_ambiguous_width(AmbiguousWidth::Wide);
        let mut odd_surface =
            Surface::new(79, 3, FILL_STYLE).with_ambiguous_width(AmbiguousWidth::Wide);

        draw_border(&mut surface, 0, 0, 80, 3, glyphs, DRAW_STYLE);
        draw_overflow_indicators(
            &mut surface,
            0,
            0,
            80,
            3,
            '↑',
            '↓',
            true,
            true,
            glyphs.vertical,
            DRAW_STYLE,
        );
        draw_border(&mut odd_surface, 0, 0, 79, 3, glyphs, DRAW_STYLE);
        draw_overflow_indicators(
            &mut odd_surface,
            0,
            0,
            79,
            3,
            '↑',
            '↓',
            true,
            true,
            glyphs.vertical,
            DRAW_STYLE,
        );

        assert_eq!(surface.cell(76, 0).unwrap().symbol, "↑");
        assert!(surface.cell(77, 0).unwrap().wide_continuation);
        assert_eq!(
            surface.cell(78, 0).unwrap().symbol,
            glyphs.top_right.to_string()
        );
        assert!(surface.cell(79, 0).unwrap().wide_continuation);
        assert_eq!(surface.cell(76, 2).unwrap().symbol, "↓");
        assert!(surface.cell(77, 2).unwrap().wide_continuation);
        assert_eq!(odd_surface.cell(74, 0).unwrap().symbol, "↑");
        assert!(odd_surface.cell(75, 0).unwrap().wide_continuation);
        assert_eq!(odd_surface.cell(76, 0).unwrap().symbol, " ");
        assert_eq!(
            odd_surface.cell(77, 0).unwrap().symbol,
            glyphs.top_right.to_string()
        );
    }

    #[test]
    fn wide_panel_content_does_not_overlap_border() {
        let mut shell = UiShell::default();
        shell.profile.ambiguous_width = AmbiguousWidth::Wide;
        let mut menu = shell.menu_bar(None);
        menu.active = Some(MenuSelection::menu_only(0));
        let area = Rect::new(0, 0, 80, 14);
        let rect =
            clamp_menu_rect(dropdown_rect_for_menu(&shell, &menu, 0).unwrap(), area).unwrap();
        let mut surface = Surface::new(area.width, area.height, FILL_STYLE)
            .with_ambiguous_width(AmbiguousWidth::Wide);

        draw_active_menu(&mut surface, &shell, &menu, area);

        assert_eq!(shell.border_columns(), 2);
        assert!(
            surface
                .cell(rect.x + 1, rect.y + 1)
                .unwrap()
                .wide_continuation
        );
        assert_eq!(surface.cell(rect.x + 2, rect.y + 1).unwrap().symbol, " ");
        assert_eq!(surface.cell(rect.x + 3, rect.y + 1).unwrap().symbol, "N");
        assert!(
            shell
                .menu_entry_command_at_in_area(
                    MenuSelection::menu_only(0),
                    rect.x + 2,
                    rect.y + 1,
                    area,
                )
                .is_none()
        );
        assert!(
            shell
                .menu_entry_command_at_in_area(
                    MenuSelection::menu_only(0),
                    rect.x + 3,
                    rect.y + 1,
                    area,
                )
                .is_some()
        );
    }

    #[test]
    fn built_in_border_sets_have_uniform_width_in_both_modes() {
        for glyphs in [
            GlyphSet::unicode_single_line().border,
            GlyphSet::ascii().border,
        ] {
            let border_glyphs = [
                glyphs.top_left,
                glyphs.top_right,
                glyphs.bottom_left,
                glyphs.bottom_right,
                glyphs.horizontal,
                glyphs.vertical,
                glyphs.left_tee,
                glyphs.right_tee,
                glyphs.top_tee,
                glyphs.bottom_tee,
                glyphs.cross,
            ];
            for mode in [AmbiguousWidth::Narrow, AmbiguousWidth::Wide] {
                let expected = char_width(glyphs.vertical, mode);
                for glyph in border_glyphs {
                    assert_eq!(
                        char_width(glyph, mode),
                        expected,
                        "glyph {glyph:?} in {mode:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn overflow_indicators_place_both_glyphs() {
        let mut surface = filled_surface(10, 6);

        draw_overflow_indicators(
            &mut surface,
            2,
            1,
            6,
            4,
            '^',
            'v',
            true,
            true,
            GLYPHS.vertical,
            DRAW_STYLE,
        );

        assert_eq!(surface.row_text(1), "......^...");
        assert_eq!(surface.row_text(4), "......v...");
        assert_eq!(
            surface
                .cell(6, 1)
                .map(|cell| (cell.symbol.as_str(), cell.style)),
            Some(("^", DRAW_STYLE))
        );
        assert_eq!(
            surface
                .cell(6, 4)
                .map(|cell| (cell.symbol.as_str(), cell.style)),
            Some(("v", DRAW_STYLE))
        );
    }

    #[test]
    fn overflow_indicators_respect_flags() {
        let mut above_only = filled_surface(8, 5);
        let mut below_only = above_only.clone();

        draw_overflow_indicators(
            &mut above_only,
            1,
            1,
            5,
            3,
            '^',
            'v',
            true,
            false,
            GLYPHS.vertical,
            DRAW_STYLE,
        );
        draw_overflow_indicators(
            &mut below_only,
            1,
            1,
            5,
            3,
            '^',
            'v',
            false,
            true,
            GLYPHS.vertical,
            DRAW_STYLE,
        );

        assert_eq!(above_only.row_text(1), "....^...");
        assert_eq!(above_only.row_text(3), "........");
        assert_eq!(below_only.row_text(1), "........");
        assert_eq!(below_only.row_text(3), "....v...");
    }

    #[test]
    fn overflow_indicators_suppressed_when_too_small() {
        let original = filled_surface(6, 4);
        let mut narrow = original.clone();
        let mut short = original.clone();

        draw_overflow_indicators(
            &mut narrow,
            1,
            0,
            3,
            3,
            '^',
            'v',
            true,
            true,
            GLYPHS.vertical,
            DRAW_STYLE,
        );
        draw_overflow_indicators(
            &mut short,
            1,
            0,
            4,
            2,
            '^',
            'v',
            true,
            true,
            GLYPHS.vertical,
            DRAW_STYLE,
        );

        assert_eq!(narrow, original);
        assert_eq!(short, original);
    }

    #[test]
    fn out_of_bounds_placements_are_no_ops() {
        let mut surface = filled_surface(3, 2);
        let original = surface.clone();

        draw_border(&mut surface, u16::MAX, u16::MAX, 2, 2, GLYPHS, DRAW_STYLE);
        draw_overflow_indicators(
            &mut surface,
            u16::MAX,
            u16::MAX,
            4,
            3,
            '^',
            'v',
            true,
            true,
            GLYPHS.vertical,
            DRAW_STYLE,
        );

        assert_eq!(surface, original);
    }
}
