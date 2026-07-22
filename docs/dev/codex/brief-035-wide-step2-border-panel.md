# Brief 035 — Wide geometry step 2: width-aware border & panel painting

Implementation brief. **Step 2 of the plan in
`docs/dev/codex/brief-033-wide-geometry-plan.md`** (read its "Geometry decision"
section first — it defines the exact tiling this brief implements). Steps 1 (done)
and 3–4 are separate; do NOT do them here.

## Goal

Make the box-drawing border and bordered-panel painting correct when the border
glyph is 2 columns wide (Wide mode), while keeping the Narrow/ASCII path
(border glyph width 1) **byte-for-byte identical**. This step changes *painting*
only; it does NOT change window body/gutter geometry (step 3, `surface_window.rs`
inner/body math stays exactly as it is) or `dun-cli` (step 4). When done, a
Wide-mode 80-column window's top border is 2 corners + 38 horizontals filling
exactly 80 cells, and bordered menus/overlays inset their content past a 2-column
border.

## What already exists

- `Surface` stores `ambiguous_width` and its `set_text`/`set_char` already place
  a width-2 glyph into two cells (head + `wide_continuation`) and stop at the
  surface edge (`surface.rs`).
- `dun_term::char_width(ch, mode)` is the width authority.
- Step 1 routed all text *measurement* through the mode; this step is *painting*.

## Design (implement exactly the plan's geometry decision)

Let `b` = the border glyph's column width under the Surface's mode
(`char_width(vertical_glyph, mode)`; `b == 1` for Narrow and for ASCII in either
mode, `b == 2` for Unicode + Wide). The built-in corner/horizontal/vertical
glyphs share one width; add a test asserting that.

1. **`surface.rs`** — add a small read-only method
   `pub(crate) fn glyph_width(&self, ch: char) -> usize` returning
   `dun_term::char_width(ch, self.ambiguous_width).unwrap_or(1)`, so callers get
   `b` without reaching for the mode directly.

2. **`render/surface_draw.rs::draw_border`** — keep the current width-1 body of
   the function as the exact path when `b == 1` (so Narrow/ASCII output is
   unchanged), and add a `b > 1` path that tiles by glyph width per the plan:
   - Left corner at `x`; right corner at `x + width - b`.
   - Horizontal glyphs start at `x + b` and advance by `b`; paint a glyph only if
     all `b` of its cells fit before the right corner (`col + b <= x + width - b`).
   - Vertical glyphs at the same left/right physical columns as the corners
     (`x` and `x + width - b`), for each interior row.
   - **Odd residual**: if a one-column gap remains just before the right corner
     (odd Wide widths, e.g. width 79 → 37 horizontals, residual at 76, right
     corner at 77–78), fill it with a border-styled space — never a partial glyph
     and never overflow.
   - **Degenerate** (`height == 1`, or `width < 2 * b`): tile only complete
     horizontal glyphs and fill any residual with styled spaces (the Wide
     analogue of the current single-row/thin path).
   All painting goes through `set_char`/`surface.set_text`, which already handles
   the two-cell placement; `draw_border`'s job is choosing columns.

3. **`render/surface_draw.rs::draw_overflow_indicators`** — its
   `column = x + width - 2` assumes a 1-column right border. Make it replace the
   **last complete horizontal slot** before the right corner under the mode
   (i.e. `x + width - b - <slot width>`), not the right corner. Use the surface's
   `glyph_width`.

4. **`shell.rs`** — expose `pub fn border_columns(&self) -> u16` =
   `char_width(border_vertical_glyph, self.profile.ambiguous_width)` (as u16),
   for panel insets. (Use the active glyph set's vertical border char.)

5. **Bordered panels** — `render/menu.rs`, `render/surface_layers.rs`,
   `render/overlay.rs`, `render/surface_overlay.rs`, and `hit.rs`: replace the
   hardcoded 1-column border inset with `panel_inset = b + 1` so content clears a
   Wide border. Narrow stays `x + 2` / `width - 4`; Wide becomes `x + 3` /
   `width - 6`. Window titles that sit on the border use `top_inset = max(2, b)`
   (preserves Narrow placement, clears a Wide corner). Change ONLY the border/inset
   arithmetic these files use; do not touch unrelated layout.

## Scope

- Files you MAY modify:
  - `crates/dun-ui/src/surface.rs` (add `glyph_width`; do NOT change `set_text`/
    `fill_rect` — already done)
  - `crates/dun-ui/src/render/surface_draw.rs`
  - `crates/dun-ui/src/shell.rs`
  - `crates/dun-ui/src/render/menu.rs`, `.../render/surface_layers.rs`,
    `.../render/overlay.rs`, `.../render/surface_overlay.rs`
  - `crates/dun-ui/src/hit.rs`
  - dun-ui test modules for the named tests
- Files/areas you MUST NOT touch:
  - `crates/dun-ui/src/render/surface_window.rs` (window inner/body/gutter
    geometry = step 3) and `crates/dun-ui/src/text.rs` (step 1, done)
  - `crates/dun-ui/src/frame/**`
  - `crates/dun-cli/**`, `crates/dun-core/**`, `crates/dun-term/**`,
    `crates/dun-config/**`
  - any `Cargo.toml`/`Cargo.lock`, `.git`, `docs/**`, `i18n/**`, `vm-test/**`,
    `reference/**`, `hosts/**`

## Deliverable

- The `b`-aware border/panel painting above.
- Tests (colocated with the existing `surface_draw.rs` tests):
  1. `wide_border_tiles_eighty_columns_without_overwrite_or_overflow` — a
     Wide-mode 80×N surface: assert the top border is corners at cells 0–1 and
     78–79 with 38 horizontals between, every glyph head at an even column with a
     `wide_continuation` follower, and nothing painted past cell 79.
  2. `wide_border_places_odd_residual_before_right_corner` — width 79: right
     corner at 77–78, residual styled space at 76, 37 horizontals, no overflow.
  3. `wide_overflow_indicator_replaces_last_complete_horizontal_slot`.
  4. `wide_panel_content_does_not_overlap_border` — a bordered panel in Wide mode
     starts content at `x + 3`.
  5. A test asserting the built-in border sets have uniform corner/horizontal/
     vertical width in both modes (the invariant step 3 relies on).
- Every existing test in `surface_draw.rs` and all Narrow golden snapshots remain
  **byte-for-byte unchanged** (the `b == 1` path is the old code verbatim).

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`).
2. Narrow is sacred: the `b == 1` branch must be today's exact behavior; do not
   update any Narrow golden snapshot. If one changes, your Narrow path diverged.
3. This step's Wide output is not yet consistent with body layout (that is
   step 3) — that's expected; the gate tests here are border-local, and the
   default mode is Narrow so nothing regresses.
4. Match each file's local test style.
5. Stop-loss: if the same step fails twice, or a change needs a file outside
   Scope (e.g. it pulls in `surface_window.rs` geometry), STOP and report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why.
2. Verification — each command's verbatim output (suite counts; note skips).
3. Verdict.
4. Stop-loss / open questions (empty if none).
