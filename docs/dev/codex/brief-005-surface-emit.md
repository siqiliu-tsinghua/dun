# Brief 005 — Surface diff emitter (Surface → SGR byte stream)

Implementation brief. Renderer-replacement slice 2: brief-002 landed the
`Surface` cell grid; this slice adds the pure emitter that turns a `Surface`
(or the diff between two) into terminal bytes. No event-loop integration, no
ratatui changes — that is a later slice.

## Goal

`crates/dun-ui` gains a private module `surface_emit` with two `pub(crate)`
functions that append terminal bytes to a caller-provided `Vec<u8>`:

- `emit_full(next: &Surface, out: &mut Vec<u8>)` — repaint every cell.
- `emit_diff(prev: &Surface, next: &Surface, out: &mut Vec<u8>)` — repaint
  only changed cells; identical surfaces append nothing; dimension mismatch
  falls back to exactly the `emit_full` byte stream.

The module is covered by golden byte-string unit tests and the workspace is
green.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-ui/src/surface.rs` — the `Surface`/`SurfaceCell` grid this
  emitter consumes (symbols carry appended zero-width chars;
  `wide_continuation` marks the second column of a wide glyph).
- `crates/dun-term/src/theme/style.rs`, `.../theme/color.rs` — `Style`,
  `StyleAttrs` (bold/underline/reverse), `TerminalColor`
  (Default/Ansi/Indexed). There is no RGB variant.
- `crates/dun-ui/src/lib.rs` — module list; `surface` is currently
  `#[allow(dead_code)]` pending integration; `surface_emit` gets the same
  treatment.

## Specification (byte-exact)

Escape sequences (write literal bytes, no helper deps):

- Cursor position: `ESC [ <row+1> ; <col+1> H` (CUP, 1-based).
- Style: one full-respecification SGR whenever the pen must change:
  `ESC [ 0 <;attrs> ; <fg> ; <bg> m`, always starting from `0` (reset) so
  attributes never leak. Attribute codes: bold `1`, underline `4`, reverse
  `7`, in that order, each only if set. Foreground: Default `39`,
  `Ansi(Black..=White)` `30..=37`, `Ansi(BrightBlack..=BrightWhite)`
  `90..=97`, `Indexed(n)` `38;5;n`. Background: Default `49`, `40..=47`,
  `100..=107`, `48;5;n`.
  Example: plain Indexed(117) on Default = `ESC[0;38;5;117;49m`.

Emission model:

- The emitter tracks pen (last SGR written) and cursor position internally
  per call. Each call is self-contained: it assumes nothing about the
  terminal's current SGR (the first cell written is always preceded by an
  SGR) and emits CUP before the first cell of any discontiguous run. After
  writing a cell the cursor advances by the cell's display width (1, or 2
  for a wide head). If the next run starts exactly at the current cursor
  position, no CUP is emitted.
- `emit_full`: for each row, CUP to `(row, 0)`, then write every cell in
  order, changing SGR only when the cell style differs from the pen. Do NOT
  emit clear-screen; the surface covers every cell. No trailing reset.
- `emit_diff`: a cell is "changed" when symbol, style, or
  `wide_continuation` differ between `prev` and `next`. Walk rows, group
  changed cells into runs, emit CUP + cells per run (SGR only on pen
  change).
- Wide glyphs: continuation cells are never written directly. A changed
  continuation cell re-emits its head instead (extend the run left to the
  head); writing the head covers both columns, then skip the continuation.
- Continuation cells inside `emit_full` row walks are skipped (the head's
  glyph already advanced the cursor past them).
- Zero-width/zero-height surfaces emit nothing.

## Scope

- Files you MAY modify:
  - `crates/dun-ui/src/surface_emit.rs` (new; implementation + its
    `#[cfg(test)] mod tests`);
  - `crates/dun-ui/src/lib.rs` (add the `#[allow(dead_code)] mod
    surface_emit;` declaration only);
  - `crates/dun-ui/src/surface.rs` (ONLY if a small `pub(crate)` read
    accessor is genuinely needed; `cell(x, y)`, `width()`, `height()`
    already exist — prefer them).
- Files/areas you MUST NOT touch (defaults for every brief):
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`,
    `docs/**`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock` (no new dependencies —
    the SGR encoder is hand-written byte pushes);
  - `vm-test/**`, `reference/**`, `hosts/**`, every other crate.

## Deliverable

- `surface_emit.rs` implementing the specification above.
- Unit tests in the same file, golden byte-string style (compare
  `String::from_utf8(out)` against literals with `\x1b`), covering at
  least:
  1. `identical_surfaces_emit_nothing`
  2. `full_repaint_golden_bytes` — small surface (e.g. 3x2, one styled
     cell), exact full byte stream asserted;
  3. `single_cell_change_emits_cup_sgr_symbol`
  4. `adjacent_same_style_changes_share_one_cup_and_sgr`
  5. `style_change_within_run_emits_sgr_without_cup`
  6. `wide_head_change_rewrites_head_and_skips_continuation`
  7. `continuation_change_reemits_head`
  8. `dimension_mismatch_falls_back_to_full_repaint` (bytes equal to
     `emit_full`)
  9. `color_and_attr_code_table` — Default/Ansi/Bright/Indexed fg+bg and
     bold/underline/reverse codes appear exactly as specified;
  10. `abutting_runs_skip_cup` — second run starting at the cursor emits no
      CUP.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force;
   if you think `unsafe` is unavoidable, STOP and report.
2. **The 1 MiB dual-platform size budget is real.** Minimal diff; no new
   dependencies; plain `Vec<u8>` pushes and `itoa`-free integer formatting
   via `write!`-less manual digits or `push_str(&n.to_string())` is fine —
   do not add formatting layers. Test-only code is exempt.
3. **All untrusted text goes through the sanitizer.** Not applicable here —
   `Surface` cells are already sanitized upstream; do not add new text
   ingestion paths.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** You are not touching
   dun-cli; if you find you need to, STOP and report.
5. **Tests are layered and colocated.** This module's tests live in the same
   file (`#[cfg(test)] mod tests`), matching `surface.rs`.
6. **Terminal-detection env is pinned in harnesses.** Not applicable — no
   process spawning in these tests.
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report — do not keep tuning.

## Verification (MANDATORY — you run it; iterate to green)

Run exactly these and paste results verbatim:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dun-ui --no-fail-fast
```

Loop: edit → test → fix → rerun, until green. Never claim a result without
the verbatim lines.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave all changes
  in the working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network. The
  only commands you run are file edits within Scope, `cargo`, and `python3`
  for parsing output.
- Minimal diff: no drive-by reformatting, renames, or comment changes
  outside the task.
- You MUST paste the real verbatim verification output. If a run did not
  reach green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command run, with the exact verbatim output lines
   (suite counts; note any environment-dependent skips).
3. The finding / verdict.
4. Stop-loss / open questions — where you stopped and why (empty if none).
