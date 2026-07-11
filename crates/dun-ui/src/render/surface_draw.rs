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
    style: Style,
) {
    if width < 4 || height < 3 {
        return;
    }

    let column = x.saturating_add(width).saturating_sub(2);
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
    use dun_term::{AnsiColor, BorderGlyphs, Style, TerminalColor};

    use super::{draw_border, draw_overflow_indicators};
    use crate::surface::Surface;

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
    fn overflow_indicators_place_both_glyphs() {
        let mut surface = filled_surface(10, 6);

        draw_overflow_indicators(&mut surface, 2, 1, 6, 4, '^', 'v', true, true, DRAW_STYLE);

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

        draw_overflow_indicators(&mut narrow, 1, 0, 3, 3, '^', 'v', true, true, DRAW_STYLE);
        draw_overflow_indicators(&mut short, 1, 0, 4, 2, '^', 'v', true, true, DRAW_STYLE);

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
            DRAW_STYLE,
        );

        assert_eq!(surface, original);
    }
}
