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
- [x] Coalesce continuous ordinary character input into undo transactions while
  keeping paste, movement, selection, delete, and replace boundaries separate.
- [x] Coalesce continuous same-direction Backspace/Delete runs into undo
  transactions.
- [x] Add UTF-8-safe word movement, word selection, and word delete commands.
- [x] Keep the horizontal cursor position visible in long editor lines.
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
- [x] Add file-dialog overlay structure tests for Microsoft Edit-like visual
  fields.
- [x] Refine selection and search highlight geometry for soft-wrapped lines.
- [x] Add visual-row scrolling for soft-wrapped editor panes.

## UI Polish Backlog

Scope: these are non-`rum`, non-manual polish tasks that should be handled with
automated tests only. Manual screenshot comparison and external terminal
inspection stay outside this section.

- [x] Add automated text-snapshot coverage for Microsoft Edit-like menu,
  window, status, and modal chrome.
- [x] Keep long dropdown menus usable on short terminals, including visible
  overflow indicators and correct mouse hit testing.
- [x] Add visible overflow indicators for scrollable modal lists such as
  Open/Save As and Switch Buffer.
- [x] Polish command prompt completion display so candidates are visible in
  the prompt overlay, not only status history.
- [x] Strengthen ASCII/16-color fallback rendering tests for menus, dialogs,
  scrollbars, and viewport markers.
- [x] Tighten small-terminal and narrow-pane rendering assertions beyond
  no-panic smoke tests.
- [x] Keep helper-window and modal text layout covered by automated rendering
  assertions for Help, Config Diagnostics, Status History, Outline, Search
  Results, Command Output, and file/dialog overlays.
- [x] Keep mouse hit testing aligned with rendered menu/dialog/scrollbar
  geometry after every UI polish change.

## Code Hygiene

- [x] Document the safe Rust policy and code organization rules.
- [x] Document the staged oversized-file splitting plan.
- [x] Split `crates/dun-cli/src/main.rs` tests by behavior family before the
  next large CLI feature batch.
- [x] Split `dun-cli` pure status/help/file-dialog/text-width helpers out of
  `crates/dun-cli/src/main.rs`.
- [ ] Split `crates/dun-cli/src/main.rs` implementation into app, input,
  dialogs, files, terminal, command-output, and helper-text modules.
- [ ] Continue the `dun-cli` split with Stage 4 app-state method groups.
- [ ] Split `crates/dun-ui/src/lib.rs` into model, render, hit-testing, text,
  and test modules.
- [ ] Split `crates/dun-config/src/lib.rs` into keys, parser, defaults, and
  validation modules.
- [ ] Split `crates/dun-core/src/buffer.rs` into buffer storage, cursor,
  selection, edit, undo, search, and tests.
- [ ] Split `crates/dun-core/src/workspace.rs` and
  `crates/dun-term/src/theme.rs` when they are next touched for substantive
  work.

## Real Terminal Testing

Scope: do this after the current file-splitting line. Keep it automated and
skip cleanly when `tmux` is unavailable.

- [ ] Build a `tmux` screen-grid harness for fixed-size real-terminal runs.
- [ ] Capture alternate-screen output with `tmux capture-pane`, preserving SGR
  color and attributes.
- [ ] Add a normalized cell-grid parser shared by PTY/tmux tests, introducing
  `vt100` only if it stays Rust `1.85` compatible and lightweight enough.
- [ ] Add assertions for menu position, tiled borders, status bar placement,
  focused cursor, selection attributes, and semantic color output.
- [ ] Keep mouse behavior in existing PTY/event-level tests rather than the
  `tmux` harness.
- [ ] Add a later Microsoft Edit differential path that compares only projected
  semantic regions such as editor body text, cursor, selection coverage, and
  color classes, not whole-screen pixel or cell equality.
- [ ] Keep GUI pixel screenshots as optional manual visual regression only,
  outside the CI/main automated test path.

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
- [x] Define opt-in OSC 52 external-copy config with a payload byte limit.
- [x] Define configurable file-dialog modal keybindings.
- [x] Define large-file and display limits.
- [x] Expose built-in defaults through `--dump-config`.
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
- [x] Add UTF-8-safe prompt cursor editing for command/find/replace/go-to-line
  prompts.
- [x] Add bracketed paste routing for editor buffers, prompts, and file
  dialogs.
- [x] Add right-click paste status handling without invoking external clipboard
  commands.
- [x] Implement internal Cut/Copy/Paste baseline for active selections without
  using the OS clipboard.
- [x] Add keyboard selection baseline with `Shift+Arrow` and `Shift+Home/End`.
- [x] Add editor PageUp/PageDown movement and viewport synchronization.
- [x] Add `Shift+PageUp/PageDown` page-wise selection.
- [x] Report Undo/Redo status feedback.
- [x] Add scroll range and horizontal offset status fields.
- [x] Add optional mouse wheel scrolling for editor panes.
- [x] Add cached search match highlighting and focused match status fields.
- [x] Add command-line `replace all QUERY TEXT` with one undo transaction.
- [x] Add explicit horizontal viewport scroll commands.
- [x] Add a lightweight vertical scrollbar indicator for long buffers.
- [x] Add incremental Find and Replace query preview in modal prompts.
- [x] Add an interactive Replace confirmation flow with Replace, Skip, All,
  and Cancel actions.
- [x] Add mouse click/drag support for the editor scrollbar when mouse support
  is enabled.
- [x] Add horizontal viewport edge indicators and ratatui visual smoke tests
  for viewport polish.
- [x] Add current-line selection command and keyboard binding.
- [x] Add ignore-case and whole-word search prefixes for Find and Replace.
- [x] Add mouse selection edge scrolling.
- [x] Add file-dialog overwrite confirmation, inline error retention, and
  session recent-directory reuse.
- [x] Add stable ratatui text snapshot coverage for baseline UI layout.
- [x] Add buffer switcher overlay for already-open buffers.
- [x] Detect external file changes and reject unsafe Save overwrites.
- [x] Add explicit Reload from disk for focused file buffers.
- [x] Add line commands for copy/delete/move/indent/outdent/trim.
- [x] Add word-wrap, visible-whitespace, and bookmark baseline commands.
- [x] Add Turbo Pascal-style shell escape that suspends and resumes the TUI.
- [x] Add Run Command prompt with bounded read-only output buffer.
- [x] Add Run Command history/output polish and read-only output pane reuse.
- [x] Add PTY smoke coverage for shell escape suspend/resume behavior.
- [x] Add opt-in OSC 52 external copy for active selections while preserving
  the internal clipboard fallback.
- [x] Add Command Output clear/copy/stderr/save commands.
- [x] Add Command Output summary/stdout/stderr navigation and output find.
- [x] Add Command Output Save dialog integration.
- [x] Add Command Output status/truncated quick jumps and View-menu coverage.
- [x] Strengthen Command Output Save dialog overwrite/error coverage.
- [x] Add Command Output index, body-line jumps, and next/previous search
  repeat.
- [x] Polish Config Diagnostics summaries for source, clipboard, limits, and
  keymap coverage.
- [x] Improve Config Diagnostics grouping with top-level Summary and Paths.
- [x] Add Config Diagnostics section jump commands.
- [x] Make soft-wrap PageUp/PageDown movement and selection use visual rows.
- [x] Strengthen soft-wrap paging tests for wide characters, tabs, and control
  bytes.
- [x] Add document start/end navigation for read-only and editable panes.
- [x] Keep menu mnemonics unique within each menu.
- [x] Add read-only outline/section list and section jump commands.
- [x] Add read-only search result list and numbered result jumps.
- [x] Add Command Output section navigation and stdout/stderr-only views.
- [x] Add command-line Tab completion for built-in command families.
- [x] Expand Outline detection for common Markdown, INI/TOML, Rust, and shell
  section lines.
- [x] Add `n`/`p` and `Enter` row navigation for Outline and Search Results.
- [x] Add command-line completion candidate cycling and path completion.
- [x] Make Command Output only-views searchable/saveable as the current output.
- [x] Restore focus from closable read-only helper panes to their source where
  a source exists.

## File and Display Safety

- [x] Implement UTF-8-first file loading behavior.
- [x] Open invalid UTF-8 files as read-only escaped fallback buffers.
- [x] Define invalid-byte fallback behavior.
- [x] Track file-text encoding and expose escaped-byte fallback state.
- [x] Reject unstable/corrupt reads when file metadata changes during Open.
- [x] Prevent save from silently corrupting lossy/fallback buffers.
- [x] Save files through same-directory temporary files and atomic rename.
- [x] Refuse normal Save when the loaded file's metadata snapshot no longer
  matches the current path.
- [x] Add readable path diagnostics for common open/save failures.
- [x] Define large-file soft limit behavior.
- [x] Add large-file performance baselines.
- [x] Add lightweight release binary size audit for macOS and Debian builds.
- [x] Add lightweight runtime memory and startup baseline for macOS and Debian
  builds.
- [x] Add release size repeat checklist.
- [x] Add dependency/feature audit and minimal default-build feature policy.
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
- [x] Fix strict VT100/16-color output so low-capability profiles do not emit
  256-color-style `38;5;n` or `48;5;n` SGR sequences.
- [x] Add automated event-level coverage for common modified terminal keys.
- [x] Run the external SSH and low-capability Debian VM matrix for `d2c832f`.
- [ ] Run a real server-console/KVM ASCII path before a tagged release that
  claims KVM coverage.
- [x] Add static Microsoft Edit reference tests for source-visible menu,
  status, color, and terminal setup markers.

## Deferred

- [ ] OSC 52 paste/query support or platform-specific clipboard command
  integration.
- [x] Crash recovery and orphaned atomic-save temp-file cleanup.
- [ ] `rum` configuration evaluation.
- [ ] `dun-plugin-api`.
- [ ] `dun-plugin-rum`.
- [ ] Syntax highlighting plugins backed by `rum`.
- [ ] Log viewing and filtering after `rum` is ready to embed.
- [ ] Memory watchdog design for long-running plugin evaluation.
- [x] Broad terminal compatibility test harness.
