# Brief 016 — PTY test: a panic must not leave the terminal wrecked

Implementation brief. `install_panic_terminal_restore` is an A-level safety
invariant (AGENTS.md): without it, any panic leaves the user on the alternate
screen in raw mode. For a remote editor this is the worst failure mode there
is — you SSH in, it panics, and your terminal is unusable until you `reset`.

It has **no test**. `crates/dun-cli/src/terminal/lifecycle.rs` sits at 53.7%
coverage, and CLAUDE.md records the hook as "verified working" — by hand.

## Goal

A PTY test that panics the editor *while it owns the terminal* and asserts the
terminal was handed back: alternate screen left, bracketed paste off, mouse
capture off, and the panic message actually reaches the user.

## Fidelity boundary — state this, do not paper over it

The release profile sets `panic = "abort"`, so `TerminalGuard::drop` never runs
on a panic and the hook is the only thing that restores the terminal. The test
binary (`CARGO_BIN_EXE_dun`) is a **debug** build, where panics unwind. Panic
hooks run under both strategies, so the hook's restore logic — the thing under
test — is exercised faithfully; what the test does *not* cover is the abort path
itself. Say so in a comment on the test. Do not claim more than it proves.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-cli/src/terminal/lifecycle.rs`:
  - `install_panic_terminal_restore()` — the hook. It emits, in order:
    `DisableMouseCapture`, `DisableBracketedPaste`, `LeaveAlternateScreen`,
    flush, `disable_raw_mode()`, then chains to the default hook (which prints
    the panic message to stderr).
  - `TerminalGuard::enter(mouse_enabled)` — `enable_raw_mode` +
    `EnterAlternateScreen` + `EnableBracketedPaste` (+ `EnableMouseCapture`).
- The observable escape sequences in the PTY output:
  - leave alternate screen: `\x1b[?1049l`
  - bracketed paste off: `\x1b[?2004l`
  - mouse capture off: crossterm emits the `?1006l`/`?1015l`/`?1002l`/`?1000l`
    family. Assert on whichever crossterm 0.28 actually writes — check by
    running, do not guess.
  - raw mode is a termios call and is **not** observable in the output stream.
    Do not invent an assertion for it.
- `crates/dun-cli/tests/pty_smoke.rs` — the existing suite (7 tests) and its
  shape. `crates/dun-cli/tests/support/pty.rs` — `run_dun_in_pty`,
  `pty_test_guard()`, `command_on_path("expect")`, `assert_output_contains`.
  The harness skips cleanly when `expect(1)` is absent; keep that behaviour.
- `crates/dun-cli/src/terminal/event_loop.rs` — `run_event_loop`, which is
  entered *after* `TerminalGuard::enter`, and draws before polling for events.
- `crates/dun-cli/src/main.rs` — where the hook is installed and the guard is
  entered.

## Specification

### 1. A debug-only panic trigger

The test needs the editor to panic *after* it has taken the terminal. Add a
trigger that cannot exist in a shipped binary:

```rust
// In run_event_loop, immediately after the first successful `backend.draw(...)`
// so the terminal is fully in raw mode on the alternate screen and the startup
// frame is on screen for the harness to sync on.
#[cfg(debug_assertions)]
if std::env::var_os("DUN_TEST_PANIC").is_some() {
    panic!("DUN_TEST_PANIC");
}
```

- It MUST be `#[cfg(debug_assertions)]`. The release profile has
  `debug-assertions = false`, so this compiles out entirely: no bytes, no env
  lookup, and no hidden panic trigger in the editor people actually run.
- Fire it after the first draw, not before, so the PTY harness can still sync on
  the startup marker.
- Do not add any other runtime behaviour.

### 2. The test

Add to `crates/dun-cli/tests/pty_smoke.rs`, e.g.
`pty_smoke_restores_the_terminal_when_it_panics`:

- Skip cleanly if `expect(1)` is not on PATH, like the others.
- Launch `dun` under the PTY with `DUN_TEST_PANIC=1` in the environment. You
  will need to extend the harness to pass env vars — `run_dun_in_pty` currently
  does not. Add a variant or an extra parameter in
  `crates/dun-cli/tests/support/pty.rs`; keep the existing call sites working.
- Assert on the captured output that the terminal was handed back:
  - contains `[?1049l` (left the alternate screen);
  - contains `[?2004l` (bracketed paste disabled);
  - contains the panic message (`DUN_TEST_PANIC`) — a panic the user cannot see
    is its own bug.
- Add a second case with mouse capture enabled (there is already a
  `pty_smoke_quits_cleanly_with_mouse_capture_enabled` for the config shape to
  copy) asserting the mouse-disable sequence is emitted too.
- The process will not exit 0. Assert the failure is a panic, not a hang: the
  run must terminate on its own.

### 3. Order matters

The hook restores in the order mouse → paste → alt-screen → raw. Assert the
sequences are present; asserting their relative order is a bonus if it is cheap
and stable. Do not make the test brittle on exact byte layout.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/terminal/event_loop.rs` (ONLY the `cfg(debug_assertions)`
    panic trigger — nothing else);
  - `crates/dun-cli/tests/pty_smoke.rs`;
  - `crates/dun-cli/tests/support/pty.rs` (env-var support for the harness).
- Files/areas you MUST NOT touch:
  - `crates/dun-cli/src/terminal/lifecycle.rs` — the hook is correct; this brief
    tests it, it does not rewrite it. If you believe it is wrong, STOP and
    report rather than "fixing" it.
  - every other crate;
  - `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `PROGRESS.md`, `TODO.md`, `docs/**`,
    `README.md`;
  - `.git`, git config, any `Cargo.toml`, `Cargo.lock` (no new dependencies).
  - `vm-test/**`, `reference/**`, `hosts/**`.

## Deliverable

- The `cfg(debug_assertions)` panic trigger.
- PTY env-var support in the harness, existing call sites unchanged.
- The panic-restore test(s), with a comment stating the debug/abort fidelity
  boundary.
- Confirmation in your report that `DUN_TEST_PANIC` does **not** appear in a
  release build: run `cargo build --release -p dun-cli` and
  `strings target/release/dun | grep -c DUN_TEST_PANIC` (expect `0`).

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **The 1 MiB dual-platform size budget is real.** The trigger must be
   `cfg(debug_assertions)` so it contributes zero bytes. Claude gates size.
3. **Terminal restore is an A-level invariant.** You are testing it, not
   redesigning it. Do not touch `lifecycle.rs`.
4. **No hidden backdoors in the shipped binary.** No env var, no CLI flag, no
   key sequence that panics in a release build.
5. **PTY tests must skip, not fail, without `expect(1)`.** Follow the existing
   pattern exactly.
6. **Do not leave the developer's terminal wrecked** if the test misbehaves —
   the PTY is a child pty, but be careful with the `pty_test_guard()` lock.
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --release -p dun-cli && strings target/release/dun | grep -c DUN_TEST_PANIC
```

The last command must print `0`. Loop: edit → test → fix → rerun, until green.
Paste verbatim output.

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
2. Verification — each command with verbatim output lines, including the
   `strings | grep -c` result.
3. The finding / verdict — in particular, the exact escape sequences crossterm
   emitted, so the assertions are grounded in what actually happened rather than
   what the docs claim.
4. Stop-loss / open questions (empty if none).
