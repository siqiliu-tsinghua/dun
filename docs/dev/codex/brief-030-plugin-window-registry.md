# brief-030: plugin window ownership registry (pure bookkeeping)

Implementation brief. Slice B, first mechanical package. Claude owns the
security spine (live grant, enforcement) and the workspace wiring; this brief
delivers only the isolated ownership bookkeeping those will call.

## Goal

A new `dun-cli` module `plugin_windows` provides `PluginWindows`, a pure
bookkeeping layer tracking which windows each plugin owns. It enforces two
invariants from `docs/plugin-protocol.md` ("Ownership and namespacing"): a
per-plugin cap of at most two windows, and own-only destruction (a plugin may
only close a window it opened, never another plugin's). It holds no workspace,
buffer, or rendering state — it operates on opaque `WindowId` values the caller
passes in. It is not yet wired to the real window API; that grant-gated wiring
lands in a later slice, so the module is `#[allow(dead_code)]` scaffolding for
now. When done, the named tests pass and the workspace stays green.

## Context pointers

- Read `AGENTS.md` (invariants, engineering rules) and
  `docs/plugin-protocol.md` (the "Capability Model" and "Ownership and
  namespacing" sections) before touching anything.
- Key files:
  - `crates/dun-cli/src/plugin_windows.rs` — NEW, the whole deliverable.
  - `crates/dun-cli/src/main.rs` — the prelude/module hub; add one `mod` line.
  - `crates/dun-cli/src/plugins.rs` — sibling plugin module, for style only
    (do not modify).
  - `crates/dun-core/src/lib.rs` — re-exports `WindowId` (import from
    `dun_core`).
- Acceptance is mechanical: the named tests decide, not prose.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/plugin_windows.rs` (create it);
  - `crates/dun-cli/src/main.rs` — ONLY to add `mod plugin_windows;` to the
    existing `mod` list (it currently runs `mod plugins;` on line ~44; place
    the new line adjacent). Do not touch the prelude `use` lists or anything
    else in `main.rs`.
- Files/areas you MUST NOT touch (defaults for every brief):
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`,
    `docs/**` (except this brief file if you must note something — prefer not);
  - `.git`, git config, `Cargo.toml`, `Cargo.lock`;
  - `vm-test/**`, `reference/**`;
  - any `dun-core`, `dun-plugin`, `dun-config`, `dun-term`, `dun-ui` source,
    and any `dun-cli` file other than the two above.

## Deliverable

`crates/dun-cli/src/plugin_windows.rs` containing exactly:

- A module doc comment stating it is per-plugin window ownership bookkeeping
  per `docs/plugin-protocol.md`, holds no workspace state, and is not yet
  wired (hence the temporary `#[allow(dead_code)]`, removed when a later slice
  wires it).
- `#![allow(dead_code)]` at the top of the module, with a one-line comment
  that it is temporary scaffolding until the grant-gated workspace wiring
  lands.
- `use dun_core::WindowId;`
- `pub(crate) const MAX_WINDOWS_PER_PLUGIN: usize = 2;`
- `pub(crate) struct PluginWindows` (derive `Debug, Default`) holding the
  ownership map. Use a simple owned structure (e.g. a `Vec` of per-plugin
  entries keyed by `plugin_id: String`); no `HashMap` (keep it frugal — the
  plugin count is tiny). Windows are `WindowId`.
- Methods (all `pub(crate)`), with the exact semantics below:
  - `fn count(&self, plugin_id: &str) -> usize` — how many windows this plugin
    currently owns.
  - `fn can_open(&self, plugin_id: &str) -> bool` — `count < MAX`.
  - `fn record_opened(&mut self, plugin_id: &str, window: WindowId) -> bool` —
    if the plugin is under the cap AND does not already own `window`, record
    it and return `true`; otherwise record nothing and return `false`.
  - `fn owns(&self, plugin_id: &str, window: WindowId) -> bool` — whether that
    exact plugin owns that exact window.
  - `fn record_closed(&mut self, plugin_id: &str, window: WindowId) -> bool` —
    if that plugin owns `window`, remove it and return `true`; otherwise
    remove nothing and return `false`. It MUST NOT remove a window owned by a
    different plugin, even when the `WindowId` matches.
  - `fn take_all(&mut self, plugin_id: &str) -> Vec<WindowId>` — remove and
    return all of that plugin's windows (for the caller to close on unload or
    crash); leaves that plugin owning none and other plugins untouched.
- A `#[cfg(test)] mod tests` covering, at minimum:
  1. **cap**: two `record_opened` for one plugin return `true`; a third
     returns `false`; `can_open` is `true`, `true`, `false` across the
     sequence; `count` ends at 2.
  2. **own-only isolation** (the security invariant): plugin `"a"` opens `w1`;
     `owns("a", w1)` is true and `owns("b", w1)` is false; `record_closed("b",
     w1)` returns `false` and `"a"` still owns `w1`; then `record_closed("a",
     w1)` returns `true` and `count("a") == 0`.
  3. **reap**: a plugin owning two windows; `take_all` returns both (assert
     the set, order-independently), `count` is then 0, and a second plugin's
     windows are untouched.
  4. **duplicate**: `record_opened` of the same `(plugin, window)` twice — the
     second returns `false` and `count` stays 1.

Construct `WindowId` values in tests as `WindowId(1)`, `WindowId(2)`, … (the
field is public: `pub struct WindowId(pub u64)`).

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** Every crate root has
   `#![forbid(unsafe_code)]`; no `unsafe` here.
2. **The 1 MiB dual-platform size budget is real.** This module is unwired, so
   fat LTO strips it and the delta must be zero — do not add dependencies, do
   not reach for `HashMap`/`format!`/generics. Keep it a small `Vec`.
3. **No untrusted text reaches the terminal here** — this module prints
   nothing; it only bookkeeps ids. Do not add any I/O or rendering.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** Your only change there
   is adding `mod plugin_windows;` to the module list. Do not add prelude
   `use` entries (nothing consumes the type yet) and do not reorder existing
   lines.
5. **Tests are colocated.** Put the tests in `#[cfg(test)] mod tests` inside
   `plugin_windows.rs`, matching the style of `crates/dun-cli/src/plugins.rs`.
6. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report — do not keep tuning.

## Verification (MANDATORY — you run it; iterate to green)

Run exactly these and paste results verbatim:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dun-cli plugin_windows
cargo test --workspace --no-fail-fast
```

Loop: edit → test → fix → rerun, until green. The `plugin_windows` filter must
show your new tests running (not "0 filtered out to 0"). Never claim a result
without the verbatim lines. The tmux-backed suite needs tmux; if unavailable it
skips cleanly — say so rather than reporting it green.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave all changes in
  the working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network. The
  only commands you run are file edits within Scope, `cargo`, and `python3`
  for parsing output.
- Minimal diff: no drive-by reformatting, renames, or comment changes outside
  the task.
- You MUST paste the real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command run, with the exact verbatim output lines
   (suite counts; note any environment-dependent skips).
3. The finding / verdict.
4. Stop-loss / open questions — where you stopped and why (empty if none).
