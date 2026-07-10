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
executes the selected item, and `Esc` closes it. Long dropdowns scroll on
short terminals so the selected entry remains visible, with overflow
indicators and matching mouse hit testing.
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
Opened file buffers retain a verified metadata snapshot. Normal Save refuses
to overwrite when the path has changed or disappeared on disk; Reload refreshes
the focused file buffer from disk when the user explicitly chooses to discard
the in-memory state.
Ignored release-mode performance baselines cover large-file open/search/scroll
and visible-window rendering.
Open, Save, and Save As failures include the relevant path and normalized
diagnostics for common cases such as missing files, directories, missing parent
directories, permission denial, and read-only destinations.
Open and Save As now use larger modal file dialogs with a path input, directory
match list, Up/Down and PageUp/PageDown selection, directory navigation, Tab
path completion, a parent-directory entry, hidden-file filtering with `Ctrl+H`
toggle, Home/End/Left/Right/Delete path editing, empty/no-match diagnostics,
visible list overflow indicators, and mouse-wheel list scrolling when mouse
support is enabled. File-dialog modal keys have their own typed config
bindings. Open/Save As errors remain inside the dialog for correction,
successful dialogs remember the last directory for the session, and Save As
asks for a second Enter before overwriting an existing file.
Lightweight modal prompts remain available for Find, Replace, and Go To Line
entry, with Left/Right/Home/End/Delete/Backspace editing at UTF-8 character
boundaries. Find previews matches while typing, supports next/previous
navigation and selected match highlighting, accepts `/i`, `/w`, and `/iw`
prefixes for ignore-case and whole-word search, and the focused status area
reports the active match count. Interactive Replace uses the same search
prefixes, previews the query, then uses a confirmation modal for replace, skip,
replace-all, or cancel. The command prompt still offers direct
`replace QUERY TEXT` and `replace all QUERY TEXT`, and Go To Line moves the
cursor by 1-based line number.
The editor surface includes a line-number gutter plus focused buffer name,
dirty/read-only markers, and status fields for position, total lines,
selection, active search count, visible scroll range, horizontal scroll offset,
line ending, file-text encoding, terminal profile, and focused window index.
It also includes a lightweight buffer switcher for open tiled buffers, line
commands for copy/delete/move/indent/outdent/trim, line bookmarks shown in the
gutter, a display-layer word-wrap toggle, and visible-whitespace markers with
ASCII fallback. In word-wrap mode, scrolling, cursor visibility, selection
highlights, search-match highlights, gutters, and scrollbars operate on
wrapped visual rows instead of only whole logical lines.
Keyboard selection supports `Shift+Arrow` and `Shift+Home/End` when those
strokes are not consumed by the configured keymap, and `Ctrl+L` selects the
current line, giving Cut/Copy a pure keyboard path without requiring mouse
support. PageUp/PageDown move by the visible pane height, using wrapped visual
rows when word-wrap is active, and `Ctrl+Home/End` jump to document
start/end. `Ctrl+Left/Right`
move by UTF-8-safe word boundaries, `Ctrl+Backspace/Delete` delete by word, and
`Ctrl+Shift+Left/Right` extends selection by word when the terminal delivers
those modifiers. `Shift+PageUp` and `Shift+PageDown` extend selection by the
visible pane height.
Undo groups continuous ordinary character typing and continuous same-direction
Backspace/Delete runs into transactions, while cursor movement, selection
changes, replace, newline, paste-like bulk insertion, undo, and redo keep clear
transaction boundaries. Undo and redo commands report visible status for
successful actions and empty stacks.
On narrow panes, the gutter is dropped before it consumes the editable body,
pane titles/status fields are clipped by terminal display width, and long lines
scroll horizontally to keep the focused cursor visible. `edit.scroll_left` and
`edit.scroll_right` provide explicit viewport movement, and long buffers show a
lightweight right-border scrollbar thumb. Horizontally clipped lines show
small edge indicators at the body boundary.
Buffer text, pane titles, and status fields are sanitized before rendering so
file content and file names cannot emit terminal control sequences.
By default, `F1` opens a read-only Help window with the active configured key
reference, and `F2` opens a read-only Status History window with recent status
and error messages. `F5` reloads the active configuration without restarting
the editor, `F6` opens Config Diagnostics with source, terminal, clipboard,
limit, keymap, and important-unbound-command summaries. The command prompt can
jump directly to diagnostics sections such as `config keymap` or
`diagnostics limits`. `reloadfile` explicitly reloads the focused file buffer
from disk after any dirty-buffer confirmation. After a Find, `results` opens
a read-only match list and `results N` jumps back to that match. In Search
Results, `n`/`p` move between listed entries, `Home`/`End` jump to the first
or last entry, and `Enter` jumps back to the selected source location. The
built-in Outline pane was removed in the 2026-07 slimming stage; document
structure listing is planned to return as a plugin role.
`Ctrl+P` opens a command prompt for actions such as `help`, `config`,
`reload-config`, `theme`, `open`, `save`, `quit`, and full command ids such as
`window.split_horizontal`; Tab completes built-in command families, cycles
ambiguous candidates, shows ambiguous completion candidates in the prompt
overlay, and completes path arguments for commands such as `open` and
`save-as`.
`Ctrl+W,S` performs a Turbo Pascal-style shell escape: `dun` suspends the TUI,
restores the normal terminal, runs the user's shell, then resumes and redraws
after the shell exits. `Ctrl+W,O` opens a Run Command prompt for one-shot
non-interactive commands; stdout and stderr are captured with bounded memory,
decoded through the same safe display path as files, and shown in a read-only
Command Output window with per-stream byte counts and truncation status. The
Run Command prompt keeps its own bounded history, separate from the editor
command prompt. The advanced Command Output command family (in-pane search,
section/body jumps, stdout/stderr-only derived panes, and dedicated
copy/save/clear commands) was removed in the 2026-07 slimming stage; the pane
remains a normal read-only buffer, so ordinary selection, copy, and Find work
inside it.
Tiling defaults use `Ctrl+W,H`/`Ctrl+W,V` to split, `Ctrl+W,Arrow` to move
focus, and `Ctrl+W,Shift+Arrow` to resize. `Alt+Arrow` and
`Alt+Shift+Arrow` remain compatibility aliases for terminals that deliver
Option/Meta keys, but the primary path does not depend on macOS Command, Fn,
or Option-key terminal settings.
The command prompt keeps a bounded in-memory history navigated with Up/Down.
Dirty buffers are protected by a status-line confirmation before quit, new,
open, or close would discard changes.
Cut, Copy, and Paste are implemented with a process-local internal clipboard:
they operate on the active selection and still use the normal buffer edit path
so read-only buffers reject mutation. External copy is available only through
the explicit `edit.copy_external` command and opt-in `clipboard.osc52.enabled`
configuration; it emits a bounded OSC 52 clipboard sequence and always keeps
the same text in the internal clipboard as fallback.
Terminal bracketed paste is enabled during the TUI session and restored on
exit. Paste text is treated as untrusted input: editor paste goes through the
normal buffer insertion path, prompt and file-dialog paste is kept single-line
and never auto-submits, and confirmation prompts ignore paste. Right-click
paste only waits for the terminal to deliver bracketed paste data; `dun` does
not call external clipboard commands, query the terminal clipboard, or perform
OSC 52 paste.
Mouse support is optional and disabled by default; when enabled in config,
left-clicks can focus tiled windows, place the cursor in an editor body, drag
text selections including edge scrolling, drag split borders, open top-menu
dropdowns, and click
submenu commands. Mouse wheel events scroll editor panes and file dialog lists,
right-border scrollbar clicks or drags scroll long editor buffers, and
terminals that deliver horizontal wheel events can scroll the focused editor
viewport left/right.
File dialog list clicks enter directories, open selected files from Open, and
update the Save As path input without immediately saving.
The external SSH and low-capability terminal release matrix is documented in
[docs/terminal-compatibility-checks.md](./docs/terminal-compatibility-checks.md);
the local PTY harness covers common terminal profiles, small VT100-style
fallback, terminal escape payloads, invalid-byte fallback files, and
event-level coverage for common modified keys after crossterm has parsed them.
External host results still need to be recorded before a tagged release.
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
- The final v0.1 release executable must be no larger than 1 MiB on both
  audited macOS and Debian builds; see
  [docs/feature-budget.md](./docs/feature-budget.md).
- The plugin system is protocol-first. The `dun` protocol client is required
  core infrastructure, while `rum` is a future optional pure-sandbox host.

## Product Goal

The first product line should make the common remote editing loop fast:

1. Open a text file or start an untitled buffer.
2. Navigate and edit safely over SSH.
3. Search and replace.
4. Split the workspace when comparing files.
5. Switch between open buffers and reload changed files deliberately.
6. Temporarily drop to a shell or capture one-shot command output.
7. Save with predictable host-owned file I/O.

The long-term plugin story is still aimed at operational filters: operators
should be able to write concise rule-style filters for custom logs. The
protocol can be tested with trusted external fixture hosts first; the strong
third-party safety claim waits for a pure-sandbox host such as future `rum`.

## Non-Goals

For the initial line, `dun` is not:

- a GUI editor;
- a full IDE;
- a native dynamic-library plugin host;
- a shell automation environment;
- an embedded terminal emulator;
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

Plugin protocol support is planned before `rum` integration:

- `dun-plugin-api` or an equivalent crate/module: host-neutral protocol
  messages, roles, policies, input snapshots, output intents, and validation.
- `dun-plugin-rum`: future pure `rum` host adapter, optional and separately
  sized.

The default editor line should stay small and pure Rust. Current size audit
results put size-oriented `dun` binaries around 0.8-1.0 MiB on macOS/Debian,
with empty-startup RSS in the low-megabyte range on the audited macOS and
Debian hosts, while `rum` is currently treated as an approximately 6 MiB
runtime dependency. That makes `rum` valuable but too large for the default
editor executable. The required plugin protocol client must fit inside the
1 MiB `dun` budget; if it causes a budget failure, optional editor features are
trimmed before the protocol client. External hosts, including future
`dun-rum-host`, are separate optional artifacts.

## Plugin Boundary

The safety boundary is intentionally host-owned.

Plugins speak the Dun Plugin Protocol over framed stdio. They receive bounded
input snapshots from `dun` and return structured data or command intents.
`dun` validates those results against the plugin role and policy, then performs
any actual editor action itself.

Protocol compatibility is not a sandbox. A future pure `rum` host can provide
the strong untrusted-third-party safety claim because it should have no file,
process, network, terminal, environment, or editor-state side effects. Python,
shell, Rust, or other external hosts are useful for tests and local workflows,
but they are user-trusted unless an OS sandbox is added outside `dun`.

Invariants:

- untrusted plugins do not perform file I/O;
- untrusted plugins do not perform process or network I/O;
- untrusted plugins do not directly mutate editor state;
- untrusted plugins do not directly write terminal output;
- file operations are always performed by `dun` core code;
- plugin output is intent, not authority.

See [AUDIT.md](./AUDIT.md) and
[docs/plugin-protocol.md](./docs/plugin-protocol.md) for the security model
and protocol boundary.

## Terminal Compatibility

Rendering must go through an explicit terminal profile:

- encoding: UTF-8 or ASCII;
- colors: 256-color, 16-color, mono, and later truecolor if useful;
- glyphs: Unicode or ASCII;
- capabilities: conservative assumptions for low-end `TERM` values.

The UI must not assume Nerd Fonts, truecolor, enabled mouse support, or Unicode
line drawing. For 16-color profiles, the terminal output path rewrites
crossterm palette SGR sequences into legacy 16-color SGR forms instead of
emitting 256-color-style `38;5;n` or `48;5;n` controls.

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
- [docs/release-size-audit.md](./docs/release-size-audit.md): lightweight
  release binary size baseline for macOS and Debian builds.
- [docs/debian-vm.md](./docs/debian-vm.md): Debian measurement VM connection
  details and working conventions.
- [docs/feature-budget.md](./docs/feature-budget.md): hard v0.1 runtime size
  gate, required feature set, and optional feature trim order.
- [docs/feature-triage.md](./docs/feature-triage.md): working inventory and
  A/B/C/D classification for the v0.1 slimming stage.
- [docs/plugin-protocol.md](./docs/plugin-protocol.md): host-neutral plugin
  protocol, trust classes, role policy, and completion criteria.
- [docs/runtime-resource-audit.md](./docs/runtime-resource-audit.md):
  lightweight startup and RSS baselines for macOS and Debian builds.
- [docs/release-smoke-checklist.md](./docs/release-smoke-checklist.md):
  bounded automated and release-signoff checks for release candidates.
- [docs/dependency-audit.md](./docs/dependency-audit.md): dependency shape,
  feature policy, and repeat checklist for keeping the default build small.
- [docs/code-organization-guidelines.md](./docs/code-organization-guidelines.md):
  safe Rust policy, file-size thresholds, module split rules, and directory
  organization guidance.
- [docs/file-splitting-plan.md](./docs/file-splitting-plan.md): staged,
  test-gated plan for splitting the current oversized Rust source files.
- [docs/configuration.md](./docs/configuration.md): current Rust-owned config
  file loader and supported keys.
- [docs/crate-map.md](./docs/crate-map.md): current Rust workspace crate
  boundaries and dependency rules.
