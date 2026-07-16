# brief-031: plugin menu contribution model + validation + label i18n

Implementation brief. Slice C, first mechanical package. Claude owns the
security spine (a new `EditorCommand` plugin-action variant, the menu-invoke
dispatch, the grant-gated window open path) and the menu-bar wiring; this brief
delivers only the isolated, typed menu-contribution model those will consume.

## Goal

A new `dun-plugin` module `menu` provides the typed model for a plugin's menu
contribution, parsed and validated from a protocol payload, plus locale label
resolution. It mirrors `validate.rs` (the `overlay-write` capability's
validator): `menu.rs` is the `menu` capability's contribution validator. A
plugin contributes exactly one top-level entry with a bounded, flat list of
items (`docs/plugin-protocol.md`, "Capability Model" / "Menu label i18n");
every label is a locale-tag map that must include the `en_US` fallback. The
module is pure (only `std` + the crate's `Json`), holds no editor or rendering
state, and — because its items are `pub` — needs no `#[allow(dead_code)]`. When
done, the named tests pass and the workspace stays green.

## Context pointers

- Read `AGENTS.md` (invariants, engineering rules) and the "Capability Model"
  and "Menu label i18n" sections of `docs/plugin-protocol.md` before touching
  anything.
- Key files:
  - `crates/dun-plugin/src/menu.rs` — NEW, the whole deliverable.
  - `crates/dun-plugin/src/validate.rs` — the pattern to mirror: a
    parse-and-validate module returning `Result<_, &'static str>`, with a
    `#[cfg(test)] mod tests` using `json::obj`/`json::str`/`Json::Arr`.
    **Read it first.**
  - `crates/dun-plugin/src/json.rs` — the JSON API: `Json::Obj(Vec<(String,
    Json)>)` (a `pub` variant — destructure it to iterate a tag→label map,
    since keys are unknown locale tags), `Json::get`, `Json::as_str`,
    `Json::as_arr`; test builders `json::obj`, `json::str`, `Json::Arr`.
  - `crates/dun-plugin/src/lib.rs` — add `pub mod menu;` and a re-export line.
- Acceptance is mechanical: the named tests decide, not prose.

## Scope

- Files you MAY modify:
  - `crates/dun-plugin/src/menu.rs` (create it);
  - `crates/dun-plugin/src/lib.rs` — ONLY to add `pub mod menu;` (alongside the
    other `pub mod` lines) and a `pub use menu::{...}` re-export next to the
    existing `pub use` lines.
- Files/areas you MUST NOT touch (defaults for every brief):
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`, `docs/**`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock`;
  - `vm-test/**`, `reference/**`;
  - every other source file in every crate (including all other `dun-plugin`
    files besides `menu.rs` and the two `lib.rs` lines).

## Deliverable

`crates/dun-plugin/src/menu.rs` containing:

- A module doc comment: this is the `menu` capability's contribution validator,
  mirroring `validate.rs`; labels are locale-tag maps with a required `en_US`
  fallback; the module enforces a structural no-control-character bound but the
  full `DisplaySanitizer` runs later at render time (not here).
- `use crate::json::Json;`
- Public constants:
  - `pub const MENU_MAX_ITEMS: usize = 16;`
  - `pub const MENU_MAX_LABEL_CHARS: usize = 64;`
  - `pub const MENU_MAX_ACTION_ID_CHARS: usize = 64;`
  - `pub const MENU_FALLBACK_TAG: &str = "en_US";`
- `pub struct LabelSet` wrapping a `Vec<(String, String)>` (locale tag → label;
  small, linear lookup — NO `HashMap`). Methods:
  - `pub fn resolve(&self, active_tags: &[String]) -> &str` — the first tag in
    `active_tags` present in the set returns its label; otherwise the
    `MENU_FALLBACK_TAG` label (validation guarantees it is present, so this
    never panics). Return `&str` borrowed from `self`.
  - a private `fn get(&self, tag: &str) -> Option<&str>`.
- `pub struct PluginMenuItem { pub label: LabelSet, pub action_id: String }`.
- `pub struct PluginMenu { pub top_label: LabelSet, pub items: Vec<PluginMenuItem> }`
  with `pub fn from_payload(payload: &Json) -> Result<Self, &'static str>`.
- `from_payload` parses this shape and validates as follows (return a distinct
  `&'static str` message per failure):
  - `payload.get("top_label")` must be a `Json::Obj`; parse it into a
    `LabelSet`.
  - `payload.get("items")` must be a `Json::as_arr` of length `1..=MENU_MAX_ITEMS`
    (reject empty and reject `> MENU_MAX_ITEMS`).
  - each item is a `Json::Obj` with `label` (a `Json::Obj` → `LabelSet`) and
    `action_id` (a `Json::as_str`).
  - `action_id`: non-empty, `<= MENU_MAX_ACTION_ID_CHARS` chars, and every char
    `is_ascii_graphic()` (no spaces, no control bytes).
  - `LabelSet` parsing (shared for top and item labels): each object entry is a
    tag (non-empty key) → label string (`as_str`); reject a duplicate tag; each
    label is `1..=MENU_MAX_LABEL_CHARS` chars (count `chars()`, not bytes) and
    contains no control character (`char::is_control`); the set MUST contain a
    non-empty `MENU_FALLBACK_TAG` entry.
- A `#[cfg(test)] mod tests` covering, at minimum:
  1. **accepts** a valid menu (top label with `en_US` + `zh-CN`, two items each
     with `en_US` labels and distinct `action_id`s); assert `items.len() == 2`
     and an `action_id`.
  2. **resolve prefers an active tag**: a `LabelSet` with `en_US` and `zh-CN`,
     `resolve(&["zh-CN".into()])` returns the `zh-CN` label.
  3. **resolve falls back to `en_US`**: same set, `resolve(&["fr".into()])`
     returns the `en_US` label; `resolve(&[])` returns the `en_US` label.
  4. **rejects**, each its own assertion: a top label missing `en_US`; an item
     label missing `en_US`; a label longer than `MENU_MAX_LABEL_CHARS`; a label
     containing a control character (e.g. `"a\nb"` or `"x\u{1b}y"`); more than
     `MENU_MAX_ITEMS` items; zero items; an empty `action_id`; an `action_id`
     with a space; a duplicate tag in a label set.

Build `Json` test inputs with `json::obj([...])`, `json::str(...)`, and
`Json::Arr(vec![...])` as `validate.rs` tests do.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]`; no `unsafe`.
2. **The 1 MiB dual-platform size budget is real.** No dependencies, no
   `HashMap`, no `format!` in non-test code, no generics. Use `Vec` + linear
   scans; the maps are tiny.
3. **Untrusted text.** Labels and action ids come from an untrusted host. This
   module enforces a structural bound only (length + no control characters);
   do NOT try to render or print them, and do NOT call any sanitizer here — the
   full `DisplaySanitizer` runs at render time in the wiring Claude adds later.
4. **`lib.rs` is the crate root.** Add `pub mod menu;` with the other `pub mod`
   lines and one `pub use menu::{...}` with the other re-exports; change nothing
   else there. (This is `dun-plugin`'s `lib.rs`, not the `dun-cli` prelude hub.)
5. **Tests are colocated.** Put tests in `#[cfg(test)] mod tests` inside
   `menu.rs`, matching `validate.rs`'s test style.
6. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

Run exactly these and paste results verbatim:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dun-plugin menu
cargo test --workspace --no-fail-fast
```

Loop: edit → test → fix → rerun, until green. The `menu` filter must show your
new tests running. Never claim a result without the verbatim lines. The
tmux-backed suite needs tmux; if unavailable it skips cleanly — say so rather
than reporting it green.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave changes in the
  working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network. The
  only commands you run are file edits within Scope, `cargo`, and `python3`.
- Minimal diff: no drive-by reformatting, renames, or comment changes outside
  the task.
- You MUST paste the real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command run, with the exact verbatim output lines.
3. The finding / verdict.
4. Stop-loss / open questions — where you stopped and why (empty if none).
