# Changelog

Notable user-visible changes. This project follows [semantic
versioning](https://semver.org/); the development record behind each entry is
in [docs/dev/PROGRESS.md](docs/dev/PROGRESS.md).

## Unreleased

### Added

- **An installed configuration, and your own on top of it.** Configuration is
  now two layers: `<bin>/../share/dun/config` — derived from the running
  binary, so `/opt/dun/bin/dun` reads `/opt/dun/share/dun/config` — and then
  your `~/.config/dun/config` applied over it key by key. What you set wins;
  what you leave alone keeps the installed value. A shared machine can now have
  a theme, a plugin and a keymap for everyone while each user changes only what
  they care about. `--no-config` disables both. An invalid *installed* file is
  reported in the status line and stepped over rather than being fatal: it
  belongs to whoever installed `dun`, and one mistake there must not stop every
  user of the machine from editing. Your own file is still a startup error when
  it is invalid.
- **Catalogs are also found in the installation directory.** `dun` looked for
  translations only in an `i18n/` directory beside the active configuration
  file, which a system-wide install has no way to fill: every user started in
  English until they made their own copy. There is now a second location,
  `<bin>/../share/dun/i18n`, alongside the installed configuration — so an
  administrator can translate the interface for everyone, and a relocated
  binary keeps its catalogs. Your own directory still wins, a broken file there
  is still reported rather than replaced by the shared copy, and `--no-config`
  still disables both. `F6` lists the search path under **Paths** and the
  configuration base layer under **Source**.
- **`scripts/build.sh`.** Asks which build you want (size-optimised
  `-Zbuild-std` or ordinary `cargo build --release`, defaulting to whichever
  your toolchain supports) and whether to build the syntect syntax-highlighting
  host with it, then builds. Interactive only when stdin is a terminal;
  `--yes`, `--budget`, `--plain`, `--syntect`, `--no-syntect` for scripts.
- **`scripts/install.sh`.** Building `dun` produced one executable and nothing
  else, so a first run had no configuration file and no catalogs — an English
  interface no matter what the locale said. The script installs the binary and
  the syntect plugin into `<prefix>/bin`, writes `<prefix>/share/dun/config`
  from `dun --dump-config` with the plugin enabled in it, copies the catalogs
  to `<prefix>/share/dun/i18n`, leaves a commented `~/.config/dun/config` for
  your own settings, and reports which catalog the current locale selects. The
  prefix is `$HOME/.local` unless you say otherwise; it asks where to install
  (`~/.local`, `/usr/local`, `/opt/dun`, or somewhere else), whether to enable
  the plugin, and whether to add the binary's directory to `PATH` in your
  shell's rc file. Every question is asked before anything is written: the plan
  is shown in full and confirmed once, so an interrupted interview leaves the
  machine untouched. `--package FILE` writes a tarball — binary, plugin,
  catalogs, scripts — for installing on a machine that cannot build, where the
  same script installs from the unpacked tree. Nothing is overwritten without
  `--force`, and `--dry-run` prints the plan and stops.
- **`scripts/uninstall.sh`.** Takes back out what `install.sh` put in, and
  nothing else: the binary — only if it identifies itself as `dun` — the plugin
  host beside it, the installed configuration, and the catalogs this tree
  ships. A catalog you added is reported and kept, your own
  `~/.config/dun/config` survives unless you pass `--purge`, and directories go
  only when they end up empty (a prefix made for `dun` alone with them, a
  shared `/usr/local` never). Same shape as the installer: plan, one
  confirmation, then removal.
- **Folding.** `Ctrl+X,F` folds the selected lines or unfolds the fold at the
  cursor; `Ctrl+X,A` unfolds everything in the buffer. Both are on the View
  menu. A folded range draws one placeholder row showing the hidden line count
  and an excerpt of the first line; the gutter keeps the fold's start line
  number and aggregates bookmarks from the whole range. Folds are manual, do
  not nest, survive editing (an edit that touches a fold drops it), and are
  never written to disk. Go To Line, bookmark navigation and committed search
  jumps expand a fold holding their target; the live Find preview deliberately
  does not, so cancelling a search cannot lose fold state.

### Fixed

- **A command no longer leaves processes running behind it.** `Ctrl+X,O` gives
  each command its own process group and cleans that group up when the command
  finishes or times out. A command that backgrounds something — `make &`, a
  pipeline whose tail outlives the shell — now returns as soon as the shell
  exits instead of occupying the whole timeout, and what it started does not
  survive. The status line says when something was killed. The trade is
  deliberate: **`Ctrl+X,O` is not a way to launch a background service**, and
  never reliably was — before this it hung the editor instead. Use Shell Escape
  (`Ctrl+X,S`) for that. A command that deliberately escapes with `setsid` is
  still beyond reach; no way of preventing that works across all four supported
  platforms.

- **`Ctrl+X,O` can no longer hang the editor.** Running a command that left
  anything in the background — `make &`, starting a daemon, any pipeline whose
  tail outlived the shell — froze `dun` for as long as that process lived, with
  no way out. The capture waited for end-of-file on the command's output, which
  needs *every* copy of the pipe closed, and a background process inherits one.
  The 30 s timeout did not help: the shell itself had already exited normally,
  so as far as `dun` was concerned nothing had timed out. Output is now read
  under a deadline and the capture always returns, reporting the output as
  truncated if something was still holding it open. A command that leaves a
  background process still costs the full timeout for now, and what it leaves
  behind is not yet cleaned up.

- **A `#` inside a quoted configuration value no longer corrupts it.** Comments
  were stripped from the whole line before quotes were honoured, so
  `plugin.syntect.command = "/opt/dun#tools/host"` became `"/opt/dun` — cut at
  the `#`, with the unbalanced opening quote still attached to what `dun` then
  tried to execute. An unterminated quote raised no error at all. A line is now
  split at its first `=` and a value beginning with `"` or `'` runs to its
  match, with `#` literal inside it; an unterminated quote is a startup error
  naming the line. Two consequences worth knowing: `#` is now usable as a
  keybinding (`key.edit.find = "Alt+#"` — it was rejected as "missing key"
  before), and `scripts/install.sh` quotes the host path it writes, so
  installing under a prefix containing `#` produces a configuration `dun` can
  read back. A value containing `#` must be quoted; there is deliberately no
  escape syntax, so a value with `#` *and* both quote characters cannot be
  represented.

- **Bookmarks follow their text.** A bookmark marked a line *number*: inserting
  five lines above a bookmark on line 10 left it on line 10 while the text it
  marked moved to 15. Bookmarks now shift with every edit, including bulk
  replace and undo.

### Changed

- **The plugin protocol's JSON is now strict.** Numbers must follow RFC 8259 —
  `01`, `1.`, `-.5` and `00.5` were accepted before and are now rejected, while
  `-0`, `1e+2` and `1E5` remain valid. Object keys must be unique: duplicates
  were previously accepted with the *first* value winning, which is the
  opposite of what `serde_json`, JavaScript and Python do, so `dun` and a host
  could read the same bytes differently. Each object may now carry at most 64
  members — the protocol's largest real object is a seven-member envelope, and
  the cap is what makes duplicate detection affordable without allocating. An
  integer field outside 0 to 2^53-1 is now reported as malformed rather than
  *missing*, which is what it looked like before. `hosts/check-host.py` applies
  the same duplicate-key rule, so a host that passes the checker parses in
  `dun`. Every shipped host was checked against the stricter rules and none
  needed changing.
- `Ctrl+X,F` and `Ctrl+X,A` are now default bindings. A configuration that
  binds either chord to something else will be rejected at startup with a
  message naming both commands and how to unbind one
  (`key.edit.unfold_all = none`).
- **Solaris builds are ~343 KB smaller.** `scripts/release-build.sh` now links
  with `-z noldynsym` on Solaris, dropping the symbol-table sections the native
  link editor keeps so `pstack` and `dtrace` can name local frames. Stack
  traces from a release binary lose those names; set
  `DUN_SOLARIS_KEEP_LDYNSYM=1` to keep them, which is also what you need if you
  link with GNU `ld`. All four supported platforms now build under the 1 MiB
  budget.
- `scripts/release-build.sh` is POSIX `sh` instead of `bash`, so it runs on
  FreeBSD, whose base system has no `bash`.

## v0.1.0 — 2026-07-28

First release. `dun` is a terminal text editor for remote operations work: a
single binary under 1 MiB, a tiling keyboard-first interface, and a plugin
protocol that runs hosts as separate processes.

### Editing

- UTF-8 text editing with undo grouped into transactions — a run of typing
  collapses into one step, while paste, movement, selection, and deletion start
  new ones; same-direction `Backspace`/`Delete` runs also collapse.
- Word-wise movement, selection, and deletion at UTF-8 boundaries; keyboard
  selection with `Shift` on every movement key; page movement that follows
  visual rows when word wrap is on.
- Line operations: copy, delete, move up/down, indent, outdent, trim trailing
  whitespace.
- Per-buffer bookmarks with a gutter marker, and visible whitespace markers —
  both view state that never changes the file.
- Word wrap as a display mode: scrolling, selection, search highlights, and
  the gutter all operate on wrapped visual rows.

### Search

- Find and Replace with live match preview and a match counter.
- Query flags as a `/`-prefixed first word: `/i` ignore case, `/c` force case
  sensitivity, `/w` whole word, combinable as `/iw`.
- Interactive replace answering to `r` / `s` / `a` / `c`, and
  `replace all QUERY TEXT` from the command prompt landing in one undo
  transaction.
- A read-only Search Results list with jump-to-match navigation.

### Windows and files

- A tiling split tree: split, directional focus, resize, equalize, rotate,
  collapse, and close with tree repair. No tabs, no floating windows.
- Open/Save As dialogs with path completion, hidden-file toggle, directory
  navigation, and a second-`Enter` overwrite confirmation.
- Buffer switcher, reload from disk, and a dirty-buffer confirmation before
  quit, new, open, or close.

### Safety

- Saves go through a same-directory temporary file and an atomic rename;
  orphaned temp files from a crashed session are reconciled, and newer recovery
  candidates are preserved and reported rather than deleted.
- A file that changed on disk since it was opened is not silently overwritten.
- Files that are not valid UTF-8 open read-only with escaped bytes, and are
  blocked from Save and Save As, rather than being decoded lossily.
- A large-file soft limit (16 MiB by default) is enforced before the file is
  read into memory.
- All buffer text, file names, and status fields are sanitized before
  rendering: C0/C1 controls, escape and OSC sequences, bidirectional overrides
  (Trojan Source, CVE-2021-42574), zero-width format characters, and the
  Unicode tag block are rendered visibly instead of acting.

### Terminal

- Capability detection with 256-color, 16-color, and monochrome variants of
  every theme, plus ASCII glyph fallback; `NO_COLOR` is honoured.
- East Asian ambiguous-width glyphs are probed at startup, so a terminal that
  draws them double-wide does not shift the layout.
- Four themes: `dun`, `msedit`, `dark`, `turbo`, with per-role colour
  overrides.
- Optional mouse support: window focus, cursor placement, selection, split
  dragging, menu and dialog clicks, wheel scrolling.
- Terminal I/O is in-house — no `crossterm`, no `ratatui`, no `mio`.

### Shell and command output

- `Ctrl+X,S` suspends to a shell and resumes with the screen intact.
- `Ctrl+X,O` runs one command into a read-only Command Output pane with a byte
  cap and a configurable timeout.
- A command prompt with Tab completion over every one of the 91 command ids.

### Clipboard

- An internal clipboard by default; no external clipboard commands are ever
  invoked.
- Opt-in OSC 52 copy (`clipboard.osc52.enabled`) and, separately, OSC 52 paste
  (`clipboard.osc52.allow_read`) — enabling copy never grants read. Reads wait
  at most 500 ms and fall back to the internal clipboard.
- Bracketed paste is routed into buffers, prompts, and dialogs as untrusted
  input; prompt paste stays single-line and never auto-submits.

### Plugins

- A host-neutral plugin protocol over framed stdio JSON. Hosts are separate
  processes; nothing is loaded into the editor.
- Roles as named capability bundles — `syntax-highlight` and `log-filter` in
  v0 — with the configured trust class as the grant gate. A host that
  over-claims its trust is rejected at launch.
- Plugin-contributed menus and keybindings, with author-declared mnemonics;
  `Ctrl+T` is reserved for every plugin so a plugin cannot shadow an editor
  key.
- Every result is validated before it reaches editor state; a host that hangs,
  crashes, floods, or returns malformed output is dropped with a status
  message.
- Reference hosts in `hosts/`: syntax highlighting via syntect, Pygments, and
  a dependency-free Lua host, plus log-filter hosts in Python and Lua.

### Interface languages

- Ten translations shipped as external catalogs — `de`, `es`, `fr`, `it`,
  `ja`, `ko`, `pt`, `ru`, `zh-Hans`, `zh-Hant` — selected from the environment.
  English is compiled in as the fallback. Menu mnemonics and key names stay
  English by construction, so a translation cannot break a keybinding.
- Nine of the ten are machine-translated and unreviewed; see
  [docs/i18n.md](docs/i18n.md).

### Known limitations

- `edit.cut`, `edit.copy`, and `edit.paste` ship with no default keybinding —
  `Ctrl+C`, `Ctrl+X`, and `Ctrl+V` are all spoken for in a terminal. They are
  on the Edit menu and the command prompt, and the user guide gives two
  working binding sets.
- OSC 52 paste depends on the terminal answering a clipboard read, which many
  refuse by default.
- Plugin dropdown entries have no mnemonic when a host declares none; this is
  deliberate, since no derivation rule survives translation.
