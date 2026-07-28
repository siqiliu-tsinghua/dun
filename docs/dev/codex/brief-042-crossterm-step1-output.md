# Brief 042 — crossterm replacement step 1: own the output escapes

Implementation brief. **Step 1 ("Brief 1") of the accepted plan produced for
brief 041** (the crossterm-replacement track; see
`docs/dev/codex/brief-041-crossterm-replacement-plan.md` for the fixed
constraints). This step replaces crossterm's OUTPUT byte emission only. Raw
mode, input parsing, the event loop, and both manifests stay on crossterm —
steps 2–5 are separate.

## Goal

dun emits its own VT output bytes through a new platform-neutral
`terminal/vt/output.rs`; crossterm's `execute!`/`queue!`/Command usage in
`lifecycle.rs`, `surface_backend.rs`, and `ambiguous_width.rs` is gone. One
intentional behavior delta (decided): mouse capture narrows from crossterm's
five modes (`1000/1002/1003/1015/1006`) to **`1000/1002/1006`** — dun only
handles SGR mouse events; all-motion (`1003`) and urxvt (`1015`) encodings are
dropped, and the pty_smoke assertion updates to match. Everything else is
byte-identical output; golden frames must not change.

## Exact change

1. **New `crates/dun-cli/src/terminal/vt/mod.rs` + `vt/output.rs`** —
   platform-neutral byte emission over `std::io::Write`, two call styles
   mirroring current semantics: *queue* (write, no flush) and *execute*
   (write + flush). Emissions:
   - enter/leave alternate screen: `ESC[?1049h` / `ESC[?1049l`
   - enable/disable bracketed paste: `ESC[?2004h` / `ESC[?2004l`
   - enable mouse: `ESC[?1000h` `ESC[?1002h` `ESC[?1006h` (that order);
     disable: `ESC[?1006l` `ESC[?1002l` `ESC[?1000l` (reverse order)
   - cursor hide/show: `ESC[?25l` / `ESC[?25h`
   - cursor move-to from 0-based `(column, row)` u16: `ESC[{row+1};{column+1}H`
     (checked 1-based conversion — row first, then column)
   - clear all: `ESC[2J`; move-to-column-0: `ESC[1G`; clear current line:
     `ESC[2K`
   Verify each byte string against crossterm's definitions before hardcoding
   (`~/.cargo/registry/src/*/crossterm-0.28.1/src/{terminal.rs,event.rs,cursor.rs}`) —
   the plan's cross-check found alternate screen at `terminal.rs:217`, paste at
   `event.rs:411`, cursor ops at `cursor.rs:54`, mouse at `event.rs:311`/`:346`.
2. **`terminal/lifecycle.rs`** — replace every `execute!` command usage (panic
   restore lines ~19–21, enter ~72–75, runtime mouse toggle ~91–93, suspend
   ~115–118, drop ~157–162) with execute-style `vt::output` calls, preserving
   the exact ordering and first-error semantics. Keep
   `enable_raw_mode`/`disable_raw_mode` (crossterm) untouched.
3. **`terminal/surface_backend.rs`** — replace the `queue!` usages (cursor
   hide/move/show ~30/33/46, clear-all ~39) with queue-style calls into
   `self.writer`; no implicit flush anywhere it doesn't flush today.
4. **`terminal/ambiguous_width.rs`** — `clear_probe` emits `ESC[1G` + `ESC[2K`
   via `vt::output` (queue then flush, as today); the mio read loop, probe
   bytes, and parsing stay untouched.
5. **`terminal/mod.rs`** — wire the `vt` module.
6. **`crates/dun-cli/tests/pty_smoke.rs`** (~line 390) — the mouse-mode
   assertions expect exactly `1000/1002/1006` enable and reverse-order disable;
   `1003`/`1015` must be asserted ABSENT.
7. **`docs/dev/terminal-compatibility-checks.md`** — update the mouse claim to:
   SGR mouse (`1006`) with `1000`/`1002` reporting under xterm-family/tmux/
   screen; keyboard remains primary. One tight paragraph, same section as the
   current mouse text.

## Scope

- Files you MAY modify: the seven items above (new `vt/` files; the four
  existing `terminal/` files; `tests/pty_smoke.rs` mouse assertions only;
  `docs/dev/terminal-compatibility-checks.md` mouse paragraph only) + colocated
  unit tests for `vt/output.rs`.
- Files/areas you MUST NOT touch: raw-mode/input/event-loop code
  (`event_loop.rs`, `input.rs`, `main.rs`), any `Cargo.toml`/`Cargo.lock`,
  other crates, other tests, other docs, `.git`, `i18n/**`, `hosts/**`,
  `vm-test/**`, `reference/**`.

If a change needs a file outside Scope, STOP and report.

## Deliverable

- The `vt::output` module + the four migrated files + the two allowed
  test/docs updates.
- Unit tests with **literal byte oracles**: hardcode the expected byte strings
  in the tests (do NOT reference the implementation's constants — the
  independent-oracle rule, AGENTS.md); cover every emission, the exact mouse
  enable/disable ordering, and queue-vs-execute flush behavior via a
  flush-counting writer.
- Prove load-bearing (run these yourself, then restore): (a) emit 0-based
  coordinates in move-to → a test fails; (b) swap row/column in move-to → a
  test fails; (c) reorder the mouse disable sequence → a test fails.

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`).
2. Narrow rendering is sacred: golden frames and every non-mouse PTY/tmux
   assertion must pass unchanged — this step is byte-identical output except
   the decided mouse narrowing.
3. `CSI row;col H` order and 1-based conversion are the classic traps.
4. Queue-style must not flush; lifecycle's execute-style must flush exactly
   where `execute!` does today (its first-error cleanup semantics depend on
   attempting every step).
5. Stop-loss: same failure twice, or an out-of-scope file needed → STOP,
   report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Note the pty_smoke and tmux suites' results explicitly (tmux/expect needed —
say so if absent rather than reporting green). Claude runs the dual-platform
size gate and release smoke at the gate.

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why.
2. Verification — each command's verbatim output (suite counts; PTY/tmux
   noted).
3. Mutation evidence — the three load-bearing runs, verbatim.
4. Verdict.
5. Stop-loss / open questions (empty if none).
