# Tiling Window Model

`dun` should use a lightweight tiling workspace, not a desktop UI.

The first screen opens as one editor buffer and should still feel close to
Microsoft Edit: menu/status chrome, one text area, line numbers, and a simple
keyboard-first flow. When the user needs more context, they can split the
workspace like `tmux`, `i3`, or `awesome`.

Reference material:

- Microsoft Edit remains the visual reference for the single-buffer surface.
- Turbo Vision remains useful as a historical command/menu/dialog reference,
  but `dun` will not use its overlapping desktop model.
- Rust Turbo Vision local reference: `reference/turbo-vision-4-rust`, commit
  `8a4c93d93efecc672a3e7ce330af35514ce1baf7`.

The reference is conceptual only. Do not import `turbo-vision` as a dependency
for the initial line, and do not copy source code into `dun`.

## Product Direction

The workspace should be small, predictable, and cheap:

- no sidebars;
- no tabs;
- no floating windows;
- no z-order;
- no window shadows;
- no desktop-style maximize/minimize system;
- no dependency on mouse support.

Instead, all user-facing views are leaves in a split tree.

## Why Tiling

The operational troubleshooting workflow needs multiple views, but not a full
desktop:

- file beside file;
- search results beside the edited buffer;
- config file beside notes;
- error list in a small lower pane;
- future raw log beside filtered log.
- help or command prompt in a temporary split.

A tiling model gives comparison without overlap, has simple rendering, works on
small terminals, and is easy to drive by keyboard over SSH.

## Core Terms

- `Window`: a visible leaf in the workspace tree. It owns a view state such as
  edit buffer, read-only file, help text, prompt, or future log/filter view.
- `Split`: an internal tree node with an axis and size ratio.
- `Workspace`: the root split tree plus the focused window id.
- `CollapsedWindow`: a window that keeps its place in the tree but renders only
  a title bar.

The word "window" means a tiled child region, not a floating desktop window.

## State Model

Recommended core state shape:

```text
Workspace
  root: LayoutNode
  focused: WindowId
  windows: Vec<WindowState>

LayoutNode
  Leaf(WindowId)
  Split {
    axis: Horizontal | Vertical
    ratio: u16              # 1..999, interpreted as first child weight
    first: Box<LayoutNode>
    second: Box<LayoutNode>
  }

WindowState
  id: WindowId
  title: String
  kind: Edit | ReadOnly | Log | Filter | Help | Prompt
  collapsed: bool
  view: ViewState
```

The core model must not depend on `ratatui`. Layout resolution should be pure
and testable.

## Rendering Model

Rendering is deterministic:

1. reserve optional top menu/command line;
2. reserve bottom status line;
3. resolve the tiling tree into rectangles;
4. draw each window frame and title;
5. draw focused window with active style;
6. draw content clipped to each window's body.

Because windows never overlap, there is no z-order or damage tracking problem.

## Collapsed Windows

There is no desktop-style minimize.

`CollapseWindow` keeps a window in the layout but renders only its title bar.
This is useful for temporarily making room while keeping context visible.

Important distinction:

- collapse saves screen space only;
- close releases the view;
- future unload/reload may release memory while preserving a reopen token.

For memory-constrained devices, `CloseWindow` and future `UnloadWindow` are more
important than collapse.

## Commands

All operations route through `EditorCommand`.

```text
WindowCommand
  SplitHorizontal
  SplitVertical
  FocusLeft
  FocusRight
  FocusUp
  FocusDown
  ResizeLeft
  ResizeRight
  ResizeUp
  ResizeDown
  Equalize
  RotateSplit
  MoveWindowLeft
  MoveWindowRight
  MoveWindowUp
  MoveWindowDown
  CollapseWindow
  ExpandWindow
  ToggleCollapse
  CloseWindow
  OnlyWindow
```

`OnlyWindow` closes or hides other panes according to a policy chosen later. It
is not a maximize state; it is a command that simplifies the current layout.

## Initial Keybinding Shape

Final keybindings belong in configuration, but the default should feel familiar
to terminal users:

- split horizontal;
- split vertical;
- move focus by direction;
- resize by direction;
- close focused window;
- collapse/expand focused window;
- equalize layout;
- open command palette or command line.

Every window operation must be keyboard accessible. Mouse resizing can be added
later as a convenience only.

## Layout Algorithms

Required pure functions:

- split focused leaf;
- close leaf and repair tree;
- find directional neighbor;
- resize nearest split edge;
- collapse and expand leaf;
- equalize all ratios;
- rotate focused split axis;
- resolve tree to rectangles;
- enforce minimum window size.

Minimum size is terminal-profile dependent. ASCII/low-end profiles should have
slightly more conservative minimums because borders consume visible columns.

## View Kinds

Initial view kinds:

- `Edit`: mutable text buffer;
- `ReadOnly`: protected text buffer;
- `Help`: static help text;
- `Prompt`: command/filter input pane.

Future view kinds:

- `Log`: large/streaming log source;
- `Filter`: filtered log output.

Tabs are not part of the model. Opening another file creates another window or
reuses the focused window based on the current command.

## Plugin Boundary

Future plugins do not manage layout directly. They may return layout-related
command intents only when their `PluginRole` and `PluginPolicy` allow it.

All actual split/close/resize operations are performed by `dun`.

## Implementation Order

1. Define `WindowId`, `Workspace`, `LayoutNode`, and `WindowState`.
2. Implement pure split-tree layout resolution.
3. Implement split focused window.
4. Implement focus movement.
5. Implement close focused window and tree repair.
6. Implement resize commands.
7. Implement collapse/expand.
8. Render single-window msedit-style frame.
9. Render multiple tiled windows.
10. Add optional mouse affordances after keyboard flow works.
11. Add log and filter views after `rum` is ready enough to embed deliberately.

## Open Questions

- Whether the top menu is always visible or replaced by a one-line command bar
  in compact mode.
- Whether `OnlyWindow` should close other views, collapse them, or switch to a
  temporary single-window layout.
- How to show many collapsed windows on very short terminals.
- Whether a filter expression should live in a dedicated `Prompt` window or in
  a command-line overlay.
