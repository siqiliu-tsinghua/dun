# Microsoft Edit Reference Notes

This document records what `dun` is borrowing conceptually from Microsoft
Edit, and what it is not borrowing.

Reference repository:

- URL: <https://github.com/microsoft/edit>
- Local checkout: `reference/msedit`
- Local checkout is ignored by git via `/reference/`.
- Reference commit inspected: `10cbfcc7330c894f2173611029df44ca5cb6fd77`
- License: MIT

## Use of Reference

The reference is for interaction and visual study only. Do not copy source code
into `dun`.

`microsoft/edit` currently targets a newer Rust toolchain than `dun` and uses
its own immediate-mode TUI/framebuffer stack. `dun` will use `ratatui`, target
Rust `1.85`, and keep a different product focus: Linux/macOS SSH operations,
log inspection, and a future pure `rum` plugin boundary.

## Reference Tests

`crates/dun-cli/tests/msedit_reference.rs` provides the current lightweight
reference baseline:

- if a local `edit` binary is on `PATH`, `edit --help` is checked for stable
  CLI reference markers such as `--help`, `--version`, and
  `FILE[:LINE[:COLUMN]]`;
- `dun --help` is checked against Dun's own CLI contract;
- `reference/msedit` source is statically scanned for stable menu, status bar,
  color, and terminal setup markers.

The tests intentionally do not copy or snapshot Microsoft Edit source. They
also do not run a live Microsoft Edit TUI differential by default: the
reference app queries terminal palette, cursor position, and device attributes
during startup, so a reliable live comparison needs a terminal-response-aware
probe harness rather than the minimal `expect(1)` runner used for Dun's local
smoke tests.

Static observations currently covered by tests:

- `draw_menubar.rs` defines top-level `File`, `Edit`, `View`, and `Help`
  menus with mnemonic letters and shortcuts such as `Ctrl+N`, `Ctrl+O`,
  `Ctrl+S`, `Ctrl+Q`, `Ctrl+Z`, `Ctrl+Y`, `Ctrl+F`, `Ctrl+R`, `Ctrl+G`,
  `Ctrl+P`, and `Alt+Z`;
- `draw_statusbar.rs` exposes language, newline style, encoding, indentation,
  cursor location, dirty marker, and filename fields;
- `main.rs` derives the menu/status background by blending terminal background
  with bright blue, computes contrast for foreground text, configures floater
  and modal colors, and performs terminal setup through alternate screen,
  mouse/bracketed-paste/meta modes, OSC palette queries, foreground/background
  queries, cursor-position probing, and device-attributes probing.

## Useful Observations

### Screen Structure

The visible frame is simple and dense:

- top menu bar;
- central editor area;
- line-number gutter;
- right-side scrollbar;
- bottom status bar;
- modal dialogs layered over the editor.

This is a good visual baseline for `dun`, but not a sufficient workspace model.
`dun` needs lightweight tiled child windows for operations work. See
[window-management.md](./window-management.md).

### State Model

The reference app keeps application state in a central state object with many
modal intent flags such as search, file picker, save, close, go-to-file, and
about dialogs.

For `dun`, use the same general idea but make it more typed:

- `AppState` owns global UI state.
- `WorkspaceState` owns documents and, later, log views.
- `DialogState` is an enum instead of many independent booleans.
- `PendingAction` captures host-owned side effects such as save/close.
- `EditorCommand` is the normalized action boundary for keymaps, menus, and
  future plugins.

### Draw Order

The reference draw path is effectively:

1. menu bar;
2. editor;
3. status bar;
4. modal/pending overlays;
5. remaining global shortcuts.

For `dun`, keep a similar deterministic order:

1. collect input;
2. translate input to `EditorCommand`;
3. apply command to state;
4. render menu/editor/status/dialog layers;
5. execute approved host side effects outside rendering.

Rendering should not directly perform file I/O.

### Menu Model

The reference menu groups are small:

- File;
- Edit;
- View;
- Help.

For `dun`, start with:

- File: new/open/save/save-as/close/quit;
- Edit: undo/redo/cut/copy/paste/select-all/find/replace;
- View: word wrap, go to line, go to buffer;
- Filter: deferred until the future log/rum workflow.
- Help: about and key reference.

The extra `Filter` group is not part of the first baseline.

### Status Bar

The reference status bar carries high-value, compact document state:

- newline style;
- encoding;
- indentation settings;
- cursor location;
- dirty marker;
- filename.

For `dun`, status bar fields should include:

- mode: edit or read-only, with log/filter modes later;
- encoding/rendering profile;
- newline style when known;
- indentation settings for editable buffers;
- cursor position;
- dirty marker;
- current file or stream label.

### Dialogs

The reference uses centered modal dialogs for about, unsaved changes, file
picker, language, encoding, and go-to flows.

For `dun`, dialogs should be enum-driven:

- `DialogState::OpenFile`;
- `DialogState::SaveAs`;
- `DialogState::Find`;
- `DialogState::Replace`;
- `DialogState::GoToLine`;
- `DialogState::GoToBuffer`;
- `DialogState::UnsavedChanges`;
- `DialogState::About`;
- `DialogState::ErrorLog`.

Each dialog should return a typed result or command; it should not mutate
editor state by hidden side effect.

### Color and Fallback

The reference queries terminal colors and derives menu/modal colors from the
terminal palette. `dun` should keep a simpler first version:

- default `msedit`-like 256-color theme;
- 16-color fallback theme;
- monochrome fallback;
- ASCII glyph fallback.

A later version may query terminal color palette if it can be done without
hurting SSH compatibility.

`dun` should use Microsoft Edit-style single-line borders by default:

```text
┌────┐
│    │
└────┘
```

ASCII fallback uses:

```text
+----+
|    |
+----+
```

Turbo Vision-style double-line frames can be an optional theme later, not the
default.

### Input

The reference normalizes keyboard and mouse input into its own input types and
supports VS Code-like shortcuts for common editing tasks.

For `dun`, normalize crossterm events into:

- `KeyStroke`;
- `MouseEvent` only when enabled by terminal profile;
- `EditorCommand`;
- `TextInput`.

Do not make mouse support required for any workflow.

## Initial Dun UI Model

Recommended first model:

```text
App
  terminal_profile: TerminalProfile
  theme: Theme
  glyphs: GlyphSet
  workspace: Workspace
  focus: FocusTarget
  dialog: Option<DialogState>
  command_log: VecDeque<StatusMessage>

Workspace
  root: LayoutNode
  focused: WindowId
  windows: Vec<WindowState>

ViewState
  EditBuffer(BufferView)
  ReadOnlyBuffer(BufferView)
  Log(LogView)              # future

LayoutNode
  Leaf(WindowId)
  Split { axis, ratio, first, second }

EditorCommand
  File(...)
  Edit(...)
  View(...)
  Search(...)
  Filter(...)               # future
  Dialog(...)
  App(...)
```

## Implementation Constraints

- Use `ratatui` widgets and layout primitives.
- Keep editor core independent of `ratatui`.
- Keep file I/O outside rendering.
- Keep command execution testable without a terminal.
- Keep keybindings configurable.
- Keep mouse support optional.
- Keep all future plugin outputs behind `EditorCommand` or narrower typed
  result types.
- Maintain Rust `1.85` compatibility.

## Open Questions

- Whether future log/filter features deserve a dedicated `Filter` top-level
  menu or should stay under `View`/`Search`.
- Whether first-version dialogs should be custom ratatui widgets or small
  reusable state machines.
- Whether the first text buffer should use a simple `Vec<String>` model or move
  directly to a rope/gap-buffer design.
- How much of status bar interactivity should exist before mouse support.
