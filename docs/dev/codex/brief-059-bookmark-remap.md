# Brief 059 — Bookmarks must follow their text

## Goal

A bookmark marks a place in the text. Today it marks a *line number*, and the
number does not move when the text does: bookmark line 10, insert five lines at
the top of the buffer, and the bookmark still says 10 while the line it marked
is now 15. Fix that, by moving bookmark storage into `TextBuffer` and remapping
it inside the one primitive every text mutation passes through.

This is a defect in a shipped feature, found while planning folding. It is
first because folding needs exactly the same remapping, and fixing the smaller
case establishes the mechanism and its tests on a change that can be reasoned
about whole.

## Current state (verified, cite these when you read them)

- Storage is CLI-side: `bookmarks: Vec<usize>` on `BufferState`.
  `crates/dun-cli/src/app/buffer_state.rs:28`
- The only two adjustments that exist are
  `normalize_bookmarks` — clamp to the last line, sort, dedup —
  and `remap_bookmarks_for_line_move` — swap two indices.
  `crates/dun-cli/src/app/buffer_state.rs:127-145`
- Neither shifts by a line delta. Callers are `editing.rs:298`,
  `editing.rs:332`, `file_io.rs:239`, `view_commands.rs:13`, `view_commands.rs:59`.
- Every text mutation funnels through one primitive, which splices whole lines
  and therefore knows the exact line-structure change:
  `crates/dun-core/src/buffer/edit.rs:219-249` — see the
  `self.lines.splice(range.start.line..=range.end.line, replacement)` call.
- `replace_all` calls that primitive **once per match, in reverse order**, so a
  "remember the last edit" design is wrong by construction.
  `crates/dun-core/src/buffer/search.rs:44-84`
- Undo and redo also reach it, so a fix placed there covers them for free.
  `crates/dun-core/src/buffer/undo.rs:135-168`

## The design (decided — implement this, do not redesign)

**Move bookmark storage into `TextBuffer`** and remap inside
`replace_range_inner`. The alternative — a journal the CLI drains after each
command — was rejected: it makes correctness depend on every future caller
remembering to drain, and this project prefers a structural guarantee over a
convention. Folding will place its fold ranges beside the bookmarks and reuse
the same hook.

For a replacement over `range` producing `replacement.len()` lines:

```text
removed  = range.end.line - range.start.line + 1
inserted = replacement.len()
```

Apply to every bookmark line `b`:

- `b < range.start.line` — unchanged.
- `b >= range.start.line + removed` — shift by `inserted - removed` (signed).
- otherwise (`b` is inside the replaced span) — the marked line no longer
  exists as such; clamp to `range.start.line`.

Then sort, dedup, and clamp to the last line, which is what
`normalize_bookmarks` already does — move that behaviour into core too.

Keep the existing user-visible surface exactly as it is: the three commands,
the gutter marker, the `[Mark]` status field, strict-circular navigation.
This brief changes *where the state lives and when it is corrected*, nothing
about what the user sees, except that a bookmark now stays on its line.

## Scope

Files you MAY modify:

- `crates/dun-core/src/buffer/model.rs` — bookmark storage on `TextBuffer`.
- `crates/dun-core/src/buffer/edit.rs` — the remap inside the primitive.
- `crates/dun-core/src/buffer/mod.rs` — re-exports if needed.
- `crates/dun-cli/src/app/buffer_state.rs` — remove the moved state and the
  two adjustment helpers; forward to the buffer.
- `crates/dun-cli/src/app/editing.rs`, `view_commands.rs`, `file_io.rs` — the
  call sites listed above.
- `crates/dun-ui/src/frame/gutter.rs` and `crates/dun-ui/src/model.rs` — only
  if the gutter's bookmark source changes shape.
- Tests: `crates/dun-core/src/buffer/tests/`, `crates/dun-cli/src/tests/markers.rs`.

Everything in `TEMPLATE.md`'s MUST NOT list still applies. Do not touch the
command ids, keymap defaults, menu entries, help text, or any `i18n/` catalog —
this brief adds no user-visible string.

## Deliverable

- Bookmarks stored on `TextBuffer`, remapped in `replace_range_inner`.
- The CLI reads them through the buffer; `normalize_bookmarks` and
  `remap_bookmarks_for_line_move` are gone or reduced to thin forwards.
- Tests below, each with its mutation named in your report.

## Tests (write these; name them in your report with the mutation that kills each)

1. `bookmark_shifts_when_lines_are_inserted_above` — bookmark line 10, insert
   five lines at line 0, bookmark is 15 and still marks the same text.
   Mutation: drop the shift branch; must fail.
2. `bookmark_shifts_when_lines_are_deleted_above` — the symmetric case.
   Mutation: same branch; must fail.
3. `bookmark_inside_a_replaced_span_clamps_to_the_edit_start` — bookmark line
   12, replace lines 10-15 with one line. Mutation: leave it unshifted; must
   fail.
4. `replace_all_shifts_every_bookmark_once` — several matches on separate
   lines, replacement text with a different line count, bookmarks above,
   between, and below. This is the case a last-edit design gets wrong; make
   the test prove ordering. Mutation: remap only the final replacement; must
   fail.
5. `undo_restores_bookmark_positions` — insert above, undo, bookmark is back
   where it started. Mutation: skip remapping on the undo path; must fail.
6. `move_line_still_swaps_its_bookmark` — the existing Move Line behaviour must
   survive the move to core. Mutation: drop the swap; must fail.
7. Whatever existing bookmark tests in `crates/dun-cli/src/tests/markers.rs`
   need to keep passing — they do, unchanged, or you explain why a changed
   expectation is a fix rather than a regression.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.**
2. **Size budget.** This is runtime code in `dun-core`, which every buffer
   pays for. Keep the storage a plain sorted `Vec<usize>` with no allocation
   when empty; no new dependency, no generic explosion. Claude measures on
   macOS and Debian.
3. **Bookmarks are not text.** Adding or moving a bookmark must not change the
   buffer's revision, enter undo history, or dirty the buffer. Dirty state is
   fingerprinted from text; check that it stays that way.
   `crates/dun-core/src/buffer/mod.rs:54-85`
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** If a symbol moves
   crates, its import list moves with it.
5. **A guard must be able to fail.** For each test above, run the mutation,
   paste the failure, then reverse the mutation **by editing it back** — never
   `git checkout`, the tree carries your other work.
6. **Stop-loss.** If the same step fails twice for the same reason, stop and
   report.

## Verification (MANDATORY — run these, paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Then, for each of the six mutations, paste the failing test name and the
assertion line, and confirm the tree is restored afterwards.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify files outside Scope.
- Full machine access, but touch NOTHING outside this repo, no network.
- Minimal diff.
- Paste the real verbatim output; if a run is not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why.
2. Verification — the three commands, verbatim.
3. Mutation proofs — six entries, each: mutation, failing test, restored.
4. Anything you found that this brief did not anticipate.
