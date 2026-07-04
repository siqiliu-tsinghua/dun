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
- [ ] Commit the current workspace/documentation baseline.

## `dun-core`

- [ ] Replace placeholder id types with an allocation strategy.
- [ ] Define first real text buffer representation.
- [ ] Define cursor and selection types.
- [ ] Define edit transaction type.
- [ ] Implement insert/delete/newline.
- [ ] Implement undo/redo.
- [ ] Implement dirty-state tracking.
- [ ] Implement split focused window.
- [ ] Implement close focused window and tree repair.
- [ ] Implement directional focus movement.
- [ ] Implement split ratio resize.
- [ ] Implement collapse/expand.
- [ ] Add unit tests for buffer edits.
- [ ] Add unit tests for split-tree transitions.

## `dun-term`

- [ ] Define full `TerminalProfile`.
- [ ] Detect UTF-8 vs ASCII rendering mode.
- [ ] Detect or configure 256-color vs 16-color vs mono.
- [ ] Define Microsoft Edit-like default palette.
- [ ] Define ASCII border and indicator glyphs.
- [ ] Define 16-color fallback theme.
- [ ] Add tests for glyph fallback selection.

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
- [ ] Implement display sanitizer for C0/C1 controls.
- [ ] Render `ESC`, OSC, BEL, NUL, DEL, CR, and backspace visibly.
- [ ] Add tests for terminal-injection payloads.
- [ ] Cap display work for very long lines.

## Deferred

- [ ] Mouse selection, right-click paste, and split dragging.
- [ ] `rum` configuration evaluation.
- [ ] `dun-plugin-api`.
- [ ] `dun-plugin-rum`.
- [ ] Syntax highlighting plugins backed by `rum`.
- [ ] Log viewing and filtering after `rum` is ready to embed.
- [ ] Memory watchdog design for long-running plugin evaluation.
- [ ] Broad terminal compatibility test harness.
