# Crate Map

`dun` uses a small workspace so the core editor model stays testable without a
terminal and UI work does not leak into file/buffer logic.

Current crates:

```text
dun-cli
  -> dun-config
  -> dun-core
  -> dun-term
  -> dun-ui

dun-ui
  -> dun-config
  -> dun-core
  -> dun-term

dun-config
  -> dun-core
  -> dun-term

dun-core
  no internal dun dependencies

dun-term
  no internal dun dependencies
```

## `dun-core`

Owns terminal-independent editor state and operations.

Responsibilities:

- buffer ids and buffer metadata;
- tiled workspace ids and layout tree;
- typed editor commands;
- edit transactions and undo/redo later;
- search/replace core later;
- pure layout and state transition tests.

Must not depend on:

- `ratatui`;
- terminal backends;
- config file parsing;
- future `rum` adapter;
- filesystem side effects beyond explicitly designed file model helpers.

## `dun-term`

Owns terminal profile, glyphs, and theme primitives.

Responsibilities:

- `TerminalProfile`;
- UTF-8 vs ASCII rendering profile;
- 256-color, 16-color, and mono color profile;
- Microsoft Edit-style single-line glyphs;
- ASCII fallback glyphs;
- theme identifiers and theme data.

This crate should stay lightweight. It may later contain environment probing,
but terminal raw-mode setup belongs closer to `dun-cli`/`dun-ui`.

## `dun-ui`

Owns ratatui-facing rendering and interaction shell.

Responsibilities:

- menu/status/editor shell;
- tiled workspace rendering;
- dialogs;
- command palette or command line;
- translating resolved editor state into widgets.

Rules:

- rendering does not perform file I/O;
- rendering receives sanitized display cells/spans, not raw untrusted bytes;
- UI actions emit `EditorCommand` or dialog results.

`ratatui` should be added here when the first UI loop is implemented.

## `dun-config`

Owns typed configuration.

Responsibilities:

- keybinding model;
- theme selection;
- terminal override settings;
- resource/large-file limits;
- command id mapping for typed keybindings;
- future config validation.

The first config format can be Rust defaults. Future `rum` config evaluation
must produce this typed model rather than mutating runtime state directly.

## `dun-cli`

Owns process-level entry.

Responsibilities:

- argument parsing;
- terminal setup and restoration;
- startup file opening;
- constructing config/profile/workspace/UI;
- exit codes.

`dun-cli` should stay thin. Product behavior belongs in library crates.

## Deferred Crates

Do not create these yet:

- `dun-plugin-api`;
- `dun-plugin-rum`;
- `dun-log`.

They become useful only after the editor baseline is working and `rum` has a
stable release-facing host API.
