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

`dun --dump-config` prints the built-in default configuration, including
global command bindings and Open/Save As modal bindings, so it can be used as a
starting point for a user config file.

## Format

Supported scalar keys:

```text
theme = msedit | turbo | dark | dun
terminal.encoding = utf8 | ascii
terminal.colors = 256 | 16 | mono
mouse.enabled = true | false
clipboard.osc52.enabled = true | false
clipboard.osc52.max_bytes = 16 KiB
limits.editable_file_soft_limit_bytes = 16 MiB
limits.line_display_soft_limit_bytes = 16 KiB
```

Byte values accept plain bytes or binary units: `KiB`, `MiB`, and `GiB`.

Keybindings use command ids:

```text
key.app.quit = Ctrl+Q
key.app.reload_config = F5
key.app.config_diagnostics = F6
key.app.command_output_summary = none
key.app.command_output_stdout = none
key.app.command_output_stderr = none
key.app.command_output_save = none
key.edit.find = Ctrl+F
key.edit.copy_external = Ctrl+W,Ctrl+C
key.window.split_horizontal = Ctrl+W,H
key.window.focus_left = Ctrl+W,Left
key.window.resize_right = Ctrl+W,Shift+Right
```

Set a command to `none`, `disabled`, or `unbind` to remove its default binding:

```text
key.edit.find = none
```

Changing a command binding removes that command's default bindings first,
including default aliases such as `Ctrl+W,Left` and `Alt+Left` for the same
window-focus command. If the new key sequence conflicts with another command,
config validation fails.
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
`Ctrl+W` sequences because macOS Command and Fn are not generally delivered to
terminal applications, and Option only appears as Alt/Meta when the terminal is
configured to send it. `Alt+Arrow` and `Alt+Shift+Arrow` remain compatibility
aliases where available.
Unbound `Shift+Arrow` and `Shift+Home/End` strokes extend the editor selection
as a fallback after the active keymap is checked. Binding one of those strokes
to a command gives the configured command priority.
Default editor movement also includes PageUp/PageDown for visible-page
movement, using wrapped visual rows when word-wrap is active.
`Shift+PageUp/PageDown` uses the same page model for selection,
`Ctrl+Home/End` moves to the document start/end,
`Ctrl+W,[`/`Ctrl+W,]` for explicit horizontal viewport scrolling,
`Ctrl+Left/Right` for word movement, `Ctrl+Shift+Left/Right` for word
selection, `Ctrl+Backspace/Delete` for word deletion, and `Ctrl+L` for
selecting the current line. These are ordinary command bindings and can be
remapped or disabled like other `key.edit.*` entries.

Find and Replace prompts accept search prefixes before the query: `/i query`
for ignore-case, `/w query` for whole-word, and `/iw query` for both.
The command prompt also accepts Command Output helpers:
`output index`, `output summary`, `output status`, `output stdout`,
`output stdout-body`, `output stderr`, `output stderr-body`,
`output truncated`, `output find QUERY`, `output next`, `output previous`,
`output clear`, `output copy`, `output save`, and `output save PATH`.
Config Diagnostics can also be opened at a section with commands such as
`config keymap`, `config limits`, or `diagnostics file-dialog-keymap`.

Mouse support is optional and disabled by default. When `mouse.enabled = true`,
`dun` enables terminal mouse capture and accepts left-clicks for tiled-window
focus, editor-body cursor placement, text selection drag, split-border drag,
menu command dispatch, editor scrollbar click/drag scrolling, and mouse wheel
scrolling inside editor panes and file dialogs. Right-click paste waits for
terminal bracketed paste data when the terminal supports it.

External copy is optional and disabled by default. When
`clipboard.osc52.enabled = true`, `edit.copy_external` copies the active
selection to the internal clipboard and emits an OSC 52 clipboard write if the
UTF-8 payload is no larger than `clipboard.osc52.max_bytes`. `dun` does not
query OSC 52 paste data or call platform clipboard commands.

## Example

```text
# ~/.config/dun/config
theme = dark
terminal.colors = 16
mouse.enabled = false
clipboard.osc52.enabled = false
clipboard.osc52.max_bytes = 16 KiB
limits.editable_file_soft_limit_bytes = 8 MiB

key.app.quit = Ctrl+Q
key.app.reload_config = F5
key.app.config_diagnostics = F6
key.app.command_output_summary = none
key.app.command_output_stdout = none
key.app.command_output_stderr = none
key.app.command_output_save = none
key.edit.find = Ctrl+F
key.edit.copy_external = Ctrl+W,Ctrl+C
key.edit.scroll_left = Ctrl+W,[
key.edit.scroll_right = Ctrl+W,]
key.window.split_horizontal = Ctrl+W,H
key.window.split_vertical = Ctrl+W,V
key.window.focus_left = Ctrl+W,Left
key.window.resize_right = Ctrl+W,Shift+Right
key.file_dialog.toggle_hidden = F8
```

Future `rum` configuration should produce the same typed `Config` model rather
than mutating editor state directly.
