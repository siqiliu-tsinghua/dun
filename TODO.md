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

- [ ] Select `ratatui` and backend versions compatible with Rust `1.85`.
- [ ] Render menu bar.
- [ ] Render status bar.
- [ ] Render single editor area with line-number gutter.
- [ ] Render Microsoft Edit-style single-line borders for tiled windows.
- [ ] Render ASCII fallback borders.
- [ ] Render multiple tiled windows from resolved layout rectangles.
- [ ] Keep rendering free of file I/O.

## `dun-config`

- [ ] Define typed keybinding schema.
- [ ] Define default keymap.
- [ ] Define theme selection config.
- [ ] Define terminal override config.
- [ ] Define large-file and display limits.
- [ ] Add config validation tests.

## `dun-cli`

- [ ] Add argument parsing.
- [ ] Add terminal setup and restoration guard.
- [ ] Create initial untitled workspace.
- [ ] Open file path passed on command line.
- [ ] Wire config/profile/workspace/UI construction.
- [ ] Return stable exit codes.

## File and Display Safety

- [ ] Define UTF-8-first file loading behavior.
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
