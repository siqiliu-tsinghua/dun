# Brief 017 — Implement `window.only`; give collapse/expand a way in

Implementation brief. The contract tests from brief-015 found three window
commands that exist on the command surface but that no user can reach without
typing their id into the `Ctrl+P` prompt:

- `window.only` — a **stub**. It only reports "Only window is not implemented
  yet", while still owning a command id, appearing in the config keymap surface,
  and being bindable. Implement it.
- `window.collapse` / `window.expand` — the behaviour already exists in
  `dun-core` (`Workspace::collapse_focused` / `expand_focused`); only the
  one-way commands have no menu entry and no key. Wire them up.

## Goal

1. `window.only` closes every window except the focused one, without ever losing
   unsaved work.
2. `window.collapse`, `window.expand` and `window.only` each get a menu entry and
   a default keybinding.
3. All three come out of `PROMPT_ONLY_COMMANDS` in
   `crates/dun-cli/src/tests/contracts.rs`. The reachability contract asserts the
   unreachable set *equals* that allowlist in both directions, so leaving them in
   will fail the test — that is the tripwire working, not a problem to route
   around.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-core/src/workspace/model.rs` — `Workspace { pub root: LayoutNode,
  pub focused: WindowId, pub windows: Vec<WindowState>, … }`. `LayoutNode` is
  `Leaf(WindowId)` or `Split { … }`. `WindowState` has `collapsed: bool` and
  `buffer_id: BufferId`.
- `crates/dun-core/src/workspace/split.rs` — `split_focused`, `close_focused`
  (returns `Err(WorkspaceError::CannotCloseLastWindow)` on the last window).
- `crates/dun-core/src/workspace/mod.rs` — `collapse_focused`, `expand_focused`,
  `toggle_focused_collapse`, `window_count`.
- `crates/dun-cli/src/app/windows.rs` — `handle_window_command`, the
  `WindowCommand::Only` stub at the `set_status("Only window is not implemented
  yet")` line, `close_focused_window_unchecked`, `close_focused_file`,
  `drop_buffer_if_unreferenced`, `focus_window_for_buffer`.
- `crates/dun-cli/src/dialogs/confirm.rs` — `PendingAction`.
- `crates/dun-cli/src/app/file_dialogs.rs` — `confirm_focused_dirty`,
  `confirm_any_dirty`, `handle_confirm_key_event`, `save_confirmed_action`,
  `discard_confirmed_action`, `continue_pending_action`.
- `crates/dun-cli/src/app/prompt_dialogs.rs` — the confirm prompt text has two
  `match` arms over `PendingAction` that must learn the new variant.
- `crates/dun-ui/src/frame/menu.rs` — the View menu.
- `crates/dun-config/src/keys/keymap.rs` — `default_editor()`.
- `crates/dun-cli/src/help/content.rs` — the in-editor help listing. It already
  lists the other window commands; it does NOT list these three.

## Specification

### 1. `dun-core`: `Workspace::only_focused`

Add to `crates/dun-core/src/workspace/split.rs`:

```rust
/// Make the focused window the only window. Returns the windows that were
/// removed, so the caller can drop their buffers.
pub fn only_focused(&mut self) -> Result<Vec<WindowState>, WorkspaceError> { … }
```

- If the focused id is not in `windows`, return `WorkspaceError::FocusMissing`.
- If there is only one window, return `Ok(Vec::new())` — a no-op, not an error.
- Otherwise: set `root = LayoutNode::Leaf(focused)`, drain every window whose id
  is not `focused` out of `self.windows` and return them, and clear `collapsed`
  on the survivor (a sole window cannot meaningfully be collapsed).
- Do not touch `next_window_id` / `next_buffer_id`.
- Colocated tests in dun-core: window count drops to 1, the survivor is the
  formerly focused window, the returned list is the others, the layout is a bare
  `Leaf`, a collapsed survivor is expanded, and a single-window workspace is
  unchanged.

### 2. `dun-cli`: the command, with the dirty-data traps

`WindowCommand::Only => self.only_focused_window()`.

**Trap 1 — confirm only for work that would actually be lost.** Closing the
other windows drops their buffers *if nothing else references them*. A buffer
shown in both the focused window and another one survives, so it must not
prompt. Only a dirty buffer that is referenced **solely** by windows being
closed is at risk.

**Trap 2 — the confirm dialog steals the focus.** `confirm_any_dirty` calls
`focus_window_for_buffer`, which moves `workspace.focused` to the dirty buffer's
window. `window.only` keeps *the focused window*, so after a confirm round-trip
the focus is on the wrong window and `only` would keep the wrong one. The target
must therefore be remembered across the confirm: give the pending action a
payload.

```rust
// crates/dun-cli/src/dialogs/confirm.rs
pub(crate) enum PendingAction {
    …
    /// `window.only` — close every other window, keeping this one. Carries the
    /// target because the confirm dialog refocuses the dirty buffer's window.
    OnlyWindow(WindowId),
}
```

**Trap 3 — an infinite confirm loop.** Discard does NOT clear the dirty flag.
`discard_confirmed_action` special-cases `Quit` for exactly this reason:

```rust
match confirm.action {
    PendingAction::Quit => self.should_quit = true,   // bypasses the re-check
    action => { …; self.continue_pending_action(action); }
}
```

If `OnlyWindow` merely re-entered the checked path on discard, it would find the
same still-dirty buffer and prompt forever. Mirror `Quit` exactly:

- **Save** → save that buffer, then re-enter the **checked** path, which finds
  the next at-risk dirty buffer or finishes. (`continue_pending_action`.)
- **Discard** → restore the focus to the target and run the **unchecked** path
  immediately, discarding the rest. (A new arm in `discard_confirmed_action`,
  alongside `Quit`.)
- **Cancel** → nothing happens; the existing `cancel_confirm` already covers it.

So:

```rust
pub(crate) fn only_focused_window(&mut self) {
    if self.workspace.window_count() <= 1 {
        self.set_status("Already the only window");
        return;
    }
    let target = self.workspace.focused;
    if self.confirm_dirty_buffer_losing_its_last_window(target) {
        return; // a confirm is open; it will call back in
    }
    self.only_focused_window_unchecked(target);
}
```

`confirm_dirty_buffer_losing_its_last_window(target)` finds the first dirty
buffer that is referenced by no window other than the ones about to close (i.e.
not by the target window), focuses it, starts
`PendingAction::OnlyWindow(target)`, and returns true. Otherwise false.

`only_focused_window_unchecked(target)` sets `workspace.focused = target`, calls
`workspace.only_focused()`, drops each removed window's buffer with
`drop_buffer_if_unreferenced`, and sets a status naming how many windows closed
(e.g. `Closed 2 other window(s)`).

`continue_pending_action`: `PendingAction::OnlyWindow(target) => {
self.workspace.focused = target; self.only_focused_window(); }` — the checked
path, so Save loops on to the next at-risk buffer.

`discard_confirmed_action`: add an arm next to `Quit` that restores the focus to
`target` and calls `only_focused_window_unchecked(target)`.

Both `match` arms in `prompt_dialogs.rs` need the new variant (the
`Save(s) Discard(d) Cancel(c)` prompt text applies unchanged).

### 3. Menu entries and keybindings

Menu (`crates/dun-ui/src/frame/menu.rs`), in the **View** menu. Mnemonics must
stay unique within the menu — `H V E C Z [ ] X W S D R` are taken; `O`, `M` and
`P` are free. Place them with the other window entries:

```
Split Horizontal (H)
Split Vertical (V)
Equalize (E)
Only Window (O)        <- window.only        NEW
Toggle Collapse (C)
Collapse (M)           <- window.collapse    NEW
Expand (P)             <- window.expand      NEW
Word Wrap (Z)
… (rest unchanged)
```

Keybindings (`default_editor()`). The `Ctrl+X` leader is Emacs-flavoured, and
Emacs binds `C-x 1` to *delete-other-windows* — take the convention. Free
letters after `Ctrl+X` are `A D F G I K L M N P W`. All four candidates below
were verified to parse:

```
Ctrl+X,1  -> window.only        (Emacs C-x 1)
Ctrl+X,M  -> window.collapse    (Minimize)
Ctrl+X,P  -> window.expand      (exPand; matches the menu mnemonic)
```

Do NOT try to bind `+` or `,` — the key-sequence grammar splits on `+` for
modifiers and on `,` for chords, so neither can be a key.

Add the three to `crates/dun-cli/src/help/content.rs` alongside the other window
commands.

### 4. Update the contract allowlist

Remove `window.only`, `window.collapse` and `window.expand` from
`PROMPT_ONLY_COMMANDS` in `crates/dun-cli/src/tests/contracts.rs`. The nine
`app.config_diagnostics_*` entries stay. The contract asserts the unreachable set
equals the allowlist exactly, so this must be done or the test fails — and it
must *only* be done because the commands genuinely became reachable.

### 5. Tests

- dun-core: as listed in §1.
- dun-cli, `window.only`:
  - closes the other windows and keeps the focused one;
  - single window → no-op with a status, not an error;
  - a dirty buffer that would be dropped prompts first, and **Cancel** leaves
    every window open and the buffer untouched;
  - **Save** saves it and then completes;
  - **Discard** completes without saving, and does not prompt again (guard
    against the infinite loop — assert the confirm is gone and the window count
    is 1);
  - the focused window is still the original one after a confirm round-trip
    (guard against trap 2);
  - a dirty buffer that is *also* shown in the focused window does NOT prompt
    (guard against trap 1);
  - a collapsed focused window is expanded when it becomes the only one.
- dun-cli: `window.collapse` / `window.expand` reachable from the menu mnemonic
  and from their new keybinding.

## Scope

- Files you MAY modify:
  - `crates/dun-core/src/workspace/split.rs` (+ its colocated tests);
  - `crates/dun-cli/src/app/windows.rs`;
  - `crates/dun-cli/src/dialogs/confirm.rs`;
  - `crates/dun-cli/src/app/file_dialogs.rs`;
  - `crates/dun-cli/src/app/prompt_dialogs.rs` (only the `PendingAction` arms);
  - `crates/dun-ui/src/frame/menu.rs`;
  - `crates/dun-config/src/keys/keymap.rs`;
  - `crates/dun-cli/src/help/content.rs`;
  - `crates/dun-cli/src/tests/**` (including `contracts.rs`).
- Files/areas you MUST NOT touch:
  - `crates/dun-core/src/command.rs` — the commands already exist;
  - `crates/dun-config/src/commands.rs` — the ids already exist;
  - `crates/dun-plugin`, `crates/dun-term`;
  - `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `PROGRESS.md`, `TODO.md`, `docs/**`,
    `README.md` (Claude writes the docs);
  - `.git`, git config, any `Cargo.toml`, `Cargo.lock` (no new dependencies);
  - `vm-test/**`, `reference/**`, `hosts/**`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **The 1 MiB dual-platform size budget is real.** Claude gates size.
3. **Never discard unsaved work without confirming.** This is an A-level
   invariant (AGENTS.md). `window.only` drops buffers; traps 1–3 above are the
   whole reason this brief is long. Read them again.
4. **Do not weaken the contract tests to make them pass.** The three commands
   come off the allowlist because they became reachable — not to silence a
   failure.
5. **Menu mnemonics must stay unique per menu**, and the mnemonic contract test
   derives them from the labels, so a collision fails loudly. Good.
6. **The key grammar cannot express `+` or `,` as a key.**
7. **Tests are layered and colocated.**
8. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
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
3. The finding / verdict — in particular, confirm how each of the three traps is
   handled, and that the discard path cannot loop.
4. Stop-loss / open questions (empty if none).
