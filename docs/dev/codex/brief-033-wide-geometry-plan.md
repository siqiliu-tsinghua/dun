# Brief 033 — Design plan for wide-terminal window geometry (design only)

**Diagnostic/design brief. NO source change. Produce a step-by-step
implementation plan; Claude evaluates it, then dispatches the steps as separate
implementation briefs.**

## Goal

Produce a concrete, step-by-step implementation *plan* (not code) for making
`dun` render its UTF-8 box-drawing UI correctly when the terminal treats Unicode
East Asian Ambiguous-width glyphs as **double-width** (the
`terminal.ambiguous-width = wide` mode). Today, in that mode the box-drawing
border overflows/clips because the render geometry assumes every glyph is one
column. The plan must solve the *geometry* problem, not just width arithmetic,
and it must break the work into ordered steps each of which is independently
implementable, testable, and keeps the default (Narrow) behavior byte-for-byte
unchanged.

## What already exists (do not re-plan these)

Committed groundwork you build the plan on top of:

- `dun-term`: `AmbiguousWidth { Narrow, Wide }` on `TerminalProfile`
  (`ambiguous_width` field), and the central functions
  `dun_term::char_width(ch, mode) -> Option<usize>` /
  `dun_term::str_width(s, mode) -> usize` (the only place allowed to touch
  `unicode-width`; Narrow = `width`, Wide = `width_cjk`). This is the width
  authority; the plan routes everything through it.
- `dun-config`: `terminal.ambiguous-width = narrow | wide` sets the mode on the
  resolved `TerminalProfile`. `UiShell` exposes it as `shell.profile.ambiguous_width`.
- `dun-ui/src/surface.rs`: `Surface` already carries the mode
  (`with_ambiguous_width` builder; `SurfaceRenderer::render` and `snapshot.rs`
  pass `shell.profile.ambiguous_width`). `Surface::set_text` already places a
  width-2 glyph into two cells with a `wide_continuation` marker, and stops at
  the surface edge. `fill_rect` only ever fills width-1 glyphs (space/dot).

So a single ambiguous glyph already consumes the correct number of cells inside
the `Surface` grid under Wide mode. What is NOT handled is everything that
computes *positions and sizes* assuming one column per glyph.

## The problem to solve (the plan must address each)

1. **Border drawing.** `dun-ui/src/render/surface_draw.rs::draw_border` walks
   columns one at a time (`for column in x+1..right { set_char(column, ...) }`)
   and places corners at `x` and `right = x + width - 1`. Under Wide mode each
   box glyph occupies 2 cells, so: the horizontal run overwrites its own
   continuation cells, and the right vertical/corners land where `set_text`
   truncates at the surface edge — the border is wrong. How should a border of a
   `width`-column window be drawn when its glyphs are 2 cells wide? (Corners,
   horizontal runs, verticals; odd/even `width`; what happens at an odd residual
   column.)
2. **Inner/body geometry.** `dun-ui/src/render/surface_window.rs` computes
   `inner_area = width-2, height-2` and a `body_area` after the gutter, assuming
   1-column borders. Under Wide mode the left+right borders consume more columns,
   so the usable body is narrower. The plan must say exactly how body width,
   gutter placement, current-line/cursor column, plugin-highlight and
   search-match coordinates are derived so they stay consistent with what
   `Surface`/`draw_border` actually paint.
3. **Text measurement threading.** `dun-ui/src/text.rs` has `display_width`,
   `fit_text_to_width`, `status_text_for_width`, `wrap_line_segments` — the
   layout/truncation helpers — and they call `unicode-width`'s narrow reading
   directly. They are used by ~15 files (menu, status, overlay, window title,
   scroll, highlight, frame). The plan must specify how the mode reaches each
   (they mostly run inside functions that already have `shell`/profile), and
   whether to add an `AmbiguousWidth` parameter, a small context struct, or
   another shape — with the tradeoff. It must enumerate the call sites.
4. **Cross-crate reach.** Determine whether wide body width affects `dun-cli`'s
   view sync (`AppState::sync_view_for_area` / scrolling / horizontal scroll),
   or whether it can be contained in `dun-ui`. State the answer with evidence.
5. **Consistency invariant.** Whatever geometry is chosen, layout measurement
   (`display_width`) and actual painting (`Surface`) must agree, or content
   overflows/underfills. The plan must name the one or two load-bearing tests
   that would prove a wide border and body line up (e.g. a Wide-mode render of a
   bordered 80-col window whose top border fills exactly 80 cells with no
   overflow, and whose body content starts/ends at the right columns).

## Context pointers

- Read `AGENTS.md` (invariants) and skim `docs/dev/solaris-vm.md` (the motivating
  quirk: Solaris tmux renders box-drawing and `◆` as 2 columns).
- Key files: `dun-ui/src/surface.rs`, `.../render/surface_draw.rs`,
  `.../render/surface_window.rs`, `.../text.rs`, and the `display_width`
  callers under `.../render/*` and `.../frame/*`; `dun-term/src/width.rs` and
  `profile.rs`; `dun-cli/src/app/*` for the view-sync question.
- The 1 MiB dual-platform size budget is real: the plan should prefer threading
  an existing value over new heavyweight abstractions, and add no dependencies.

## Scope

- Files you MAY modify: **NONE — design only, no source change.** Leave the
  working tree untouched (`git status --short` must be empty when you finish).
- You may read anything and run read-only commands (`grep`, `cargo tree`,
  `cargo check` is allowed only if you make no edits — prefer not to).

## Deliverable — an implementation plan

A written plan, in your final report, structured as:

1. **Geometry decision.** Precisely how a Wide-mode bordered window is laid out:
   border glyph placement, resulting body width/origin, gutter, cursor column,
   and how odd/even widths and any residual column are handled. Include a tiny
   ASCII sketch of an 80-column window's top border and one body row in Wide
   mode (how many box glyphs, where the body starts).
2. **Ordered steps.** 2–5 steps, each: which files/functions change, the shape
   of the change, how Narrow stays byte-identical, and the named test(s) that
   gate it. Order them so each compiles and passes on its own.
3. **Call-site inventory.** The exact list of `display_width`/`fit_text_to_width`/
   `status_text_for_width`/`wrap_line_segments` call sites and how each obtains
   the mode.
4. **Cross-crate verdict.** Whether `dun-cli` view sync is affected, with the
   evidence (file:line) you based it on.
5. **Risks / open questions.** Anything genuinely ambiguous that Claude should
   decide before implementation.

## Hard rules

- Do NOT edit any source file, commit, branch, push, or touch git. This is a
  design deliverable only.
- Do NOT invent global mutable state or thread-locals for the mode; if the only
  clean answer seems to require that, say so as an open question instead.
- Base every claim on real files (cite `path:line`); do not hand-wave the
  call-site inventory.

## Report format (your final message)

The five-part plan above. Be concrete enough that each step could become its own
implementation brief without further discovery. If a part is genuinely
undecidable from the code, say so under Risks rather than guessing.
