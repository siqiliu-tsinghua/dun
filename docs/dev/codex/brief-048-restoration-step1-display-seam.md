# Brief 048 — F12/F13 restoration step 1: shared display-coordinate seam

Implementation brief. **Step 1 of the accepted plan produced for brief 047**
(the F12/F13 restoration track; see
`docs/dev/codex/brief-047-f12-f13-restoration-plan.md` for the fixed
constraints and Claude's frozen decisions). This step builds the seam and
splits the oversized files; it implements **no user-visible command** — F13
lands in step 2, F12 in step 3.

## Goal

One authoritative source-byte↔display-cell mapping (`EditorTextDisplay` in
`dun-ui`) that every body-text consumer routes through, plus a mapped
sanitizer entry point in `dun-core` and whitespace marker glyphs in
`dun-term`, with the `visible_whitespace` flag threaded to the seam
(default false, nothing sets it yet). **Behavior is identical:** every
existing test passes unchanged, golden frame snapshots are byte-identical,
no new `EditorCommand`, no keymap/menu/help/i18n change. The oversized
`app/editing.rs` and `app/buffer_state.rs` get their planned splits as pure
moves.

## Exact change

1. **`crates/dun-term/src/glyphs.rs`** — add `WhitespaceGlyphs { space, tab,
   end_of_line }` to `GlyphSet`: UTF-8 profile `·`/`→`/`¶`, ASCII profile
   `.`/`>`/`$` (spec: `53fe7f8^:crates/dun-ui/src/text.rs:17-47`). Selection
   stays profile-owned like the existing glyph families.
2. **`crates/dun-core/src/display.rs`** — a mapped-character sanitizer entry
   point beside the existing one. Invariants (frozen, brief 047 risk 4):
   count the ORIGINAL source UTF-8 bytes against `max_bytes` (never the
   mapped output); map only accepted source characters; mapped and unmapped
   output goes through the existing classification; append the logical
   suffix (EOL marker) only when the source line was NOT truncated;
   `SanitizedLine::{bytes_consumed,truncated}` stay in source coordinates.
3. **New `crates/dun-ui/src/display_map.rs`** — `EditorTextDisplay`,
   constructed from `DisplaySanitizer`, ambiguous-width mode, glyphs, and
   the visible-whitespace flag. It owns the one implementation of:
   source byte → display column; display column → valid source boundary;
   sanitized line output; line display width; wrapped segments/row count;
   wrapped row+column → source position; logical EOL mapping to
   `line.len()`. All width via `dun_term` char-width (ambiguous-aware);
   no literal `+1` cell math outside source-byte accounting.
4. **Route every consumer through it**: `frame/text.rs`, `frame/cursor.rs`,
   `frame/highlight.rs` (selection/search/plugin spans), `frame/scroll.rs`,
   `hit.rs` (and the `app/mouse.rs` call sites), replacing their per-file
   raw-width/wrap helpers. `BufferView::with_visible_whitespace` lands
   default-false so renderer tests can exercise the seam directly.
5. **`crates/dun-cli` splits (pure moves + call-site updates, no logic
   edits)**: viewport/wrap extension methods out of `app/buffer_state.rs`
   (21.5k) into new `app/buffer_viewport.rs`, taking the shared display
   value instead of `AmbiguousWidth`-only arguments; viewport-oriented
   handlers out of `app/editing.rs` (26.8k) into new `app/view_commands.rs`.
   Wire modules in `app/mod.rs`; adapt mouse, search/replace, helper-pane,
   status, and frame call sites. Remove or narrow the duplicate raw-width
   helpers in `files/text.rs`.
6. **Docs**: update `AUDIT.md`, `docs/dev/crate-map.md`, and
   `docs/dev/code-organization-guidelines.md` for the seam boundary and the
   completed splits — factual, minimal paragraphs.

## Scope

- Files you MAY modify: `crates/dun-term/src/glyphs.rs`,
  `crates/dun-core/src/display.rs`, `crates/dun-ui/src/**` (including the
  new `display_map.rs` and `tests/`), `crates/dun-cli/src/app/**`
  (including the two new modules), `crates/dun-cli/src/files/text.rs`,
  colocated `dun-core`/`dun-term` tests, `AUDIT.md`, `docs/dev/crate-map.md`,
  `docs/dev/code-organization-guidelines.md`.
- Files/areas you MUST NOT touch: `crates/dun-core/src/command.rs`,
  `crates/dun-config/**`, `i18n/**`, `hosts/**`, help/menu/status text of
  any kind, any `Cargo.toml`/`Cargo.lock`, `AGENTS.md`, `CLAUDE.md`,
  `README.md`, `PROGRESS.md`, `TODO.md`, other docs, `.git`, `vm-test/**`,
  `reference/**`.

If a change needs a file outside Scope, STOP and report.

## Deliverable

- The seam (`display_map.rs` + mapped sanitizer + glyphs) with every listed
  consumer rerouted; the two `dun-cli` module splits; the three doc updates.
- New unit tests with **independent oracles** (hard-coded expected
  coordinates/strings — never derived from the implementation):
  - mapped sanitizer: raw-byte cap (a one-byte cap on a space maps the
    marker, reports `bytes_consumed == 1`, sets `truncated`, appends no
    EOL); classification preserved for mapped output.
  - `EditorTextDisplay`: hard-coded Narrow AND Wide coordinates for `中`,
    space, tab, and the three marker glyphs (`·`/`→`/`¶` are East Asian
    Ambiguous: two cells in Wide); display-column → source-boundary
    round-trips on multi-byte text; wrapped row+column → source position;
    EOL maps to `line.len()`.
  - default-off parity: with the flag false, seam output equals today's
    sanitizer output for a representative corpus (wide, tab, control-byte,
    long-line cases).
- Prove load-bearing (run these yourself, then reverse the edit — never
  `git checkout`): (a) cap transformed bytes instead of raw source bytes →
  a test fails; (b) treat an ambiguous marker as one cell in Wide mode → a
  test fails; (c) return a non-boundary from display-column → source →
  a test fails.

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`).
2. **Byte-identical is the tripwire.** Every existing test and golden frame
   must pass UNCHANGED. If any reroute (mouse hits, plugin highlight
   geometry, scrollbar math) seems to require changing an existing
   assertion, that is a behavior change — STOP and report; do not adjust
   the assertion.
3. The splits are pure moves: relocated functions keep their bodies; only
   signatures change where the shared display value replaces
   `AmbiguousWidth` arguments, and call sites follow.
4. All untrusted text still goes through the sanitizer; the mapped entry
   must not open a bypass (mapped output re-enters classification).
5. Long-line caps: the seam must not allocate proportional to full
   untruncated lines; the existing display-work caps stay in force.
6. Stop-loss: same failure twice, or an out-of-scope file needed → STOP,
   report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Note the pty_smoke and tmux suites' results explicitly (tmux/expect needed —
say so if absent rather than reporting green). Claude runs the macOS budget
build, the binding Debian measurement, and release smoke at the gate.

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working
  tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format (final message)

1. What changed — per file, line ranges, one-line why.
2. Verification — each command's verbatim output (suite counts; PTY/tmux
   noted).
3. Mutation evidence — the three load-bearing runs, verbatim.
4. Verdict.
5. Stop-loss / open questions (empty if none).
