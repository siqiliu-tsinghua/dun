# Brief 025 — Split `ui_text/status.rs` by domain (i18n slice 4c)

Implementation brief. Pure file surgery: **no behaviour change, no new keys,
no translation work, no test changes.** The i18n extraction is finished
(slices 1–4b); this brief pays off the size debt it left behind.

`crates/dun-cli/src/ui_text/status.rs` is **42,355 bytes / 265 keys**, over the
`35k` "architecture debt" line in `docs/code-organization-guidelines.md`. It
currently runs on an explicit temporary exception recorded there, which says the
exception expires by splitting the file by domain once the status text stops
growing. It has stopped growing. Split it.

## Goal

`ui_text/status.rs` becomes `ui_text/status/` with seven domain modules, each
comfortably under `10k`. Every `const` keeps its **exact current name**, every
call site keeps compiling **untouched**, `ALL` stays a single flat enumeration,
and the whole test suite passes with **zero edits to any test**.

## The acceptance criterion (read this twice)

When you are done, `git status --short` must show changes **only** under
`crates/dun-cli/src/ui_text/`. Nothing else. Not `main.rs`, not a call site,
not a test, not `i18n/zh-CN.conf`.

If anything outside `ui_text/` needs to change, you have broken the re-export
contract — fix the re-exports, not the caller. That is the whole point of the
exercise: the key table is data, and moving data between files must be
invisible to everyone who reads it.

## Context pointers

- Read `AGENTS.md` and `docs/code-organization-guidelines.md` (File Size Policy,
  and the "Explicit temporary exception" section naming this file).
- `crates/dun-cli/src/ui_text/mod.rs` — the machinery. It already does
  `pub(crate) use chrome::*; pub(crate) use status::*;` (glob re-export is what
  keeps `ui_text::SOME_CONST` working from every call site), and assembles
  `#[cfg(test)] ALL` from `chrome::ALL` and `status::ALL` with a `const` loop.
  **`mod.rs` should not need to change at all** — keep `status::ALL` as its
  contract.
- `crates/dun-cli/src/ui_text/chrome.rs` — the shape to copy: a flat list of
  `pub(crate) const NAME: TextKey = ("key", "English");` plus a
  `#[cfg(test)] pub(crate) const ALL: &[TextKey] = &[...];`.
- `ALL` is `#[cfg(test)]` and must stay that way — it must not ship in the
  binary.

## The split

Create `crates/dun-cli/src/ui_text/status/` with `mod.rs` plus seven modules.
The mapping below is **exhaustive** — every one of the 265 keys has a home; do
not invent an eighth module and do not leave a key behind. Assign by the key's
prefix (the string, not the const name):

| Module | Key prefixes | Keys |
| --- | --- | ---: |
| `window.rs` | `status.window.*`, `status.workspace-error.*`, `status.pane.*`, `window.untitled`, `window.untitled-numbered` | 47 |
| `file.rs` | `status.open.*`, `status.save.*`, `status.save-as.*`, `status.reload.*`, `status.file.*`, `status.file-dialog.*`, `status.path-error.*`, `status.atomic-temp.*`, `status.dialog.*`, `status.unsaved.*`, `status.buffer-switcher.*` | 39 |
| `edit.rs` | `status.buffer-error.*`, `status.copy.*`, `status.copy-line.*`, `status.cut.*`, `status.paste.*`, `status.external-copy.*`, `status.delete-line.*`, `status.move-line.*`, `status.indent.*`, `status.outdent.*`, `status.trim.*`, `status.undo.*`, `status.redo.*`, `status.wrap.*`, `status.scroll.*` | 49 |
| `search.rs` | `status.find.*`, `status.replace.*`, `status.replace-all.*`, `status.replacement.*`, `status.search-results.*`, `status.go-to-line.*`, `status.list.*` | 40 |
| `prompt.rs` | `status.prompt.*`, `status.completion.*` | 11 |
| `command.rs` | `status.command.*`, `status.command-line.*`, `status.run.*`, `status.shell.*`, `status.theme.*`, `status.config.*`, `status.config-diagnostics.*`, `status.plugin.*`, `status.help.*`, `status.history.*`, `status.aux-window.*` | 59 |
| `command_output.rs` | `command-output.*` | 20 |

47 + 39 + 49 + 40 + 11 + 59 + 20 = **265**. If your count of any module differs,
you have mis-assigned a key — find it before proceeding, and say so in the
report.

`status/mod.rs`:

- declares the seven modules and re-exports them: `pub(crate) use window::*;`
  and so on for each — this is what keeps `ui_text::STATUS_WINDOW_CLOSED`
  resolving from every call site;
- assembles `#[cfg(test)] pub(crate) const ALL: &[TextKey]` from the seven
  module `ALL` slices, as **one** flat enumeration. Prefer a single nested
  `const` loop over an array of the module slices
  (`const MODULES: [&[TextKey]; 7] = [window::ALL, file::ALL, …]`) rather than
  seven copy-pasted loops. `ui_text/mod.rs` keeps consuming `status::ALL`
  exactly as it does today.

Each domain module keeps its own `#[cfg(test)] pub(crate) const ALL: &[TextKey]`
listing its own keys, and keeps the section comments that currently group those
keys (move them with their keys — do not drop or reword them).

## Rules

- **Move, do not edit.** Const names, key strings, English defaults, and section
  comments are moved verbatim. If you find yourself retyping a string, stop and
  copy it instead. A typo here is a silent English or key regression that the
  completeness test would only catch if it changed the *key*; changing an
  *English default* would pass every test and quietly alter the UI.
- No new keys. No renames. No reordering beyond what the move requires (keep
  each domain's keys in their current relative order).
- No changes to `tr`, `tr_fmt`, `tr_template`, `substitute`, `placeholder_count`.
- No changes to `i18n/zh-CN.conf` — key strings are unchanged, so it stays valid.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
git status --short
```

The suite must pass **with no test edited**, and `git status --short` must list
only files under `crates/dun-cli/src/ui_text/`. Paste both verbatim.

Then prove the move was faithful, mechanically. This is the real check for this
brief — more informative than any mutation, because the failure mode here is not
a broken invariant but a **silently retyped string**.

Write a `python3` script that extracts every `("key", "English default")` pair
from the pre-split file (`git show HEAD:crates/dun-cli/src/ui_text/status.rs`)
and every pair from the new `ui_text/status/*.rs` files, sorts both, and asserts
the two sets are **identical**. Paste the script and its output. If they differ
by even one byte of English, you dropped or retyped something — find it.

Note why this matters: a mistyped *key* would be caught by the completeness test,
but a mistyped *English default* would pass the entire suite and quietly change
what users see.

Also confirm the sizes: every new module under `10k`, and print `wc -c` for the
new directory.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave all changes in
  the working tree; Claude runs the authoritative gate and commits.
  (`git show` and `git status` are read-only and fine.)
- Do NOT modify files outside `crates/dun-cli/src/ui_text/`. If you believe you
  must, STOP and write that in the report instead — it means the re-exports are
  wrong.
- Full machine access, but touch NOTHING outside this repo, no network. The only
  commands you run are file edits within Scope, `cargo`, `git show`/`git status`,
  and `python3` for the table comparison.
- Minimal diff: no drive-by reformatting or renames.
- You MUST paste the real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — the new file list with `wc -c` for each.
2. Verification — each command with verbatim output, including `git status
   --short` and the before/after key-table comparison script and its result.
3. Key counts per module, against the table above.
4. Stop-loss / open questions (empty if none).
