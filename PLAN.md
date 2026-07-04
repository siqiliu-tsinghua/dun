# PLAN

`dun` should grow from a reliable terminal editor core into an operations
oriented log inspection tool with a future pure-plugin layer.

## Design Principles

1. Core first, plugins later.
2. Rust owns state and side effects.
3. Plugins compute; `dun` authorizes and executes.
4. Terminal compatibility is a first-class feature.
5. Log workflows must work on large files and remote machines.
6. Architecture should allow `rum` later without depending on an unstable API
   now.

## Target Architecture

The long-term architecture can be represented as:

```text
CLI / process layer
  argument parsing, startup, exit codes

Editor core
  buffers, edits, undo/redo, search, log views

Command layer
  typed commands, validation, dispatch, keymap bindings

Terminal layer
  capability detection, input normalization, colors, glyphs

UI layer
  ratatui widgets, dialogs, menus, status, command palette

Plugin API layer
  roles, policies, input snapshots, output intents

Runtime adapters
  built-in Rust plugins now; future pure rum adapter later
```

## Phase 0: Repository Baseline

- Establish project documents.
- Record the plugin security boundary.
- Record terminal compatibility requirements.
- Keep the repository free of unstable runtime dependencies.

## Phase 1: Minimal Editor Shell

- Create a Rust `1.85` project.
- Add a minimal TUI event loop.
- Detect terminal profile conservatively.
- Render a simple editor frame with menu/status/edit area.
- Normalize keyboard input into typed commands.
- Exit cleanly and restore terminal state.

## Phase 2: Text Editing Core

- Implement text buffer data structures.
- Implement cursor movement and scrolling.
- Implement insert/delete/newline.
- Implement open/save through Rust host code.
- Implement dirty-state tracking.
- Implement focused undo/redo.
- Add tests for edit operations and file round trips.

## Phase 3: Search and Log Viewing

- Add search within buffers.
- Add read-only log view mode.
- Add streaming or chunked reading for large files.
- Add tail-follow mode if practical.
- Add built-in grep-like filtering.
- Add extracted field display for structured log views.

## Phase 4: Plugin Boundary Without rum

- Define `PluginRole`.
- Define `PluginPolicy`.
- Define `PluginRequest`.
- Define `PluginResponse`.
- Define command-intent validation.
- Add built-in Rust plugin implementations for syntax highlighting and log
  filtering.
- Add fixture runtime tests for policy enforcement.

## Phase 5: Configuration

- Define a typed configuration model.
- Support keymap, theme, terminal overrides, and plugin registration.
- Initially load configuration through Rust-owned parsing.
- Keep the model compatible with a future pure `rum` configuration evaluator.

## Phase 6: Future rum Adapter

This phase starts only after `rum` has a stable release-facing host API.

- Add a `dun-plugin-rum` adapter.
- Run untrusted code with pure-only capability policy.
- Encode plugin input snapshots into `rum`.
- Decode structured plugin output.
- Enforce time, work, and memory limits at the host boundary.
- Keep all file operations in `dun`.
- Add adversarial tests for forbidden side effects and invalid outputs.

## Phase 7: Hardening

- Crash recovery paths.
- Corrupt file handling.
- Non-UTF-8 file strategy.
- Low-capability terminal test matrix.
- Large log performance baselines.
- Security audit suite for plugin policy enforcement.
