# Brief 008 — Surface window layer drawing

Implementation brief. Renderer-replacement slice 3c (window half): port the
window layer (`render::window::render_window` and its sub-passes) from the
ratatui `Frame`/`Buffer` to the in-house `Surface`, wire it into the Surface
entry point, and extend the parity harness to the full frame for
overlay-free fixtures. The overlay/modal layer is the next brief (009).

The acceptance bar is mechanical and self-correcting: `tests/surface_parity.rs`
compares the Surface output to the ratatui output cell by cell. Run it, read
the `mismatch at (x, y)` message, fix, rerun. You do not need to reason about
correctness in the abstract — the harness is the oracle.

## Goal

`crates/dun-ui` gains `render/surface_window.rs` with a `pub(crate)` function

```rust
pub(crate) fn draw_window(
    surface: &mut Surface,
    shell: &UiShell,
    window: &UiWindow,
    workspace: TuiRect,
) -> Option<(u16, u16)>;
```

that draws one window onto the Surface exactly as `render_window` draws it
onto the ratatui buffer, and **returns** the focused terminal cursor position
(the Surface path owns no terminal handle) instead of calling
`set_cursor_position`. It is wired into `render_ui_frame_to_surface` so the
full frame (menu, status, windows, active dropdown) is drawn, and the parity
harness passes over the full frame for every overlay-free fixture.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-ui/src/render/window.rs` — the ratatui twin `render_window` and
  its sub-functions (`render_gutter`, `render_current_line`,
  `render_selection`, `render_plugin_highlights`, `render_search_matches`,
  `render_scrollbar`, `render_horizontal_edges`, `render_window_title`,
  `offset_rect` [already `pub(crate)`], `window_title_for_width`
  [`pub(crate)`]). Mirror each pass and, critically, **preserve their exact
  order** — paint order is load-bearing (body < current-line < plugin
  highlights < search < selection < edges < scrollbar).
- `crates/dun-ui/src/render/surface_frame.rs` — the entry point. It currently
  has a placeholder window loop that only computes the cursor via
  `window_cursor_position`. Replace that: call `draw_window` per window and
  fold the returned cursor (`cursor = draw_window(...).or(cursor)`), then
  delete the now-unused `window_cursor_position` helper. Keep the overlay
  TODO comment.
- Primitives you build on (all already landed):
  - `Surface::set_text`, `fill_rect`, `set_style`, `style_run`, `cell`,
    `width`, `height` (`crates/dun-ui/src/surface.rs`).
  - `render::surface_draw::draw_border` (window border).
  - The overlay passes (current line, selection, plugin highlights, search
    matches) recolor already-painted body cells → use `style_run`.
- Body text: `render_window` builds `sanitized_line_to_ratatui(shell, line)`
  per visual row and blits them with `Paragraph::new(..).style(editor_text)`.
  The Paragraph fills the body area with the `editor_text` base style, then
  paints each line's segments left to right, clipped to the body width. Mirror
  this: fill the body area with `editor_text` + spaces, then for each
  `SanitizedLine` draw its `DisplaySegment`s with `set_text` at the running
  column, each segment styled by its `DisplayClass` (the same palette lookup
  `render::chrome`'s private `display_segment_style` does:
  Text→`editor_text`, Control→`control`, Escape→`escape`,
  Truncation→`truncation`). Add a small `draw_sanitized_line` helper (in
  `surface_window.rs`) for this; clip at the body's right edge.
- `crates/dun-ui/src/tests/surface_parity.rs` — the harness.
  `assert_region_matches(surface, buffer, x, y, w, h)` is `pub(super)`; reuse
  it. `render_both` builds both renderers for a fixture. The module comment
  states the parity contract (glyph+fg+bg exact; modifiers `surface ⊆
  ratatui`).
- `crates/dun-ui/src/tests/rendering.rs` — the existing ratatui fixtures to
  mirror; the overlay-free ones are the parity targets (see below).

## Specification

`draw_window` mirrors `render_window` pass for pass:

1. Compute `area = offset_rect(window.rect, workspace)`; if `area.width == 0
   || area.height == 0`, return `None`.
2. Fill `area` with the `editor` palette style and spaces (the ratatui
   `Block::default().style(editor)`).
3. `draw_border` over `area` with `window.border` and the focused/unfocused
   border style (`window_border_focused` / `window_border`).
4. Draw the title via `window_title_for_width` at `(area.x + 2, area.y)` with
   the focused/unfocused title style, exactly as `render_window_title` (bail
   when `area.width <= 4`).
5. If `window.collapsed || area.width <= 2 || area.height <= 2`, return the
   cursor result (step 11) — no interior.
6. Gutter: same geometry as `render_gutter` — fill the gutter columns with the
   `gutter` style, draw each `UiGutterLine.label`, and the separator glyph in
   the `gutter_separator` style.
7. Body: fill the body area with `editor_text` + spaces, then draw each
   `window.body` line via `draw_sanitized_line` at successive rows, clipped to
   the body width (mirror the `Paragraph`).
8. Current line, plugin highlights, search matches, selection — in this order,
   each a `style_run` over the mapped row/columns with the matching palette
   style (`current_line`, the five `syntax_*`, `search_match` /
   `active_search_match`, `selection_text`), mirroring the respective
   `render_*` sub-functions' clipping.
9. Horizontal edge indicators: mirror `render_horizontal_edges` (unicode/ascii
   glyphs, `truncation` style).
10. Scrollbar: mirror `render_scrollbar` (thumb glyph, `scrollbar_thumb`
    style).
11. Cursor: mirror the tail of `render_window` — compute
    `(area.x + cursor.x, area.y + cursor.y)`, return `Some((x, y))` only if it
    falls inside `area`, else `None`. (Steps 5's early return yields `None`.)

Wire into `render_ui_frame_to_surface`: in the window loop,
`cursor = draw_window(surface, shell, window, workspace).or(cursor)`; keep the
"last window wins" fold already documented there. Draw the active menu after
the loop as now.

Faithfulness is the bar: for the same fixture the Surface must match the
ratatui buffer per the parity contract over the full frame (menu row, window
region, status row, and any active dropdown). Do NOT change the ratatui path.

## Scope

- Files you MAY modify:
  - `crates/dun-ui/src/render/surface_window.rs` (new);
  - `crates/dun-ui/src/render/mod.rs` (add
    `#[allow(dead_code)] pub(crate) mod surface_window;`);
  - `crates/dun-ui/src/render/surface_frame.rs` (wire `draw_window`; remove
    `window_cursor_position`);
  - `crates/dun-ui/src/tests/surface_parity.rs` (add full-frame parity tests
    for the overlay-free fixtures);
  - `crates/dun-ui/src/render/chrome.rs` — ONLY if you must widen the private
    `display_segment_style`'s palette mapping for reuse; prefer replicating
    the four-arm `DisplayClass` match inside `surface_window.rs` instead. If
    you touch chrome.rs, note it.
- Files/areas you MUST NOT touch (defaults for every brief):
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`, `PLAN.md`,
    `docs/**`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock` (no new dependencies);
  - `crates/dun-ui/src/surface.rs`, `render/surface_draw.rs`,
    `render/surface_layers.rs`, `render/window.rs`, `render/overlay.rs`, and
    the other ratatui render bodies (the twins stay);
  - `vm-test/**`, `reference/**`, `hosts/**`, every other crate.

## Deliverable

- `render/surface_window.rs` with `draw_window` + `draw_sanitized_line` and
  any private helpers.
- `surface_frame.rs` wired to `draw_window`, `window_cursor_position` removed;
  its existing tests still pass.
- Parity tests in `surface_parity.rs` mirroring these overlay-free fixtures
  from `tests/rendering.rs`, each asserting `assert_region_matches` over the
  **full** surface:
  1. plain single window (`frame_for_workspace`, 60x8 into 60x10);
  2. tiny tiled split (the `8x2`/`8x4` tiled fixture);
  3. viewport-polish markers fixture (scrollbar + edges);
  4. plugin-highlight fixture (syntax spans);
  5. the full menu/window/status layout snapshot fixture;
  6. an active-dropdown fixture (menu open over a window) — full frame,
     confirming the dropdown still matches once windows are drawn under it.
- If any fixture cannot reach full-frame parity, STOP and report the exact
  `mismatch at (x, y)` rather than weakening `assert_region_matches`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force;
   if you think `unsafe` is unavoidable, STOP and report.
2. **The 1 MiB dual-platform size budget is real.** Minimal diff; no new
   dependencies; no new `format!`-heavy layers. This module is dead code until
   the cutover; it must not pull anything new into the build. Test-only code
   is exempt.
3. **All untrusted text goes through the sanitizer.** Body lines are already
   `SanitizedLine`; title/gutter text uses the existing sanitized helpers
   (`window_title_for_width`, the gutter labels are pre-built). Never draw raw
   buffer bytes; do not re-route around the sanitizer.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** You are not touching
   dun-cli; if you find you need to, STOP and report.
5. **Tests are layered and colocated.** Parity tests live in
   `tests/surface_parity.rs`; any unit tests for `draw_window` internals live
   in `surface_window.rs`'s `#[cfg(test)] mod tests`.
6. **Terminal-detection env is pinned in harnesses.** Not applicable — these
   tests spawn no process (ratatui `TestBackend`, no PTY).
7. **Stop-loss is real.** If the same parity fixture fails twice for the same
   reason after a genuine fix attempt, STOP and report the mismatch — do not
   weaken the assertion or special-case cells.

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
3. The finding / verdict — including any parity mismatch you could not
   resolve.
4. Stop-loss / open questions — where you stopped and why (empty if none).
