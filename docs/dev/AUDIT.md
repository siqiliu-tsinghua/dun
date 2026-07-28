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
- in-house terminal lifecycle, sys shim, and VT core;
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

## Safe Rust Boundary

`dun`'s own Rust code is safe Rust by policy.

Required controls:

- crate roots and Rust test/support entry points use `#![forbid(unsafe_code)]`;
- `unsafe` blocks, `unsafe fn`, `unsafe impl`, `unsafe trait`, and
  `unsafe extern` are not introduced during normal development;
- if an unsafe operation ever appears unavoidable, it requires an explicit
  design decision before implementation;
- third-party dependencies with unsafe internals remain part of the trusted
  computing base and are tracked through dependency audits.

Current audit status: the repository has zero real unsafe code in `dun` crates.
The only `unsafe` word in crate source is a normal string token used by the
Rust outline parser.

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
- Startup terminal-response polling uses the in-house Unix `rustix` poll path,
  and parsing is bounded to 256 bytes total and 32 bytes per CSI under one
  500 ms deadline. Only a valid cursor-position report for column 2 or 3
  followed by a syntactically valid DA1 sentinel may change the detected
  ambiguous-width mode; malformed, incomplete, oversized, or failed probes
  fall back to Narrow without mutating editor state.

These invariants describe authority mediated by `dun`. A host-neutral protocol
can prevent a plugin from asking `dun` to exceed its role, but it cannot by
itself sandbox an external Python script, shell script, or arbitrary binary.
Only runtimes with a real pure sandbox can be treated as safe for untrusted
third-party code.

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

Host-owned destructive editor actions must confirm before dirty buffer content
is discarded. Plugins may request allowed edit intents later, but they must not
bypass this confirmation boundary.

Forbidden pattern:

1. Plugin receives filesystem capability.
2. Plugin reads or writes paths by itself.
3. Plugin returns unvalidated side effects.

## Plugin Process and Protocol Boundary

The plugin system is protocol-first. `dun` speaks the Dun Plugin Protocol over
framed stdio to an external host process, and validates all role outputs before
applying them. The protocol is documented in
[docs/plugin-protocol.md](../plugin-protocol.md).

The protocol client is required core infrastructure. The `rum` runtime is not:
future `rum` integration must be an optional host that speaks the same protocol.

Trust classes:

| Trust class | Security claim |
| --- | --- |
| `pure-sandbox` | Runtime cannot perform filesystem, process, network, terminal, environment, or editor-state side effects outside the bounded inputs and structured outputs. Future pure `rum` is expected to use this class. |
| `user-trusted-external` | External executable or script speaks the protocol, but may still have normal OS authority outside `dun`. Users must explicitly configure and trust it. |
| `unsupported-unsafe` | Unknown runtime, unknown trust class, or direct authority request. Rejected by default. |

Required controls for all protocol hosts:

- length-prefixed messages with an allocation cap before decoding;
- request ids for all asynchronous work;
- buffer or stream revisions for state-sensitive results;
- per-role input and output limits;
- timeout, cancellation, crash, malformed-frame, and oversized-output handling;
- diagnostics treated as untrusted display text;
- stale results discarded without mutating editor state;
- rejected results leave editor state unchanged except for a bounded
  diagnostic.

Additional controls for `user-trusted-external` hosts:

- launch only from explicit configuration;
- execute the configured path directly, not through a shell;
- inherit no unnecessary file descriptors;
- pass a minimal environment or explicit environment whitelist;
- document that protocol compliance does not prevent the external process from
  using ordinary OS authority outside `dun`.

## Process Boundary

`dun` supports user-explicit process actions, but they are trusted user
operations, not plugin capabilities.

Allowed user actions:

- Shell Escape suspends the TUI, restores the normal terminal, runs the user's
  shell, then resumes the TUI after the shell exits.
- Run Command executes one non-interactive shell command, captures stdout and
  stderr with bounded per-stream memory and a configured timeout
  (`limits.run_command_timeout_ms`) that kills non-terminating processes, and
  shows the result with byte counts, timeout state, and truncation state in a
  read-only buffer.

Required controls:

- these actions are reachable only through typed `AppCommand` dispatch or the
  command prompt, not through untrusted file content;
- future untrusted plugins must not receive direct process-spawn authority;
- plugin command intents must be validated and, for process execution, require
  explicit user confirmation if that capability is ever added;
- captured process output is treated as untrusted display input and passes
  through the same decoding and terminal-control sanitizer path as file text;
- command output buffers are read-only.
- command output save/copy/clear/navigation actions operate on the bounded
  read-only Command Output buffer and still go through typed editor commands
  or the command prompt;
- stdout-only and stderr-only Command Output views are derived read-only
  buffers generated from the bounded captured output; they do not expose a
  process, filesystem, terminal, or plugin capability. When focused, current
  Command Output find/copy/save actions operate on that bounded derived buffer;
- Command Output search reuses the normal read-only Find path and only changes
  cursor, selection, and search-highlight state;
- Command Output Save through the file dialog uses the same host-owned path
  listing, overwrite confirmation, path diagnostics, and atomic write path as
  other editor saves, but it does not make the output buffer editable.

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

## Buffer Display Safety

Every byte that came from a file, paste, log, config, plugin, or error payload
is untrusted for terminal rendering.

Required controls:

- do not emit C0/C1 control bytes directly to the terminal;
- never emit `ESC` or OSC sequences from buffer content;
- render controls using visible notation such as `^[`, `^G`, `^@`, and `^?`;
- neutralize invisible characters that change meaning without drawing anything:
  the bidirectional formatting characters (the Trojan Source class,
  CVE-2021-42574 — a right-to-left override makes rendered text read in an order
  the bytes do not have, so a reviewer trusting their eyes sees code that is not
  the code that will run, and a hostile file name is disguised), the zero-width
  format characters (a zero-width space inside an identifier reads as a normal
  identifier), and the Unicode tag block (encodes arbitrary ASCII in characters
  that draw nothing). These are `Cf`, not control characters, so a check for
  `char::is_control` alone does not see them. Combining marks are deliberately
  exempt: they modify a base glyph the reader can see, so they are ordinary text
  rather than a disguise;
- cap display work for very long lines;
- keep original bytes separate from display cells;
- make lossy or read-only fallback decoding visible to the user.

The sanitizer is proven by exhaustion over every Unicode scalar value in each
profile, plus an end-to-end test that poisons every attacker-influenceable text
field (buffer body, file name, window title, both status halves, plugin
indicator, and every part of a modal) and asserts against the bytes the surface
emitter actually writes — because a perfect sanitizer is worthless if a field
never reaches it.

Saving must not silently corrupt files opened through a fallback or lossy path.
Editable saves use host-owned same-directory temporary files followed by
atomic rename. The save path rejects read-only destinations before replacement
and resolves symlink paths to their target so saving does not replace a symlink
with a regular file. Dun only reconciles temp files matching its own atomic-save
name format for the same destination; stale temp files older than the
destination are removed, while newer recovery candidates are preserved and
reported instead of being silently deleted.

Opening a file must not create an editable buffer from an unstable path. Dun
checks file metadata before and after reading, rejects size or modification
changes, and on Unix rejects device/inode changes so a path replacement during
Open does not become an apparently normal buffer.
After a successful Open or Save, editable file buffers keep that verified
metadata snapshot. Normal Save compares the current path with the stored
snapshot and refuses to overwrite if the file changed, disappeared, or stopped
being the same regular file. Reload is the explicit user action for replacing
the in-memory buffer with the current disk contents.

Non-UTF-8 files must not be decoded through locale guesses or lossy conversion.
Dun's current strategy is UTF-8 first: valid UTF-8 becomes editable text, while
unknown byte streams become an escaped byte view tagged as `EscapedBytes`. That
state is visible in the UI and is not save-safe.

Current implementation:

- `dun-core::decode_file_text` is the single file-byte decoding strategy for
  editable Open. It returns either UTF-8 text or an escaped byte view.
- `dun-core::DisplaySanitizer` converts untrusted text into `DisplaySegment`
  values before UI rendering. Its mapped entry point applies mappings only
  after accepting the source character under the source-byte cap, classifies
  mapped output through the same sanitizer, and emits a logical suffix only
  for an untruncated source line.
- `dun-ui::EditorTextDisplay` is the single buffer-body coordinate seam for
  sanitized output, source-byte/display-cell conversion, wrapping, highlights,
  scrolling, and mouse hits. It uses the terminal's ambiguous-width policy and
  profile-owned glyphs; callers do not measure raw body text independently.
- `dun-ui` sanitizes pane titles and status fields before final ratatui
  rendering, so file names, paths, and status/error messages do not bypass the
  display sanitizer.
- `dun-ui` also sanitizes modal overlay titles, body lines, prompt input, and
  button text before rendering.
- Open/Save As file dialog directory entries and path input are rendered
  through the same modal overlay sanitizer; the dialog may list directories,
  including `..` and optionally hidden dotfiles, but all actual file open/save
  operations remain in `dun-cli` validated file I/O paths.
- Save As detects existing non-directory paths before dispatching a save and
  requires the same path to be submitted twice before replacement. Open and
  Save As failures remain inside the dialog for correction instead of falling
  through to a hidden state change.
- File-dialog keybindings are typed modal actions. Remapping those keys changes
  dialog navigation/editing dispatch only; it does not grant direct filesystem,
  process, network, terminal, or plugin capabilities.
- UTF-8 mode uses Unicode control pictures for C0 controls.
- ASCII mode uses caret notation and escapes non-ASCII characters as
  `\u{...}`.
- C1 controls are rendered as visible code point markers.
- Color16 terminal profiles route in-house terminal output through a bounded
  SGR rewriter so ANSI 0-15 palette colors become legacy 16-color SGR controls
  instead of 256-color-style `38;5;n` or `48;5;n` controls.
- CLI buffers track file-text encoding metadata; Save and Save As reject
  read-only or non-save-safe fallback buffers.
- CLI file buffers track read/save metadata snapshots; Save rejects stale
  snapshots instead of silently overwriting external changes.
- Long-line display work is capped by byte count without splitting a UTF-8
  character. Mapped display text reports truncation and bytes consumed in
  original source coordinates.
- A process panic hook uses the in-house lifecycle and sys shim to restore the
  terminal (mouse capture, bracketed paste, alternate screen, raw mode) before
  the release profile's `panic = "abort"` kills the process, so panics do not
  leave the user's terminal in raw mode.
- Mouse capture is disabled by default, enabled only through typed config, and
  restored on exit or runtime disable. Current mouse input can focus tiled
  windows, place the cursor, update a text selection, resize a split, or
  open a typed dropdown menu and dispatch an existing submenu `EditorCommand`.
  Editor scrollbar clicks and drags map to bounded viewport scroll requests.
  In file dialogs, mouse clicks can select visible entries, enter directories,
  including `..`, or submit an Open path through the same validated Open path.
  Mouse input does not trigger paste, arbitrary direct file operations, or
  plugin actions.
- Bracketed paste is enabled only during the TUI session and disabled during
  terminal restoration. `Event::Paste` payloads are treated as untrusted text:
  editor paste enters through the normal buffer insertion path, prompt and
  file-dialog paste is kept single-line, and confirmation prompts ignore paste.
  Pasted control bytes remain buffer content and must be neutralized at
  rendering time. Paste must not parse terminal escapes inside editor state,
  auto-submit prompts, or invoke external clipboard commands. Right-click paste
  only records a status hint and waits for the terminal to deliver bracketed
  paste data.
- Cut, Copy, and command Paste use a process-local internal clipboard. Copy can
  read selected text from editable or read-only buffers; Cut and internal Paste
  still enter through editable buffer operations and reject read-only targets.
  This internal clipboard does not grant access to the OS clipboard, external
  commands, files, plugins, processes, or the network.
- External copy is a separate user-explicit command, `edit.copy_external`, and
  is disabled unless `clipboard.osc52.enabled = true`. When enabled, it copies
  the selected text to the internal clipboard first, refuses payloads larger
  than `clipboard.osc52.max_bytes`, and emits only a host-constructed OSC 52
  clipboard-write sequence containing base64-encoded selected text.
- External paste is a separate user-explicit command,
  `edit.paste_external`, and is disabled unless
  `clipboard.osc52.allow_read = true`; the write opt-in never grants read
  access. The response parser is armed only around the host-constructed query,
  validates base64 and UTF-8 fallback text under the shared decoded-byte cap,
  and waits under one 500 ms deadline while ordinary input stays FIFO-queued.
  Missing or malformed responses fall back once to the internal clipboard; a
  valid empty response does not. The resulting text uses the normal edit and
  display-sanitizer path, including raw control bytes—there is no new
  insertion-time scrubber. Late or duplicate responses are ignored after the
  reader is disarmed. There is no platform clipboard command execution.
- Tests cover OSC title/clipboard/hyperlink payloads, CSI/SGR/clear-screen,
  DCS, graphics escapes, bracketed paste markers, `ESC`, BEL, NUL, DEL, CR,
  backspace, tabs, all C0/C1 controls, ASCII fallback, truncation, and final
  ratatui `TestBackend` rendering.
- The external SSH and low-capability terminal release matrix is documented in
  `docs/dev/terminal-compatibility-checks.md`; release candidates must record real
  external host results separately from local PTY automation.
- Local PTY automation now covers a broader terminal matrix, small
  VT100/C-locale startup, terminal escape payload files, and invalid-byte
  fallback files.

## Future rum Integration Requirements

Before adding a `rum` adapter:

- `rum` must have a release-facing host API stable enough to target.
- A minimal `dun` build must remain usable without the `rum` runtime.
- The host-neutral Dun Plugin Protocol must already have role and policy tests.
- Simple editor behavior should stay in Rust core code; `rum` should be
  reserved for high-leverage plugin workflows such as complex log filtering,
  structured extraction, advanced text transforms, and semantic plugin logic.
- The adapter must use pure-only evaluation for untrusted plugins.
- The adapter must map `rum` values into `dun` output types before validation.
- The adapter must reject unknown or malformed output.
- The adapter must enforce timeout/cancel limits.
- A memory budget strategy must be documented before enabling long-running
  plugin workflows.
- `dun-rum-host` must be a separate optional artifact rather than a dependency
  of the default 1 MiB editor executable.

## Audit Test Checklist

Add tests for:

- plugin output attempting a forbidden command;
- plugin output with oversized data;
- plugin output with malformed structure;
- log lines containing terminal escape sequences;
- huge log records;
- invalid UTF-8 handling strategy;
- buffer text containing `ESC`, OSC, BEL, NUL, DEL, CR, and backspace;
- buffer text containing the C1 single-byte CSI (`U+009B`), bidirectional
  overrides (`U+202E` and the rest), zero-width format characters, and the tag
  block — including the end-to-end check that every text field reaches the
  sanitizer, asserted on the emitted bytes;
- save behavior for lossy/fallback opened files;
- large-file threshold behavior;
- external SSH and low-capability terminal release matrix results;
- plugin timeout;
- plugin cancellation;
- plugin crash or runtime failure;
- editor state unchanged after rejected plugin output;
- file save path only reachable through `dun` core code.
