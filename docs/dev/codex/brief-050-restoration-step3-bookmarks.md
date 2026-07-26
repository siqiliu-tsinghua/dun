# Brief 050 — F12/F13 restoration step 3: bookmarks (F12), full trail

Implementation brief. **Step 3 (final) of the accepted plan produced for
brief 047** (see it for the spec and Claude's frozen decisions; `brief-048`
seam, `brief-049` F13 precede this). Restores **F12 bookmarks only**.
`53fe7f8^` is the behavior spec; do not `git revert`.

## Goal

Per-buffer line bookmarks return on the full trail: toggle/next/previous
commands, a `*` gutter marker, a `[Mark]` status bracket, strict circular
navigation with column clamping, and normalization on reload / delete-line /
move-line — matching `53fe7f8^` exactly, re-landed against today's
architecture. Bookmarks are per-`BufferState` sorted `Vec<usize>` (0-based
logical line indices), initially empty, view state (never dirties, never
serialized), shared across split views of one `BufferId` and reaped with the
buffer.

## Exact change

1. **`crates/dun-core/src/command.rs`** — add `ToggleBookmark`, `NextBookmark`,
   `PreviousBookmark` to `EditCommand` (spec `53fe7f8^:...command.rs:39-41`).
2. **`crates/dun-config/src/commands.rs`** — ids `edit.toggle_bookmark`,
   `edit.next_bookmark`, `edit.previous_bookmark` in all three tables; bump the
   edit-variant round-trip count 45 → 48 (`tests/keys.rs`).
3. **`crates/dun-config/src/keys/keymap.rs`** — defaults (Claude's decision,
   all verified free): `Ctrl+X,K` → ToggleBookmark, `Ctrl+X,N` → NextBookmark,
   `Ctrl+X,L` → PreviousBookmark, placed after the `Ctrl+X,.` line.
4. **`crates/dun-term/src/glyphs.rs`** — add `bookmark: char` to
   `IndicatorGlyphs`; `*` in BOTH the unicode and ASCII profiles (spec used a
   literal `*`, `53fe7f8^:...gutter.rs:37-38`).
5. **`crates/dun-cli/src/app/buffer_state.rs`** — add `bookmarks: Vec<usize>`
   beside `visible_whitespace`, default empty; port `normalize_bookmarks`
   (clamp each index to the last existing line, sort, dedup —
   `53fe7f8^:...buffer_state.rs:526-533`) and the line-move remap helper
   (`53fe7f8^:...buffer_state.rs:535-550`).
6. **Handlers** — put toggle/next/previous in a focused module (extend
   `app/view_commands.rs`; do NOT grow `editing.rs`):
   - toggle: add or remove the cursor's 0-based line, keep sorted; report
     1-based `status.bookmark.added` (`Bookmarked line {}`) /
     `status.bookmark.removed` (`Removed bookmark at line {}`); missing focused
     buffer → `status.bookmark.buffer-missing`
     (`53fe7f8^:...editing.rs:366-385`).
   - next/previous: normalize first; next selects the first bookmark strictly
     greater than the cursor line, wrapping to the first; previous the last
     strictly smaller, wrapping to the last; retain the current byte column
     clamped to the target line; ensure visible; report `status.bookmark.line`
     (`Bookmark: line {}`); empty → `status.bookmark.none` (`Bookmark: none
     set`) (`53fe7f8^:...editing.rs:387-426`). Strict `>`/`<`, not `>=`/`<=`.
7. **Edit hooks** — `crates/dun-cli/src/app/editing.rs` dispatch arms plus the
   `delete_current_line` / `move_current_line` bodies: a successful Delete Line
   normalizes bookmarks; a successful Move Line swaps the source and
   destination lines' bookmark membership then normalizes
   (`53fe7f8^:...editing.rs:249-290`). No other edit remaps bookmarks — this is
   the exact old behavior; do not add general tracking.
8. **Lifecycle** — Reload clones the bookmarks into the replacement
   `BufferState` and normalizes against the new file length
   (`file_io.rs`, beside the `visible_whitespace` line);
   `drop_buffer_if_unreferenced` (`windows.rs:356`) already drops the whole
   `BufferState`, so bookmarks die with the last view for free — a split
   sharing the `BufferId` shares the state; New/Open start empty. Verify, do
   not re-implement, the shared/reap path.
9. **Frame + gutter** — `BufferView::with_bookmarks(&[usize])`
   (`crates/dun-ui/src/model.rs`), threaded from `BufferState`
   (`app/frame.rs`); `crates/dun-ui/src/frame/gutter.rs` sets the marker char
   to `self.glyphs.indicators.bookmark` when
   `buffer.bookmarks.contains(&line_index)` ONLY on the logical line's first
   visual row (`row_offset == 0`); continuation rows stay blank. The marker
   occupies the gutter's last column (the `{:>label_digits$}{marker}` format
   already places it there) and must NOT widen the gutter.

   **Rendering fix (Claude decision — the plan's cell layout was stale).**
   Today's Surface gutter is `decimal_digits + separator_width` wide and
   `draw_gutter` (`crates/dun-ui/src/render/surface_window.rs:204`) paints the
   vertical separator OVER that last column — which for an unmarked row is a
   blank the separator legitimately claims, but it would erase a bookmark
   `*`. Fix with **Option C: on a bookmarked row the marker replaces the
   separator in that edge cell; the gutter width is unchanged.** Concretely:
   add `marked: bool` to `UiGutterLine` (`crates/dun-ui/src/model.rs`), set it
   in `gutter.rs` for the first visual row of a bookmarked line, and in
   `draw_gutter` draw the separator only when `!line.marked` (the label
   already carries the `*` in the last column for marked rows). Do NOT widen
   the gutter and do NOT restore the old separate marker column — that would
   shift the body one column for every user and churn every gutter golden.
   Net visual: unmarked `12│` unchanged; bookmarked `12*` (the `*` sits at the
   gutter edge, interrupting the rule like an IDE breakpoint dot).
10. **Status** — `app/status_view.rs`: insert `[Mark]` (via
    `status.detail.bookmark`) only when the cursor is on a bookmarked line,
    at the old ordering position (`53fe7f8^:...status_view.rs:93-104`).
11. **Command-line** — `app/command_line.rs`: `"mark"` and `"bookmark"` both
    toggle (zero args); completion (`command_line.rs`) advertises `"mark"`
    only (the intentional asymmetry, `53fe7f8^`). Next/previous have no short
    alias — reachable only via their canonical ids.
12. **Menu + help** — `crates/dun-ui/src/frame/menu.rs`: three `VIEW_ENTRIES`
    after Visible Whitespace — `Toggle Bookmark (K)`, `Next Bookmark (N)`,
    `Previous Bookmark (L)`; `help/content.rs`: `Toggle bookmark on current
    line`, `Go to next bookmark`, `Go to previous bookmark`.
13. **i18n** — add the twelve keys with English defaults to their source
    tables and to all ten `i18n/*.conf` in the SAME commit.

Keys and English defaults (exact):

| Key | English |
| --- | --- |
| `menu.view.toggle-bookmark` | `Toggle Bookmark (K)` |
| `menu.view.next-bookmark` | `Next Bookmark (N)` |
| `menu.view.previous-bookmark` | `Previous Bookmark (L)` |
| `help.command.edit.toggle_bookmark` | `Toggle bookmark on current line` |
| `help.command.edit.next_bookmark` | `Go to next bookmark` |
| `help.command.edit.previous_bookmark` | `Go to previous bookmark` |
| `status.bookmark.buffer-missing` | `Bookmark failed: focused buffer is missing` |
| `status.bookmark.added` | `Bookmarked line {}` |
| `status.bookmark.removed` | `Removed bookmark at line {}` |
| `status.bookmark.none` | `Bookmark: none set` |
| `status.bookmark.line` | `Bookmark: line {}` |
| `status.detail.bookmark` | `Mark` |

## Scope

- Files you MAY modify: items 1–13 above and their colocated tests, the
  `tests/markers.rs` modules (CLI + UI, from step 2), `i18n/*.conf`, and the
  affected snapshot goldens. **Item 9 additionally authorizes
  `crates/dun-ui/src/render/surface_window.rs`** (gate the separator paint on
  `!line.marked`) **and the `UiGutterLine` struct in
  `crates/dun-ui/src/model.rs`** (the `marked: bool` field).
- Files/areas you MUST NOT touch: `crates/dun-ui/src/display_map.rs` and the
  seam internals (frozen); the F13 visible-whitespace logic (only add beside
  it); any `Cargo.toml`/`Cargo.lock`, `AGENTS.md`, `CLAUDE.md`, `README.md`,
  `PROGRESS.md`, `TODO.md`, other docs, `.git`, `hosts/**`, `vm-test/**`,
  `reference/**`.

If a change needs a file outside Scope, STOP and report.

## Deliverable

- The full-trail F12 restoration + the twelve-key ten-file i18n batch.
- Tests (independent oracles; restore `53fe7f8^` assertions adapted to
  today's layout, in the `markers.rs` modules):
  - toggle add/remove/sort/dedup; a second buffer is unaffected (no leak).
  - next/previous strict + circular, retain column, clamp on a shorter target
    line; empty-set status.
  - reload into a SHORTER file clamps + dedups the surviving bookmarks.
  - Delete Line normalizes; Move Line swaps the source/destination markers.
  - shared-`BufferId` split retention; final-close/unreferenced clearing.
  - gutter `*` for one-digit and 10+-digit line numbers, Wide mode,
    soft-wrap continuation (marker only on the first visual row, including a
    viewport that begins inside a continuation), and narrow-gutter omission.
  - **a surface-render test** (not only the frame model) proving the `*`
    reaches the drawn Surface and is NOT overwritten by the separator, and
    that an unmarked row still shows the separator — this is the exact
    layering defect that blocked the first attempt, so it must be pinned at
    the render layer.
  - `[Mark]` only on a bookmarked cursor line; combined status ordering with
    Whitespace/Wrap present.
  - `mark`+`bookmark` both toggle; completion advertises `mark` only;
    Ctrl+X,K/N/L dispatch; command-id round-trip.
  - a translated-status call-site test in at least one shipped language.
  - update the help/menu goldens; add a bookmark frame snapshot; inspect
    every changed golden.
- Prove load-bearing (run yourself, then reverse the edit — never
  `git checkout`): (a) change navigation `>`/`<` to `>=`/`<=` OR drop the
  wrap fallback → the strict-circular test fails; (b) omit the Move Line
  bookmark swap → the line-move remap test fails; (c) draw the separator
  unconditionally in `draw_gutter` (ignore `line.marked`) → the
  surface-render test fails because the `*` is overwritten.

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`).
2. The 1 MiB dual-platform budget is real; minimal diff, no new deps. Claude
   runs the dual-platform size gate and, since this closes the track, the
   binding Debian measurement + release smoke.
3. Bookmarks are view state: never dirty the buffer, never serialize.
4. All gutter/status text still routes through the sanitizer; the `*` marker
   is a `GlyphSet` glyph, not a raw literal downstream.
5. i18n completeness tests fail if any of the twelve keys is missing from any
   of the ten files or a source table; add them together.
6. Navigation edge cases are the correctness core: cursor exactly on a
   bookmark, single bookmark, cursor past the last bookmark, an empty set, a
   target line shorter than the retained column. Cover each.
7. Stop-loss: same failure twice, or an out-of-scope file needed → STOP,
   report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Note the pty_smoke and tmux suites' results explicitly. Claude runs the
macOS budget build, the binding Debian measurement, release smoke, and the
four-platform functional matrix at the gate.

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
