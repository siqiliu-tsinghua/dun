# Terminal Compatibility Checks

This document records the current terminal compatibility smoke plan for `dun`.
It is intentionally practical: the goal is to catch terminal lifecycle,
fallback rendering, and key-routing problems that are likely over SSH.

## Scope

The first editor baseline targets:

- local Linux and macOS terminals;
- SSH sessions into Linux or macOS hosts;
- `tmux`/`screen` style multiplexers;
- UTF-8 plus 256-color terminals by default;
- VT100-like, ASCII, and 16-color fallback modes;
- keyboard-first operation without requiring mouse support.

This document is the release terminal matrix source of truth. Automated PTY
tests cover local regressions, but tagged releases still need a manual pass on
real external SSH hosts because terminal emulators, multiplexers, locales, and
KVM devices can disagree about the same key names and glyph capabilities.

## Automated Baseline

The default regression gate is:

```text
cargo test -p dun-cli --test pty_smoke
```

That test runs the real `dun` binary in a pseudo-terminal through `expect(1)`.
Most cases use `80x24`; one low-capability case uses `40x12` to catch narrow
terminal regressions. The harness sends `Ctrl+Q` and checks startup/exit under:

- `TERM=xterm-256color`, UTF-8 locale;
- `TERM=screen-256color`, UTF-8 locale;
- `TERM=tmux-256color`, UTF-8 locale;
- `TERM=screen`, UTF-8 locale;
- `TERM=xterm-color`, `C` locale;
- `TERM=vt100`, `C` locale;
- `TERM=ansi`, `C` locale;
- `TERM=dumb`, `C` locale;
- `NO_COLOR=1` with a UTF-8 locale.

It also opens fixtures for:

- a normal UTF-8 file;
- a file containing terminal escape payloads, checking that the raw payload
  sequences are not emitted as file content;
- an invalid-byte file, checking that escaped bytes and `Escaped Bytes` status
  are visible;
- a `mouse.enabled = true` config, checking that mouse capture setup still
  starts and exits cleanly;
- a `40x12` VT100/C-locale startup case.

If `expect(1)` is not installed, the test exits successfully after printing a
skip message. Full workspace tests still run normally.

## Release Matrix

Before a tagged release, run the checklist below against each applicable case
and record the result in the release notes or a linked issue.

```text
ID              environment          command profile
AUTO-PTY        local PTY             cargo test -p dun-cli --test pty_smoke
LOCAL-UTF8      local terminal        TERM=xterm-256color, UTF-8 locale
LOCAL-MONO      local terminal        NO_COLOR=1, UTF-8 locale
LOCAL-VT100     local terminal        TERM=vt100, LANG=C, LC_CTYPE=C
SSH-UTF8        ssh direct            TERM=xterm-256color, UTF-8 locale
SSH-TMUX        ssh inside tmux       TERM=screen-256color, UTF-8 locale
SSH-SCREEN      ssh inside screen     TERM=screen, UTF-8 locale
SSH-VT100       ssh direct            TERM=vt100, LANG=C, LC_CTYPE=C
SSH-MONO        ssh direct            NO_COLOR=1, UTF-8 locale
SSH-SMALL       ssh direct            40x12 and 80x24 terminal sizes
KVM-ASCII       server console/KVM    ASCII or C locale, no mouse assumption
```

The current workspace has a project-local Debian VirtualBox VM available over
SSH for external terminal checks. Real server console/KVM results are still not
covered and must be gathered before a tagged release that claims that path.

## Latest External VM Run

```text
date: 2026-07-08
dun revision: d2c832f
host path: macOS host -> ssh -p 2222 -> Debian VirtualBox VM
remote OS: Debian trixie
rust: Debian rustc 1.85.0, cargo 1.85.0
terminal tools: expect 5.45.4, tmux 3.5a, screen 4.09.01
source: local git archive of d2c832f unpacked at /tmp/dun-matrix-d2c832f-DVaLv0

AUTO-PTY: pass
  cargo test -p dun-cli --test pty_smoke
  7 tests passed, including shell escape suspend/resume coverage.

VM-WORKSPACE: pass
  cargo test --workspace --quiet
  170 passed, 2 ignored in dun-cli unit coverage; all other workspace crates
  passed.

SSH-UTF8: pass
  TERM=xterm-256color LANG=C.UTF-8 LC_CTYPE=C.UTF-8
  Opened /tmp/dun-terminal-smoke.txt through ssh -tt, rendered the editor
  surface, accepted Ctrl+Q, and restored the terminal.

SSH-MONO: pass
  TERM=xterm-256color NO_COLOR=1 LANG=C.UTF-8 LC_CTYPE=C.UTF-8
  Rendered without color palette dependencies, accepted Ctrl+Q, and restored
  the terminal.

SSH-VT100: pass
  TERM=vt100 LANG=C LC_CTYPE=C
  ASCII glyph fallback worked, the editor exited cleanly, and the 16-color
  output used legacy SGR forms such as 37;44 and 93;44 instead of
  256-color-style 38;5;n or 48;5;n sequences.

SSH-SMALL: pass
  TERM=vt100 LANG=C LC_CTYPE=C, stty rows 12 cols 40
  Layout remained usable, clipped without overlap, and used legacy 16-color
  SGR output rather than 256-color-style SGR output.

SSH-TMUX: pass
  Outer TERM=xterm-256color, tmux 3.5a, app TERM=screen-256color
  Rendered in tmux, accepted Ctrl+Q, ended the tmux session, and restored SSH.

SSH-SCREEN: pass with environment warning
  Outer TERM=xterm-256color, screen 4.09.01, app TERM=screen
  Rendered and exited cleanly through screen.
  Debian's /etc/screenrc printed screen-owned warnings for unknown
  deflogin/login commands before the app surface.

SSH-ESCAPE-PAYLOAD: pass
  /tmp/dun-terminal-escape.txt rendered ESC, OSC, BEL, and SGR payload bytes
  as visible control notation rather than executing them.

SSH-INVALID-BYTES: pass
  /tmp/dun-terminal-invalid.bin opened as Escaped Bytes with \xFF visible and
  the fallback state shown in the status bar.

SSH-SHELL-ESCAPE: pass
  Ctrl+W,S suspended the alternate-screen TUI, ran
  /tmp/dun-shell-escape-d2c832f.sh through SHELL, printed
  dun-shell-escape-ssh on the normal terminal, then resumed and redrew the TUI.

SSH-RUN-COMMAND: pass
  Ctrl+W,O opened the Run Command prompt. Submitting "printf ssh-run" opened a
  read-only Command Output pane showing stdout, "Stdout: 7 bytes, complete",
  "Stderr: 0 bytes, complete", and "Truncated: no".

KVM-ASCII: not run
  No real server console or KVM path was available in this run.
```

## Result Record

Use this compact record for each manual run:

```text
date:
dun commit:
case ID:
local terminal:
remote OS:
ssh path:
multiplexer:
TERM:
LANG:
LC_CTYPE:
NO_COLOR:
stty size:
result: pass | fail | blocked
notes:
```

Failures should be converted into either a focused regression test or a TODO
entry with the exact terminal, locale, and key sequence that failed.

## Setup

Build the binary first:

```text
cargo build -p dun-cli
```

Create a small file for open/save checks:

```text
printf 'alpha\nbeta\n' > /tmp/dun-terminal-smoke.txt
printf 'safe\033]0;owned\a\n\033[31mred?\033[0m\n' > /tmp/dun-terminal-escape.txt
printf 'ok\377\n' > /tmp/dun-terminal-invalid.bin
```

Run either the local binary or an installed `dun`:

```text
./target/debug/dun
./target/debug/dun /tmp/dun-terminal-smoke.txt
./target/debug/dun /tmp/dun-terminal-escape.txt
./target/debug/dun /tmp/dun-terminal-invalid.bin
TERM=vt100 LC_CTYPE=C ./target/debug/dun /tmp/dun-terminal-smoke.txt
NO_COLOR=1 ./target/debug/dun /tmp/dun-terminal-smoke.txt
```

Over SSH, use the same commands after copying or installing the binary on the
remote host. For size-sensitive checks, resize the terminal manually or use the
multiplexer's resize controls before launching `dun`.

## Checklist

Startup and exit:

- The app enters the alternate screen without printing shell prompts into the
  editor surface.
- `Ctrl+Q` exits cleanly.
- The terminal returns to the shell with raw mode disabled.
- Cursor visibility and line discipline are normal after exit.

Visual baseline:

- The top menu bar and bottom status bar are visible.
- The default UTF-8/256-color case uses Microsoft Edit-like colors.
- UTF-8 profiles use single-line Unicode borders.
- VT100/ASCII fallback uses `+`, `-`, and `|` borders.
- Mono fallback remains readable without relying on color differences.
- Narrow terminals clip status/title text without overlap.
- No raw file controls or escape payloads execute in the terminal.
- Non-UTF-8 files open read-only with escaped bytes visible in the buffer and
  status fields.

Editing:

- Printable ASCII text inserts at the focused cursor.
- UTF-8 text displays correctly in UTF-8 profiles.
- Arrow keys, Home, End, Enter, Backspace, and Delete work.
- `Ctrl+S` saves a loaded file.
- `Ctrl+O` and `Ctrl+Shift+S` open modal file dialogs with a path input and
  directory match list.
- In file dialogs, printable text edits the path, Tab completes a unique or
  common path prefix, Up/Down moves the visible match selection,
  PageUp/PageDown moves by a page, Left/Right/Home/End and Delete edit the
  path input, `Ctrl+H` toggles hidden files, Enter opens or saves, and Esc
  cancels. Dotfiles are hidden by default unless the typed prefix starts with
  `.`, and `..` is available for parent-directory navigation. File-dialog
  errors should stay inline for correction, and Save As should require a
  second Enter before overwriting an existing file. Modal keys can be remapped
  in config for terminals or KVM paths that do not deliver the defaults.
- `Ctrl+P` opens the command prompt, and Up/Down recall command history.
- Prompt inputs support Left/Right/Home/End/Delete/Backspace at UTF-8 character
  boundaries.
- `Shift+Arrow` extends the editor selection by character or line, and
  `Shift+Home`/`Shift+End` extends to the current line edge when those strokes
  are not remapped by the active keymap.
- `Ctrl+L` selects the current line.
- PageUp/PageDown move by the visible editor pane height, using wrapped visual
  rows when word-wrap is active, and Shift+PageUp/PageDown extends selection
  by the same page model when delivered.
- `Ctrl+Home/End` move to document start/end in editable and read-only panes
  when the terminal delivers those modified keys.
- `Ctrl+Left/Right` move by word, `Ctrl+Backspace/Delete` delete by word, and
  `Ctrl+Shift+Left/Right` extends selection by word when the terminal delivers
  those modifiers.
- Long editor lines scroll horizontally to keep the cursor visible; the status
  bar reports the visible line range and horizontal offset, and clipped rows
  show left/right edge indicators.
- `Ctrl+W,[` and `Ctrl+W,]` scroll the focused editor viewport left/right when
  the line is wider than the pane.
- Long buffers display a lightweight right-border scrollbar thumb when there
  are more lines than fit in the pane.
- With mouse enabled, clicking or dragging the right-border scrollbar should
  scroll the editor body.
- Terminals that emit horizontal mouse wheel events should scroll the focused
  editor viewport without requiring a separate capability.
- Continuous ordinary typing and continuous same-direction Backspace/Delete
  should undo as grouped steps. Cursor movement, selection changes, newline,
  word delete, replace, and paste should make separate undo steps.
- Undo/Redo should report `Undo`, `Redo`, `Nothing to undo`, or
  `Nothing to redo` in the status area.
- Edit menu or command-line ids `edit.copy`, `edit.cut`, and `edit.paste`
  operate on the process-local internal clipboard. They should copy/cut the
  active selection and paste through the normal edit path without requiring OS
  clipboard access.
- With `clipboard.osc52.enabled = true`, command id `edit.copy_external`
  should copy the active selection internally and emit a bounded OSC 52
  clipboard write. Record whether the local terminal, SSH path, `tmux`, or
  `screen` accepts or filters the OSC 52 clipboard update; failure to update
  the host clipboard should not break the internal clipboard fallback.
- Terminal bracketed paste inserts into editor buffers through the normal edit
  path. In prompts and Open/Save As dialogs, multiline paste is converted to a
  single-line input and does not submit automatically. Right-click paste only
  works when the terminal sends bracketed paste data.
- Dirty buffers ask for confirmation before quit, new, open, or close.

Search and navigation:

- `Ctrl+F` opens Find, previews matches while typing, Enter keeps the
  previewed match, and Esc restores the prior cursor or selection. Prefixes
  `/i`, `/w`, and `/iw` enable ignore-case and whole-word matching.
- `F3` and `Shift+F3` move between matches; all visible matches are
  highlighted and the status fields show the active match count.
- `Ctrl+R` performs the replace prompt flow, previews the query, and then uses
  a confirmation modal for Replace, Skip, All, or Cancel. It accepts the same
  search prefixes as Find. Command prompt `replace all QUERY TEXT` replaces all
  matches as one undo step.
- `Ctrl+G` moves to a valid 1-based line number.
- `F1` opens the key reference window.
- `F2` opens the status history window.

Process actions:

- `Ctrl+W,S` should suspend the alternate-screen TUI, run the configured
  `$SHELL` with normal terminal stdio, and return to a usable editor after the
  shell exits.
- `Ctrl+W,O` should open the Run Command prompt. Submitting
  `printf dun-run` should create or reuse a read-only Command Output pane with
  exit status, stdout/stderr byte counts, and truncation status.
- Command prompt `output index`, `output summary`, `output status`,
  `output stdout`, `output stdout-body`, `output stderr`,
  `output stderr-body`, `output truncated`, `output find dun-run`,
  `output next`, `output previous`, `output copy`, `output clear`,
  `output save /tmp/dun-output.txt`, and pathless `output save` should operate
  on the read-only Command Output pane without making it editable. Pathless
  save should open the Save Command Output file dialog.
- `config keymap`, `config limits`, and `diagnostics file-dialog-keymap`
  should open Config Diagnostics and jump to the named section.
- Up/Down in the Run Command prompt should navigate only previous run-command
  inputs, not the editor command prompt history.

Tiling:

- `Ctrl+W,H` and `Ctrl+W,V` split the workspace.
- `Ctrl+W,Arrow` moves focus between panes.
- `Ctrl+W,Shift+Arrow` resizes the nearest split where the terminal sends
  shifted arrow keys.
- `Alt+Arrow` focus movement works as a compatibility alias where the terminal
  sends Option/Meta keys.
- `Alt+Shift+Arrow` resize works as a compatibility alias where available.
- `Ctrl+W,C` collapses or expands the focused pane.
- `Ctrl+W,X` closes the focused pane without leaking stale buffer state.

Low-capability expectations:

- If `Alt+Arrow` or `Alt+Shift+Arrow` is not delivered by a terminal or KVM,
  `Ctrl+W,Arrow` focus movement and `Ctrl+W,Shift+Arrow` resize remain the
  primary required path. Terminals that cannot deliver shifted arrows should
  record resize as a keybinding compatibility note, not an editor state
  failure.
- Mouse support is optional and disabled by default. If enabled, left-click
  focus, cursor placement, selection drag, split drag, top-menu dropdowns, and
  submenu clicks should work in capable terminals. Open/Save As file dialog
  list clicks should enter directories; Open should open files, and Save As
  should only update the path input. Mouse wheel events should scroll editor
  panes and file dialog lists when delivered by the terminal. Editor scrollbar
  clicks and drags should scroll long buffers. Mouse support is not required
  for passing the matrix.
- `Alt+F`, `Alt+E`, `Alt+V`, and `Alt+H` should open the grouped menus where
  the terminal sends Alt-modified character keys. If a terminal or KVM cannot
  deliver those strokes, command-line prompt and direct command keybindings
  remain the required keyboard path.
- ASCII/VT100 cases must remain keyboard usable even if colors are reduced or
  absent.
- UTF-8 text is not required to render correctly in `LANG=C`/ASCII cases, but
  the UI must not emit broken terminal controls or panic.

Pass criteria:

- No panic.
- No hung raw terminal.
- No broken alternate-screen restoration.
- No visible terminal escape injection from file content.
- Required keyboard-only workflows remain usable.

Known terminal limitations are acceptable if they are surfaced as keybinding
configuration needs rather than editor state corruption.
