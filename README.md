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
The top menu is grouped into Microsoft Edit-style File/Edit/View/Help menus,
with dropdown entries backed by the same typed command model as keybindings.
When the active keymap does not consume them, `Alt+F`, `Alt+E`, `Alt+V`, and
`Alt+H` open those menus; arrow keys move through an open menu, `Enter`
executes the selected item, and `Esc` closes it.
The default `msedit` theme now follows the local Microsoft Edit screenshots
more closely: blue menu/status chrome, green active top-menu labels, gray
dropdown/modal panels, a compact bracket-style status bar, a gutter separator,
and a muted current-line highlight.
It can open valid UTF-8 file paths supplied on the command line and save the
focused buffer back to that path through a same-directory temp file and atomic
rename. Stale atomic-save temp files are cleaned up, while newer recovery
candidates are preserved and reported. Invalid UTF-8 files open as read-only
escaped fallback buffers instead of being decoded lossy. Opened buffers track a
file-text encoding state: UTF-8 files are editable and save-safe, while
non-UTF-8 byte streams are shown as escaped bytes, marked read-only, and
blocked from Save/Save As.
Editable file loading enforces the configured soft limit before reading large
files into memory; the default editable limit is 16 MiB. If a file changes,
disappears, or is replaced while being read, Open rejects the unstable snapshot
and asks the user to retry.
Ignored release-mode performance baselines cover large-file open/search/scroll
and visible-window rendering.
Open, Save, and Save As failures include the relevant path and normalized
diagnostics for common cases such as missing files, directories, missing parent
directories, permission denial, and read-only destinations.
Open and Save As now use larger modal file dialogs with a path input, directory
match list, Up/Down and PageUp/PageDown selection, directory navigation, Tab
path completion, a parent-directory entry, hidden-file filtering with `Ctrl+H`
toggle, Home/End/Left/Right/Delete path editing, empty/no-match diagnostics,
and mouse-wheel list scrolling when mouse support is enabled. File-dialog
modal keys have their own typed config bindings.
Lightweight modal prompts remain available for Find, Replace, and Go To Line
entry, with Left/Right/Home/End/Delete/Backspace editing at UTF-8 character
boundaries. Find now supports next/previous navigation and selected match
highlighting, Replace can replace the current or next match, and Go To Line
moves the cursor by 1-based line number.
The editor surface includes a line-number gutter plus focused buffer name,
dirty/read-only markers, and status fields for position, total lines,
selection, line ending, file-text encoding, terminal profile, and focused
window index.
On narrow panes, the gutter is dropped before it consumes the editable body,
and pane titles/status fields are clipped by terminal display width.
Buffer text, pane titles, and status fields are sanitized before rendering so
file content and file names cannot emit terminal control sequences.
By default, `F1` opens a read-only Help window with the active configured key
reference, and `F2` opens a read-only Status History window with recent status
and error messages. `F5` reloads the active configuration without restarting
the editor, `F6` opens Config Diagnostics, and `Ctrl+P` opens a command prompt
for actions such as `help`, `config`, `reload-config`, `theme`, `open`,
`save`, `quit`, and full command ids such as `window.split_horizontal`.
Tiling defaults use `Ctrl+W,H`/`Ctrl+W,V` to split, `Ctrl+W,Arrow` to move
focus, and `Ctrl+W,Shift+Arrow` to resize. `Alt+Arrow` and
`Alt+Shift+Arrow` remain compatibility aliases for terminals that deliver
Option/Meta keys, but the primary path does not depend on macOS Command, Fn,
or Option-key terminal settings.
The command prompt keeps a bounded in-memory history navigated with Up/Down.
Dirty buffers are protected by a status-line confirmation before quit, new,
open, or close would discard changes.
Terminal bracketed paste is enabled during the TUI session and restored on
exit. Paste text is treated as untrusted input: editor paste goes through the
normal buffer insertion path, prompt and file-dialog paste is kept single-line
and never auto-submits, and confirmation prompts ignore paste. Right-click
paste only waits for the terminal to deliver bracketed paste data; `dun` does
not call external clipboard commands or emit OSC 52 clipboard writes.
Mouse support is optional and disabled by default; when enabled in config,
left-clicks can focus tiled windows, place the cursor in an editor body, drag
text selections, drag split borders, open top-menu dropdowns, and click
submenu commands. File dialog list clicks enter directories, open selected
files from Open, and update the Save As path input without immediately saving.
The external SSH and low-capability terminal release matrix is documented in
[docs/terminal-compatibility-checks.md](./docs/terminal-compatibility-checks.md);
the local PTY harness covers common terminal profiles, small VT100-style
fallback, terminal escape payloads, and invalid-byte fallback files. External
host results still need to be recorded before a tagged release.
Lightweight Microsoft Edit reference tests check the local `edit --help`
contract when available and statically scan `reference/msedit` source for menu,
status bar, color, and terminal setup reference markers.

## CLI Usage

```text
dun [OPTIONS] [--] [PATH]
```

Supported options are `--help`/`-h`, `--version`/`-V`, `--config PATH`,
`--dump-config`, and `--no-config`. `dun` exits with `0` for
success/help/version/default-config output, `1` for runtime or file I/O
errors, and `2` for command-line usage errors.

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

The UI must not assume Nerd Fonts, truecolor, enabled mouse support, or Unicode
line drawing.

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
- [docs/terminal-compatibility-checks.md](./docs/terminal-compatibility-checks.md):
  PTY smoke coverage and manual SSH terminal compatibility checklist.
- [docs/performance-baselines.md](./docs/performance-baselines.md): ignored
  large-file performance baseline tests and current local sample output.
- [docs/configuration.md](./docs/configuration.md): current Rust-owned config
  file loader and supported keys.
- [docs/crate-map.md](./docs/crate-map.md): current Rust workspace crate
  boundaries and dependency rules.
