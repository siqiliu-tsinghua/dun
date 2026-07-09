# TODO

This file tracks active and near-term work. Completed decisions and finished
items belong in [PROGRESS.md](./PROGRESS.md).

## Current Stage: Plugin Protocol Client

The active plan is the required host-neutral plugin protocol client. This does
not wait for `rum`; `rum` is a future optional pure-sandbox host that must
speak the same protocol.

The protocol client is a required runtime feature under
[docs/feature-budget.md](./docs/feature-budget.md). If it pushes
`target/release/dun` over `1,048,576` bytes on audited macOS or Debian builds,
trim optional editor features before cutting protocol-client functionality.
External plugin hosts and future runtime packages are separate artifacts and do
not count toward the default `dun` executable size.

Implementation reference: [docs/plugin-protocol.md](./docs/plugin-protocol.md).

### Protocol Specification

- [ ] Freeze protocol v0 message envelope: protocol version, `request_id`,
  `plugin_id`, `role`, optional buffer/stream `revision`, and payload.
- [ ] Define length-prefixed stdio framing:
  `u32 little-endian payload_length` plus UTF-8 JSON payload.
- [ ] Define frame and payload caps before allocation.
- [ ] Define structured protocol errors for malformed frame, unsupported
  version, unknown role, policy rejection, timeout, cancellation, host crash,
  oversized output, and stale revision.
- [ ] Define stderr handling as bounded human diagnostics, never protocol.

### Role and Policy Model

- [ ] Define `PluginRole` with at least `SyntaxHighlight`, `LogFilter`,
  `TextTransform`, and `ConfigHelper` variants.
- [ ] Define `TrustClass`: `pure-sandbox`, `user-trusted-external`, and
  `unsupported-unsafe`.
- [ ] Define `PluginPolicy`: max input bytes, max output bytes, timeout,
  diagnostic cap, allowed outputs, and whether user confirmation is required.
- [ ] Define plugin manifest/config fields for id, command, runtime,
  trust class, roles, and per-role policy overrides.
- [ ] Reject unknown trust classes, unknown roles, missing command paths, and
  any direct filesystem/process/network/terminal/editor authority request.

### Transport and Host Lifecycle

- [ ] Add a small Rust-owned protocol client module or crate without adding
  `rum` or heavy runtime dependencies.
- [ ] Launch configured external hosts directly, not through a shell.
- [ ] Pass only stdin/stdout/stderr plus a minimal environment or explicit
  whitelist.
- [ ] Implement `Hello`/`HelloAck`, `LoadPlugin`, role `Request`/`Response`,
  `Diagnostic`, `CancelRequest`, `Error`, and `Shutdown` paths.
- [ ] Add per-request timeout and cancellation.
- [ ] Kill or quarantine a host after malformed frames, oversized output,
  timeout, failed cancellation, EOF during frame, or process crash.
- [ ] Ensure plugin host failure never corrupts buffers, file state, terminal
  state, or workspace layout.

### First Applied Role

- [ ] Implement one visible low-risk role end to end, preferably
  `SyntaxHighlight`.
- [ ] Send bounded visible or nearby text snapshots with buffer revision and
  language hint.
- [ ] Validate returned style spans: known style ids, in-range line/column
  coordinates, sorted or normalized ranges, bounded count, and matching
  revision.
- [ ] Discard stale results when the buffer revision has changed.
- [ ] Apply validated results through existing UI highlight paths without
  granting plugins access to UI or terminal APIs.
- [ ] Keep plugin diagnostics sanitized and visible through existing status or
  diagnostics surfaces.

### Fixture Hosts and Tests

- [ ] Add a Rust fixture host for CI-grade protocol tests.
- [ ] Add optional Python fixture/examples only outside the required CI path.
- [ ] Test handshake success and protocol-version rejection.
- [ ] Test normal request/response for the first applied role.
- [ ] Test malformed frame, malformed JSON, oversized frame, oversized output,
  unknown role, forbidden output, timeout, cancellation, host crash, and stderr
  diagnostics.
- [ ] Test stale revision rejection leaves editor state unchanged.
- [ ] Test that rejected plugin output cannot request file I/O, process spawn,
  terminal writes, direct buffer mutation, or raw control-byte rendering.

### Release Gates for This Stage

- [ ] `cargo fmt --all -- --check`.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] `cargo test --workspace`.
- [ ] Release smoke checklist passes.
- [ ] macOS `target/release/dun` remains no larger than `1,048,576` bytes.
- [ ] Debian VM `target/release/dun` remains no larger than `1,048,576` bytes.
- [ ] If either binary exceeds budget, trim optional features in
  [docs/feature-budget.md](./docs/feature-budget.md) order and record the
  result before continuing.

Do not add runtime features while either audited release binary exceeds the
1 MiB budget.

## Completed Stage: v0.1 Release Hardening

- [x] Set the hard runtime budget: `target/release/dun` must be no larger than
  `1,048,576` bytes on both audited macOS and Debian builds.
- [x] Make the checked-in release profile the size-budget profile.
- [x] Classify implemented runtime features as required or optional.
- [x] Define the optional runtime trim order.
- [x] Record the current macOS release binary size.
- [x] Record the current Debian release binary size.
- [x] Run the release smoke checklist.

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
- [x] Extract `dun-cli` AppState window, editing/clipboard, and view-state
  method groups.
- [x] Extract `dun-cli` AppState mouse interaction and command-dispatch method
  groups.
- [x] Split `crates/dun-cli/src/main.rs` implementation into app, input,
  dialogs, files, terminal, command-output, and helper-text modules.
- [x] Continue Stage 4 with AppState prompts/dialogs, helper
  panes/search-replace, command output, and file I/O method groups.
- [x] Complete Stage 4 by moving remaining AppState construction, frame/view
  sync, menu state, command-line runner, and status/path display methods out
  of `crates/dun-cli/src/main.rs`.
- [x] Continue the CLI split with remaining `main.rs` responsibilities:
  process entry/runtime loop, CLI argument parsing, startup config loading,
  pure command-line parsing/completion helpers, help text assembly, command
  output text formatting, terminal profile detection, and residual text
  formatters.
- [x] Start Stage 5 by moving terminal raw/alternate-screen lifecycle and
  16-color SGR output rewriting into `crates/dun-cli/src/terminal/`.
- [x] Continue Stage 5 with terminal input dispatch, shell/run-command host
  process boundaries, and file open/save/snapshot/atomic I/O modules.
- [x] Split `crates/dun-ui/src/lib.rs` into model, render, hit-testing, text,
  and test modules.
- [x] Start Stage 7 by moving `dun-ui` unit tests into
  `crates/dun-ui/src/tests/` behavior modules.
- [x] Continue Stage 7 with `dun-ui` pure model type extraction.
- [x] Move `dun-ui` text width, truncation, wrapping, and visible-whitespace
  helpers into `text.rs`.
- [x] Move `dun-ui` workspace/menu/overlay hit-testing methods into `hit.rs`.
- [x] Continue Stage 7 with render function extraction by visual layer.
- [x] Finish the `dun-ui` facade split by moving `UiShell` and frame model
  construction out of `crates/dun-ui/src/lib.rs`.
- [x] Split `crates/dun-config/src/lib.rs` into keys, parser, defaults, and
  validation modules.
- [x] Start Stage 8 by moving `dun-config` unit tests into
  `crates/dun-config/src/tests/` behavior modules.
- [x] Move `dun-config` config model and limits model into `config.rs` and
  `limits.rs`.
- [x] Continue Stage 8 with key/keymap/file-dialog-keymap extraction.
- [x] Continue Stage 8 with parser/default-config/validation extraction.
- [x] Split `crates/dun-core/src/buffer.rs` into buffer storage, cursor,
  selection, edit, undo, search, and tests.
- [x] Start Stage 9 by moving `dun-core` buffer tests into behavior modules,
  moving buffer model/storage into `model.rs`, and moving search/replace-all
  logic into `search.rs`.
- [x] Continue Stage 9 with cursor/selection movement extraction.
- [x] Continue Stage 9 with edit, line-ops, and undo extraction.
- [x] Split `crates/dun-core/src/workspace.rs` and
  `crates/dun-term/src/theme.rs` when they are next touched for substantive
  work.

## Terminal Test Extensions

The tmux-backed real-terminal baseline is complete; see
[PROGRESS.md](./PROGRESS.md) and
[docs/real-terminal-tui-testing.md](./docs/real-terminal-tui-testing.md). Keep
this section focused on post-baseline extensions only.

- [ ] Add normalized-grid assertions for selection attributes and richer
  semantic color output only when a concrete diff case or regression risk needs
  those projections.

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
- [x] Add static Microsoft Edit reference tests for source-visible menu,
  status, color, and terminal setup markers.

## Deferred

- [ ] OSC 52 paste/query support or platform-specific clipboard command
  integration.
- [x] Crash recovery and orphaned atomic-save temp-file cleanup.
- [ ] `rum` configuration evaluation.
- [ ] `dun-plugin-rum`.
- [ ] Syntax highlighting plugins backed by `rum`.
- [ ] Full log viewing and filtering product after the plugin protocol client
  is working; untrusted third-party defaults wait for a pure `rum` host.
- [ ] Memory watchdog design for long-running plugin evaluation.
- [x] Broad terminal compatibility test harness.
