# PLAN

`dun` should first become a reliable Microsoft Edit-like terminal editor with a
lightweight tiled workspace. After that foundation is stable, it can grow into
an operations-oriented log inspection tool with a future pure-plugin layer.

## Design Principles

1. Core first, plugin runtimes later.
2. Rust owns state and side effects.
3. Rendering is never allowed to emit untrusted control bytes.
4. Terminal compatibility is a first-class feature.
5. The first usable line is a text editor; log workflows wait until the editor
   foundation and plugin protocol boundary are ready.
6. Architecture should allow `rum` later without depending on an unstable API.
   `dun` targets a host-neutral plugin protocol first; `rum` is a future
   official pure host, not the protocol itself.
7. The UI supports multiple simultaneous views through a lightweight tiling
   split tree, not sidebars, tabs, or floating windows.
8. Keybindings are configurable because terminals and remote sessions vary.
9. The final v0.1 release executable must be at most 1 MiB on both audited
   macOS and Debian builds. The feature budget and trim order in
   [docs/feature-budget.md](./docs/feature-budget.md) govern feature
   admission and removal.

## Workspace Shape

The workspace is intentionally small:

```text
dun-core    terminal-independent editor state and commands
dun-term    terminal profile, glyphs, and themes
dun-ui      ratatui-facing rendering shell
dun-config  typed configuration and keymap model
dun-cli     process entry point
```

See [docs/crate-map.md](./docs/crate-map.md) for crate boundaries.

## Phase 0: Baseline

Status: complete.

- [x] Establish project documents.
- [x] Record plugin security boundary.
- [x] Record terminal compatibility requirements.
- [x] Record first-version editor baseline decisions.
- [x] Initialize git and Cargo.
- [x] Convert the package into a lightweight workspace.
- [x] Add minimal crate skeletons.
- [x] Commit the workspace/documentation baseline.

## Phase 1: Core Types and Pure State

Goal: enough pure data model to test editor and tiling behavior without a
terminal.

- [x] Define stable id types: `BufferId`, `WindowId`.
- [x] Define buffer metadata and editable/read-only state.
- [x] Define `EditorCommand` families.
- [x] Define `Workspace`, `LayoutNode`, and `WindowState`.
- [x] Implement split focused window.
- [x] Implement close focused window and layout tree repair.
- [x] Implement directional focus movement.
- [x] Implement split resize by ratio.
- [x] Implement collapse/expand state.
- [x] Add unit tests for each layout transition.
- [x] Choose first text buffer representation.
- [x] Define cursor, selection, edit transactions, and dirty-state tracking.

Primary crate: `dun-core`.

## Phase 2: Text Buffer Baseline

Goal: a small but correct editable UTF-8 buffer.

- [x] Choose first buffer representation.
- [x] Implement cursor movement.
- [x] Implement insert/delete/newline.
- [x] Implement selection model.
- [x] Implement edit transactions.
- [x] Implement undo/redo.
- [x] Track dirty state.
- [x] Preserve newline style where possible.
- [x] Add buffer edit tests.

Primary crate: `dun-core`.

## Phase 3: File Loading and Display Safety

Goal: safe file round trips and safe terminal display.

- [x] Implement UTF-8-first file loading.
- [x] Define invalid-byte fallback behavior.
- [x] Add read-only fallback state for unsafe/lossy opens.
- [x] Implement save/save-as through host-owned file I/O.
- [x] Save through same-directory temp files and atomic rename.
- [x] Detect recovery candidates and clean stale atomic-save temp files.
- [x] Reject file opens when metadata changes during read.
- [x] Track file-text encoding as UTF-8 or escaped unknown bytes.
- [x] Define large-file soft limit behavior.
- [x] Add visible diagnostics for fallback/large-file state.
- [x] Add readable path diagnostics for common Open/Save failures.
- [x] Implement display sanitizer for ASCII controls and terminal escapes.
- [x] Add tests for `ESC`, OSC, BEL, NUL, DEL, CR, backspace, tabs, and long
  lines.
- [x] Add a control-byte rendering audit suite for buffer text and UI chrome.

Primary crates: `dun-core`, `dun-cli`, later `dun-ui`.

## Phase 4: Terminal Profile, Themes, and Glyphs

Goal: render consistently across common SSH terminals.

- [x] Detect or configure UTF-8 vs ASCII rendering.
- [x] Detect or configure 256-color, 16-color, and mono color modes.
- [x] Define Microsoft Edit-like default theme.
- [x] Define ASCII and 16-color fallback styles.
- [x] Keep Turbo Vision/dark/Dun themes as optional later additions.
- [x] Add glyph-set tests for Unicode and ASCII borders.

Primary crate: `dun-term`.

## Phase 5: Minimal TUI Shell

Goal: a running editor frame that restores the terminal correctly.

- [x] Build a backend-neutral UI frame model from config, workspace, and buffer
  snapshots.
- [x] Resolve theme, glyph, keymap, and display sanitizer settings in `dun-ui`.
- [x] Add `ratatui` and terminal backend dependencies compatible with Rust
  `1.85`.
- [x] Enter raw/alternate-screen mode and always restore terminal state.
- [x] Render menu bar, editor area, and status bar.
- [x] Render line number gutter.
- [x] Render Microsoft Edit-style single-line borders for tiled child windows.
- [x] Translate keyboard input, including multi-stroke sequences, into
  `EditorCommand`.
- [x] Support quit.
- [x] Support cursor movement and text insertion.
- [x] Open one UTF-8 file path from the command line.
- [x] Save the focused buffer back to its loaded file path.
- [x] Support interactive open, save-as, and find entry point.
- [x] Implement find result navigation.
- [x] Render selected text ranges in the editor body.
- [x] Show focused buffer name, dirty state, and line/column status.
- [x] Add optional mouse capture for left-click window focus and body cursor
  placement.
- [x] Add mouse text selection, split dragging, and menu clicks.
- [x] Keep right-click paste deferred until paste policy is explicit.

Primary crates: `dun-cli`, `dun-ui`, `dun-core`, `dun-term`.

## Phase 6: Tiling Workspace UI

Goal: keyboard-first split workflow.

- [x] Render multiple tiled windows.
- [x] Implement split horizontal/vertical commands.
- [x] Implement focus left/right/up/down.
- [x] Implement resize left/right/up/down.
- [x] Implement close focused window.
- [x] Implement collapse/expand focused window.
- [x] Implement equalize and rotate split.
- [x] Report success and failure status for tiling commands.
- [x] Allow command-line prompt execution of window command ids.
- [x] Degrade cleanly on small terminals and narrow panes.
- [x] Ensure single-buffer startup still looks like Microsoft Edit.

Primary crates: `dun-core`, `dun-ui`.

## Phase 7: Config and Keybindings

Goal: make the app usable across inconsistent terminals.

- [x] Define typed config defaults.
- [x] Define keybinding schema.
- [x] Load a config file through Rust-owned parsing first.
- [x] Support terminal profile overrides.
- [x] Support theme selection.
- [x] Apply configured command keybindings at runtime.
- [x] Reload runtime configuration without restarting the editor.
- [x] Show active config diagnostics inside the editor.
- [x] Validate duplicate or invalid keybindings.
- [x] Keep future `rum` config evaluation as a producer of the same typed
  config.

Primary crate: `dun-config`.

## Phase 8: Editor Polish

Goal: reach a practical Microsoft Edit-like baseline.

- [x] Search and replace baseline.
- [x] Go to line.
- [x] Open/Save As file dialog baseline with keyboard selection and Tab path
  completion.
- [x] Command-line prompt baseline.
- [x] Command-line prompt history.
- [x] Runtime theme selection command.
- [x] Unsaved changes confirmation.
- [x] Error log/status history baseline.
- [x] Help/key reference screen.
- [x] Better status bar fields.
- [x] Group menu commands into File/Edit/View/Help dropdowns.
- [x] Make grouped menus usable from the keyboard without requiring mouse
  mode.
- [x] Align the default `msedit` visual chrome with local Microsoft Edit
  screenshots.
- [x] Add buffer switcher, focused-file reload, and external modification
  save protection.
- [x] Add practical line commands, bookmarks, visible-whitespace markers, and
  display-layer soft wrap.
- [x] Add shell escape and one-shot command output without embedding a terminal
  emulator.
- [x] Add read-only outline/search-result panes and command prompt completion
  for common command families.
- [x] Polish Command Output for large results with section navigation and
  stdout/stderr-only derived views.
- [x] Add helper-pane row selection/jump behavior and command prompt
  candidate/path completion polish.
- [x] Static Microsoft Edit reference baseline tests.
- [x] Automated PTY smoke tests for common SSH-style terminal profiles.
- [x] Manual terminal checklist and current-environment checks.

## Phase 9: Plugin Protocol Client

Status: planned next required runtime stage.

Starts after the editor baseline and release hardening are stable. This phase
does not wait for `rum`. It builds the host-neutral Dun Plugin Protocol client
inside `dun`; `rum` integration remains a later optional host that speaks the
same protocol.

- [ ] Define the framed stdio protocol and JSON message schema.
- [ ] Define `PluginRole`, `PluginPolicy`, trust classes, and manifest fields.
- [ ] Define plugin input snapshots and output intents for the first roles.
- [ ] Add external host process lifecycle with direct executable launch,
  bounded frames, timeouts, cancellation, crash handling, and diagnostics.
- [ ] Add fixture host tests for handshake, request/response, malformed
  output, oversized output, timeout, cancellation, crash, and stale revision.
- [ ] Implement at least one visible low-risk role end to end, preferably
  `SyntaxHighlight`, with result validation before UI application.
- [ ] Keep the default `dun` executable within the 1 MiB macOS/Debian budget;
  trim optional editor features before cutting the protocol client.
- [ ] Keep `dun-rum-host` deferred until `rum` has a stable release-facing host
  API and can provide the pure-sandbox security claim.
- [ ] Add log/filter workflows only after the protocol boundary is working.

## Phase 10: Hardening

- [x] Crash recovery paths.
- [x] Corrupt file handling.
- [x] Non-UTF-8 file strategy.
- [x] Define external SSH and low-capability terminal test matrix.
- [x] Add broad local PTY terminal compatibility harness.
- [x] Add automated modified-key event coverage for the terminal matrix.
- [x] Add release size, runtime resource, and dependency/feature lightweight
  audits for macOS and Debian baselines.
- [x] Document safe Rust and code organization guidelines.
- [x] Document the staged oversized-file splitting plan.
- [x] Split `dun-cli` unit tests into behavior-family modules.
- [x] Extract `dun-cli` pure model/state types into app, dialog,
  command-output, and terminal modules.
- [x] Extract `dun-cli` pure helper functions into help/status/text and
  file-dialog/text helper modules.
- [x] Start `dun-cli` AppState method-group extraction with window, editing,
  and view-state modules.
- [x] Start `dun-cli` process I/O boundary extraction with terminal lifecycle
  and SGR output modules.
- [x] Run external SSH and low-capability terminal matrix before release.
- [x] Large-file performance baselines.
- [x] Lightweight release binary size audit on macOS and Debian.
- [x] Security audit suite for control-byte rendering.
- [ ] Security audit suite for plugin policy after plugin APIs exist.

## Phase 11: v0.1 Release Hardening

Goal: freeze the baseline editor feature set and verify it fits the runtime
budget before broadening scope.

- [x] Define a 1 MiB macOS/Debian release executable budget.
- [x] Classify implemented runtime features as required or optional.
- [x] Define the optional runtime feature trim order.
- [x] Record current macOS release size against the checked-in release
  profile.
- [x] Record current Debian release size against the checked-in release
  profile.
- [x] Run the release smoke checklist.

Phase 11 is closed. New runtime work starts only through Phase 9's required
plugin protocol client or through explicit release blockers under
[docs/feature-budget.md](./docs/feature-budget.md).

## Phase 12: Renderer Replacement (ratatui → Surface)

Goal: remove the `ratatui` dependency by drawing directly onto the in-house
`Surface` cell grid and emitting terminal bytes with the `surface_emit`
diff encoder. Dependency hygiene, not size-critical; the budget margin is
ample. Sliced small so each step is independently verifiable, most through
Codex briefs (`docs/dev/codex/brief-00N-*`).

Architecture decision (2026-07-11, entry-point prototype
`render/surface_frame.rs`): the migration builds a **parallel** Surface
render path rather than blitting a Surface back into a ratatui `Buffer`.
`render_ui_frame_to_surface` mirrors `render_ui_frame` layer for layer and
**returns** the focused cursor position instead of calling a Frame method —
the Surface path has no terminal handle, so the dun-cli cutover appends the
cursor CUP after the `emit_diff` stream. Both paths coexist and are held to
cell-level parity by `tests/surface_parity.rs` until the cutover deletes the
ratatui path.

Parity contract: glyph, foreground, and background must match ratatui
exactly (that is the emitted, user-visible output); text modifiers are
asserted as `surface ⊆ ratatui`. ratatui's `Cell::set_style` patches
modifiers (`insert(add); remove(sub)`) and `to_ratatui_style` never sets
`sub_modifier`, so a modifier from one layer (e.g. a focused window border's
BOLD) bleeds through every plain style painted over it. The Surface path
replaces styles cleanly and carries only the intended modifier; per-layer
`surface_layers` unit tests pin those exactly. The cutover therefore
corrects a latent, effectively invisible modifier bleed rather than
regressing.

- [x] Surface cell grid (brief-002).
- [x] `surface_emit` diff encoder: `emit_full`/`emit_diff` to CUP/SGR bytes
  (brief-005).
- [x] Surface restyle primitives `set_style`/`style_run` for the overlay
  passes.
- [x] Surface chrome primitives `draw_border`/`draw_overflow_indicators`
  (brief-006).
- [x] Surface status/menu layer drawing (brief-007).
- [x] Surface entry-point prototype with the cursor-return contract and the
  parity harness (menu/status/dropdown regions green).
- [ ] Port the window layer (gutter, body text, current line, selection,
  search, plugin highlights, scrollbar, horizontal edges) and the overlay
  layer to Surface; extend parity to the full frame across the existing
  rendering fixtures (slice 3c).
- [ ] dun-cli cutover: draw into a `Surface`, emit with `emit_diff`, append
  the returned cursor position, delete the ratatui render path and its
  snapshot tests, drop the `ratatui` dependency. Gate with the tmux/PTY
  suites and a dual-platform size re-audit.
