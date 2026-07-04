# dun

`dun` is a planned terminal text editor for remote operations work: SSH into a
Linux or macOS host, inspect and edit text files, and keep working even on
conservative terminal setups.

The intended UI is a `ratatui`-based TUI with a restrained, keyboard-first
style inspired by classic editor interfaces such as `msedit`.

## Status

This repository now has a Rust workspace baseline with the first pure editor
core, terminal profile/theme layer, typed configuration/keymap layer,
backend-neutral UI model, and a minimal runnable `ratatui` shell.
It can open one UTF-8 file path supplied on the command line and save the
focused buffer back to that path.
Interactive status-line prompts are available for Open, Save As, Find,
Replace, and Go To Line entry. Find now supports next/previous navigation and
selected match highlighting, Replace can replace the current or next match,
and Go To Line moves the cursor by 1-based line number.
The editor surface includes a line-number gutter plus focused buffer name,
dirty marker, and `Ln`/`Col` status.
`F1` opens a read-only Help window with the default key reference.
Dirty buffers are protected by a status-line confirmation before quit, new,
open, or close would discard changes.

The current baseline decisions are:

- Rust `1.85` is the target toolchain.
- Linux and macOS terminals are the primary platforms.
- SSH and server-side troubleshooting are primary use cases.
- UTF-8 and 256 colors are the default rendering target.
- 16-color, low-capability, and ASCII-only fallback modes are required.
- The first usable version is a Microsoft Edit-like text editor, not the full
  log/plugin product.
- A future plugin system will be designed around `rum`, but `rum` is not a
  dependency until its release API is stable enough to embed.

## Product Goal

The first product line should make the common remote editing loop fast:

1. Open a text file or start an untitled buffer.
2. Navigate and edit safely over SSH.
3. Search and replace.
4. Split the workspace when comparing files.
5. Save with predictable host-owned file I/O.

The long-term plugin story is still aimed at operational filters:
operators should be able to write concise rule-style filters for custom logs
without granting those filters filesystem, process, or network access.

## Non-Goals

For the initial line, `dun` is not:

- a GUI editor;
- a full IDE;
- a native dynamic-library plugin host;
- a shell automation environment;
- a terminal emulator;
- a replacement for `less`, `vim`, or `emacs` in every workflow.
- a log-analysis engine in the first usable version.

## Architecture Sketch

The current codebase is a Cargo workspace with these boundaries:

- `dun-core`: buffers, cursors, selections, undo/redo, edits, search, and
  tiled workspace state.
- `dun-term`: terminal capability detection, color profile, glyph profile, and
  theme selection.
- `dun-config`: typed configuration defaults, key sequences, command ids, and
  validation.
- `dun-ui`: `ratatui` views, menus, status lines, layout, sanitization, cursor
  placement, and lightweight tiled-window rendering.
- `dun-cli`: terminal lifecycle, event loop, key input routing, and command
  application.

Future plugin crates remain planned but intentionally absent until `rum` has a
stable release-facing host API:

- `dun-plugin-api`: plugin roles, policies, input snapshots, and output intents.
- `dun-plugin-rum`: pure `rum` runtime adapter.

## Plugin Boundary

The safety boundary is intentionally host-owned.

Future `rum` plugins in `dun` must run as pure computations only. They receive
bounded input snapshots from `dun` and return structured data or command
intents. `dun` validates those results against the plugin role and policy, then
performs any actual editor action itself.

Invariants:

- untrusted plugins do not perform file I/O;
- untrusted plugins do not perform process or network I/O;
- untrusted plugins do not directly mutate editor state;
- untrusted plugins do not directly write terminal output;
- file operations are always performed by `dun` core code;
- plugin output is intent, not authority.

See [AUDIT.md](./AUDIT.md) for the security model.

## Terminal Compatibility

Rendering must go through an explicit terminal profile:

- encoding: UTF-8 or ASCII;
- colors: 256-color, 16-color, mono, and later truecolor if useful;
- glyphs: Unicode or ASCII;
- capabilities: conservative assumptions for low-end `TERM` values.

The UI must not assume Nerd Fonts, truecolor, mouse support, or Unicode line
drawing.

## Development Documents

- [AGENTS.md](./AGENTS.md): instructions for coding agents and contributors.
- [PLAN.md](./PLAN.md): architecture and staged delivery plan.
- [TODO.md](./TODO.md): active and near-term task list.
- [PROGRESS.md](./PROGRESS.md): append-only progress log.
- [AUDIT.md](./AUDIT.md): security boundary and audit checklist.
- [docs/msedit-reference.md](./docs/msedit-reference.md): local notes from
  studying Microsoft Edit as a visual and interaction reference.
- [docs/window-management.md](./docs/window-management.md): lightweight
  tiling-window workspace model inspired by `tmux`/`i3`/`awesome`.
- [docs/editor-baseline.md](./docs/editor-baseline.md): first-version product
  decisions around encoding, large files, theme, keybindings, mouse, and log
  deferral.
- [docs/crate-map.md](./docs/crate-map.md): current Rust workspace crate
  boundaries and dependency rules.
