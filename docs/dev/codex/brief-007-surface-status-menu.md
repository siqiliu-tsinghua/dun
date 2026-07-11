# Brief 007 — Surface status + menu layer drawing

Implementation brief. Renderer-replacement slice 3b: port the status bar,
menu bar, and active-dropdown drawing from the ratatui `Frame`/`Buffer` to the
in-house `Surface`. These are faithful Surface mirrors of the existing
`render_status` / `render_menu` / `render_active_menu`, drawing with the
Surface primitives from slice 3a. No entry-point change, no caller change; the
window/overlay layers and the entry-point wiring are later slices.

## Goal

`crates/dun-ui` gains a private module `render/surface_layers` with three
`pub(crate)` functions that draw onto a `Surface`:

- `draw_status(surface, shell, status, area)` — mirrors
  `render::status::render_status`.
- `draw_menu_bar(surface, shell, menu, area)` — mirrors
  `render::menu::render_menu`.
- `draw_active_menu(surface, shell, menu, area)` — mirrors
  `render::menu::render_active_menu`.

Each reproduces its ratatui twin's glyphs, styles, and layout exactly; the
only differences are the drawing target (`Surface` instead of `Buffer`) and
that styles are `dun_term::Style` passed directly (no `to_ratatui_style`
conversion). The module is `#[allow(dead_code)]` (nothing calls it yet, like
the sibling `surface`, `surface_emit`, and `surface_draw` modules), covered by
Surface-native unit tests, and the `dun-ui` suite is green.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-ui/src/render/status.rs` — `render_status` (the twin) and the
  pure helper `sanitized_status_text_for_width` (REUSE it; it is `pub(crate)`).
- `crates/dun-ui/src/render/menu.rs` — `render_menu` and `render_active_menu`
  (the twins) plus the pure helpers you REUSE unchanged:
  `dropdown_rect_for_menu`, `clamp_menu_rect`, `menu_visible_entry_range`,
  `menu_entry_text` (private — call through the existing twin's logic; if you
  need it, widen it to `pub(crate)` in menu.rs as the ONLY change there, and
  say so). Study how each twin computes geometry and styles.
- `crates/dun-ui/src/render/surface_draw.rs` — the slice-3a primitives you
  build on: `draw_border(surface, x, y, w, h, glyphs, style)` and
  `draw_overflow_indicators(surface, x, y, w, h, up, down, above, below,
  style)`.
- `crates/dun-ui/src/surface.rs` — `Surface`. Draw runs with
  `set_text(x, y, &s, style)` (returns display width advanced, clips at the
  right edge); fill backgrounds with `fill_rect(x, y, w, h, ' ', style)`;
  read back with `cell(x, y)` / `row_text(y)`.
- `crates/dun-ui/src/render/chrome.rs` — `sanitize_chrome_text`,
  `vertical_overflow_up`, `vertical_overflow_down` (REUSE).
- `crates/dun-ui/src/tests/rendering.rs` + `tests/support.rs` — how fixtures
  are built: `UiShell::default()`, `shell.frame_for_workspace_with_menu*` to
  get a `UiFrame` whose `.menu` is a `MenuBar` and `.status` is a `StatusBar`.

## Specification

Signatures (put them in `render/surface_layers.rs`; `TuiRect` is used only as
the geometry carrier the shared helpers already return):

```rust
use ratatui::layout::Rect as TuiRect;
use crate::surface::Surface;
use crate::{MenuBar, StatusBar, UiShell};

pub(crate) fn draw_status(surface: &mut Surface, shell: &UiShell, status: &StatusBar, area: TuiRect);
pub(crate) fn draw_menu_bar(surface: &mut Surface, shell: &UiShell, menu: &MenuBar, area: TuiRect);
pub(crate) fn draw_active_menu(surface: &mut Surface, shell: &UiShell, menu: &MenuBar, area: TuiRect);
```

Behavior, mirroring the twins:

- `draw_status`: compute the line via
  `sanitized_status_text_for_width(shell, status, area.width as usize)`, fill
  `area` with the `status_bar` palette style and a space, then `set_text` the
  line at `(area.x, area.y)` with that same style. (The ratatui `Paragraph`
  fills the area with its base style then paints the text; reproduce that.)
- `draw_menu_bar`: fill `area` with the `menu_bar` palette style and a space,
  then draw the item spans left to right starting at `area.x`, advancing the
  cursor by each `set_text` return value. The span sequence is exactly
  `render_menu`'s: a leading `" "` in `menu_text`; then per item a `" "`, the
  first label char in the hotkey style, the remaining chars in the item style,
  and a trailing `" "`. Active item uses `menu_active`/`menu_active_hotkey`;
  inactive uses `menu_text`/`menu_hotkey`. Clip naturally at the surface edge
  (set_text already does).
- `draw_active_menu`: mirror `render_active_menu` exactly — bail on no active
  item / missing item / no dropdown rect / clamp failure; fill the clamped
  rect with `menu_panel` and spaces; `draw_border` with `menu_panel_border`;
  compute `content_width`/`max_rows`/visible range; `draw_overflow_indicators`
  (using `vertical_overflow_up/down(shell)` glyphs and the `start > 0` /
  `end < len` flags) with `menu_panel_border`; then for each visible entry
  `set_text` its `menu_entry_text` at `(rect.x + 2, y)` with `menu_active` when
  selected else `menu_panel_text`.

Faithfulness is the acceptance bar: for the same fixture, a Surface drawn by
these functions must contain the same glyphs at the same cells as the ratatui
path produces. Do NOT change the ratatui twins.

## Scope

- Files you MAY modify:
  - `crates/dun-ui/src/render/surface_layers.rs` (new; implementation +
    `#[cfg(test)] mod tests`);
  - `crates/dun-ui/src/render/mod.rs` (add
    `#[allow(dead_code)] pub(crate) mod surface_layers;`);
  - `crates/dun-ui/src/render/menu.rs` — ONLY if you must widen a single
    existing helper (e.g. `menu_entry_text`) from private to `pub(crate)` to
    reuse it; no logic change. If you do, note it in the report.
- Files/areas you MUST NOT touch (defaults for every brief):
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`,
    `docs/**`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock` (no new dependencies);
  - `crates/dun-ui/src/surface.rs`, `render/surface_draw.rs`,
    `render/chrome.rs`, `render/status.rs`, `render/window.rs`,
    `render/overlay.rs`, and the ratatui drawing bodies in `render/menu.rs`
    (the twins stay);
  - `vm-test/**`, `reference/**`, `hosts/**`, every other crate.

## Deliverable

- `render/surface_layers.rs` implementing the three functions.
- Surface-native unit tests in the same file. Build fixtures with
  `UiShell::default()` and the `frame_for_workspace*` builders, draw onto a
  `Surface`, and assert via `row_text`/`cell`. Cover at least:
  1. `status_line_fills_row_and_draws_text` — text present at row start,
     row fully covered by `status_bar` style;
  2. `menu_bar_draws_items_with_hotkey_style` — a known item's first char
     carries the hotkey style and the rest the item style; the bar background
     is `menu_bar`;
  3. `menu_bar_marks_active_item` — the active item's cells carry
     `menu_active`/`menu_active_hotkey`;
  4. `active_menu_draws_panel_border_and_entries` — border glyphs on the
     panel edges and a known entry label present inside;
  5. `active_menu_shows_overflow_indicator_when_truncated` — with a selection
     forcing scroll, the up indicator glyph appears (mirror the existing
     `short_dropdown_keeps_selected_menu_entry_visible` fixture);
  6. `active_menu_absent_when_no_selection` — no active menu ⇒ surface
     unchanged.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force;
   if you think `unsafe` is unavoidable, STOP and report.
2. **The 1 MiB dual-platform size budget is real.** Minimal diff; no new
   dependencies; no new `format!`-heavy layers. This module is dead code until
   a later slice; it must not pull anything new into the build. Test-only code
   is exempt.
3. **All untrusted text goes through the sanitizer.** Status/menu text must
   go through the existing sanitized helpers (`sanitize_chrome_text`,
   `sanitized_status_text_for_width`, `menu_entry_text`) exactly as the twins
   do. Never draw raw label/status bytes.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** You are not touching
   dun-cli; if you find you need to, STOP and report.
5. **Tests are layered and colocated.** These tests live in the same file
   (`#[cfg(test)] mod tests`), matching `surface.rs`/`surface_draw.rs`.
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
