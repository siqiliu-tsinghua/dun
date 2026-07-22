# Brief 040 — Auto-detect step 2: mode-aware tmux/PTY test harness

Implementation brief. **Step 2 of the plan in
`docs/dev/codex/brief-038-ambiguous-width-autodetect-plan.md`** (read its
"Solaris verdict" table). Step 1 (the detector) is done; step 3 (cross-platform
+ measurement) is separate.

## Goal

Make the terminal-test harness independent of `dun`'s own width decision and
aware of the ambiguous-width mode, so the suite validates `dun` under BOTH
readings: the PTY responder answers the startup probe (no more 500 ms fallback);
tmux independently measures its own ambiguous width and the grid parser honors
it; and the four `tmux_grid` assertions become **platform-neutral** (correct on
a Narrow terminal AND a Wide one). **This is test-only** — no product code
changes. After it, macOS/Linux (Narrow tmux) stays green; Solaris (Wide tmux)
goes green, verified on the VM in step 3. The two `tmux_logfilter` tests need no
assertion change (they already assert only visible content; Wide rendering stops
the tiled-window truncation).

## Exact change (implement the plan's step 2 + the Solaris verdict table)

1. **`tests/support/terminal_grid.rs` — mode-aware parser.** Give
   `parse_terminal_grid` and `TerminalGrid` an `AmbiguousWidth`; `GridParser::
   put_char` advances by the glyph's width under that mode instead of the
   hardcoded Narrow (~lines 385–411). In border detection:
   - a border's physical width is `right_head - left + glyph_width`;
   - validate right-side glyph heads at `rect.right() + 1 - glyph_width`
     (~lines 193–203, which currently treat the right head as the last physical
     cell);
   - walk bottom-border heads by `glyph_width` (~lines 213–244, which currently
     require every physical bottom cell to hold a horizontal glyph);
   - do NOT require a horizontal glyph immediately after the top-left corner —
     the window title overwrites that spot in Wide mode.
   Add hardcoded Narrow AND Wide box-parser unit fixtures proving both.

2. **`tests/support/tmux.rs` — tmux measures its own width.** Add an independent
   temporary tmux probe session that measures the pane's ambiguous width itself
   (e.g. `printf '\r──────────'` then read `#{cursor_x}`: 10 ⇒ Narrow, 20 ⇒
   Wide), store the resulting `AmbiguousWidth` on `TmuxSession`, clean the probe
   session up reliably, and pass the mode through `capture_grid`. Do **NOT**
   source the oracle from `dun`, and do **NOT** pin `terminal.ambiguous-width` on
   the editor session — real startup auto-detection must be exercised. The
   existing pinned capability env (TERM/LANG/…) stays.

3. **`tests/support/pty.rs` — PTY answers the probe.** The expect harness must
   recognize the exact probe (`\r─\x1b[6n\x1b[c`) and reply. Add a default Narrow
   responder (`ESC[1;2R` then `ESC[?1;2c`), a dedicated Wide case (`ESC[1;3R …`),
   and one no-response case that reaches the 500 ms Narrow fallback. This
   exercises detection rather than masking it.

4. **`tests/tmux_grid.rs` — platform-neutral assertions** (per the Solaris
   verdict table):
   - baseline: assert the top row **starts with `┌`** and **contains `◆
     Untitled`** (drop the exact `┌─ ◆ Untitled` substring), and let the
     mode-aware grid assert a **physical 80-cell** border box;
   - 100×30: measure display width with the tmux-measured mode; keep the
     physical `GridRect.width == 100` assertion (not `.chars().count() == 100`);
   - cursor: expect cursor `x=3` / gutter `x=1` in Narrow and `x=5` / `x=2` in
     Wide (they follow the border+gutter geometry);
   - split: keep the raw corner/codepoint assertions; the fixed parser must keep
     both physical split rectangles valid, the second still ending at cell 79.
   Keep the existing ASCII-fallback test as an explicit encoding test.

5. **`tests/terminal_grid.rs`** — update its parser calls/fixtures for the new
   mode parameter.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/tests/support/terminal_grid.rs`,
    `crates/dun-cli/tests/support/tmux.rs`,
    `crates/dun-cli/tests/support/pty.rs`
  - `crates/dun-cli/tests/terminal_grid.rs`, `crates/dun-cli/tests/tmux_grid.rs`
- Files/areas you MUST NOT touch:
  - any product code (`crates/dun-cli/src/**`, `crates/dun-ui/**`,
    `crates/dun-core/**`, `crates/dun-term/**`, `crates/dun-config/**`) — the
    detector is done; this step only adapts the harness
  - `crates/dun-cli/tests/tmux_logfilter.rs` (its assertions are unchanged per
    the plan; if it needs `capture_grid` mode plumbing, note it, but do not
    change its assertions)
  - `crates/dun-cli/tests/msedit_diff.rs` / `msedit_reference.rs` beyond what a
    parser-signature change forces (keep behavior)
  - any `Cargo.toml`/`Cargo.lock`, `.git`, `docs/**`, `i18n/**`, `vm-test/**`,
    `reference/**`, `hosts/**`

If a change needs a file outside Scope, STOP and report.

## Deliverable

- The mode-aware parser, the tmux self-measured width, the PTY probe responder,
  and the platform-neutral `tmux_grid` assertions.
- New parser fixtures: a Narrow box and a Wide box, each asserting the physical
  rectangle and glyph heads.
- Prove load-bearing: mutate the parser back to a hardcoded Narrow advance, and
  separately move the right-glyph-head check to `rect.right()`, and confirm the
  Wide fixture fails **both** times; then restore.

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`).
2. Test-only: nothing here ships, so no size impact; but do NOT reach into
   product code to get the mode — the harness must measure the terminal itself.
3. On macOS/Linux the tmux pane is Narrow, so the tmux_grid tests must still
   pass with Narrow layout after your changes (the Wide path is exercised on the
   Solaris VM in step 3 and by the parser fixtures here).
4. tmux tests need tmux; PTY tests need `expect`/a pty — they skip cleanly when
   absent (say so rather than reporting green).
5. Stop-loss: same failure twice, or an out-of-scope file needed → STOP, report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Confirm the PTY suite is no longer paying the 500 ms fallback (the responder now
answers). Note tmux/expect skips if the tools are absent.

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why.
2. Verification — each command's verbatim output (suite counts; PTY timing; note
   skips).
3. Verdict.
4. Stop-loss / open questions (empty if none).
