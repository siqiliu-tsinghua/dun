# Brief 049 — F12/F13 restoration step 2: visible whitespace (F13), full trail

Implementation brief. **Step 2 of the accepted plan produced for brief 047**
(the F12/F13 restoration track; see
`docs/dev/codex/brief-047-f12-f13-restoration-plan.md` for the spec and
Claude's frozen decisions, and `brief-048` for the display seam this builds
on). This step restores **F13 visible whitespace only** — bookmarks (F12) are
step 3. `53fe7f8^` is the behavior spec; do not `git revert`.

## Goal

`edit.toggle_visible_whitespace` returns as a per-buffer display toggle that
renders space/tab/logical-EOL markers through the step-1 `EditorTextDisplay`
seam, on the full trail: command variant + id + default keymap + menu entry +
help entry + command-line alias + `[Whitespace]` status bracket + six i18n
keys translated in all ten `i18n/*.conf` files. English default behavior
matches `53fe7f8^` exactly. The flag is per-`BufferState`, default off, never
serialized, never dirties the buffer.

## Exact change

1. **`crates/dun-core/src/command.rs`** — add `ToggleVisibleWhitespace` to
   `EditCommand` beside `ToggleWordWrap` (spec: `53fe7f8^:...command.rs:38`).
2. **`crates/dun-config/src/commands.rs`** — map it to id
   `edit.toggle_visible_whitespace` in all three tables (parse, to-id,
   `ALL_COMMAND_IDS`); bump the edit-variant round-trip count 44 → 45
   (`crates/dun-config/src/tests/keys.rs:247` area).
3. **`crates/dun-config/src/keys/keymap.rs`** — default binding `Ctrl+X,.`
   (Claude's decision; verified free) directly after the `Ctrl+X,Z`
   Word-Wrap line (~keymap.rs:68).
4. **`crates/dun-cli/src/app/buffer_state.rs`** — add `visible_whitespace:
   bool` to `BufferState` beside `word_wrap`; constructors default it `false`.
5. **Lifecycle** (`crates/dun-cli/src/app/file_io.rs`,
   `crates/dun-cli/src/app/windows.rs`): Reload preserves the flag into the
   replacement `BufferState`; New/Open create it `false`; the flag dies with
   the buffer on final close (mirror `word_wrap`'s exact lifecycle — find
   where `word_wrap` is carried/reset and do the same, no more).
6. **Toggle handler** — in `crates/dun-cli/src/app/view_commands.rs` (the
   step-1 module), dispatch `ToggleVisibleWhitespace`: flip the focused
   buffer's flag, re-normalize the viewport through the shared display value
   (cursor/source position preserved), and set the status via the new keys.
   Statuses (spec `53fe7f8^:...editing.rs:352-364`): missing-buffer →
   `status.whitespace.buffer-missing`; on → `status.whitespace.on`; off →
   `status.whitespace.off`.
7. **`crates/dun-cli/src/app/frame.rs`** — thread
   `buffer.visible_whitespace` into the `BufferView`
   (`BufferView::with_visible_whitespace`, already on the type from step 1).
8. **Status bracket** — `crates/dun-cli/src/app/status_view.rs`: insert
   `[Whitespace]` at the old index-4 position when the focused buffer has the
   flag on (spec `53fe7f8^:...status_view.rs:87-104`), via
   `status.detail.whitespace`.
9. **Command-line alias** — `crates/dun-cli/src/app/command_line.rs`: add
   `"whitespace"` → `run_no_arg_command(..., ToggleVisibleWhitespace)` beside
   the `"wrap"` arm (~line 49); add `"whitespace"` to the completion list in
   `crates/dun-cli/src/command_line.rs` beside `"wrap"` (~line 228).
10. **Menu** — `crates/dun-ui/src/frame/menu.rs`: a `VIEW_ENTRIES` entry
    `menu.view.visible-whitespace` / English `Visible Whitespace (.)` /
    `ToggleVisibleWhitespace`, inserted directly after Word Wrap (menu.rs:212)
    and before Scroll Left. Mnemonic `.` (verified unused in the View menu).
11. **Help** — `crates/dun-cli/src/help/content.rs`: description key
    `help.command.edit.toggle_visible_whitespace`, English
    `Toggle visible whitespace`, placed right after Toggle Word Wrap.
12. **i18n table + translations** — add the six keys with English defaults to
    their source tables (`menu.view.*` in menu.rs; `help.command.*` derived
    from the id; the three `status.whitespace.*` + `status.detail.whitespace`
    in `crates/dun-cli/src/ui_text/status/edit.rs`), and add all six to every
    file under `i18n/` (de, es, fr, it, ja, ko, pt, ru, zh-Hans, zh-Hant) in
    the SAME commit so the completeness tests stay green. Machine translation
    is acceptable; keep command ids, key caps, and the mnemonic letter
    English (only prose translates).

Keys and English defaults (exact):

| Key | English |
| --- | --- |
| `menu.view.visible-whitespace` | `Visible Whitespace (.)` |
| `help.command.edit.toggle_visible_whitespace` | `Toggle visible whitespace` |
| `status.whitespace.buffer-missing` | `Whitespace failed: focused buffer is missing` |
| `status.whitespace.on` | `Visible whitespace on` |
| `status.whitespace.off` | `Visible whitespace off` |
| `status.detail.whitespace` | `Whitespace` |

## Scope

- Files you MAY modify: the items 1–12 above and their colocated tests, plus
  `i18n/*.conf`. New focused test module `crates/dun-cli/src/tests/markers.rs`
  is allowed (wire it in the test mod).
- Files/areas you MUST NOT touch: anything bookmark/F12 (that is step 3),
  `crates/dun-ui/src/display_map.rs` and the seam internals (step 1 is
  frozen; only *use* `with_visible_whitespace`), any `Cargo.toml`/`Cargo.lock`,
  `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`, other docs,
  `.git`, `hosts/**`, `vm-test/**`, `reference/**`.

If a change needs a file outside Scope, STOP and report.

## Deliverable

- The full-trail F13 restoration + the six-key ten-file i18n batch.
- Tests (independent oracles; restore the `53fe7f8^` assertions adapted to
  today's layout, in `tests/markers.rs` and `crates/dun-ui/src/tests/`):
  - Exact UTF-8 `·`/`→`/`¶` and ASCII `.`/`>`/`$` output incl. an empty
    line's lone EOL marker.
  - Default-off parity: a buffer with the flag off renders byte-identically
    to before (no marker).
  - Toggle flips per-buffer state, sets the exact English statuses; a second
    buffer is unaffected (no leak).
  - Reload preserves the flag; New/Open start off.
  - `[Whitespace]` appears only with the flag on, at the old ordering
    position.
  - Command `whitespace` and the `Ctrl+X,.` binding both dispatch the toggle;
    completion advertises `whitespace`; command-id round-trip holds.
  - A translated-status call-site test in at least one shipped language.
  - Update `help_screen.txt`/`menu_matrix.txt` goldens and add a
    visible-whitespace frame snapshot; inspect every changed golden.
- Prove load-bearing (run yourself, then reverse the edit — never
  `git checkout`): (a) make the toggle set the flag on the wrong buffer → the
  no-leak test fails; (b) drop the `[Whitespace]` bracket insertion → the
  status-ordering test fails; (c) replace a translated status with hardcoded
  English → the call-site language test fails.

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`).
2. The 1 MiB dual-platform budget is real; keep the diff minimal, no new
   dependencies, no broad new generic instantiations. Claude runs the
   dual-platform size gate.
3. All rendered text goes through the seam/sanitizer — markers are produced
   by the seam (step 1), never by concatenating raw glyphs downstream.
4. i18n completeness tests fail the build if any of the six keys is missing
   from any of the ten files or the source table; add them together.
5. The flag must not dirty the buffer or serialize; it is view state, exactly
   like `word_wrap`.
6. Stop-loss: same failure twice, or an out-of-scope file needed → STOP,
   report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Note the pty_smoke and tmux suites' results explicitly. Claude runs the
macOS budget build, the binding Debian measurement, and release smoke at the
gate.

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
