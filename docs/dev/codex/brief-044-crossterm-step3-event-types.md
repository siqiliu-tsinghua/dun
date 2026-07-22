# Brief 044 — crossterm replacement step 3: owned event types + import migration

Implementation brief. **Step 3 ("Brief 3") of the accepted plan for brief 041.**
Steps 1–2 landed (`cf1a5b6`, `919a98f`): output escapes and the rustix sys shim
are in-house; crossterm still provides `event::poll`/`event::read` and the
event types. This step makes dun's OWN event types the only vocabulary the
application speaks, with a temporary adapter at the event-loop edge converting
crossterm events into them. **crossterm remains the live input parser** — the
parser/loop cutover is step 4; manifests stay untouched until step 5.

## Goal

After this step, `crossterm` names appear ONLY at the temporary adapter +
`event::poll`/`event::read` edge (inside `terminal/`), nowhere else in
dun-cli. Application dispatch behavior is unchanged — this is a mechanical
type migration, not a behavior change.

## Exact change

1. **New `crates/dun-cli/src/terminal/vt/event.rs`** — the owned,
   crossterm-shaped subset (from the plan, verified against real usage):

   - `KeyCode`: `Backspace Enter Left Right Up Down Home End PageUp PageDown
     Tab BackTab Delete Insert F(u8) Char(char) Esc Null`
   - `KeyModifiers`: `NONE SHIFT CONTROL ALT` with `contains`, `BitOr`,
     `BitOrAssign` (no Super/Hyper/Meta, no KeyEventState)
   - `KeyEventKind`: `Press Repeat Release`
   - `KeyEvent { code, modifiers, kind }` with `new()` (Press) and
     `new_with_kind()`; equality is STRUCTURAL (plain derive — do not
     reproduce crossterm's normalizing PartialEq)
   - `MouseButton`: `Left Middle Right`
   - `MouseEventKind`: `Down(MouseButton) Up(MouseButton) Drag(MouseButton)
     Moved ScrollUp ScrollDown ScrollLeft ScrollRight`
   - `MouseEvent { kind, column, row, modifiers }`
   - `Event`: `Key(KeyEvent) Mouse(MouseEvent) Paste(String)
     Resize(u16, u16)`

   Mirror the shapes the app actually consumes (constructors, public fields,
   `contains`, modifier bit-or) so consuming code changes are import swaps.
2. **Temporary adapter** (inside `terminal/`, e.g. `event_loop.rs` or a small
   private module): convert `crossterm::event::Event` → owned `Event`.
   - Kinds map faithfully Press→Press / Repeat→Repeat / Release→Release —
     the app's existing Release-drop logic must keep working through it.
   - Modifiers: keep SHIFT/CONTROL/ALT bits, drop everything else.
   - Key codes outside the owned set (kitty/media/etc.) → `KeyCode::Null`
     (the app already ignores Null; keep that path).
   - Mouse kinds map 1:1; focus/other crossterm events map to nothing
     (adapter returns None — the loop already ignores unmatched events).
3. **`main.rs`** — the prelude aliases re-export the owned types from
   `terminal::vt::event`; no `Crossterm*`-named aliases survive.
4. **`terminal/input.rs`** — rename `key_stroke_from_crossterm` /
   `text_input_from_crossterm` to neutral names (`key_stroke_from_event` /
   `text_input_from_event` or similar); they now take the owned types.
5. **Mechanical migration of every consumer** — the plan inventoried 23
   files (6 production: `app/buffer_switcher.rs`, `app/file_dialogs.rs`,
   `app/prompt_dialogs.rs`, `app/search_replace.rs`, `terminal/input.rs`,
   `main.rs`; 17 under `crates/dun-cli/src/tests/`). Re-verify the inventory
   with a grep and migrate every hit — imports and type paths only, no logic
   edits.

## Scope

- Files you MAY modify: new `terminal/vt/event.rs`;
  `terminal/{mod,event_loop,input}.rs`; `main.rs`; files under
  `crates/dun-cli/src/app/` and `crates/dun-cli/src/tests/` STRICTLY for the
  mechanical alias/type migration; colocated unit tests for the new types +
  adapter.
- Files/areas you MUST NOT touch: `terminal/vt/output.rs`, `terminal/sys/**`,
  `terminal/ambiguous_width.rs`, `terminal/lifecycle.rs`, any
  `Cargo.toml`/`Cargo.lock`, `crates/dun-cli/tests/**` (integration tests
  speak VT bytes, not types — they must pass unchanged), other crates, docs,
  `.git`, `i18n/**`, `hosts/**`, `vm-test/**`, `reference/**`.

If a change needs a file outside Scope, STOP and report.

## Deliverable

- The owned types + adapter + full import migration.
- Unit tests: owned-type constructors and modifier bit ops; **adapter parity
  tests with independent oracles** — literal crossterm events in, literal
  owned events out (Press/Repeat/Release carried, SHIFT|CONTROL|ALT kept,
  Super dropped, unknown key → Null, each mouse kind, Paste, Resize, focus →
  None).
- A grep gate in the report: `grep -rn crossterm crates/dun-cli/src` output
  showing hits ONLY in the adapter/event-loop edge (and none in `app/`,
  `tests/`, `input.rs`).
- Prove load-bearing (run, paste, restore): (a) adapter drops SHIFT → a
  parity test fails; (b) adapter maps Release→Press → a test fails (add an
  input-layer test that Release key events are filtered, if none exists);
  (c) adapter maps unknown keys to `Char(' ')` instead of Null → a test
  fails.

## dun pitfalls (read twice)

1. Safe Rust only.
2. This step must be behaviorally invisible: every existing test passes
   unchanged except renamed helpers; golden frames untouched; PTY/tmux
   suites untouched and green.
3. crossterm's `KeyEvent` equality is non-structural (it normalizes); if any
   test relied on that, surface it in the report rather than silently
   changing expectations.
4. `KeyCode::Null` stays — `tests/terminal_io.rs:170` proves unsupported
   codes are ignored; keep that invariant reachable.
5. Stop-loss: same failure twice, or an out-of-scope file needed → STOP,
   report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Plus the grep gate above. Claude runs the size gate at the gate (expected
~neutral: types are small and crossterm is still linked).

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working
  tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format

1. What changed — per file (group the mechanical migrations), one-line why.
2. Verification — verbatim outputs + the grep gate.
3. Mutation evidence — the three load-bearing runs, verbatim.
4. Verdict.
5. Stop-loss / open questions (empty if none).
