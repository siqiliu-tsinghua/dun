# Configuration

`dun` currently supports a small Rust-owned configuration file format. This is
a baseline loader, not the future `rum` configuration language.

The loader starts from `Config::default()` and applies `key = value` overrides.
Blank lines and text after `#` are ignored.

## Loading

Configuration is loaded in this order:

1. `--no-config` disables configuration loading.
2. `--config PATH` or `--config=PATH` loads the explicit path.
3. `DUN_CONFIG` loads the path named by the environment variable.
4. `$XDG_CONFIG_HOME/dun/config` is loaded if present.
5. `$HOME/.config/dun/config` is loaded if present.
6. If no config file is found, defaults are used.

Explicit or environment-provided config paths must be readable and valid. A
missing default config file is ignored.

`dun --dump-config` prints the built-in default configuration grouped into
appearance, terminal fallback, mouse, clipboard, file/display limits,
commented plugin-host examples, global command bindings, and Open/Save As
modal bindings, so it can be used as a starting point for a user config file.

## Format

Supported scalar keys:

```text
theme = dun | msedit | turbo | dark
        # dun is the default
terminal.encoding = utf8 | ascii
terminal.colors = 256 | 16 | mono
mouse.enabled = true | false
plugins.status_bar = true | false
plugins.idle_after_ms = 300000
clipboard.osc52.enabled = true | false
clipboard.osc52.allow_read = true | false
clipboard.osc52.max_bytes = 16 KiB
limits.editable_file_soft_limit_bytes = 16 MiB
limits.line_display_soft_limit_bytes = 16 KiB
limits.run_command_timeout_ms = 30000
```

Byte values accept plain bytes or binary units: `KiB`, `MiB`, and `GiB`.

Keybindings use command ids:

```text
key.app.quit = Ctrl+Q
key.app.reload_config = F5
key.app.config_diagnostics = F6
key.app.status_history = none
key.edit.find = Ctrl+W
key.edit.move_page_down = Ctrl+F
key.edit.move_page_up = Ctrl+B
key.edit.copy_external = Ctrl+X,Ctrl+C
key.edit.paste_external = Ctrl+X,Ctrl+V
key.window.split_horizontal = Ctrl+X,H
key.window.focus_left = Ctrl+X,Left
key.window.resize_right = Ctrl+X,Shift+Right
key.window.only = Ctrl+X,1
key.window.collapse = Ctrl+X,M
key.window.expand = Ctrl+X,P
```

Set a command to `none`, `disabled`, or `unbind` to remove its default binding:

```text
key.edit.find = none
```

Changing a command binding removes that command's default bindings first,
including default aliases such as `Ctrl+X,Left` and `Alt+Left` for the same
window-focus command.

If the new key sequence is still claimed by another command, config validation
fails and names both claimants — a conflict is usually a config binding landing
on a default the user never wrote down:

```text
key.edit.find = Ctrl+K
# dun: invalid keymap: `Ctrl+K` is bound to both `edit.delete_line` and
# `edit.find`; unbind one with `key.edit.delete_line = none`
```

Because each command's own defaults are dropped before the new binding is
applied, a mutual swap needs no unbinding and validates cleanly:

```text
key.edit.find = Ctrl+K          # was Ctrl+W
key.edit.delete_line = Ctrl+W   # was Ctrl+K
```

Validation runs once after the whole file is read, so the order of entries does
not matter. To take a key unilaterally, unbind its current owner first.
The runtime input path and the in-editor Help window both use the active
keymap. Custom bindings and unbound commands are reflected on startup and after
`app.reload_config` reloads the active configuration.

File-dialog modal keybindings are separate from global editor commands. They
use single key strokes because they are active only while Open/Save As dialogs
own input:

```text
key.file_dialog.submit = Enter
key.file_dialog.cancel = Esc
key.file_dialog.complete_forward = Tab
key.file_dialog.complete_backward = BackTab
key.file_dialog.toggle_hidden = Ctrl+H
key.file_dialog.move_selection_up = Up
key.file_dialog.move_selection_down = Down
key.file_dialog.page_selection_up = PageUp
key.file_dialog.page_selection_down = PageDown
key.file_dialog.move_input_left = Left
key.file_dialog.move_input_right = Right
key.file_dialog.move_input_start = Home
key.file_dialog.move_input_end = End
key.file_dialog.delete_backward = Backspace
key.file_dialog.delete_forward = Delete
```

The equivalent `file_dialog.key.NAME = KEY` spelling is also accepted. Set an
action to `none`, `disabled`, or `unbind` to remove its default binding. Modal
bindings may overlap global editor bindings because dialogs handle them before
normal editor dispatch, but duplicate file-dialog strokes are rejected.

The default window-management bindings prefer MacBook- and SSH-friendly
`Ctrl+X` chord sequences (an Emacs-style prefix) because macOS Command and Fn
are not generally delivered to terminal applications, and Option only appears
as Alt/Meta when the terminal is configured to send it. `Alt+Arrow` and
`Alt+Shift+Arrow` remain compatibility aliases where available. For the same
reason, paging and search use plain `Ctrl`+letter bindings that reach any
terminal: `Ctrl+F`/`Ctrl+B` page down/up (in addition to PageDown/PageUp and
Fn+Down/Up) and `Ctrl+W` opens Find, following the vi and nano conventions.
Unbound `Shift+Arrow` and `Shift+Home/End` strokes extend the editor selection
as a fallback after the active keymap is checked. Binding one of those strokes
to a command gives the configured command priority.
Default editor movement also includes PageUp/PageDown for visible-page
movement, using wrapped visual rows when word-wrap is active.
`Shift+PageUp/PageDown` uses the same page model for selection,
`Ctrl+Home/End` moves to the document start/end,
`Ctrl+X,[`/`Ctrl+X,]` for explicit horizontal viewport scrolling,
`Ctrl+Left/Right` for word movement, `Ctrl+Shift+Left/Right` for word
selection, `Ctrl+Backspace/Delete` for word deletion, and `Ctrl+L` for
selecting the current line. These are ordinary command bindings and can be
remapped or disabled like other `key.edit.*` entries.

Find and Replace prompts accept search prefixes before the query: `/i query`
for ignore-case, `/w query` for whole-word, and `/iw query` for both.
Config Diagnostics can also be opened at a section with commands such as
`config keymap`, `config limits`, or `diagnostics file-dialog-keymap`.
`results` opens the current Find result list and `results N` jumps to a
listed match. When the Search Results pane is focused, `n`/`p` select listed
entries, `Home`/`End` jump to the first or last listed entry, and `Enter`
jumps to the selected source location. Tab completion is available for
built-in command families such as `config` and `theme` when the cursor is at
the end of the command prompt. Ambiguous completions are shown in the status
line and can be cycled with Tab/BackTab. Path arguments complete for `open`,
`save`, `save-as`, and `reloadfile`.
The advanced `output ...` command family and the `outline` section list were
removed in the 2026-07 slimming stage (feature-triage F46 and F20).

Mouse support is optional and disabled by default. When `mouse.enabled = true`,
`dun` enables terminal mouse capture and accepts left-clicks for tiled-window
focus, editor-body cursor placement, text selection drag, split-border drag,
menu command dispatch, editor scrollbar click/drag scrolling, and mouse wheel
scrolling inside editor panes and file dialogs. Right-click paste waits for
terminal bracketed paste data when the terminal supports it.

External copy is optional and disabled by default. When
`clipboard.osc52.enabled = true`, `edit.copy_external` copies the active
selection to the internal clipboard and emits an OSC 52 clipboard write if the
UTF-8 payload is no larger than `clipboard.osc52.max_bytes`.

External paste is separately optional and disabled by default. When
`clipboard.osc52.allow_read = true`, `edit.paste_external` (default
`Ctrl+X,Ctrl+V`) sends an OSC 52 read query and waits up to 500 ms for a
response no larger than the shared decoded-byte
`clipboard.osc52.max_bytes` limit. A nonempty response is pasted through the
normal edit path; a valid empty response reports an empty terminal clipboard
without using stale internal data. A missing, malformed, or over-limit
response falls back once to the process-local clipboard. The ordinary
`edit.paste` command never queries the terminal. The write opt-in
`clipboard.osc52.enabled` never grants read permission.

This setting only authorizes `dun` to ask. The terminal or multiplexer owns the
actual clipboard gate and may filter, deny, or ignore OSC 52 reads, in which
case the bounded internal fallback applies. `dun` does not call platform
clipboard commands.

## Plugin status indicator

A plugin host is a resident process. `plugins.status_bar = true` shows every
configured host at the right edge of the status bar so an idle one can be
spotted and unloaded (`plugin unload [plugin-id]`) — useful on a small remote
box. It is off by default. Each host contributes one chip with four states,
concatenated in configuration order (`[pyg][logf idle]`):

```text
[pyg]          loaded, recently active
[pyg idle]     loaded but no activity for plugins.idle_after_ms   (highlighted)
[pyg error]    the host's last request or launch failed           (highlighted)
[pyg off]      unloaded via `plugin unload`
```

`plugins.idle_after_ms` defaults to `300000` (5 minutes); `0` disables idle
flagging entirely, leaving only the error state highlighted. The threshold is
deliberately generous: a syntax-highlight host is idle whenever you stop
typing, so a short threshold reports noise rather than signal.

Highlighted states are drawn in the active theme's `warning` color (see
[Colors](#colors)) — specifically the theme's warning pair swapped, so the
warning color becomes the chip background against a foreground the theme
already guarantees contrasts with it. Nothing is hardcoded, and the indicator
stays legible under 16-color and monochrome fallback. `dun` does not measure
the host's memory: reading that needs per-OS code, so "resident and idle" is
the proxy the indicator reports.

## Colors

The active theme (`theme = …`) supplies a full compiled-in palette. Individual
palette colors can be overridden on top of the selected theme without
redefining the whole theme; unset components keep the theme's default, so a
config can change just one background, one foreground, or one role's
attributes.

Each palette role is addressed by a stable snake_case id. Override a role with
either the granular keys or the shorthand:

```text
# Granular: any subset of fg / bg / attrs
color.editor.bg = 233
color.warning.fg = bright_red
color.title.attrs = bold, underline

# Shorthand: foreground only, or `foreground / background`
color.dirty = 208
color.warning = 196 / 0
```

A color value is a palette index `0`–`255`, an ANSI color name, or `default`
(the terminal's own default). ANSI names are `black`, `red`, `green`,
`yellow`, `blue`, `magenta`, `cyan`, `white`, and their `bright_*` variants
(`bright_black` … `bright_white`; `brightblack` and `bright-black` spellings
are also accepted). Prefer palette indexes `16`–`255` when you need a color
that is immune to the terminal's own ANSI (0–15) remapping.

`attrs` is a comma- or space-separated list of `bold`, `underline`, `reverse`,
or `none` (`none` clears all attributes and cannot be combined with others).

Overrides are applied after terminal-profile fallback, so they still take
effect when the palette degrades to 16-color or monochrome. Run
`dun --dump-config` to list every overridable role with its current default
value as commented, copy-ready `color.<role> = …` lines.

## Plugin hosts

Plugin host entries use an identifier containing only lowercase ASCII letters,
digits, and hyphens. Each host requires a command path, an explicit trust
class, and at least one role:

```text
plugin.example.command = /path/to/plugin-host
plugin.example.trust = user-trusted-external
plugin.example.roles = syntax-highlight, log-filter
plugin.example.timeout_ms = 2000
plugin.example.max_frame_bytes = 256 KiB
```

The allowed trust classes are `pure-sandbox` and `user-trusted-external`.
Use `pure-sandbox` only when the host runtime is separately known to prevent
file, process, network, terminal, environment, and editor-state side effects.
Speaking the Dun Plugin Protocol does not make an external program sandboxed;
ordinary executables and scripts must be configured as
`user-trusted-external`.

The allowed roles are `syntax-highlight`, `log-filter`, `text-transform`, and
`config-helper`. Roles are comma-separated, and repeating a role in one entry
is an error. `timeout_ms` defaults to `2000`, and `max_frame_bytes` defaults to
`256 KiB`; both must be greater than zero. Frame limits accept the same binary
byte units as other byte-valued settings.

Keys for one identifier accumulate into a single entry, and a later value for
the same key replaces the earlier value. This also applies when overlaying
configuration onto an already parsed `Config`. The command value is a path the
editor executes directly, without a shell. Parsing this section only builds
and validates typed configuration; launching is the editor's decision: each
entry runs as its own host, syntax-highlight-only hosts launch on their first
edit, and hosts whose grant carries UI contributions (`menu`/`window`, i.e. a
trusted `log-filter` entry) launch eagerly at startup so their handshake can
advertise what they contribute. The `plugin` command reports every host;
`plugin load <plugin-id>` and `plugin unload <plugin-id>` address one host,
with the id optional while only one host is configured.

## UI language

Menu text can be translated through external resource files; see
[i18n.md](./i18n.md) for the design and the safety rules. There is no config
key: the language comes from the locale environment (first nonempty of
`LC_ALL`, `LC_MESSAGES`, `LANG`), and translations load from an `i18n/`
directory next to the active config file — `~/.config/dun/i18n/zh-CN.conf`
under default discovery. The repository ships reference translations in its
top-level `i18n/` directory.

English is compiled in and needs no files. Missing translation files are
silently fine; a present-but-invalid file is rejected whole with a status
message and the editor stays English. On ASCII-only terminals translations
are skipped entirely. With `--no-config`, everything built-in applies,
including the English UI text. Menu mnemonics — `(F)`, `(N)` — always come
from the built-in English labels, so keyboard navigation is the same in
every language and translation files cannot change or break it.

## Example

```text
# ~/.config/dun/config
theme = dark
terminal.colors = 16
mouse.enabled = false
clipboard.osc52.enabled = false
clipboard.osc52.allow_read = false
clipboard.osc52.max_bytes = 16 KiB
limits.editable_file_soft_limit_bytes = 8 MiB

key.app.quit = Ctrl+Q
key.app.reload_config = F5
key.app.config_diagnostics = F6
key.edit.find = Ctrl+W
key.edit.copy_external = Ctrl+X,Ctrl+C
key.edit.paste_external = Ctrl+X,Ctrl+V
key.edit.scroll_left = Ctrl+X,[
key.edit.scroll_right = Ctrl+X,]
key.window.split_horizontal = Ctrl+X,H
key.window.split_vertical = Ctrl+X,V
key.window.focus_left = Ctrl+X,Left
key.window.resize_right = Ctrl+X,Shift+Right
key.file_dialog.toggle_hidden = F8
```

Future `rum` configuration should produce the same typed `Config` model rather
than mutating editor state directly.
