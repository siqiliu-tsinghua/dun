# Brief 071 — Deterministic plugin cleanup at editor exit

Implementation brief. Step G4, the last of the plan approved from brief 067.

## Why this is not polish

G3 (`664058f`) sweeps a host's process group from `HostClient::kill` and
`Drop`. But `Drop` only runs if the worker thread that owns the client gets to
run. It does not at editor exit:

- `crates/dun-cli/src/plugins.rs:173` spawns the worker with `thread::spawn`
  and **discards the `JoinHandle`**;
- `PluginHost` stores only channels (`jobs`, `events`) — no handle, no pgid;
- `crates/dun-cli/src/main.rs:186-195` runs the event loop, calls
  `backend.show_cursor()`, and returns. **Nothing shuts plugins down.**

So when the user quits, the process exits, detached workers die where they
stand, no `HostClient` is dropped, no group is swept, and hosts and their
helpers survive. **G3's guarantee does not currently hold on the most common
exit path.** This step is what makes it true.

## Goal

When `dun` exits normally, no plugin host and no host helper survives, and the
quit does not become noticeably slower — with no plugins configured it must not
get slower at all.

## Specification (decided — implement exactly this)

1. **Know the group before the handshake.** The worker learns its host's
   process-group id the moment the child is spawned, which is earlier than
   `HostEvent::Started` (that arrives only after a successful handshake, so a
   host that launched but hung or failed the handshake would be invisible).
   Share it as an **`Arc<AtomicU32>`**, written by the worker right after
   launch and read by the main thread at exit, with `0` meaning "not known
   yet". Lock-free, and it covers every window including a hung launch.
   Expose whatever accessor `HostClient` needs for this;
   `crates/dun-plugin/src/client.rs` is in scope for that alone.
2. **Keep the handle.** `PluginHost` stores the worker's `JoinHandle` and that
   shared pgid cell.
3. **An explicit exit path**, `PluginHosts::shutdown_all`, called from `main`
   after the event loop returns:
   1. ask every worker to stop (they already own the graceful protocol
      `Shutdown` through `HostClient::shutdown`);
   2. wait for the workers to finish, against **one shared deadline for all
      hosts, not one per host**, so N plugins do not multiply the wait;
   3. when the deadline passes, **sweep the group of every host that has not
      finished**, using the existing guarded kill. This both guarantees the
      cleanup and unwedges a worker blocked reading from a stuck host;
   4. return. Do not block further — the process is exiting and the sweep is
      the guarantee.
4. **Restore the terminal first.** The wait must not happen while the screen is
   still in raw mode. Make the ordering **explicit** in `main` rather than
   relying on the implicit drop order of locals — state in your report how the
   ordering is guaranteed.
5. **Zero hosts, zero cost.** With no plugins configured, `shutdown_all` must
   return immediately and add no measurable latency to quit.

## Safety — unchanged, restated

The sweep uses the guard that moved into `dun-plugin` in G3
(`group_kill_target`). Do not add a second copy, do not bypass it, and
`rustix::process::kill_current_process_group` — `kill(0, sig)` — must not
appear in the diff.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/plugins.rs`
  - `crates/dun-cli/src/plugins/worker.rs`
  - `crates/dun-cli/src/main.rs`
  - `crates/dun-plugin/src/client.rs` — **only** to expose the host's
    process-group id to its owner
  - `crates/dun-cli/src/tests/plugins/lifecycle.rs`
  - `crates/dun-cli/tests/` — a PTY/tmux test if the exit path needs one
  - `docs/plugin-authoring.md`
- Files/areas you MUST NOT touch:
  - `i18n/**` — no new UI text; if you think one is needed, STOP and report;
  - `Cargo.toml`, `Cargo.lock` — everything needed is already a dependency;
  - `crates/dun-cli/src/terminal/shell.rs` — the command path is done (G1, G2);
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `TODO.md`, `CHANGELOG.md`, and all of
    `docs/**` except `docs/plugin-authoring.md`;
  - `.git`, git config, `vm-test/**` (local SSH keys), `reference/**`.

## Test requirements (this is what the gate checks)

- **The real exit path, with the sentinel oracle.** A fixture host spawns a
  helper that writes a `ready` sentinel, sleeps, then writes `survived` and
  exits. Start the editor with that host configured, let the host launch, quit
  the editor normally, and assert: `ready` appeared (**without this the test
  passes vacuously when the helper never starts**), and after the helper's full
  sleep plus margin, `survived` never appeared. This has to go through the
  actual quit, not a direct call to `shutdown_all` — the defect is precisely
  that the real exit path skips the cleanup.
- **A host that never completes its handshake is still cleaned up.** This is
  the case that motivates the `Arc<AtomicU32>` over `HostEvent::Started`; cover
  it or explain why it is unreachable.
- **Quit stays fast with no plugins.** Assert an upper bound on the exit path
  with zero hosts configured.
- Keep helpers short-lived (a few seconds) and clean up temp dirs, so a failed
  run leaves nothing behind on the machine or the VMs.
- **Report your own mutation runs**, at least: (a) drop the `shutdown_all` call
  from `main`, (b) remove the post-deadline sweep so only the graceful stop
  remains and a wedged host survives. Each must trip a named test. Restore by
  reversing each edit, never `git checkout`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.**
2. **The 1 MiB dual-platform size budget is real.** Binding Debian is at
   **796,976** with **251,600** to spare. This step adds no dependency and
   should be small. Claude measures all four platforms.
3. **`crates/dun-cli/src/main.rs` is the prelude hub.** Modules use
   `use crate::*`; if you move or remove a symbol, update the import lists in
   `main.rs` in the same change.
4. **Terminal-detection env is pinned in harnesses** — TERM, COLORTERM, LANG,
   LC_CTYPE, NO_COLOR (see `crates/dun-cli/tests/support/`). A PTY test that
   leaks one of these becomes environment-dependent.
5. **The tmux suite skips cleanly without tmux.** If you add a tmux-backed
   test, say explicitly whether it ran or skipped — a smaller total reported as
   green would hide it.
6. **Four platforms.** macOS, Debian, FreeBSD and Solaris all run this; Claude
   gates all four. Solaris tmux renders ambiguous-width glyphs double-width, so
   prefer an oracle that does not depend on rendered output.
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Baseline at `f9e7594` is **941 passed / 0 failed / 3 ignored** (the three are
pre-existing `#[ignore]`d performance baselines). Report your totals and state
whether tmux was available.

Report the measured quit latency with zero hosts and with one host, since
"does not get noticeably slower" is the claim.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify files outside Scope. If you believe you must, STOP and report.
- Full machine access, but touch NOTHING outside this repo, no network.
- Do not leave stray processes behind; every helper must exit on its own.
- You MUST paste real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. How the terminal-restore-before-wait ordering is guaranteed.
3. Verification — commands with verbatim output, the tmux statement, and the
   two quit-latency measurements.
4. Your two mutation runs, each with the test it tripped.
5. Stop-loss / open questions.
