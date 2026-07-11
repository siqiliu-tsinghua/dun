# Brief 013 — Config-driven per-color overrides (`dun-config`)

Implementation brief. Layer user color overrides on top of the selected
builtin theme. Builds on brief 012, which added `Palette::role`/`role_mut`
and `PALETTE_ROLE_IDS` to `dun-term`. This brief is `dun-config` only.

## Goal

A user's config file can override individual palette colors on top of
whichever theme is selected, without redefining the whole theme:

- Granular keys: `color.<role>.fg`, `color.<role>.bg`, `color.<role>.attrs`.
- Shorthand: `color.<role> = <fg>` (fg only) or `color.<role> = <fg> / <bg>`.

Unset components keep the theme default. Overrides apply to the *resolved*
theme (after profile fallback), so they survive the 256→16→mono selection.

A color value is a palette index `0`–`255`, an ANSI color name
(`black`, `red`, …, `bright_blue`, …), or `default`. `attrs` is a
comma/space list of `bold`, `underline`, `reverse`, or `none`.

`dun --dump-config` (which prints `default_config_text()`) gains a commented
`# Color overrides` section listing every role with its dun-theme default, so
the roles are discoverable.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-term` (already landed, do NOT modify): `Palette::role(&self, id)
  -> Option<Style>`, `Palette::role_mut(&mut self, id) -> Option<&mut Style>`,
  `pub const PALETTE_ROLE_IDS: &[&str]` (41 snake_case ids). Types
  `TerminalColor { Default, Ansi(AnsiColor), Indexed(u8) }`, `AnsiColor`
  (16 variants: `Black`…`White`, `BrightBlack`…`BrightWhite`),
  `StyleAttrs { bold, underline, reverse }` (`Copy`), `Style { fg, bg, attrs }`.
  All are re-exported at the `dun_term` crate root (`dun_term::TerminalColor`
  etc.) — no `Cargo.toml` change is needed; `dun-config` already depends on
  `dun-term`.
- `crates/dun-config/src/config.rs` — the `Config` struct and its
  `resolved_theme(&self, detected) -> Theme`. `resolved_theme` currently is
  `Theme::for_profile(self.theme, self.terminal_profile(detected))`.
- `crates/dun-config/src/parser.rs` — `apply_config_entry` is the big
  `match key.as_str()` dispatcher. Keys are normalized by
  `normalize_config_key` (lowercased; `-` and space → `_`) BEFORE the match,
  so `color.window-border.fg` arrives as `color.window_border.fg`. Values pass
  through `unquote_value`. Existing prefix arms use
  `_ if key.starts_with("key.") => …`. Mirror that for `color.`.
- `crates/dun-config/src/defaults.rs` — `default_config_text()` builds the
  dump string. Add the color section here.
- `crates/dun-config/src/lib.rs` — module list and re-exports.
- `crates/dun-config/src/tests/` — colocated tests; `mod.rs` lists modules,
  `support.rs` is the shared prelude (`use crate::*`).

## Specification

### 1. New module `crates/dun-config/src/colors.rs`

```rust
use dun_term::{AnsiColor, PALETTE_ROLE_IDS, Palette, StyleAttrs, TerminalColor};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ColorOverrides {
    entries: Vec<RoleOverride>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RoleOverride {
    role: &'static str, // canonical id from PALETTE_ROLE_IDS
    fg: Option<TerminalColor>,
    bg: Option<TerminalColor>,
    attrs: Option<StyleAttrs>,
}
```

`ColorOverrides` impl:

- `pub fn is_empty(&self) -> bool`.
- `pub fn set_fg(&mut self, role: &'static str, color: TerminalColor)`,
  `set_bg`, and `pub fn set_attrs(&mut self, role: &'static str, attrs:
  StyleAttrs)` — each finds the existing `RoleOverride` for `role` or pushes a
  fresh one (all-`None`) via a private `entry_mut`, then sets that component.
  Callers pass the canonical `&'static str` (see `canonical_role`).
- `pub fn apply_to(&self, palette: &mut Palette)` — for each entry,
  `if let Some(style) = palette.role_mut(entry.role)` then overwrite only the
  `Some` components (`style.fg = fg`, etc.).

Free helpers (make them `pub(crate)` so tests and parser can call them):

- `pub(crate) fn canonical_role(id: &str) -> Option<&'static str>` —
  `PALETTE_ROLE_IDS.iter().copied().find(|role| *role == id)`.
- `pub(crate) fn parse_color(spec: &str) -> Option<TerminalColor>`:
  - trim; build a lowercased form with `-` and spaces removed for name
    matching (so `bright_blue`, `bright-blue`, `bright blue`, `brightblue`
    all match).
  - `"default"` → `TerminalColor::Default`.
  - all-ASCII-digit → parse `u8`; `0..=255` → `TerminalColor::Indexed(n)`;
    out of range or overflow → `None`.
  - ANSI names → `TerminalColor::Ansi(...)`: `black red green yellow blue
    magenta cyan white brightblack brightred brightgreen brightyellow
    brightblue brightmagenta brightcyan brightwhite`.
  - anything else → `None`.
- `pub(crate) fn format_color(color: TerminalColor) -> String` — inverse:
  `Default` → `"default"`; `Indexed(n)` → `n.to_string()`; `Ansi(c)` → the
  canonical snake name (`black`, …, `bright_black`, …, `bright_white`).
- `pub(crate) fn parse_attrs(spec: &str) -> Option<StyleAttrs>` — split on
  commas and/or whitespace; each token is `bold`/`underline`/`reverse`/`none`.
  `none` (or an empty list) → `StyleAttrs::NONE`; `none` mixed with other
  tokens → `None` (error); unknown token → `None`. Build the struct from the
  set flags.
- `pub(crate) fn format_attrs(attrs: StyleAttrs) -> String` — join the set
  flags with `", "` in the order bold, underline, reverse; `"none"` when no
  flag is set.

Round-trip guarantee: `parse_color(&format_color(c)) == Some(c)` for every
`AnsiColor`, a few `Indexed`, and `Default`; likewise
`parse_attrs(&format_attrs(a)) == Some(a)`.

### 2. `Config` field + layering (`config.rs`)

- Add `pub colors: ColorOverrides` to `Config` (import from `crate::colors`).
- `Default for Config` sets `colors: ColorOverrides::default()`.
- `resolved_theme`:

  ```rust
  pub fn resolved_theme(&self, detected: TerminalProfile) -> Theme {
      let mut theme = Theme::for_profile(self.theme, self.terminal_profile(detected));
      self.colors.apply_to(&mut theme.palette);
      theme
  }
  ```

`Config` keeps `#[derive(Clone, Debug, PartialEq, Eq)]`; `ColorOverrides`
derives the same, so this compiles unchanged.

### 3. Parser wiring (`parser.rs`)

Add ONE arm to `apply_config_entry`'s match, alongside the other prefix arms
(place it before the `key.` arms):

```rust
_ if key.starts_with("color.") => {
    apply_color_override(config, &key["color.".len()..], value, line_number)?;
}
```

New `apply_color_override(config: &mut Config, rest: &str, value: &str,
line_number: usize) -> Result<(), ConfigParseError>`:

- `match rest.rsplit_once('.')`:
  - `Some((role, comp))` where `comp` is `"fg" | "bg" | "attrs"`:
    - `let role = canonical_role(role).ok_or_else(|| ConfigParseError::line(
      line_number, format!("unknown color role `{role}`")))?;`
    - `"fg"`/`"bg"` → `parse_color(value).ok_or_else(|| … "invalid color
      `{value}`" …)?`, then `config.colors.set_fg(role, …)` / `set_bg`.
    - `"attrs"` → `parse_attrs(value).ok_or_else(|| … "invalid attrs
      `{value}`" …)?`, then `config.colors.set_attrs(role, …)`.
  - `Some((_, comp))` (dot present, `comp` not a component) →
    `Err(… "unknown color component `{comp}`; expected fg, bg, or attrs")`.
  - `None` (no dot → shorthand): `let role = canonical_role(rest).ok_or_else(
    … "unknown color role `{rest}`")?;` then parse the shorthand value:
    - `split_once('/')`: if present, left = fg spec, right = bg spec; parse
      both (each may be `default`); `set_fg` + `set_bg`.
    - if absent, the whole value is the fg spec; `set_fg` only.
    - a spec that fails `parse_color` → `Err(… "invalid color `{spec}`")`.

Use `import` `use crate::colors::{canonical_role, parse_attrs, parse_color};`
at the top of `parser.rs`.

### 4. Dump section (`defaults.rs`)

After the keybinding sections, append a commented color section. Resolve the
default theme's 256-color palette for the shown defaults:

```rust
use dun_term::{PALETTE_ROLE_IDS, TerminalProfile};
// …
let palette = config.resolved_theme(TerminalProfile::utf8_256()).palette;
out.push_str(
    "\n# Color overrides (theme defaults shown; uncomment and edit)\n\
# Shorthand `color.<role> = <fg> / <bg>`, or granular `color.<role>.fg`,\n\
# `color.<role>.bg`, `color.<role>.attrs`. A color is a palette index 0-255,\n\
# an ANSI name (red, bright_blue, …), or `default`. Attrs is a comma list of\n\
# bold, underline, reverse, or none.\n",
);
for id in PALETTE_ROLE_IDS {
    let style = palette.role(id).expect("listed role resolves");
    let mut line = format!(
        "# color.{id} = {} / {}",
        format_color(style.fg),
        format_color(style.bg)
    );
    if style.attrs != StyleAttrs::NONE {
        line.push_str(&format!("  # attrs: {}", format_attrs(style.attrs)));
    }
    line.push('\n');
    out.push_str(&line);
}
```

Import `format_color`, `format_attrs`, and `StyleAttrs` as needed
(`use crate::colors::{format_attrs, format_color};` and
`use dun_term::StyleAttrs;`). Keep every emitted color line commented, so the
existing `default_config_text` round-trip test still parses clean.

### 5. `lib.rs` re-export

- Add `mod colors;`.
- Add `pub use colors::ColorOverrides;`.
- The `pub(crate)` helpers do not need root re-export; tests reach them via
  `crate::colors::{…}`.

## Scope

- Files you MAY modify:
  - `crates/dun-config/src/colors.rs` (NEW);
  - `crates/dun-config/src/config.rs`;
  - `crates/dun-config/src/parser.rs`;
  - `crates/dun-config/src/defaults.rs`;
  - `crates/dun-config/src/lib.rs`;
  - `crates/dun-config/src/tests/mod.rs` (register `mod colors;`);
  - `crates/dun-config/src/tests/colors.rs` (NEW);
  - `crates/dun-config/src/tests/config.rs` (only if you add a
    resolved-theme-with-override assertion here).
- Files/areas you MUST NOT touch:
  - `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `PROGRESS.md`, `TODO.md`, `docs/**`,
    `README.md`;
  - `.git`, git config, any `Cargo.toml`, `Cargo.lock` (no new deps);
  - every other crate: `dun-term` (012 is done — do NOT edit it), `dun-cli`,
    `dun-ui`, `dun-core`, `dun-plugin`;
  - `vm-test/**`, `reference/**`, `hosts/**`.

## Deliverable

- `colors.rs` with `ColorOverrides` + the four `pub(crate)` helpers +
  `canonical_role`.
- `Config.colors` field, layered in `resolved_theme`.
- `color.*` parser arm + `apply_color_override`.
- Dump section listing every role, all commented.
- Tests in `crates/dun-config/src/tests/colors.rs` (add
  `use dun_term::{AnsiColor, StyleAttrs, TerminalColor};` at the top; the
  module gets `crate::*` via `use super::support::*;`):
  1. `parse_color_accepts_index_name_and_default` — `"17"` →
     `Indexed(17)`, `"red"` → `Ansi(Red)`, `"bright_blue"`/`"bright-blue"` →
     `Ansi(BrightBlue)`, `"default"` → `Default`; `"256"`, `"-1"`, `"mauve"`
     → `None`.
  2. `parse_attrs_handles_lists_and_none` — `"bold"`, `"bold, underline"`,
     `"bold underline reverse"`, `"none"` (→ `NONE`); `"none, bold"` and
     `"sparkle"` → `None`.
  3. `color_specs_round_trip` — `parse_color(&format_color(c)) == Some(c)`
     for all 16 `AnsiColor`, `Indexed(0)`, `Indexed(231)`, `Default`; and
     `parse_attrs(&format_attrs(a)) == Some(a)` for `NONE`, `BOLD`,
     `{bold+underline+reverse}`.
  4. `granular_override_changes_only_one_component` — parse
     `"color.editor.bg = 17"`, resolve theme (`TerminalProfile::utf8_256()`),
     assert `editor.bg == Indexed(17)` while `editor.fg` and every OTHER role
     equal the un-overridden dun palette.
  5. `shorthand_sets_fg_and_optional_bg` — `"color.warning = 196 / 0"` sets
     both; `"color.dirty = 208"` sets fg only (bg unchanged from theme).
  6. `attrs_override_applies` — `"color.title.attrs = bold, underline"` →
     resolved `title.attrs` has bold && underline.
  7. `overrides_layer_on_selected_theme` — a config with `theme = dark` plus
     `color.warning.fg = 200` resolves to the dark palette with
     `warning.fg == Indexed(200)`.
  8. `unknown_role_and_component_are_line_errors` — `"color.nope.fg = 1"` and
     `"color.editor.glow = 1"` both `Err` with the reported line number.
  9. `dump_lists_color_roles` — `default_config_text()` contains
     `"# Color overrides"` and `"# color.editor = "`, and still
     `parse_config(&text).unwrap().validate().unwrap()` succeeds.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **The 1 MiB dual-platform size budget is real.** Claude gates size. This is
   a small parser + a `Vec` of overrides; no new dependencies.
3. **Overrides apply post-resolution.** Layer onto `theme.palette` AFTER
   `Theme::for_profile`, so a 16-color or mono fallback still receives them.
   Do not gate overrides on the color profile.
4. **Key normalization already ran.** `apply_config_entry` sees a normalized
   key (lowercase, `-`/space → `_`); role ids in `PALETTE_ROLE_IDS` are
   snake_case, so a direct string match works. Do not re-normalize the role.
5. **Tests are layered and colocated** — new tests live in
   `tests/colors.rs`, registered in `tests/mod.rs`.
6. **Minimal diff.** No reformatting or renames outside the change.
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dun-config --no-fail-fast
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
