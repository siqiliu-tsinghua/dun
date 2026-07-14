# Brief 026 — Locale script fallback, and a validator for every translation

Implementation brief. Two defects block shipping more languages. Both are in
the *mechanism*, not the translations, and both must land before any new
language file does.

## Defect 1 — the locale chain cannot reach a translation it has

`locale_candidates` (`crates/dun-config/src/i18n.rs`) resolves a locale to
`["<lang>-<REGION>", "<lang>"]` and nothing else. So today, with
`i18n/zh-CN.conf` shipped:

| user locale | dun looks for | gets |
| --- | --- | --- |
| `zh_CN.UTF-8` | `zh-CN`, `zh` | Simplified Chinese |
| `zh_SG.UTF-8` | `zh-SG`, `zh` | **English** |
| `zh_TW.UTF-8` | `zh-TW`, `zh` | **English** |

A Singaporean user reads exactly the same Simplified Chinese as a mainland
user and gets English, because the chain has no way to know that `zh-SG` and
`zh-CN` share a **script**. That knowledge is a linguistic fact, and it needs
a (very small) table.

### The fix: a script step in the candidate chain

Insert the script tag between the region tag and the bare language:

| user locale | new candidate chain |
| --- | --- |
| `zh_CN` / `zh_SG` / `zh_MY` | `zh-CN` → `zh-Hans` → `zh` |
| `zh_TW` / `zh_HK` / `zh_MO` | `zh-TW` → `zh-Hant` → `zh` |
| `zh` (no region) | `zh-Hans` → `zh` |
| `de_DE` | `de-DE` → `de` (no script table entry → unchanged) |

Rules:

- The region-specific tag stays **first**, so a user who writes their own
  `~/.config/dun/i18n/zh-CN.conf` still overrides the shipped file.
- The script table lives in `dun-config` and is tiny — currently Chinese
  only:
  `zh` + {`CN`, `SG`, `MY`} → `Hans`; `zh` + {`TW`, `HK`, `MO`} → `Hant`;
  `zh` with no region → `Hans` (the CLDR likely-subtag default).
  A language with no entry gets today's two-step chain, unchanged.
- Script tags are title-case (`Hans`, `Hant`), region tags upper-case,
  language lower-case — that is BCP 47, and the file names must match
  exactly, because the loader looks up `<dir>/<tag>.conf` by name.
- **Do not** add an alias table mapping one region onto another
  (`zh-SG → zh-CN`). That would bake a product judgement ("mainland is the
  canonical Simplified") into the core. Script is a fact; canonical region is
  an opinion.

### The rename that follows

`i18n/zh-CN.conf` → `i18n/zh-Hans.conf`. The content does not change. The name
now says what the file *is* (Simplified Chinese, independent of country),
which is why every `zh_CN`/`zh_SG`/`zh_MY` user can now reach it. Update every
reference to the old name (tests use `include_str!`).

Traditional Chinese (`zh-Hant.conf`) is **not** part of this brief. It is a
translation, and it comes in a later one. Until it exists, `zh_TW` users
correctly get English rather than the wrong script.

## Defect 2 — only `zh-CN` is validated

`crates/dun-cli/src/tests/i18n.rs` validates the shipped translation through a
hardcoded `include_str!("../../../../i18n/zh-CN.conf")`. Any *new* language
file would ship completely unchecked: missing keys, wrong placeholder counts,
and sanitizer-hostile values would all reach users.

### The fix: validate every file in `i18n/`

Replace the hardcoded checks with a test that **discovers** every
`i18n/*.conf` at test time (resolve the directory from
`env!("CARGO_MANIFEST_DIR")`) and, for each file, asserts all of:

1. **It parses** under the real loader rules (`dun_config::parse_catalog`) —
   which already enforces the ≤256-byte values and rejects anything the
   display sanitizer would escape.
2. **The file is within the size cap** (`MAX_CATALOG_FILE_BYTES`).
3. **Completeness**: every key in `ui_text::ALL` and every key from
   `help::content::help_translation_keys()` is present. Failure must list the
   missing keys, as the current test does — that list is what makes a
   translation finishable.
4. **Placeholder integrity**: each value's `{}` count equals its English
   default's. (A mismatch silently falls back to English at runtime, so a
   shipped mismatch is a bug, not a preference.)
5. **No unknown keys**: every key in the file is one the UI can actually look
   up. A typo'd key is dead weight that silently does nothing; report it.
6. **The destructive-action guard** (new, and the reason this matters):
   `confirm.button.save`, `confirm.button.discard` and `confirm.button.cancel`
   must each be present, non-empty, and **pairwise distinct**. These three
   words are drawn next to the literal keys `(s)`, `(d)`, `(c)` that the
   dialog answers to, so a translation that renders two of them identically —
   or leaves one empty — invites a user to press `(d)` and lose unsaved work.
   Everything else in the UI is protected by the vocabulary rule; this is the
   one place a bad translation can destroy data.

The repo holds *its own* files to completeness. That is stricter than the
runtime, which deliberately tolerates a partial user-supplied file by falling
back to English per key — do not change the loader's tolerance.

Keep the existing behavior tests (the ones that drive real command paths and
assert exact English + exact zh output). They should keep passing; only the
file name they reference changes.

## Scope

- Files you MAY modify:
  - `crates/dun-config/src/i18n.rs` (the candidate chain + script table) and
    `crates/dun-config/src/tests/i18n.rs` (its unit tests);
  - `crates/dun-cli/src/tests/i18n.rs` (the generic validator);
  - `i18n/zh-CN.conf` → `i18n/zh-Hans.conf` (rename; content unchanged);
  - any other file *only* if it references the old file name and fails to
    compile otherwise — say so in the report.
- Files/areas you MUST NOT touch:
  - the loader's runtime tolerance for partial files
    (`crates/dun-cli/src/i18n_loading.rs`) beyond what the rename requires;
  - `crates/dun-cli/src/ui_text/**` — no key changes;
  - `crates/dun-core/**`, `crates/dun-ui/**`, `crates/dun-term/**`,
    `crates/dun-plugin/**`;
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`,
    `docs/**` (Claude updates the docs at the gate);
  - `.git`, git config, any `Cargo.toml`, `Cargo.lock` — no new dependencies;
  - `vm-test/**`, `reference/**`, `hosts/**`, `scripts/**`.

## Deliverable

- The script-aware candidate chain, with unit tests covering every row of the
  table above **plus** the unchanged behavior for a language with no script
  entry (`de_DE`, `ja_JP`) and the existing `C`/`POSIX`/junk rejections.
- The generic validator, with all six checks.
- The rename, with every reference updated.
- In your report: confirm by test that `zh_SG.UTF-8` and `zh_MY.UTF-8` now
  resolve to the shipped Simplified file, and that `zh_TW.UTF-8` still gets
  English (because `zh-Hant.conf` does not exist yet) — that pair is the whole
  point of the change.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **The 1 MiB dual-platform size budget is real.** The script table is a
   `match`, not a `HashMap`. Claude measures both platforms at the gate.
3. **Case matters.** `zh-Hans`, not `zh-hans` or `ZH-HANS`: the loader opens
   `<dir>/<tag>.conf` by name, and the repo file must match byte for byte on a
   case-sensitive filesystem, even though macOS would forgive you. Get the
   casing right in the code *and* in the file name.
4. **Do not weaken the runtime loader.** A user's partial file must still work
   key-by-key. Only the repo's own files are held to completeness, and only in
   tests.
5. **The validator must fail loudly and usefully.** When a key is missing, the
   message must list exactly which keys — a translator finishes a file by
   reading that list.
6. **Stop-loss is real.** Same step failing twice for the same reason → STOP
   and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Loop until green. The tmux-backed suite needs tmux; if unavailable it skips
cleanly — say so rather than reporting it green.

Then prove the validator is load-bearing, with **two** mutations, restoring by
reversing your edit (never `git checkout` — the tree holds your uncommitted
work):

1. Delete one key from `i18n/zh-Hans.conf` → the completeness check must fail
   and name that key.
2. Make `confirm.button.discard` identical to `confirm.button.save` → the
   destructive-action guard must fail.

Paste the verbatim output of both, and of the restored green run.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave all changes in
  the working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network. The
  only commands you run are file edits within Scope, `cargo`, `git mv`/`git
  status` (read-only or rename), and `python3` for parsing output.
- Minimal diff: no drive-by reformatting, renames, or comment changes outside
  the task.
- You MUST paste the real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command with verbatim output, including both mutants.
3. The resolution table: what `zh_CN`, `zh_SG`, `zh_MY`, `zh_TW`, `zh_HK`,
   `zh`, `de_DE`, `C` each resolve to now.
4. Stop-loss / open questions (empty if none).
