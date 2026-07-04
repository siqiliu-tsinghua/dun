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

This checklist is not a release certification matrix yet. Before a tagged
release, repeat it on real external SSH hosts and record failures as TODO
items or regression tests.

## Automated Baseline

The default regression gate is:

```text
cargo test -p dun-cli --test pty_smoke
```

That test runs the real `dun` binary in a pseudo-terminal through `expect(1)`.
It fixes the PTY size to `80x24`, sends `Ctrl+Q`, and checks startup/exit under:

- `TERM=xterm-256color`, UTF-8 locale;
- `TERM=screen-256color`, UTF-8 locale;
- `TERM=vt100`, `C` locale;
- startup with a UTF-8 file path.

If `expect(1)` is not installed, the test exits successfully after printing a
skip message. Full workspace tests still run normally.

## Manual Matrix

Run the checklist below in these environments when available:

```text
local terminal             TERM=xterm-256color       UTF-8 locale
ssh direct                 TERM=xterm-256color       UTF-8 locale
ssh inside tmux            TERM=screen-256color      UTF-8 locale
ssh inside screen          TERM=screen               UTF-8 locale
forced low capability      TERM=vt100                LC_CTYPE=C
forced mono                NO_COLOR=1                UTF-8 locale
```

The current workspace does not provide an external SSH host. Current coverage is
therefore the automated PTY baseline plus local command execution. The external
SSH matrix remains a release-hardening task.

## Setup

Build the binary first:

```text
cargo build -p dun-cli
```

Create a small file for open/save checks:

```text
printf 'alpha\nbeta\n' > /tmp/dun-terminal-smoke.txt
```

Run either the local binary or an installed `dun`:

```text
./target/debug/dun
./target/debug/dun /tmp/dun-terminal-smoke.txt
TERM=vt100 LC_CTYPE=C ./target/debug/dun /tmp/dun-terminal-smoke.txt
NO_COLOR=1 ./target/debug/dun /tmp/dun-terminal-smoke.txt
```

Over SSH, use the same commands after copying or installing the binary on the
remote host.

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
- No raw file controls or escape payloads execute in the terminal.

Editing:

- Printable ASCII text inserts at the focused cursor.
- UTF-8 text displays correctly in UTF-8 profiles.
- Arrow keys, Home, End, Enter, Backspace, and Delete work.
- `Ctrl+S` saves a loaded file.
- `Ctrl+O` and `Ctrl+Shift+S` open status-line prompts.
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

Pass criteria:

- No panic.
- No hung raw terminal.
- No broken alternate-screen restoration.
- No visible terminal escape injection from file content.
- Required keyboard-only workflows remain usable.

Known terminal limitations are acceptable if they are surfaced as keybinding
configuration needs rather than editor state corruption.
