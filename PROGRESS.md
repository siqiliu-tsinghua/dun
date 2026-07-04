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
