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

Host-owned destructive editor actions must confirm before dirty buffer content
is discarded. Plugins may request allowed edit intents later, but they must not
bypass this confirmation boundary.

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

## Buffer Display Safety

Every byte that came from a file, paste, log, config, plugin, or error payload
is untrusted for terminal rendering.

Required controls:

- do not emit C0/C1 control bytes directly to the terminal;
- never emit `ESC` or OSC sequences from buffer content;
- render controls using visible notation such as `^[`, `^G`, `^@`, and `^?`;
- cap display work for very long lines;
- keep original bytes separate from display cells;
- make lossy or read-only fallback decoding visible to the user.

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

Non-UTF-8 files must not be decoded through locale guesses or lossy conversion.
Dun's current strategy is UTF-8 first: valid UTF-8 becomes editable text, while
unknown byte streams become an escaped byte view tagged as `EscapedBytes`. That
state is visible in the UI and is not save-safe.

Current implementation:

- `dun-core::decode_file_text` is the single file-byte decoding strategy for
  editable Open. It returns either UTF-8 text or an escaped byte view.
- `dun-core::DisplaySanitizer` converts untrusted text into `DisplaySegment`
  values before UI rendering.
- `dun-ui` sanitizes pane titles and status fields before final ratatui
  rendering, so file names, paths, and status/error messages do not bypass the
  display sanitizer.
- `dun-ui` also sanitizes modal overlay titles, body lines, prompt input, and
  button text before rendering.
- Open/Save As file dialog directory entries and path input are rendered
  through the same modal overlay sanitizer; the dialog may list directories,
  including `..` and optionally hidden dotfiles, but all actual file open/save
  operations remain in `dun-cli` validated file I/O paths.
- File-dialog keybindings are typed modal actions. Remapping those keys changes
  dialog navigation/editing dispatch only; it does not grant direct filesystem,
  process, network, terminal, or plugin capabilities.
- UTF-8 mode uses Unicode control pictures for C0 controls.
- ASCII mode uses caret notation and escapes non-ASCII characters as
  `\u{...}`.
- C1 controls are rendered as visible code point markers.
- CLI buffers track file-text encoding metadata; Save and Save As reject
  read-only or non-save-safe fallback buffers.
- Long-line display work is capped by byte count without splitting a UTF-8
  character.
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
  auto-submit prompts, or use OSC 52/external clipboard commands in the
  baseline. Right-click paste only records a status hint and waits for the
  terminal to deliver bracketed paste data.
- Cut, Copy, and command Paste use a process-local internal clipboard. Copy can
  read selected text from editable or read-only buffers; Cut and internal Paste
  still enter through editable buffer operations and reject read-only targets.
  This internal clipboard does not grant access to the OS clipboard, OSC 52,
  external commands, files, plugins, processes, or the network.
- Tests cover OSC title/clipboard/hyperlink payloads, CSI/SGR/clear-screen,
  DCS, graphics escapes, bracketed paste markers, `ESC`, BEL, NUL, DEL, CR,
  backspace, tabs, all C0/C1 controls, ASCII fallback, truncation, and final
  ratatui `TestBackend` rendering.
- The external SSH and low-capability terminal release matrix is documented in
  `docs/terminal-compatibility-checks.md`; release candidates must record real
  external host results separately from local PTY automation.
- Local PTY automation now covers a broader terminal matrix, small
  VT100/C-locale startup, terminal escape payload files, and invalid-byte
  fallback files.

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
- Log/filter workflows should wait until this boundary can be used deliberately.

## Audit Test Checklist

Add tests for:

- plugin output attempting a forbidden command;
- plugin output with oversized data;
- plugin output with malformed structure;
- log lines containing terminal escape sequences;
- huge log records;
- invalid UTF-8 handling strategy;
- buffer text containing `ESC`, OSC, BEL, NUL, DEL, CR, and backspace;
- save behavior for lossy/fallback opened files;
- large-file threshold behavior;
- external SSH and low-capability terminal release matrix results;
- plugin timeout;
- plugin cancellation;
- plugin crash or runtime failure;
- editor state unchanged after rejected plugin output;
- file save path only reachable through `dun` core code.
