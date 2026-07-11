# Brief 012 — Palette `warning` role + role-id accessor

Implementation brief. Foundation for config-driven color overrides: add a
semantic `warning` role to the theme `Palette`, populate it in every builtin
theme, and add a role-id ↔ palette-field accessor so a later slice
(`dun-config`) can override individual colors by name. This brief is
`dun-term` only; the config parsing/layering is brief 013.

## Goal

`crates/dun-term`:

1. `Palette` gains a `pub warning: Style` field — the canonical alert color
   (for the planned idle-plugin status indicator and any future alert UI).
2. Every builtin theme populates `warning` with the exact value listed below.
3. `Palette` gains `role(&self, id: &str) -> Option<Style>` and
   `role_mut(&mut self, id: &str) -> Option<&mut Style>` mapping a stable
   role-id string to the matching field, plus
   `pub const PALETTE_ROLE_IDS: &[&str]` listing every role id.
4. Tests cover the accessor/ids consistency and a couple of `warning` values.
   `cargo test -p dun-term` is green.

Note: `warning` is not rendered anywhere yet — it is a defined, overridable
slot that the idle-plugin indicator will consume later. That is expected; do
not wire it into any render path in this brief.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-term/src/theme/palette.rs` — the `Palette` struct (~40 `pub
  <name>: Style` fields). Add `warning` and the accessors/ids here.
- `crates/dun-term/src/theme/builtins.rs` — every `Palette { … }` literal:
  `msedit_256`, `msedit_16`, `mono`, `turbo_256`, `turbo_16`, and
  `with_256_darkish` (shared by `dark_256`/`dun_256`). Each must set
  `warning`. `AnsiColor`, `TerminalColor::Indexed`, `Style::new`,
  `StyleAttrs::BOLD` are already in scope there.
- `crates/dun-term/src/theme/style.rs` — `Style` and `StyleAttrs` are `Copy`,
  so `role` can return `Option<Style>` by value.
- `crates/dun-term/src/theme/tests.rs` — existing theme tests; add the new
  ones here.

## Specification

### `warning` values per theme

Add `warning:` to each `Palette` literal with exactly these styles (all
`BOLD`, foreground listed, background = that theme's editor background):

- `msedit_256`: `Style::new(TerminalColor::Indexed(214), editor_bg, StyleAttrs::BOLD)`
- `msedit_16`: `Style::new(TerminalColor::Ansi(AnsiColor::BrightYellow), editor_bg, StyleAttrs::BOLD)`
- `mono`: reuse the local `bold` (`Default`/`Default` + BOLD)
- `turbo_256`: `Style::new(TerminalColor::Indexed(226), bg, StyleAttrs::BOLD)` (bg is the local `Indexed(19)`)
- `turbo_16`: `Style::new(TerminalColor::Ansi(AnsiColor::BrightYellow), bg, StyleAttrs::BOLD)`
- `with_256_darkish`: `Style::new(warning, editor_bg, StyleAttrs::BOLD)` — reuse
  the existing `warning` local (the fn already takes a `warning: u8` param;
  `dark` passes 214, `dun` passes 203, so no signature change is needed).

### Accessors and ids (in `palette.rs`)

```rust
impl Palette {
    pub fn role(&self, id: &str) -> Option<Style> { … }
    pub fn role_mut(&mut self, id: &str) -> Option<&mut Style> { … }
}

pub const PALETTE_ROLE_IDS: &[&str] = &[ … ];
```

- The role id for each field is its field name verbatim (snake_case):
  `editor`, `editor_text`, `menu_bar`, `menu_text`, `menu_hotkey`,
  `menu_active`, `menu_active_hotkey`, `menu_panel`, `menu_panel_text`,
  `menu_panel_hotkey`, `menu_panel_border`, `status_bar`, `status_text`,
  `window_border`, `window_border_focused`, `title`, `title_focused`,
  `gutter`, `gutter_separator`, `current_line`, `selection`,
  `selection_text`, `search_match`, `active_search_match`, `scrollbar_thumb`,
  `modal_scrim`, `modal`, `modal_text`, `modal_border`, `modal_input`,
  `dirty`, `read_only`, `control`, `escape`, `truncation`, `syntax_keyword`,
  `syntax_comment`, `syntax_string`, `syntax_number`, `syntax_emphasis`,
  `warning`.
- `role`/`role_mut` are a single `match id { "editor" => …, _ => return None }`
  over exactly those ids. `PALETTE_ROLE_IDS` lists exactly the same set in the
  same order. Re-export both `PALETTE_ROLE_IDS` and the `Palette` methods from
  the crate as needed so `dun-config` can use them (they are already `pub` via
  `pub use palette::Palette` — add `pub use palette::PALETTE_ROLE_IDS`).

## Scope

- Files you MAY modify:
  - `crates/dun-term/src/theme/palette.rs`;
  - `crates/dun-term/src/theme/builtins.rs`;
  - `crates/dun-term/src/theme/mod.rs` (only to re-export `PALETTE_ROLE_IDS`
    if needed);
  - `crates/dun-term/src/lib.rs` (only to re-export `PALETTE_ROLE_IDS` if the
    crate root re-exports palette items);
  - `crates/dun-term/src/theme/tests.rs`.
- Files/areas you MUST NOT touch:
  - `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `PROGRESS.md`, `TODO.md`, `docs/**`,
    `README.md`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock` (no dependencies);
  - every other crate (`dun-config`, `dun-ui`, `dun-cli`, `dun-core`,
    `dun-plugin`) — the config layering is a separate brief;
  - `vm-test/**`, `reference/**`, `hosts/**`.

## Deliverable

- `warning` field added and populated in all six `Palette` literals.
- `role`, `role_mut`, `PALETTE_ROLE_IDS` implemented and exported.
- Tests in `tests.rs`:
  1. `palette_role_ids_all_resolve` — for every id in `PALETTE_ROLE_IDS`,
     `Theme::default().palette.role(id).is_some()`, and a bogus id returns
     `None`; assert `PALETTE_ROLE_IDS.len()` equals the field count (41).
  2. `role_mut_overrides_a_single_field` — mutate one role via `role_mut`,
     assert only that field changed.
  3. `themes_define_a_warning_color` — e.g. `dun_256().palette.warning.fg ==
     Indexed(203)`, `dark_256().palette.warning.fg == Indexed(214)`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **The 1 MiB dual-platform size budget is real.** Claude gates size. This is
   const palette data + a match; negligible. No new dependencies.
3. **Sanitizer / render paths** — not touched here; `warning` is not rendered
   in this brief.
4. **`crates/dun-cli/src/main.rs` prelude** — you are not touching dun-cli.
5. **Tests are layered and colocated** — theme tests live in
   `theme/tests.rs`.
6. **Terminal-detection env** — not relevant (no process spawns).
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dun-term --no-fail-fast
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
3. The finding / verdict.
4. Stop-loss / open questions (empty if none).
