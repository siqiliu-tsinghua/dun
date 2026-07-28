# Microsoft Edit Reference Notes

This document records what `dun` is borrowing conceptually from Microsoft
Edit, and what it is not borrowing.

> **Written 2026-07-04, when `dun` was a plan.** Its observations of
> `microsoft/edit` still hold, and so does the visual and interaction lineage.
> Its forward-looking sentences do not: `dun` went on to retire `ratatui`
> (`858e876`) and `crossterm` (`877b7ad`) and render through its own `Surface`
> grid — ending up closer to msedit's own in-house approach than this note
> anticipated. Sentences below that state what `dun` *will* do have been
> corrected in place; the Open Questions are kept as they were asked, with
> their answers noted.

Reference repository:

- URL: <https://github.com/microsoft/edit>
- Local checkout: `reference/msedit`
- Local checkout is ignored by git via `/reference/`.
- Local UI screenshots: `reference/msedit-截图/`
- Reference commit inspected: `10cbfcc7330c894f2173611029df44ca5cb6fd77`
- License: MIT

## Use of Reference

The reference is for interaction and visual study only. Do not copy source code
into `dun`.

`microsoft/edit` currently targets a newer Rust toolchain than `dun` and uses
its own immediate-mode TUI/framebuffer stack. `dun` targets Rust `1.85` and
keeps a different product focus: Linux/macOS SSH operations, log inspection,
and a plugin boundary whose hosts are separate processes. It began on
`ratatui` and later replaced it with an in-house `Surface` renderer, arriving
at a stack much like the one described here.

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

`dun` mirrors the same top-level File/Edit/View/Help grouping in its baseline
menu model. Exact color and pixel-level menu parity remain separate reference
work because Microsoft Edit adapts colors from terminal palette probes.

## Screenshot Observations

Local screenshots captured from Microsoft Edit show several visual details that
are more specific than the source-level scan:

- The top menu bar and bottom status bar use the same blue family, while the
  editor body is a dark blue-gray.
- The active top-level menu uses a green background with dark text, not the
  same blue selection style used inside editor content.
- Dropdown menus use a gray panel, bright single-line border, left padding, and
  a right-aligned shortcut column.
- Menu item labels expose mnemonics inline with parenthesized letters, for
  example `Open...(O)`, and the mnemonic is underlined.
- The editor body has a permanent left gutter separator line. The current line
  is shown with a full-width muted gray row highlight.
- The status bar is compact and bracket-driven: document kind, line ending,
  encoding, indentation width, cursor location, and filename are visible in one
  row.
- Modal dialogs dim the inactive editor/menu/status layers. Dialog panels use a
  gray background, bright border, and title text embedded in the top border.
- Small prompt dialogs, such as Go To Line/Column, are centered and much less
  intrusive than the file picker.
- The Open dialog combines path text, file name input, a directory listing, and
  an internal scrollbar in one centered modal.
- The About dialog uses centered text and a bracket-style button such as
  `[OK]`.

These observations suggest a staged visual alignment path for `dun`:

1. Done: tune the `msedit` theme toward the screenshot colors: blue menu/status,
   dark blue-gray editor, gray dropdown/modal panels, green active top menu,
   and muted gray current-line highlight.
2. Done: add menu chrome details: active-menu color, dropdown shortcut column,
   mnemonic markers, and gray dropdown panel.
3. Done: reformat the default status bar toward bracketed document fields while
   preserving Dun-specific fields such as terminal profile and window index.
4. Done: add a lightweight modal prompt layer for Go To Line, Find, Replace,
   command entry, and confirmations.
5. Done: add an enum-driven Open/Save As file dialog baseline with path input,
   directory match list, keyboard selection, directory navigation, and Tab path
   completion while keeping lightweight prompts for smaller interactions.
6. Done: tighten Open/Save As dialog labels and list rows toward the reference
   modal style while keeping ASCII-safe file type markers and configurable
   modal shortcuts.

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

- Keep the editor core independent of the renderer. (Originally "use `ratatui`
  widgets and layout primitives"; rendering is in-house since `858e876`, and
  the independence is what mattered.)
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
- Whether first-version dialogs should be custom widgets or small reusable
  state machines. *(Answered: state machines, drawn by `dun-ui` overlay
  rendering.)*
- Whether the first text buffer should use a simple `Vec<String>` model or move
  directly to a rope/gap-buffer design. *(Answered: the simple model; the
  large-file baselines in performance-baselines.md show it holds.)*
- How much of status bar interactivity should exist before mouse support.

## Size Engineering Study (2026-07-10)

Where the ~506 KB release binary actually comes from (source:
`reference/msedit` at 2.0.0; Homebrew binary measured 506,376 bytes):

1. **Panic machinery removal (the largest lever).** Their
   `.cargo/release.toml` states "The backtrace code for panics in Rust is
   almost as large as the entire editor" and builds with
   `panic = "immediate-abort"` plus `-Zbuild-std=std,panic_abort` and
   `panic-immediate-abort`. Their README officially recommends
   `RUSTC_BOOTSTRAP=1` for stable toolchains — the same stable compiler,
   with `-Z` unlocked. This matches dun's own attribution finding (~90+ KiB
   of std is gimli/addr2line/rustc_demangle). Note the trade: with
   `panic_immediate_abort` panic hooks do not run, which would disable dun's
   terminal-restore-on-panic hook and panic messages. A middle configuration
   (build-std with the `backtrace` std feature dropped, plain
   `panic = "abort"`) may keep hooks/messages while shedding the
   symbolization stack; needs measurement.
2. **Zero-dependency UI stack.** Unix builds depend on `libc` only. They
   self-implement: a diffing framebuffer with a color-mix hash cache
   (`framebuffer.rs`), an immediate-mode TUI (`tui.rs`), a VT parser
   (`vt.rs`), and platform abstractions (`sys/`). No crossterm, no ratatui.
   For dun this informed the ratatui replacement that later shipped; owning SGR
   emission would also let dun delete the 16-color SGR rewriter layer.
3. **Profile details.** `opt-level = "s"` (they measured it well against
   size), fat LTO, `codegen-units = 1`, and notably `debug = "full"` +
   `split-debuginfo = "packed"` + `strip = "symbols"` — a fully debuggable
   release via external dSYM/dwp at no binary-size cost. dun currently ships
   no debug info at all; adopting this is free.

Not worth copying for dun's scope: the line-index-free O(n) buffer with SIMD
`memchr2` seeks and custom grapheme measurement (built for >1 GiB files; dun
caps editable files at 16 MiB), arena allocators, and dynamic ICU offload
for search. They are the right calls for msedit's goals, but dun's
`Vec<String>` model is proportionate to its limits.

msedit does not avoid `format!` (23 uses); smallness comes from the three
structural decisions above, not from ascetic code style.
