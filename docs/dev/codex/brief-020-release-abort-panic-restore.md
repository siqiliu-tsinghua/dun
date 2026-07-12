# Brief 020 — Cover the panic path that actually ships

Implementation brief. Brief 016 tested the panic hook, but only in a debug
build, and its own commit says so:

> Still not covered: the release abort path, where the hook is not merely first
> but the only restorer.

That is the path users get. `[profile.release] panic = "abort"`, so
`TerminalGuard::drop` never runs on a panic and the hook is the *only* thing
standing between a crash and a wrecked terminal. `lifecycle.rs` is at 54.9%
coverage on an A-level safety invariant (AGENTS.md).

Measured, not assumed: with the trigger's cfg removed and the binary built
`--release`, a panic exits with **status 6 (SIGABRT)**; the debug binary exits
**101** (the ordinary unwind panic). The abort really happens.

## Why this is stronger than the debug test, not just different

In debug the panic unwinds, so `TerminalGuard::drop` restores the terminal by
itself — which is why brief 016 had to assert on the *order* of the escape
sequences relative to the panic message to prove the hook ran at all. Under
abort there is no `Drop`. If the hook does not run, the terminal is simply never
restored. So "the restore sequences are present" becomes a load-bearing
assertion on its own, and the mutant (remove the hook) fails loudly.

## Goal

1. Make the panic trigger available to a **release** test build without letting
   it near a shipped binary.
2. A PTY test that panics a release-profile `dun` and asserts the terminal was
   handed back — proving the hook alone did it.
3. Report which of `lifecycle.rs`'s remaining branches are still uncovered.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-cli/src/terminal/event_loop.rs` — the existing trigger:
  ```rust
  #[cfg(debug_assertions)]
  if std::env::var_os("DUN_TEST_PANIC").is_some() {
      panic!("DUN_TEST_PANIC");
  }
  ```
- `crates/dun-cli/src/terminal/lifecycle.rs` — `install_panic_terminal_restore`
  (the hook), `TerminalGuard::{enter, set_mouse_enabled, suspend, resume}` and
  its `Drop`. Do NOT change any of it.
- `crates/dun-cli/tests/pty_smoke.rs` — the existing suite, including
  `pty_smoke_restores_the_terminal_when_it_panics` and its comment explaining
  the debug/abort boundary. `crates/dun-cli/tests/support/pty.rs` —
  `run_dun_in_pty_with_env`, `pty_test_guard`, `command_on_path("expect")`.
- `Cargo.toml` (workspace root) — `[profile.release]` sets `panic = "abort"`.
- `scripts/release-build.sh` — the shipped build. It must never see the feature.

## Specification

### 1. Widen the trigger's gate

```rust
#[cfg(any(debug_assertions, feature = "test-panic-hook"))]
if std::env::var_os("DUN_TEST_PANIC").is_some() {
    panic!("DUN_TEST_PANIC");
}
```

Add to `crates/dun-cli/Cargo.toml`:

```toml
[features]
test-panic-hook = []
```

It must **not** be a default feature. This is the one place in this brief where
touching a `Cargo.toml` is allowed, and only for this.

The three builds must then behave like this, and you must verify all three:

| build | trigger present? |
| --- | --- |
| `cargo test` (debug) | yes — `debug_assertions` |
| `cargo test --release --features test-panic-hook` | yes — the feature |
| `scripts/release-build.sh` (shipped) | **no** |

### 2. The release-abort test

Add to `crates/dun-cli/tests/pty_smoke.rs`, gated so it only exists in the
feature build:

```rust
#[cfg(feature = "test-panic-hook")]
#[test]
fn pty_smoke_restores_the_terminal_when_a_release_build_aborts() -> io::Result<()> { … }
```

- Skip cleanly without `expect(1)`, like every other test here.
- Launch with `DUN_TEST_PANIC=1`.
- Assert the terminal was handed back: `[?1049l` (left the alternate screen)
  and `[?2004l` (bracketed paste off) are in the output, and the panic message
  reached the user.
- Assert the process **aborted** rather than unwound. On Unix,
  `std::os::unix::process::ExitStatusExt::signal()` should be `Some(6)`
  (SIGABRT). Check what actually comes back before you write the assertion —
  measure, do not guess.
- **No ordering assertion is needed here, and adding one would obscure the
  point.** Under abort there is no `Drop` to muddy the water: the sequences are
  present only if the hook ran. Say so in a comment.

Leave the existing debug test exactly as it is. The two together cover both
panic strategies, and the debug one's ordering assertion is still the only thing
that makes it load-bearing.

### 3. Prove it is load-bearing

Comment out the `install_panic_terminal_restore()` call in `main.rs`, run the
release-abort test, and confirm it fails **because the terminal was never
restored** (not merely out of order). Restore the call and confirm green. Paste
both.

### 4. Report the remaining lifecycle gaps

`lifecycle.rs` will still not be at 100%. Run
`cargo llvm-cov --workspace --summary-only` (it is installed) and, in your
report, name which functions/branches in `lifecycle.rs` remain uncovered — the
`TerminalGuard::enter` rollback paths and the `suspend`/`resume` error arms are
the likely ones. Do NOT try to cover them in this brief; they need injected I/O
failures and are their own piece of work. Just say what is left.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/terminal/event_loop.rs` (the trigger's cfg only);
  - `crates/dun-cli/Cargo.toml` (the `[features]` table only);
  - `crates/dun-cli/tests/pty_smoke.rs`;
  - `crates/dun-cli/tests/support/pty.rs` (only if the harness needs to expose
    the exit signal).
- Files/areas you MUST NOT touch:
  - `crates/dun-cli/src/terminal/lifecycle.rs` — you are testing it, not
    rewriting it. If you believe it is wrong, STOP and report;
  - the workspace root `Cargo.toml`, `Cargo.lock`, and every other
    `Cargo.toml` — **no new dependencies**;
  - `scripts/**` — the shipped build must keep working untouched;
  - every other crate;
  - `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `PROGRESS.md`, `TODO.md`, `docs/**`,
    `README.md` (Claude writes the docs and the release checklist);
  - `.git`, git config;
  - `vm-test/**`, `reference/**`, `hosts/**`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **The 1 MiB dual-platform size budget is real.** The feature is off by
   default, so the shipped binary must be byte-identical. Claude gates it.
3. **No hidden backdoor in the shipped binary.** `strings` on the
   `scripts/release-build.sh` output must find `DUN_TEST_PANIC` **zero** times.
   Verify it and paste the number.
4. **Terminal restore is an A-level invariant.** Do not touch `lifecycle.rs`.
5. **Measure the exit status, do not assume it.** Signal 6 is what a shell
   reports as 134; what Rust hands back through `ExitStatus` may differ. Look.
6. **PTY tests skip, not fail, without `expect(1)`.**
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo test --release --features test-panic-hook -p dun-cli --test pty_smoke
scripts/release-build.sh && strings target/x86_64-apple-darwin/release/dun | grep -c DUN_TEST_PANIC
```

The last command must print `0`. Paste verbatim output for all of it, plus the
mutant run from §3.

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
2. Verification — every command with verbatim output, including the mutant run
   and the `strings | grep -c` count.
3. The finding — the exact exit status the abort produced, and the list of
   `lifecycle.rs` branches still uncovered.
4. Stop-loss / open questions (empty if none).
