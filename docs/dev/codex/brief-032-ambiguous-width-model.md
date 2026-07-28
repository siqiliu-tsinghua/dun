# Brief 032 — Configurable ambiguous-width model (stage A)

Implementation brief. Pre-diagnosed mechanical change. The named tests decide.

## Goal

`dun` must be able to lay out and render using either of the two legitimate
interpretations of Unicode *East Asian Ambiguous*-width characters (the
box-drawing block U+2500–257F, geometric shapes like U+25C6 `◆`, etc.): the
Western/default reading (Ambiguous = 1 column, what `dun` does today) or the
East-Asian reading (Ambiguous = 2 columns, what Solaris tmux and CJK-configured
terminals use). When a host terminal treats these glyphs as double-width,
`dun`'s box-drawing UI currently overflows and clips (see `docs/dev/solaris-vm.md`).

This brief adds a **width mode carried on `TerminalProfile`** and routes every
character-display-width computation through **one pair of central functions**
that honor it. It adds a config option `terminal.ambiguous-width = narrow |
wide` (default `narrow`, i.e. today's behavior). It does **not** add runtime
auto-detection — that is a separate later brief. When you are done, setting
`terminal.ambiguous-width = wide` makes `dun` keep its UTF-8 box glyphs but
budget each ambiguous glyph as 2 columns, so a border fills the pane exactly on
a terminal that renders those glyphs double-width.

`unicode-width` (already a workspace dependency) provides both readings:
`UnicodeWidthChar::width` / `UnicodeWidthStr::width` are the narrow reading and
`::width_cjk` are the wide reading. Use them; do not hand-roll width tables.

## Context pointers

- Read `AGENTS.md` (invariants) and this file fully before touching anything.
  Also skim `docs/dev/solaris-vm.md` for the motivating quirk.
- Crate dependency direction: `dun-term` is the lowest layer; `dun-config`,
  `dun-ui`, `dun-cli` all depend on it. So the width mode enum and the two
  central width functions live in **`dun-term`**, and everyone calls them.
- `UiShell` already has `pub profile: TerminalProfile` (`dun-ui/src/shell.rs`),
  and `AppState` reaches it as `self.shell.profile`. That field is the source of
  the width mode at every call site.
- Acceptance is mechanical: the named tests decide, not prose.

## Design (implement exactly this)

### 1. `dun-term` — the mechanism

- `crates/dun-term/Cargo.toml`: add `unicode-width.workspace = true` to
  `[dependencies]` (this dependency already exists in the workspace and in
  `dun-ui`/`dun-cli`; you are only letting `dun-term` use it too). This is the
  one allowed `Cargo.toml` edit.
- New public enum in `crates/dun-term/src/profile.rs` (next to
  `EncodingProfile`):
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
  pub enum AmbiguousWidth {
      #[default]
      Narrow,
      Wide,
  }
  ```
- Add a field to `TerminalProfile`: `pub ambiguous_width: AmbiguousWidth`.
  Update `TerminalProfile::new` and every other constructor
  (`utf8_256`, `utf8_16`, `ascii_16`, `ascii_mono`, `vt100`, the `Default` impl,
  and `from_capabilities`) to set it to `AmbiguousWidth::Narrow`. Do **not**
  change `from_capabilities`'s detection logic otherwise — stage A ships Narrow
  by default; auto-detection is a later brief. If `new` takes positional args
  today, prefer adding a defaulted field and constructing with `..` where
  ergonomic, or extend the signature — your choice, but keep every existing
  caller compiling and Narrow.
- Two central functions, public from `dun-term` (module of your choice, e.g. a
  new `width.rs`, re-exported from the crate root next to `TerminalProfile`):
  ```rust
  pub fn char_width(ch: char, mode: AmbiguousWidth) -> Option<usize>
  pub fn str_width(text: &str, mode: AmbiguousWidth) -> usize
  ```
  Implement with `match mode { Narrow => UnicodeWidthChar::width(ch) / UnicodeWidthStr::width(text), Wide => ::width_cjk(..) }`.
  These are the ONLY place in the whole workspace allowed to call
  `unicode-width` directly.
- Export `AmbiguousWidth`, `char_width`, `str_width` from `dun-term/src/lib.rs`.

### 2. `dun-config` — the option

- `TerminalOverrides` (in `dun-config/src/config.rs`): add
  `pub ambiguous_width: Option<AmbiguousWidth>` and apply it in `apply_to`
  exactly like `encoding`.
- `dun-config/src/parser.rs`: parse `terminal.ambiguous-width` with a
  `parse_ambiguous_width(value) -> Option<AmbiguousWidth>` accepting `"narrow"`
  and `"wide"`, following the exact shape of the existing
  `terminal.encoding` / `parse_encoding_profile` handling (unknown value → the
  same style of `ConfigParseError::line(line_number, "unknown terminal ambiguous width")`).
  This error is a plain `ConfigParseError`, not i18n — do NOT touch `i18n/` or
  `ui_text`.
- `dun-config/src/defaults.rs`: add a commented default line under the existing
  `# Terminal fallback overrides` block: `# terminal.ambiguous-width = narrow`.
- Re-export `AmbiguousWidth` from `dun-config` if that matches how
  `EncodingProfile` is surfaced (it re-exports `dun-term` types in
  `dun-config/src/lib.rs`); mirror that.

### 3. `dun-ui` — route rendering through the mode

The rule: **every** current `UnicodeWidthChar::width` / `UnicodeWidthStr::width`
call in `dun-ui` becomes `dun_term::char_width(ch, mode)` /
`dun_term::str_width(s, mode)` with the mode threaded in. Remove the
`use unicode_width::...` imports from these files once no direct call remains.

- `crates/dun-ui/src/surface.rs`: **do NOT change the `Surface::new` signature**
  (it has ~40 call sites, almost all tests, and changing it would cascade out of
  scope). Instead: add an `ambiguous_width: AmbiguousWidth` field, initialized to
  `AmbiguousWidth::Narrow` in `new` (so every existing `Surface::new(w, h, style)`
  caller keeps working unchanged), plus a builder
  `pub(crate) fn with_ambiguous_width(mut self, mode: AmbiguousWidth) -> Self`
  that sets it. `set_text` (line ~60) and `fill_rect` (line ~119) use
  `dun_term::char_width(ch, self.ambiguous_width)`. This keeps the mode on the
  struct without touching any `Surface::new` call.
- `crates/dun-ui/src/render/surface_frame.rs`: `SurfaceRenderer::render` builds
  the `Surface` — chain `.with_ambiguous_width(shell.profile.ambiguous_width)`
  onto the `Surface::new(...)` there (line ~44). The two `Surface::new` calls in
  that file's tests keep the default (Narrow) — leave them.
- `crates/dun-ui/src/snapshot.rs`: `frame_snapshot` builds a `Surface` (line
  ~15) and has `shell` — chain `.with_ambiguous_width(shell.profile.ambiguous_width)`
  the same way so a snapshot honors the profile's mode.
- **All other `Surface::new` call sites stay as they are** (tests in
  `surface.rs`, `surface_emit.rs`, `render/surface_layers.rs`,
  `render/surface_draw.rs`, `tests/**`): they get the default Narrow mode and
  need no edit. Only the production render + snapshot paths opt into the mode.
- `crates/dun-ui/src/text.rs`, `crates/dun-ui/src/frame/text.rs`,
  `crates/dun-ui/src/render/surface_window.rs`: each width helper takes an
  `AmbiguousWidth` parameter (thread it from the caller, which has access to the
  shell/profile) and calls the central functions. Follow the call chain up to a
  point that has the profile; if a helper is called from several places, add the
  parameter and update all callers.

### 4. `dun-cli` — route the remaining call sites

Same rule, mode sourced from `self.shell.profile.ambiguous_width` (or the
nearest available `UiShell`/`TerminalProfile`):

- `crates/dun-cli/src/app/buffer_state.rs` (lines ~318, ~326)
- `crates/dun-cli/src/app/status_view.rs` (line ~72)
- `crates/dun-cli/src/files/text.rs` (lines ~20, ~30)
- `crates/dun-cli/src/dialogs/line_input.rs` (lines ~30, ~31)
- `crates/dun-cli/src/help/status.rs` (line ~23)
- `crates/dun-cli/src/help/content.rs` (line ~188)
- `crates/dun-cli/src/main.rs` line ~33: drop the
  `use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};` prelude import once
  no `dun-cli` module calls `unicode-width` directly.

If any one of these call sites cannot reach a `TerminalProfile` cleanly without
a large signature cascade, STOP and report it (do not invent a global) — but
most are inside `AppState`/render methods that already have `self.shell`.

### Completeness invariant (you must satisfy this)

After your change, this command must print **only** lines inside
`dun-term`'s own width module (the central functions):

```
grep -rn 'UnicodeWidthChar::width\|UnicodeWidthStr::width\|\.width_cjk\|unicode_width' crates/*/src --include=*.rs | grep -v '/tests/'
```

Every other `.width()` you see in the tree is `Rect`/`Surface`/`area` geometry
(fields and methods named `width`) — leave those alone. Only the
`unicode-width`-crate calls move.

## Scope

- Files you MAY modify:
  - `crates/dun-term/Cargo.toml` (the one allowed dependency edit: add
    `unicode-width.workspace = true`)
  - `crates/dun-term/src/profile.rs`, `crates/dun-term/src/lib.rs`, and a new
    `crates/dun-term/src/width.rs` if you choose that layout
  - `crates/dun-config/src/config.rs`, `.../parser.rs`, `.../defaults.rs`,
    `.../lib.rs`
  - `crates/dun-ui/src/surface.rs`, `.../snapshot.rs`, `.../text.rs`,
    `.../frame/text.rs`, `.../render/surface_window.rs`,
    `.../render/surface_frame.rs`
  - `crates/dun-cli/src/app/buffer_state.rs`, `.../app/status_view.rs`,
    `.../files/text.rs`, `.../dialogs/line_input.rs`, `.../help/status.rs`,
    `.../help/content.rs`, `.../main.rs`
  - test modules colocated with the above (e.g. `src/tests/**`, in-file
    `#[cfg(test)]`) needed for the named tests
- Files/areas you MUST NOT touch:
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `TODO.md`, `docs/**`, `i18n/**`,
    `crates/dun-cli/src/ui_text/**`
  - any other `Cargo.toml` or `Cargo.lock`; `.git`; `vm-test/**`;
    `reference/**`; `hosts/**`
  - `from_capabilities`'s detection heuristics (only add the defaulted field)

## Deliverable

- The `AmbiguousWidth` mechanism, config option, and full call-site cutover.
- Tests (colocated, matching local style):
  1. **dun-term width functions.** `char_width('─', Narrow) == Some(1)` and
     `char_width('─', Wide) == Some(2)`; same contrast for `'◆'` (U+25C6);
     an ASCII char is 1 in both; a genuinely Wide CJK char (e.g. `'中'`) is 2
     in both. `str_width` mirrors this for a short mixed string.
  2. **dun-config parse.** `terminal.ambiguous-width = wide` yields
     `AmbiguousWidth::Wide` in the resolved `TerminalProfile`; `narrow` yields
     Narrow; an unknown value is a parse error.
  3. **Surface layout invariant (the load-bearing one).** With a `Surface`
     built in `Wide` mode, `set_text(0, 0, "──────────", style)` advances the
     column by **20** (ten ambiguous glyphs × 2), versus **10** in `Narrow`
     mode; and the ten glyphs occupy 20 cells (each `─` followed by a
     `wide_continuation` cell) without overflowing a 20-wide surface. This is
     the invariant that keeps a wide-terminal border from overflowing.

## dun pitfalls (read twice)

1. **Safe Rust only** — every crate root has `#![forbid(unsafe_code)]`.
2. **1 MiB dual-platform size budget is real** — Claude gates it. `width_cjk`'s
   table already ships (unicode-width includes it), so this should be close to
   byte-neutral; keep the diff mechanical, add no new dependencies beyond
   letting `dun-term` use the existing `unicode-width`.
3. **`dun-cli/src/main.rs` is the prelude hub** — modules use `use crate::*`;
   update its import list in the same change (you are removing a `unicode_width`
   import there).
4. **Tests are layered and colocated** — match the local style of each file.
5. **Stop-loss** — if the same step fails twice for the same reason, STOP and
   report. The `Surface` mode is added via a `with_ambiguous_width` builder
   precisely so no `Surface::new` call site changes; if you instead find the
   `text.rs`/`frame/text.rs`/`surface_window.rs` helper's `AmbiguousWidth`
   parameter cascading into a file not in Scope, STOP and report rather than
   widening scope.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Also paste the output of the completeness-invariant grep above; it must show
only the central functions in `dun-term`.

Loop: edit → test → fix → rerun, until green. The tmux-backed suite needs tmux
and `/usr/bin/python3`; if unavailable those tests skip cleanly — say so rather
than reporting them green.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave changes in the
  working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and report.
- Minimal diff: no drive-by reformatting or unrelated renames.
- Paste real verbatim verification output. If a run is not green, say so.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command with verbatim output (suite counts; note tmux/
   python skips); plus the completeness-grep output.
3. The finding / verdict.
4. Stop-loss / open questions (empty if none).
