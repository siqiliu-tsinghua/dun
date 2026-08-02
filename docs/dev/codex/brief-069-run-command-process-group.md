# Brief 069 — Kill the command's process group

Implementation brief. Step G2 of the plan approved from brief 067. G1
(`4b362e7`) bounded the capture read so it always returns; this step stops the
command leaving processes behind and stops a backgrounding command costing the
full timeout.

**Read the safety section before you write any code.** This is the first brief
in this project that sends a signal to a process group, and the failure mode is
that `dun` kills itself, the user's shell, and everything else in that session.

## Goal

Put each captured command in its own process group, and clean that group up
when the command finishes or times out. A backgrounding command must return
promptly instead of occupying the whole timeout, and must not leave descendants
running. The user must be told when something was killed.

## Verified current behaviour (treat as given)

After G1, `run_command_capture` (`crates/dun-cli/src/terminal/shell.rs:94`)
spawns `$SHELL -c <command>` and reads under a deadline, so it always returns.
It still kills only the shell (`wait_with_timeout`), so:

- `sleep 300 & echo started` occupies the **full timeout** before returning
  truncated output, because nothing closes the inherited pipe;
- the `sleep` keeps running after `dun` has moved on.

## Safety — the requirement, not a risk note

`rustix::process::kill_process_group(pid, sig)` performs **`kill(-pid, sig)`**:
you pass the **positive** process-group id and rustix negates it. Passing an
already-negated value, or the wrong id, signals the wrong group.

The catastrophic case is signalling **dun's own process group**, which
SIGKILLs the editor, its plugin hosts, and typically the user's shell session.
`Command::process_group(0)` is what prevents it — but if it ever silently
no-ops on some platform, the child stays in dun's group and its pgid *is* dun's
pgid.

Therefore:

1. **Derive the target from the child**, `child.id()`, never from a constant,
   a default, or `0`.
2. **Refuse to signal** when the target equals this process's own group
   (`rustix::process::getpgrp()`), or when it is `<= 1`. `pgid 1` is called out
   in rustix's own docs as signalling everything you have permission to signal.
3. **Never call `rustix::process::kill_current_process_group`** — it is
   `kill(0, sig)`, exactly the disaster. It must not appear anywhere in the
   diff.
4. Put the decision in a **pure function** so it can be tested directly, e.g.
   `fn group_kill_target(child_pid: u32, own_pgid: u32) -> Option<Pid>`
   returning `None` for every refusal case. The caller signals only on `Some`.
   A refusal is not an error to report to the user; fall through to reaping the
   direct child exactly as today.

## Specification (decided — implement exactly this)

1. **New group per command.** Call `CommandExt::process_group(0)` on the
   `Command` before `spawn`. It is safe and stable; `pre_exec` is `unsafe` and
   forbidden here — do not reach for it.
2. **Enable rustix's `process` feature** in the workspace `Cargo.toml:30`
   feature list. Add no new package.
3. **On timeout**, signal the group with `SIGKILL` through the guarded target,
   then reap the direct child as today.
4. **On normal shell exit**, signal the group the same way. `ESRCH` means
   nothing remained — that is the ordinary case and is **not** an error and
   **not** something the user is told about. A successful signal means
   descendants were there and were killed; record that.
5. **No SIGTERM grace.** The command already has a user-configured hard
   deadline, and this runs synchronously on the UI thread; a second grace period
   would lengthen a freeze the user is already waiting through.
6. **Report it.** Add a `background_processes_killed: bool` to
   `CommandRunResult` (`crates/dun-cli/src/command_output/model.rs:6`), set only
   when a signal actually reached a group member. When it is set, the status
   line and the Command Output `Status:` line must say so.
   - Add **whole translated sentences**, not fragments glued onto the existing
     ones — a clause appended in English does not survive translation. Enumerate
     the exact keys you add in your report.
   - The **timeout** wording does not change: `timed out; process killed`
     already tells the user something was killed.
7. **Move `elapsed` sampling to after both reader joins.** G1 left it before, so
   a backgrounding command reports the shell's 12 ms rather than the wait the
   user actually sat through. This step owns that surface, so fix it here.
8. Update `docs/user-guide.md:486-489` — `Ctrl+X,O` now owns the process group
   it creates, so a command is not a way to launch a background service; Shell
   Escape (`Ctrl+X,S`) is.

## Scope

- Files you MAY modify:
  - `Cargo.toml` — **only** to add `"process"` to rustix's feature list. No
    version change, no new dependency. `Cargo.lock` may change as a result.
  - `crates/dun-cli/src/terminal/shell.rs`
  - `crates/dun-cli/src/command_output/model.rs`
  - `crates/dun-cli/src/command_output/format.rs`
  - `crates/dun-cli/src/ui_text/status/command.rs`
  - `i18n/*.conf` — all ten catalogs
  - `crates/dun-cli/src/tests/command_output.rs`
  - `docs/user-guide.md`
- Files/areas you MUST NOT touch:
  - `crates/dun-plugin/**` — the plugin host is G3, a later step.
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `TODO.md`, `CHANGELOG.md`, and all of
    `docs/**` except `docs/user-guide.md`;
  - `.git`, git config, `vm-test/**` (local SSH keys), `reference/**`.

## Test requirements (this is what the gate checks)

Three tests:

- **The guard, directly.** Unit-test the pure `group_kill_target` function:
  a normal distinct pgid yields `Some`; a target equal to the caller's own pgid
  yields `None`; `0` and `1` yield `None`. This is the test that stands between
  a bug and a SIGKILLed editor, so it must be unmissable and independent of how
  the caller uses it.
- **No descendant survives** — the portable oracle from brief 067, and the only
  one that works on all four platforms (no `/proc`, no `pgrep`, no `ps` flags).
  A helper writes a `ready` sentinel, sleeps, then writes a `survived` sentinel
  and exits. Run it backgrounded from the command through the **real**
  `AppState::run_external_command_to_buffer` path
  (`app/command_output.rs:90`). Assert: `ready` appeared (so the helper really
  ran — without this the test passes vacuously when nothing starts at all), the
  capture returned promptly, and after the helper's full sleep plus margin
  `survived` never appeared.
- **A backgrounding command returns promptly.** Extend or complement G1's
  `command_capture_deadline_returns_from_background_descendant`: with the group
  killed, it should now return in well under the timeout rather than at it.

Rules:

- Keep every helper **short-lived** (a few seconds) and clean up temp dirs, so
  a failed run leaves nothing on the machine or the VMs.
- **Independent oracle**: assert on sentinel files and wall-clock time, not on
  the `background_processes_killed` flag the implementation sets for itself.
  Assert the flag separately, where it is the thing under test.
- **No existing test may be edited** except G1's timing bound if the group kill
  legitimately makes it faster; say so explicitly if you change it.
- **Report your own mutation runs**, at least: (a) remove `process_group(0)`,
  (b) replace the group signal with a direct-child kill, (c) weaken the guard so
  it returns `Some` for the caller's own pgid. Each must trip a specific test.
  For (c), verify by unit test only — do **not** run a mutated group kill
  against a live editor process. Restore by reversing each edit, never
  `git checkout`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `pre_exec` is out; `process_group(0)` and
   rustix are the sanctioned path.
2. **The 1 MiB dual-platform size budget is real.** This is the first step of
   the track that adds a dependency feature, so it is the first likely to cost
   pages. Binding Debian is at **788,784**, margin **259,792**. Keep the diff
   minimal. Claude measures on macOS and Debian, and this time on FreeBSD and
   Solaris too.
3. **Ten catalogs are load-bearing.** New UI text means all of `i18n/*.conf`.
   There is a catalog-completeness test; make it pass by translating, not by
   weakening the test.
4. **Four platforms, and this step is the one most likely to differ.**
   `process_group(0)` behaviour on FreeBSD and Solaris is **unverified**. If it
   does not behave as on Linux/macOS, STOP and report — that is a stop
   condition, not permission to assume POSIX equivalence.
5. **Terminal-detection env is pinned in harnesses** — TERM, COLORTERM, LANG,
   LC_CTYPE, NO_COLOR (see `crates/dun-cli/tests/support/`).
6. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Baseline at `3e9cc03` is **935 passed / 0 failed / 3 ignored** (the three are
pre-existing `#[ignore]`d performance baselines). Report your totals and state
whether tmux was available.

Report the measured wall-clock duration of the backgrounding-command test
before and after your change, since "returns promptly" is the claim.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify files outside Scope. If you believe you must, STOP and report.
- Full machine access, but touch NOTHING outside this repo, no network.
- Do not leave stray processes behind; every helper must exit on its own.
- You MUST paste real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. The exact i18n keys added, and confirmation all ten catalogs carry them.
3. Verification — commands with verbatim output, the tmux statement, and the
   before/after durations.
4. Your three mutation runs, each with the test it tripped.
5. Stop-loss / open questions — especially anything you could not verify about
   FreeBSD or Solaris.
