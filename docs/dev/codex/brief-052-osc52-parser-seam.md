# Brief 052 — OSC 52 read step 1: decoder, parser framing, event seam

Implementation brief. **Step 1 of the accepted plan produced for brief 051**
(OSC 52 clipboard read; see it and Claude's decisions below). This step adds
the bounded response parser, the base64 decoder, and the armed-query event
seam **behind an event nothing yet triggers** — no config, no command, no user
trigger. Default input behavior stays **byte-identical**: when no read is
armed, `ESC ]` keeps its current `Alt+]` behavior.

## Goal

dun can parse an OSC 52 read response (`ESC ] 52 ; c|p ; <base64> ST|BEL`)
into a validated `Event::Osc52Clipboard(String)` — but only while a read query
is armed. A `base64_decode` sits beside the existing `base64_encode`. The
`EventReader` gains an arm/cancel API and a response-only read. Nothing
triggers a query yet (that is step 2), so this brief changes no user-visible
behavior and must leave every existing test and golden untouched.

## Claude's decisions (fixed — do not revisit)

- **Armed-gated framing (the key constraint).** OSC 52 framing is entered ONLY
  when a read query is armed (`EventReader::begin_osc52_query`). When NOT
  armed, `ESC ]` must behave exactly as today — `step_escape` emits `Alt+]`
  for `]` (verified at `parser/mod.rs` step_escape `_ => step_input_byte(byte,
  ALT)`). Do not add always-on OSC consumption: it would change `Alt+]` and,
  worse, swallow real keystrokes typed after it. Terminals never send
  unsolicited OSC 52 responses, so the unarmed path needs no OSC handling.
- **Typed query action.** Add `RuntimeAction::QueryOsc52Clipboard { max_bytes }`
  (`terminal/action.rs`); this brief defines it and the arm path but the
  event-loop drain that sends the query and waits is step 2. `max_bytes` is the
  decoded-byte limit.
- **Response only when armed.** An `Event::Osc52Clipboard` is emitted only for
  a response received while armed; anything OSC-shaped while unarmed is not
  parsed at all (framing isn't entered). Empty payload → emit
  `Event::Osc52Clipboard(String::new())`.
- **Bounded, discard-on-malformed.** Oversize (encoded accumulator would exceed
  the checked `4*ceil(max_bytes/3)` bound) → clear and DiscardOsc to the
  terminator, emit nothing. Non-base64 / bad padding / lone-`ESC`-not-`\` →
  reject the whole frame, emit nothing. Truncated/expired → emit nothing,
  restore Ground. Never apply partial text.
- Safe Rust, no new deps, no manifest/lockfile changes.

## Exact change

1. **`crates/dun-cli/src/terminal/clipboard.rs`** — add:
   - `osc52_read_query() -> &'static str` returning `"\x1b]52;c;?\x07"`.
   - `base64_decode(encoded: &[u8], max_bytes: usize) -> Option<Vec<u8>>`
     mirroring the existing `base64_encode`: standard alphabet; accept `=`
     padding and (optionally) an unpadded final group of 2–3 chars; reject a
     1-char final group, misplaced/excess padding, invalid alphabet bytes, and
     nonzero unused padding bits; check the decoded limit before every push;
     return `None` (no partial output) on any error or overflow.
2. **`crates/dun-cli/src/terminal/vt/event.rs`** — add `Osc52Clipboard(String)`
   beside the existing `Event` variants.
3. **`crates/dun-cli/src/terminal/vt/parser/mod.rs`** — add the armed OSC
   framing, mirroring the `Paste`/`DiscardPaste` shape (`:372-424`):
   - an armed decoded-byte limit field, set by the reader's arm call, cleared
     on emit/cancel/expiry;
   - `State::OscPrefix { .. }`, `State::Osc52 { st_pending, .. }`,
     `State::DiscardOsc { st_pending, .. }`, and an `osc52_payload: Vec<u8>`;
   - enter OSC framing from `ESC ]` **only when armed**; when unarmed, leave
     `step_escape`'s current `]` → `Alt+]` path untouched;
   - fixed-prefix match `52;c;` / `52;p;` (both selectors → the same event;
     don't accumulate the prefix); an unrecognized OSC while armed →
     DiscardOsc to terminator, emit nothing;
   - BEL (`0x07`) finalizes immediately (never appended); ST (`ESC \`) holds
     the `ESC` and finalizes only on `\`, else the frame is malformed →
     DiscardOsc;
   - on a complete recognized frame: `base64_decode(payload, armed_limit)` →
     on `Some(bytes)` emit `Event::Osc52Clipboard` (see decode-to-text below);
     on `None` emit nothing;
   - an OSC frame deadline extending the existing escape-expiry
     (`mod.rs:116-135`); expiry clears the accumulator, disarms, restores
     Ground.
   - **decode-to-text:** run the decoded bytes through
     `dun_core::decode_file_text` (`crates/dun-core/src/file_text.rs`); use its
     returned text (valid UTF-8 stays raw incl. controls; invalid bytes become
     the existing literal escaped form). Do NOT set the buffer read-only and do
     NOT add any insertion-time scrubber — render-time `DisplaySanitizer`
     already protects output.
4. **`crates/dun-cli/src/terminal/event_reader.rs`** — add:
   - `begin_osc52_query(&mut self, max_bytes)` — arm the parser's OSC limit;
   - `cancel_osc52_query(&mut self)` — disarm;
   - `next_osc52_response(&mut self, timeout) -> io::Result<Option<String>>` —
     poll for and remove ONLY an `Osc52Clipboard` event from the bounded queue
     without popping earlier ordinary events (they stay FIFO-queued); return
     `Ok(None)` at the deadline; propagate `UnexpectedEof` on mid-frame EOF;
   - fold the OSC deadline into the existing `pending_escape_deadline()`
     selection (`event_reader.rs:91-106`); plain Paste must NOT gain this OSC
     timeout.
5. **`crates/dun-cli/src/terminal/action.rs`** — add
   `RuntimeAction::QueryOsc52Clipboard { max_bytes: usize }` (definition only;
   the `shell.rs`/`event_loop.rs` handling that actually sends + waits is
   step 2). If a match becomes non-exhaustive, add a minimal
   `unreachable!()`-free placeholder arm that does nothing, or gate it — but
   prefer leaving the send path for step 2 and only adding the variant if it
   compiles cleanly without a dead arm. If it forces a dead arm, STOP and
   report so Claude can rescope the 052/053 boundary.
6. **`crates/dun-cli/src/terminal/event_loop.rs`** — a minimal arm so an
   unsolicited/leftover `Event::Osc52Clipboard` in the ordinary dispatch is
   consumed and ignored (no insertion), keeping the ordinary loop total.
7. **`crates/dun-cli/src/terminal/mod.rs`** — export the new items as needed.

## Scope

- Files you MAY modify: the seven items above and their colocated tests
  (`terminal/vt/parser/tests.rs`, unit tests in `clipboard.rs` and
  `event_reader.rs`).
- Files/areas you MUST NOT touch: any config crate, `dun-core` (read-only use
  of `decode_file_text`), any command/keymap/menu/help/i18n/status file,
  `app/**`, snapshots, any `Cargo.toml`/`Cargo.lock`, `AGENTS.md`, `CLAUDE.md`,
  `README.md`, `PROGRESS.md`, `TODO.md`, docs, `.git`, `hosts/**`,
  `vm-test/**`, `reference/**`.

If a change needs a file outside Scope, STOP and report.

## Deliverable

- The decoder, the armed OSC parser framing, the event, and the reader API.
- Tests with **independent oracles** (hardcoded, never derived from the impl):
  - base64: hardcoded encode/decode vectors (empty, `f`, `fo`, `foo`,
    `foobar`, a `0xFF` byte); exact decoded-cap success and cap-plus-one
    reject; invalid alphabet, bad length, misplaced/excess padding, nonzero
    unused bits, embedded whitespace; a full `0x00..=0xFF` round-trip
    (supplemental, not the oracle).
  - parser (extend `parser/tests.rs`): armed `52;c;` and `52;p;`; BEL and ST;
    byte-by-byte split through prefix/payload/terminator; truncated
    prefix/payload, lone-ST-`ESC`, expiry recovery; exact cap and cap-plus-one;
    non-base64; valid empty; an unrecognized armed OSC swallowed then a
    sentinel ordinary key parses; OSC bytes inside an active bracketed paste
    stay literal paste content; **unarmed `ESC ]` still emits `Alt+]` and the
    following bytes parse normally** (the byte-identical guarantee); an
    unarmed OSC-shaped stream emits no clipboard event.
  - event_reader (use the existing fake source): ordinary keys before/after a
    response stay FIFO-queued while the response is extracted; no-response
    poll returns at its deadline without consuming ordinary events; cancel
    disarms; partial OSC then zero-byte read → `UnexpectedEof`, no event.
- Prove load-bearing (run yourself, then reverse the edit — never
  `git checkout`): (a) change the cap boundary `>`→`>=` → the cap-plus-one
  test fails; (b) accept ST's `ESC` without requiring `\` → a terminator test
  fails; (c) enter OSC framing when unarmed → the `Alt+]`-preserved test fails.

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`).
2. Byte-identical default behavior: no existing test or golden may change.
   The whole feature is inert until step 2 arms a query.
3. All decoded text rides the render-time sanitizer; no insertion-time
   scrubber, no read-only buffer.
4. Bounded everything: the decoded cap, the encoded accumulator bound, the OSC
   frame deadline. Malformed → discard to terminator, emit nothing.
5. Stop-loss: same failure twice, or an out-of-scope file needed (esp. a
   forced dead match arm in `action.rs`/`event_loop.rs`) → STOP, report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Note the pty_smoke and tmux suites' results explicitly. Claude runs the macOS
budget build at the gate (Debian batches at the step-3 closure).

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working
  tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format (final message)

1. What changed — per file, line ranges, one-line why.
2. Verification — each command's verbatim output (suite counts; PTY/tmux
   noted).
3. Mutation evidence — the three load-bearing runs, verbatim.
4. Verdict.
5. Stop-loss / open questions (empty if none).
