# Brief 061 — Folding step 2: fold state and edit remapping

## Goal

Give `TextBuffer` a fold set that survives editing, beside the bookmarks that
already live there. No user can create a fold yet — there is still no command,
no key, no menu entry — but the state exists, the core maintains it correctly
across every mutation, and tests prove it.

Step 1 (`058447f`) built the coordinate seam and routed everything through it
with an always-empty fold set. This step fills the set. Step 3 will render
placeholders and step 4 will expose commands.

## What exists to build on

- `crates/dun-ui/src/line_map.rs` — `FoldRange`, `FoldSet`, `EditorLineDisplay`,
  `EditorVisualRows`. **`FoldSet` and `FoldRange` move to `dun-core`** in this
  step, because the buffer must own them; `dun-ui` re-exports or imports them
  so the seam keeps compiling unchanged.
- `crates/dun-core/src/buffer/model.rs` — `TextBuffer` already carries
  `bookmarks: Vec<usize>`; the fold set goes beside it.
- `crates/dun-core/src/buffer/edit.rs:219` — `replace_range_inner`, the one
  primitive every mutation reaches, already remaps bookmarks. Read that block
  first: the fold remap goes in the same place and follows the same reasoning.
- `crates/dun-core/src/buffer/line_ops.rs` — `swap_adjacent_lines` holds its
  bookmarks aside across the generic remap and maps them with
  `swapped_adjacent_line`. Folds need the analogous treatment, and the comment
  there explains why a caller that knows its intent declares it rather than
  letting the primitive infer it.

## The remap policy (decided — implement this)

For a replacement over `range` producing `inserted` lines in place of
`removed`:

- A fold entirely **above** the replaced span (its `end_line_exclusive <=
  range.start.line`) is unchanged.
- A fold entirely **below** (its `start_line >= range.start.line + removed`)
  shifts both endpoints by `inserted - removed`.
- A fold that **touches the replaced span in any way** — including a deletion
  that spans only its first or last line — is **removed**. The text it covered
  no longer means what it meant; keeping a range over rewritten lines is worse
  than dropping it. Say so in a comment.
- Afterwards: clamp to the line count, drop ranges shorter than two lines,
  merge overlaps, keep the set sorted and non-overlapping.

A fold swap under `swap_adjacent_lines` follows the bookmark precedent: a fold
covering exactly the two swapped lines is meaningless to swap, so the touching
rule applies and it is dropped. Do not add a special case.

Fold mutations must **not** change the buffer revision, enter undo history, or
dirty the buffer — the same invariant bookmarks have. Undo of an edit that
dropped a fold does **not** restore that fold; fold state is deliberately
outside `EditTransaction`.

## Scope

Files you MAY modify:

- `crates/dun-core/src/buffer/model.rs` — the fold set field.
- `crates/dun-core/src/buffer/mod.rs` — the accessor surface: `folds()`,
  `set_folds()`, `insert_fold()`, `remove_fold_at(line)`, `clear_folds()`, and
  whatever normalisation helper they share. Keep it small and typed.
- `crates/dun-core/src/buffer/edit.rs` — the remap in `replace_range_inner`.
- `crates/dun-core/src/buffer/line_ops.rs` — the swap path.
- `crates/dun-core/src/lib.rs` — re-export `FoldRange`/`FoldSet`.
- `crates/dun-ui/src/line_map.rs` — move the two types out, import them back.
- `crates/dun-ui/src/lib.rs` — adjust the re-export.
- Tests: `crates/dun-core/src/buffer/tests/`, `crates/dun-ui/src/tests/line_map.rs`.

You MAY NOT touch: `crates/dun-cli/**` (no CLI wiring in this step), any
`i18n/` catalog, command ids, keymap defaults, menu entries, help text, or the
plugin path. Everything in `TEMPLATE.md`'s MUST NOT list applies.

## Acceptance

1. `cargo test --workspace --no-fail-fast` passes with **no existing test
   edited**. 885 today; your additions are the only increase.
2. New tests in `crates/dun-core/src/buffer/tests/`, each named in your report
   with the mutation that kills it:
   - `fold_above_an_edit_is_untouched`
   - `fold_below_an_edit_shifts_by_the_line_delta`
   - `fold_touched_by_an_edit_is_dropped` — cover three shapes: an edit inside
     the fold, a deletion spanning its first line, and one spanning its last.
   - `replace_all_remaps_every_fold_once` — the reverse-order multi-call case
     that defeats any "remember the last edit" design.
   - `undo_shifts_folds_back_but_does_not_resurrect_a_dropped_one`
   - `swapping_lines_drops_a_fold_over_them`
   - `fold_mutations_do_not_dirty_the_buffer_or_enter_undo`
3. The `FoldSet` invariants — sorted, non-overlapping, no range shorter than
   two lines, clamped to the line count — hold after every operation above.
   Assert them in a helper the tests share rather than by eye.
4. Empty-fold behaviour is unchanged: no allocation, and the identity path in
   `EditorLineDisplay` still holds. The step-1 seam tests must pass untouched.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.**
2. **Size budget.** Debian is at 776,496 with 272,080 to spare, so there is
   room, but this is `dun-core` and every buffer pays. An empty `FoldSet` must
   not allocate. Claude measures on both platforms after this lands.
3. **Do not infer intent from content.** The gate rejected exactly that in the
   bookmark step: a caller that knows what it is doing declares it. If you find
   yourself comparing text to decide what an edit meant, stop and report.
4. **`crates/dun-cli/src/main.rs` is the prelude hub** — but you are not
   touching `dun-cli` in this step, so if you think you must, stop and report.
5. **A guard must be able to fail.** Mutate, paste the failure, restore **by
   editing back** — never `git checkout`.
6. **Stop-loss.** Same failure twice for the same reason: stop and report.

## Verification (MANDATORY — run these, paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Then the mutations from Acceptance 2.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify files outside Scope.
- Full machine access, but touch NOTHING outside this repo, no network.
- Minimal diff.
- Paste real verbatim output; if a run is not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why.
2. Verification — the three commands, verbatim, with the test count.
3. Mutation proofs — one per test above: mutation, failing test, restored.
4. Anything the policy above did not cover, and what you did about it.
