# Brief 047 — Design plan for restoring bookmarks (F12) and visible whitespace (F13) (design only)

**Diagnostic/design brief. NO source change.** Produce a step-by-step
restoration plan; Claude evaluates it, then dispatches the steps as
implementation briefs and gates each. (Plan-first workflow — see CLAUDE.md.)

## Why (user decision 2026-07-23)

1. The Distinctive Plugins stage closed 2026-07-23; the restoration review
   (TODO.md "Deferred", first item) is the next mainline stage. The review
   decision is made: **restore F12 (bookmarks) + F13 (visible whitespace)**,
   the strongest candidates in docs/feature-triage.md "Restoration Path".
   F46 (advanced Command Output) stays removed — the LogFilter-plugin overlap
   rationale still holds — and F20 (Outline) still returns as a
   `DocumentStructure` plugin role, not a core revert.
2. Both were removed 2026-07-10 at `53fe7f8` (triage batch 3, combined Debian
   delta −12,288 at the time) under pre-build-std size pressure. Post
   build-std, size alone no longer justifies the removal; the binding Debian
   margin is 308,944 bytes.
3. **A plain `git revert 53fe7f8` is dead.** A `git merge-tree` revert
   simulation against HEAD conflicts in 10 of the commit's 26 files (TODO.md,
   `app/buffer_state.rs`, `app/frame.rs`, `app/status_view.rs`,
   `keys/keymap.rs`, `frame/highlight.rs`, `frame/menu.rs`, `frame/text.rs`,
   `dun-ui/src/text.rs`, docs/feature-triage.md), and even the hunks that
   apply cleanly would reintroduce pre-i18n hardcoded English and
   pre-refactor idioms. Since the removal, the codebase went through: the
   full UI i18n conversion (every user-visible string is a key + ten
   translation files), the ratatui → Surface renderer replacement, the
   wide-geometry/ambiguous-width work in the dun-ui text layer, the
   `Ctrl+W,*` → `Ctrl+X,*` keymap family rename, plugin menu injection and
   plugin keybinding leaders, and the crossterm replacement. Therefore:
   **`53fe7f8^` is the behavior specification, not a patch source** — the
   features are re-landed against today's architecture, full-trail.

## Claude's decisions (bake these into the plan — not open questions)

- **Both features return, full-trail** (AGENTS.md): code + command ids +
  keymap defaults + menu entries + help text + tests + README/docs — the
  exact reverse of the removal trail.
- **Reuse the original command vocabulary** unless today's tables make it
  impossible (report if so): `EditCommand::{ToggleVisibleWhitespace,
  ToggleBookmark, NextBookmark, PreviousBookmark}` and their original
  command ids / command-line aliases from `53fe7f8^`.
- **i18n is mandatory and same-step.** Every user-visible string lands as
  keys (`menu.*`, `help.command.<id>`, `status.*`) with English defaults plus
  ALL TEN `i18n/*.conf` translation files updated in the same step, so the
  completeness tests (`every_shipped_translation_is_valid_and_complete`,
  help/menu key tests) stay green at every commit. Machine translation is
  acceptable (matches the shipped files' stated status). The vocabulary rule
  holds: command ids and key caps stay English; only prose translates.
- **Keymap: the old chords do not transplant 1:1.** The old defaults were
  `Ctrl+W,.` (toggle whitespace), `Ctrl+W,M` (toggle bookmark), `Ctrl+W,N`
  (next), `Ctrl+W,P` (previous). The prefix family is now `Ctrl+X,*`
  (`Ctrl+W` is single-stroke Find). Today `Ctrl+X,.` and `Ctrl+X,N` are
  free; `Ctrl+X,M` and `Ctrl+X,P` are TAKEN (Window Collapse/Expand,
  `keys/keymap.rs:195-196`). The plan must propose default chords for all
  four commands with a complete collision inventory of the current default
  keymap and mnemonic rationale; Claude picks from the proposal. (New
  built-in chords under the existing `Ctrl+X` prefix cannot be shadowed by
  plugin leaders — built-ins win by construction — but note any interaction
  you find anyway.)
- **Rendering must be wide-aware.** The F13 whitespace transform and F12
  gutter markers integrate with today's `dun-ui` frame/text/gutter/highlight
  code (post-Surface, post-wide-geometry), not the 07-10 shapes. Display
  width, not char count; the sanitizer and soft-wrap paths must keep their
  invariants.
- **Safe Rust, no new dependencies, size measured per batch** (macOS proxy +
  binding Debian, release smoke on runtime commits). Restored tests return
  where still meaningful, adapted to today's module layout; name the
  mutation targets for every invariant-guarding test (Claude mutation-proves
  them at the gate).

## What exists (read before planning)

- **The spec:** `git show 53fe7f8` (the full 26-file removal diff) and the
  pre-removal tree via `git show 53fe7f8^:<path>`. Inventory rows:
  docs/feature-triage.md:125-126.
- **Today's landscape** (all changed after the removal — verify each with
  `path:line`, do not trust this summary): i18n key tables in
  `crates/dun-cli/src/ui_text/` + `crates/dun-ui/src/frame/menu.rs` menu
  keys + `crates/dun-cli/src/help/content.rs` help keys, ten files under
  `i18n/`; `EditorCommand` (~112 variants, `crates/dun-core/src/command.rs`)
  with `dun-config` command-id tables and `ALL_COMMAND_IDS` round-trip
  tests; the default keymap `crates/dun-config/src/keys/keymap.rs`
  (`Ctrl+X` family); per-buffer state in
  `crates/dun-cli/src/app/buffer_state.rs`; status fields in
  `crates/dun-cli/src/app/status_view.rs`; the wide-aware render layer in
  `crates/dun-ui/src/{text.rs,frame/text.rs,frame/gutter.rs,frame/highlight.rs}`;
  plugin menu injection appended after built-in menus and plugin keybinding
  leaders consulted after the built-in keymap.
- The test safety net: 780/0 workspace tests on four platforms at `877b7ad`;
  golden frame snapshots; tmux/PTY harnesses; completeness/uniqueness tests
  over the i18n tables and menu mnemonics.

## The plan must address each

1. **Feature spec extraction.** The exact behavior of F12 and F13 from
   `53fe7f8^`, `path:line` into the old tree: commands and aliases,
   per-buffer bookmark state and its lifecycle (file reload? buffer close?),
   gutter marker rendering, whitespace display transforms (which characters,
   which paths), status brackets, menu entries and mnemonics, help entries,
   and every test that pinned them.
2. **Per-piece mapping onto today's code.** For each spec piece: the target
   file/function today (`path:line`), what replaced or absorbed the old
   location, and revert-vs-rewrite per piece with a one-line reason.
3. **i18n key plan.** Every new key with its table file, English default,
   which completeness test binds it, and the ten-translation batch shape.
4. **Keymap proposal.** Default chords for the four commands: collision
   inventory of the whole current default keymap, proposed primary +
   alternative per command, mnemonic rationale.
5. **Ordered steps (likely 2–4), each its own implementation brief.**
   Sequence conservatively — F13 (render-layer toggle, no persistent state)
   before F12 (state + commands + gutter + status) unless the inventory
   argues otherwise. Per step: files/functions, gate tests, what can regress
   if done wrong, and where the dual-platform size measurements land.
6. **Test plan.** Which removed tests return (and their new homes), what new
   coverage today's rules demand (wide chars/tabs under visible whitespace,
   soft-wrap interplay, snapshot/golden updates, bookmark state across
   reload/close), and the named mutation targets per invariant.
7. **Risks / open questions for Claude** — including: does the old
   status-bracket layout still fit today's `status_view` field set; does
   `ToggleVisibleWhitespace` interact with the sanitizer, soft-wrap, or
   horizontal-scroll caps; gutter width interaction with line numbers and
   wide glyphs; anything in the old behavior today's architecture makes
   wrong (search, don't assume).

## Scope

- Files you MAY modify: **NONE — design only.** Leave the tree clean
  (`git status --short` empty when done). Read anything, including git
  history (`git show`, `git log -S`); run read-only commands; do not
  `cargo build`/edit.

## Hard rules

- Do NOT edit any source file, commit, branch, push, or touch git state.
- Base every claim on real files (`path:line` — today's tree) or explicit
  `53fe7f8^:<path>` references (the old tree); do not hand-wave the mapping
  or the keymap collision inventory.
- Safe Rust only in the design — if some piece seems to require `unsafe`,
  that is an open question for Claude, not a design choice.

## Report format (final message)

The seven-part plan above, concrete enough that each step could become its
own implementation brief without further discovery.
