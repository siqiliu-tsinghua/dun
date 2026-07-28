# Brief 037 — Wide geometry step 4: dun-cli consumes the shared geometry & mode

Implementation brief. **Final step (4) of the plan in
`docs/dev/codex/brief-033-wide-geometry-plan.md`** (read its "Step 4" and
"Cross-crate verdict" sections). Steps 1–3 are done and committed.

## Goal

Make `dun-cli`'s view state use the **same** window geometry and ambiguous-width
mode the renderer uses, so that under Wide mode the editor's body width,
wrapping, horizontal scrolling, and cursor visibility agree with what dun-ui
paints — and finish the cutover so **`unicode-width` is called only from
`dun-term`**. Narrow behavior stays byte-for-byte identical. When done, a Wide
80-column editor's CLI view context reports body width 73 (matching dun-ui), and
the whole-window Wide render lines up end to end.

## Why this step is needed (cross-crate)

`AppState::sync_view_for_area` → `ensure_cursor_visible` uses
`BufferViewContext.body_width`, which comes from `editor_body_width`
(`buffer_state.rs`), which independently assumes `rect.width - 2`. Under Wide the
renderer uses `rect.width - 4` (two 2-column borders) before the gutter, so CLI
and UI would disagree about which columns are visible and how lines wrap. The
shared `UiShell::window_geometry` (added in step 3) is the single source of
truth; `dun-cli` must consume it too.

## Exact change

1. **Body width from geometry.** Replace `editor_body_width` and its duplicated
   gutter math with `self.shell.window_geometry(rect.width, rect.height,
   Some(line_count)).body.width` (and `body.x` where the origin is needed). The
   call sites are `app/view_state.rs::buffer_view_context` (feeds
   `sync_view_for_area` in `app/frame.rs`) and anywhere else that computed a body
   width or gutter for view state.

2. **Thread the mode through `BufferState` view math.** The operations that
   compute display columns, wrapping, horizontal scroll, or cursor visibility
   (`buffer_state.rs` ~225–246 scroll, ~293–310 cursor visibility, ~410–480
   wrapped rows; `clamp_to_display_column`, `display_width_for_editor_char`, and
   the wrapped-row byte-column helpers) take an explicit `AmbiguousWidth`
   (sourced from `self.shell.profile.ambiguous_width`) and go through
   `dun_term::char_width`/`str_width`.

3. **Remaining CLI measurement sites** (from the earlier inventory): `files/text.rs`
   (~17–30), `app/status_view.rs`, `dialogs/line_input.rs`,
   `dialogs/file_dialog.rs`, `app/prompt_dialogs.rs`, `help/status.rs`,
   `help/content.rs`, and the overlay cursor construction — pass the mode and use
   the authority. Drag-selection and scrollbar mouse math (`app/mouse.rs`
   ~220–260) use the shared body origin/width from `window_geometry`, not a local
   `rect.width - 2`.

4. **Finish the cutover.** Remove every direct `unicode-width` use from `dun-cli`
   AND from `dun-ui` (including test modules), then remove the `unicode-width`
   dependency from `crates/dun-cli/Cargo.toml` and `crates/dun-ui/Cargo.toml`.
   Update `dun-cli/src/main.rs`'s prelude imports accordingly. After this,
   `unicode-width` is a dependency of `dun-term` only.

If threading the mode makes `buffer_state.rs` (already ~20k chars) materially
larger, extracting its cohesive wrapped-view methods into a new
`crates/dun-cli/src/app/buffer_wrap.rs` is acceptable and encouraged (see
`docs/dev/code-organization-guidelines.md`); keep the split mechanical.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/app/view_state.rs`, `.../app/frame.rs`,
    `.../app/buffer_state.rs` (and a new `.../app/buffer_wrap.rs` if you split),
    `.../app/editing.rs`, `.../app/mouse.rs`, `.../app/helper_panes.rs`,
    `.../app/search_replace.rs`, `.../app/status_view.rs`
  - `crates/dun-cli/src/files/text.rs`
  - `crates/dun-cli/src/dialogs/line_input.rs`, `.../dialogs/file_dialog.rs`,
    `.../app/prompt_dialogs.rs`
  - `crates/dun-cli/src/help/status.rs`, `.../help/content.rs`
  - `crates/dun-cli/src/main.rs` (prelude imports)
  - `crates/dun-cli/Cargo.toml`, `crates/dun-ui/Cargo.toml` (remove the
    `unicode-width` dependency — the one allowed manifest edit)
  - any `dun-ui`/`dun-cli` **test** module that still imports `unicode-width`
    (replace with `dun_term::char_width`/`str_width` or `AmbiguousWidth::Narrow`)
  - relevant dun-cli test modules for the named tests / snapshots
- Files/areas you MUST NOT touch:
  - `crates/dun-core/**`, `crates/dun-term/**`, `crates/dun-config/**`
  - the dun-ui **logic** from steps 1–3 (`text.rs`, `render/**`, `frame/**`,
    `surface*.rs`, `model.rs`, `shell.rs`) — you may only remove a leftover
    `unicode-width` import from a dun-ui test file
  - `Cargo.lock` beyond what `cargo` regenerates from the two manifest edits
  - `.git`, `docs/**`, `i18n/**`, `vm-test/**`, `reference/**`, `hosts/**`

If a change needs a file outside Scope, STOP and report.

## Deliverable

- CLI view state consuming `window_geometry`; the mode threaded through the CLI
  measurement/wrap/scroll/cursor paths; the `unicode-width` cutover finished.
- Tests:
  1. `wide_sync_view_uses_rendered_body_width` — for an 80-col one-line Wide
     editor, assert the CLI view context body width is 73 (independently of
     dun-ui), matching the renderer.
  2. `wide_wrapping_counts_ambiguous_glyphs_as_two_columns`.
  3. `wide_horizontal_scroll_keeps_cursor_inside_physical_body`.
  4. A Wide 80-column end-to-end snapshot (whole window: border, gutter, body,
     cursor line up).
  5. All existing Narrow golden snapshots (`src/tests/snapshots.rs`, the snapshot
     directory) remain **byte-for-byte unchanged**.
- Final integrity scan — paste it — must show `unicode_width` /
  `UnicodeWidthChar` / `UnicodeWidthStr` **only** in
  `crates/dun-term/src/width.rs`:
  ```
  grep -rn 'unicode_width\|UnicodeWidthChar\|UnicodeWidthStr' crates --include='*.rs'
  ```
- Prove a wide test is load-bearing: temporarily restore a `rect.width - 2` body
  width in the CLI path and confirm `wide_sync_view_uses_rendered_body_width`
  fails; then restore. (State that you did this.)

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`).
2. Narrow is sacred: no Narrow golden snapshot may change. If one does, a
   geometry/mode path diverged from today's values — fix the code, not the
   snapshot.
3. `dun-cli/src/main.rs` is the prelude hub: modules use `use crate::*`; when you
   drop the `unicode-width` import there, keep the import list consistent.
4. This is the step that makes Wide fully consistent CLI↔UI; after it, a Wide
   render should line up end to end. The default mode is still Narrow.
5. Size budget: removing a dependency and threading a `Copy` enum should be
   budget-neutral or better; add nothing heavyweight. Claude runs the
   dual-platform size gate.
6. Stop-loss: if the same step fails twice, or a change needs an out-of-scope
   file, STOP and report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

plus the final integrity grep above.

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why (note a `buffer_wrap.rs`
   split if you did one).
2. Verification — each command's verbatim output (suite counts; note skips) plus
   the integrity grep.
3. Verdict.
4. Stop-loss / open questions (empty if none).
