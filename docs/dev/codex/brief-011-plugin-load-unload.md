# Brief 011 — Explicit plugin load/unload commands

Implementation brief. Add user-driven `plugin load` / `plugin unload` command
prompt commands that start and stop the syntax-highlight host process on
demand, so an idle host does not stay resident (memory-friendly). Lazy launch
already exists (the worker only spawns the host on the first job); this brief
adds explicit unload (shut the host down now, stop relaunching) and load
(resume). No automatic idle-unload, no new protocol message, no plugin-id
argument (there is one highlight host today — the first configured
`syntax-highlight` entry).

## Goal

- `plugin unload` gracefully shuts the running highlight host down and stops
  it relaunching on later edits, freeing its memory.
- `plugin load` re-enables it; the next edit relaunches it (lazy).
- `plugin` with no argument reports the current state.
- The behavior is covered by unit tests and the full feature trail (help text,
  docs) is updated. `cargo test --workspace` is green.

## Context pointers

- Read `AGENTS.md` first — note the feature-trail rule (code + help + docs +
  tests for any user-visible feature) and `#![forbid(unsafe_code)]`.
- `crates/dun-cli/src/plugins.rs` — `PluginHighlighter` (sender side:
  `jobs: mpsc::Sender<HighlightJob>`, `last_request` dedupe, `schedule`,
  `poll`, `plugin_id`, `from_entries`, and a `#[cfg(test)] for_tests`) and
  `highlight_worker` (owns `Option<HostClient>`, lazily launches on the first
  job, relaunches after `RELAUNCH_COOLDOWN` on failure). This is the core
  change.
- `crates/dun-plugin/src/client.rs` — `HostClient::shutdown(self)` sends the
  `Shutdown` message and consumes the client (graceful stop); `Drop` also
  kills. Use `shutdown` for unload.
- `crates/dun-cli/src/app/command_line.rs` — `run_command_line` matches the
  first token (`"theme"`, `"config"`, `"goto"`, …) and dispatches to
  per-command handlers that take `args: &[String]`. Add a `"plugin"` arm in
  the same style. `run_no_arg_command` and the existing handlers show the
  status-setting pattern.
- `crates/dun-cli/src/tests/plugins.rs` — existing worker/highlighter tests,
  including `schedule_dedupes_identical_snapshots_and_sends_changed_ones`
  (uses `for_tests`) — update these for the new channel type.
- Help/command reference: `crates/dun-cli/src/help/content.rs` (where prompt
  commands are documented). README and `docs/plugin-protocol.md` for the
  user-facing paragraph.

## Specification

Worker message channel (`plugins.rs`):

- Introduce `enum WorkerMessage { Job(HighlightJob), Load, Unload }` and change
  the channel from `HighlightJob` to `WorkerMessage` (both `from_entries` and
  `for_tests`).
- `PluginHighlighter::schedule` sends `WorkerMessage::Job(job)` (dedupe logic
  unchanged).
- Add `PluginHighlighter::unload(&mut self)`: send `WorkerMessage::Unload` and
  **reset `self.last_request = None`** so that after a later `load` the same
  visible snapshot is re-sent (otherwise the dedupe key would suppress the
  re-highlight). Track a local `unloaded: bool` for state reporting; return or
  expose it via an `is_loaded()` accessor for the command handler's status.
- Add `PluginHighlighter::load(&mut self)`: send `WorkerMessage::Load`, clear
  the local `unloaded` flag, and reset `last_request = None` so the next
  scheduled snapshot is not deduped away.
- `highlight_worker`: keep a `client: Option<HostClient>` and add an
  `unloaded: bool` (default false). Each wake, **drain the whole backlog**
  before acting so control messages are never lost and jobs still coalesce:
  block on `messages.recv()`, then `while let Ok(m) = messages.try_recv()`,
  folding into `unloaded`/newest-job as you go —
  - `WorkerMessage::Unload` → if a client is live, `client.take().shutdown()`
    (ignore the result); set `unloaded = true`.
  - `WorkerMessage::Load` → set `unloaded = false`.
  - `WorkerMessage::Job(j)` → keep as the newest pending job (overwrite).
  After draining: if `unloaded`, drop the job (do nothing); otherwise run the
  newest job through the existing lazy-launch + request path unchanged. A
  `recv` error (channel closed) still exits the loop.

Command prompt (`command_line.rs`):

- Add `"plugin" => self.run_plugin_command(args)` to the `run_command_line`
  match.
- `run_plugin_command(&mut self, args: &[String])`:
  - no args → report state: if `self.highlighter` is `None`, status
    "No syntax-highlight plugin configured"; else
    "Plugin <id> is loaded" / "… is unloaded" per `is_loaded()`.
  - `"unload"` → if a highlighter exists, call `unload()` and set status
    "Plugin <id> unloaded"; else the not-configured status.
  - `"load"` → if a highlighter exists, call `load()` and set status
    "Plugin <id> loaded (starts on the next edit)"; else not-configured.
  - any other arg → status "Usage: plugin [load|unload]".
  Match the sanitized-status conventions of the neighboring handlers.

Feature trail:

- Add the `plugin`, `plugin load`, `plugin unload` commands to the help/command
  reference in `help/content.rs` alongside the other prompt commands.
- Add a short paragraph to `README.md` (near the plugin description) and to
  `docs/plugin-protocol.md` noting that a configured highlight host launches
  lazily and can be unloaded/reloaded from the command prompt to free memory.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/plugins.rs`;
  - `crates/dun-cli/src/app/command_line.rs`;
  - `crates/dun-cli/src/tests/plugins.rs` (update for the channel type; add
    load/unload worker tests);
  - `crates/dun-cli/src/help/content.rs`;
  - `README.md`, `docs/plugin-protocol.md` (the feature-trail doc updates —
    explicitly in scope for this brief).
- Files/areas you MUST NOT touch:
  - `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `PROGRESS.md`, `TODO.md`, other
    `docs/**`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock` (no dependency changes);
  - `crates/dun-plugin/**` (the client's `shutdown` is already there),
    `crates/dun-ui/**`, `crates/dun-core/**`, `crates/dun-config/**`
    (no new `EditorCommand` variant — this is a prompt-only command like
    `theme`/`goto`, not keybound);
  - `vm-test/**`, `reference/**`, `hosts/**`.

## Deliverable

- The worker message channel + `load`/`unload`/state on `PluginHighlighter`.
- The `plugin` command-prompt handler.
- Tests in `tests/plugins.rs`:
  1. update `schedule_dedupes_…` and `for_tests` to the `WorkerMessage`
     channel (assert on `WorkerMessage::Job(..)`);
  2. `unload_then_load_resets_dedupe_so_next_snapshot_resends` — after
     `unload()` then `load()`, an identical snapshot that would otherwise be
     deduped is sent again;
  3. a worker-level test (using `for_tests` + a fake job) that an `Unload`
     message makes a subsequent `Job` produce no launch/outcome, and a `Load`
     re-enables it. If a worker-level test needs host process control, keep it
     to the message-plumbing level (no real host) — the existing tests show
     the seam.
- Full feature trail updated (help, README, docs).

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]`; STOP if you
   think you need `unsafe`.
2. **The 1 MiB dual-platform size budget is real.** Claude runs the size gate;
   keep the diff minimal, no new deps, no `format!`-heavy layers beyond the
   handful of status strings.
3. **All untrusted text goes through the sanitizer.** Status messages use the
   plugin id (already validated config) — keep them within the existing
   sanitized status path; do not print raw host output here.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** If you add a
   `pub(crate)` item that main.rs re-exports (e.g. `WorkerMessage` used in
   tests), update the import lists in the same change.
5. **Tests are layered and colocated.** New tests live in
   `crates/dun-cli/src/tests/plugins.rs`.
6. **Terminal-detection env is pinned in harnesses.** Not relevant here (no
   process/PTY spawn in these unit tests).
7. **Stop-loss is real.** If a test fails twice for the same reason after a
   genuine fix, STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Loop until green; paste the verbatim output lines (suite counts). Note any
tmux/PTY skips honestly.

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
2. Verification — each command with verbatim output lines.
3. The finding / verdict.
4. Stop-loss / open questions (empty if none).
