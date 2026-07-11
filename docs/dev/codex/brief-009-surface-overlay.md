# Brief 009 — Surface overlay/modal layer drawing

Implementation brief. Renderer-replacement slice 3d (overlay half): port the
modal overlay layer (`render::overlay::render_overlay`) from the ratatui
`Frame`/`Buffer` to the in-house `Surface`, wire it into the Surface entry
point, and extend the parity harness to full-frame parity over the
overlay-bearing fixtures. This is the last render layer; after it the Surface
path draws the entire frame and the dun-cli cutover (a separate,
non-Codex slice) can proceed.

The acceptance bar is mechanical and self-correcting: `tests/surface_parity.rs`
compares Surface output to ratatui output cell by cell. Run it, read the
`mismatch at (x, y)` message, fix, rerun. The harness is the oracle.

## Goal

`crates/dun-ui` gains `render/surface_overlay.rs` with a `pub(crate)` function

```rust
pub(crate) fn draw_overlay(
    surface: &mut Surface,
    shell: &UiShell,
    overlay: &UiOverlay,
    area: TuiRect,
) -> Option<(u16, u16)>;
```

that draws the modal exactly as `render_overlay` draws it and **returns** the
input-field cursor position (if the overlay has an input with a
`cursor_column`) instead of calling `set_cursor_position`. It is wired into
`render_ui_frame_to_surface` after the active menu, and its cursor **overrides**
the window cursor when present (the modal is on top). The parity harness then
passes over the full frame for the overlay fixtures.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-ui/src/render/overlay.rs` — the ratatui twin `render_overlay`
  (mirror it region for region, in the same order) and the reusable
  `pub(crate)` helpers `overlay_layout` (returns `OverlayLayout { rect, .. }` —
  use it for the panel rect; it sanitizes internally and calls the same layout
  math the twin uses, so the rect is identical) and the pure width/fit helpers
  it relies on.
- `crates/dun-ui/src/render/surface_frame.rs` — the entry point. After the
  active-menu draw and the "Slice 3d: overlay drawing lands here" comment, add:
  `if let Some(overlay) = &ui_frame.overlay { cursor =
  draw_overlay(surface, shell, overlay, TuiRect::new(0, 0, width, height))
  .or(cursor); }`. The `.or(cursor)` gives overlay-cursor precedence.
- Primitives (all landed): `Surface::set_text`, `fill_rect`, `set_style`,
  `style_run`, `cell`, `width`, `height`; `surface_draw::draw_border`,
  `surface_draw::draw_overflow_indicators`; the sanitize/fit helpers
  `render::chrome::sanitize_chrome_text`, `crate::fit_text_to_width`,
  `crate::display_width`.
- `crates/dun-ui/src/tests/surface_parity.rs` — the harness.
  `assert_region_matches(surface, buffer, x, y, w, h)` is `pub(super)` and
  `render_both` builds both renderers. The module comment states the parity
  contract (glyph+fg+bg exact; modifiers `surface ⊆ ratatui`).
- `crates/dun-ui/src/tests/rendering.rs` — the overlay fixtures to mirror
  (prompt overlay, file-dialog overlay, modal-list overflow).

## Specification — the CRITICAL fill distinction

`render_overlay` uses two different fill mechanisms, and the Surface twin must
match each exactly or parity fails on leaked glyphs:

- **Style-only passes (preserve the glyph underneath, change only the
  style).** ratatui uses `cell.set_style(..)` here, which keeps the symbol.
  Use `Surface::style_run` / `set_style`, NOT `fill_rect`:
  - the **scrim**: every cell of `area` (`render_overlay`'s first loop) — it
    recolors the already-drawn windows beneath the modal without erasing their
    glyphs;
  - the **panel background**: `Block::default().style(modal)` over the panel
    `rect` is `buf.set_style(rect, modal)` — style-only over the whole panel
    rect (glyphs under the interior are preserved, then overwritten only where
    border/title/content draw).
- **Opaque space-fills (reset the glyph to a space).** ratatui uses
  `cell.set_char(' ').set_style(..)` here. Use `Surface::set_text` with a
  space run (or `fill_rect`):
  - the **input row** background (`rect.x+2 .. rect.x+width-2`) before the
    input text;
  - the **selected list row** background before the entry text.

Everything else mirrors `render_overlay` directly: bail if `area.width < 12 ||
area.height < 5`; get `rect` from `overlay_layout`; `draw_border(rect,
modal_border)`; title (`rect.width > 6`, fitted `" {title} "`, `modal_text` at
`rect.x+2, rect.y`); content lines (fitted to `inner_width`, `modal_text`,
stopping at `rect.y + rect.height - 1`); input row (opaque fill in
`modal_input`, fitted text, then compute the cursor position exactly as the
twin: `rect.x + 2 + min(cursor_column, inner_width-1)` on the input row —
return it); list entries (selected row opaque-filled in `modal_input`, text in
`modal_input` when selected else `modal_text`); `draw_overflow_indicators(rect,
up, down, list_has_more_above, list_has_more_below, modal_border)`; buttons
(centered, `modal_text`). Preserve the exact row-advance order of the twin so
row positions match.

The returned cursor is `Some((x, row))` only when the overlay has an input and
`cursor_column.is_some()`, else `None`.

Faithfulness is the bar: for the same fixture the Surface must match the
ratatui buffer over the full frame per the parity contract. Do NOT change the
ratatui path.

## Scope

- Files you MAY modify:
  - `crates/dun-ui/src/render/surface_overlay.rs` (new);
  - `crates/dun-ui/src/render/mod.rs` (add
    `#[allow(dead_code)] pub(crate) mod surface_overlay;`);
  - `crates/dun-ui/src/render/surface_frame.rs` (wire `draw_overlay`; replace
    the overlay TODO comment);
  - `crates/dun-ui/src/tests/surface_parity.rs` (add full-frame parity tests
    for the overlay fixtures).
- Files/areas you MUST NOT touch (defaults for every brief):
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`, `PLAN.md`,
    `docs/**`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock` (no new dependencies);
  - `crates/dun-ui/src/surface.rs`, the other `surface_*` render modules,
    `render/window.rs`, `render/overlay.rs`, and the other ratatui render
    bodies (the twins stay; `overlay_layout` is already `pub(crate)`);
  - `vm-test/**`, `reference/**`, `hosts/**`, every other crate.

## Deliverable

- `render/surface_overlay.rs` with `draw_overlay` + any private helpers.
- `surface_frame.rs` wired to `draw_overlay` with overlay-cursor precedence.
- Parity tests in `surface_parity.rs` mirroring the three overlay fixtures
  from `tests/rendering.rs` (prompt overlay, file-dialog overlay, modal-list
  overflow), each asserting `assert_region_matches` over the **full** surface,
  and — where the fixture has an input cursor — asserting the returned cursor
  equals ratatui's `get_cursor_position`.
- If any fixture cannot reach full-frame parity, STOP and report the exact
  `mismatch at (x, y)` rather than weakening `assert_region_matches`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force;
   if you think `unsafe` is unavoidable, STOP and report.
2. **The 1 MiB dual-platform size budget is real.** Minimal diff; no new
   dependencies. This module is dead code until the cutover; it must not pull
   anything new into the build. Test-only code is exempt.
3. **All untrusted text goes through the sanitizer.** Title, lines, input,
   list, and buttons must pass through `sanitize_chrome_text` exactly as the
   twin does before drawing. Never draw raw overlay bytes.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** You are not touching
   dun-cli; if you find you need to, STOP and report.
5. **Tests are layered and colocated.** Parity tests live in
   `tests/surface_parity.rs`; any unit tests for `draw_overlay` internals live
   in `surface_overlay.rs`'s `#[cfg(test)] mod tests`.
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
