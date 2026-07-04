# dun

`dun` is a planned terminal text editor for remote operations work: SSH into a
Linux or macOS host, inspect and edit text files, read large logs, filter
custom log formats, and keep working even on conservative terminal setups.

The intended UI is a `ratatui`-based TUI with a restrained, keyboard-first
style inspired by classic editor interfaces such as `msedit`.

## Status

This repository has a minimal Cargo/git baseline and is otherwise at the
planning and architecture stage. The editor core has not been implemented yet.

The current baseline decisions are:

- Rust `1.85` is the target toolchain.
- Linux and macOS terminals are the primary platforms.
- SSH and server-side troubleshooting are primary use cases.
- UTF-8 and 256 colors are the default rendering target.
- 16-color, low-capability, and ASCII-only fallback modes are required.
- A future plugin system will be designed around `rum`, but `rum` is not a
  dependency until its release API is stable enough to embed.

## Product Goal

`dun` should make the common operational loop fast:

1. Open or tail a service log.
2. Search, narrow, and filter records.
3. Use small, local custom filters for non-standard log formats.
4. Inspect related files.
5. Apply small edits safely when needed.

The long-term plugin story is especially aimed at operational filters:
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

## Architecture Sketch

The code should keep these boundaries clear even if the first implementation
starts as a single Cargo package:

- `dun-core`: buffers, cursors, selections, undo/redo, edits, search, and log
  view data structures.
- `dun-term`: terminal capability detection, color profile, glyph profile, and
  input normalization.
- `dun-ui`: `ratatui` views, menus, dialogs, status lines, command palette, and
  layout.
- `dun-command`: typed editor commands and command validation.
- `dun-config`: configuration loading and validation.
- `dun-plugin-api`: plugin roles, policies, input snapshots, and output intents.
- `dun-plugin-rum`: future `rum` runtime adapter, added only after `rum` has a
  stable release-facing host API.

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
