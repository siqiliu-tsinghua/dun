# Plugin Protocol

This document defines the host-neutral plugin boundary for `dun`.

The plugin system is protocol-first. `dun` does not bind the plugin model to
`rum`, Python, Lua, shell scripts, native dynamic libraries, or any other
runtime. A runtime becomes usable only by speaking the Dun Plugin Protocol and
passing `dun`'s role and policy checks.

`rum` remains the intended official safe runtime later, but it is only one
future host implementation. The `dun` core plugin client is required editor
infrastructure and must stay inside the 1 MiB release budget. External plugin
hosts and future runtime packages are optional artifacts and do not count
toward `target/release/dun`.

## Size Rule

The plugin protocol client is a required runtime feature. If adding the client
pushes the audited macOS or Debian release binary above `1,048,576` bytes,
remove optional editor features in the documented trim order before cutting the
protocol client.

The required client includes:

- protocol message types;
- role and policy model;
- request id and revision handling;
- bounded input snapshots;
- output validation;
- timeout, cancellation, and crash handling;
- application of validated results for supported roles;
- fixture-host tests.

The required client excludes:

- the `rum` runtime;
- Python, Lua, JavaScript, or shell runtimes;
- native dynamic plugin loading;
- bundled third-party plugins;
- OS-level sandbox managers.

## Transport

The default transport is stdio between `dun` and a child plugin host process.

Use framed messages:

```text
u32 little-endian payload_length
UTF-8 JSON payload
```

Rules:

- stdout is reserved for framed protocol messages only;
- stderr is human-readable diagnostics only and is never parsed as protocol;
- a malformed frame terminates the host session;
- payload size is capped before allocation;
- request and response messages carry `request_id`;
- buffer-sensitive messages carry a `revision`;
- stale responses are discarded when the target revision no longer matches.

JSON is the first protocol format because it is easy to inspect and stable
enough while the protocol evolves. Binary encodings such as CBOR or MessagePack
may be considered only after the JSON protocol is stable and measured as a real
problem.

Serialization is hand-rolled, not serde (user decision 2026-07-10). The
client parses untrusted host output, so the parser is part of the audited
trusted computing base: the in-tree ~400-line JSON module with pre-parse
frame caps and an explicit depth limit is preferred over adding the serde
family (which would also outlive the ratatui removal as the sole keeper of
its proc-macro dependency tree). Revisit only if role/payload complexity
makes hand-written field mapping error-prone; switching later is a local
refactor because the wire format stays JSON.

## Trust Classes

Protocol compatibility is not a security claim. A host process can still do
anything its runtime and operating system allow outside the protocol unless it
is separately sandboxed.

| Trust class | Meaning | Default loading |
| --- | --- | --- |
| `pure-sandbox` | Runtime cannot perform file, process, network, terminal, environment, or editor-state side effects. Future pure `rum` is the intended implementation. | Eligible for automatic use after policy checks. |
| `user-trusted-external` | External executable or script speaks the protocol, but may still have ordinary OS authority outside `dun`. | Explicit config only. |
| `unsupported-unsafe` | Unknown runtime, unknown trust class, or direct authority request. | Rejected by default. |

`dun` can protect only the authority it controls. Role policy prevents a plugin
from asking `dun` to save files, mutate buffers, run commands, or write the
terminal outside its role. It cannot by itself stop a Python script or external
binary from reading files or opening sockets outside the protocol. Those hosts
must be documented as user-trusted.

## Process Launch Rules

For `user-trusted-external` fixture and development hosts:

- launch the configured executable directly, not through a shell;
- do not pass editor file descriptors other than stdin/stdout/stderr;
- pass a minimal environment or an explicit environment whitelist;
- set per-request and per-host timeouts;
- cap stdin frame size, stdout frame size, stderr capture, and diagnostics;
- kill the host process on malformed frames, timeout, cancellation failure, or
  oversized output;
- report crashes as structured plugin diagnostics without corrupting editor
  state.

These controls do not make the host sandboxed. They protect `dun`'s own state,
terminal, memory budget, and UI responsiveness.

## Message Families

The first protocol version should include these families:

| Message | Direction | Purpose |
| --- | --- | --- |
| `Hello` | host to `dun` or `dun` to host | Protocol version, host id, runtime, trust class, and advertised roles. |
| `HelloAck` | reply | Accepted protocol version and effective policy limits. |
| `LoadPlugin` | `dun` to host | Load a plugin package, source blob, or configured host entry. |
| `UnloadPlugin` | `dun` to host | Drop plugin state and caches. |
| `Request` | `dun` to host | Role-specific bounded input snapshot. |
| `Response` | host to `dun` | Role-specific structured output. |
| `Diagnostic` | host to `dun` | Bounded warning/error/status item for display or logs. |
| `CancelRequest` | `dun` to host | Ask the host to stop a request. |
| `Error` | either | Protocol or role-policy error. |
| `Shutdown` | `dun` to host | Graceful host exit. |

Every role-specific `Request` and `Response` must include:

```text
request_id
plugin_id
role
revision, when tied to an editor buffer or stream
```

## Initial Roles

The protocol should start with roles that prove the whole request, validation,
and application path without giving plugins authority.

| Role | Input snapshot | Allowed output |
| --- | --- | --- |
| `SyntaxHighlight` | Language hint, buffer id token, revision, visible or bounded text slice. | Style spans with bounded line/range coordinates and known style classes. |
| `LogFilter` | Stream id token, chunk index, bounded text chunk, final flag. | Keep/drop decisions, match ranges, extracted fields, tags, and bounded summary data. Derived stream views and command-output section filtering (the removed built-in `output only`/section-jump family) are expected to return as plugin-provided behavior under this role. |
| `TextTransform` | Selection or bounded text slice, cursor context, revision. | Proposed edit patch or replacement text requiring `dun` validation and, where appropriate, user confirmation. |
| `ConfigHelper` | Defaults, config context, environment summary without secrets. | Typed config patch against the Rust-owned `Config` model. |

`dun` may implement these roles incrementally. The first implementation should
prove at least one visible, low-risk role end to end, preferably
`SyntaxHighlight`, because stale revision handling and bounded style-span
validation are easy to test.

A `DocumentStructure` role is a recorded future need: the built-in Outline
pane (Markdown/INI/TOML/Rust/shell section heuristics) was removed in the
2026-07 slimming stage with the expectation that plugins provide section
listings through a bounded snapshot in, label/line list out contract.

## Output Validation

`dun` validates all plugin output before applying it.

Required checks:

- known `plugin_id`, role, and `request_id`;
- matching buffer or stream revision;
- output type allowed by the plugin role;
- output size below the role limit;
- coordinates within the input snapshot;
- style ids, tags, command ids, or field names known or allowed by policy;
- no raw terminal control bytes in diagnostics or user-facing text;
- no file, process, network, terminal, or editor-mutation authority request
  outside the role policy.

Rejected output must leave editor state unchanged except for a bounded
diagnostic.

## Completion Criteria

The protocol-client stage is complete when:

1. the protocol and trust model are documented here;
2. `dun` has typed protocol, role, policy, and error models;
3. `dun` can launch a configured external fixture host over framed stdio;
4. the host handshake, request, response, diagnostic, cancellation, timeout,
   crash, malformed-frame, and oversized-output paths are tested;
5. at least one role is applied end to end after validation;
6. stale revision results are discarded;
7. `cargo test --workspace`, clippy, and release smoke pass;
8. audited macOS and Debian release binaries remain within the 1 MiB budget,
   after optional-feature trimming if necessary.

`rum` integration is not part of this completion definition. A future
`dun-rum-host` must speak this same protocol and add the separate pure-sandbox
security claim.
