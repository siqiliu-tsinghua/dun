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
- file-text decoding strategy for UTF-8 and escaped unknown bytes;
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
- modal keybinding models for local dialog actions;
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
- in-house terminal lifecycle, setup, panic restoration, and suspension;
- a Unix sys shim for tty acquisition, raw mode, size, direct reads, and
  level-triggered polling;
- platform-neutral VT output, owned event types, and bounded input parsing;
- a SIGWINCH-aware event reader and the runtime event loop;
- startup file opening;
- constructing config/profile/workspace/UI;
- exit codes.

The terminal module is its own backend: no external terminal backend or
readiness abstraction sits below it. Platform-neutral VT code remains separate
from the Unix sys shim so a future platform shim can reuse the core. `dun-cli`
should otherwise stay thin; product behavior belongs in library crates.

## Plugin Protocol Crates

The host-neutral plugin protocol may be added before `rum` is ready:

- `dun-plugin-api` or an equivalent module/crate for protocol messages, roles,
  policies, input snapshots, output intents, and validation.

It becomes useful after the editor baseline is working and before runtime
adapters are added. The protocol client is required core infrastructure, so it
must stay inside the default release-size budget.

Still deferred:

- `dun-plugin-rum`;
- `dun-log`.

`dun-plugin-rum` becomes useful only after `rum` has a stable release-facing
host API and can provide the pure-sandbox security claim.

The default workspace must not depend on `rum`. Keep common editor behavior in
the Rust crates above; add `dun-plugin-rum` only as a separate optional host
for workflows that justify carrying the runtime footprint.
