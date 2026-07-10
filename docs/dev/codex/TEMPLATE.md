# Codex Task Brief Template (dun)

Claude owns direction, diagnosis, and target selection; Codex executes one
brief exactly and self-verifies; Claude runs the authoritative gate afterward
and commits. Copy this file to `docs/dev/codex/brief-NNN-<slug>.md`, fill
every section, then dispatch via `scripts/dispatch-brief.sh NNN`.

The dispatcher runs `cdx exec -s danger-full-access` with stdin from
`/dev/null` and logs to `/tmp/dun_cdx_brief_NNN.log`. Full machine access is
the sandbox level, not the permission: the brief's **Scope** and **Hard
rules** are the real fence.

Two brief shapes use this template:

- **Diagnostic brief** — measure/inventory and report; no source change.
- **Implementation brief** — a precise, pre-diagnosed mechanical change with
  a named test gate.

---

## Goal

One paragraph. What must be true when you are done.

## Context pointers

- Read `AGENTS.md` (invariants, engineering rules) and the `docs/` entries
  named below before touching anything.
- Key files for this task: <list with one-line roles>.
- Acceptance is mechanical: the named tests decide, not prose.

## Scope

- Files you MAY modify: <explicit list, or "NONE — diagnostic only">.
- Files/areas you MUST NOT touch (defaults for every brief):
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`,
    `docs/**` (except a doc file this brief explicitly lists as in-scope);
  - `.git`, git config, `Cargo.toml`, `Cargo.lock` (unless the Goal is
    exactly a dependency change);
  - `vm-test/**` (contains local SSH keys), `reference/**`.

## Deliverable

Bullet list of concrete artifacts (for diagnostic: the tables/numbers; for
implementation: code + tests).

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** Every crate root has
   `#![forbid(unsafe_code)]`; if you think `unsafe` is unavoidable, STOP and
   report.
2. **The 1 MiB dual-platform size budget is real.** Claude gates any
   runtime-code change with release builds on macOS AND Debian. Keep diffs
   minimal; no new dependencies; no new `format!`-heavy layers or broad
   generic instantiations in runtime code. Test-only code is exempt (tests
   do not ship).
3. **All untrusted text goes through the sanitizer.** Anything that can
   reach the terminal (buffer text, titles, status, prompts, dialogs) must
   pass `DisplaySanitizer`/the existing sanitized paths. Never print raw
   file/command bytes.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** Modules use
   `use crate::*`; if you move or remove a symbol, the import lists in
   `main.rs` must be updated in the same change.
5. **Tests are layered and colocated.** Unit/behavior tests live in each
   crate's `src/tests/` behavior modules; dun-ui rendering tests use the
   ratatui `TestBackend` snapshot helpers; PTY/tmux tests live in
   `crates/dun-cli/tests/`. Match the local style of the file you extend.
6. **Terminal-detection env is pinned in harnesses.** Any test that spawns
   the editor must pin/clear TERM, COLORTERM, LANG, LC_CTYPE, NO_COLOR (see
   `crates/dun-cli/tests/support/`); a leaked variable makes the test
   environment-dependent.
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report — do not keep tuning.

## Verification (MANDATORY — you run it; iterate to green)

Implementation brief, run exactly these and paste results verbatim:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast   # or the narrower -p suite the brief names
```

Loop: edit → test → fix → rerun, until green. Never claim a result without
the verbatim lines. Note: the tmux-backed suite requires tmux; if it is
unavailable the tests skip cleanly — say so rather than reporting them green.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave all changes
  in the working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network. The
  only commands you run are file edits within Scope, `cargo`, and `python3`
  for parsing output.
- Minimal diff: no drive-by reformatting, renames, or comment changes
  outside the task.
- You MUST paste the real verbatim verification output. If a run did not
  reach green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why (or
   "diagnostic only, no source change").
2. Verification — each command run, with the exact verbatim output lines
   (suite counts; note any environment-dependent skips).
3. The finding / verdict.
4. Stop-loss / open questions — where you stopped and why (empty if none).
