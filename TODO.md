# TODO

This file tracks active and near-term work. Completed decisions and finished
items belong in [PROGRESS.md](./PROGRESS.md).

## Active Baseline

- [x] Create the Rust `1.85` workspace structure.
- [x] Add ignored local reference area for studying Microsoft Edit and Turbo
  Vision.
- [x] Create initial crates: `dun-core`, `dun-term`, `dun-ui`, `dun-config`,
  `dun-cli`.
- [x] Add crate boundary documentation.
- [x] Commit the current workspace/documentation baseline.

## `dun-core`

- [x] Replace placeholder id types with an allocation strategy.
- [x] Define first real text buffer representation.
- [x] Define cursor and selection types.
- [x] Define edit transaction type.
- [x] Implement insert/delete/newline.
- [x] Implement undo/redo.
- [x] Implement dirty-state tracking.
- [x] Implement split focused window.
- [x] Implement close focused window and tree repair.
- [x] Implement directional focus movement.
- [x] Implement split ratio resize.
- [x] Implement collapse/expand.
- [x] Add unit tests for buffer edits.
- [x] Add unit tests for split-tree transitions.

## `dun-term`

- [x] Define full `TerminalProfile`.
- [x] Detect UTF-8 vs ASCII rendering mode.
- [x] Detect or configure 256-color vs 16-color vs mono.
- [x] Define Microsoft Edit-like default palette.
- [x] Define ASCII border and indicator glyphs.
- [x] Define 16-color fallback theme.
- [x] Add tests for glyph fallback selection.

## `dun-ui`

- [x] Build backend-neutral frame model from config, workspace, and buffers.
- [x] Resolve theme, glyph, keymap, and display sanitizer in `UiShell`.
- [x] Sanitize buffer lines before they enter the UI frame model.
- [x] Select `ratatui` and backend versions compatible with Rust `1.85`.
- [x] Render menu bar.
- [x] Render status bar.
- [ ] Render single editor area with line-number gutter.
- [x] Render Microsoft Edit-style single-line borders for tiled windows.
- [x] Render ASCII fallback borders.
- [x] Render multiple tiled windows from resolved layout rectangles.
- [x] Render focused buffer cursor inside the active window body.
- [x] Render selected text ranges in the active window body.
- [x] Keep rendering free of file I/O.

## `dun-config`

- [x] Define typed config defaults.
- [x] Define typed keybinding schema.
- [x] Define default keymap.
- [x] Support multi-stroke key sequence prefix matching.
- [x] Define theme selection config.
- [x] Define terminal override config.
- [x] Define large-file and display limits.
- [x] Add config validation tests.

## `dun-cli`

- [ ] Add argument parsing.
- [x] Add terminal setup and restoration guard.
- [x] Create initial untitled workspace.
- [x] Open file path passed on command line.
- [x] Save focused buffer back to its loaded file path.
- [x] Wire config/profile/workspace/UI construction.
- [x] Apply editor commands to the focused buffer/window.
- [x] Add interactive status-line prompts for open, save-as, and find entry.
- [x] Route printable text input into the focused buffer.
- [x] Track pending multi-stroke key sequences.
- [x] Keep the focused cursor line visible while drawing.
- [x] Implement find result navigation.
- [ ] Return stable exit codes.

## File and Display Safety

- [x] Implement UTF-8-first file loading behavior.
- [x] Reject invalid UTF-8 files instead of opening them lossy.
- [ ] Define invalid-byte fallback behavior.
- [ ] Prevent save from silently corrupting lossy/fallback buffers.
- [ ] Define large-file soft limit behavior.
- [x] Implement display sanitizer for C0/C1 controls.
- [x] Render `ESC`, OSC, BEL, NUL, DEL, CR, and backspace visibly.
- [x] Add tests for terminal-injection payloads.
- [x] Cap display work for very long lines.

## Deferred

- [ ] Mouse selection, right-click paste, and split dragging.
- [ ] `rum` configuration evaluation.
- [ ] `dun-plugin-api`.
- [ ] `dun-plugin-rum`.
- [ ] Syntax highlighting plugins backed by `rum`.
- [ ] Log viewing and filtering after `rum` is ready to embed.
- [ ] Memory watchdog design for long-running plugin evaluation.
- [ ] Broad terminal compatibility test harness.
