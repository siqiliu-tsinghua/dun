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
