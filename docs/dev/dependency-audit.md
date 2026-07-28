# Dependency and Feature Audit

This document records the current dependency shape and the feature policy for
keeping `dun` lightweight.

## Current Direct Runtime Dependencies

As of 2026-07-23, `dun-cli` has these direct in-workspace dependencies:

```text
dun-config
dun-core
dun-plugin
dun-term
dun-ui
```

The default runtime graph has three external direct dependencies:

| Dependency | Feature set | Purpose |
| --- | --- | --- |
| `rustix v0.38.44` | default features off; `std`, `stdio`, `termios`, `event` | Safe Unix terminal detection, raw mode, size queries, direct reads, and level-triggered `poll(2)`. |
| `signal-hook v0.3.18` | default features off | Flag-based `SIGWINCH` registration for bounded resize observation. |
| `unicode-width v0.2.0` | unchanged | Terminal-cell width calculation in `dun-term`. |

The regenerated workspace lockfile contains 26 packages. The terminal stack
has no external backend or general-purpose readiness layer; VT output, event
types, parsing, lifecycle, and event-loop semantics are owned by `dun`.

The current tree does not include direct or transitive packages named `tokio`,
`reqwest`, `openssl`, `syntect`, `regex`, `serde`, `tree-sitter`, `wasmtime`,
or `rum`.

## Feature Policy

Default builds should remain suitable for small remote systems:

- do not add `rum` to the default workspace until the release-facing host API
  is stable and the adapter is deliberately designed;
- keep common editor behavior in Rust crates rather than requiring a plugin
  runtime;
- prefer dependencies that can be built on Rust `1.85`;
- prefer dependencies with narrow default features or controllable backend
  features;
- avoid async/network/TLS stacks in the default editor unless a core feature
  demonstrably needs them;
- avoid syntax-highlighting or parser stacks in the default line until their
  memory and binary impact are measured;
- add a size and runtime-resource note when a dependency is expected to affect
  binary size, startup, memory, or terminal portability.

The future runtime direction remains optional or late-loaded:

- the host-neutral plugin protocol client is part of the required editor path
  and must remain small enough for the 1 MiB budget;
- `dun-plugin-rum` should be separate from the basic editor executable;
- untrusted `rum` evaluation must be pure-only;
- protocol-compatible external scripts or binaries are user-trusted unless
  separately sandboxed;
- role/policy validation belongs to `dun`;
- basic editing, file I/O, search, tiling, configuration diagnostics, shell
  escape, and one-shot command output do not require `rum`.

## Repeat Checklist

When changing dependencies or features:

1. Run `cargo tree -p dun-cli --locked --depth 1`.
2. Run `cargo tree -p dun-cli --locked --edges normal --prefix none | sort -u
   | wc -l`.
3. Inspect feature changes with `cargo tree -p dun-cli --locked -e features`.
4. Search the tree for large dependency families that change the product
   profile, especially async/network/TLS, parser/highlighter, compression,
   database, GUI, and plugin-runtime crates.
5. Rebuild the release-size baseline if the dependency is on the default
   runtime path.
6. Re-run the runtime-resource audit if the dependency affects startup, file
   loading, rendering, command execution, or plugin/runtime behavior.
