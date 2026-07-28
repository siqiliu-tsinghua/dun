# Brief 063 — Folding step 4: commands, and the full trail

## Goal

Let a user fold. Two commands, their keys, menu entries, help text, status
messages, and all ten translation catalogs — plus the navigation rules that
decide what happens when something needs to reach a line that is currently
hidden.

This is the last step. After it, folding is a feature rather than machinery:
steps 1–3 built the coordinate seam (`058447f`), the fold state and its edit
remapping (`d7e91fb`), and the placeholder row (`0c4e921`).

One thing inverts here. The previous three steps kept warning that tests
covered the fold path while production only ever ran the empty-fold path. From
this step on, **the fold path is the production path**. Test it as the live
one, and keep the empty-fold assertions that already exist — they are now the
regression guard for everyone who never folds anything.

## The two commands

| Command id | Default key | View menu |
| --- | --- | --- |
| `edit.toggle_fold` | `Ctrl+X,F` | `Toggle Fold (F)` |
| `edit.unfold_all` | `Ctrl+X,A` | `Unfold All (A)` |

Verified free as of this brief: `Ctrl+X` second strokes `A`, `D`, `F`, `G`,
`I`, `W`; View menu mnemonics `A`, `B`, `F`, `G`, `I`, `J`, `Q`, `T`, `U`, `Y`.
Re-check rather than trusting this line.

`ALL_COMMAND_IDS` goes from 91 to 93. Put the handlers in a new
`crates/dun-cli/src/app/folding.rs` rather than growing `editing.rs`, which is
already near the size guideline.

## Behaviour (decided — implement this)

**Toggle Fold**

- With a selection spanning two or more lines: fold those lines. Use the
  existing selected-line rule where a selection ending at column zero excludes
  that final line. Merge with any overlapping existing fold into their union.
  Move the cursor to `(start_line, 0)` and clear the selection.
- With no selection, cursor inside a fold: unfold that fold.
- Otherwise: a status message saying at least two lines must be selected. No
  state change.

**Unfold All** clears every fold in the focused buffer and reports how many.

**Reaching a hidden line.** These expand the fold containing their target
*before* moving: Go To Line, next/previous bookmark, and a **committed** search
jump (Find submitted, Find Next, Find Previous, and the Replace confirmation
presenting its current target). Horizontal, word, Home and End movement from a
placeholder also expands first, so the cursor never enters hidden text
invisibly.

**Find preview must NOT expand.** The live preview as the user types in the
Find prompt has to leave folds alone. The prompt's saved state holds cursor,
selection and search only — no fold snapshot — so expanding during preview
would make cancelling lossy. This distinction is the subtle part of the step;
if you cannot see how to keep preview and commit separate, stop and report
rather than guessing.

**Vertical movement** already works through the seam from step 1: Down from the
row above a fold lands on the placeholder, and the next Down lands after the
fold. Verify rather than rework.

## The full trail

Under-counting this is how a `dun` feature ships half-done, so enumerate it
before you start.

- Two `EditCommand` variants, two ids in `ALL_COMMAND_IDS`, `command_id`, and
  `command_from_id`.
- Two default keybindings in the compiled keymap.
- Two View menu entries with the mnemonics above.
- Two help entries — help keys derive from command ids in `HELP_SECTIONS`.
- Status messages. At minimum: folded N lines, unfolded, nothing to unfold,
  selection too short, buffer missing. Follow the shape of the existing
  bookmark and whitespace status keys.
- **All ten catalogs in `i18n/`.** The validator discovers files rather than
  naming them, so a missing key names the file and key it is missing from —
  but only if you run it.

## Scope

Files you MAY modify: `crates/dun-core/src/command.rs`;
`crates/dun-config/src/commands.rs` and `src/keys/keymap.rs`;
`crates/dun-ui/src/frame/menu.rs`; `crates/dun-cli/src/app/` (including the new
`folding.rs`), `src/help/content.rs`, `src/ui_text/`; every file in `i18n/`;
and the test modules of the crates you touch.

You MAY NOT touch: the plugin request path, `scripts/`, `vm-test/`, or any
document outside `docs/dev/codex/`. Everything in `TEMPLATE.md`'s MUST NOT list
applies.

## Acceptance

1. `cargo test --workspace --no-fail-fast` passes. 900 today; existing tests
   change only if a changed expectation is a fix you explain.
2. New tests, each named in your report with the mutation that kills it:
   - `toggle_fold_folds_the_selected_lines_and_clears_the_selection`
   - `toggle_fold_without_a_selection_unfolds_at_the_cursor`
   - `toggle_fold_reports_when_fewer_than_two_lines_are_selected`
   - `unfold_all_clears_every_fold_and_reports_the_count`
   - `go_to_line_and_bookmark_jumps_expand_a_hidden_target`
   - `committed_search_jump_expands_but_preview_does_not` — both halves in one
     test, because the pair is the invariant.
   - `every_catalog_has_the_fold_trail` — or rely on the existing completeness
     test if it already covers new keys by construction; say which.
3. The command-id round trip holds: `command_from_id(command_id(c)) == c` for
   both new commands, via the existing contract test.
4. Folding is still invisible to a user who never folds: the empty-fold
   assertions from steps 1–3 pass untouched.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.**
2. **Size budget.** Debian was 776,496 at step 1 with 272,080 to spare, and
   steps 2–3 are not yet measured. This step adds ten catalogs' worth of keys,
   which historically costs about 4 KiB. Keep the handlers small; no `format!`
   where a status key with a placeholder will do.
3. **Menu mnemonics are declared, never derived.** Both entries need theirs,
   unique within the View menu.
4. **Translations are values only.** Catalog files carry the base text; the
   editor appends mnemonics and key captions itself. Do not put `(F)` or a key
   name inside a translated string.
5. **A guard must be able to fail.** Mutate, paste the failure, restore **by
   editing back** — never `git checkout`.
6. **Stop-loss.** Same failure twice for the same reason: stop and report. The
   preview-versus-commit distinction is the likeliest place to get stuck; that
   is a reporting case, not a guessing case.

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
2. The full-trail checklist, ticked, with the ten catalogs listed.
3. Verification — the three commands, verbatim, with the test count.
4. Mutation proofs — one per test above.
5. Anything the behaviour specification did not cover.
