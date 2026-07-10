# brief-004 — plugin host entries in dun-config

## Goal

`dun-config` gains typed, validated plugin-host configuration so the
protocol client can later be wired from user config. When you are done the
key schema below parses into a typed model, every invalid form is a clear
config error, `--dump-config` documents the section, and the docs describe
it. dun-config must NOT gain a dependency on `dun-plugin` (the mapping to
protocol types happens later in `dun-cli`).

## Context pointers

- Read `AGENTS.md` and `docs/plugin-protocol.md` (Trust Classes, Process
  Launch Rules) first.
- Key files:
  - `crates/dun-config/src/config.rs` — the `Config` model to extend.
  - `crates/dun-config/src/parser.rs` — `apply_config_entry` key dispatch
    (see the `key.file_dialog.` prefix arms for the pattern to follow;
    `parse_byte_count` exists for byte values).
  - `crates/dun-config/src/validation.rs` — error-text mapping pattern.
  - `crates/dun-config/src/defaults.rs` — `default_config_text` sections.
  - `crates/dun-config/src/tests/parser.rs` — test style to match.
  - `docs/configuration.md` — user documentation (in scope).
- Acceptance is mechanical: the named tests decide.

## Scope

- Files you MAY modify:
  - `crates/dun-config/src/plugins.rs` (new module),
  - `crates/dun-config/src/{lib.rs,config.rs,parser.rs,validation.rs,defaults.rs}`
    (minimal integration only),
  - `crates/dun-config/src/tests/plugins.rs` (new) and
    `crates/dun-config/src/tests/mod.rs` (register it),
  - `docs/configuration.md` (add the section).
- Plus the standard MUST-NOT list from `docs/dev/codex/TEMPLATE.md`.

## Deliverable

Key schema (flat `key = value`, one plugin per `<id>`):

```text
plugin.<id>.command = /absolute/or/relative/host/path
plugin.<id>.trust = user-trusted-external | pure-sandbox
plugin.<id>.roles = syntax-highlight, log-filter
plugin.<id>.timeout_ms = 2000            # optional, default 2000, nonzero
plugin.<id>.max_frame_bytes = 256 KiB    # optional, default 256 KiB, nonzero, byte units ok
```

Typed model in `plugins.rs` (public from the crate root like the other
config types):

```rust
pub struct PluginEntry {
    pub id: String,
    pub command: std::path::PathBuf,
    pub trust: PluginTrust,               // enum { PureSandbox, UserTrustedExternal }
    pub roles: Vec<PluginRole>,           // enum { SyntaxHighlight, LogFilter, TextTransform, ConfigHelper }
    pub timeout_ms: u64,
    pub max_frame_bytes: usize,
}
```

`Config` gains `pub plugins: Vec<PluginEntry>` (default empty).

Rules (each with a line-numbered parse error or a validation error, matching
the crate's existing error style):

- `<id>`: nonempty, `[a-z0-9-]` only; anything else is a parse error on
  that line.
- Keys for the same `<id>` accumulate into one entry; a repeated same key
  for the same id overrides (normal overlay semantics).
- Validation (in `Config::validate`, so overlays are checked once,
  end-state): every entry must have a nonempty `command`, an explicit
  `trust`, at least one role; `timeout_ms` and `max_frame_bytes` nonzero;
  duplicate roles within one entry rejected. Unknown trust or role values
  are line-level parse errors with the allowed values in the message.
- `--dump-config` (`defaults.rs`): a `# Plugin hosts` section that, with no
  plugins configured (the default), emits only commented example lines so
  the dump stays parseable and round-trips.

Tests (`tests/plugins.rs`): a two-plugin happy path with defaults and
overrides; id character rejection; unknown trust / unknown role messages
name the allowed values; missing command / missing trust / missing roles
each fail validation; zero timeout and zero frame cap fail; duplicate role
fails; overlay override of a single key wins; `default_config_text()`
parses cleanly (`parse_config(&default_config_text())` — extend the
existing roundtrip test only if one already covers this, otherwise add it
here).

## dun pitfalls (read twice)

`docs/dev/codex/TEMPLATE.md` §dun pitfalls items 1, 2, 5, 7. This is
runtime code shipped in the release binary once dun-cli wires plugins:
keep parsing plain and allocation-light, no new dependencies, no regex.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dun-config
cargo test --workspace --no-fail-fast
```

Paste the `test result:` lines verbatim.

## Hard rules

All of `docs/dev/codex/TEMPLATE.md` §Hard rules apply verbatim.

## Report format (your final message)

Per `docs/dev/codex/TEMPLATE.md` §Report format.
