# Brief 015 — Contract tests: what the UI declares, the runtime must honour

Implementation brief. A recent bug hunt found three defects of one shape: some
part of the program *declares* something and another part never honours it.

- Every menu entry advertises a mnemonic in its label ("Open... (O)"), but
  nothing dispatched a bare letter, so all 40 were dead keys.
- `file.close` was a bare alias for `window.close`: one command listed twice
  under two names, and neither closed a file.
- The README listed `mono` as a selectable theme; `theme = mono` is rejected.

Hand-written tests do not catch this class, because the bug is *absence*. The
fix is tests that derive their expectations **from the declaration itself**, so
they cover new menu entries, commands and roles automatically as they are added.

## Goal

Add exhaustive, self-maintaining contract tests. Where a contract cannot hold
today, do NOT change behaviour to force it green: record the exceptions in an
explicit allowlist and report them. Claude decides what to do with them.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-core/src/command.rs` — `EditorCommand` and its four sub-enums
  (`AppCommand`, `FileCommand`, `EditCommand`, `WindowCommand`).
- `crates/dun-config/src/commands.rs` — `command_id(&EditorCommand) ->
  &'static str` (an exhaustive match) and `command_from_id(&str) ->
  Result<EditorCommand, CommandParseError>`.
- `crates/dun-config/src/tests/keys.rs::command_ids_round_trip` — the current
  test is a **spot check** of three hand-picked commands. Replace it with an
  exhaustive one.
- `crates/dun-config/src/keys/keymap.rs::default_editor()` — the compiled-in
  keymap. `Keymap::sequence_for_command`, `Keymap::bindings`.
- `crates/dun-ui/src/frame/menu.rs` — the four menus and their entry labels.
  `crates/dun-ui/src/hit.rs` already has `entry_mnemonic(label) -> Option<char>`
  (`pub(crate)`) and `menu_entry_index_for_mnemonic`, plus `menu_count`,
  `menu_entry_count`, `menu_entry_command`.
- `crates/dun-cli/src/tests/menus_keyboard.rs` — existing mnemonic tests. They
  currently hardcode a `MENU_MNEMONICS` table; derive it instead.
- `crates/dun-cli/src/help/content.rs` — `HelpSection` / `HelpCommand`, the
  in-editor help listing.
- `crates/dun-cli/src/app/command_line.rs` — the `Ctrl+P` prompt. Note its
  final arm falls through to `command_from_id`, so **every** command id is
  reachable by typing it. Reachability must therefore be judged *excluding* the
  prompt (see C7).
- `crates/dun-term` — `PALETTE_ROLE_IDS` (41 role ids).
- `crates/dun-config/src/defaults.rs` — `default_config_text()` emits one
  commented `# color.<role> = ...` line per role.

## Specification

### 1. `ALL_COMMAND_IDS` (`dun-config`)

There is currently no way to enumerate every `EditorCommand`. Add, next to
`command_id` in `crates/dun-config/src/commands.rs`:

```rust
/// Every command id, in one list, so contract tests can iterate the whole
/// command surface. Adding an `EditorCommand` variant makes `command_id` fail
/// to compile until an arm is added; the length assertion in
/// `all_command_ids_round_trip` is the tripwire that this list was updated too.
pub const ALL_COMMAND_IDS: &[&str] = &[ … ];
```

- It must contain exactly the id that `command_id` returns for every variant of
  `EditorCommand` (all four sub-enums), with no duplicates.
- Re-export it from `crates/dun-config/src/lib.rs`.
- This is `const` data of `&'static str`; it costs no meaningful bytes (the
  same pattern as `PALETTE_ROLE_IDS`). No new dependencies.

### 2. Contracts to assert

Write each as a named test. C1–C3 live in `crates/dun-cli/src/tests/`
(they need `AppState` + `UiShell`), C4–C6 and C9 in `crates/dun-config/src/tests/`,
C7 in `dun-cli`.

**C1 — every menu entry has a unique, derivable mnemonic.**
For each menu, for each entry: `entry_mnemonic(label)` is `Some`, and no two
entries in the same menu share one (case-insensitively). Derive the letters from
the labels; do NOT hardcode a table. Replace the `MENU_MNEMONICS` constant in
`menus_keyboard.rs`. Expose whatever accessor `dun-ui` needs for this — add
`UiShell::menu_entry_mnemonic(&self, menu_index, entry_index) -> Option<char>`
in `hit.rs`.

**C2 — every mnemonic dispatches its own entry.** Exhaustively: for each menu
index and entry index, open that menu on a fresh `AppState`, press the bare
letter, and assert the command that ran is exactly
`shell.menu_entry_command(menu, entry)`. (Drive it through `handle_key_event`
like the existing tests. To observe *which* command ran without executing
side effects, prefer asserting `menu_entry_index_for_mnemonic(menu, ch) ==
Some(entry)` for every entry, plus the two existing end-to-end dispatch tests.
Do not try to execute Quit or Shell Escape.)

**C3 — no two menu entries dispatch the same command.** Collect every
`(menu, entry) -> EditorCommand` and assert the commands are pairwise distinct
across the whole menu bar. This is the contract `file.close` violated.

**C4 — every command id round-trips.** For every id in `ALL_COMMAND_IDS`:
`command_from_id(id)` is `Ok`, and `command_id(&that)` is the same id.
Assert `ALL_COMMAND_IDS.len()` equals the real variant count and that the ids
are unique. Replace the spot-check `command_ids_round_trip`; keep its existing
alias assertions (e.g. `edit.move-word-right` with a hyphen) as a separate test.

**C5 — every default keybinding names a real command.** For every binding in
`Keymap::default_editor()`, `command_id(&binding.command)` is in
`ALL_COMMAND_IDS`.

**C6 — every help entry names a real command.** For every `HelpCommand` in
`help/content.rs`, its command's id is in `ALL_COMMAND_IDS`.

**C7 — every command is reachable without typing its id.** For every id in
`ALL_COMMAND_IDS`, it must be at least one of:
  - bound in `Keymap::default_editor()`, or
  - present as a menu entry, or
  - listed in an explicit `PROMPT_ONLY_COMMANDS: &[&str]` allowlist in the test.

The `Ctrl+P` prompt falls through to `command_from_id`, so *everything* is
technically reachable by typing — that is exactly why the prompt must be
excluded here: a command with no key and no menu entry is one no user will ever
find. **Expect this test to find some.** Put them in `PROMPT_ONLY_COMMANDS`
with a one-line comment each, and list every one of them in your report. Do NOT
add keybindings or menu entries to make it pass.

**C9 — the palette roles and the dumped config agree.** Every id in
`PALETTE_ROLE_IDS` appears exactly once as a `# color.<role> = ` line in
`default_config_text()`, and there are no `color.` lines for ids outside it.

### 3. Theme self-consistency (`dun-config`)

**C8** — for every `ThemeName` variant, `parse_theme_name(name.as_str())`
returns that variant; and the set of *primary* names the parser accepts is
exactly the set of `as_str()` values. (Aliases such as `microsoftedit` and
`turbovision` are fine and should keep working — assert them separately.)
`parse_theme_name` is private to `parser.rs`; test it from within that module's
test scope or via `parse_config("theme = …")`, whichever is cleaner.

## Scope

- Files you MAY modify:
  - `crates/dun-config/src/commands.rs` (add `ALL_COMMAND_IDS`);
  - `crates/dun-config/src/lib.rs` (re-export);
  - `crates/dun-config/src/tests/` (existing files, and a new module if you
    add one — register it in `tests/mod.rs`);
  - `crates/dun-ui/src/hit.rs` (add `menu_entry_mnemonic`);
  - `crates/dun-ui/src/lib.rs` (only if a re-export is needed);
  - `crates/dun-cli/src/tests/` (existing files, and a new module if you add
    one — register it in `tests/mod.rs`).
- Files/areas you MUST NOT touch:
  - `crates/dun-core/**` — do not add or change commands;
  - `crates/dun-ui/src/frame/menu.rs` — do not add, remove, or relabel menu
    entries to make a contract pass;
  - `crates/dun-config/src/keys/keymap.rs` — do not add bindings to make C7
    pass;
  - `crates/dun-cli/src/help/content.rs`;
  - any non-test runtime behaviour anywhere;
  - `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `PROGRESS.md`, `TODO.md`, `docs/**`,
    `README.md`;
  - `.git`, git config, any `Cargo.toml`, `Cargo.lock` (no new dependencies —
    no `strum`, no `proptest`, no `insta`);
  - `vm-test/**`, `reference/**`, `hosts/**`.

## Deliverable

- `ALL_COMMAND_IDS` + re-export.
- `UiShell::menu_entry_mnemonic`.
- Contract tests C1–C9, named for what they protect, each with a comment naming
  the real bug it would have caught.
- `PROMPT_ONLY_COMMANDS` allowlist, if C7 needs one, with every entry reported.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **The 1 MiB dual-platform size budget is real.** Claude gates size.
   `ALL_COMMAND_IDS` is const `&'static str` data; no dependencies.
3. **Do NOT change behaviour to make a contract green.** These tests exist to
   *find* the gaps. An allowlist plus a report is the correct outcome; silently
   adding a keybinding or renaming a menu entry is not.
4. **Derive, do not hardcode.** A test that restates a table by hand has to be
   maintained by hand and will drift — that is how these bugs survived. Read the
   menus, the keymap and `ALL_COMMAND_IDS` at runtime.
5. **Tests are layered and colocated.** dun-config contracts in
   `dun-config/src/tests/`, menu/app contracts in `dun-cli/src/tests/`.
6. **Do not execute Quit or Shell Escape** from a test — assert the mapping
   rather than dispatching those two.
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Loop: edit → test → fix → rerun, until green. Paste verbatim output.

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
3. **The findings** — every command in `PROMPT_ONLY_COMMANDS`, and any other
   contract that could not hold as written. This is the point of the brief; do
   not bury it.
4. Stop-loss / open questions (empty if none).
