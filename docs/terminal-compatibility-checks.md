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
- keyboard-first operation without mouse support.

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
- an invalid-byte file, checking that escaped bytes and `Escaped bytes` status
  are visible;
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

The current workspace does not provide an external SSH host. Current coverage is
therefore the automated PTY baseline plus local command execution. External
results must be gathered before a tagged release.

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
- `Ctrl+O` and `Ctrl+Shift+S` open status-line prompts.
- `Ctrl+P` opens the command prompt, and Up/Down recall command history.
- Dirty buffers ask for confirmation before quit, new, open, or close.

Search and navigation:

- `Ctrl+F` opens Find and selects the first match.
- `F3` and `Shift+F3` move between matches.
- `Ctrl+R` performs the replace prompt flow.
- `Ctrl+G` moves to a valid 1-based line number.
- `F1` opens the key reference window.
- `F2` opens the status history window.

Tiling:

- `Ctrl+W,H` and `Ctrl+W,V` split the workspace.
- `Alt+Arrow` focus movement works where the terminal sends those keys.
- Resize bindings work where the terminal sends `Alt+Shift+Arrow`.
- `Ctrl+W,C` collapses or expands the focused pane.
- `Ctrl+W,X` closes the focused pane without leaking stale buffer state.

Low-capability expectations:

- If `Alt+Arrow` or `Alt+Shift+Arrow` is not delivered by a terminal or KVM,
  the failure is recorded as a keybinding compatibility note, not an editor
  state failure.
- Mouse support is not required for passing the matrix.
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
