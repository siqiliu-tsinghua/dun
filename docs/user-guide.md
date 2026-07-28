# dun User Guide

`dun` is a terminal text editor for remote operations work: SSH into a host,
read and edit files, capture command output, and keep working on terminals that
are older, narrower, or more conservative than your laptop's. It is a single
binary under 1 MiB that links only against the system C library and its usual
companions — nothing to install alongside it, nothing to keep in sync.

This guide covers using `dun`. Two companion documents go deeper:
[configuration.md](./configuration.md) is the exhaustive key reference, and
[plugin-authoring.md](./plugin-authoring.md) is for writing plugins.

- [Installing](#installing)
- [First run](#first-run)
- [The screen](#the-screen)
- [Editing](#editing)
- [Find and replace](#find-and-replace)
- [Files](#files)
- [Windows](#windows)
- [Running commands](#running-commands)
- [The command prompt](#the-command-prompt)
- [Clipboard, including over SSH](#clipboard-including-over-ssh)
- [Appearance and terminal fallbacks](#appearance-and-terminal-fallbacks)
- [Interface language](#interface-language)
- [Plugins](#plugins)
- [Keyboard reference](#keyboard-reference)
- [Troubleshooting](#troubleshooting)

## Installing

`dun` builds with Rust `1.85` or newer on Linux, macOS, FreeBSD, and Solaris.
There are no system dependencies to install first.

```sh
git clone <repository-url> dun
cd dun
cargo build --release
./target/release/dun --version
```

Copy `target/release/dun` anywhere on your `PATH`. To put it on a server, copy
the single file — it needs nothing else, though a binary built on your machine
runs only on hosts of the same OS and architecture.

**On the two builds.** `cargo build --release` is the ordinary build and is what
you want. The repository also has `scripts/release-build.sh`, which produces a
significantly smaller binary by rebuilding the standard library
(`-Zbuild-std`); that is the build the project's 1 MiB size budget is measured
against. It needs the `rust-src` component and sets `RUSTC_BOOTSTRAP=1` to use
an unstable flag on a stable toolchain. Use it if you care about the last
200 KiB; otherwise ignore it.

## First run

```sh
dun              # an empty untitled buffer
dun notes.txt    # open a file
dun -- -weird    # everything after -- is a path
```

Options are `-h`/`--help`, `-V`/`--version`, `--config PATH`, `--dump-config`,
and `--no-config`. `dun` exits `0` on success, `1` on a runtime or file I/O
error, and `2` on a command-line usage error, so it behaves in a script the way
you would expect.

Quit with `Ctrl+Q`. If a buffer has unsaved changes, a dialog asks first, and
answers to single letters: **s** save, **d** discard, **c** cancel.

`dun` starts with built-in defaults. To customise it, write
`~/.config/dun/config`; `dun --dump-config > ~/.config/dun/config` gives you
every default as a commented starting point. The load order is `--no-config`
(disables everything), then `--config PATH`, then `$DUN_CONFIG`, then
`$XDG_CONFIG_HOME/dun/config`, then `$HOME/.config/dun/config`. A missing
default file is fine; an explicitly named one that is missing or invalid is an
error.

## The screen

```
┌ menu bar ─────────────────────────────────────────────────────┐
│ File  Edit  View  Help                                         │
├───────────────────────────────────────────────────────────────┤
│ ◆ notes.txt                        ← window title, ◆ = focused │
│  1 the line-number gutter is on the left                       │
│  2 the current line is highlighted                             │
├───────────────────────────────────────────────────────────────┤
│ [Plain Text] [LF] [UTF-8] [Spaces:4] 12:5 [View 1-20/36] …     │
└ status bar ───────────────────────────────────────────────────┘
```

The menu bar opens with `Alt+F`, `Alt+E`, `Alt+V`, `Alt+H` when the active
keymap does not claim those keys. Arrow keys move, `Enter` runs the entry, `Esc`
closes. Every menu entry is the same typed command as the equivalent keybinding,
so nothing is reachable only by mouse or only by menu.

The status bar shows the file type, line ending, encoding, indentation, the
cursor's line and column, the visible line range, and — when something has just
happened — a status message. `F2` opens **Status History** if a message scrolled
past before you read it.

Four read-only helper windows open over the workspace: **Help** (`F1`, generated
from your active keymap, so it is correct even after you rebind everything),
**Status History** (`F2`), **Config Diagnostics** (`F6`, what config was loaded
from where, and which important commands are unbound), and **Search Results** /
**Command Output**, described below. Close a helper window with `Ctrl+X,X` and
focus returns where it came from.

## Editing

Movement is conventional: arrows, `Home`/`End`, `PageUp`/`PageDown`,
`Ctrl+Left`/`Ctrl+Right` by word, `Ctrl+Home`/`Ctrl+End` to the document ends.
Hold `Shift` with any of them to extend a selection. `Ctrl+A` selects the
buffer, `Ctrl+L` the current line.

| Action | Key |
| --- | --- |
| Undo / redo | `Ctrl+Z` / `Ctrl+Y` |
| Cut / copy / paste (internal clipboard) | *no default key* — see below |
| Delete word left / right | `Ctrl+Backspace` / `Ctrl+Delete` |
| Delete line | `Ctrl+K` |
| Copy line | `Ctrl+X,Y` |
| Move line up / down | `Ctrl+X,U` / `Ctrl+X,J` |
| Indent / outdent | `Tab` / `Shift+Tab` |
| Trim trailing whitespace | `Ctrl+X,T` |
| Toggle word wrap | `Ctrl+X,Z` |
| Toggle visible whitespace | `Ctrl+X,.` |
| Toggle bookmark | `Ctrl+X,K` |
| Next / previous bookmark | `Ctrl+X,N` / `Ctrl+X,L` |
| Go to line | `Ctrl+G` |

Undo groups typing into sensible transactions: a run of ordinary characters
collapses into one undo step, while paste, movement, selection, and deletion
start new ones. A run of `Backspace` or `Delete` in the same direction also
collapses into one step.

**The `Ctrl+X` prefix.** Keys that would otherwise collide with terminal or
shell conventions live behind `Ctrl+X` as two-stroke chords. Press `Ctrl+X`,
then the second key. Nothing is lost if you forget the second stroke — press
`Esc`.

**Cut, copy, and paste have no default keys.** The three keys everyone reaches
for are all spoken for in a terminal: `Ctrl+C` raises `SIGINT`, `Ctrl+X` is the
chord prefix, and `Ctrl+V` means "quote the next key" in many line editors. So
`edit.cut`, `edit.copy`, and `edit.paste` ship unbound, reachable from the
**Edit** menu (`Alt+E`) or by name at the command prompt. Bind them to whatever
your terminal leaves free. The DOS-editor trio parses and is the closest thing
to a convention here, if your terminal delivers modified `Insert`/`Delete`:

```
key.edit.cut = Shift+Delete
key.edit.copy = Ctrl+Insert
key.edit.paste = Shift+Insert
```

If it does not, use the chord family, which every terminal that runs `dun` can
send. `A`, `D`, `F`, `G`, `I`, and `W` are the second strokes no default binding
claims:

```
key.edit.cut = Ctrl+X,W
key.edit.copy = Ctrl+X,A
key.edit.paste = Ctrl+X,G
```

`F6` shows which of your bindings took effect, and an invalid key name is
reported at startup rather than silently dropped.

Selecting text and pressing your terminal's own copy shortcut also works, and
`Ctrl+X,Ctrl+C` / `Ctrl+X,Ctrl+V` reach the *system* clipboard over SSH once
OSC 52 is enabled — see [Clipboard](#clipboard-including-over-ssh).

Bookmarks are per-buffer positions marked with a `*` in the gutter; `Ctrl+X,N`
and `Ctrl+X,L` cycle through them in document order and wrap around. Visible
whitespace draws tabs and trailing spaces as marks; it is off by default and
costs nothing when off.

## Find and replace

`Ctrl+W` opens Find, `F3` and `Shift+F3` step through matches, and `Ctrl+R`
opens Replace. Matches highlight as you type, and the status bar shows which
match you are on.

A query can carry flags as a `/`-prefixed first word:

| Query | Meaning |
| --- | --- |
| `error` | case-sensitive substring (the default) |
| `/i error` | ignore case |
| `/c error` | force case-sensitive |
| `/w error` | whole word only |
| `/iw error` | combine flags |

The prefix only counts when the flags are followed by whitespace and a query, so
a literal search for `/i` still works.

Replace is interactive. At each match the dialog answers to single letters:
**r** replace this one, **s** skip it, **a** replace all remaining, **c** cancel.
Cancelling keeps what was already replaced and tells you the counts.

To replace without stepping through matches, use the command prompt:
`replace all QUERY TEXT` applies every replacement in one undo transaction.

`results` (or the View menu) opens **Search Results**: a read-only list of every
match with its line number. `n` and `p` move, `Enter` jumps to the match, and
numbered entries can be jumped to directly.

## Files

| Action | Key |
| --- | --- |
| New | `Ctrl+N` |
| Open | `Ctrl+O` |
| Save | `Ctrl+S` |
| Save As | `Ctrl+Shift+S` |
| Reload from disk | `Ctrl+X,E` |
| Close buffer | `Ctrl+X,Q` |
| Switch buffer | `Ctrl+X,B` |

The Open and Save As dialogs complete paths with `Tab` (`Shift+Tab` cycles
backwards), navigate with the arrow keys and `PageUp`/`PageDown`, show hidden
files with `Ctrl+H`, and accept `..` to go up. Save As asks before overwriting,
and asks twice — the same path has to be submitted again to confirm.

`dun` is deliberate about file safety, because a remote editor that corrupts a
file is worse than no editor:

- **Saves are atomic.** The new content is written to a temporary file in the
  same directory and renamed over the original, so an interrupted save cannot
  leave a half-written file. Orphaned temp files from a crashed session are
  cleaned up on the next run, while anything newer than the target is kept and
  reported rather than deleted.
- **A changed file is not silently overwritten.** Every opened file keeps a
  verified metadata snapshot. If the file on disk has changed, moved, or
  disappeared since it was loaded, Save refuses and says so. Use `Ctrl+X,E` to
  reload and lose your in-memory changes deliberately, or Save As to write
  somewhere else.
- **Non-UTF-8 files open read-only.** Rather than decoding them lossily, `dun`
  shows the bytes escaped, marks the buffer read-only, and blocks Save and Save
  As. You can read a binary or a Latin-1 log safely; you cannot corrupt it.
- **Large files are refused before they are read.** The default limit is 16 MiB
  (`limits.editable_file_soft_limit_bytes`). Raise it if you mean to.
- **An unstable read is rejected.** If the file changes while it is being read,
  Open reports the unstable snapshot instead of opening a torn one.

Control bytes in file content — escape sequences, OSC strings, `BEL`, `NUL`,
`DEL`, stray `CR` — are rendered visibly rather than sent to your terminal. A
log full of ANSI escapes cannot reprogram your terminal through the editor.

## Windows

`dun` tiles: a split tree, no floating windows, no tabs. Every window shows a
buffer, and the same buffer can be open in several windows.

| Action | Key | Alias |
| --- | --- | --- |
| Split horizontally / vertically | `Ctrl+X,H` / `Ctrl+X,V` | |
| Focus left / right / up / down | `Ctrl+X,←→↑↓` | `Alt+←→↑↓` |
| Resize toward a side | `Ctrl+X,Shift+←→↑↓` | `Alt+Shift+←→↑↓` |
| Equalize splits | `Ctrl+X,=` | |
| Rotate a split's orientation | `Ctrl+X,R` | |
| Collapse / expand | `Ctrl+X,M` / `Ctrl+X,P` | |
| Toggle collapse | `Ctrl+X,C` | |
| Close window | `Ctrl+X,X` | |
| Only this window | `Ctrl+X,1` | |

The `Alt` aliases exist because some terminals and KVMs eat `Alt`; the `Ctrl+X`
forms always work. Closing a window repairs the tree — the remaining sibling
takes the space.

## Running commands

`Ctrl+X,S` suspends `dun` and drops you to a shell, Turbo Pascal style. Exit the
shell and the editor comes back with its screen intact.

`Ctrl+X,O` runs a single command and captures its output into a read-only
**Command Output** pane without leaving the editor. Output is bounded, and the
command is killed after `limits.run_command_timeout_ms` (30 s by default).

Inside Command Output you can view stdout only, stderr only, or the summary;
search it; jump to a section or a numbered line; save it to a file; and clear
it. It is the pane a log-filter plugin writes into, which is why filtering the
output of a command you just ran is a two-step operation rather than a pipeline.

## The command prompt

`Ctrl+P` opens a prompt that runs commands by name. `Tab` completes, and
completion cycles through candidates; paths complete too. History is kept for
the session.

```
theme msedit           switch theme at runtime
open /var/log/syslog   open a path
find /i timeout        search with flags
replace all foo bar    replace everything in one undo step
goto 420               jump to a line
run journalctl -n 50   run a command into Command Output
results                open the search results list
buffers                switch buffer
save / saveas PATH     write the buffer
wrap / whitespace      toggle word wrap / visible whitespace
plugin load ID         load or reload a plugin host
config keymap          jump to a Config Diagnostics section
status                 status history
commands               list what the prompt accepts
```

Anything the prompt does not recognise as one of those verbs is tried as a
**command id**. Every command in the editor has one — 91 of them across four
families (`app.*`, `edit.*`, `file.*`, `window.*`) — so `window.split_vertical`
or `edit.trim_trailing_whitespace` can be run by name even if it has no
keybinding. The same ids are what you bind keys to in the config file, and
`F6` lists the important ones that are currently unbound.

## Clipboard, including over SSH

By default `dun` uses an **internal clipboard**: cut, copy, and paste work
inside the editor and never touch the system clipboard or shell out to
`pbcopy`/`xclip`. That is deliberate — external clipboard commands do not exist
on a bare server and do not work across SSH.

To move text between `dun` on a remote host and your local machine, enable
OSC 52, a terminal escape sequence that carries clipboard data over the SSH
connection itself:

```
clipboard.osc52.enabled = true      # copy out:  Ctrl+X,Ctrl+C
clipboard.osc52.allow_read = true   # paste in:  Ctrl+X,Ctrl+V
clipboard.osc52.max_bytes = 16384   # payload cap
```

The two switches are separate on purpose: enabling copy never grants read
access to your clipboard.

Reading is best-effort and depends on your terminal, not on `dun`. Most
terminals disable clipboard *reads* by default or prompt before answering,
because a remote program that can read your clipboard can read whatever you last
copied. If nothing answers within about half a second, `dun` falls back to the
internal clipboard and says so. Ordinary `Ctrl+V` always pastes the internal
clipboard and is unaffected.

Terminal-side paste (bracketed paste, `Cmd+V`, middle-click) is routed into
buffers, prompts, and dialogs and works without any of this.

## Appearance and terminal fallbacks

Four themes ship: `dun` (the default — pale sand text and a buckskin accent on
deep shadow), `msedit` (Microsoft Edit-style blue chrome), `dark` (neutral dark
with a cyan accent), and `turbo` (Borland Turbo Vision deep blue). Select one
with `theme = msedit` in the config or `theme msedit` at the prompt, and
override individual colors with `color.<role>` entries — `dun --dump-config`
lists every role.

`dun` detects what the terminal can do and falls back on its own. Each theme
carries 256-color, 16-color, and monochrome variants; the monochrome variant
emits no color at all, only bold, underline, and reverse. You can force the
outcome:

```
terminal.colors = 256 | 16 | mono
terminal.encoding = utf8 | ascii          # ascii replaces box drawing with -|+
terminal.ambiguous-width = narrow | wide  # East Asian ambiguous-width glyphs
```

`NO_COLOR` in the environment forces monochrome, as it does elsewhere.

Ambiguous-width characters — box drawing, `◆`, and similar — are rendered one
cell wide by some terminals and two by others, which shifts an entire tiled
layout if guessed wrong. `dun` probes the terminal at startup and adapts, so the
setting is an override for when the probe cannot run, not something you normally
touch.

Mouse support is off by default (`mouse.enabled = true` turns it on): click to
focus a window and place the cursor, drag to select, drag a split to resize,
wheel to scroll, click menus and dialog entries. Leaving it off keeps your
terminal's own selection and copy behaviour intact, which is usually what you
want over SSH.

## Interface language

`dun` ships ten translations — `de`, `es`, `fr`, `it`, `ja`, `ko`, `pt`, `ru`,
`zh-Hans`, `zh-Hant` — as external files. English is compiled in and is the
fallback for any key a catalog is missing.

The language comes from the environment, in the order `LC_ALL`, `LC_MESSAGES`,
`LANG`:

```sh
LC_ALL=ja_JP.UTF-8 dun
```

Catalogs are looked up in an `i18n/` directory **next to the active config
file**, so `~/.config/dun/config` pairs with `~/.config/dun/i18n/ja.conf`. Copy
the repository's `i18n/` directory there. You do not need a config file for
this — with no config, `dun` still looks in `~/.config/dun/i18n/`. `--no-config`
is the exception: it disables catalog loading with everything else, so that run
is English.

If the interface stays English, that lookup is almost always why: the catalogs
are not beside the config file `F6` reports as active.

Nine of the ten catalogs are machine-translated and have not been reviewed by a
native speaker; the project says so rather than implying review is pending.
Menu mnemonics, key names, and command ids stay English by construction, so a
translation cannot break a keybinding. Corrections are welcome — see
[i18n.md](./i18n.md).

## Plugins

A plugin is a **separate program** that `dun` launches and talks to over its
stdin and stdout. Plugins are not loaded into the editor's address space, cannot
be a shared library, and can only reach the parts of the editor the protocol
exposes. The reference plugins in `hosts/` are Python, Lua, and Rust programs
that add syntax highlighting and log filtering.

Installing one is unpacking a folder; uninstalling one is deleting it. A
plugin's own settings live with the plugin, not in `dun`'s config. What goes in
your config is only how to launch it and what it is trusted with:

```
plugin.pygments.command = /opt/dun-hosts/pygments/dun-pygments-host.py
plugin.pygments.trust = user-trusted-external
plugin.pygments.roles = syntax-highlight
plugin.pygments.timeout_ms = 2000
```

A **role** is a named bundle of capabilities — what the plugin may see and do.
`syntax-highlight` receives buffer text and returns styling; `log-filter` may
contribute a menu, own windows, read a command's output stream, and take input
from a scratch buffer. The **trust class** decides which roles may be granted at
all. `dun` validates every result before applying it, and a plugin that hangs,
crashes, or returns malformed output is dropped with a status message rather
than taking the editor with it.

`Ctrl+T` is reserved for plugins: every plugin's chords live under it, so a
plugin can never shadow an editor key. `plugin load ID` and `plugin unload ID`
manage hosts at runtime, and `plugins.status_bar = true` shows a status-bar chip
per host.

Writing one is documented in [plugin-authoring.md](./plugin-authoring.md); the
wire protocol is [plugin-protocol.md](./plugin-protocol.md).

## Keyboard reference

**The authoritative list is in the editor.** `F1` builds the key reference from
your active keymap, and `dun --dump-config` prints every binding as
configuration. Both stay correct after you rebind things; a table in a document
does not.

Rebinding is one line per command id in the config:

```
key.edit.find = Ctrl+F
key.window.split_vertical = Alt+V
key.app.quit = Ctrl+X,Ctrl+C     # two-stroke chords are allowed
```

A command may have several bindings — the defaults bind window focus to both
`Ctrl+X,←` and `Alt+←` for exactly that reason.

The defaults, by area:

| Area | Keys |
| --- | --- |
| Application | `F1` help, `F2` status history, `F5` reload config, `F6` config diagnostics, `Ctrl+P` command prompt, `Ctrl+Q` quit, `Ctrl+X,O` run command, `Ctrl+X,S` shell |
| File | `Ctrl+N` new, `Ctrl+O` open, `Ctrl+S` save, `Ctrl+Shift+S` save as, `Ctrl+X,E` reload, `Ctrl+X,Q` close, `Ctrl+X,B` switch buffer |
| Edit | `Ctrl+Z`/`Ctrl+Y` undo/redo, `Ctrl+A` select all, `Ctrl+L` select line, `Ctrl+K` delete line, `Ctrl+G` go to line, `Ctrl+W` find, `F3`/`Shift+F3` next/previous, `Ctrl+R` replace |
| Windows | `Ctrl+X,H`/`Ctrl+X,V` split, `Ctrl+X,←→↑↓` focus, `Ctrl+X,X` close, `Ctrl+X,1` only |
| Reserved | `Ctrl+T` — plugin chords |

## Troubleshooting

**The borders are garbled, or the layout is one column off.** Your terminal
disagrees with `dun` about ambiguous-width characters. Set
`terminal.ambiguous-width = wide` (common inside `tmux` on some systems), or
`terminal.encoding = ascii` to sidestep the question entirely.

**Everything is monochrome.** `NO_COLOR` is set, or the terminal did not report
color support. Force it with `terminal.colors = 256`.

**`Alt` shortcuts do nothing.** Some terminals and KVM switches never deliver
`Alt`. Every `Alt` binding has a `Ctrl+X` equivalent; `F1` shows both.

**Paste from my laptop does nothing.** `Ctrl+X,Ctrl+V` needs
`clipboard.osc52.allow_read = true` *and* a terminal willing to answer a
clipboard read — many refuse by default. Your terminal's own paste (`Cmd+V`,
middle-click) works regardless.

**Save refuses.** Either the file changed on disk since you opened it (reload
with `Ctrl+X,E`, or use Save As), or the buffer is a non-UTF-8 read-only view.
The status message says which.

**The file is too large to open.** Raise
`limits.editable_file_soft_limit_bytes`. The limit exists so a stray multi-gigabyte
log does not take the host down with the editor.

**A plugin does nothing.** `F6` shows what was loaded and any error, the
status-bar chip (`plugins.status_bar = true`) shows per-host state, and
`plugin load ID` reloads a host after you fix its config.

**Something scrolled past before I read it.** `F2`.
