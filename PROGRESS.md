# PROGRESS

This is an append-only progress log. Keep new entries dated and factual.

## 2026-07-04

- Established the initial product direction for `dun`: a Rust `1.85`
  `ratatui`-based terminal editor for Linux/macOS SSH operations work.
- Confirmed the primary workflow: inspect and edit text files, read and filter
  logs, and support custom operational log filters.
- Confirmed terminal compatibility goals: UTF-8 plus 256 colors by default,
  with 16-color and ASCII fallback profiles.
- Surveyed the neighboring `rum` project and its current `rum-host` direction.
- Decided not to depend on `rum` yet because its release API is not fixed.
- Established the future plugin boundary: `rum` is used only as a pure
  evaluator inside `dun`; roles, policies, resources, and editor API access are
  owned by `dun`.
- Created the initial project documents: `README.md`, `AGENTS.md`, `PLAN.md`,
  `TODO.md`, `PROGRESS.md`, and `AUDIT.md`.
- Initialized the git repository and minimal Cargo binary package.
- Confirmed the local toolchain is `cargo 1.85.0` and `rustc 1.85.0`.
- Added `/reference/` to `.gitignore` and cloned `microsoft/edit` there as a
  local-only reference checkout.
- Inspected Microsoft Edit commit `10cbfcc7330c894f2173611029df44ca5cb6fd77`
  for visual layout, menu/status/dialog organization, and state model.
- Added `docs/msedit-reference.md` to record reference observations and Dun's
  own planned ratatui-oriented UI model.
- Found the Rust Turbo Vision port: `turbo-vision` / `aovestdipaperino/turbo-vision-4-rust`.
- Added a local-only checkout at `reference/turbo-vision-4-rust` and inspected
  commit `8a4c93d93efecc672a3e7ce330af35514ce1baf7`.
- Rejected the heavier desktop model for Dun's core UI: no sidebars, tabs,
  floating windows, z-order, or desktop-style maximize/minimize in the baseline.
- Decided Dun should keep a Microsoft Edit-inspired single-buffer opening
  screen, then grow through a lightweight tiling split tree inspired by
  `tmux`/`i3`/`awesome`.
- Rewrote `docs/window-management.md` around tiled child windows, split
  commands, directional focus, resizing, collapse/expand, and tree repair.
- Confirmed first-version product scope: build the Microsoft Edit-like editor
  foundation first; defer log/filter workflows until `rum` integration is ready.
- Chose UTF-8 as the default file encoding with a conservative invalid-byte
  fallback path.
- Chose Microsoft Edit-style default colors and single-line borders, with
  ASCII/16-color fallback and optional future Turbo Vision/dark/Dun themes.
- Decided keybindings must be configurable because terminals and KVM devices
  vary.
- Deferred mouse selection/paste and split dragging until the keyboard-first
  baseline works.
- Added `docs/editor-baseline.md`.
- Converted the root package into a Cargo workspace with five initial crates:
  `dun-core`, `dun-term`, `dun-ui`, `dun-config`, and `dun-cli`.
- Added minimal compile-tested skeleton types for commands, workspace layout,
  terminal profiles, glyphs, themes, config limits, and the CLI entry point.
- Added `docs/crate-map.md`.
- Rewrote `PLAN.md` and `TODO.md` into crate-specific implementation phases
  and task lists.
- Implemented the first pure `dun-core` tiling workspace model: split focused
  window, close focused window with tree repair, directional focus, ratio
  resize, collapse/expand state, equalize, rotate split, and rectangle
  resolution.
- Added `dun-core` unit tests for split-tree transitions and workspace edge
  cases. `cargo test --workspace` passes.
- Implemented the first `dun-core` text buffer baseline with line-based UTF-8
  storage, cursor and selection types, insert/delete/newline, replace
  transactions, undo/redo, dirty tracking, line-ending round trips, and
  read-only edit rejection.
- Added buffer unit tests for UTF-8 boundaries, CRLF/LF parsing, selection
  replacement, multiline deletion, undo/redo, dirty tracking, and read-only
  behavior. `cargo test --workspace` passes.
- Implemented the first `dun-term` terminal rendering baseline: profile
  selection for UTF-8/ASCII and 256-color/16-color/mono, Unicode and ASCII
  glyph sets, Microsoft Edit-style 256-color and 16-color themes, mono
  fallback, and optional Turbo/dark/Dun theme entries.
- Implemented `dun-core::DisplaySanitizer` for safe display of untrusted text,
  including visible control rendering, OSC/escape neutralization, ASCII
  fallback escaping, and long-line caps.
- Implemented the first `dun-config` typed configuration schema: theme
  selection, terminal overrides, limits validation, typed key sequences,
  default keymap, command ids, duplicate binding validation, and tests.
- Connected `dun-ui` to config/core/term with a backend-neutral frame model
  that resolves theme, glyphs, keybindings, sanitized buffer lines, menu data,
  status data, and tiled window rectangles without file I/O.
- Added the first runnable `ratatui` shell using `ratatui 0.29.0` and
  `crossterm 0.28.1`, selected by Cargo as compatible with Rust `1.85`.
- Added terminal lifecycle handling in `dun-cli`: raw mode, alternate screen,
  restoration guard, redraw loop, terminal profile detection from environment,
  crossterm key event conversion, keymap lookup, and `Ctrl+Q` quit.
- Added the first editor command application layer in `dun-cli`: commands now
  dispatch to app, file, edit, and window handlers over focused buffers/windows.
- Added text input routing for printable characters, edit commands for cursor
  movement/newline/backspace/delete/undo/redo/select-all, and scroll tracking
  to keep the focused cursor line visible.
- Added multi-stroke key sequence prefix tracking so configured bindings such
  as `Ctrl+W,H` can trigger tiled-window commands.
- Added close-window cleanup for unreferenced CLI buffer state.
- Added focused cursor placement to the `dun-ui` frame/rendering model, mapped
  through sanitized display text, terminal display width, and per-buffer
  vertical scroll offsets.
- Extended tests for CLI command application, text input filtering, key
  sequence dispatch, config sequence prefixes, and UI cursor mapping.
  `cargo test --workspace` passes.
- Added the first file open/save baseline in `dun-cli`: startup accepts one
  file path, UTF-8 text files load into the focused buffer, invalid UTF-8 is
  rejected before entering the TUI, `Ctrl+S` saves loaded buffers back to their
  path, and status messages report open/save results.
- Added CLI file tests for command-line open, CRLF preservation, invalid UTF-8
  rejection, save-to-path, save-without-path status, and clearing file metadata
  on `FileCommand::New`. `cargo test --workspace` passes.
- Added an interactive status-line prompt mode in `dun-cli` with `Enter`
  submit, `Esc` cancel, and `Backspace` editing. `FileCommand::Open` now
  prompts for a path and opens it, `FileCommand::SaveAs` prompts for a path and
  attaches the focused buffer to that file, and `EditCommand::Find` captures a
  query for the future find baseline.
- Added prompt tests for open, save-as, find query capture, cancel behavior,
  and keeping prompt input out of the editor buffer. `cargo test --workspace`
  passes.
- Added the first find baseline: `dun-core::TextBuffer::find_all` returns
  UTF-8-safe match ranges, `Ctrl+F` prompts for a query and selects the first
  match, `F3` moves to the next match, and `Shift+F3` moves to the previous
  match with wraparound status feedback.
- Added selected-range rendering in `dun-ui`, using the existing selection
  theme colors and display-width mapping for UTF-8 text.
- Added tests for core search ranges, find prompt navigation, next/previous
  wraparound, missing-query/missing-match status, and UI selection mapping.
  `cargo test --workspace` passes.
- Added a line-number gutter to `dun-ui` windows. Gutter width follows the
  buffer line count, respects vertical scroll offsets, is disabled on very
  narrow panes, and shifts editor text/cursor/selection geometry consistently.
- Added focused-buffer status text in `dun-cli`: the left status area shows the
  focused file or untitled buffer with dirty/read-only markers, and the right
  status area shows 1-based `Ln`/`Col` using terminal display width.
- Added tests for scrolled gutter labels, shifted cursor/selection geometry,
  dirty buffer status, and wide-character column status. `cargo test
  --workspace` passes.

## 2026-07-05

- Added unsaved-changes confirmation in `dun-cli` before dirty buffers are
  lost through quit, new, open, or close-window commands.
- Confirmation is status-line based: `s` saves and continues, `d` discards and
  continues, and `c`/`Esc` cancels. Untitled buffers use the existing Save As
  prompt before continuing the pending action.
- Added CLI tests for new/open/quit/close dirty-buffer protection, save-before
  quit, and Save As-before-quit. `cargo test --workspace` passes.
- Added the first replace baseline in `dun-cli`: `Ctrl+R` prompts for search
  text and replacement text, then replaces the current selected match or the
  next match from the cursor with wraparound. Empty replacement text is valid
  and deletes the match.
- Added replace tests for prompt flow, selected-match replacement, empty
  replacement, and missing-match status. `cargo test --workspace` passes.
- Added Go To Line baseline: `Ctrl+G` and the menu command open a status-line
  prompt, accept a 1-based line number, preserve the current column where
  possible, and report invalid or out-of-range input.
- Added CLI tests for successful Go To Line movement and invalid/out-of-range
  input, plus a config test for the default `Ctrl+G` binding. `cargo test
  --workspace` passes.
- Added the first Help/key reference screen: `F1` or the Help menu command
  opens a read-only tiled Help window, focuses an existing Help window instead
  of creating duplicates, and lists the current default editor/window/prompt
  keys.
- Added CLI tests for Help window creation, read-only help content, duplicate
  prevention, cleanup on close, and F1 dispatch. `cargo test --workspace`
  passes.
- Added status history baseline: `set_status` records recent status messages
  with simple info/error classification, `F2` or the Status menu opens a
  read-only Status History window, and the window refreshes while it is open.
- Added tests for status history window creation, duplicate prevention, close
  cleanup, live refresh, capped history length, F2 dispatch, default keymap,
  and Status menu exposure. `cargo test --workspace` passes.
- Expanded the right status bar details in `dun-cli`: it now shows current
  line over total line count, display column, active selection summary,
  line-ending style, terminal encoding/color profile, and focused window index.
- Added CLI tests for UTF-8 display columns, CRLF metadata, terminal profile
  labels, focused window index, and selection summaries. `cargo test
  --workspace` passes.
- Added stable CLI argument parsing in `dun-cli`: `--help`/`-h`,
  `--version`/`-V`, optional single startup path, `--` path separator,
  unknown-option errors, and multi-path usage errors.
- Changed the process entry point to return explicit exit codes: `0` for
  success/help/version, `1` for runtime or file I/O errors, and `2` for
  command-line usage errors. Added parser and exit-code tests. `cargo test
  --workspace` passes.
- Added the first PTY smoke test harness for `dun-cli` using the system
  `expect(1)` command when available. The tests run the real `dun` binary in a
  fixed-size pseudo-terminal, send `Ctrl+Q`, and check clean startup/exit under
  `xterm-256color`, `screen-256color`, and `vt100`-style profiles, plus startup
  rendering for an opened UTF-8 file.
- Recorded future optional Microsoft Edit reference/differential tests in
  `TODO.md`, to be added only when Dun's own baseline behavior is mature enough
  for useful comparison.
- Added `docs/terminal-compatibility-checks.md` with the current PTY regression
  gate, manual SSH terminal matrix, startup/edit/search/tiling checklist, and
  pass criteria for terminal restoration and fallback rendering.
- Closed the Phase 8 manual terminal-check item for the current workspace by
  documenting current-environment coverage. Real external SSH host coverage is
  kept as a release-hardening task rather than implied by local automation.
- Stabilized the PTY smoke harness so it waits for a rendered ready marker
  before sending `Ctrl+Q`, reducing timing-sensitive missed-key failures.
  `cargo test -p dun-cli --test pty_smoke` and `cargo test --workspace` pass.
- Implemented editable file soft-limit enforcement in `dun-cli`. Startup and
  interactive Open now reject files above
  `Config::limits.editable_file_soft_limit_bytes` before they become editable
  buffers, with the current default remaining 16 MiB.
- Added CLI tests for accepting files exactly at the soft limit, rejecting
  files over the limit on startup, and reporting over-limit files through the
  Open prompt status path.
- Implemented invalid-byte fallback file loading in `dun-cli`. Invalid UTF-8
  files now open as read-only buffers with valid UTF-8 spans preserved and
  invalid bytes escaped visibly as `\xNN`.
- Added save protection for read-only fallback buffers: Save and Save As reject
  them instead of writing escaped fallback text over the original bytes.
- Added CLI tests for invalid UTF-8 fallback display, valid Unicode preservation
  inside fallback buffers, and Save/Save As rejection for fallback buffers.
- Added the first Rust-owned configuration file loader. `dun-config` now parses
  line-based `key = value` overrides for theme, terminal overrides, limits, and
  command keybindings without adding external parser dependencies.
- Wired startup config loading into `dun-cli` through `--config PATH`,
  `--config=PATH`, `--no-config`, `DUN_CONFIG`, and default
  `$XDG_CONFIG_HOME/dun/config` or `$HOME/.config/dun/config` discovery.
- Added parser and CLI tests for config overrides, keybinding unbinds,
  duplicate key detection, explicit config loading, and parse-error reporting.
- Integrated configured command keybindings into the runtime command path:
  changed bindings replace their defaults, disabled bindings stop dispatching,
  and startup config conflicts report the config path plus duplicate key text.
- The Help window now renders its key reference from the active keymap and
  includes command ids for config editing. `cargo test -p dun-config` and
  `cargo test -p dun-cli` pass.
- Added runtime config reload through `AppCommand::ReloadConfig`, default
  `F5`, and `app.reload_config`. Reload keeps buffers and workspace intact,
  updates shell/keymap/theme-derived state and limits, refreshes an open Help
  window, and reports failures through status history without exiting.
- Added CLI tests for successful reload, failed reload preserving the previous
  keymap, and Help refresh after reload. `cargo test --workspace` passes.
- Added Config Diagnostics through `AppCommand::ConfigDiagnostics`, default
  `F6`, and `app.config_diagnostics`. The read-only diagnostics window shows
  config source/request, environment/default paths, detected/effective terminal
  profile, theme, limits, glyph mode, and the active keymap.
- Added CLI tests for opening Config Diagnostics, F6 dispatch, and diagnostics
  refresh after runtime config reload.
- Added a command-line prompt baseline on `AppCommand::CommandLine` (`Ctrl+P`
  by default). It supports help/config/status/reload/quit, open/save/save-as,
  new/close, find/replace, go-to-line, command listing, simple quotes, and
  backslash escapes.
- Added CLI tests for command prompt dispatch, quoted file paths, dirty-buffer
  open protection, unknown commands, and parser errors.
- Added runtime theme selection through the command-line prompt:
  `theme` reports the active theme and `theme msedit|turbo|dark|dun` switches
  the current session without writing config files. Runtime reload restores the
  configured theme.
- Added CLI tests for theme switching, diagnostics refresh, unknown theme
  errors, and reload restoring the configured theme.
- Polished tiled-window command application in `dun-cli`: split, focus,
  resize, equalize, rotate, collapse/expand, close, and the deferred `only`
  command now report visible success or failure status instead of silently
  ignoring no-op/error cases.
- The command-line prompt now falls back to the typed command-id parser, so
  full ids such as `window.split_horizontal` and `window.focus_left` can be
  executed directly when they take no arguments.
- Added CLI tests for tiling command status feedback, close-last-window
  rejection, and command-line execution of `window.*` command ids.
- Added an atomic save baseline in `dun-cli`: Save and Save As now write
  through same-directory temporary files, sync the temporary file, and rename
  it over the destination only after the write succeeds.
- Atomic saves preserve existing destination permissions, reject read-only
  destinations before replacement, clean up temporary files on error, and
  resolve symlink paths so the linked target is updated without replacing the
  symlink itself.
- Added CLI tests for atomic temp cleanup, read-only destination rejection, and
  save-through-symlink behavior.
- Added command-line prompt history in `dun-cli`. Submitted non-empty commands
  are stored in a bounded in-memory history, consecutive duplicates are
  skipped, Up/Down recall previous or newer commands, and Down restores the
  current prompt draft after history navigation.
- Added Help text for command history navigation and CLI tests for recall,
  repeat-last-command behavior, history capping, duplicate suppression, and
  keeping non-command prompts unaffected.
- Polished Open, Save, and Save As path diagnostics. File-operation errors now
  include the relevant path and normalize common cases such as missing files,
  directory paths, missing parent directories, permission denial, read-only
  destinations, non-regular files, and large-file soft-limit rejection.
- Added CLI tests for missing Open paths, directory Open paths, missing Save As
  parent directories, directory Save As targets, and read-only Save targets.
- Polished tiled rendering for small terminals. Split layout keeps both
  children visible when the parent has room, narrow panes drop the line-number
  gutter before it crowds out the editable body, tiny title-bar-only panes omit
  body/gutter/cursor rendering, and title/status text now clips by terminal
  display width.
- Added core/UI tests for tiny split rectangles, narrow panes with large line
  numbers, CJK title/status clipping, and title-bar-only pane rendering.
- Added ignored large-file performance baselines in `dun-cli`. The baseline
  covers startup open, sparse and missing `find_all`, scroll synchronization to
  end of file, visible-window UI frame construction, ratatui `TestBackend`
  drawing, and long-line display caps.
- Added `docs/performance-baselines.md` with the run command, environment
  overrides for fixture sizes, covered paths, and the current local release
  sample. `cargo test -p dun-cli --release large_file_perf -- --ignored
  --nocapture` passes with an 8 MiB fixture.
- Added crash-recovery handling for Dun atomic-save temp files. Open and Save
  now reconcile same-destination `.dun-save-<pid>-<attempt>.tmp` files:
  obsolete temp files older than the destination are removed, while newer temp
  files are preserved as recovery candidates and reported through status text.
- Added CLI tests for stale temp cleanup on open, newer recovery temp
  reporting, stale temp cleanup after a successful save, and preservation of a
  pre-existing recovery candidate across Save.
- Expanded the control-byte rendering security audit suite. `dun-core` now
  tests a matrix of terminal-control payloads including CSI/SGR, clear-screen,
  OSC title/clipboard/hyperlink, DCS, graphics escapes, bracketed paste
  markers, all C0 controls, and all C1 controls in both UTF-8 and ASCII modes.
- Hardened `dun-ui` chrome rendering by sanitizing pane titles and status
  fields before final ratatui rendering. Added UI tests for malicious
  title/status text, ASCII fallback chrome, and final `TestBackend` output with
  untrusted title/body/status payloads.
- Added corrupt/unstable file handling for Open. Editable file loading now
  captures metadata before reading and rejects the open if the file length,
  modification time, or Unix device/inode changes before the read completes, or
  if the path disappears.
- Added CLI tests for stable reads, truncation during read, deletion during
  read, and same-size replacement detection on Unix.
- Added an explicit non-UTF-8 file strategy. `dun-core` now decodes file bytes
  as UTF-8 when possible and otherwise returns a read-only escaped byte view
  tagged as `EscapedBytes`.
- CLI buffer state now tracks file-text encoding, shows escaped-byte fallback
  state in buffer/status fields, and treats only UTF-8 buffers as save-safe.
- Defined the external SSH and low-capability terminal release matrix. The
  matrix now covers local PTY, direct SSH, tmux, screen, VT100/C locale, mono,
  small terminal, and KVM/ASCII-style cases with a result-record template and
  low-capability pass criteria.
- Expanded `dun-cli` PTY tests into a broader terminal compatibility harness.
  The harness now covers xterm/screen/tmux/VT100/ANSI/dumb/NO_COLOR profiles,
  a small VT100-style terminal, safe rendering of terminal escape payload
  files, and invalid-byte escaped fallback files.
- Added lightweight static Microsoft Edit reference tests. The test suite
  checks `edit --help` when the reference binary is available, verifies Dun's
  own CLI contract, and scans `reference/msedit` source for menu, status bar,
  color, and terminal setup reference markers.
- Added an optional mouse support baseline. `mouse.enabled` defaults to false
  and can be loaded or reloaded through config; when enabled, `dun-cli` enters
  crossterm mouse capture, restores it on exit or disable, and handles left
  clicks for tiled-window focus plus editor-body cursor placement.
- Added core/UI/CLI tests for workspace coordinate focus, UI hit testing,
  wide-character cursor mapping, disabled mouse behavior, window focus, and
  body cursor placement.
- Extended optional mouse support with menu command clicks, editor-body
  selection drag, and split-border dragging. Split dragging is backed by a pure
  `dun-core` split handle and ratio update API.
- Added tests for menu hit testing, mouse menu dispatch, selection drag,
  split drag, and a PTY smoke case that starts and exits with
  `mouse.enabled = true`.
- Reworked the top menu into grouped File/Edit/View/Help dropdowns in
  `dun-ui`, with submenu labels, active-menu rendering, keymap-derived
  shortcut text, and hit testing that returns existing typed `EditorCommand`
  values.
- Updated `dun-cli` mouse handling so top-menu clicks open or close dropdowns,
  submenu clicks dispatch commands, outside clicks close an open menu, and
  `Esc` closes a menu before normal keymap dispatch.
- Added keyboard menu access for the grouped menus. If the active keymap does
  not consume the stroke first, `Alt+F/E/V/H` opens File/Edit/View/Help,
  Left/Right switches menus, Up/Down changes the selected entry, Enter
  dispatches the selected typed command, and Esc closes the menu.
- Added `.DS_Store` to `.gitignore` and recorded local Microsoft Edit
  screenshot observations. The visual alignment backlog now calls out active
  top-menu color, gray dropdown/modal panels, status-bar field formatting,
  current-line highlight, menu mnemonics, and modal prompt/file-dialog work.
- Tuned the default `msedit` theme and rendering chrome against the local
  screenshots: blue menu/status bars, green active top-menu labels, gray
  dropdown panels, bracket-style status fields, current-line highlighting, and
  a persistent gutter separator.
- Added a lightweight modal overlay renderer and routed existing prompt and
  unsaved-confirmation state through it, preserving the current prompt logic
  while moving Open/Save As/Find/Replace/Go To Line/Command prompts off the
  status bar.
- Added MacBook-friendly default window-management aliases: `Ctrl+W,Arrow`
  moves tiled focus and `Ctrl+W,Shift+Arrow` resizes splits, while `Alt`
  arrow bindings remain compatibility aliases for terminals that deliver
  Option/Meta keys.
- Added an Open/Save As file dialog baseline. `dun-ui` overlays now support
  selectable list rows and wider file-dialog panels; `dun-cli` owns a typed
  file-dialog state with directory listing, Up/Down selection, directory
  navigation, Esc cancel, Enter submit, and Tab path completion.
- Added tests for Open dialog directory navigation, unique Tab completion,
  selection-driven Open, Save As directory completion before save, and file
  dialog overlay rendering.
- Added optional mouse support for Open/Save As file dialogs. `dun-ui` now
  exposes overlay list-row hit testing, and `dun-cli` maps file dialog clicks
  to directory navigation, Open file submission, or Save As path-input updates
  without adding direct mouse-driven file I/O paths.
- Added file dialog list scrolling and page navigation. File dialogs now keep
  an explicit scroll offset, PageUp/PageDown move selection by page, mouse
  wheel events scroll the list, and click hit testing follows the scrolled
  visible range.
- Added file dialog parent-directory and hidden-file polish. Open/Save As
  lists now include a `..` directory entry, hide dotfiles by default, reveal
  dotfiles when the typed prefix starts with `.`, and support `Ctrl+H` to
  toggle hidden entries without bypassing validated file I/O.
- Added file dialog path-input editing, diagnostics, visual polish, and modal
  key configuration. Open/Save As inputs now support Left/Right/Home/End and
  Delete/Backspace at UTF-8 character boundaries, show clearer empty/no-match
  diagnostics, render more explicit file-list labels, and resolve file-dialog
  actions through typed configurable single-stroke bindings listed in Help and
  Config Diagnostics.
- Added `--dump-config` to print the built-in default configuration, including
  global command bindings and file-dialog modal bindings, and covered it with
  parser/config tests.
- Added UTF-8-safe cursor editing to lightweight prompts, so command/find/
  replace/go-to-line inputs support Left/Right/Home/End/Delete/Backspace
  without touching the editor buffer.
- Added bracketed paste handling. During the TUI session `dun-cli` enables
  bracketed paste, routes `Event::Paste` into editor buffers, prompts, or file
  dialogs, ignores paste during unsaved-change confirmations, and disables
  bracketed paste during terminal restoration.
- Added right-click paste status handling for mouse-enabled terminals without
  invoking external clipboard commands or OSC 52 clipboard behavior.
- Added tests for prompt cursor editing, bracketed paste routing, read-only
  paste rejection, right-click paste status, and file-dialog overlay structure
  fields used by the Microsoft Edit-like modal layout.
- Added an internal Cut/Copy/Paste baseline in `dun-cli`. The app now keeps a
  process-local internal clipboard, copies selected text without mutating the
  buffer, cuts selected text through the normal delete transaction path, and
  pastes the internal clipboard through the normal insertion path.
- Internal clipboard behavior rejects empty selections and read-only mutation
  targets with visible status messages, remains separate from the OS clipboard
  and OSC 52, and is covered by CLI tests for copy, cut, paste replacement,
  undo after cut, empty clipboard, and read-only buffers.
- Added keyboard selection baseline support. `dun-core::TextBuffer` now has
  UTF-8-safe selection extension methods for left/right/up/down and line
  start/end. `dun-cli` maps unbound `Shift+Arrow` and `Shift+Home/End` strokes
  to those methods after the active keymap has had priority.
- Added tests for selection anchor/cursor behavior, vertical preferred-column
  selection, line-edge selection, CLI Shift selection fallback, and configured
  Shift-arrow bindings taking precedence over fallback selection.
- Added undo transaction coalescing for continuous ordinary character input in
  `dun-core`. Cursor movement, selection changes, delete, replace, newline,
  paste-like `insert_str` calls, undo, and redo break the active typing group.
- Added buffer tests covering UTF-8 typing coalescing, cursor-motion breaks,
  paste-like bulk insertion boundaries, and redo followed by new typing.
- Added same-direction Backspace/Delete undo coalescing in `dun-core`, while
  keeping direction changes and word deletion as explicit transaction
  boundaries.
- Added UTF-8-safe word movement, word selection, and word deletion commands.
  Defaults now bind `Ctrl+Left/Right`, `Ctrl+Shift+Left/Right`, and
  `Ctrl+Backspace/Delete`, with command ids available through config and the
  command prompt.
- Added editor PageUp/PageDown movement based on the active pane body height,
  plus optional mouse wheel scrolling for editor panes that keeps the cursor
  visible.
- Added tests for delete-run undo grouping, word boundary behavior, default
  keybindings, editor page movement, word edit command dispatch, and editor
  mouse wheel scrolling.
- Added scroll position indicators to the focused detail status. The status
  now reports the visible line range and includes an `X` offset when a long
  line is horizontally scrolled.
- Added horizontal editor scrolling for long lines by carrying a display-column
  offset through `BufferState`, `BufferView`, cursor mapping, selection
  mapping, body rendering, and mouse hit testing.
- Added `Shift+PageUp/PageDown` selection commands, default keybindings, help
  text, and command ids. Selection extends by the active pane body height.
- Added visible Undo/Redo status feedback for successful actions, empty stacks,
  and read-only/focused-buffer errors.
- Added tests for horizontal render/hit mapping, horizontal cursor visibility,
  page-wise selection, scroll status, and Undo/Redo status feedback.
- Added cached search result view state per buffer. Find now feeds `dun-ui`
  search-match highlights, tracks the active match, and exposes `[Find n/m]`
  in the focused status fields without recomputing matches every frame.
- Polished replace flow: replacing one match now reports the replacement text,
  advances to the next remaining match when possible, and keeps search
  highlights in sync. The command prompt also accepts `replace all QUERY TEXT`.
- Added `TextBuffer::replace_all` as a single undo transaction and covered it
  with core and CLI tests.
- Added explicit horizontal viewport commands `edit.scroll_left` and
  `edit.scroll_right`, default `Ctrl+W,[`/`Ctrl+W,]` bindings, View-menu
  entries, command ids, help text, and horizontal mouse wheel handling when a
  terminal delivers it.
- Added lightweight right-border scrollbar thumbs for vertically scrollable
  editor panes, with 256-color, 16-color, and mono theme styles.
- Added incremental Find and Replace query preview in modal prompts. Typing in
  Find/Replace Find now selects the matching text immediately, Enter commits
  the previewed match, and Esc restores the prior cursor, selection, and search
  state.
- Reworked interactive Replace into a confirmation modal with Replace, Skip,
  All, and Cancel actions. Command-line `replace QUERY TEXT` remains a direct
  current/next replacement path, while interactive replacement no longer
  mutates text until the user confirms an action.
- Added mouse click and drag support for editor right-border scrollbars when
  `mouse.enabled = true`, backed by UI hit testing that maps scrollbar rows to
  target first-visible lines.
- Added horizontal viewport edge indicators for clipped long lines and a
  ratatui `TestBackend` visual smoke test that verifies the edge marker and
  scrollbar thumb are emitted. `cargo test -p dun-ui -p dun-cli` passes.
- Added selection polish: `edit.select_line` selects the current line including
  the following separator when possible, defaults to `Ctrl+L`, appears in Help
  and the Edit menu, and mouse drag selection now scrolls when dragged to a
  window edge.
- Added search/replace polish: Find, interactive Replace, and command-line
  replace accept `/i`, `/w`, and `/iw` search prefixes for ignore-case and
  whole-word matching. Search caches now preserve these options, and
  option-aware replace-all remains one undo transaction.
- Added viewport polish for wide-character horizontal scroll boundaries.
  Cursor, selection, and search-match mapping now use the actual visible byte
  start as the body origin, avoiding false highlights when a viewport begins
  inside a double-width character.
- Added file-dialog polish: Open/Save As errors stay in the dialog as inline
  messages, successful dialogs remember the last directory for the session,
  and Save As requires a second Enter before overwriting an existing file.
- Added a stable ratatui text snapshot helper and baseline UI layout snapshot
  coverage. `cargo test -p dun-core -p dun-config -p dun-ui -p dun-cli`
  passes.
- Ran the external Debian VirtualBox SSH matrix subset for commit `e636460`.
  Debian system packages provide `rustc 1.85.0` and `cargo 1.85.0`. The VM
  passed `cargo test -p dun-cli --test pty_smoke`, `cargo test --workspace
  --quiet`, direct SSH UTF-8, SSH mono, SSH tmux, SSH screen, escape-payload,
  and invalid-byte fallback checks. Strict VT100/small-terminal checks exposed
  that the current ratatui/crossterm path still emits `38;5;n`/`48;5;n` SGR
  color sequences for 16-color styles; this is now tracked in TODO.
- Fixed strict VT100/16-color output by routing Color16 terminal profiles
  through a stdout wrapper that rewrites crossterm's 0-15 palette SGR
  sequences into legacy 16-color SGR forms. Added unit coverage for split CSI
  handling and PTY assertions that Color16 profiles do not emit
  `38;5;n`/`48;5;n`. Re-ran the Debian VirtualBox matrix subset: PTY smoke,
  full workspace tests, direct SSH UTF-8, mono, VT100, small VT100, tmux,
  screen, escape-payload, and invalid-byte fallback checks all pass.
- Added editor polish batch: buffer switcher overlay, focused-file reload,
  external modification detection through validated file metadata snapshots,
  Save refusal when a loaded file changed on disk, line commands for copy,
  delete, move, indent, outdent, and trim, word-wrap and visible-whitespace
  display toggles, and line bookmarks with gutter markers. Added default
  keybindings, command ids, Help/menu entries, command-line aliases, UI
  rendering support, and focused tests across `dun-core`, `dun-cli`, and
  `dun-ui`. Current soft wrap is display-layer only; selection/search
  highlight geometry is deferred for a later polish pass.
- Added lightweight host process actions without embedding a terminal
  emulator. `app.shell_escape` (`Ctrl+W,S`) suspends raw/alternate-screen TUI
  state, runs the user's shell with inherited stdio, resumes terminal state,
  and redraws. `app.run_command` (`Ctrl+W,O`) opens a Run Command prompt,
  executes a non-interactive shell command with null stdin, captures stdout and
  stderr with a 512 KiB per-stream cap, and displays decoded output in a
  read-only Command Output window. Added command ids, File-menu entries,
  command prompt aliases, Help entries, config coverage, and CLI tests.
- Polished host process actions: Run Command now has a dedicated bounded
  prompt history, Command Output records stdout/stderr byte counts and explicit
  truncation state, repeated runs reuse and refresh the read-only output pane,
  and the PTY smoke suite covers shell escape suspend/resume with a temporary
  non-interactive shell script.
- Ran the external Debian VirtualBox terminal matrix for `d2c832f` from a
  local git archive snapshot without touching the VM's dirty `/home/fft/dun`
  worktree. The VM passed `cargo test -p dun-cli --test pty_smoke`,
  `cargo test --workspace --quiet`, direct SSH UTF-8, mono, VT100, 40x12
  VT100, tmux, screen, escape-payload, invalid-byte fallback, shell escape, and
  Run Command checks. Real server-console/KVM ASCII coverage remains unavailable
  and is still tracked separately.
- Added soft-wrap highlight geometry for selections and search matches. Wrapped
  rows now map highlights through the same visual row model used by body
  rendering, with tests covering multi-row selection and active search spans.
- Added opt-in OSC 52 external copy. `clipboard.osc52.enabled` defaults to
  false, `clipboard.osc52.max_bytes` bounds the payload, and
  `edit.copy_external` keeps the selected text in the internal clipboard even
  when terminal clipboard output is disabled or rejected by the byte limit.
- Polished Command Output with typed clear, copy, stderr-jump, and
  command-line save actions. The read-only output pane can now be cleared,
  copied to the internal clipboard, navigated directly to stderr, or saved via
  `output save PATH`.
- Strengthened soft-wrap viewport behavior from line-level scrolling to
  visual-row scrolling. Wrapped panes now preserve an intra-line visual row
  offset, and UI rendering, cursor placement, selection/search highlights,
  gutters, scrollbars, hit testing, mouse wheel scrolling, and status fields
  use that offset.
- Expanded Command Output navigation and search. Typed commands can jump to
  summary, stdout, or stderr; command prompt `output find QUERY` focuses the
  read-only output buffer and reuses the normal Find cache/highlight path; and
  `output save` without a path opens the file dialog while `output save PATH`
  remains available.
- Polished Config Diagnostics with default-config guidance, binding counts,
  important-unbound command summaries, and explicit OSC 52/limit details.
- Completed a follow-up editor/output polish batch. Soft-wrap PageUp/PageDown
  now moves and extends selection by wrapped visual rows, including exact-width
  wrap-boundary cursor positions. Command Output gained status and truncation
  quick jumps through app commands, command prompt helpers, Help, and the View
  menu; Save Output dialog tests now cover overwrite confirmation and write
  errors. Config Diagnostics now starts with readable Summary and Paths groups
  before detailed source, terminal, clipboard, limits, and keymap sections.
- Added a second navigation polish batch. Command Output now includes an Index
  section, can jump to stdout/stderr first non-empty body lines, and supports
  next/previous search repeat through commands and the command prompt.
  Soft-wrap paging has focused coverage for wide characters, tabs, and control
  bytes. Config Diagnostics can jump directly to named sections such as
  keymap, limits, and file-dialog keymap. `Ctrl+Home/End` now move to document
  start/end in both editable and read-only panes, and menu mnemonic uniqueness
  is covered by UI tests.
- Added read-only navigation helper panes for outlines and search results.
  `outline` lists section-like lines for the focused buffer and can jump back
  by number or name; `results` lists the current Find matches and can jump back
  by match number. Command Output gained section-relative navigation and
  stdout/stderr-only derived read-only panes. The command prompt now completes
  built-in command families with Tab, and automated coverage includes common
  modified-key events after crossterm parsing.
- Polished helper navigation and command completion. Outline detection now
  recognizes common Markdown headings, INI/TOML sections, Rust items, and shell
  functions. Outline and Search Results panes support `n`/`p` row selection and
  `Enter` source jumps, and closing helper panes returns focus to the source
  buffer or full Command Output pane where applicable. Command Output
  stdout/stderr-only views can be searched and saved as the current output.
  Command prompt completion now lists ambiguous candidates, cycles them with
  Tab/BackTab, and completes file-path arguments for open/save/output-save
  commands.
- Added a lightweight release binary size audit. On commit `4d89d07`, default
  release builds measured 1,627,136 bytes on macOS x86_64 and 1,881,392 bytes
  on Debian x86_64. A size-oriented release profile using `opt-level=z`, fat
  LTO, one codegen unit, stripped symbols, and `panic=abort` measured 859,544
  bytes on macOS and 1,034,840 bytes on Debian. Results and exact commands are
  recorded in `docs/release-size-audit.md`.
- Recorded the footprint conclusion from the size audit: the small Rust
  editor core is roughly 0.8-1.0 MiB in the audited size-oriented builds,
  while `rum` is currently treated as an approximately 6 MiB runtime. Therefore
  ordinary editor features should stay in Rust core code, and future `rum`
  integration should be optional or late-loaded for high-leverage plugin logic
  such as custom log filters and advanced text transforms.
- Added lightweight runtime-resource baselines for the size-oriented binaries.
  The audited empty TUI startup was 27 ms / 1,328 KiB RSS on macOS x86_64 and
  15 ms / 2,872 KiB RSS on the Debian x86_64 VM; opening a 1.1 MiB UTF-8 file
  measured 47 ms / 5,016 KiB on macOS and 32 ms / 6,500 KiB on Debian.
  Over-limit 17 MiB files were rejected before becoming editor buffers.
- Added dependency and feature audit documentation. `dun-cli` currently has
  direct runtime dependencies on `crossterm`, `ratatui`, `unicode-width`, and
  internal crates; the normal dependency tree has 67 unique package lines.
  `ratatui` default features remain disabled, `crossterm` default features are
  the first future feature-reduction candidate, and the default tree still has
  no `rum`, async/network/TLS stack, parser/highlighter stack, or plugin
  runtime dependency.
- Added a non-`rum`, non-manual UI polish backlog and completed the automated
  polish items in that scope. Long menu dropdowns now keep the selected entry
  visible on short terminals, render overflow indicators, and use the same
  scrolled range for mouse hit testing. Scrollable modal lists such as Open,
  Save As, and Switch Buffer expose above/below overflow state to the renderer.
  Ambiguous command prompt completions now appear in the prompt overlay, and
  the renderer has additional automated coverage for menu/dialog overflow,
  ASCII fallback chrome, viewport markers, and modal list hit geometry.
- Audited the current unsafe boundary and recorded the project policy:
  `dun`'s own crates remain zero-real-unsafe and use `#![forbid(unsafe_code)]`
  at crate/test-support entry points. Added
  `docs/code-organization-guidelines.md`, adapted from the neighboring `rum`
  project's file-slimming rules, with file-size thresholds, assess-on-touch
  split policy, safe Rust requirements, current oversized-file hotspots, and
  preferred future module boundaries for `dun-cli`, `dun-ui`, `dun-core`,
  `dun-config`, and `dun-term`.
- Added `docs/file-splitting-plan.md`, a staged migration plan for the current
  oversized files. The plan starts with `dun-cli` test extraction, then moves
  pure model/helper code before app-state method groups and process I/O
  boundaries, followed by `dun-ui`, `dun-config`, `dun-core::buffer`,
  workspace, and theme splits. Each stage requires `cargo fmt --all`,
  `cargo test --workspace`, and `git diff --check`.
- Started the file-splitting line by completing Stage 1 for `dun-cli` tests.
  The old inline `#[cfg(test)] mod tests` in `crates/dun-cli/src/main.rs` is
  now `mod tests;`, with 200 tests split under `crates/dun-cli/src/tests/` by
  behavior family plus shared helpers in `tests/support.rs`. `main.rs` dropped
  from 558,750 bytes / 16,386 lines to 362,654 bytes / 10,802 lines after the
  move. `cargo fmt --all`, `cargo test --workspace`, and `git diff --check`
  passed.
- Completed Stage 2 of the `dun-cli` split by moving pure state/model types
  out of `main.rs`. New internal modules now hold `AppState` fields,
  `BufferState`, search state, status history entries, dialog prompt/file
  dialog/buffer-switcher/confirmation state, command-output result models, and
  runtime terminal actions. `main.rs` dropped to 309,368 bytes / 9,075 lines
  after this stage. `cargo fmt --all`, `cargo test --workspace`, and
  `git diff --check` passed.
- Completed Stage 3 of the `dun-cli` split by moving pure helper functions
  out of `main.rs`. Status, environment, buffer metadata, outline/search
  results, config-diagnostics, and command-output section helpers now live
  under `crates/dun-cli/src/help/`; file-dialog path/listing helpers and
  display-width wrapping helpers now live under `crates/dun-cli/src/files/`.
  AppState-mutating behavior remains in `main.rs` for the next method-group
  stage. `main.rs` dropped to 283,680 bytes / 8,188 lines after this stage.
  `cargo fmt --all`, `cargo test --workspace`, and `git diff --check` passed.
- Started Stage 4 of the `dun-cli` split by extracting AppState method groups.
  Window behavior now lives in `app/windows.rs`; editing, selection,
  clipboard, bookmarks, paging, and text-input behavior lives in
  `app/editing.rs`; focused-buffer accessors, status recording, and buffer
  view-context calculation live in `app/view_state.rs`. `main.rs` dropped to
  249,993 bytes / 7,245 lines after this batch. `cargo fmt --all` and
  `cargo test --workspace` passed after each method-group move.
- Started Stage 5 of the `dun-cli` split by extracting terminal process I/O
  boundaries. `terminal/lifecycle.rs` now owns raw mode, alternate-screen,
  bracketed paste, mouse capture, suspend/resume, and drop-time terminal
  restoration. `terminal/sgr.rs` now owns `TerminalColorRewrite`,
  `TerminalWriter`, and 16-color SGR rewriting. `main.rs` dropped to 240,826
  bytes / 6,919 lines after this batch. `cargo fmt --all` and
  `cargo test --workspace` passed.
- Completed the planned Stage 5 process I/O boundary extraction. Terminal input
  dispatch and crossterm key/text conversion now live in
  `terminal/input.rs`; shell escape, one-shot command execution, bounded
  stdout/stderr capture, and command-run status formatting live in
  `terminal/shell.rs`; stable file snapshots, UTF-8/fallback loading, save
  snapshot checks, path diagnostics, same-directory atomic writes, and
  atomic-temp cleanup live under `files/{snapshot,open,save,atomic}.rs`.
  `main.rs` dropped to 213,794 bytes / 6,030 lines after this batch.
  `cargo fmt --all` and `cargo test --workspace` passed.
- Continued Stage 4 by moving AppState mouse interaction behavior into
  `app/mouse.rs` and central command dispatch into `app/commands.rs`.
  `app/mouse.rs` now owns mouse-driven workspace focus, selection, split
  dragging, scrollbar dragging, menu clicks, and file-dialog hit behavior.
  `app/commands.rs` now owns editor/app/file command dispatch, configured key
  sequence dispatch, auxiliary-window key dispatch, runtime-action requests,
  and config reload application. `main.rs` dropped to 190,121 bytes / 5,386
  lines after this batch. `cargo fmt --all` and `cargo test --workspace`
  passed.
- Continued Stage 4 by extracting the remaining large AppState method groups.
  `app/file_io.rs` owns file open/save/save-as/reload/new-buffer mutation
  paths; `app/helper_panes.rs` owns Help, Config Diagnostics, Status History,
  Outline, and read-only helper-pane refresh/jump behavior; `app/command_output.rs`
  owns command-output pane behavior and save-dialog integration;
  `app/search_replace.rs` owns find/replace/search-results/go-to-line behavior;
  `app/prompt_dialogs.rs`, `app/file_dialogs.rs`, and
  `app/buffer_switcher.rs` own modal prompt, file-dialog/confirmation, and
  buffer-switcher state machines. `main.rs` dropped to 86,816 bytes / 2,520
  lines after this batch, and the new files stay below the file-size debt
  threshold. `cargo fmt --all` and `cargo test --workspace` passed after the
  large moves.
- Completed Stage 4 by moving the final AppState method groups out of
  `main.rs`. AppState construction now lives in `app/bootstrap.rs`; buffer view
  assembly and workspace-area synchronization live in `app/frame.rs`; menu
  state and menu dispatch live in `app/menus.rs`; command-line command runners
  live in `app/command_line.rs`; focused path/status display helpers live in
  `app/status_view.rs`. `crates/dun-cli/src/main.rs` no longer contains an
  `impl AppState` block and dropped to 61,522 bytes / 1,835 lines after this
  batch. `cargo fmt --all` and `cargo test --workspace` passed.
- Completed Stage 6 of the `dun-cli` split by reducing `main.rs` to process
  entry, top-level CLI dispatch, and TUI startup orchestration. CLI argument
  parsing, startup config loading, command-line prompt parsing/completion,
  help content, read-only helper buffers, command-output formatting, terminal
  profile detection, the runtime event loop, OSC52 clipboard formatting, and
  shared small constants/helpers now live in named modules. `main.rs` dropped
  to 7,211 bytes / 180 lines after this stage. `cargo fmt --all` and
  `cargo test --workspace` passed.
- Started Stage 7 of the `dun-ui` split by moving the inline unit tests out of
  `crates/dun-ui/src/lib.rs`. The 50 tests now live under
  `crates/dun-ui/src/tests/` as `model`, `hit`, `rendering`, and `fallback`
  behavior modules with shared helpers in `support.rs`. `dun-ui/src/lib.rs`
  dropped to 98,060 bytes / 3,003 lines after this test-only move.
  `cargo fmt --all` and `cargo test -p dun-ui` passed.
- Continued Stage 7 by moving backend-neutral `dun-ui` data types into
  `model.rs`, text width/truncation/wrapping/visible-whitespace helpers into
  `text.rs`, and workspace/menu/overlay hit-testing methods into `hit.rs`.
  `crates/dun-ui/src/lib.rs` dropped to 79,198 bytes / 2,330 lines after this
  batch. `cargo fmt --all`, `cargo test -p dun-ui`, `cargo test --workspace`,
  and `git diff --check` passed.
- Completed Stage 7.5 by moving ratatui rendering into
  `crates/dun-ui/src/render/`, split by visual layer: chrome/style conversion,
  menu/dropdown rendering and menu geometry, modal overlay rendering and
  layout, status bar rendering, and window/body/chrome rendering. `lib.rs`
  now keeps `UiShell`, frame model construction, menu/status model
  construction, and facade exports; it dropped to 48,749 bytes / 1,307 lines
  after this batch. `cargo fmt --all` and `cargo test -p dun-ui` passed.
- Started Stage 8 of the `dun-config` split. Inline unit tests moved into
  `crates/dun-config/src/tests/` as config, keys, parser, and validation
  behavior modules with shared helpers in `support.rs`. The typed config model
  moved into `config.rs`, and file/line display limits moved into `limits.rs`.
  `crates/dun-config/src/lib.rs` dropped to 61,590 bytes / 1,524 lines after
  this batch. `cargo fmt --all` and `cargo test -p dun-config` passed.
- Continued Stage 8 by moving `dun-config` key parsing, key sequence display,
  editor keymap defaults, and file-dialog modal keymap/action mapping into
  `crates/dun-config/src/keys/`. `lib.rs` remains the facade plus parser,
  command id mapping, default config text, and validation error text, and
  dropped to 34,597 bytes / 768 lines after this batch. `cargo fmt --all` and
  `cargo test -p dun-config` passed.
- Completed the remaining Stage 8 `dun-config` split. Command id mapping now
  lives in `commands.rs`, default config text in `defaults.rs`, line-based
  config parsing in `parser.rs`, and validation/error text conversion in
  `validation.rs`. `crates/dun-config/src/lib.rs` is now a 28-line facade, and
  the largest new file is `parser.rs` at 10,380 bytes / 322 lines.
  `cargo fmt --all` and `cargo test -p dun-config` passed.
- Started Stage 9 of the `dun-core::buffer` split. The old
  `crates/dun-core/src/buffer.rs` is now `buffer/mod.rs`; inline buffer tests
  moved into behavior modules under `buffer/tests/`; public buffer model and
  storage types moved into `buffer/model.rs`; search and replace-all behavior
  moved into `buffer/search.rs`. The main buffer module dropped from 74,231
  bytes / 2,413 lines to 44,866 bytes / 1,447 lines after this batch.
  `cargo fmt --all` and `cargo test -p dun-core` passed after each sub-step.
- Completed the remaining Stage 9 `dun-core::buffer` split. Cursor and word
  movement now live in `buffer/cursor.rs`; selection commands in
  `buffer/selection.rs`; insert/delete/range replacement in `buffer/edit.rs`;
  line-oriented editing in `buffer/line_ops.rs`; undo/redo and merge logic in
  `buffer/undo.rs`. `buffer/mod.rs` is now 170 lines and keeps facade exports,
  construction, text shape accessors, validation, dirty-state fingerprinting,
  and revision tracking. `cargo fmt --all` and `cargo test -p dun-core`
  passed.
- Completed Stage 10 for the remaining split-plan files. `dun-core::workspace`
  now has separate model, split/close, focus, split-hit, resize, facade, and
  test modules; `dun-term::theme` now has separate color, style, palette,
  built-in theme constructor, facade, and test modules. The old
  `workspace.rs` and `theme.rs` files are gone; public exports remain routed
  through the same crate facades. `cargo fmt --all`, `cargo test -p dun-core`,
  `cargo test -p dun-term`, `cargo test --workspace`, and `git diff --check`
  passed during the split.
- Finished the remaining `dun-ui` facade split. `UiShell` moved to
  `shell.rs`; workspace frame construction and buffer geometry moved into
  `frame/{mod,cursor,gutter,highlight,menu,scroll,status,text}.rs`; `lib.rs`
  is now only module declarations and facade re-exports. `crates/dun-ui/src/lib.rs`
  dropped from 48,749 bytes / 1,307 lines to 1,043 bytes / 33 lines, and the
  largest new implementation file is `frame/menu.rs` at 12,932 bytes / 270
  lines. `cargo fmt --all`, `cargo test -p dun-ui`, `cargo test --workspace`,
  and `git diff --check` passed.
- Started the automated tmux real-terminal test line. Added
  `crates/dun-cli/tests/support/tmux.rs` with fixed-size tmux session startup,
  visible-pane capture, SGR-preserving capture, stable-screen polling, key
  injection, and cleanup. Added `tests/tmux_grid.rs` covering 80x24 baseline
  chrome, 100x30 fixed-pane dimensions, command-prompt-triggered tiled split
  rendering, and ASCII/16 fallback chrome with no 256-color SGR. The tests skip
  cleanly when tmux is unavailable or cannot create its socket. `cargo fmt --all`
  and `cargo test -p dun-cli --test tmux_grid` passed.
- Added the first normalized tmux cell-grid parser. `TmuxGrid` now records
  fixed width/height, visible cells, basic SGR attributes, SGR foreground and
  background colors, and tmux-reported cursor coordinates. The tmux grid tests
  now assert the initial focused editor cursor at `(3, 2)` and menu reverse/bold
  attributes through parsed cells. `cargo fmt --all` and
  `cargo test -p dun-cli --test tmux_grid` passed.
- Extracted the normalized terminal grid parser into shared
  `crates/dun-cli/tests/support/terminal_grid.rs`. PTY and tmux tests now use
  the same `TerminalGrid` model; the parser covers visible cells, basic SGR
  attributes, ANSI/indexed/RGB colors, clear-screen/clear-line, and common raw
  CSI cursor movement. Added pure parser tests plus a PTY raw-output grid
  assertion for opened UTF-8 files. `cargo fmt --all`,
  `cargo test -p dun-cli --test terminal_grid`,
  `cargo test -p dun-cli --test tmux_grid`, and the focused PTY grid test
  passed.
- Added normalized-grid assertion helpers:
  `assert_text_at`, `assert_line_contains`, `find_border_box`, and
  `find_border_boxes`. Border-box discovery recognizes both Microsoft
  Edit-style Unicode single-line boxes and ASCII fallback boxes, including
  tiled split panes. The tmux tests now assert single-pane and split-pane
  rectangles through parsed cells instead of only plain captured text.
  `cargo fmt --all`, `cargo test -p dun-cli --test terminal_grid`,
  `cargo test -p dun-cli --test tmux_grid`, and the focused PTY grid test
  passed.
- Added the first Microsoft Edit differential baseline. The tmux harness can
  now start an arbitrary executable with the same fixed dimensions and terminal
  environment used for `dun`. `tests/msedit_diff.rs` starts `dun` and `edit`
  against the same plain UTF-8 file, projects each normalized grid to
  editor-body text plus body-relative cursor position, compares the initial
  open state and a shared `Down`/`Right` cursor movement, and skips cleanly when
  `edit` is not on `PATH`. Failure output includes side-by-side projected text
  and cursor differences. The test-plan docs now state that mouse remains
  PTY/event-level coverage, pixel screenshots are optional manual visual
  regression only, and selection/color projections are post-baseline extensions
  added only for concrete diff cases or regression risks.
- Closed out the tmux-backed real-terminal baseline as a completed phase in
  `TODO.md`, leaving only post-baseline terminal test extensions active. The
  Microsoft Edit differential test now runs independent fixed-size sessions for
  `Right Right`, `End`, `Down Up`, and `Down Right` cursor-motion cases, each
  comparing initial and post-key projected body text plus relative cursor
  position. The `TerminalGrid` parser tests now cover wide characters, tabs,
  CRLF, selective SGR reset, and save/restore cursor sequences. The terminal
  testing docs now record CI/VM availability: `expect` and `tmux` are
  recommended automated-test dependencies, `edit` is an optional differential
  dependency, missing tools clean-skip, and no GUI terminal is required.
- Polished the buffer switcher interaction. The switcher now handles `Home` and
  `End` to jump to the first and last open buffers, and its overlay footer
  advertises the new shortcuts alongside the existing Up/Down and PageUp/PageDown
  movement keys. Added regression coverage that creates many buffers, uses
  Home/End in the switcher, and verifies focus moves to the first and last
  buffers.
- Polished focused reload/search/config/release UX. The command prompt help now
  advertises `reloadfile`, focused file status has regression coverage for
  external disk changes, Outline and Search Results panes handle `Home`/`End`
  for first/last entry selection, `--dump-config` groups defaults into readable
  sections, and `docs/release-smoke-checklist.md` defines the bounded automated
  release smoke gate.
- Established the v0.1 release-size governance rule: `target/release/dun` must
  be no larger than 1 MiB on both audited macOS and Debian builds. Added a
  checked-in size-budget release profile and `docs/feature-budget.md`, which
  classifies implemented runtime features as required or optional and gives a
  concrete trim order for optional features when the size gate fails.
- Rebuilt the checked-in release profile on macOS and the Debian VM. The
  measured `target/release/dun` sizes are 863,664 bytes on macOS x86_64 and
  1,038,936 bytes on Debian x86_64, so both pass the 1 MiB gate. Debian has
  only 9,640 bytes of margin, so new runtime work is budget-sensitive.
- Ran the v0.1 release smoke checklist on 2026-07-09. Local macOS
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test --workspace` passed. The release binary built with
  `cargo build --release --locked -p dun-cli`; `--version`, `--help`, and
  `--dump-config` ran successfully, with the macOS binary measuring 863,664
  bytes. The Debian VM clean archive build used Debian system `rustc 1.85.0`
  and `cargo 1.85.0`, used the same locked release command, produced a
  1,038,936 byte binary, printed `dun 0.1.0`, and emitted 19 help lines plus
  121 default-config lines. Focused smoke
  subsets also passed: `cargo test -p dun-cli file_io` reported 32 passed and
  2 ignored performance tests, while `pty_smoke`, `terminal_grid`, and
  `tmux_grid` reported 7, 6, and 5 passed tests respectively. Removed the
  active server-console/KVM release item; v0.1 delivery should state only the
  automated, SSH, multiplexer, locale, color, size, and VM terminal coverage
  actually recorded.
- Decided the plugin system is protocol-first instead of `rum`-first. The
  required `dun` runtime feature is a small host-neutral framed-stdio plugin
  client with role/policy validation, bounded snapshots, timeout/cancel/crash
  handling, stale revision rejection, and fixture-host tests. The client must
  fit inside the 1 MiB macOS/Debian release budget; if it does not, optional
  editor features are trimmed before cutting the protocol client. `rum` remains
  a future optional pure-sandbox host, while Python/Rust/script fixture hosts
  are user-trusted unless separately sandboxed.
- Repositioned `dun` as a standalone lightweight TUI editor and started the
  v0.1 slimming stage (2026-07-10). Added CLAUDE.md session guidance,
  documented Debian VM access with tracked `vm-test/vm-run`/`vm-sync` helper
  scripts, cleaned two stale checkouts off the VM after verifying their
  uncommitted diffs had already landed, and created the feature triage
  inventory (docs/feature-triage.md) with A/B/C/D decision rules over 48
  separately-removable units.
- Measured the plugin-client size spike (branch `spike/plugin-client-size`,
  `c7f042c`): a working framed-stdio + hand-rolled JSON protocol client with
  envelope/role/policy model, span validation, timeout/cancel/crash handling,
  and a fixture-host end-to-end path costs +62,032 bytes on macOS (925,696
  total) and +77,824 bytes on Debian (1,116,760 total, 68,184 bytes over the
  1 MiB gate). Derived trim target: free ~147-187 KiB on Debian before the
  real client lands. cargo-bloat attribution shows ~90+ KiB of std is
  unremovable panic-backtrace machinery on stable, so trims must come from
  feature code.
