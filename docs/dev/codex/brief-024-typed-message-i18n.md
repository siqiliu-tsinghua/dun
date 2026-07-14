# Brief 024 — Typed-message i18n (slice 4b-2: the text a catalog cannot reach)

Implementation brief. Slices 4a and 4b-1 translated every status message that
could be reached by passing a `&TextCatalog` down to where the text is built.
What is left is the text that **cannot** be reached that way, because it is
*stored* or *type-erased* before anyone with a catalog can see it:

- `FileDialogState::message` — an English `String` **stored in state** by
  methods (`refresh_entries`, `toggle_hidden`, `submit`, …) that have no
  catalog and cannot get one: they run on user keystrokes deep inside the
  dialog.
- `path_error_detail` / `path_io_error` — dun-authored text (`not found`,
  `permission denied`, …) **baked into an `io::Error`'s message string**,
  then carried across `io::Result` boundaries until some display site formats
  it with `{}`. By then the structure is gone.
- `"Untitled"` — the string lives in `dun-core`, which **cannot** depend on
  `dun-config`, where `TextCatalog` lives.
- Command Output buffer *content* — the last user of the English
  `exit_status_text`.

The fix for all four is the same shape: **stop storing rendered English; store
typed data, and render at the point where a catalog is in hand.** The designs
below are already decided. Implement them; do not substitute your own.

## Goal

None of the four sources above produces English text on a translated locale.
With an empty catalog every string is **byte-identical** to today. No public
signature outside `dun-cli` changes, and `dun-core` is not touched at all.

## Context pointers

- Read `AGENTS.md` and `docs/i18n.md` first.
- `docs/dev/codex/brief-023-status-helper-i18n.md` — the predecessor. Its
  **vocabulary rule still applies**: text the user types back (paths, command
  ids, theme names) stays English and is passed as a `{}` argument.
- `crates/dun-cli/src/ui_text/` — `mod.rs` (machinery + `ALL`), `chrome.rs`,
  `status.rs` (the key table). New keys go in `status.rs` and its `ALL` list.
- `crates/dun-cli/src/tests/i18n.rs` — the completeness, placeholder-mismatch
  and uniqueness tests walk `ALL` and will cover your new keys automatically.
- `crates/dun-cli/src/files/save.rs` — `path_io_error`, `path_error_detail`,
  `path_error_label`.
- `crates/dun-cli/src/dialogs/file_dialog.rs` — `FileDialogState`, its
  `message` field, `overlay()` (which **does** have a catalog).
- `crates/dun-cli/src/files/dialog.rs` — `file_dialog_list_message`.
- `crates/dun-cli/src/command_output/format.rs` — `command_output_text`.

## Design 1 — typed path errors, without disturbing `io::Result`

`io::Error` can carry an arbitrary custom error as its payload
(`io::Error::new(kind, custom)`), and it can be recovered later with
`get_ref()` + `downcast_ref::<T>()`. That is the whole trick: the `io::Result`
plumbing and every existing signature stay exactly as they are, but the
structure survives to the display site.

In `files/save.rs`:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PathErrorDetail {
    NotFound,
    PermissionDenied,
    ParentMissing,        // today's "parent directory does not exist"
    DestinationReadOnly,  // today's "destination is read-only"
    Other(String),        // the OS message — NOT translatable, passed through
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathIoError {
    pub(crate) label: String,       // path.display(), or "(empty path)"
    pub(crate) detail: PathErrorDetail,
}
```

- `PathErrorDetail::classify(&io::Error) -> PathErrorDetail` replaces the body
  of today's `path_error_detail`, using the same `ErrorKind` + sentinel-message
  rules. Keep `path_error_detail` as a thin English wrapper if anything still
  needs it; delete it if nothing does.
- `PathIoError` implements `Display` producing **exactly** today's English
  (`"{label}: {detail}"`), and `std::error::Error`.
- `path_io_error(path, error)` builds `io::Error::new(error.kind(),
  PathIoError { .. })`. Because `Display` is byte-identical, every existing
  `format!("Save failed: {error}")` keeps producing the same English string
  with no change at all.
- Add `pub(crate) fn path_error_status_text(catalog: &TextCatalog,
  error: &io::Error) -> String`: if `error.get_ref().and_then(|e|
  e.downcast_ref::<PathIoError>())` yields our type, render it through the
  catalog (label stays verbatim — it is a path); otherwise fall back to
  `error.to_string()` (a raw OS error is not ours to translate).
- Use it at the display sites that currently do `&error.to_string()` for
  open/save/save-as/reload failures in `app/file_io.rs` and
  `app/file_dialogs.rs`.

Keys: one per `PathErrorDetail` variant except `Other`, plus the
`"{label}: {detail}"` frame.

## Design 2 — a typed file-dialog message

`FileDialogState` derives `Clone, Debug, PartialEq, Eq`, so the message type
must too — which is exactly why it **cannot** hold an `io::Error`. It holds
`PathErrorDetail` from Design 1 instead. Do Design 1 first.

In `dialogs/file_dialog.rs`:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileDialogMessage {
    NoMatches,
    NoVisibleMatches,                                   // hidden-files hint
    NoMatchesForPrefix(String),
    OnlyHiddenFiltered,
    DirectoryEmpty,
    CannotList { directory: String, detail: PathErrorDetail },
    HiddenFiles { shown: bool },
    ConfirmOverwrite(String),                           // path
    /// Already-rendered text composed elsewhere *with* a catalog — the
    /// open/save failure status that `app/file_dialogs.rs` copies into the
    /// dialog. This is the one honest escape hatch; do not use it for
    /// anything a variant can express.
    Text(String),
}
```

- `message: Option<FileDialogMessage>`; every `self.message = Some(...)` site
  stores a variant instead of a `String`.
- `file_dialog_list_message` (`files/dialog.rs`) returns
  `Option<FileDialogMessage>` — same branching, typed results.
- Add `FileDialogMessage::render(&self, catalog: &TextCatalog) -> String`, and
  call it in `FileDialogState::overlay(...)`, which already receives the
  catalog.
- The two sites in `app/file_dialogs.rs` that copy an already-translated
  status into the dialog use `FileDialogMessage::Text(status.clone())`.

## Design 3 — `"Untitled"`, without touching `dun-core`

`dun-core` keeps producing its neutral `"Untitled"` / `"Untitled-{n}"` titles;
it is the core and must not learn about catalogs. `dun-cli` **overwrites the
title at the two points where such a window is created**:

- after `Workspace::new_untitled()` in `AppState::from_loaded_config`;
- after a successful split in `app/windows.rs` (the new window's title);
- and the existing reset-to-untitled site in `app/file_io.rs`.

Add `WINDOW_UNTITLED` (`"Untitled"`) and `WINDOW_UNTITLED_NUMBERED`
(`"Untitled-{}"`) keys. Because a missed site would silently leave English,
add a test that asserts, with the zh catalog: the startup window title is the
translated one, **and** the window created by a split is the translated
numbered one.

## Design 4 — Command Output buffer content

`command_output_text(result)` takes `&TextCatalog` and keys its fixed labels
(`Dun Command Output`, `Command:`, `Shell:`, `Status:`, `Elapsed:`, `Limit:`,
`Stdout:`, …), using the catalog-aware exit-status formatter that already
exists in `terminal/shell.rs`. The caller in `app/command_output.rs` has the
catalog. Once nothing outside uses the English `exit_status_text`, delete it —
`localized_exit_status_text` with an empty catalog is the English form.

Keep the *values* untranslated: the command line, the shell path, byte counts,
durations.

## Explicitly NOT in this brief

- **Do not split `ui_text/status.rs`.** It is over the size threshold with a
  recorded temporary exception; the domain split is brief-025, after this
  brief stops adding keys to it. Adding your keys to it is expected.
- Do not touch `dun-core`, or any crate other than `dun-cli`.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/files/save.rs`, `crates/dun-cli/src/files/dialog.rs`,
    `crates/dun-cli/src/files/mod.rs` (re-exports);
  - `crates/dun-cli/src/dialogs/file_dialog.rs`;
  - `crates/dun-cli/src/command_output/format.rs`,
    `crates/dun-cli/src/terminal/shell.rs` (only to retire
    `exit_status_text`);
  - `crates/dun-cli/src/app/*.rs` (call sites, the two title-override points);
  - `crates/dun-cli/src/ui_text/status.rs` and `crates/dun-cli/src/ui_text/mod.rs`
    (new keys + `ALL`);
  - `crates/dun-cli/src/main.rs` (import lists must follow);
  - `crates/dun-cli/src/tests/*.rs` (new tests; existing test files only where
    a signature change forces it — say so in the report);
  - `i18n/zh-CN.conf`.
- Files/areas you MUST NOT touch:
  - `crates/dun-core/**`, `crates/dun-ui/**`, `crates/dun-config/**`,
    `crates/dun-term/**`, `crates/dun-plugin/**`;
  - `crates/dun-cli/src/help/text.rs` (section labels stay English — they are
    typed vocabulary);
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`,
    `docs/**`;
  - `.git`, git config, any `Cargo.toml`, `Cargo.lock` — no new dependencies;
  - `vm-test/**`, `reference/**`, `hosts/**`, `scripts/**`.

## Deliverable

- The four designs implemented; new keys in `status.rs` + `ALL`; zh-CN
  translations; tests.
- Tests (at least): a file-dialog message rendered in zh through a real
  dialog interaction; a path error (open a directory that cannot be listed, or
  a missing file) rendered in zh **and** byte-identical in English; the
  untitled-title test from Design 3 (startup **and** after split).
- In your report: anything you had to leave English, and why.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
   `downcast_ref` is safe std API — no `unsafe` is needed anywhere here.
2. **English output must be byte-identical with an empty catalog.** Hundreds of
   existing tests assert exact strings — they are the fence. `PathIoError`'s
   `Display` is what keeps `format!("{error}")` unchanged; get it exactly right
   (including the `(empty path)` label case) before anything else.
3. **`FileDialogState` derives `Clone + PartialEq + Eq`.** Whatever you put in
   the message enum must satisfy those — that is why the path detail is typed
   and why `io::Error` may not appear in it.
4. **The 1 MiB dual-platform size budget is real.** Flat data, no macros, no new
   abstraction layers. Claude measures both platforms at the gate.
5. **`main.rs` is the prelude hub.** Modules use `use crate::*`; moved or
   renamed symbols must be reflected in its import lists in the same change.
6. **Stop-loss is real.** Same step failing twice for the same reason → STOP and
   report. The four designs are independent apart from 2-depends-on-1: if one
   defeats you, finish the others, leave that one untouched, and say so.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Loop until green. The tmux-backed suite needs tmux; if unavailable the tests
skip cleanly — say so rather than reporting them green.

Then prove one new test load-bearing: break one rendering path (point a variant
at the wrong key), confirm the test fails naming it, and restore it. Restore by
**reversing your edit** — never `git checkout` a file; the working tree holds all
of your uncommitted work.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave all changes in
  the working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write that
  in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network. The only
  commands you run are file edits within Scope, `cargo`, and `python3` for
  parsing output.
- Minimal diff: no drive-by reformatting, renames, or comment changes outside the
  task.
- You MUST paste the real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command with verbatim output (suite counts; note any
   environment-dependent skips), including the mutant run.
3. Counts: keys added, zh entries added; anything left English and why.
4. Stop-loss / open questions (empty if none).
