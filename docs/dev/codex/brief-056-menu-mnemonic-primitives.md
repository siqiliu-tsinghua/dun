# Brief 056 — step A: shared top-level mnemonic primitives in `dun-ui`

Step A of the plan from brief 055. **Behavior-preserving.** Nothing a user can
see may change: this step only extracts the mnemonic rules that are currently
inlined so that step B can reuse them for plugin menus instead of duplicating
them.

## Goal

`dun-ui` composes a translated top-level menu label as `"{base} ({M})"` with
`M` taken from the compiled English label's first letter, and matches a typed
mnemonic with a trailing-parens-then-first-character rule. Both rules are
inlined today and are not reachable from `dun-cli`, which is why plugin menus
bypass them entirely. Extract them into named, tested helpers and make the
existing code call those helpers, with **byte-identical output**.

When this step is done, no behavior has changed and step B can call the
helpers.

## Context pointers

- Read `AGENTS.md` and `docs/i18n.md` (section "Menu mnemonics are invariant")
  before touching anything.
- `crates/dun-ui/src/frame/menu.rs:325` `menu_label` — composes a translated
  top-level label; the mnemonic is `english.chars().next()`.
- `crates/dun-ui/src/frame/menu.rs:337` `entry_label` — the dropdown analogue,
  which keeps the English trailing `(M)`. **Not in scope**, but read it: the
  two rules are deliberately different and must stay different.
- `crates/dun-ui/src/hit.rs:287` `mnemonic_matches` —
  `entry_mnemonic(label).or_else(|| label.chars().next())`, case-insensitive.
- `crates/dun-ui/src/hit.rs:296` `entry_mnemonic` — the trailing-parens parser.
- `MENUS` in `crates/dun-ui/src/frame/menu.rs` — the built-in menu table.

## Scope

- Files you MAY modify:
  - `crates/dun-ui/src/frame/menu.rs`
  - `crates/dun-ui/src/hit.rs`
  - `crates/dun-ui/src/lib.rs` (re-exports only)
  - `crates/dun-ui/src/tests/i18n.rs`
  - `crates/dun-ui/src/tests/hit.rs`
- Files/areas you MUST NOT touch:
  - anything under `crates/dun-cli/**`, `crates/dun-core/**`,
    `crates/dun-config/**`, `crates/dun-plugin/**`, `hosts/**`, `i18n/**`;
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`,
    `docs/**`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock`;
  - `vm-test/**`, `reference/**`, `acceptance/**`.

## Deliverable

Four helpers, named exactly as below, each with a doc comment saying which
rule it encodes and why the two mnemonic rules differ:

- `english_menu_mnemonic(label: &str) -> Option<char>` — the **top-level**
  rule: the label's first character, accepted **only** when it is ASCII
  alphabetic, normalized to uppercase. Returns `None` otherwise. This is the
  rule step B will apply to a plugin's `en_US` label.
- `menu_label_mnemonic(label: &str) -> Option<char>` — the **matching** rule
  as it exists today: trailing `(M)` if present, else the first character.
  Case handling must stay exactly as it is now.
- `compose_translated_menu_label(base: &str, mnemonic: char) -> String` —
  exactly `format!("{base} ({mnemonic})")`, nothing else.
- `built_in_menu_mnemonics() -> impl Iterator<Item = char>` (or a `Vec<char>`;
  your choice, state it) — derived from `MENUS`. **Do not hard-code a second
  F/E/V/H table anywhere in the implementation**; the whole point is that step
  B's collision check reads the real set. A hard-coded list is allowed only
  inside a *test*, as the oracle.

Then make the existing code call them: `menu_label` uses
`english_menu_mnemonic` + `compose_translated_menu_label`, and
`mnemonic_matches` uses `menu_label_mnemonic`.

Re-export from `crates/dun-ui/src/lib.rs` only what step B will need.

### One behavior question you must not paper over

`menu_label` today does `english.chars().next().unwrap_or('?')` — it cannot
fail, and a non-alphabetic first character would still be composed. The new
`english_menu_mnemonic` returns `Option`. For the built-in menus every label
starts with an ASCII letter, so the observable output is identical either way.
Keep it that way: if `english_menu_mnemonic` returns `None` for a built-in
label, that is a bug in the table, not a case to paper over with `'?'`. State
in your report what you did when the helper returns `None` in `menu_label`,
and why it cannot happen for `MENUS`.

## Invariants (the reason this step exists)

- Built-in English labels stay exactly `File`, `Edit`, `View`, `Help`.
- Every shipped translation renders byte-identically to today, suffix included.
- Keyboard matching stays case-insensitive.
- Menu indices and mouse geometry do not change.
- `entry_label` and `entry_mnemonic` keep their current dropdown behavior.

## Tests

- New `built_in_top_level_mnemonics_are_unique_ascii_letters` — assert the set
  from `built_in_menu_mnemonics()` equals `['F', 'E', 'V', 'H']` (hard-coded
  here as the oracle) and that they are unique.
- New unit tests for `english_menu_mnemonic` covering: an ASCII letter start,
  a lowercase start normalizing to uppercase, a digit start, a non-ASCII start
  (e.g. `日志过滤`), an empty label. These pin the rule step B depends on.
- Existing tests that must still pass unchanged — run them by name and paste
  the result: `empty_catalog_keeps_english_labels_borrowed`,
  `translated_labels_compose_the_english_mnemonic`,
  `mnemonics_keep_working_on_translated_labels`, plus the dun-ui menu render
  snapshots.

Do **not** add any test that asserts new user-visible behavior — there is
none in this step.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** Every crate root has
   `#![forbid(unsafe_code)]`; if you think `unsafe` is unavoidable, STOP and
   report.
2. **The 1 MiB dual-platform size budget is real.** Claude gates runtime-code
   changes with release builds on macOS AND Debian. This step should be
   size-neutral or nearly so; if you find yourself adding allocation or a
   generic layer to make the helpers "nice", stop and keep it concrete.
3. **All untrusted text goes through the sanitizer.** Not expected to come up
   here; do not add a raw print.
4. **`crates/dun-cli/src/main.rs` is the prelude hub** — out of scope this
   step, but if you change a `dun-ui` public name you would break it. Don't
   rename anything existing; only add.
5. **Tests are layered and colocated.** Match the local style of the file you
   extend.
6. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report. In particular: if extracting `mnemonic_matches` into a
   helper turns out to change any snapshot, STOP — that means the rules are
   not what this brief says and Claude needs to re-plan.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Paste results verbatim. The tmux-backed suite needs tmux; if unavailable those
tests skip cleanly — say so rather than reporting them green.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave changes in the
  working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network.
- Minimal diff: no drive-by reformatting, renames, or module splits. In
  particular **do not split `plugins.rs` or any other file** — that was
  deliberately dropped from the plan.
- You MUST paste the real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. The exact signatures you chose, and what `menu_label` does when
   `english_menu_mnemonic` returns `None`.
3. Evidence that output is unchanged: the named existing tests, passing.
4. Verification — each command, verbatim output.
5. Stop-loss / open questions (empty if none).
