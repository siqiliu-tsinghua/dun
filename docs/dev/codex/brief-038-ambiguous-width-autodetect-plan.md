# Brief 038 — Design plan for ambiguous-width auto-detection (design only)

**Diagnostic/design brief. NO source change.** Produce a step-by-step
implementation plan; Claude evaluates it, then dispatches the steps as
implementation briefs and gates each. (Plan-first workflow — see CLAUDE.md.)

## Goal

Produce a concrete, step-by-step *plan* (not code) for **stage B**: making `dun`
auto-detect at startup whether the terminal renders Unicode East Asian
Ambiguous-width glyphs as 1 or 2 columns, and set `TerminalProfile.ambiguous_width`
accordingly — so a user on a wide terminal (Solaris tmux, CJK-configured
terminals) gets correct rendering **without** setting `terminal.ambiguous-width`.
The explicit config option must still win when set (it is the override). When
detection is impossible or times out, fall back to Narrow (today's default).

## What already exists (stage A, do not re-plan)

- `AmbiguousWidth {Narrow, Wide}` on `TerminalProfile`; `dun_term::char_width`/
  `str_width`; the whole render layer honors the mode; `terminal.ambiguous-width
  = narrow|wide` sets it via config. `TerminalProfile::from_capabilities`
  currently hardcodes `Narrow`.
- The editor enters raw mode via `crossterm` in `dun-cli/src/terminal/`
  (`TerminalGuard::enter`), and reads input through `crossterm::event`. The size
  is read with `crossterm::terminal::size()`.

## The proven blueprint (study it)

Microsoft's `edit` already does exactly this — read
`reference/msedit/crates/edit/src/bin/edit/main.rs` around the startup query
(~lines 585–680): it writes `\r…\x1b[6n` (an ambiguous glyph `…` then a
DSR/cursor-position request), plus `\x1b[c` (a DA1 primary-device-attributes
query) as a **sentinel** — because not all terminals answer CPR/OSC but all
answer DA1, so DA1's response marks the end of reading, with a read timeout as a
backstop. It parses the CPR (`R`) response and sets `ambiguous_width =
params[1] - 1` (the column the cursor advanced to minus 1 = the glyph's width).
On width 2 it re-flows. Our `cursor_x`-based Solaris diagnosis is the same idea.
Note: msedit uses a `static mut` global; **dun is `#![forbid(unsafe_code)]`**, so
the plan must set the mode on the `TerminalProfile`/`UiShell` value, not a global.

## The plan must address each

1. **Where & when.** Detection must run after raw mode is entered (so the reply
   isn't echoed) and before the first frame, on the real terminal. Which
   function (`TerminalGuard::enter` neighborhood? a new step in `run_tui`?), and
   how the detected `AmbiguousWidth` reaches the `TerminalProfile` that
   `AppState`/`UiShell` already hold (they're built before raw mode today —
   sequencing matters).
2. **The probe.** The exact bytes to write (an ambiguous glyph + DSR + DA1
   sentinel), how to read/parse the responses (does `crossterm` expose
   `cursor::position()` / DA1, or must we read raw and parse?), the timeout, and
   cleanup (the probe output must not be left on screen — where/how to erase or
   over-draw). Cite crossterm APIs by name.
3. **Override precedence.** Config `terminal.ambiguous-width` (when the user set
   it) must win over detection; detection only fills the unset case. State how
   `TerminalOverrides`/detection compose (detection result becomes the "detected"
   profile that overrides apply on top of).
4. **Test-environment behavior.** How does the probe behave under the tmux/pty
   test harnesses and in CI? tmux answers CPR; a bare pty may not — the DA1
   sentinel + timeout must keep startup fast and deterministic. Decide whether
   tests that spawn the editor should pin the mode (e.g. via config) to stay
   deterministic, and whether the probe should be skipped when stdin/stdout is
   not a tty.
5. **THE SOLARIS QUESTION (answer it with evidence).** Today Solaris runs
   725/6: four `tmux_grid` and two `tmux_logfilter` failures, all the
   ambiguous-width overflow under the *default Narrow* mode. After auto-detect,
   dun on Solaris tmux will pick Wide and render a 40-glyph / 80-cell border.
   **Will those six tests then pass, or do their assertions need to become
   mode-aware?** Read the actual capture/assert logic — `crates/dun-cli/tests/
   tmux_grid.rs`, `crates/dun-cli/tests/support/terminal_grid.rs`
   (`parse_terminal_grid` / `find_border_box`), and `crates/dun-cli/tests/
   tmux_logfilter.rs` — and determine, with `path:line` evidence, whether a
   Wide-rendered border/body satisfies them as-is (e.g. does `parse_terminal_grid`
   count a 2-cell glyph as width 2, so the border box is still 80 wide?) or
   whether specific assertions hardcode Narrow character counts/positions and
   must be updated. If updates are needed, say exactly which and how (and whether
   they can be made platform-neutral, i.e. assert the same for a Wide terminal
   and a Narrow one). This is the deliverable Claude most needs.

## Context pointers

- Read `AGENTS.md`, `docs/dev/solaris-vm.md`, and the stage-A briefs 032–037.
- Detection touches `dun-cli` startup; the mode lives on `dun-term`'s
  `TerminalProfile`; config override is in `dun-config`.
- 1 MiB dual-platform budget is real; the probe should add no dependency and
  little code.

## Scope

- Files you MAY modify: **NONE — design only.** Leave the tree clean
  (`git status --short` empty when done). Read anything; run read-only commands
  (`grep`, `sed`); do not `cargo build`/edit.

## Deliverable — an implementation plan

1. **Detection design** — where it runs, the exact probe bytes, parsing,
   timeout, cleanup, and how the result reaches `TerminalProfile` before the
   first frame; override precedence with config.
2. **Ordered steps** (2–4), each: files/functions, the change shape, how Narrow
   default and the config override stay intact, the gate test(s).
3. **Test-harness plan** — probe behavior under tmux/pty/CI, tty check, and
   whether/which spawn-the-editor tests pin the mode.
4. **The Solaris verdict** — will the six tmux tests pass after auto-detect, or
   which assertions need mode-aware updates, with `path:line` evidence.
5. **Risks / open questions** for Claude to decide before implementation.

## Hard rules

- Do NOT edit any source file, commit, branch, push, or touch git.
- Do NOT invent global mutable state or thread-locals (dun forbids unsafe); if
  the only clean answer seems to need that, say so as an open question.
- Base every claim on real files (`path:line`); do not hand-wave the Solaris
  verdict or the crossterm API question.

## Report format (final message)

The five-part plan above, concrete enough that each step could become its own
implementation brief without further discovery.
