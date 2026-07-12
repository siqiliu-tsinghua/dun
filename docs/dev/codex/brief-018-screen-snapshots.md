# Brief 018 — Screen snapshots: a cheap oracle for what the editor draws

Implementation brief. Today's bug hunt found two defects that a rendered-screen
golden would have caught the moment it was introduced:

- the file dialogs painted their region with `style_run`, which only restyles
  cells, so the editor text underneath **bled through** the modal wherever the
  dialog's own text did not reach;
- status messages were written into the frame and then unconditionally
  overwritten, so every command's feedback was **invisible**.

Neither was caught by any test, because asserting on a screen means writing down
what the screen should look like — which nobody does by hand for 24 rows. A
snapshot does it for you: a human reviews the golden once, and thereafter any
unintended change is a diff.

## Deliberately NOT using `insta`

This repo has **zero** dev-dependencies and two runtime ones. It hand-rolled
JSON rather than take serde, and dropped ratatui. The parts of `insta` we would
actually use — render, compare against a checked-in golden, regenerate on
demand — are about sixty lines. `cargo insta review` is nice, but a golden file
in git is reviewed by `git diff`, and that review then lives in the commit.

So: hand-rolled, no new dependencies. Do not add `insta`, `similar`, or
anything else.

## Goal

1. A snapshot renderer in `dun-ui` that turns a frame into a deterministic,
   diff-friendly text form (glyphs **and** styles).
2. A golden-file harness in `dun-cli`'s tests: compare against a checked-in
   file, and rewrite every golden when `UPDATE_SNAPSHOTS=1` is set.
3. Goldens for the screens that matter, including the two bugs above.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-ui/src/surface.rs` — `Surface`, `SurfaceCell`, `Surface::cell`
  and `row_text` are `pub(crate)`; `crates/dun-ui/src/render/surface_frame.rs`
  has `render_ui_frame_to_surface`.
- `crates/dun-ui/src/tests/support.rs::render_frame_text` — the existing
  text-only renderer; the new public function supersedes it for goldens.
- `crates/dun-term` — `Style { fg, bg, attrs }`, `TerminalColor { Default,
  Ansi(AnsiColor), Indexed(u8) }`, `StyleAttrs { bold, underline, reverse }`.
- **`dun-cli` is a binary crate.** Integration tests under
  `crates/dun-cli/tests/` cannot import `AppState`; that is why every
  `AppState` test lives in `crates/dun-cli/src/tests/`. The snapshot tests must
  live there too, and the goldens next to them.
- `crates/dun-cli/src/tests/support.rs` — the shared test prelude,
  `temp_file_path`, `send_text`.
- `crates/dun-cli/src/terminal/event_loop.rs` — shows how a frame is assembled:
  `shell.frame_for_workspace_with_menu_selection(...)`, then `status.left`,
  `status.right`, `status.plugin`, `overlay` are filled in. Snapshots must
  build the frame **the same way**, or they will not show what users see.

## Specification

### 1. `dun-ui`: the snapshot renderer

Add one public function (and whatever private helpers it needs):

```rust
/// Render a frame exactly as the terminal backend would, and format it for a
/// golden file: the glyph grid, a per-cell style map, and a legend.
pub fn frame_snapshot(shell: &UiShell, frame: &UiFrame, width: u16, height: u16) -> String;
```

Output format — stable, compact, and readable in a diff:

```text
size: 80x24  theme: dun  colors: Color256
cursor: 3,2

text:
 0|  File  Edit  View  Help
 1|┌─ ◆ sample.txt ──────────────────────────┐
 …
23|[Plain Text] [LF] [UTF-8] …

style:
 0|AAAABAAAAABAAAA…
 1|CDDDDDDDDDDDDDDC
 …

legend:
A = 255/24
B = 223/24 b
C = 180/234 b
D = 187/234
```

- The style map assigns a legend key per **distinct** `Style`, in order of first
  appearance scanning rows top-to-bottom, columns left-to-right, so the mapping
  is deterministic. Use `A`–`Z`, then `a`–`z`, then `0`–`9`; if a frame somehow
  needs more than 62, fall back to a two-character key rather than panicking.
- Colour spelling: `d` for `Default`, the number for `Indexed(n)`, the
  lowercase ANSI name for `Ansi(_)` (`red`, `bright_blue`, …). Attributes are a
  suffix of `b`/`u`/`r` (bold/underline/reverse), omitted when none.
- `cursor:` is the frame's cursor position, or `none`.
- Wide (double-width) cells: render the glyph in the first column and a space in
  its continuation column, matching what `row_text` already does. Do not panic
  on them.
- The function must not allocate a terminal or touch stdout.

Size: this is a `pub fn` that the binary never calls, so LTO should drop it
entirely. Claude gates the byte delta; do not add anything to a render path.

### 2. `dun-cli`: the golden harness

In `crates/dun-cli/src/tests/`, add a module (e.g. `snapshots.rs`) with:

```rust
/// Compare `actual` against `src/tests/snapshots/<name>.txt`.
/// With `UPDATE_SNAPSHOTS=1` set, rewrite the file instead of asserting.
fn assert_snapshot(name: &str, actual: &str);
```

- Resolve the directory from `env!("CARGO_MANIFEST_DIR")`, not the process CWD.
- A missing golden with `UPDATE_SNAPSHOTS` unset is a **failure**, with a
  message telling the reader to rerun with `UPDATE_SNAPSHOTS=1` and review the
  diff. Never create a golden silently — a golden nobody looked at is worth
  nothing.
- On mismatch, print a readable diff (the first differing line and its
  neighbours is enough; do not pull in a diff crate).
- Trailing whitespace on a row is significant (a modal that fails to blank its
  region shows up as leftover glyphs) — do NOT trim it away. Do end every
  golden with a single newline.

Add a helper that drives `AppState` and produces the snapshot text, assembling
the frame exactly as `run_event_loop` does (status left/right, plugin
indicator, overlay).

### 3. Determinism — the traps

A golden that changes between machines is worse than no golden.

- **Absolute paths.** The Open dialog prints `Look in: /Users/…`, the status bar
  prints file names, `Save As` prints a full path. Every snapshot that can
  contain one must redact it: give the helper a list of `(needle, replacement)`
  pairs, applied to the *text* before formatting, and use `<TMP>` /
  `<FIXTURE>` placeholders. A golden containing a home directory is a bug.
- **Directory listings.** The Open dialog lists the working directory, whose
  contents differ per machine and per run. Point the dialog at a temp directory
  the test creates with a fixed set of entries.
- **Time, pids, sizes.** None should appear; if one does, redact it.
- Fixed terminal sizes per snapshot. Fixed fixture text.
- The plugin indicator is off by default and the startup status message is
  cleared — leave both alone.

### 4. The goldens

At minimum, and each with a comment naming what it protects:

1. `startup_80x24` — the resting screen.
2. `open_dialog` — **the modal must not have editor text bleeding through it.**
   Lay real buffer text under it, as the bug did.
3. `save_as_dialog`.
4. `confirm_unsaved` — the unsaved-changes modal.
5. `buffer_switcher`.
6. `find_prompt`, `go_to_line_prompt`.
7. `file_menu_open` — the File dropdown over buffer text (same bleed class).
8. `split_two_panes` — **the focused pane is warm, the unfocused one recedes
   into haze**; this is the only test that would catch that split regressing.
9. `help_screen`, `search_results`, `status_history`.
10. `status_after_failed_command` — e.g. Undo on an empty stack. **The status
    line must show "Nothing to undo"**, not the buffer readout. This is the
    invisible-feedback bug.
11. One per theme at startup: `theme_dun`, `theme_msedit`, `theme_turbo`,
    `theme_dark`.
12. `fallback_16_color` and `fallback_mono` — forced via `TerminalOverrides`.
13. `narrow_40x10` — the gutter is dropped and titles clip.

## Scope

- Files you MAY modify:
  - `crates/dun-ui/src/` — the snapshot renderer and its module wiring
    (`lib.rs` re-export). Do NOT change any existing render behaviour.
  - `crates/dun-cli/src/tests/**`, including new `snapshots.rs` and the golden
    directory `crates/dun-cli/src/tests/snapshots/`.
- Files/areas you MUST NOT touch:
  - any runtime behaviour in `dun-cli`, `dun-core`, `dun-config`, `dun-term`;
  - `crates/dun-ui/src/render/**` — snapshots *observe* the renderer, they do
    not change it. If a golden looks wrong, that is a finding to report, not a
    render bug to fix in this brief;
  - `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `PROGRESS.md`, `TODO.md`, `docs/**`,
    `README.md`;
  - `.git`, git config, any `Cargo.toml`, `Cargo.lock` — **no new
    dependencies**, and none are needed;
  - `vm-test/**`, `reference/**`, `hosts/**`.

## Deliverable

- `dun_ui::frame_snapshot`.
- The golden harness with `UPDATE_SNAPSHOTS=1`.
- The goldens from §4, each generated, **read by you**, and sanity-checked
  before being committed. A golden you did not look at is a golden that locks in
  a bug.
- In your report: state explicitly that you inspected each golden, and flag
  anything in them that looks wrong. That is a finding, not something to fix.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **The 1 MiB dual-platform size budget is real.** `frame_snapshot` must be
   dead code in the shipped binary. No new dependencies. Claude gates size.
3. **No new dependencies. Not `insta`. Not a diff crate.** See the section above
   for why; this is a decision, not an oversight.
4. **A golden must be reproducible on another machine.** Absolute paths,
   directory listings, and anything time- or pid-derived are the traps.
5. **Do not trim trailing whitespace out of a row.** Leftover glyphs at the end
   of a modal row are exactly the bleed-through bug; trimming would hide it.
6. **`dun-cli` is a binary crate** — snapshot tests go in `src/tests/`, not
   `tests/`.
7. **Do not "fix" the renderer.** If a golden reveals something ugly, report it.
8. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Then prove the harness actually fails on a change, rather than rubber-stamping:

```
# Perturb one golden by hand, confirm the test fails and names the row.
# Restore it, confirm green.
```

Paste verbatim output for all of it.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave changes in the
  working tree; Claude gates and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and report.
- Full machine access, but touch NOTHING outside this repo, no network. Only
  file edits within Scope, `cargo`, and `python3` for parsing output.
- Minimal diff; no drive-by reformatting or renames.
- Paste real verbatim verification output; if not green, say so.

## Report format (your final message)

1. What changed — per file, line ranges, one-line why.
2. Verification — each command with verbatim output, including the
   perturb-a-golden check.
3. The finding — confirmation that you read every golden, plus anything in them
   that looks wrong.
4. Stop-loss / open questions (empty if none).
