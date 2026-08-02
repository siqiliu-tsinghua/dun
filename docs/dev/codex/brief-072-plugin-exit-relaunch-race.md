# Brief 072 — Plugin exit cleanup, without the relaunch race

Implementation brief. This restores the work reverted at `1651513` and fixes
the race that forced the revert. Read brief 071 first for the original goal;
this brief only adds what that one got wrong.

## What happened

Brief 071's implementation (commit `493caaf`, plus a test-timing fix
`a35f05c`) was reverted at `1651513` because
`normal_exit_sweeps_helper_when_host_never_finishes_handshake` failed
intermittently on Debian — **2 of 5 runs in isolation**, so it was not load or
test-suite parallelism. macOS, FreeBSD and Solaris passed.

**The test was right and the implementation was wrong.** Diagnosis, verified
against the code:

- `HostClient::sweep_process_group` cleared the shared cell with
  `process_group.swap(0, …)`.
- A host whose handshake hangs cycles: launch → handshake timeout → `launch`
  returns `Err` → `HostClient` dropped → `Drop` → `kill()` → **cell zeroed**,
  group killed → `RELAUNCH_COOLDOWN` (5 s, `plugins.rs:44`) → relaunch → new
  host, new pgid stored.
- **For the whole cooldown the cell reads 0.** If the editor exits in that
  window, `PluginHosts::shutdown_all` reads 0, `group_kill_target(0, _)`
  returns `None`, nothing is swept, the process exits, and the host relaunched
  moments later — along with its helper — survives.

The window was seconds wide, which is why it hit so often.

An unrelated and *correct* fix also went in `a35f05c` and is reverted only
because it now guards nothing: the test's two timing margins were too small for
a slow VM. Restore that too — see "Test timing" below.

## Decisions (implement these; do not re-open)

1. **Stop clearing the shared cell on teardown.** `HostClient`'s own sweep must
   kill using the pgid without zeroing the shared slot. Keep idempotence with
   private state if you need it — a separate `bool`/`Option` inside
   `HostClient` — never by mutating the cell the owner reads. This alone
   shrinks the window from the full 5 s cooldown to the microseconds between
   `spawn` and the store that immediately follows it.
2. **Do not relaunch once shutdown has begun.** `shutdown_all` sets a shared
   stop flag (an `AtomicBool` alongside the pgid cell is fine) *before* it asks
   workers to stop; the worker checks it immediately before spawning a host and
   gives up instead. That closes what decision 1 narrows.
3. **Order stays: signal, wait against one shared deadline, then sweep whatever
   has not finished.** Unchanged from brief 071, which was right about this.
4. **Everything else about `493caaf` stands** — the `Arc<AtomicU32>` published
   right after spawn, the `JoinHandle` on `PluginHost`, the explicit
   restore-then-wait ordering in `main` with deferred `?`, and the immediate
   return when no hosts are configured. Restore them rather than redesigning:
   `git show 493caaf` is the reference.

**A stale pgid is acceptable; a zeroed one is not.** If the cell holds the id of
a group that is already gone, the kill returns `ESRCH` and nothing happens —
that is the same trade G2 and G3 already make. The guard (`group_kill_target`)
still refuses our own group and `<= 1`, and must keep doing so.

## Test timing (restore from `a35f05c`)

The oracle needs two margins in opposite directions, and both were originally
too small for a loaded VM:

- the helper's sleep must **outlast** the worst-case ready-to-quit-plus-deadline
  window, or a survival sentinel means only that the helper finished on its own;
- the observation must **outlast** the helper's sleep, or an absent sentinel
  means only "still sleeping" and the assertion passes vacuously.

`HELPER_SLEEP` 2 s → 8 s, `HUNG_HOST_OBSERVATION` 3.25 s → 10 s, the hung host
hangs 20 s, and the hung-handshake quit bound goes 1 s → 3 s. Keep the comment
explaining why both bounds exist.

## The gate this brief has to clear

**Prove the test catches the bug that caused the revert.** This is the
requirement that matters most, because a test that only passes is worthless
here — the reverted implementation *passed* on three platforms.

1. Restore `493caaf`'s implementation **without** decisions 1 and 2, keeping the
   restored test timings.
2. Show the test fails against it — on Debian if you can reach it, otherwise say
   plainly that you could only reproduce on some platforms and report the counts
   you saw. It is intermittent, so **run it at least 10 times** and report the
   failure count, not a single run.
3. Then apply decisions 1 and 2 and show the same loop passing 10/10.

Report both loops verbatim. If you cannot make the test fail against the
known-bad implementation at all, **STOP and report that** — it means the test
still cannot see the defect and widening or rewriting it is the real task.

Also keep the two mutation runs from brief 071 (drop the `shutdown_all` call;
remove the post-deadline sweep) and report which test each trips.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/plugins.rs`
  - `crates/dun-cli/src/plugins/worker.rs`
  - `crates/dun-cli/src/main.rs`
  - `crates/dun-plugin/src/client.rs`
  - `crates/dun-cli/src/tests/plugins/lifecycle.rs`
  - `crates/dun-cli/tests/plugin_exit.rs` (recreate it)
  - `docs/plugin-authoring.md`
- Files/areas you MUST NOT touch:
  - `i18n/**` — no new UI text; if you think one is needed, STOP and report;
  - `Cargo.toml`, `Cargo.lock` — everything needed is already a dependency;
  - `crates/dun-cli/src/terminal/shell.rs` — the command path is done;
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `TODO.md`, `CHANGELOG.md`, and all of
    `docs/**` except `docs/plugin-authoring.md`;
  - `.git`, git config, `vm-test/**` (local SSH keys), `reference/**`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.**
2. **The 1 MiB dual-platform size budget is real.** Binding Debian is at
   **796,976** with **251,600** to spare. No new dependency; this should be
   small. Claude measures all four platforms.
3. **`crates/dun-cli/src/main.rs` is the prelude hub** — update its import lists
   in the same change if you move a symbol.
4. **Terminal-detection env is pinned in harnesses** — TERM, COLORTERM, LANG,
   LC_CTYPE, NO_COLOR (see `crates/dun-cli/tests/support/`).
5. **Do not leave stray processes behind.** Helpers must exit on their own; the
   hung host's 20 s bound is what keeps a failed run from lingering.
6. **Four platforms.** Only Debian exposed this. Do not conclude a race is fixed
   because macOS is green.
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Baseline at `1651513` is **941 passed / 0 failed / 3 ignored**. Report your
totals and state whether tmux was available.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify files outside Scope. If you believe you must, STOP and report.
- Full machine access, but touch NOTHING outside this repo, no network.
- You MUST paste real verbatim verification output, including both 10-run
  loops. If a run did not reach green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. The two 10-run loops: against the known-bad implementation, and against the
   fixed one, with failure counts.
3. Verification — commands with verbatim output, plus the tmux statement.
4. The two brief-071 mutation runs, each with the test it tripped.
5. Stop-loss / open questions.
