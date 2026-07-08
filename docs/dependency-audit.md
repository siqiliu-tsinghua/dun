# Dependency and Feature Audit

This document records the current dependency shape and the feature policy for
keeping `dun` lightweight.

## Current Direct Runtime Dependencies

As of 2026-07-08, `dun-cli` has these direct dependencies:

```text
crossterm v0.28.1
dun-config
dun-core
dun-term
dun-ui
ratatui v0.29.0
unicode-width v0.2.0
```

The normal dependency tree for `dun-cli` contains 67 unique package lines in
the current local audit. This is a coarse count, not a size guarantee.

The workspace config already disables `ratatui` default features and enables
only its `crossterm` backend:

```toml
ratatui = { version = "0.29", default-features = false, features = ["crossterm"] }
```

`crossterm` is currently pulled with its default feature set, including
`bracketed-paste`, `events`, and `windows`. This is acceptable for the current
cross-platform terminal baseline, but it is the first feature set to re-check
if the binary grows enough to justify a feature-reduction pass.

The current tree does not include direct or transitive packages named
`tokio`, `reqwest`, `openssl`, `syntect`, `regex`, `serde`, `tree-sitter`,
`wasmtime`, or `rum`. `strum` and `strum_macros` are present through terminal
backend dependencies; they are not related to the future `rum` runtime.

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

The future plugin direction remains optional or late-loaded:

- `dun-plugin-rum` should be separate from the basic editor path;
- untrusted `rum` evaluation must be pure-only;
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

