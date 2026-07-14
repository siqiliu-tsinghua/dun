# Brief 023 — Status helper i18n (slice 4b-1: the stateless text builders)

Implementation brief, and the direct successor of brief-022. That brief
converted the 172 status call sites whose text is a literal in `app/*.rs`,
and reported 50 call sites it could not convert because the sentence is
built by a **helper function** that has no catalog. This brief gives those
helpers a catalog.

It is mechanical, but it is not blind: four decisions below are already
made. Do not re-litigate them, and do not invent a fifth.

## The rule this slice runs on

**Vocabulary the user types stays English; the prose around it
translates.** This is the same invariant as menu mnemonics (`(F)`) and
help key caps (`Enter`, `Ctrl+X`) — a translation must never change a
string the user has to *type back*.

Concretely, these stay English and are passed as `{}` arguments, never
keyed:

- theme names (`theme_command_values()` — they are config values);
- config-diagnostics section tokens (`config_diagnostics_section_values()`
  and `ConfigDiagnosticsSection::label()` — the command line parses those
  exact tokens in `parse_config_diagnostics_section`);
- command ids and the command-name list inside `COMMAND_LINE_HELP`;
- file paths, buffer names, queries, counts.

## Goal

Every status message deferred by brief-022 whose text is composed by a
helper in `help/`, `command_line.rs`, `terminal/shell.rs`, `files/`,
`config_loading.rs`, or `dialogs/prompt.rs` is looked up through
`ui_text::tr`/`tr_fmt`, has a key in the `ui_text` table and `ALL`, and has
a zh-CN translation. With an empty catalog every message is
**byte-identical** to today. `ui_text.rs` is split before it grows.

## Context pointers

- Read `AGENTS.md` and `docs/i18n.md` first.
- `docs/dev/codex/brief-022-status-message-i18n.md` — the predecessor; its
  key naming (`status.<domain>.<slug>`), template rule, and completeness
  test are what you extend.
- `crates/dun-cli/src/ui_text.rs` — `TextKey`, `tr`, `tr_fmt`,
  `tr_template`, `substitute`, `placeholder_count`, and the `#[cfg(test)]
  `ALL` list. Every new key goes in the table and in `ALL`.
- `crates/dun-cli/src/tests/i18n.rs` —
  `shipped_zh_catalog_translates_all_dialog_chrome` (completeness +
  placeholder-mismatch, walks `ALL`) and `ui_text_keys_are_unique` cover
  your new keys automatically.
- The helpers to convert, with their current homes:
  - `help/status.rs`: `buffer_error_text`, `workspace_error_text`,
    `axis_name`, `replacement_status_text`;
  - `command_line.rs`: `command_line_parse_error_text`,
    `COMMAND_LINE_HELP`;
  - `terminal/shell.rs`: `command_run_status`, `exit_status_text`;
  - `files/open.rs`: `opened_file_status`, `reloaded_file_status`;
  - `files/atomic.rs`: `status_with_atomic_temp_report`;
  - `config_loading.rs`: `ConfigSource::status_text`;
  - `dialogs/prompt.rs`: `PromptCompletionState::status_text`.
- Their call sites are the ones brief-022 listed as deferred (see its
  report section in the log, or just follow the compiler after you change
  a signature). `AppState` always has `&self.shell.catalog`.

## Decisions already made (do not change)

1. **`COMMAND_LINE_HELP` must be split.** It is 276 bytes — over the
   catalog's 256-byte per-value cap, so it cannot be a single key. Apply
   the vocabulary rule: keep the command-name list as a compiled-in
   `&'static str` constant, and key only the prose around it, e.g.
   `("status.command-line.help", "Commands: {}, or any command id such as {}")`
   with the list and the example id as arguments. The rendered English
   string must be byte-identical to today's.
2. **`ConfigDiagnosticsSection::label()` is NOT translated.** It returns
   the section token the user types (`summary`, `keymap`, …) and sits next
   to the English `config_diagnostics_section_values()` list. It stays a
   `&'static str`; only the surrounding status prose gets a key, with the
   label as a `{}` argument. Same for `theme_command_values()`.
3. **The i18n loader's own diagnostic stays English.** When a translation
   file fails to load there is no catalog to translate the complaint with;
   an English diagnostic is the only honest output. Leave
   `i18n_loading.rs` and the bootstrap/reload diagnostic path alone. (The
   `ConfigSource::status_text` part of that same reload message *is*
   translated — only the appended i18n diagnostic stays English.)
4. **Duration and exit formatting.** In `command_run_status`, translate the
   sentence frames (`"Command returned {} in {}"`,
   `"Command timed out after {} and was killed"`) and `exit_status_text`'s
   `"exit {}"` / `"terminated"`. Do **not** touch `duration_status_text` —
   `1.20s` / `350ms` are numeric formats, not prose.

## Split `ui_text.rs` before you grow it

`ui_text.rs` is 33,411 bytes. `docs/code-organization-guidelines.md` puts
20k–35k in the split-plan range ("any change touching the file should state
the split boundary or start the split"), and this brief adds keys. So:
start the split, as part of this change:

```
crates/dun-cli/src/ui_text/
  mod.rs      // TextKey, tr, tr_fmt, tr_template, substitute,
              // placeholder_count, and ALL (assembled from the modules)
  chrome.rs   // prompt/confirm/switcher/dialog/window-title keys (slice 3)
  status.rs   // status.* keys (slices 4a + 4b)
```

`ALL` stays a single flat enumeration (concatenate the modules' own lists);
the completeness and uniqueness tests must keep working unchanged. Re-export
so existing `ui_text::CONST_NAME` call sites keep compiling — a rename of a
public const is a drive-by change and is out of scope.

## Specification

- Helper signatures take `&TextCatalog` and return the translated text.
  Keep them where they live; do not move logic between modules.
- New keys follow `status.<domain>.<slug>` in the `[a-z0-9._-]` charset,
  grouped with a section comment.
- Every dynamic value becomes a `{}` argument via `tr_fmt`. English
  templates must reproduce today's output exactly.
- Multi-branch text (e.g. UTF-8 vs escaped-bytes open status) gets one key
  per branch, not string surgery.
- zh-CN: add the new keys to `i18n/zh-CN.conf` in the existing style, under
  the existing `# Status messages` section or a new sub-comment. Values must
  satisfy the loader (single line, ≤256 bytes, nothing the sanitizer would
  escape) and keep the exact placeholder count of the English template — the
  existing tests enforce both.
- Tests: extend `crates/dun-cli/src/tests/i18n.rs` with at least two
  behavior tests that drive a real path through a converted helper (for
  example a window command that fails with a `WorkspaceError`, and a
  successful file open), asserting the byte-exact English baseline **and**
  the zh output. The `ALL` completeness/uniqueness tests must stay green.

## Explicitly NOT in this brief (brief-024 will do them)

Leave these alone; they need a typed-message refactor, not catalog
threading, and mixing the two would make both unreviewable:

- `files/save.rs`: `path_error_detail` / `path_io_error` — the text is
  carried inside an `io::Error` across `io::Result` boundaries where no
  catalog exists.
- `dialogs/file_dialog.rs`: `FileDialogState::message` — English text
  *stored in state* by methods that have no catalog.
- `"Untitled"` window titles (`dun-core` owns the string; `dun-core` cannot
  depend on `dun-config`, where `TextCatalog` lives).

If you find yourself needing any of them to finish a call site, **stop at
that call site**, leave it English, and list it in your report.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/ui_text.rs` → `crates/dun-cli/src/ui_text/` (split
    as specified);
  - `crates/dun-cli/src/help/status.rs`, `crates/dun-cli/src/command_line.rs`,
    `crates/dun-cli/src/terminal/shell.rs`, `crates/dun-cli/src/files/open.rs`,
    `crates/dun-cli/src/files/atomic.rs`,
    `crates/dun-cli/src/config_loading.rs`,
    `crates/dun-cli/src/dialogs/prompt.rs` (the named helpers only);
  - `crates/dun-cli/src/app/*.rs` (call sites passing the catalog);
  - `crates/dun-cli/src/main.rs` (import lists must follow);
  - `crates/dun-cli/src/tests/*.rs` (new i18n tests; other test files only
    where a signature change forces it — say so in the report);
  - `i18n/zh-CN.conf`.
- Files/areas you MUST NOT touch:
  - `crates/dun-cli/src/files/save.rs`,
    `crates/dun-cli/src/dialogs/file_dialog.rs`,
    `crates/dun-cli/src/help/text.rs` (the section labels stay English);
  - `crates/dun-core/**`, `crates/dun-ui/**`, `crates/dun-config/**`,
    `crates/dun-term/**`, `crates/dun-plugin/**`;
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`,
    `docs/**`;
  - `.git`, git config, any `Cargo.toml`, `Cargo.lock` — no new
    dependencies;
  - `vm-test/**`, `reference/**`, `hosts/**`, `scripts/**`.

## Deliverable

- Converted helpers + their call sites; the `ui_text/` split; new keys in
  the table and `ALL`; zh-CN translations; the behavior tests.
- In your report: call sites converted, keys added, and any call site you
  had to leave English (with the reason).

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **The 1 MiB dual-platform size budget is real.** Flat const data only —
   no macros generating code, no new abstraction layers. Claude measures
   both platforms at the gate.
3. **English output must be byte-identical with an empty catalog.** Hundreds
   of existing tests assert exact status strings; they are the fence. If one
   fails, your conversion changed behavior — fix the conversion, not the
   test.
4. **`main.rs` is the prelude hub.** Modules use `use crate::*`; if a symbol
   moves (the `ui_text` split!), the import lists in `main.rs` follow in the
   same change.
5. **Borrow-checker pattern:** composing with `&self.shell.catalog` inside a
   `&mut self` call needs the `String` bound to a local first. Existing
   converted sites show the shape.
6. **Stop-loss is real.** Same step failing twice for the same reason → STOP
   and report. If the work runs long, finish whole helpers (helper +
   all its call sites + its keys + its zh entries) and report how far you
   got with everything green, rather than leaving a helper half-converted.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Loop until green. The tmux-backed suite needs tmux; if it is unavailable the
tests skip cleanly — say so rather than reporting them green.

Then prove one new behavior test is load-bearing: point one converted helper
at the wrong key, confirm the test fails, and restore it. Restore by
reversing your edit — **do not `git checkout` the file**, the working tree
holds all of your uncommitted work.

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

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command with verbatim output (suite counts; note any
   environment-dependent skips), including the mutant run.
3. Counts: helpers converted, call sites touched, keys added, zh entries
   added; anything left English and why.
4. Stop-loss / open questions (empty if none).
