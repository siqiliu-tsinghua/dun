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
- [x] Render grouped File/Edit/View/Help dropdown menus.
- [x] Add keyboard navigation for grouped dropdown menus.
- [x] Render status bar.
- [x] Render single editor area with line-number gutter.
- [x] Render Microsoft Edit-style single-line borders for tiled windows.
- [x] Render ASCII fallback borders.
- [x] Render multiple tiled windows from resolved layout rectangles.
- [x] Render focused buffer cursor inside the active window body.
- [x] Render selected text ranges in the active window body.
- [x] Add UI hit testing for optional mouse focus and cursor placement.
- [x] Add submenu hit testing for optional mouse command dispatch.
- [x] Polish tiled rendering for small terminals and narrow panes.
- [x] Keep rendering free of file I/O.
- [x] Tune `msedit` theme colors against local Microsoft Edit screenshots.
- [x] Add Microsoft Edit-like active top-menu color and gray dropdown panel.
- [x] Add menu mnemonics and right-aligned shortcut column rendering.
- [x] Add current-line row highlight and persistent gutter separator.
- [x] Add lightweight modal prompt rendering for Go To Line, Find, Replace,
  and confirmations.
- [x] Add larger Open/Save As file dialog baseline after lightweight modals.
- [x] Add Tab path completion to the Open/Save As file dialog baseline.
- [x] Add mouse hit testing for file dialog entries.
- [x] Add file dialog list scrolling and PageUp/PageDown navigation.
- [x] Add parent-directory and hidden-file polish to file dialogs.
- [x] Add file dialog path-input cursor movement and Home/End editing.
- [x] Add file dialog empty/no-match diagnostics and tighter Open/Save visuals.

## `dun-config`

- [x] Define typed config defaults.
- [x] Define typed keybinding schema.
- [x] Define default keymap.
- [x] Load config files through Rust-owned parsing.
- [x] Apply configured command keybindings in the runtime input path.
- [x] Reload runtime configuration without restarting the editor.
- [x] Show active config diagnostics inside the editor.
- [x] Support multi-stroke key sequence prefix matching.
- [x] Define theme selection config.
- [x] Define terminal override config.
- [x] Define optional mouse enablement config.
- [x] Define configurable file-dialog modal keybindings.
- [x] Define large-file and display limits.
- [x] Add config validation tests.
- [x] Add MacBook-friendly `Ctrl+W` window focus/resize aliases while keeping
  `Alt` compatibility bindings where terminals deliver them.

## `dun-cli`

- [x] Add argument parsing.
- [x] Add terminal setup and restoration guard.
- [x] Create initial untitled workspace.
- [x] Open file path passed on command line.
- [x] Save focused buffer back to its loaded file path.
- [x] Wire config/profile/workspace/UI construction.
- [x] Apply editor commands to the focused buffer/window.
- [x] Add runtime config reload command.
- [x] Add config diagnostics screen.
- [x] Add command-line prompt baseline.
- [x] Add command-line prompt history.
- [x] Add runtime theme selection command.
- [x] Add interactive file dialogs for open/save-as and modal prompts for
  find/replace/go-to-line entry.
- [x] Route printable text input into the focused buffer.
- [x] Track pending multi-stroke key sequences.
- [x] Keep the focused cursor line visible while drawing.
- [x] Implement find result navigation.
- [x] Implement replace current/next match baseline.
- [x] Implement go-to-line prompt.
- [x] Implement read-only help/key reference window.
- [x] Generate help/key reference content from the active keymap.
- [x] Implement status history window.
- [x] Expand status bar detail fields.
- [x] Show focused buffer name, dirty state, and line/column status.
- [x] Confirm before quit/new/open/close would discard dirty buffers.
- [x] Return stable exit codes.
- [x] Report visible success/failure status for tiling window commands.
- [x] Run full command ids from the command-line prompt, including
  `window.*` commands.
- [x] Add optional mouse capture, left-click window focus, and body cursor
  placement.
- [x] Add mouse text selection, menu clicks, and split dragging.
- [x] Document right-click paste and clipboard safety policy.

## File and Display Safety

- [x] Implement UTF-8-first file loading behavior.
- [x] Open invalid UTF-8 files as read-only escaped fallback buffers.
- [x] Define invalid-byte fallback behavior.
- [x] Track file-text encoding and expose escaped-byte fallback state.
- [x] Reject unstable/corrupt reads when file metadata changes during Open.
- [x] Prevent save from silently corrupting lossy/fallback buffers.
- [x] Save files through same-directory temporary files and atomic rename.
- [x] Add readable path diagnostics for common open/save failures.
- [x] Define large-file soft limit behavior.
- [x] Add large-file performance baselines.
- [x] Implement display sanitizer for C0/C1 controls.
- [x] Render `ESC`, OSC, BEL, NUL, DEL, CR, and backspace visibly.
- [x] Add tests for terminal-injection payloads.
- [x] Add control-byte rendering audit suite for buffer text and UI chrome.
- [x] Cap display work for very long lines.

## Terminal Compatibility Testing

- [x] Add PTY smoke tests for common SSH-style terminal profiles.
- [x] Expand PTY tests into a broad terminal compatibility harness.
- [x] Document manual SSH terminal checks and current-environment verification.
- [x] Define the external SSH and low-capability terminal release matrix.
- [ ] Run the external SSH and low-capability terminal release matrix before
  a tagged release.
- [x] Add static Microsoft Edit reference tests for source-visible menu,
  status, color, and terminal setup markers.

## Deferred

- [ ] Right-click paste and clipboard/bracketed-paste implementation.
- [x] Crash recovery and orphaned atomic-save temp-file cleanup.
- [ ] `rum` configuration evaluation.
- [ ] `dun-plugin-api`.
- [ ] `dun-plugin-rum`.
- [ ] Syntax highlighting plugins backed by `rum`.
- [ ] Log viewing and filtering after `rum` is ready to embed.
- [ ] Memory watchdog design for long-running plugin evaluation.
- [x] Broad terminal compatibility test harness.
