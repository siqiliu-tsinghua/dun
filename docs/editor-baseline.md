# Editor Baseline Decisions

This document records the first usable product line for `dun`.

The initial target is a lightweight Microsoft Edit-style terminal editor with a
tiling split workspace. Log processing, `rum` plugins, and programmable filters
remain important long-term goals, but they are not part of the first editor
baseline.

## Initial Scope

First baseline:

- open one file or start with one untitled buffer;
- show a Microsoft Edit-like menu/status/editor surface;
- edit UTF-8 text;
- save and save-as;
- search and replace;
- undo/redo;
- basic tiled splits;
- configurable keybindings;
- safe rendering of all file content.

Deferred:

- log tailing and structured log filtering;
- `rum` integration;
- plugin execution;
- mouse-first workflows;
- broad encoding conversion;
- regex-heavy search/filter tooling.

## Encoding

Default encoding is UTF-8.

The first fallback should be deliberately simple:

- valid UTF-8 is editable;
- invalid UTF-8 is displayed safely with visible byte escapes or replacement
  markers;
- editing invalid-byte files may be read-only until a clearer byte-preserving
  edit model exists;
- ASCII-only terminals use ASCII-safe rendering, not a different file encoding
  model.

Saving must not silently corrupt unknown bytes. If a buffer was opened in a
lossy/read-only fallback mode, the UI must make that state visible and reject
normal Save/Save As unless a byte-preserving edit model exists.

Current implementation note: valid UTF-8 files open as editable buffers.
Invalid UTF-8 files open as read-only fallback buffers. Valid UTF-8 spans stay
readable, invalid bytes and non-newline controls are rendered as visible
escapes such as `\xFF`, and ordinary Save/Save As rejects those read-only
buffers to avoid corrupting the original file. This is an explicit file-text
strategy rather than best-effort encoding detection: `dun-core` decodes file
bytes as UTF-8 when possible, otherwise produces an escaped byte view tagged as
`EscapedBytes`. The CLI stores that file-text encoding with the buffer, shows it
in the status line, and treats only UTF-8 buffers as save-safe. Editable saves
write a same-directory temporary file, sync it, and atomically rename it over
the destination. Existing destination permissions are preserved, read-only
destinations are rejected before replacement, and symlink paths are resolved so
the linked target is updated without replacing the symlink itself.
When opening or saving a path, Dun reconciles its own same-directory atomic-save
temp files for that destination: stale temp files older than the destination are
removed, while newer temp files are preserved as recovery candidates and
reported through status text.
Open uses a stable-read check: after reading a file, Dun rechecks the path's
metadata and rejects the open if the file length, modification time, or Unix
device/inode changed, or if the file disappeared. This avoids editing a mixed
or stale snapshot when another process is rotating or replacing the file.

Open and save failures report the relevant path plus a normalized reason for
common cases: missing paths, directories where files are expected, missing
parent directories, permission denial, read-only destinations, and large-file
soft-limit rejection. Unstable reads report that the file changed while reading
and should be retried.

## Large Files

Use a Vim-inspired conservative policy rather than promising arbitrary large
editable files.

Principles:

- normal editable buffers have a size threshold;
- files over the threshold open with a prompt or in a protected read-only mode;
- expensive features can be disabled for large files;
- scanning operations must be cancelable;
- very long lines need display caps;
- saving should use predictable host-owned file I/O.

The exact threshold is not fixed yet. It should be configurable and informed by
testing on modest SSH/server environments.

Current implementation note: editable file loading uses the typed
`Config::limits.editable_file_soft_limit_bytes` value. The default is 16 MiB.
Files larger than the limit are rejected before becoming editable buffers, and
interactive Open reports the failure through the status line. A protected
read-only large-file mode remains a later step.

The future log viewer can use a different chunked/streaming model, but that is
not part of the first editor baseline.

## Buffer Model

The first `dun-core` text buffer uses `Vec<String>` lines.

This is deliberately simple and testable:

- the buffer always has at least one line;
- internal line separators are modeled as line boundaries, not stored inside
  each line;
- `Position.column` is a UTF-8 byte offset and must fall on a character
  boundary;
- cursor movement is UTF-8 character-boundary aware, not display-width aware;
- LF and CRLF files round trip through the buffer's `LineEnding` setting;
- edits are recorded as replace transactions for undo/redo.

This model is sufficient for the first editor baseline. If large-file testing
shows it is not acceptable, the public concepts should remain while the storage
can move to a rope or chunked representation.

## UI Theme

The default theme should feel like Microsoft Edit:

- colored top menu bar;
- colored bottom status bar;
- dark editor background;
- focused tiled window visible but not visually heavy;
- single-line Unicode borders for dialogs and tiled windows.

Required fallback:

- ASCII borders: `+`, `-`, `|`;
- 16-color palette;
- monochrome-safe style.

Optional later themes:

- `msedit`: default;
- `turbo`: more classic Turbo Vision-inspired colors;
- `dark`: restrained generic dark theme;
- `dun`: a project-owned theme tuned for long SSH sessions.

The theme layer must be independent from the editor core.

The first theme baseline is terminal-backend independent:

- `TerminalProfile` chooses UTF-8 vs ASCII and 256-color vs 16-color vs mono;
- `GlyphSet` chooses Unicode single-line borders or ASCII-safe borders;
- `Theme` carries a named `Palette` of abstract colors and attributes;
- the default profile is UTF-8 plus 256 colors;
- the default `msedit` 256-color palette follows the local Microsoft Edit
  screenshots with blue menu/status chrome, green active top-menu labels, gray
  dropdown/modal panels, and a dark blue-gray editor body;
- VT100/low-capability fallback uses ASCII glyphs and the 16-color palette;
- mono fallback uses bold/reverse attributes rather than color assumptions.

The ratatui layer should map these abstract styles into backend-specific
styles. It should not decide terminal capability policy by itself.

## Display Safety

Buffer, log, paste, config, plugin, and error text must be sanitized before
rendering.

The first sanitizer emits display segments rather than raw strings:

- printable UTF-8 text passes through in UTF-8 mode;
- ASCII mode escapes non-ASCII text as `\u{...}`;
- C0 and C1 controls are never emitted directly;
- UTF-8 mode renders C0 controls with Unicode control pictures;
- ASCII mode renders C0 controls with caret notation such as `^[` and `^G`;
- OSC payloads become visible text because `ESC` and `BEL` are replaced;
- long lines are capped without splitting UTF-8 characters.

The UI layer also sanitizes pane titles and status fields before rendering.
This keeps file names, paths, prompt/status text, and error messages from
becoming terminal control sequences.

## Keybindings

Keybindings must be configurable from the start.

Reasons:

- terminals differ;
- KVM/IPMI devices often handle modifier keys poorly;
- SSH clients vary;
- macOS terminal applications do not normally deliver Command or Fn to TUI
  programs, and Option only becomes Alt/Meta when the terminal is configured
  that way;
- users may prefer Microsoft Edit, Vim-like, or tmux-like split commands.

The initial implementation may hard-code defaults internally, but the command
model must not assume those defaults are permanent.

The first config schema is typed:

- `KeySequence` is a comma-separated list of `KeyStroke` values;
- `KeyStroke` contains a key plus Shift/Ctrl/Alt modifiers;
- `Keymap` resolves key sequences to `EditorCommand`;
- duplicate key sequences are rejected by validation;
- command ids such as `file.save` and `window.split_horizontal` round trip to
  typed commands;
- theme and terminal profile overrides live in the same typed `Config`.

The first UI integration consumes this model before ratatui is introduced. It
builds a backend-neutral frame containing grouped File/Edit/View/Help menus, a
status bar, tiled windows, resolved glyphs/theme, and sanitized buffer lines.

The first runnable terminal shell uses `ratatui 0.29` and `crossterm 0.28`.
Cargo selected these versions as Rust `1.85` compatible. The shell currently
supports:

- raw mode and alternate screen setup;
- restoration on normal exit and drop;
- environment-based terminal profile detection;
- rendering the menu, tiled window frame, sanitized body, and status line;
- rendering an active dropdown menu from the same typed command entries used
  by keybindings;
- highlighting the keyboard-selected submenu entry when a menu is opened from
  the keyboard;
- rendering current-line highlight, a persistent gutter separator, compact
  bracket-style status fields, lightweight modal prompts, and larger Open/Save
  As file dialogs with path input plus a selectable directory match list;
- crossterm key events mapped into the typed keymap;
- Alt+F/E/V/H menu mnemonics after the active keymap has had the first chance
  to consume those strokes;
- `Ctrl+Q` quit through `EditorCommand::App(Quit)`.

## Mouse

Mouse support is useful but not required for the first baseline. It is disabled
by default and enabled only through `mouse.enabled = true`.

Current mouse baseline:

- terminal mouse capture is entered and restored only when enabled;
- left-clicking a tiled window focuses that window;
- left-clicking an editor body places the cursor at the nearest valid text
  position;
- dragging in an editor body updates the current text selection;
- dragging a tiled split border resizes that split ratio;
- clicking a top-menu label opens its dropdown;
- clicking a submenu item dispatches its existing `EditorCommand`;
- clicking an Open/Save As file dialog list entry enters directories; in Open
  it opens files, while in Save As it updates the path input without saving;
- mouse wheel events scroll Open/Save As file dialog lists when the terminal
  delivers them;
- pressing `Esc` closes an open menu before normal keymap dispatch;
- clicks in the status area do nothing.

Deferred mouse features:

- right-click paste where terminal support allows it;
- clipboard integration.

Every feature must remain keyboard accessible.

Paste policy:

- right-click events are ignored until a paste path is implemented;
- paste input must come from a typed terminal event or editor command, never
  from ad hoc escape parsing in editor state code;
- pasted text is untrusted text and must enter through the same edit
  transaction path as normal insertion;
- pasted controls are buffer content only and must not be interpreted as
  terminal controls or editor commands;
- prompts may accept pasted text, but paste must not auto-submit a prompt;
- Open/Save As file dialogs use Rust-owned directory listing, `..` navigation,
  hidden-file filtering, Tab path completion, and PageUp/PageDown list
  navigation, while all actual open/save file operations still go through the
  same validated editor file I/O paths;
- external clipboard commands and OSC 52 clipboard writes are out of scope for
  the baseline.

## Log and rum Work

Do not implement the log/filter product line before `rum` is ready enough to
embed deliberately.

When `rum` is available, it can power:

- safe custom log filters;
- richer search/replace;
- internal safety-audit plugins;
- text transformation plugins.

Until then, `dun` should stay focused on the editor foundation and the tiling
workspace.
