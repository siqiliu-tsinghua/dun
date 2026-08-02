# Brief 070 — Plugin host process-group cleanup

Implementation brief. Step G3 of the plan approved from brief 067. G2
(`f9bc92d`) gave captured commands their own process group and cleaned it up;
this does the same for plugin hosts, reusing — not re-implementing — the guard
that keeps a group kill from hitting the editor.

## Goal

`HostClient::kill` (`crates/dun-plugin/src/client.rs:666-669`) is
`Child::kill()` + `wait()`, so a host that spawned helpers leaves them running
on timeout, protocol violation, unload, and editor exit. Give each host its own
process group and sweep that group whenever the host is killed or reaped.

Urgency is low and the brief should stay small: all five shipped hosts under
`hosts/` are single-process, so this is a third-party-host risk, not a live
one. Do not grow it into a resource-limits feature.

## Decisions already made (implement these; do not re-open)

1. **rustix goes into `crates/dun-plugin/Cargo.toml`**, whose `[dependencies]`
   table is currently empty. rustix is already a workspace dependency and
   already in `Cargo.lock`, so this adds **no new package**. The `process`
   feature was enabled workspace-wide by G2. No doc claims this crate is
   dependency-free — the "dependency-free" wording in
   `docs/plugin-protocol.md:524,528` is about the Lua and Python *hosts*.
2. **The guard moves; it is not copied.** `group_kill_target` and the guarded
   kill currently live in `crates/dun-cli/src/terminal/shell.rs`. Move that
   guard into `dun-plugin`, export it, and have `shell.rs` use it from there.
   `dun-cli` already depends on `dun-plugin`, so the direction works.
   **Two copies of a function whose job is to stop dun SIGKILLing itself is not
   acceptable** — one implementation, one set of tests, two callers.
   **Move its unit test with it.** A refactor that leaves the guard covered by
   nothing is the failure this project keeps hitting; the adversarial cases
   (own pgid, `0`, `1`) must still be asserted after the move.
3. **`#[cfg(unix)]`.** `dun-plugin` contains no `cfg(unix)`/`cfg(windows)`
   today — it is portable Rust, and the project's stated layering "keeps the
   Windows door open". Put the process-group spawn and the group kill behind
   `#[cfg(unix)]` with a documented non-unix fallback that behaves as today
   (`Child::kill()`), so the door stays open.
4. **No new UI text, and therefore no i18n trail.** G2 told the user because
   the user typed the command. Nobody asked for a host's helpers, so cleaning
   them up is silent. If you believe a message is required, STOP and report
   rather than adding one — new UI text means all ten catalogs and belongs to a
   step that plans for it.
5. **The protocol shutdown handshake is the grace period.** `shutdown`
   (`client.rs:~600`) already sends `Shutdown` and waits `SHUTDOWN_GRACE`
   before calling `kill()`. Do not add a second SIGTERM stage. Sweep the group
   after the host is reaped — including after a *successful* shutdown, because
   a host can exit cleanly and still have left helpers behind.

## Safety — unchanged from G2, restated because it is the whole risk

`kill_process_group(pid, sig)` performs **`kill(-pid, sig)`**: pass the
**positive** group id. Aimed at dun's own group it SIGKILLs the editor, every
plugin host, and usually the user's shell session.
`Command::process_group(0)` is what prevents that, and the guard exists because
it might not. Derive the target from the child's own pid, refuse when it equals
`getpgrp()` or is `<= 1`, and **never** let
`rustix::process::kill_current_process_group` — `kill(0, sig)` — appear in the
diff.

## Scope

- Files you MAY modify:
  - `crates/dun-plugin/Cargo.toml`
  - `crates/dun-plugin/src/client.rs`
  - `crates/dun-plugin/src/lib.rs` (to export the moved guard)
  - `crates/dun-plugin/src/bin/fixture-host.rs`
  - `crates/dun-plugin/tests/protocol.rs`
  - `crates/dun-cli/src/terminal/shell.rs` (to consume the moved guard)
  - `docs/plugin-protocol.md`, `docs/plugin-authoring.md`
  - `Cargo.lock` may change as a result; do not hand-edit it.
- Files/areas you MUST NOT touch:
  - the workspace `Cargo.toml` — G2 already enabled rustix's `process` feature;
  - `i18n/**` — see decision 4;
  - `crates/dun-cli/src/plugins.rs` and `plugins/worker.rs` — editor-exit worker
    ownership is **G4**, a later step;
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `TODO.md`, `CHANGELOG.md`, and all of
    `docs/**` except the two named above;
  - `.git`, git config, `vm-test/**` (local SSH keys), `reference/**`.

## Test requirements (this is what the gate checks)

- **The moved guard keeps its adversarial unit test** — normal pgid yields
  `Some`, own pgid yields `None`, `0` and `1` yield `None`. It must live with
  the implementation after the move, and `dun-cli` must still compile against
  it.
- **No host descendant survives**, using the portable sentinel oracle from
  brief 067 — the only one that works on all four platforms (no `/proc`, no
  `pgrep`, no `ps` flags). Extend `fixture-host.rs` with a mode that spawns a
  helper which writes a `ready` sentinel, sleeps, then writes `survived` and
  exits. Drive it through a real `HostClient` in
  `crates/dun-plugin/tests/protocol.rs`. Assert `ready` appeared — **without
  that the test passes vacuously when the helper never starts** — and that
  after the helper's full sleep plus margin, `survived` never appeared.
- Cover both routes: a host killed on **timeout or protocol violation**, and a
  host that **shuts down cleanly** but left a helper behind. The second is the
  one that is easy to miss.
- Keep every helper short-lived (a few seconds) and clean up temp dirs, so a
  failed run leaves nothing on the machine or the VMs.
- **Independent oracle**: assert on sentinel files and wall-clock time, not on
  any flag the implementation sets for itself.
- **Report your own mutation runs**, at least: (a) remove `process_group(0)`
  from the host spawn, (b) revert the group kill to `Child::kill()`. Each must
  trip the sentinel test. Restore by reversing each edit, never `git checkout`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `pre_exec` is out; `process_group(0)` and
   rustix are the sanctioned path.
2. **The 1 MiB dual-platform size budget is real.** Binding Debian is at
   **792,880** with **255,696** to spare. G2 spent one page for the same
   mechanism, so this step should cost little — most of what it needs is
   already linked. Claude measures on all four platforms.
3. **Do not weaken the existing menu/validator tests.** `menu.rs:390-398` and
   the protocol suite exist for other reasons; leave them alone.
4. **Four platforms.** `process_group(0)` was verified identical on macOS,
   Debian, FreeBSD and Solaris during G2, so that question is settled — but the
   fixture-host lifecycle is new code and Claude gates all four.
5. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Baseline at `d23cdf8` is **939 passed / 0 failed / 3 ignored** (the three are
pre-existing `#[ignore]`d performance baselines). Report your totals and state
whether tmux was available.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify files outside Scope. If you believe you must, STOP and report.
- Full machine access, but touch NOTHING outside this repo, no network.
- Do not leave stray processes behind; every helper must exit on its own.
- You MUST paste real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Where the guard now lives, and proof its adversarial test moved with it.
3. Verification — commands with verbatim output, plus the tmux statement.
4. Your two mutation runs, each with the test it tripped.
5. Stop-loss / open questions.
