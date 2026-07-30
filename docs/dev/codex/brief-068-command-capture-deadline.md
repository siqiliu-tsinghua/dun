# Brief 068 — Bound the command-capture read (stop the editor hanging)

Implementation brief. Step G1 of the plan approved from brief 067. This is the
step that stops `dun` hanging indefinitely; do it alone and do it first. Process
groups, killing descendants, and the plugin client are **later steps** — do not
reach for them here.

## Goal

`run_command_capture` can block the editor's UI thread forever. Give the reader
threads an absolute deadline so the capture always returns, and make the
function always join them instead of dropping still-blocked handles on the error
path. No process is killed in this step and no dependency changes.

## Verified current behaviour (measured — treat as given, do not re-derive)

`read_capped_stream` (`crates/dun-cli/src/terminal/shell.rs:156-179`) loops until
`read()` returns 0. It keeps reading after passing `stream_limit` — it just
stops accumulating — so it exits only at **EOF**, which requires every write end
of the pipe to be closed, including ends inherited by descendants of the shell.

`wait_with_timeout` (`shell.rs:131-148`) kills only the shell. So a command that
leaves a descendant running keeps the pipe open, the reader thread never
returns, and `join_captured_stream` (`shell.rs:114`) blocks its caller —
`run_external_command_to_buffer` (`crates/dun-cli/src/app/command_output.rs:90`),
which runs **synchronously on the UI thread**.

Reproduced with a standalone program mirroring `shell.rs` exactly:

```
command: sleep 300 & echo started
[11.9ms] shell reaped, timed_out=false
[12.0ms] now joining reader threads...
*** STILL BLOCKED in join after 8s ***
```

`timed_out=false` is the point: the shell exits normally in 12 ms because the
`sleep` is backgrounded, so the 30 s timeout never fires and nothing bounds the
wait.

Two facts you may rely on, both verified:

- **No existing test covers this.** The only timeout test,
  `run_command_kills_non_terminating_commands_after_timeout`
  (`crates/dun-cli/src/tests/command_output.rs:122-140`), runs a *foreground*
  `sleep 5`, which leaves no descendant holding the pipe.
- **`rustix::event::poll` is already used and already linked** —
  `crates/dun-cli/src/terminal/sys/unix.rs:188`, `event` feature already on
  (`Cargo.toml:30`). Follow that call site's local idiom. **Add no dependency
  and change no feature in this brief.**

## Specification (decided — implement exactly this)

1. **One absolute deadline**, computed once as `started + timeout`, shared by
   both reader threads and by `wait_with_timeout`. Do not give each reader its
   own relative budget.
2. **`read_capped_stream` becomes deadline-aware.** Before each read, compute
   the remaining time; poll the fd for readability with that timeout; on
   expiry, stop and return what has been collected with `truncated = true`.
   Retry on `EINTR`. When the poll reports hangup, **drain the readable data
   before accepting EOF** — treating hangup as immediate EOF loses the tail of
   the output.
3. **Keep draining past the cap.** Today the loop keeps reading after
   `stream_limit` and only stops accumulating; preserve that. Stopping the read
   at the cap would leave a *foreground* child blocked on a full pipe, which
   trades this bug for a worse one.
4. **Always join both readers** before the function returns any result or any
   error. Today an error propagated at `shell.rs:112` drops the handles and
   leaves the threads detached and blocked; that is part of what this brief
   fixes.
5. **No process is killed here.** After this step, `sleep 300 & echo started`
   still occupies the full timeout before returning truncated output — that is
   expected and is fixed in the next step by killing the process group. The
   invariant this brief must establish is only that the capture **always
   returns**.
6. Update `docs/user-guide.md:486-488`, which currently promises "the command is
   killed after `limits.run_command_timeout_ms`". That is not true when the
   command leaves a background process: nothing was killed and the editor hung.
   State what is actually guaranteed after this step — the capture returns
   within the timeout, and output may be reported truncated.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/terminal/shell.rs`
  - `crates/dun-cli/src/tests/command_output.rs`
  - `docs/user-guide.md`
- Files/areas you MUST NOT touch:
  - `Cargo.toml`, `Cargo.lock` — **no dependency or feature change in this
    brief.** If you conclude one is needed, STOP and report.
  - `crates/dun-plugin/**` — the plugin client is a later step.
  - `crates/dun-cli/src/app/command_output.rs` — the caller does not change.
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `TODO.md`, and all of `docs/**`
    except `docs/user-guide.md`;
  - `.git`, git config, `vm-test/**` (local SSH keys), `reference/**`.

## Test requirements (this is what the gate checks)

Two tests, and they must fail for different reasons:

- **Unit: the reader releases an open pipe.** Give `read_capped_stream` a pipe
  whose write end is deliberately held open, and assert it returns by its
  deadline with `truncated = true`. **The test must not be able to hang the
  suite**: have the supervising thread close the write end and fail loudly if
  the deadline is missed, rather than blocking forever.
- **Production path: a backgrounding command returns.** Drive
  `AppState::run_external_command_to_buffer` — the real caller at
  `app/command_output.rs:90` — with a command that exits its shell while a
  descendant keeps stdout open, and assert the call returns within a bound
  comfortably under the descendant's lifetime. Set
  `limits.run_command_timeout_ms` low, as the existing timeout test does.
  **Keep the descendant short-lived** (a few seconds), so that if cleanup ever
  fails, a test run leaves nothing behind on the machine — nothing in this step
  kills it.

Rules:

- **Independent oracle.** Assert on elapsed wall time and the returned/`truncated`
  state, not on any internal flag the implementation sets for its own use.
- **No existing test may be edited.** `run_command_kills_non_terminating_commands_after_timeout`
  must still pass unchanged. If you need to change it, STOP and report.
- **Report your own mutation run**: disable the deadline branch and show both
  new tests fail (the unit one on its deadline, the production one on its time
  bound). Restore by reversing the edit — never `git checkout`, the tree is
  dirty with your own work.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe** (`#![forbid(unsafe_code)]` in every crate
   root). `pre_exec` and raw `libc` calls are not available; you should not need
   either here.
2. **The 1 MiB dual-platform size budget is real.** `shell.rs` ships. Binding
   platform Debian is at **788,784** with **259,792** to spare. No new
   dependencies, no new generic layers. Claude measures on macOS + Debian.
3. **Static, translated status text.** This step should need no new UI string;
   reuse the existing truncated-output states. If you believe a new string is
   required, STOP and report rather than inventing one — new UI text means all
   ten `i18n/*.conf` catalogs and belongs to a step that plans for it.
4. **Terminal-detection env is pinned in harnesses.** Any test that spawns the
   editor must pin/clear TERM, COLORTERM, LANG, LC_CTYPE, NO_COLOR (see
   `crates/dun-cli/tests/support/`).
5. **Four platforms.** macOS, Debian, FreeBSD and Solaris all run this code, and
   Claude gates on all four. Use no Linux-only construct; `poll` via `rustix` is
   the portable path already in the tree.
6. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report — do not keep tuning.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

The workspace baseline at `8a3dddf` is **933 passed / 0 failed / 3 ignored**
(the three are pre-existing `#[ignore]`d performance baselines). Report your
totals and say explicitly whether tmux was available, because the PTY suite
skips cleanly without it and a smaller total reported as green would hide that.

Also report the wall-clock duration of your new production-path test, so the
bound can be judged rather than taken on trust.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave changes in the
  working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network.
- Do not leave stray processes behind. Any helper your tests spawn must exit on
  its own within a few seconds.
- Minimal diff: no drive-by reformatting, renames, or comment changes outside
  the task.
- You MUST paste real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command run, with exact verbatim output lines, the tmux
   statement, and the new test's measured duration.
3. Your mutation run, with both tests' failures.
4. Stop-loss / open questions — where you stopped and why (empty if none).
