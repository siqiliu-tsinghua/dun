# Brief 053 — OSC 52 read step 2: config, command, 500 ms wait, fallback, PTY, docs

Implementation brief. **Step 2 of the accepted plan for brief 051** (OSC 52
clipboard read; step 1 = brief 052, the parser/decoder/reader seam, landed at
`110aa08`). This step wires the user-facing feature: a config opt-in, a
command + keybinding + menu, the typed query action, the 500 ms
synchronous-feel wait, and the fallback — all on the full trail with i18n and
docs. The user confirmed the decisions below on 2026-07-27.

## Goal

`edit.paste_external` (default `Ctrl+X,Ctrl+V`, Edit-menu `Paste External`),
gated on `clipboard.osc52.allow_read`, sends the OSC 52 read query (armed via
step 1's `EventReader::begin_osc52_query`), waits up to 500 ms for the
`Event::Osc52Clipboard` response, pastes it at the issuing cursor through the
normal edit path, and on no/empty/malformed response falls back to the
internal clipboard — with exact status messages, translated across all ten
catalogs.

## Fixed decisions (user-confirmed 2026-07-27 — do not revisit)

- **Config:** add `Osc52Config::allow_read: bool`, default `false`; keep
  `enabled` as the write opt-in; reads share `max_bytes` (decoded-byte limit).
  Enabling write must NOT authorize reads.
- **Command/trigger:** `EditCommand::PasteExternal` / id `edit.paste_external`;
  default binding **`Ctrl+X,Ctrl+V`** (verified free: `Ctrl+X,V` is vertical
  split, `Ctrl+X,Ctrl+C` is external copy — this is the consistent
  counterpart); Edit-menu entry `Paste External (<mnemonic>)` placed next to
  the existing `Copy External`; internal `edit.paste` stays unchanged and
  never queries the terminal.
- **Timing:** synchronous-feel bounded wait, `OSC52_READ_TIMEOUT = 500 ms`
  (matches the startup probe), absolute deadline starting after the query
  flush. Poll for the response in slices of `min(250 ms, remaining)`, leaving
  ordinary key/mouse/paste/resize events queued (FIFO) via step 1's
  `EventReader::next_osc52_response`. NOT asynchronous.
- **Empty response** = a valid empty host clipboard: report it and do NOT paste
  stale internal clipboard.
- **Do NOT split `editing.rs`** into `app/clipboard.rs` in this brief — keep
  the new `paste_external` beside the existing clipboard methods. (A separate
  mechanical split may follow if the file crosses the size guideline; not
  here.)

## Exact change

### Config (`dun-config`)
1. `crates/dun-config/src/config.rs` — add `allow_read: bool` to `Osc52Config`,
   default `false` (in the `Default` impl).
2. `crates/dun-config/src/parser.rs` — parse `clipboard.osc52.allow_read`
   (mirror the `clipboard.osc52.enabled` arm ~`:151-160`).
3. `crates/dun-config/src/defaults.rs` — emit `allow_read` in `--dump-config`
   beside `enabled`/`max_bytes` (~`:37-45`).
4. Config tests (`crates/dun-config/src/tests/parser.rs`, `tests/config.rs`) —
   parse + default-dump coverage for the new field, independent of `enabled`.

### Core command + ids + keymap (`dun-core`, `dun-config`)
5. `crates/dun-core/src/command.rs` — add `EditCommand::PasteExternal`.
6. `crates/dun-config/src/commands.rs` — id `edit.paste_external` in the id
   list and BOTH mappings; bump the exhaustive edit-variant count in
   `crates/dun-config/src/tests/keys.rs` (currently 48 → 49) and add the
   round-trip.
7. `crates/dun-config/src/keys/keymap.rs` — default `Ctrl+X,Ctrl+V` →
   `EditorCommand::Edit(EditCommand::PasteExternal)`.

### Dispatch + query + wait + fallback (`dun-cli`)
8. `crates/dun-cli/src/app/editing.rs` — dispatch `PasteExternal` to a new
   `paste_external()` beside the clipboard methods. It: checks
   `clipboard.osc52.allow_read`; if disabled, falls straight back to the
   internal clipboard with the disabled status; if enabled, sets the waiting
   status and requests `RuntimeAction::QueryOsc52Clipboard { max_bytes }`.
   Refactor the existing internal paste (`paste_internal_clipboard`, ~`:589`)
   to return a small outcome enum so both the normal path and the fallback can
   choose statuses without duplicating insertion logic.
9. `crates/dun-cli/src/terminal/action.rs` — add
   `RuntimeAction::QueryOsc52Clipboard { max_bytes: usize }`.
10. `crates/dun-cli/src/terminal/shell.rs` — handle the new action: arm the
    reader (`begin_osc52_query(max_bytes)`), write+flush the query bytes
    (`osc52_read_query()`, reusing the `WriteTerminal` flush path ~`:22`),
    record `now + 500 ms`. Query-write I/O errors keep the current fatal
    behavior.
11. `crates/dun-cli/src/terminal/event_loop.rs` — the pending-read phase: while
    a read is pending, draw the waiting status, poll `next_osc52_response(
    min(250ms, remaining))`, and on response/timeout apply the outcome
    (`complete_external_paste(text)` for a nonempty response; empty-report; or
    the single internal fallback), then resume ordinary FIFO dispatch. Cancel
    the arm on completion. A late response after timeout is already ignored by
    step 1 (unarmed → no event).
12. `crates/dun-cli/src/app/*` — `complete_external_paste(text)`: empty → set
    `Terminal clipboard is empty`, no fallback; nonempty → set `Pasted terminal
    clipboard` then `handle_paste`, letting any read-only/collapsed/missing
    failure from `handle_paste` override the success text.

### Menu + help + status + i18n
13. `crates/dun-ui/src/frame/menu.rs` — Edit-menu entry
    `menu.edit.paste-external` / English `Paste External (<free mnemonic>)` /
    `EditCommand::PasteExternal`, next to `Copy External`. Pick a mnemonic not
    already used in the Edit menu (the menu completeness/uniqueness test
    enforces this).
14. `crates/dun-cli/src/help/content.rs` — help entry
    `help.command.edit.paste_external`, English `Paste from the terminal
    clipboard`.
15. `crates/dun-cli/src/ui_text/status/edit.rs` — the status keys + `ALL`
    entries for every message below.
16. All ten `i18n/*.conf` — every new `menu.*`, `help.command.*`, and
    `status.*` key, in the same commit (completeness tests enforce it).

Status messages (exact English; keys under `status.external-paste.*` or the
existing `status` naming — match the file's convention):

| Situation | English |
| --- | --- |
| waiting | `External paste: waiting for terminal clipboard` |
| success (from terminal) | `Pasted terminal clipboard` |
| terminal clipboard empty | `Terminal clipboard is empty` |
| no response, internal used | `Terminal clipboard unavailable; pasted internal clipboard` |
| no response, internal empty | `Terminal clipboard unavailable; internal clipboard empty` |
| read not opted in, internal used | `External paste disabled; pasted internal clipboard` |
| read not opted in, internal empty | `External paste disabled; internal clipboard empty` |

Preserve the more specific existing paste error for read-only / missing-buffer
/ collapsed-pane cases (from `handle_paste`).

### Config diagnostics
17. `crates/dun-cli/src/app/helper_panes.rs` — the OSC 52 diagnostics line:
    show both the write opt-in and the read opt-in distinctly (rename the
    current ambiguous field to `osc52_write` and add `osc52_read`, keep
    `osc52_max_bytes`); update `crates/dun-cli/src/tests/helper_panes.rs`.

### Tests
18. Application/config unit tests (independent oracles): `allow_read` defaults
    false and parses independently; `--dump-config` round-trips both flags;
    `edit.paste_external` id round-trips; default key is exactly `Ctrl+X,Ctrl+V`;
    disabled command falls back internally WITHOUT emitting a query; enabled
    command emits the typed query carrying `max_bytes`; a valid response
    replaces an active selection via the normal edit path and stays raw buffer
    content (control bytes included, render-sanitized); invalid UTF-8 becomes
    the escaped fallback text; empty response does not use stale internal
    clipboard; timeout uses internal clipboard exactly once; timeout with empty
    internal reports that; all ten catalogs stay complete; menu-matrix/help
    snapshots change only for the new command/status.
19. PTY (`crates/dun-cli/tests/pty_smoke.rs` + `tests/support/pty.rs`): the
    harness must FIRST match the exact emitted query, THEN send a hardcoded
    response — never reply blindly. Two cases: (a) `allow_read=true`, trigger
    `Ctrl+X,Ctrl+V`, answer with hardcoded base64, assert the payload appears
    in the normalized grid and the query was observed; (b) seed the internal
    clipboard, clear the buffer, trigger, observe the query, send NO response,
    assert the fallback status and the internal text restored, with elapsed
    ≥ 500 ms and well under the harness ceiling. Also assert a response
    carrying a terminal-control payload never emits a raw escape to the
    terminal output.

### Docs (behavior change → same-change doc updates, per AGENTS.md)
20. `README.md` (paste feature paragraph), `docs/configuration.md`
    (`clipboard.osc52.allow_read` + the read behavior + the terminal-owns-the-
    gate note), `docs/dev/terminal-compatibility-checks.md` (add OSC 52 read to the
    bounded-input surface as "parsed when enabled, best-effort at the
    terminal/multiplexer boundary"; note `Alt+]` is byte-identical when a read
    is not pending), `docs/dev/editor-baseline.md` if it enumerates clipboard
    behavior, and `AUDIT.md` (the read path rides the display sanitizer; no new
    scrubber; base64/UTF-8 validation under a cap).

## Scope

- Files you MAY modify: the items 1–20 above and their colocated tests, plus
  `i18n/*.conf` and the listed docs and snapshots.
- Files/areas you MUST NOT touch: the step-1 seam internals
  (`terminal/vt/parser/**`, `terminal/vt/event.rs`, `terminal/clipboard.rs`
  decoder/query — only *call* them), `terminal/event_reader.rs` (use its
  step-1 API; do not change it), any `Cargo.toml`/`Cargo.lock`, `AGENTS.md`,
  `CLAUDE.md`, `PROGRESS.md`, `TODO.md`, `.git`, `hosts/**`, `vm-test/**`,
  `reference/**`, and `docs/dev/**`.

If a change needs a file outside Scope, STOP and report.

## Deliverable + load-bearing proofs

- The full-trail feature + the i18n batch + docs.
- Prove load-bearing (run yourself, then reverse — never `git checkout`):
  (a) make `paste_external` ignore `allow_read` (always query) → the
  disabled-falls-back-without-query test fails; (b) remove the internal
  fallback on timeout → the PTY no-response case fails; (c) paste the internal
  clipboard on an empty terminal response → the empty-response test fails.

## dun pitfalls (read twice)

1. Safe Rust only; no new deps; no manifest/lockfile changes.
2. The 1 MiB dual-platform budget is real; Claude runs the macOS budget build
   at this gate and the binding Debian measurement at the step-3 (054) close.
3. Enabling write (`enabled`) must never grant read (`allow_read`).
4. Internal Paste must never start querying the terminal.
5. A late/duplicate response must not double-paste (step 1 disarms on
   completion/timeout; verify the wait phase cancels the arm).
6. All pasted text rides the render sanitizer; no insertion-time scrubber.
7. Stop-loss: same failure twice, or an out-of-scope file needed → STOP,
   report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Note the pty_smoke and tmux suites explicitly (they must actually run — say so
if tmux/expect is absent). Claude runs the macOS budget build at the gate.

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format (final message)

1. What changed — per file, line ranges, one-line why.
2. Verification — each command's verbatim output (suite counts; PTY/tmux noted).
3. Mutation evidence — the three load-bearing runs, verbatim.
4. Verdict.
5. Stop-loss / open questions (empty if none).
