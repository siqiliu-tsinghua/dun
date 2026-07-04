# TODO

This file tracks active and near-term work. Completed decisions and finished
items belong in [PROGRESS.md](./PROGRESS.md).

## Active

- [x] Create the Rust `1.85` project structure.
- [ ] Decide whether the first implementation is a single package or a
  workspace with small crates.
- [ ] Select the terminal backend stack compatible with `ratatui` and Rust
  `1.85`.
- [ ] Add a minimal TUI startup/shutdown path that restores terminal state.
- [ ] Define `TerminalProfile`, color profile, and glyph profile.
- [ ] Define the initial `EditorCommand` enum.
- [ ] Define core buffer, cursor, and viewport types independent of UI code.
- [ ] Add tests for buffer edits.
- [ ] Add a simple file open/save path owned by Rust core code.
- [ ] Add README run instructions once code exists.

## Plugin Boundary Prep

- [ ] Define `PluginRole`.
- [ ] Define `PluginPolicy`.
- [ ] Define plugin input snapshot types.
- [ ] Define plugin output intent types.
- [ ] Add validation that rejects commands outside the plugin role.
- [ ] Add a fixture plugin runtime for tests.
- [ ] Add a built-in Rust log filter plugin.
- [ ] Keep `rum` out of Cargo dependencies until its release host API is ready.

## Terminal Compatibility

- [ ] Detect UTF-8 vs ASCII rendering mode.
- [ ] Detect or configure 256-color vs 16-color fallback.
- [ ] Add ASCII border and indicator glyphs.
- [ ] Avoid hard dependency on mouse support.
- [ ] Add manual test notes for common SSH terminals.

## Log Workflow

- [ ] Add read-only log view mode.
- [ ] Add search and filter pipeline.
- [ ] Add extracted fields display model.
- [ ] Add large-file loading strategy.
- [ ] Add tail-follow design notes before implementation.

## Deferred

- [ ] Integrate `rum` through a pure-only runtime adapter.
- [ ] Add `rum` configuration evaluation.
- [ ] Add syntax highlighting plugins backed by `rum`.
- [ ] Add memory watchdog design for long-running plugin evaluation.
- [ ] Add broad terminal compatibility test harness.
