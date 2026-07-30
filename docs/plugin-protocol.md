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
- role, capability, and policy model;
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
- numbers must use the RFC 8259 grammar: no leading zeros, bare `1.`, or
  missing integer part such as `-.5`;
- object keys must be unique, and each object may contain at most 64 members;
- integer fields must be in the inclusive range 0 to 2^53 - 1
  (9,007,199,254,740,991); a present value outside that range is malformed,
  not missing;
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
family, which would have outlived the ratatui removal as the sole keeper of
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

`dun` can protect only the authority it controls. The protocol carries no
authority-request fields: a plugin cannot ask `dun` to save files, run commands,
or mutate arbitrary state, because no message expresses such a request. What a
plugin may cause `dun` to do is bounded by the capabilities its role was granted
(see Capability Model), each a typed, validated channel into a `dun`-owned
object. `dun` cannot by itself stop a Python script or external binary from
reading files or opening sockets outside the protocol; those hosts are
`user-trusted-external`, and the user's config opt-in is the consent.

Trust class is the grant gate for capabilities. A `pure-sandbox` runtime is
eligible for automatic granting of read-only and validated-write capabilities
(buffer/stream reads, overlay and surface writes). Capabilities that are
UI-invasive or that execute user-authored code in the host — window,
scratch-input/execute, menu, keybinding — require `user-trusted-external` plus an
explicit config opt-in — today, declaring the role together with
`user-trusted-external` trust in config is that opt-in. The gate is wired
live: after the handshake `dun` computes the granted set from the
config-declared roles and trust (`GrantedCapabilities::for_roles`), rejects a
host whose self-declared trust class exceeds the configured one, and refuses
ungranted channels (an `overlay-write` request without its grant fails; a menu
contribution without the `menu` grant is ignored).

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

Every configured entry runs as its own host on its own worker thread. Launch
timing is hybrid, decided per host by its grant: a host granted `menu` or
`window` launches eagerly at startup (and on `plugin load`), because only its
handshake can advertise the UI it contributes, while a highlight-only host
keeps the memory-saving lazy launch on its first job. From the command prompt,
`plugin unload [plugin-id]` gracefully shuts one host down and suppresses
relaunches to free memory (its menu contribution is removed with it);
`plugin load [plugin-id]` re-enables it — lazily for highlight-only hosts,
immediately for eager ones — and `plugin` reports every host's state. The id
is optional when exactly one host is configured.

## Message Families

The first protocol version should include these families:

| Message | Direction | Purpose |
| --- | --- | --- |
| `Hello` | host to `dun` or `dun` to host | Protocol version, host id, runtime, trust class, and advertised roles. |
| `HelloAck` | reply | Accepted protocol version, effective policy limits, and an optional `menu` contribution (honored only if the host holds the `menu` capability). |
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

## Capability Model

`role` was a permission concept in the embedded-`rum` design: a role selected a
policy group that granted capabilities to `rum` functions. That model depended
on `dun` controlling the plugin's execution, which only held while `rum` ran
in-process. Across a protocol boundary `dun` cannot grant an external process
authority over the outside world, so permission-as-role is obsolete.

What survives — and what the protocol now means by *capability* — is inward: the
permission to touch a `dun`-owned object through a typed, validated channel. The
protocol carries no authority-request fields, so `dun` grants no ambient power;
it only consumes typed output a plugin produces for the capabilities its role
holds. A **role is a named bundle of capabilities**: named because config and UI
read better as `roles = log-filter` than as a raw capability list, and because a
name is a routing and dispatch handle.

The security value did not disappear, it changed direction. In the `rum` era a
role was the *power granted to the plugin*; now it is *which typed channels `dun`
will accept from the plugin*. A `syntax-highlight` plugin can only ever emit
style spans; it structurally cannot emit an edit patch, because `dun` runs no
other validator for it.

### Capability vocabulary (v0)

Every capability exposes an object `dun` already owns; none introduces a new
subsystem. Each is bounded, validated on every message, and — for the invasive
ones — trust-gated.

| Capability | `dun` object | What the plugin may do | Bounds / trust |
| --- | --- | --- | --- |
| `buffer-read` | editor buffer | receive a bounded read snapshot (text slice, language, revision) | `max_snapshot_lines`; pure-sandbox auto |
| `stream-read` | command-output stream | receive bounded stream chunks (chunk, index, final) | frame caps; pure-sandbox auto |
| `overlay-write` | buffer rendering | return style spans over rendered text; never mutates buffer text | `max_spans`, in-range coords; pure-sandbox auto |
| `surface-write` | plugin-owned window buffer | write validated, sanitized text into its own surface | size caps; pure-sandbox auto |
| `window` | tiled window / dock | create and destroy its own windows | ≤2 per plugin + aggregate/terminal-size fallback; destroy own only; `user-trusted-external` |
| `scratch-input` | editable scratch buffer + execute | own a window backed by a `dun`-native editable buffer; the user edits it with `dun`'s own editing engine; an `execute` action submits the whole buffer text to the host as one blob (no keystroke routing). The submitted snippet runs in the host's interpreter, never in `dun`. | one scratch window; `user-trusted-external` |
| `menu` | menu bar | create one top-level menu entry; all items hang beneath it; each item dispatches a request to the host | one top-level subtree; bounded item count/depth/label length; label i18n required; `user-trusted-external` |
| `keybinding` | keymap + event loop | bind chords beneath the single leader prefix `dun` reserves for all plugins (Emacs `C-x` style) | ≤32 chords; the leader is `dun`'s, not the plugin's; first claimant wins a chord; `user-trusted-external` |

Read-only and validated-write capabilities (`buffer-read`, `stream-read`,
`overlay-write`, `surface-write`) are eligible for `pure-sandbox` auto-granting.
UI-invasive and code-executing capabilities (`window`, `scratch-input`, `menu`,
`keybinding`) require `user-trusted-external` and an explicit config opt-in.

### Ownership and namespacing

Each plugin gets a namespaced slice of `dun`-owned UI surfaces, all tagged by
`plugin_id`:

- one menu subtree (one top-level entry, every item beneath it);
- up to two tiled windows, placed by `dun` — a plugin never asks for a
  position. The first splits the focused window **side by side**, so the
  editor keeps its own column and the plugin gets the other. The second
  stacks **under the plugin's own first window**, giving `main | upper /
  lower`. Splitting the focused window again instead would put a third
  column on screen, and at 80 columns that leaves every pane too narrow to
  read;
- a chord space beneath the shared plugin leader (the prefix itself is
  `dun`'s, shared by every plugin, and never a plugin's to claim);
- at most two windows.

Ownership makes "destroy/remove only your own" automatic: each surface is a
subtree the plugin exclusively owns. On `plugin unload`, host crash, or timeout,
`dun` reaps the plugin's menu subtree, key prefix, and windows together.

### Menu label i18n

`dun`'s own UI is fully translated (English compiled in as the `&'static`
fallback, other tags as external files). Plugin-contributed labels follow the
same shape: a menu-contribution message carries labels as a locale-tag map. The
author must supply at least `en_US`; other tags are optional. `dun` resolves by
the active locale and falls back to `en_US`. A single-tag contribution is legal;
a contribution missing `en_US` is rejected.

### The reserved plugin leader

Every plugin binds under one editor-owned prefix, **`Ctrl+T`**. Hosts do not
choose it and no longer declare `leader`; the field is accepted and ignored so
older hosts keep working.

A shared prefix is what makes a plugin binding *structurally* unable to shadow
an editor key. The previous per-plugin leader could only ever be **checked**
against the live keymap — and a check is the wrong tool, because it turns a
design guarantee into a runtime race the user has to be told about.

`Ctrl+T` rather than another letter: the unbound Ctrl-letters are I, J, M, T
and U, and three of those are unreachable in a terminal — Ctrl+I arrives as
byte `0x09` (Tab), Ctrl+M as `0x0D` (Enter) and Ctrl+J as `0x0A`, all matched
before the Ctrl-letter branch of the input parser. T was already the reference
host's own pick.

Two ways a plugin still loses a binding, and they report differently on
purpose:

- **a chord another plugin claimed first.** Configuration order wins and the
  later host's whole contribution is dropped, so it is never half-bound. The
  user resolves it by editing *that plugin's* config — see below.
- **the leader itself is bound in the user's keymap.** Then no plugin can bind
  anything, and one message names the leader instead of blaming a chord.

### Where a plugin's own settings live

`dun`'s config owns the *grant* — `plugin.<id>.command`, `trust`, `roles` and
the resource limits. Those are `dun`'s decisions about a plugin, and `trust` is
a security gate.

Everything else belongs to the plugin. A host should read its own settings from
its own directory, so that installing is unpacking a folder and uninstalling is
deleting one. `dun` never reads, validates or dumps that file: a setting for a
plugin that is not installed simply does not exist as far as the editor is
concerned, and `--dump-config` keeps showing only what `dun` itself owns.

This is also the answer to "how do I change a plugin's shortcut": edit the
plugin's own config, and its host declares different chords at the next
handshake. No protocol change and no editor change is involved.

### Menu mnemonics

Mnemonics are the author's to choose. `dun` supplies no general rule for
dropdown entries because none exists: an IDE host's `Find References` and
`Format Document` both begin with `F`, and only its author knows which should
own the key. Both levels therefore take an **optional** declaration, and both
are **language-independent** — the same key works in every locale, which is
the same invariant `dun`'s own menus hold (see `docs/i18n.md`).

| | field | when omitted |
| --- | --- | --- |
| top-level | `top_mnemonic` | derived from the `en_US` label's first character, accepted only when it is an ASCII letter |
| dropdown entry | `mnemonic` on the item | **no letter shortcut**; arrows, Enter and the mouse still reach it |

A declared mnemonic is one ASCII graphic character — deliberately as wide as
`dun`'s own set, which uses `.`, `[` and `]` as well as letters
(`Visible Whitespace (.)`, `Scroll Left ([)`). Parentheses are the single
exclusion: labels render as `label (M)` and the matcher reads the last
parenthesised group, so a parenthesis mnemonic would parse ambiguously.

Rendering differs between the two levels, and the difference is load-bearing
rather than cosmetic. The top-level matcher falls back to a label's first
character, so `Log Filter` needing `L` is left alone and only gains ` (M)`
when an active translation is selected or the declared letter is not the
first one. **The entry matcher has no such fallback** — it reads only a
trailing `(M)` — so a declared entry mnemonic is *always* composed, even when
the label already starts with that letter. Omitting the suffix there would
leave a key that silently does nothing.

Collisions differ too. Built-in menus claim first, then plugin menus in
configuration order; a built-in or earlier-plugin collision rejects the whole
conflicting **subtree**, because a top-level menu nobody can open is useless.
A duplicate *within* one plugin's dropdown drops only the later entry's
shortcut: the entry and its siblings stay, since they remain reachable by
arrows, Enter and the mouse. Each subtree rejection produces a translated
status diagnostic naming the plugin and, for a collision, the mnemonic. Claims
are recomputed on refresh, so unloading an earlier host can promote a later
contribution.

A raw English label whose trailing parenthesized mnemonic contradicts the
derived first-character rule is invalid.

### Error and diagnostic surface

Snippet execution failures and any host-side error surface through a
`dun`-owned diagnostic pane, fed by the existing `Diagnostic`, `Error`, and
bounded stderr channels (`max_diagnostics`, `max_stderr_bytes`). It is
`dun`-owned on purpose: a plugin cannot suppress its own errors, and the pane
does not count against the plugin's two-window budget. The host reports
interpreter errors from a submitted snippet as bounded `Diagnostic`/`Error`
items; `dun` displays them tagged by `plugin_id`.

### Cost discipline

A capability is worth its binary weight only if it exposes an object `dun`
already maintains for a first-class feature. The v0 vocabulary is drawn entirely
from existing internals: buffers and the editing engine, the command-output
stream, the highlight overlay, the tiled window tree, the menu bar, and the
keymap. Anything that would require a new `dun` subsystem (for example, a
build/process manager an IDE-style plugin might want) is a separate, separately
budgeted decision, never something a capability silently pulls in. Each
capability's `dun`-side implementation lands with the slice that first exercises
it and is measured per batch on Debian (size deltas are non-additive under
`opt-level = "z"` + fat LTO).

## Capability Infrastructure (build order)

This stage builds the role/capability mechanism and the open capability APIs
first, without committing to any concrete product plugin. No distinctive plugin
is shipped here; each capability API is proven end to end by a minimal **fixture
host** (the pattern already used by `hosts/check-host.py` and
`crates/dun-plugin/src/bin/fixture-host.rs`), not by a polished plugin. Fixture
hosts are required: an open API with no consumer is neither testable end to end
nor measurable as real-use size.

Slices, each with a fixture host, protocol tests, and a Debian size
measurement (all four slices and the three v0 data channels — `surface-write`,
`stream-read`, `scratch-input` + execute — have landed and are measured; the
stage closed 2026-07-23, with per-chunk detail in TODO.md and
docs/dev/release-size-audit.md):

- **A — mechanism spine.** Capability vocabulary as types; role as a named
  capability bundle; config declares `roles` → capabilities; handshake
  advertises and `dun` grants (trust-gated); per-capability validation dispatch
  (generalizing the per-role `validate.rs`); `plugin_id` ownership tagging and
  unload reaping. Proven with `buffer-read`/`stream-read` in and
  `overlay-write`/`surface-write` out — the cheapest capabilities, with existing
  precedent.
- **B — windows and scratch input.** `window` lifecycle (≤2, aggregate/terminal
  fallback, own-only destroy) and `scratch-input` (a `dun`-native editable
  buffer plus the `execute` submit). Fixture host opens/closes a window, writes
  a surface, and receives a submitted snippet blob.
- **C — menu.** `menu` contribution (one top-level subtree, label i18n,
  menu-invoke dispatch, structural bounds, menu-bar width handling). Fixture
  host contributes a top-level menu whose items dispatch requests.
- **D — keybinding (landed 2026-07-18).** A `keybinding` contribution reserves
  one leader prefix keystroke and binds chords beneath it (Emacs `C-x` style).
  The host advertises it in the HelloAck (`keybinding` field: `leader` + `chords`
  of `{ key, action_id }`), honored only under the `keybinding` grant. `dun`
  parses each leader/chord string into a real keystroke and installs a
  `[leader, chord] -> PluginAction` plugin keymap consulted after the built-in
  keymap, reusing the event loop's existing pending-prefix handling
  (`pending_keys` + `has_sequence_prefix`). Collision validation drops a whole
  contribution whose leader is already a built-in binding or prefix, is claimed
  by another plugin, or fails to parse — a plugin can never shadow a built-in
  binding. The fixture host registers a `Ctrl+J` leader with a `p -> ping`
  chord.

The first real consumers have been built: Python and Lua `log-filter` hosts
under [hosts/](../hosts/), exercising the full bundle
(`{ stream-read, surface-write, window, scratch-input, menu, keybinding }`).
They were the ergonomics acceptance test this paragraph promised, and the
revision pass happened as three acceptance findings, all fixed: oversized
stream feeds are now split into bounded chunks with a FIFO pending queue and
surface accumulation (`4a841e2`); the hosts' keybinding leader moved from
`Ctrl+L` (collides with the built-in SelectLine) to `Ctrl+T` (`c560379`); and
a collision-rejected keybinding contribution now surfaces a status diagnostic
instead of disappearing silently (`959915f`). Live tmux acceptance
(`crates/dun-cli/tests/tmux_logfilter.rs`: menu injection, keybinding →
scratch, execute → surface, command → stream → surface) is green on macOS,
Debian, FreeBSD, and Solaris (2026-07-23).

**The v0 capability surface is frozen as of 2026-07-23.** The eight-capability
vocabulary above and its validator set are the v0 contract; extensions — new
capabilities, richer `LogFilter` output — are v1 candidates and wait for a
concrete consumer need.

One slice-A deferral is retired rather than built: the planned sum-typed
per-capability validator dispatch. The shipped design keys validators by
capability (`validate.rs`: `validate_spans` for `overlay-write`,
`validate_surface` for `surface-write`, `validate_stream_verdict` for
`stream-read`) and each typed request method dispatches to its validator
statically. The request methods return distinct types, so a runtime sum
dispatch would only wrap them in an enum no caller needs; static per-method
dispatch is the design of record. Revisit only if dynamically defined roles
ever appear.

## Role Bundles

A role names a bundle of capabilities. The protocol should start with bundles
that prove the whole request, validation, and application path without giving
plugins ambient authority. Two role bundles are defined today, both applied end
to end: `SyntaxHighlight` (`{ buffer-read, overlay-write }`) and `LogFilter`
(`{ stream-read, surface-write, window, scratch-input, menu, keybinding }`,
which is how a host first becomes eligible for the `menu`/`window` capabilities;
its stream/surface/scratch application paths landed with the v0 data channels
and are exercised by the real hosts under `hosts/`). The remaining rows below
are illustrative bundles whose `dun`-side implementation is demand-driven per
the build order above.

A host advertises its menu contribution in the `HelloAck` payload (`menu`
field). `dun` parses it with the `menu` capability's validator and honors it
only when the host was granted `menu` (see Capability Model); an ungranted host
that advertises a menu is ignored, and a malformed menu from a granted host
fails the handshake. Menus are therefore static, fixed at launch; a dynamic
`MenuContribute` message is deferred until a real need appears. On the editor
side each worker ships the validated contribution to the main thread with its
launch report, where it lives on the host's `PluginHost` entry (cleared on
unload, reinstalled by the relaunch handshake). `dun` resolves it (labels
against the active locale and top-level mnemonic policy under Menu label i18n)
into the menu bar after the built-in menus; an
invoked item is an `EditorCommand::PluginAction { plugin_id, action_id }`
(not user-bindable — a generic `command_id`, `plugin.action`, no
`command_from_id` round-trip; the same command a `keybinding` leader chord
produces). Dispatch routes by the action's declared kind: a `surface` action
(gated on the host holding `window`) opens or reuses the plugin's read-only
`WindowKind::PluginSurface` (≤2 windows per plugin, reaped on unload/reload,
released on a user close) and, when the host also holds `surface-write`, sends
a per-action request whose validated lines fill the surface on response; a
`scratch` action opens the plugin's editable `WindowKind::PluginScratch`, and
`execute` submits the scratch buffer's whole text to the host — both gated on
`scratch-input` — with the host's result lines filling the surface window.

| Role | Input snapshot | Allowed output |
| --- | --- | --- |
| `SyntaxHighlight` | Language hint, buffer id token, revision, visible or bounded text slice. | Style spans with bounded line/range coordinates and known style classes. |
| `LogFilter` | Stream id token, chunk index, bounded text chunk, final flag. | v0 (frozen): one keep/drop decision per input line; kept lines fill the plugin's surface window. Richer outputs — match ranges, extracted fields, tags, bounded summary data — are v1 candidates behind a concrete consumer need. Command-output section filtering (the removed built-in `output only`/section-jump family) is likewise expected to return as plugin-provided behavior under this role. |
| `TextTransform` | Selection or bounded text slice, cursor context, revision. | Proposed edit patch or replacement text requiring `dun` validation and, where appropriate, user confirmation. |
| `ConfigHelper` | Defaults, config context, environment summary without secrets. | Typed config patch against the Rust-owned `Config` model. |

`dun` implements these roles incrementally. `SyntaxHighlight` proved the whole
request, validation, and application path end to end (stale revision handling
and bounded style-span validation are easy to test); the capability
infrastructure above is what the remaining bundles are built on.

A `DocumentStructure` role was a recorded future need after the built-in
Outline pane (Markdown/INI/TOML/Rust/shell section heuristics) was removed in
the 2026-07 slimming stage. **That need is withdrawn (2026-07-28.)** The
reasoning that retired it applies to the replacement too: a navigation aid for
an unfamiliar file is wanted precisely on a cold open of a strange host, which
is when a plugin host is not installed. If document structure returns it should
be as folds computed from indentation — no type knowledge, no plugin, present
on first launch — with plugin-supplied ranges as an optional enrichment rather
than the delivery mechanism.

## Output Validation

`dun` validates all plugin output before applying it.

Required checks:

- known `plugin_id`, role, and `request_id`;
- matching buffer or stream revision;
- output type allowed by the plugin's granted capabilities;
- output size below the role limit;
- coordinates within the input snapshot;
- style ids, tags, command ids, or field names known or allowed by policy;
- no raw terminal control bytes in diagnostics or user-facing text;
- output rides only granted capability channels; there is no authority-request
  field to honor, so any output outside the plugin's capabilities is rejected.

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

## Reference Hosts

Host-neutrality is demonstrated by working `syntax-highlight` hosts in three
languages under [hosts/](../hosts/): Rust (syntect), Python (Pygments), and
dependency-free Lua (hand-written JSON codec and lexer). A language-agnostic
conformance checker, `hosts/check-host.py`, validates any host command against
the wire behavior the client relies on (envelope fields, revision echo,
character-column span bounds, style vocabulary). The `log-filter` role has
dependency-free Python and Lua hosts (`hosts/python-logfilter`,
`hosts/lua-logfilter`, with `hosts/sample-logs` fixtures) — the first real
consumers of the capability APIs, driven live by
`crates/dun-cli/tests/tmux_logfilter.rs`. These are examples outside the
editor build and its gates; the CI fixture host remains
`crates/dun-plugin/src/bin/fixture-host.rs`.
