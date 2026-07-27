# Brief 054 — translate the two diagnostic window bodies

## Goal

The Status History and Config Diagnostics windows are half-translated: their
title bars, their status messages, and every status-history entry already go
through the catalog, but the **document heading and the section headings
inside the window body are hardcoded English in all eleven languages**. Make
that body text catalog-driven like the rest of the UI, add the new keys to the
compiled English defaults and to all ten shipped catalogs, and prove with a
test that section jumping still works when the headings are translated.

English output must stay **byte-identical**: `tr()` falls back to the compiled
English default, so a run with no catalog loaded must produce exactly what it
produces today.

## Context pointers

- Read `AGENTS.md` (invariants, engineering rules) and `docs/i18n.md` (the
  catalog model, key naming, and why English is compiled in) before touching
  anything.
- Key files:
  - `crates/dun-cli/src/app/helper_panes.rs` — builds both windows; line ~355
    `String::from("Dun Status History\n\n")` and line ~374
    `String::from("Dun Config Diagnostics\n\n")` are the raw literals. Line
    ~262 is the section lookup described under "The one design constraint".
  - `crates/dun-cli/src/help/text.rs` — `ConfigDiagnosticsSection::heading()`
    (lines ~86-98) returns the nine hardcoded section headings. **This file
    has zero catalog references today**; it will gain them.
  - `crates/dun-cli/src/ui_text/chrome.rs` — how a `TextKey` const is declared
    and registered in the module's key array (see
    `WINDOW_CONFIG_DIAGNOSTICS_TITLE` at ~87 and the array at ~152). Follow
    this exact pattern.
  - `crates/dun-cli/src/tests/i18n.rs` — `translation_defaults()` gathers every
    key from `ui_text::ALL`, `help::content::help_translation_keys()` and
    `dun_ui::menu_translation_keys()`, and the shipped catalogs are validated
    against it. A key added without all ten catalog entries must fail here.
  - `i18n/*.conf` — the ten shipped catalogs.
- Acceptance is mechanical: the named tests decide, not prose.

## The evidence this is a defect, not policy

Do not "fix" this by declaring the bodies intentionally English. Two facts rule
that out: the catalog already carries `window.config-diagnostics.title` and
`window.status-history.title`, and the Help window body is fully translated
(`help/content.rs` uses `tr` ~31 times, e.g. `tr(catalog, "help.title",
"Dun Help")`). Neither `docs/i18n.md` nor the catalog comments exempt
diagnostics.

## The one design constraint (already decided — do not redesign)

`ConfigDiagnosticsSection::heading()` is not only rendered into the window
buffer; `app/helper_panes.rs:~262` also uses it as a **lookup key**:

```rust
.and_then(|buffer| line_with_exact_text(&buffer.buffer, section.heading()))
```

So the writer and the searcher must agree. The decided design is:

- `heading()` becomes catalog-aware — `heading(catalog: &TextCatalog) -> &str`
  (or an equivalent signature that takes the catalog), returning
  `ui_text::tr(catalog, KEY)`.
- **Both** the builder that writes the heading into the buffer and the lookup
  at ~262 pass the same `&self.shell.catalog`, so `line_with_exact_text` keeps
  matching.
- Do NOT invent a separate invariant anchor, a second lookup mechanism, or
  stored line indices. Keep the diff minimal.

`ConfigDiagnosticsSection::label()` (the lowercase forms at ~100-111, which
fill the `{}` in `status.config-diagnostics.section`) is **out of scope** —
translating it would change existing English status output. Leave it alone.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/app/helper_panes.rs`
  - `crates/dun-cli/src/help/text.rs`
  - `crates/dun-cli/src/ui_text/chrome.rs` (or the ui_text module file where
    these keys most naturally belong — one file, follow the existing grouping)
  - `crates/dun-cli/src/tests/i18n.rs` and/or
    `crates/dun-cli/src/tests/helper_panes.rs` — for the new tests
  - `i18n/de.conf`, `es.conf`, `fr.conf`, `it.conf`, `ja.conf`, `ko.conf`,
    `pt.conf`, `ru.conf`, `zh-Hans.conf`, `zh-Hant.conf`
- Files/areas you MUST NOT touch:
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`,
    `docs/**`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock`;
  - `vm-test/**`, `reference/**`, `acceptance/**`, `hosts/**`.

## Deliverable

- New `TextKey` consts + compiled English defaults for:
  - the Status History document heading (`Dun Status History`);
  - the Config Diagnostics document heading (`Dun Config Diagnostics`);
  - the nine `ConfigDiagnosticsSection` headings: `Summary`, `Paths`,
    `Source`, `Terminal`, `Input`, `Clipboard`, `Limits`, `Keymap`,
    `File Dialog Keymap`.
  Name them consistently with the existing `window.*` / section conventions
  you find in `ui_text/chrome.rs`; state the names you chose in your report.
- Additionally: the status-history entry severity tag rendered as `[info]`
  (produced by `entry.level.label()`, called at `app/helper_panes.rs:~365`).
  Find its definition. **If** it is user-visible display text, route it through
  the catalog the same way and add its keys. If you find it is an identifier
  reused as a machine-readable token, leave it and say so in your report with
  the evidence.
- All ten shipped catalogs updated with every new key, translated into that
  file's language, matching the tone and terminology already used in that file
  (e.g. reuse the wording each catalog already uses for
  `window.config-diagnostics.title`).
- Tests:
  1. **Load-bearing:** with the shipped `zh-Hans` catalog installed, opening
     Config Diagnostics and jumping to a section still lands on the right
     line — i.e. the translated heading is found. Model it on the existing
     `shipped_zh_catalog()` helper in `tests/i18n.rs`.
  2. The rendered Config Diagnostics body under `zh-Hans` contains no
     occurrence of the English heading strings.
  3. English is unchanged: with the default (empty) catalog, the body still
     contains exactly `Dun Config Diagnostics` / `Dun Status History` and the
     nine English section headings.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** Every crate root has
   `#![forbid(unsafe_code)]`; if you think `unsafe` is unavoidable, STOP and
   report.
2. **The 1 MiB dual-platform size budget is real.** Claude gates any
   runtime-code change with release builds on macOS AND Debian. Keep the diff
   minimal; no new dependencies. Translations themselves are external resource
   files and cost the binary nothing, but new runtime code paths do — prefer
   `tr()` lookups over new formatting layers.
3. **All untrusted text goes through the sanitizer.** Anything reaching the
   terminal must pass the existing sanitized paths. Catalog values are already
   length-capped and validated; do not add a raw print.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** Modules use
   `use crate::*`; if you move or remove a symbol, update the import lists in
   `main.rs` in the same change.
5. **Tests are layered and colocated.** Match the local style of the file you
   extend.
6. **Terminal-detection env is pinned in harnesses.** Any test that spawns the
   editor must pin/clear TERM, COLORTERM, LANG, LC_CTYPE, NO_COLOR.
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report — do not keep tuning. In particular, if making `heading()`
   catalog-aware turns out to require threading the catalog through more than
   two or three call sites, STOP and report the call-site inventory rather
   than performing a wide refactor.

## Translation quality

You are producing machine translations, and they will be reviewed. Do not
guess at terminology: for each new key, look at how that same catalog already
renders the closest existing concept and stay consistent with it. Where a
heading is a bare technical noun that the catalog already leaves untranslated
elsewhere (check before assuming), keep it consistent rather than inventing a
new word. In your report, list the eleven values you used for **one** key
(`Summary`) so the choices can be spot-checked quickly.

## Verification (MANDATORY — you run it; iterate to green)

Run exactly these and paste results verbatim:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Loop: edit → test → fix → rerun, until green. Never claim a result without the
verbatim lines. The tmux-backed suite requires tmux; if unavailable those tests
skip cleanly — say so rather than reporting them green.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave all changes in
  the working tree; Claude runs the authoritative gate and commits.
- The working tree already contains unrelated uncommitted changes under
  `acceptance/` and `docs/`. Leave them exactly as they are.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network.
- Minimal diff: no drive-by reformatting, renames, or comment changes outside
  the task.
- You MUST paste the real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. The key names you chose, and the eleven values for `Summary`.
3. The `entry.level.label()` verdict with evidence.
4. Verification — each command, verbatim output lines (suite counts; note any
   environment-dependent skips).
5. Stop-loss / open questions — where you stopped and why (empty if none).
