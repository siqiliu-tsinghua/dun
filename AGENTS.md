# AGENTS

This file gives working instructions for agents and contributors changing
`dun`.

## Project Shape

`dun` is a Rust `1.85` terminal editor for Linux and macOS terminals, with
remote SSH usage as a primary scenario. It will eventually support sandboxed
plugins through `rum`, but the editor must be useful before that integration
exists.

Do not add a `rum` dependency until the `rum` release-facing host API is stable
enough for this repository to target deliberately.

Keep the default editor small and Rust-owned. Implement simple and common
editor features in `dun` itself; reserve future `rum` integration for
high-leverage plugin work that needs the language runtime's Wolfram-like
expressiveness, such as complex log filters, structured extraction, advanced
text transforms, or semantic plugin logic. A minimal build must not require the
`rum` runtime.

## Document Responsibilities

- `README.md`: user-facing overview, goals, status, and high-level architecture.
- `AGENTS.md`: contribution rules and project invariants for automated agents
  and human maintainers.
- `PLAN.md`: staged design plan and architectural sequencing.
- `TODO.md`: active task list; keep it focused on near-term work.
- `PROGRESS.md`: append-only history of completed work and decisions.
- `AUDIT.md`: security model, threat notes, invariants, and audit checklist.
- `docs/code-organization-guidelines.md`: safe Rust, file-size, module
  splitting, and directory organization rules.

When changing behavior or architecture, update the relevant document in the
same change.

## Engineering Rules

- Target Rust `1.85`.
- Keep dependencies compatible with Rust `1.85`.
- Keep `dun` source code safe Rust. All crate roots and Rust test/support
  entry points must use `#![forbid(unsafe_code)]`; do not add `unsafe` without
  an explicit design decision.
- Prefer small, typed internal APIs over stringly command plumbing.
- Keep editor state owned by Rust core code.
- Prefer Rust core implementations for simple editor features before reaching
  for a plugin/runtime boundary.
- Keep terminal rendering behind profile/theme/glyph abstractions.
- Keep log processing streaming-friendly; avoid whole-file assumptions for
  large logs.
- Add tests with the feature being implemented. For broad editor behavior,
  prefer focused core tests before UI tests.
- Do not introduce native dynamic plugin loading in the initial line.
- Follow [docs/code-organization-guidelines.md](./docs/code-organization-guidelines.md).
  Implementation files should stay under about `10k` characters when
  practical. Files over `20k` need a split assessment when touched, and files
  over `35k` are architecture debt unless an explicit exception is recorded.
  Prefer real responsibility boundaries over arbitrary size cuts.

## Plugin Security Invariants

The future plugin model is role and policy based, but `rum` itself is only a
pure evaluator inside `dun`.

Required invariants:

- all future untrusted `rum` evaluations use a pure-only sandbox policy;
- filesystem, process, network, terminal, and editor mutation capabilities are
  never exposed directly to untrusted `rum` code;
- `dun` passes plugins bounded input snapshots;
- plugins return structured data or command intents;
- `dun` validates every output against the plugin role and policy;
- `dun` performs all file operations itself;
- plugin failures must not corrupt editor state.

Roles should be modeled in `dun`, not in `rum`. Example roles include
configuration, UI description, syntax highlighting, log filtering, text
transformation, and command generation.

## Terminal Compatibility Rules

The default rendering target is UTF-8 plus 256 colors, but fallback behavior is
part of the product:

- support 16-color terminals;
- support ASCII glyph fallback;
- avoid assuming truecolor;
- avoid assuming mouse support;
- avoid assuming box drawing characters render correctly;
- avoid relying on fonts with private-use glyphs.

Use a `TerminalProfile`-style abstraction before exposing terminal capability
checks to the rest of the UI.

## Current Build Stage

The repository is a Rust workspace. Keep crate boundaries aligned with
[docs/crate-map.md](./docs/crate-map.md). The first implementation line should
establish:

- core editor types independent of `ratatui`;
- a command model;
- terminal profile detection;
- minimal TUI shell;
- tests for core buffer and command behavior.

Do not start with plugin runtime integration. Start with seams that allow a
future runtime adapter.
