# AGENTS

This file gives working instructions for agents and contributors changing
`dun`.

## Project Shape

`dun` is a Rust `1.85` terminal editor for Linux and macOS terminals, with
remote SSH usage as a primary scenario. Its plugin system is protocol-first:
the editor speaks a host-neutral protocol, and future `rum` support is an
optional pure-sandbox host rather than the protocol itself.

Do not add a `rum` dependency until the `rum` release-facing host API is stable
enough for this repository to target deliberately.

Keep the default editor small and Rust-owned. Implement simple and common
editor features in `dun` itself. The plugin protocol client is required core
infrastructure and must fit inside the 1 MiB budget; future runtime hosts such
as `rum`, Python fixtures, or user-trusted external tools must remain separate
artifacts. A minimal build must not require the `rum` runtime.

## Document Responsibilities

The repository root keeps only the four documents a reader needs first;
everything written for people working *on* `dun` lives under `docs/dev/`, and
`docs/` itself is reserved for users and plugin authors.

- `README.md`: user-facing overview, goals, status, and high-level architecture.
- `AGENTS.md`: contribution rules and project invariants for automated agents
  and human maintainers.
- `TODO.md`: active task list; keep it focused on near-term work.
- `CLAUDE.md`: session orientation and the live working plan.
- `docs/plugin-protocol.md`: host-neutral plugin protocol, trust classes,
  role-policy boundary, transport, and completion criteria.
- `docs/i18n.md`: UI text translation design — compiled English defaults,
  external per-language resource files, mnemonic and sanitizer invariants.
- `docs/dev/PLAN.md`: staged design plan and architectural sequencing.
- `docs/dev/PROGRESS.md`: append-only history of completed work and decisions.
- `docs/dev/AUDIT.md`: security model, threat notes, invariants, and audit
  checklist.
- `docs/dev/code-organization-guidelines.md`: safe Rust, file-size, module
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
- Treat the host-neutral plugin protocol client as a required runtime feature.
  If it pushes the audited release binary over budget, trim optional editor
  features before cutting protocol-client functionality.
- Keep the final v0.1 release executable within the hard runtime budget:
  the `scripts/release-build.sh` binary must be no larger than `1,048,576`
  bytes on both audited macOS and Debian builds. That script (checked-in
  `[profile.release]` plus the 2026-07-10 build-std contract) is the budget
  build; plain `cargo build --release` is a dev build, not a budget claim.
  If the gate fails, do not add runtime features; follow
  [docs/dev/feature-budget.md](./docs/dev/feature-budget.md) and
  [docs/dev/feature-triage.md](./docs/dev/feature-triage.md).
- Keep the repository's shell scripts POSIX `sh`: they run on all four
  supported platforms, where `bash` may be absent (FreeBSD base) and
  `/bin/sh` may be ksh93 without `local` (Solaris). No GNU-only flags, and
  no `tar -z`.
- Configuration and translation catalogs load in two layers: the installed
  one derived from the running binary (`<bin>/../share/dun`) and the user's
  own over it, key by key. A broken *installed* file reports and steps
  aside; a broken *user* file is a startup error. Keep that asymmetry — one
  machine-wide mistake must not stop every user of that machine.
- Keep terminal rendering behind profile/theme/glyph abstractions.
- Keep log processing streaming-friendly; avoid whole-file assumptions for
  large logs.
- Add tests with the feature being implemented. For broad editor behavior,
  prefer focused core tests before UI tests.
- Prove a test load-bearing before trusting it. A test that passes against a
  broken implementation protects nothing, and this repo has shipped two such
  tests that only mutation caught: a panic-restore test that `TerminalGuard::drop`
  satisfied without the hook, and a sanitizer test that asked the implementation
  to mark its own homework. Whenever you add or change a test that guards a
  correctness or safety invariant — anything under [AUDIT.md](./docs/dev/AUDIT.md), plus
  terminal restore, atomic save, dirty confirmation, and display sanitization —
  break the code it covers on purpose, confirm the test fails and names the
  fault, then restore. State that you did this. Two failure shapes to watch:
  a test whose oracle calls the same predicate the implementation uses (weaken
  one and both move together), and a test that asserts an escape sequence is
  *present* while some other mechanism, not the code under test, is what put it
  there.
- Prefer an oracle that is independent of the implementation: hardcode the
  expected bytes/strings in the test rather than deriving them from a function
  the implementation also calls. When an invariant is exhaustible (a per-item
  check over a small closed input space, e.g. every Unicode scalar), exhaust it
  rather than sampling — it is stronger than a property test and needs no
  dependency.
- Verify against reality before reasoning from memory: drive the real binary
  (tmux/PTY), read the actual source of a dependency, or measure the actual
  value, rather than asserting how something behaves. Terminal, rendering, and
  Unicode behavior in particular have repeatedly turned out otherwise than
  assumed.
- Do not introduce native dynamic plugin loading in the initial line.
- Follow [docs/dev/code-organization-guidelines.md](./docs/dev/code-organization-guidelines.md).
  Implementation files should stay under about `10k` characters when
  practical. Files over `20k` need a split assessment when touched, and files
  over `35k` are architecture debt unless an explicit exception is recorded.
  Prefer real responsibility boundaries over arbitrary size cuts.

## Plugin Security Invariants

The plugin model is role and policy based. `dun` owns the protocol, roles,
policies, validation, and application of results. `rum` is only a future
pure-sandbox host for the protocol.

Required invariants:

- all future untrusted `rum` evaluations use a pure-only sandbox policy;
- host-neutral plugin protocol messages are bounded, versioned, and validated
  by `dun`;
- filesystem, process, network, terminal, and editor mutation capabilities are
  never exposed directly to untrusted plugin code through `dun`;
- `dun` passes plugins bounded input snapshots;
- plugins return structured data or command intents;
- `dun` validates every output against the plugin role and policy;
- `dun` performs all file operations itself;
- plugin failures must not corrupt editor state.

Roles should be modeled in `dun`, not in `rum`. Example roles include
configuration, UI description, syntax highlighting, log filtering, text
transformation, and command generation.

Protocol-compatible external scripts or binaries are not automatically safe.
Unless they run in a real pure sandbox, treat them as user-trusted external
tools that must be enabled explicitly.

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

## Crate Boundaries

The repository is a Rust workspace of six crates. Keep the boundaries aligned
with [docs/dev/crate-map.md](./docs/dev/crate-map.md); the direction of
dependency is the invariant, not the file layout:

- `dun-core` owns editor state and the typed command model, and knows nothing
  about terminals, rendering, or configuration;
- `dun-term` owns capability profiles, glyph fallback, and themes;
- `dun-config` owns the typed config, keymap, and command-id parsing;
- `dun-ui` owns the backend-neutral frame model and renders it onto the
  in-house `Surface` grid — there is no third-party TUI framework in the tree;
- `dun-plugin` owns the protocol client and its validators;
- `dun-cli` owns process lifecycle, terminal I/O, the event loop, and command
  application.

Rendering and terminal I/O are both in-house: `ratatui` was retired at
`858e876` and `crossterm` at `877b7ad`. Do not reintroduce a framework
dependency to solve a rendering or input problem without a size measurement and
an explicit decision.
