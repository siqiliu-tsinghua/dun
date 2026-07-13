# Brief 022 — Status message i18n (slice 4a: AppState call sites)

Implementation brief. i18n slices 1–3 landed (menus, help window, dialog
chrome; see `docs/i18n.md`). What remains English is the status line: every
`set_status`/`status_message` string composed inline in `app/*.rs`. This
brief converts those call sites to the existing catalog mechanism. It is a
large but strictly mechanical transformation: the mechanism, the key table
pattern, the template rule, and the completeness-test pattern all already
exist — you are extending them, not designing anything.

## Goal

Every user-visible status message whose text is composed as a literal
`format!`/string **directly at a call site in `crates/dun-cli/src/app/`**
(plus `PromptKind::label()`/`name()` text) is looked up through
`ui_text::tr`/`ui_text::tr_fmt`, has a key declared once in
`crates/dun-cli/src/ui_text.rs`, and has a Simplified Chinese translation in
`i18n/zh-CN.conf`. With an empty catalog the English output is
byte-identical to today. Out of scope (a later brief threads catalogs
through their signatures): strings composed inside `dialogs/*.rs`,
`files/*.rs`, `help/*.rs`, `command_line.rs`, `command_output/*.rs`,
`plugins.rs`, and `terminal/*.rs` helpers — if a `set_status` call site
passes through a string built by such a helper (e.g.
`opened_file_status(...)`, `buffer_error_text(...)`,
`workspace_error_text(...)`, `path_error_detail(...)`), leave that call
site alone and list it in your report under "deferred to 4b".

## Context pointers

- Read `AGENTS.md` and `docs/i18n.md` first. The i18n design doc defines
  the rules you must not re-invent.
- `crates/dun-cli/src/ui_text.rs` — the single-source key table:
  `TextKey = (key, English default)`, `tr`, `tr_fmt`, `tr_template`,
  `substitute`, `placeholder_count`, and the `#[cfg(test)] ALL` list the
  completeness test enumerates. **Every new key goes here, and into `ALL`.**
- `crates/dun-cli/src/tests/i18n.rs` —
  `shipped_zh_catalog_translates_all_dialog_chrome` is the completeness +
  placeholder-mismatch test over `ui_text::ALL`; it will automatically
  cover your new keys. `tr_fmt_substitutes_and_survives_broken_templates`
  shows the template semantics.
- `crates/dun-cli/src/app/` — the call sites. `AppState` always has the
  catalog at hand as `&self.shell.catalog`.
- `crates/dun-cli/src/dialogs/prompt.rs` — `PromptKind::label()` and
  `PromptKind::name()`: `const fn -> &'static str` used as status-message
  prefixes ("Find: no matches for …") and names ("Find cancelled"). These
  two become catalog lookups (see Specification).
- `i18n/zh-CN.conf` — the shipped reference translation. Section comments
  group keys; keep that style.

## Specification

### 1. Key naming and the table

- Keys are `status.<domain>.<slug>` in the existing charset
  `[a-z0-9._-]` — e.g. `status.save.failed`, `status.find.no-matches`,
  `status.window.split-failed`. Choose short, greppable slugs; group
  related consts together in `ui_text.rs` with a section comment, and
  append every one to `ALL`.
- Dynamic values (paths, buffer names, queries, counts, key sequences,
  command ids, error details) become `{}` arguments via `tr_fmt`. The
  English default template must reproduce today's output exactly.
- Prompt-kind prefixes: add per-kind `label` and `name` TextKeys (10 keys,
  e.g. `prompt.find.label` = "Find: ", `prompt.find.name` = "Find";
  ReplaceFind and ReplaceWith share the Replace name as today). Replace
  `PromptKind::label()`/`name()` with methods that take
  `&TextCatalog` and return the translated `&str` (keep the doc comment
  explaining the label is a sentence opener). Update every caller. The
  `#[cfg(test)] status_text` helpers may keep English literals.
- Multi-branch messages (e.g. a message that says "shown"/"hidden") get
  one key per branch, not string surgery.

### 2. What must NOT change

- Message *content*: with an empty catalog, every status message is
  byte-identical to before. No rewording, no punctuation changes, no
  "improvements". If today's text looks wrong, report it, don't fix it.
- The sanitizer, the catalog loader, the template mechanism, menus, help,
  dialog-chrome keys — all frozen. You extend `ui_text.rs` and call
  sites only.
- Anything the Goal lists as out of scope. When in doubt whether a helper
  composed the string: if the full sentence text is not a literal at the
  `app/*.rs` call site, defer it.

### 3. zh-CN translations

- Add a `# Status messages` section to `i18n/zh-CN.conf` translating every
  new key. Match the tone of the existing zh entries (simplified Chinese,
  fullwidth punctuation where natural, key names/ids/paths stay as `{}`
  substitutions). Values must satisfy the loader (single line, ≤256 bytes,
  nothing the sanitizer would escape) and must keep the exact placeholder
  count of the English template — the existing tests enforce both.

### 4. Tests

- Extend the i18n test module (`crates/dun-cli/src/tests/i18n.rs`):
  - a uniqueness test: no duplicate key strings in `ui_text::ALL` (protects
    the table as it grows);
  - at least three behavior tests that build an `AppState`, install a
    catalog (see `shell.catalog` uses in existing tests), trigger a real
    status path (for example: cancel a prompt, an empty-query find, a
    window command failure), and assert the zh text appears in
    `status_message` — and that with an empty catalog the English text is
    exactly today's.
- The existing completeness/mismatch tests must pass with the full new key
  set (that is your signal the zh file is complete and placeholder-safe).

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/ui_text.rs` (new consts + `ALL`);
  - `crates/dun-cli/src/app/*.rs` (call-site conversion only);
  - `crates/dun-cli/src/dialogs/prompt.rs` (`PromptKind::label`/`name` as
    specified, and their callers wherever they live);
  - `crates/dun-cli/src/main.rs` (only if an import list must follow);
  - `crates/dun-cli/src/tests/i18n.rs` and other `src/tests/*.rs` files
    **only where an existing test asserts a status string whose call site
    you converted and the English output is unchanged — such tests should
    not need edits; if one does, explain why in the report**;
  - `i18n/zh-CN.conf`.
- Files/areas you MUST NOT touch:
  - `crates/dun-cli/src/dialogs/` (except `prompt.rs` as above),
    `crates/dun-cli/src/files/`, `crates/dun-cli/src/help/`,
    `crates/dun-cli/src/command_line.rs`,
    `crates/dun-cli/src/command_output/`, `crates/dun-cli/src/plugins.rs`,
    `crates/dun-cli/src/terminal/`;
  - `crates/dun-core/**`, `crates/dun-ui/**`, `crates/dun-config/**`,
    `crates/dun-term/**`, `crates/dun-plugin/**`;
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`,
    `docs/**`;
  - `.git`, git config, any `Cargo.toml`, `Cargo.lock` — no new
    dependencies;
  - `vm-test/**`, `reference/**`, `hosts/**`, `scripts/**`.

## Deliverable

- Converted call sites in `app/*.rs`; new `status.*` and `prompt.*.label`/
  `prompt.*.name` keys in `ui_text.rs` + `ALL`; zh-CN translations; the
  uniqueness test and the behavior tests.
- In your report: the count of converted call sites, and the explicit list
  of call sites deferred to 4b (helper-composed strings), so the follow-up
  brief can be written from it.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **The 1 MiB dual-platform size budget is real.** This brief adds a large
   table of consts — keep it flat data, no macros generating code, no new
   layers. Claude measures both platforms at the gate.
3. **English output must be byte-identical with an empty catalog.** The
   existing test suite is the fence: hundreds of tests assert status
   strings. If a test fails, your conversion changed behavior — fix the
   conversion, never the test (except as Scope allows, with justification).
4. **`main.rs` is the prelude hub.** `use crate::*` everywhere; keep import
   lists consistent.
5. **Borrow-checker pattern:** `self.set_status(ui_text::tr_fmt(&self.shell.catalog, …))`
   borrows `self.shell` immutably while calling `&mut self` — bind the
   composed `String` to a local first when the compiler objects, exactly as
   existing converted sites do (see `search_replace.rs` title binding).
6. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report — do not keep tuning. If the call-site count makes the
   task feel unbounded, convert domain by domain (save/open/find/replace/
   window/…) and report how far you got with everything green rather than
   half-converting a domain.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Loop until green. The tmux-backed suite requires tmux; if unavailable the
tests skip cleanly — say so rather than reporting them green.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave all changes
  in the working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and
  write that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network. The
  only commands you run are file edits within Scope, `cargo`, and `python3`
  for parsing output.
- Minimal diff: no drive-by reformatting, renames, or comment changes
  outside the task.
- You MUST paste the real verbatim verification output. If a run did not
  reach green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command run, with the exact verbatim output lines
   (suite counts; note any environment-dependent skips).
3. Counts: call sites converted, keys added, zh entries added; the deferred
   list for 4b.
4. Stop-loss / open questions — where you stopped and why (empty if none).
