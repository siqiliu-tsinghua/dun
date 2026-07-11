# Brief 006 — Surface chrome drawing primitives

Implementation brief. Renderer-replacement slice 3a: port the geometric
chrome-drawing primitives (window border, vertical overflow indicators) from
the ratatui `Buffer` to the in-house `Surface`. Pure Surface functions with
pure Surface tests; no ratatui interaction, no entry-point change, no caller
change. The higher render layers (status/menu/window) and the entry-point
wiring are later slices.

## Goal

`crates/dun-ui` gains a private module `render/surface_draw` with two
`pub(crate)` functions that draw onto a `Surface` using its existing
`set_text`/`cell`/`width`/`height` API:

- `draw_border(surface, x, y, width, height, glyphs, style)` — draws a
  single-cell border box, mirroring the current `render::chrome::render_border`
  logic (corner glyphs, horizontal/vertical edges, and the degenerate
  1-row / 1-column cases).
- `draw_overflow_indicators(surface, x, y, width, height, up, down, has_above, has_below, style)`
  — mirrors `render::chrome::render_vertical_overflow_indicators`: places the
  `up` glyph two columns in from the right edge on the top row when
  `has_above`, and the `down` glyph at the same column on the bottom row when
  `has_below`; draws nothing if `width < 4 || height < 3`.

The module is `#[allow(dead_code)]` (nothing calls it yet, exactly like the
sibling `surface` and `surface_emit` modules), covered by unit tests, and the
`dun-ui` suite is green.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-ui/src/surface.rs` — the `Surface` grid. Draw glyphs with
  `set_text(x, y, &s, style)` (it returns the display width advanced and
  clips at the right edge); read back with `cell(x, y)` and `row_text(y)`.
  Coordinates are absolute surface coordinates.
- `crates/dun-ui/src/render/chrome.rs` — the ratatui originals being
  mirrored: `render_border` (lines ~60-112) and
  `render_vertical_overflow_indicators` (lines ~14-38). Reproduce their glyph
  placement exactly; the only change is the drawing target (`Surface` instead
  of `Buffer`) and the style type (`dun_term::Style` instead of a converted
  ratatui `Style` — no conversion needed).
- `crates/dun-term` — `BorderGlyphs` (fields `top_left`, `top_right`,
  `bottom_left`, `bottom_right`, `horizontal`, `vertical`), `Style`.
- `crates/dun-ui/src/render/mod.rs` — module declaration list for `render/*`.

## Specification

Signatures (put them in `render/surface_draw.rs`):

```rust
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
);

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
);
```

- `draw_border`: if `width == 0 || height == 0`, draw nothing. If
  `width == 1 || height == 1`, fill every cell of the rect with the
  `horizontal` glyph (the degenerate case in the original). Otherwise place
  `top_left`/`top_right`/`bottom_left`/`bottom_right` at the four corners,
  `horizontal` along the top and bottom edges between the corners, and
  `vertical` down the left and right edges between the corners. Each glyph is
  written with `set_text` and the given `style`. Use saturating arithmetic
  where the original does; never index a cell outside the rect. The right
  edge is `x + width - 1`, the bottom edge is `y + height - 1`.
- `draw_overflow_indicators`: return immediately if `width < 4 || height < 3`.
  The indicator column is `x + width - 2` (saturating). Write `up` at
  `(col, y)` when `has_above`, and `down` at `(col, y + height - 1)`
  (saturating) when `has_below`, both with `style`. Nothing else is touched.

Both functions must be no-ops for any placement that would fall outside the
surface (rely on `set_text`'s own bounds clipping; do not panic).

## Scope

- Files you MAY modify:
  - `crates/dun-ui/src/render/surface_draw.rs` (new; implementation +
    `#[cfg(test)] mod tests`);
  - `crates/dun-ui/src/render/mod.rs` (add
    `pub(crate) mod surface_draw;` with an `#[allow(dead_code)]` attribute,
    matching how `lib.rs` gates the `surface`/`surface_emit` modules).
- Files/areas you MUST NOT touch (defaults for every brief):
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`,
    `docs/**`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock` (no new dependencies);
  - `crates/dun-ui/src/surface.rs`, `render/chrome.rs`, and every other
    existing render module — this brief ADDS a module, it does not change the
    ratatui path;
  - `vm-test/**`, `reference/**`, `hosts/**`, every other crate.

## Deliverable

- `render/surface_draw.rs` implementing the two functions.
- Unit tests in the same file (Surface-native: build a `Surface`, draw, assert
  via `row_text`/`cell`), covering at least:
  1. `border_box_draws_corners_and_edges` — e.g. a 4x3 box with distinct
     glyphs; assert every border cell's symbol and that the interior is
     untouched (still the fill glyph);
  2. `border_single_row_fills_horizontal`;
  3. `border_single_column_fills_horizontal`;
  4. `border_zero_size_draws_nothing`;
  5. `border_cells_carry_style` — a border cell's `style` equals the passed
     style;
  6. `overflow_indicators_place_both_glyphs` — up on top row, down on bottom
     row, both at column `x + width - 2`;
  7. `overflow_indicators_respect_flags` — only the enabled one appears;
  8. `overflow_indicators_suppressed_when_too_small` — `width < 4` or
     `height < 3` draws nothing.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force;
   if you think `unsafe` is unavoidable, STOP and report.
2. **The 1 MiB dual-platform size budget is real.** Minimal diff; no new
   dependencies; no `format!`-heavy code. This module is dead code until a
   later slice, so it must not pull anything new into the build. Test-only
   code is exempt.
3. **All untrusted text goes through the sanitizer.** Not applicable —
   glyphs here are theme/border glyphs and caller-resolved chars, not file
   content. Do not add text-ingestion paths.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** You are not touching
   dun-cli; if you find you need to, STOP and report.
5. **Tests are layered and colocated.** This module's tests live in the same
   file (`#[cfg(test)] mod tests`), matching `surface.rs`.
6. **Terminal-detection env is pinned in harnesses.** Not applicable — no
   process spawning in these tests.
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report — do not keep tuning.

## Verification (MANDATORY — you run it; iterate to green)

Run exactly these and paste results verbatim:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dun-ui --no-fail-fast
```

Loop: edit → test → fix → rerun, until green. Never claim a result without
the verbatim lines.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave all changes
  in the working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network. The
  only commands you run are file edits within Scope, `cargo`, and `python3`
  for parsing output.
- Minimal diff: no drive-by reformatting, renames, or comment changes
  outside the task.
- You MUST paste the real verbatim verification output. If a run did not
  reach green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command run, with the exact verbatim output lines
   (suite counts; note any environment-dependent skips).
3. The finding / verdict.
4. Stop-loss / open questions — where you stopped and why (empty if none).
