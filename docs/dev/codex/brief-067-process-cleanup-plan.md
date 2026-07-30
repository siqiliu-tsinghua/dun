# Brief 067 — Command and plugin process cleanup (PLAN ONLY)

**This is a design-only brief. `Scope: NONE — no source change.`** You produce a
plan; Claude reviews it, decides the open questions, and dispatches the
implementation steps as separate gated briefs.

## Goal

Plan the fix for a defect that can hang `dun` indefinitely, and for the process
leak that shares its root cause. Both come down to the same thing: `dun` spawns
a child, and reasons about that child alone, while the child's descendants
outlive it and keep its pipes open.

The plan must let Claude dispatch each step independently with a named test
gate, and must state the byte cost of each option — this is runtime code under
the 1 MiB budget (binding platform Debian, currently **788,784**, margin
**259,792**).

## Verified current behaviour (measured — treat as given, do not re-derive)

### The hang

`run_command_capture` (`crates/dun-cli/src/terminal/shell.rs:87`) spawns
`$SHELL -c <command>` with piped stdout/stderr, hands each pipe to a reader
thread, then calls `wait_with_timeout` (`shell.rs:131-148`) and finally
`join_captured_stream` (`shell.rs:114`).

Two facts combine badly:

1. `wait_with_timeout` kills only the **shell** (`child.kill()`, `shell.rs:142`).
2. `read_capped_stream` (`shell.rs:156-179`) loops until `read()` returns 0. It
   keeps reading after passing `stream_limit` — it just stops accumulating — so
   it only exits at **EOF on the pipe**, which requires *every* write end to be
   closed, including those inherited by descendants.

So any command that leaves a descendant running holds the pipe open, the reader
thread never returns, and `join_captured_stream` blocks the caller forever.
`run_external_command_to_buffer` (`crates/dun-cli/src/app/command_output.rs:90`)
calls this **synchronously on the UI thread**.

Reproduced with a standalone program mirroring `shell.rs` exactly:

```
command: sleep 300 & echo started
[11.9ms] shell reaped, timed_out=false
[12.0ms] now joining reader threads...
*** STILL BLOCKED in join after 8s ***
```

Note `timed_out=false`: the shell exits normally in 12 ms because the `sleep` is
backgrounded, so **the 30 s timeout never fires at all**
(`run_command_timeout_ms`, `crates/dun-config/src/limits.rs:31`). The editor
then blocks for as long as the grandchild lives — 300 s here, unbounded in
general. There is no timeout covering this, no cap, and no recovery.

### The leak

The same missing idea, in two places:

- `shell.rs:142` on timeout: SIGKILL to the shell; its children survive.
- `crates/dun-plugin/src/client.rs:666-669`: `HostClient::kill` is
  `Child::kill()` + `wait()`. A plugin host that spawned helpers leaves them
  behind on timeout, protocol violation, unload, or editor exit. This is the
  external review's §4. The five shipped hosts under `hosts/` are all
  single-process, so today this is a third-party-host risk, not a live one —
  unlike the hang, which is reachable right now from `run-command`.

## Constraints and facts already established (rely on these)

- **`#![forbid(unsafe_code)]` in every crate root.** `CommandExt::pre_exec` is
  `unsafe` and is therefore **not available**. Do not plan around it.
- **`std::os::unix::process::CommandExt::process_group(0)` is safe**, stable
  since Rust 1.64, and the workspace is on 1.85 (`Cargo.toml:16`). It puts the
  child in a new process group whose id is the child's pid.
- **rustix 0.38 is already a direct dependency** (`Cargo.toml:30`) with
  `default-features = false, features = ["std", "stdio", "termios", "event"]`.
  `rustix::process::kill_process_group(Pid, Signal)` exists
  (`rustix-0.38.44/src/process/kill.rs:34`) but is behind the **`process`**
  feature, which is **not currently enabled**. Enabling it is a feature
  addition to an existing dependency, not a new dependency — but it must be
  measured.
- **`rustix::event::poll` is already used** at
  `crates/dun-cli/src/terminal/sys/unix.rs:188`, and the `event` feature is
  already on. Reading a pipe with a deadline therefore needs **no dependency
  change at all**.
- Four platforms must work: macOS, Debian, FreeBSD, Solaris. All four VMs are
  available to Claude for the implementation gates.

**The two problems are separable, and their costs differ.** Bounding the read
fixes the hang using infrastructure that already exists. Killing descendants
fixes the leak and needs the `process` feature. Your plan should treat that as
a sequencing opportunity, not assume one step.

## Deliverable

A plan document as your report (no files written). It must contain:

1. **Step list.** Each step: files and functions touched, what changes, the
   named test gate, and whether it is behaviour-changing. Order so each step is
   independently committable and green on its own. State explicitly which step
   makes the editor stop hanging — that one should come first unless you can
   argue otherwise.
2. **Call-site inventory**, with `path:line` evidence: every `Command::spawn` in
   `crates/**`, every place a `Child` is killed or waited on, and every caller
   of `run_command_capture` and `HostClient::kill`. Include the suspend/shell
   path (`shell.rs:83-85`) and say whether it has the same exposure.
3. **A portable test oracle for "no descendants survive."** This is the hard
   part and the reason this brief exists. Say concretely how a test asserts a
   grandchild is gone on macOS, Debian, FreeBSD **and** Solaris — `/proc` is not
   portable, `pgrep` flags differ, and the four platforms' `ps` differ (see
   docs/dev/solaris-vm.md for known command differences). If the honest answer
   is that one platform can only be checked manually, say so rather than
   inventing a uniform mechanism.
4. **Byte-cost estimate per option**, since this is runtime code. In particular:
   the `rustix` `process` feature (what does it pull in — note
   `process = ["linux-raw-sys/prctl"]`), a poll-based bounded read versus the
   current blocking read, and any new error/status text. Recommend the cheapest
   option that actually fixes the defect.
5. **Invariant preservation.** How each step keeps `forbid(unsafe_code)`, adds
   no new dependency, keeps new status text on the sanitized path, and leaves
   the terminal restored if a command is abandoned mid-run.
6. **Risks and open questions** — answer the list below with evidence where the
   code decides it, and flag clearly what is a judgement call for Claude.

## Open questions the plan must address

1. **What is the contract for a backgrounded process?** `run-command` captures
   output with a timeout. If the user runs `make &` or starts a daemon, should
   `dun` kill it on the way out? Killing it is what makes the pipe close and the
   leak stop; not killing it means the command "worked" but left something
   behind. Recommend one and say what the status line should tell the user.
2. **Bound the read, kill the group, or both?** If the reader is bounded by a
   deadline, does the group kill still matter for `run-command`? Argue it from
   the failure modes, not from tidiness.
3. **Grace before force.** SIGTERM then SIGKILL after a grace period, or SIGKILL
   alone? What does the plugin protocol's existing shutdown handshake
   (`docs/plugin-protocol.md`) already promise, and does that constrain the
   answer for `HostClient`?
4. **Does `process_group(0)` behave the same on all four platforms?** Any
   Solaris or FreeBSD caveat. If you cannot determine it by reading, name the
   experiment rather than guessing.
5. **Is the plugin client the same step or its own?** A host is long-lived and
   has a shutdown handshake; a captured command is transient. Same mechanism,
   or two?
6. **What happens to the reader threads if the command is abandoned?** If a
   bounded read gives up, does the thread leak, get detached, or get cancelled?
   Rust has no thread cancellation — say what actually happens to it and whether
   an abandoned thread can pile up across repeated commands.
7. **Does anything already depend on the current blocking behaviour?** Check the
   tests under `crates/dun-cli/src/tests/command_output.rs` and
   `command_line.rs` before proposing a change to the read loop.

## Explicitly out of scope

- **No source changes.** Not one file. If you believe a change is needed to
  answer a question, describe the experiment instead.
- Making `run_command_capture` asynchronous or moving it off the UI thread. A
  30 s freeze on a slow command is existing, intended behaviour; the defect here
  is the *unbounded* one. Note it if it constrains a decision, do not plan it.
- The plugin resource limits (CPU/memory/FD rlimits) from the review's §4.2.
  Cleanup first; limits are a later, separate question.

## Scope

- Files you MAY modify: **NONE — design only, no source change.**
- Files/areas you MUST NOT touch: everything. Specifically `AGENTS.md`,
  `CLAUDE.md`, `README.md`, `TODO.md`, `docs/**`, `.git`, git config,
  `Cargo.toml`, `Cargo.lock`, `vm-test/**` (local SSH keys), `reference/**`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** If your plan needs `unsafe`, it is the
   wrong plan — see the constraints above for the safe primitives.
2. **The 1 MiB dual-platform size budget is real.** Byte cost is part of the
   plan, not an afterthought.
3. **Name the path that actually executes.** For every proposed test, say which
   production call site it exercises. The recurring failure on this project is
   tests covering a path that does not run.
4. **Plan tests that can fail.** Claude gates every invariant test by mutating
   the implementation and confirming the test fails. State the intended oracle
   for each proposed test, and make it independent of the implementation's own
   logic.
5. **Stop-loss is real.** If the same question defeats you twice, STOP and
   report it as an open question.

## Verification

Design-only: there is no build to run and no green to reach. Verification is
that every claim carries `path:line` evidence you actually read. Do not build,
do not edit, do not format. Read-only inspection commands are fine.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify any file. This brief produces a report only.
- Full machine access, but touch NOTHING outside this repo, no network.
- Every `path:line` in the plan must be real. If you are unsure of a fact, mark
  it explicitly as unverified rather than asserting it.

## Report format (your final message)

1. **Plan** — the numbered step list per Deliverable item 1.
2. **Inventories** — Deliverable items 2 and 3, as tables with `path:line`.
3. **Byte-cost analysis** — Deliverable item 4, with your recommendation.
4. **Open questions** — answers to all seven, each marked `[evidence]` or
   `[judgement call for Claude]`.
5. **Stop-loss** — where you stopped and why (empty if none).
