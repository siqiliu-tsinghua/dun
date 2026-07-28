# Brief 062 — Folding step 3: one fold set, and the placeholder row

## Goal

Two things, in this order, because the first is a defect and the second is the
feature:

1. **Collapse the duplicate fold state.** There are currently two fold sets.
   `BufferState.folds` in `dun-cli` (`buffer_state.rs:26`) is what the renderer
   reads; `TextBuffer.folds` in `dun-core` (`model.rs:217`) is what the edit
   remap maintains. The set that stays correct across an edit is not the set
   that gets drawn. `TextBuffer` is the owner — delete the `BufferState` field
   and read through the buffer everywhere.
2. **Draw the placeholder.** `VisibleLine::Fold` currently reaches the render
   loops and has no visual form. Give it one, with the interaction rules below.

Still no way for a user to create a fold: no command, no key, no menu entry.
Tests construct fold sets directly. That comes in step 4.

## How the duplicate happened, and why it matters for your tests

Step 1 introduced the seam with an always-empty set and put the field on
`BufferState`. Step 2 moved fold ownership into `TextBuffer` and was explicitly
forbidden from touching `dun-cli`, so it could not remove the older field. The
split was Claude's, not a mistake by either step.

The reason to state it here is that it is the third instance of one pattern in
this feature: **the code was tested along the path that does not execute.**
Step 1's identity mapping had no direct test while its fold path did; step 2's
degenerate-range guard had an assertion but no input reaching it; and now the
fold set that edits maintain is not the one that renders. When you write tests
for this step, ask which path production actually takes and test that one
first.

## Placeholder specification (decided — implement this)

A folded range draws exactly **one row**, at any body width, never wrapped and
never horizontally scrolled:

```
▶ [12] fn parse_header(input: &str) -> Result<Header>
```

- The glyph comes from the terminal's glyph profile, with an ASCII fallback
  (`>`), like every other non-ASCII chrome character. Do not hard-code `▶`.
- `[N]` is the number of source lines the fold hides, in decimal.
- The excerpt is the first line of the folded range, **passed through
  `EditorTextDisplay` sanitisation** like any other buffer text, then clipped
  to the body width. Build the trusted prefix separately from the untrusted
  excerpt and concatenate after sanitising — never sanitise the whole composed
  string, which would make the prefix's own characters depend on file content.
- The placeholder carries **no translated string**, so this brief adds no
  `i18n/` key.

Layer rules on the placeholder row:

| Layer | Behaviour |
| --- | --- |
| Gutter line number | the fold's **start** line |
| Bookmark marker | shown if **any** line in the fold is bookmarked |
| Current-line highlight | applied when the cursor's line is inside the fold |
| Selection | shaded when the selection intersects the fold in any part |
| Search match | whole-row match style when any hidden line contains a match; active-match style wins over ordinary match |
| Plugin syntax spans | **never** applied to the placeholder |

Cursor and mouse: any position inside a fold maps to the placeholder row at
column 0, and a click on the placeholder resolves to `(fold.start_line, 0)`.
Step 1 already routes these through the seam; make sure the folded case is
right rather than reworking the plumbing.

## Scope

Files you MAY modify:

- `crates/dun-cli/src/app/buffer_state.rs` — delete the `folds` field.
- `crates/dun-cli/src/app/frame.rs`, `view_state.rs`, `buffer_viewport.rs` —
  the sites that read `buffer.folds`; read `buffer.buffer.folds()` instead.
- `crates/dun-ui/src/frame/text.rs`, `gutter.rs`, `highlight.rs` — the
  placeholder row and its layers.
- `crates/dun-ui/src/render/surface_window.rs` — only if layer ordering needs
  it; the existing order is body, current line, plugin, search, selection.
- `crates/dun-term/src/glyphs.rs` — the fold glyph and its ASCII fallback.
- Tests in the crates you touch.

You MAY NOT touch: command ids, keymap defaults, menu entries, help text, any
`i18n/` catalog, or the plugin request path (`app/highlight.rs`, `plugins.rs`,
`plugins/worker.rs`) — the decision that a folded buffer still sends one
contiguous request including hidden lines stands.

## Acceptance

1. `cargo test --workspace --no-fail-fast` passes with **no existing test
   edited**. 893 today.
2. After step 1, there must be exactly one fold set in the tree. Prove it:
   `grep -rn "folds" crates/dun-cli/src` shows reads through the buffer and no
   `BufferState` field.
3. New tests, each named in your report with the mutation that kills it:
   - `folded_range_draws_one_row_at_any_width` — including a body width narrower
     than the excerpt, and with word wrap on, where a wrapped source line
     contributes several rows and the placeholder still contributes one.
   - `placeholder_excerpt_is_sanitised` — a folded first line containing an
     escape sequence and a bidirectional override renders as visible markers.
     The oracle must be hard-coded expected cells, **not** a call to the same
     sanitiser the implementation uses.
   - `placeholder_gutter_shows_start_line_and_aggregated_bookmark`
   - `placeholder_takes_current_line_selection_and_search_styles`
   - `plugin_spans_never_paint_the_placeholder`
   - `cursor_and_click_inside_a_fold_resolve_to_the_start_line`
   - `empty_fold_set_renders_byte_identically` — the path production takes;
     assert a frame with no folds against the same expectations as before this
     change.
4. No new allocation per frame when the fold set is empty.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.**
2. **Size budget.** Debian is at 776,496 with 272,080 to spare. This is the
   render path: no `format!` per row where a reusable buffer will do, no
   generic explosion.
3. **The excerpt is untrusted text.** It comes from a file. It goes through the
   sanitiser before it reaches the surface, and the test for that must not use
   the sanitiser as its own oracle.
4. **A guard must be able to fail.** Mutate, paste the failure, restore **by
   editing back** — never `git checkout`.
5. **Stop-loss.** Same failure twice for the same reason: stop and report.

## Verification (MANDATORY — run these, paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Then the mutations from Acceptance 3.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify files outside Scope.
- Full machine access, but touch NOTHING outside this repo, no network.
- Minimal diff.
- Paste real verbatim output; if a run is not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why.
2. The duplicate-state removal, with the grep that proves one owner remains.
3. Verification — the three commands, verbatim, with the test count.
4. Mutation proofs — one per test above.
5. Anything the specification did not cover.
