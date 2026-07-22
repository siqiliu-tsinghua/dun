# Brief 036 — Wide geometry step 3: one shared WindowGeometry

Implementation brief. **Step 3 of the plan in
`docs/dev/codex/brief-033-wide-geometry-plan.md`** (read its "Geometry decision"
and "Step 3" sections first). Steps 1–2 are done; step 4 (dun-cli) is separate.

## Goal

Compute each window's border/gutter/body geometry **once**, in one place that
knows the ambiguous-width mode, and have every dun-ui frame subsystem and the
renderer consume it — so layout measurement and actual painting agree under Wide
mode (where the body is narrower because the borders are 2 columns each). This
replaces the scattered `width - 2` / `1 + gutter` assumptions. **Narrow output is
byte-for-byte identical** (the geometry in Narrow equals today's values). This
step does NOT touch `dun-cli` (step 4) — but it exports the `WindowGeometry` type
and `UiShell::window_geometry` so step 4 can consume the same source of truth.

## Design

### The geometry (implement exactly this)

Add `pub struct WindowGeometry` (in `crates/dun-ui/src/model.rs`, exported from
`lib.rs`) with **window-local** rectangles (x/y relative to the window's own
top-left; the renderer/hit-test add the window's physical offset):

```rust
pub struct WindowGeometry {
    pub border_columns: u16, // b
    pub inner: Rect,         // window-local
    pub gutter: Rect,        // window-local (width 0 = no gutter)
    pub body: Rect,          // window-local
    pub right_border_x: u16, // window-local x of the right border's first column
}
```

Add `UiShell::window_geometry(width: u16, height: u16, line_count: Option<usize>)
-> WindowGeometry`:

- `b = self.border_columns()` (already exists: `char_width(vertical border glyph,
  mode)`).
- `inner`: `x = b`, `y = 1`, `width = width.saturating_sub(2*b)`,
  `height = height.saturating_sub(2)`.
- `separator_width = char_width(gutter separator glyph, mode)` (= `b` for the box
  separator).
- Gutter: `None` line_count (collapsed / missing buffer) ⇒ gutter width 0.
  Otherwise `candidate = decimal_digits(line_count) + separator_width`; keep the
  gutter only when `inner.width >= candidate + 4`, else width 0. The numeric
  label occupies the columns before the separator; the separator's first column
  is `inner.x + gutter_width - separator_width`.
- `gutter`: `x = inner.x`, `y = inner.y`, `width = gutter_width`, `height = inner.height`.
- `body`: `x = inner.x + gutter_width`, `y = inner.y`,
  `width = inner.width.saturating_sub(gutter_width)`, `height = inner.height`.
- `right_border_x = width.saturating_sub(b)`.

**Invariant checks (must hold):**

- 80-col, 1-digit line count, **Narrow**: `b=1`, `inner.x=1`, `inner.width=78`,
  `gutter.width=2`, `body.x=3`, `body.width=76`, `right_border_x=79` — i.e.
  today's numbers, so nothing Narrow moves.
- 80-col, 1-digit line count, **Wide**: `b=2`, `inner.x=2`, `inner.width=76`,
  `gutter.width=3`, `body.x=5`, `body.width=73`, `right_border_x=78`.

### Wiring

- **`model.rs`**: store a `WindowGeometry` on `UiWindow` (computed when the frame
  builds the window) and **remove the now-redundant `gutter_width` field** so it
  cannot disagree with the body rectangle.
- **`frame/mod.rs`** (and wherever `UiWindow` is constructed): compute the
  geometry via `shell.window_geometry(rect.width, rect.height, line_count)` and
  store it. `line_count` is `None` for a collapsed or missing-buffer window.
- **`frame/gutter.rs`, `frame/text.rs`, `frame/cursor.rs`, `frame/highlight.rs`,
  `frame/scroll.rs`**: take positions/sizes from `window.geometry` (cursor uses
  `geometry.body.x`; wrapped text uses `geometry.body.width`; search/selection/
  plugin spans add `geometry.body.x` and clip to `geometry.body`; the gutter uses
  `geometry.gutter`). Byte ranges convert with `dun_term::str_width(.., mode)`.
- **`render/surface_window.rs`**: derive `inner`/`gutter`/`body` from
  `window.geometry` (offset by the window's physical origin) instead of the local
  `width - 2` / gutter math; the scrollbar/overflow anchors at
  `geometry.right_border_x`; the `█` scrollbar glyph is 2 columns under Wide (it
  goes through `set_char`, which already handles that). Do NOT re-derive geometry
  here.
- **`hit.rs`**: treat every left/right border column as chrome and convert body
  clicks relative to `geometry.body`; use the stored geometry, not
  `window.gutter_width`.
- **`lib.rs`**: export `WindowGeometry`.

Do not introduce global state; `WindowGeometry` is a plain value passed on the
window model.

## Scope

- Files you MAY modify:
  - `crates/dun-ui/src/model.rs`, `crates/dun-ui/src/lib.rs`,
    `crates/dun-ui/src/shell.rs`
  - `crates/dun-ui/src/frame/mod.rs`, `.../frame/gutter.rs`, `.../frame/text.rs`,
    `.../frame/cursor.rs`, `.../frame/highlight.rs`, `.../frame/scroll.rs`
  - `crates/dun-ui/src/render/surface_window.rs`
  - `crates/dun-ui/src/hit.rs`
  - dun-ui test modules (`src/tests/model.rs` etc.) — update the `gutter_width`
    assertions to read `geometry.gutter.width`, keeping the Narrow values
- Files/areas you MUST NOT touch:
  - `crates/dun-cli/**` (step 4), `crates/dun-core/**`, `crates/dun-term/**`,
    `crates/dun-config/**`
  - `crates/dun-ui/src/render/surface_draw.rs` (step 2, done),
    `crates/dun-ui/src/text.rs` measurement helpers' signatures (step 1, done —
    you call them, don't re-sign them), `crates/dun-ui/src/surface.rs`
  - any `Cargo.toml`/`Cargo.lock`, `.git`, `docs/**`, `i18n/**`, `vm-test/**`,
    `reference/**`, `hosts/**`

If wiring the geometry reaches a file not listed here (a `UiWindow` construction
site, say), STOP and report rather than widening scope.

## Deliverable

- `WindowGeometry` + `UiShell::window_geometry`, stored on `UiWindow`,
  `gutter_width` removed, all consumers switched over.
- Tests (colocated):
  1. `wide_window_geometry_aligns_gutter_body_cursor_and_spans` — an 80-col,
     one-line, Wide fixture: assert `border_columns=2`, `gutter.width=3`,
     `body.x=5`, `body.width=73`, and that a cursor / a search match / a plugin
     span land at the right physical columns.
  2. `wide_rendered_window_body_ends_before_right_border` — fill the body through
     cell 77 and assert the right border occupies cells 78–79 with no overwrite.
  3. Keep the existing `gutter_width`-value assertions (now on `geometry`) and
     all Narrow frame/CLI golden snapshots byte-identical.
- Prove the geometry test is load-bearing: temporarily restore a `width - 2` (or
  `1 + gutter`) formula in one consumer and confirm the named test fails; then
  restore. (State that you did this.)

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`).
2. Narrow is sacred: the geometry equals today's values in Narrow (the invariant
   check above), so no Narrow golden snapshot may change. If one does, the
   geometry diverged from the old formulas — fix the geometry, not the snapshot.
3. The `WindowGeometry` rects are **window-local**; the renderer/hit-test add the
   window's physical origin. Do not mix the two.
4. This step's Wide result is not yet consistent with `dun-cli` view sync (step
   4); that's expected. The default mode is Narrow, so nothing regresses.
5. Match each file's local test style.
6. Stop-loss: if the same step fails twice, or wiring needs an out-of-scope file,
   STOP and report.

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
