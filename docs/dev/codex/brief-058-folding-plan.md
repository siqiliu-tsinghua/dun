# Brief 058 — Folding: design-only plan

## Goal

Produce a concrete, step-by-step **implementation plan** for code folding in
`dun`. Write no source code. The plan must be detailed enough that each step
can be dispatched as its own implementation brief and gated independently, and
honest enough that Claude can decide from it whether the feature is worth its
cost at all.

Folding here means: collapsing a contiguous range of buffer lines so the editor
draws one placeholder row in their place, and expanding it again. The user must
be able to fold and unfold without any plugin, any language knowledge, and any
configuration.

## Why this is not a small feature (read before planning)

`dun` already has a display seam, `EditorTextDisplay` in
`crates/dun-ui/src/display_map.rs`. It is a **within-line** seam: source byte ↔
display column, plus soft-wrap segmentation inside one line
(`wrapped_segments`, `wrapped_row_count`, `wrapped_row_column_for_source_byte`).

Folding needs the **line-level** analogue — source line index ↔ visible row —
and that layer does not exist. Today every consumer open-codes the same range:

```
crates/dun-ui/src/frame/scroll.rs:96     for (visible_y, line_index) in (buffer.first_line..buffer.buffer.line_count())
crates/dun-ui/src/frame/cursor.rs:62     for line_index in buffer.first_line..position.line
crates/dun-ui/src/frame/highlight.rs:173,219,341
crates/dun-ui/src/frame/text.rs:21
```

`first_line` has ~91 references across `crates/dun-cli/src` and
`crates/dun-ui/src` (excluding tests), concentrated in
`app/buffer_viewport.rs` (~26), `app/mouse.rs` (~9), `ui/hit.rs` (~5). Every
one of them is a place where "the next line down" currently means "index + 1"
and would have to mean "the next *visible* line".

The project has done exactly this shape of refactor once: the F12/F13
restoration introduced the within-line seam in its own commit (`3b69844`)
before the features landed. Two defects were caught at the gate in that seam
commit — a 20x long-line render regression, and a duplicate untested
`scroll_status` fork. Treat that as the cost model, not as a reason to skip the
seam.

## Context pointers

Read `AGENTS.md` first (invariants, engineering rules), then:

- `docs/dev/window-management.md` — the tiled workspace model.
- `docs/dev/code-organization-guidelines.md` — file-size thresholds and where
  things belong.
- `crates/dun-ui/src/display_map.rs` — the existing within-line seam. Your
  line-level design should look like a sibling of this, not a rewrite of it.
- `crates/dun-ui/src/frame/` — `text.rs`, `scroll.rs`, `cursor.rs`,
  `highlight.rs`: the render loops that consume `first_line`.
- `crates/dun-cli/src/app/buffer_viewport.rs` — viewport/scroll state and the
  soft-wrap visual-row logic (`first_visual_row`).
- `crates/dun-cli/src/app/mouse.rs`, `crates/dun-ui/src/hit.rs` — screen row →
  buffer position.
- Bookmarks are the precedent for **per-buffer line state that survives
  edits**: find their storage and their remap-on-edit path (search
  `bookmark`), including the Move Line remap. Fold ranges have the same
  problem and should reuse the same approach if it fits.

## Scope

- Files you MAY modify: **NONE — design only.** Produce the plan as your final
  report. Do not create files, do not edit source, do not add tests.
- The MUST NOT list from `TEMPLATE.md` applies in full.

## Deliverable — the plan

Every claim in it carries `path:line` evidence. A plan that asserts without
citing is not accepted.

1. **Call-site inventory.** Every place that assumes "visible lines are
   `first_line..line_count()`" or that converts between a screen row and a
   buffer line, grouped by what it does (render, scroll, cursor visibility,
   hit test, status, highlight, plugin snapshot). For each: the file, the
   lines, and one sentence on what it must become. This inventory is the
   single most valuable part of the deliverable — under-scoping it is how a
   cross-cutting change fails.

2. **The line-level seam.** Its API, its owner crate, and its data structure.
   At minimum it must answer: given a fold set, what is the visible row for a
   source line, the source line for a visible row, the number of visible rows,
   and the next/previous visible line. Say explicitly how it **composes with
   soft wrap**, which already maps one logical line onto several visual rows —
   the two mappings stack, and the order matters.

3. **Fold state model.** Where fold ranges live (per buffer? per window? both
   have consequences when the same buffer is open in two windows — say which
   and why), how they are represented, and how they are remapped when text is
   inserted or deleted above, inside, or across a fold boundary. Name the
   bookmark code you are following.

4. **Semantic decisions**, each with a recommendation and a one-line
   rationale — these are the questions Claude will freeze before step 2:
   - What can be folded: an arbitrary selected range (manual), or only ranges
     derived from indentation, or both?
   - What does the placeholder row show, and does it participate in the gutter,
     bookmarks, current-line highlight, and search-match highlight?
   - Cursor movement: does Down from the row above a fold land on the
     placeholder or on the first line inside it? What happens to a cursor that
     is inside a range being folded?
   - Search: a match inside a folded range — auto-expand, expand-on-jump, or
     count-but-skip? Note what the existing match cache and Search Results
     window would each need.
   - Editing: an edit inside a folded range, a delete spanning a fold
     boundary, and undo of either. What is the least surprising behaviour that
     is also the cheapest to implement?
   - Persistence: does a fold survive reload, buffer switch, window close?
     (Bookmarks are the precedent; follow it unless you argue otherwise.)

5. **Step decomposition.** Three to five steps, each independently gateable,
   each with its own named test gate. Step 1 must be **the seam alone, with no
   user-visible change** — ideally byte-neutral in the release build, and say
   whether you believe it can be. State for each step what would make Claude
   reject it.

6. **Full-trail inventory for the user-visible step.** Command ids, default
   keybindings (say which chords are free — `Ctrl+X` second strokes `A`, `D`,
   `F`, `G`, `I`, `W` were free as of 2026-07-28; verify), menu entries, help
   text, status messages, and the i18n keys those imply across all ten
   catalogs in `i18n/`. Under-counting this is the classic way a `dun` feature
   ships half-done.

7. **Risks and open questions**, ranked. Include anything you could not
   resolve from the code, and say what evidence would resolve it.

8. **A recommendation on whether to build it at all.** You are not required to
   conclude yes. If the inventory comes out larger than the feature is worth,
   say so plainly and say what a cheaper 80% would be.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** Every crate root has
   `#![forbid(unsafe_code)]`.
2. **The 1 MiB dual-platform size budget is real.** Current margin is 280,272
   bytes on the binding platform, so the budget is not the blocker here — but
   estimate the cost anyway, and flag anything that would add a broad generic
   instantiation or a `format!`-heavy layer to the render path.
3. **All untrusted text goes through the sanitizer.** A fold placeholder that
   echoes buffer text (for example "the first line of the folded range") is
   untrusted text and must go through the sanitized path. Say so in the plan.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** Modules use
   `use crate::*`.
5. **Tests are layered and colocated.** Unit/behaviour tests in each crate's
   `src/tests/`; rendering assertions against the in-house `Surface` grid
   (there is no `TestBackend` — ratatui was retired at `858e876`); PTY/tmux
   tests in `crates/dun-cli/tests/`.
6. **A guard must be able to fail.** For each invariant your plan protects,
   name the test *and* the mutation that would break it. This project treats a
   test that passes against a broken implementation as worse than no test.
7. **Stop-loss is real.** If you cannot answer a design question from the
   code, write it in Risks rather than inventing an answer.

## Verification

This is a design-only brief: there is nothing to build and no test to run.
Do not run `cargo test` to "check" the tree. Your verification obligation is
different and stricter: **every `path:line` in your plan must be real.** Spot
check them by reading the files you cite.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify any file. This brief produces prose only.
- Full machine access, but touch NOTHING outside this repo, no network.
- Do not invent file paths, function names, or line numbers. If you are
  unsure, read the file.

## Report format (your final message)

1. The call-site inventory (item 1).
2. The seam design (item 2) and fold state model (item 3).
3. Semantic decisions with recommendations (item 4).
4. Step decomposition with per-step gates (item 5).
5. Full-trail inventory (item 6).
6. Risks and open questions (item 7).
7. Your build/don't-build recommendation (item 8).
