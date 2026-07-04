# AUDIT

This document records the security model for `dun`. It is not a completed
audit; it is the baseline that future implementation and tests must preserve.

## Security Goal

Untrusted configuration or plugin code must not gain authority over the user's
machine. It may only compute over bounded inputs supplied by `dun` and return
structured data or command intents that `dun` validates before execution.

## Trusted Computing Base

Trusted:

- `dun` Rust core;
- terminal backend and `ratatui` integration;
- file I/O performed by `dun`;
- plugin policy enforcement code;
- future runtime adapter code;
- the selected Rust dependencies.

Untrusted or partially trusted:

- files opened by the user;
- log content;
- project-local configuration;
- third-party plugin source;
- terminal environment variables;
- pasted input.

## Hard Invariants

- Future `rum` execution inside `dun` is pure-only.
- `FileRead`, `FileWrite`, `Diagnostic`, or any other non-pure `rum`
  capability is not granted to untrusted plugins.
- Plugins never receive direct filesystem handles or paths with authority.
- Plugins never directly mutate buffers.
- Plugins never directly write terminal output.
- Plugins never spawn processes.
- Plugins never use network access.
- Plugins return data or command intents only.
- `dun` validates every plugin result against the plugin role and policy.
- `dun` performs all actual file operations itself.

## Role and Policy Model

Roles are owned by `dun`, not by the plugin runtime.

Expected roles:

| Role | Input | Allowed output |
| --- | --- | --- |
| `Config` | startup context and defaults | configuration patch |
| `Ui` | limited editor state snapshot | UI descriptions or UI command intents |
| `SyntaxHighlight` | text slice and language hint | style spans |
| `LogFilter` | log record or bounded window | keep/drop, extracted fields, tags |
| `TextTransform` | selection or bounded text slice | edit intents |
| `Command` | command context snapshot | approved editor command intents |

Each role needs a policy that defines:

- maximum input size;
- maximum output size;
- timeout and work limits;
- allowed output variants;
- whether output may request buffer edits;
- whether user confirmation is required.

## File I/O Boundary

All file access is performed by `dun`.

Allowed pattern:

1. `dun` opens, reads, tails, or saves a file.
2. `dun` extracts bounded text or metadata.
3. A plugin computes over that bounded input.
4. The plugin returns a result.
5. `dun` validates and applies the result if allowed.

Forbidden pattern:

1. Plugin receives filesystem capability.
2. Plugin reads or writes paths by itself.
3. Plugin returns unvalidated side effects.

## Log Filter Threats

Custom logs may contain hostile content. Treat log lines as untrusted input.

Risks:

- terminal escape injection;
- excessive line length;
- invalid UTF-8 or mixed encodings;
- adversarial regex-like workloads in filters;
- very large extracted fields;
- denial of service through repeated plugin evaluation.

Required controls:

- sanitize terminal output;
- cap per-record input size;
- cap plugin output size;
- support cancellation;
- keep filtering streaming-friendly;
- keep UI responsive under slow filters.

## Future rum Integration Requirements

Before adding a `rum` adapter:

- `rum` must have a release-facing host API stable enough to target.
- `dun` must already have plugin role and policy tests.
- The adapter must use pure-only evaluation for untrusted plugins.
- The adapter must map `rum` values into `dun` output types before validation.
- The adapter must reject unknown or malformed output.
- The adapter must enforce timeout/cancel limits.
- A memory budget strategy must be documented before enabling long-running
  plugin workflows.

## Audit Test Checklist

Add tests for:

- plugin output attempting a forbidden command;
- plugin output with oversized data;
- plugin output with malformed structure;
- log lines containing terminal escape sequences;
- huge log records;
- invalid UTF-8 handling strategy;
- plugin timeout;
- plugin cancellation;
- plugin crash or runtime failure;
- editor state unchanged after rejected plugin output;
- file save path only reachable through `dun` core code.
