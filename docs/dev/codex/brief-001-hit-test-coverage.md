# brief-001 — densify dun-ui hit-testing unit tests

## Goal

`crates/dun-ui/src/tests/hit.rs` currently covers menu columns, plain body
clicks, chrome/gutter, horizontal scroll, one scrollbar case, and scrolled
dropdowns. The upcoming renderer work will churn exactly this geometry, so
the net must be denser. When you are done, the named gaps below each have a
focused test, the whole workspace test suite is green, and no runtime code
changed.

## Context pointers

- Read `AGENTS.md` first.
- Key files:
  - `crates/dun-ui/src/hit.rs` — the hit-testing implementation under test.
  - `crates/dun-ui/src/tests/hit.rs` — the file you extend; match its style
    (helpers, `UiShell::default()`, `Rect::new`, plain asserts).
  - `crates/dun-ui/src/tests/support.rs` (or the existing `use` lines in
    tests/hit.rs) — shared test imports.
  - `crates/dun-ui/src/model.rs` — `BufferView` builders (`scrolled_xy`,
    `with_first_visual_row`, `with_wrap`, `with_search`).
- Acceptance is mechanical: the tests you add pass, everything else stays
  green.

## Scope

- Files you MAY modify: `crates/dun-ui/src/tests/hit.rs` ONLY.
- Everything else is out of scope, including `crates/dun-ui/src/hit.rs`
  itself: if a new test exposes what you believe is a real defect in hit
  logic, do NOT fix it — mark the test `#[ignore = "records suspected
  defect: <one line>"]`, keep it in the diff, and report it prominently.
- Plus the standard MUST-NOT list from `docs/dev/codex/TEMPLATE.md`.

## Deliverable

New focused tests in `crates/dun-ui/src/tests/hit.rs` covering, at minimum:

1. `hit_test_overlay_list` boundaries: click on the first content column and
   one column left of it (outside), the last content column and one right of
   it, a row above `list_start_row`, the last list row, and one row past the
   list (None).
2. Wrapped-body hits (`with_wrap(true)`): a click on the second visual row
   of a wrapped line maps to the correct logical `Position`; the same with
   `with_first_visual_row(1)` applied; a wrapped line containing a wide
   character maps to a valid UTF-8 boundary.
3. Scrollbar extremes: clicks on the first and last scrollbar rows of a long
   buffer produce in-range `Scrollbar { .. }` targets (no panic, no
   out-of-bounds line).
4. Menu mnemonics: `menu_index_for_mnemonic` finds each top-level menu by
   its mnemonic char and returns None for an unused char.
5. Narrow pane without gutter: in a pane narrow enough that the gutter is
   dropped, a body click maps to a buffer position rather than `Gutter`.
6. Collapsed window: a click inside a collapsed window's rect yields a hit
   whose target is not a body text position beyond the collapsed content
   (assert the actual behavior; if it looks wrong, use the ignore-and-report
   rule above).

Aim for one behavior per test, named like the existing tests.

## dun pitfalls (read twice)

See `docs/dev/codex/TEMPLATE.md` §dun pitfalls — items 1, 5, 7 apply
directly. This brief is test-only, so the size budget is not in play.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dun-ui
cargo test --workspace --no-fail-fast
```

Paste the resulting `test result:` lines verbatim. The tmux suite skips
cleanly when tmux is unavailable; say so if it does.

## Hard rules

All of `docs/dev/codex/TEMPLATE.md` §Hard rules apply verbatim: no git
operations, nothing outside Scope, no network, minimal diff, verbatim
evidence only.

## Report format (your final message)

Per `docs/dev/codex/TEMPLATE.md` §Report format.
