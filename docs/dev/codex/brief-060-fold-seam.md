# Brief 060 — Folding step 1: the line-level seam, identity only

## Goal

Introduce the line-level coordinate seam folding needs, and route every site
that converts between a buffer line and a screen row through it — with a fold
set that is **always empty**, so nothing about the editor's behaviour changes.

When you are done there is exactly one authority for "which buffer line is on
which visible row", and today's answer through it is identical to today's
answer without it. No fold commands, no fold state the user can create, no new
string, no `i18n/` change.

This is step 1 of the plan in `docs/dev/codex/brief-058-folding-plan.md`. Claude
reviewed that plan, verified its citations, and froze the decisions below.
Where this brief and the plan differ, **this brief wins**.

## Why identity-first

`dun` has a within-line seam already, `EditorTextDisplay` in
`crates/dun-ui/src/display_map.rs`: source byte ↔ display column, and soft-wrap
segmentation inside one line. There is no line-level analogue. Six render loops
and roughly ninety references open-code `first_line..line_count()` and
`first_line + screen_row`.

If folding were added on top of that, every one of those sites would become a
place where the editor can disagree with itself about which line the user is
looking at. So the seam lands first, alone, provably behaviour-neutral, and the
existing test suite is the oracle: **it must pass unchanged**. A test you had to
edit is a behaviour change, and you must say so rather than adjust the
expectation.

## What to build

Two types in a new `crates/dun-ui/src/line_map.rs`, re-exported beside
`EditorTextDisplay`:

```rust
FoldRange { start_line, end_line_exclusive }   // sorted, non-overlapping, half-open
FoldSet                                         // empty in this step; no allocation when empty

VisibleLine::Source { line }
VisibleLine::Fold { range }                     // constructed but unreachable in this step

EditorLineDisplay::new(line_count, &FoldSet)
    visible_row_count()
    placement_for_source_line(line)
    item_for_visible_row(row)
    source_anchor_for_visible_row(row)
    next_visible_anchor(line) / previous_visible_anchor(line)
    iter_from_visible_row(row)

EditorVisualRows::new(buffer, line_map, text_display, body_width)
    total_rows()
    global_row_for_position(position)
    top_for_global_row(row)
    position_for_global_row_column(row, column)
```

`EditorLineDisplay` is the line-level sibling of the byte-level seam.
`EditorVisualRows` centralises the global wrapped-row arithmetic that is
currently duplicated between `crates/dun-ui/src/frame/text.rs:72-124` and
`crates/dun-cli/src/app/buffer_viewport.rs:323-427` — that consolidation is
most of this step's value and is worth doing even if folding never ships.

Mapping order, which must hold once folds exist and must be encoded now:

1. source lines + `FoldSet` → visible items;
2. `EditorTextDisplay` soft-wraps only `VisibleLine::Source`;
3. a `VisibleLine::Fold` contributes exactly one row regardless of body width;
4. a hidden source line contributes zero rows.

With an empty `FoldSet`, (3) and (4) are unreachable — write them anyway, and
unit-test them directly on a constructed `FoldSet`, because that is the only
part of the fold semantics this step can prove.

## Call sites to route

From the verified inventory in brief 058. Convert each to ask the seam instead
of doing its own arithmetic:

- Rendering: `crates/dun-ui/src/frame/text.rs:6-124`,
  `gutter.rs:4-62`, `scroll.rs:4-119`, `cursor.rs:6-82`,
  `highlight.rs:15-446`, `mod.rs:60-147`.
- Viewport and scrolling: `crates/dun-cli/src/app/buffer_viewport.rs` — the
  whole file is in scope; it is the densest concentration (~26 references).
- Cursor, hit testing, mouse: `crates/dun-ui/src/hit.rs:180-281`,
  `crates/dun-cli/src/app/mouse.rs:55-360`.
- Status: `crates/dun-cli/src/help/status.rs:31-78`.
- View state and lifecycle: `crates/dun-cli/src/app/view_state.rs:64-97`,
  `view_commands.rs:130-173`, `frame.rs`, `buffer_state.rs:95-114`,
  `file_io.rs:191-251`.
- `crates/dun-ui/src/model.rs` — `BufferView`'s raw `(first_line,
  first_visual_row)` becomes a typed `ViewportTop { anchor_line, wrapped_row }`.

**Out of scope in this step:** the plugin highlight path
(`crates/dun-cli/src/app/highlight.rs`, `plugins.rs`, `plugins/worker.rs`).
Claude decided that when folds exist the client will keep sending **one
contiguous request** spanning the first to last visible source line, hidden
lines included, under the existing 512-line cap — no batching, no `HighlightJob`
change, no protocol change. With an empty fold set that is what happens today,
so this path needs nothing.

## Scope

Files you MAY modify: everything named under "Call sites to route", plus
`crates/dun-ui/src/lib.rs` (module declaration and re-export), the new
`crates/dun-ui/src/line_map.rs`, and the test modules of the crates you touch
(`crates/dun-ui/src/tests/`, `crates/dun-cli/src/tests/`).

You MAY NOT touch: `crates/dun-core/**` (this seam is a rendering concern),
the plugin path named above, any `i18n/` catalog, command ids, keymap defaults,
menu entries, or help text. Everything in `TEMPLATE.md`'s MUST NOT list applies.

## Acceptance

1. `cargo test --workspace --no-fail-fast` passes **with no test edited**.
   If you believe a test must change, stop and report it instead — that is a
   behaviour change and Claude decides it, not you.
2. New unit tests in `crates/dun-ui/src/tests/` for the seam itself, exercising
   a non-empty `FoldSet` directly:
   - `line_map_identity_matches_raw_ranges` — with an empty set, visible row N
     is source line N for a range of buffer sizes, and `visible_row_count`
     equals `line_count`.
   - `line_map_hides_folded_lines` — one fold of three lines yields one visible
     item for them, and `visible_row_count` drops by two.
   - `line_map_round_trips_placement_and_lookup` — for every source line,
     `item_for_visible_row(placement_for_source_line(l))` returns `l` or the
     fold containing it.
   - `visual_rows_compose_fold_then_wrap` — a fold contributes exactly one row
     at any body width, while a wrapped source line contributes its wrapped
     count.
   Name the mutation that kills each in your report.
3. No new `format!` in a render loop, no new allocation per frame when the fold
   set is empty, no new dependency.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.**
2. **Size budget.** This is runtime code on the render path. The empty-fold
   case must stay allocation-free and should compile to roughly what it
   replaces; Claude measures on macOS and Debian before this ships. Avoid
   generic explosion: prefer concrete types over generics parameterised at each
   call site.
3. **Untrusted text still goes through the sanitizer.** This step renders no
   new text, so nothing new reaches the terminal — keep it that way.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** Moving a symbol means
   updating its import list in the same change.
5. **The existing suite is the oracle.** 881 tests pass today. Any change in
   that number, other than your four new ones, needs an explanation.
6. **A guard must be able to fail.** Mutate, paste the failure, restore **by
   editing back** — never `git checkout`, the tree carries uncommitted work.
7. **Stop-loss.** Same failure twice for the same reason: stop and report.
   Under-scoping is the expected failure here, so if you find a coordinate site
   the list above missed, name it and stop rather than expanding silently.

## Verification (MANDATORY — run these, paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Then the four mutations from Acceptance 2.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify files outside Scope.
- Full machine access, but touch NOTHING outside this repo, no network.
- Minimal diff.
- Paste real verbatim output; if a run is not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why.
2. Verification — the three commands, verbatim, with the test count.
3. Mutation proofs — four entries, each: mutation, failing test, restored.
4. Any call site the inventory missed, and anything you had to leave
   inconsistent.
