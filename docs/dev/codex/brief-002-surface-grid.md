# brief-002 — Surface cell grid for the ratatui-replacement line (slice 1)

## Goal

Add a self-contained `Surface` cell-grid module to `dun-ui`: a
width×height grid of styled cells that render code will later draw into,
replacing ratatui's `Buffer`. This slice is PURE ADDITION — no existing
file changes except the one `mod` line, no ratatui removal, no
integration. When you are done the module exists with the exact API below,
its unit tests pass, and nothing else changed.

## Context pointers

- Read `AGENTS.md` first.
- Key files:
  - `crates/dun-ui/src/lib.rs` — add `mod surface;` (keep it private for
    now; no `pub use`).
  - `crates/dun-term/src/theme/style.rs` and `theme/color.rs` — the style
    types the grid stores: `dun_term::Style { fg: TerminalColor, bg:
    TerminalColor, attrs: StyleAttrs }`; all `Copy`.
  - `crates/dun-ui/src/text.rs` — `display_width` helper; the crate already
    depends on `unicode-width` for char widths
    (`UnicodeWidthChar::width`).
- Acceptance is mechanical: the new unit tests plus the whole workspace
  suite decide.

## Scope

- Files you MAY modify:
  - `crates/dun-ui/src/surface.rs` (new file — the module + its
    `#[cfg(test)] mod tests` inline, matching dun-ui's file style),
  - `crates/dun-ui/src/lib.rs` (ONLY to add the single `mod surface;`
    line).
- Plus the standard MUST-NOT list from `docs/dev/codex/TEMPLATE.md`. Do
  not touch any render/frame/hit/model file.

## Deliverable

`crates/dun-ui/src/surface.rs` implementing exactly:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceCell {
    pub(crate) symbol: String,      // one grapheme; "" only for wide-glyph continuation
    pub(crate) style: dun_term::Style,
    pub(crate) wide_continuation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Surface {
    width: u16,
    height: u16,
    cells: Vec<SurfaceCell>,        // row-major, width*height
}
```

Methods on `Surface`:

- `new(width: u16, height: u16, fill_style: dun_term::Style) -> Self` —
  every cell is a single space with `fill_style`.
- `width()`, `height()` accessors.
- `cell(x: u16, y: u16) -> Option<&SurfaceCell>` — None outside bounds.
- `set_text(&mut self, x: u16, y: u16, text: &str, style: dun_term::Style)
  -> u16` — writes `text` starting at (x, y) on that single row, returns
  the display width actually consumed. Rules:
  - out-of-bounds y or x ≥ width writes nothing and returns 0;
  - clip at the right edge: a glyph that does not fully fit is not
    written (a width-2 glyph at the last column writes nothing there and
    stops);
  - a width-2 char occupies its cell plus a following continuation cell
    (`symbol: ""`, `wide_continuation: true`, same style);
  - a zero-width char (e.g. a combining mark) is appended to the symbol
    of the previously written cell of this call; if there is none, it is
    skipped;
  - writing over the continuation half of an existing wide glyph must
    also blank the wide glyph's head cell to a space (and vice versa:
    overwriting the head blanks its continuation) so no orphan halves
    remain;
  - control characters cannot occur (input is pre-sanitized upstream):
    `debug_assert!` on `ch.is_control()` and skip the char in release.
- `fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, symbol: char,
  style: dun_term::Style)` — clipped at the surface bounds; the symbol is
  a width-1 char (`debug_assert!` width == 1).
- `row_text(&self, y: u16) -> String` — the row's symbols concatenated,
  skipping continuation cells (for tests/snapshots).

Unit tests (inline `#[cfg(test)]`), at minimum: fill + accessors;
set_text plain ASCII with correct return width; right-edge clipping of a
width-2 char; wide char producing head + continuation and `row_text`
skipping the continuation; overwriting head/continuation halves leaving
no orphans (both directions); zero-width combining char appending to the
previous cell; out-of-bounds set_text and cell() returning 0/None;
fill_rect clipping.

## dun pitfalls (read twice)

`docs/dev/codex/TEMPLATE.md` §dun pitfalls items 1, 2, 5, 7. This module
is runtime code headed for the release binary: no new dependencies (use
the existing `unicode_width` crate), no generics, keep it plain and
small. `Vec` indexing may use checked helpers; no `unwrap`/`expect` in
non-test code paths.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dun-ui
cargo test --workspace --no-fail-fast
```

Paste the resulting `test result:` lines verbatim. Note: `mod surface;`
with no external users may need `#[allow(dead_code)]` on items or the
module — prefer one `#[allow(dead_code)]` on the `mod` line in `lib.rs`
IF clippy requires it, and say so in the report.

## Hard rules

All of `docs/dev/codex/TEMPLATE.md` §Hard rules apply verbatim.

## Report format (your final message)

Per `docs/dev/codex/TEMPLATE.md` §Report format.
