# Brief 051 — Design plan for OSC 52 clipboard read/paste (design only)

**Diagnostic/design brief. NO source change.** Produce a step-by-step
implementation plan; Claude evaluates it, decides the open questions, then
dispatches the steps as implementation briefs and gates each. (Plan-first
workflow — see CLAUDE.md.)

## Why (user decision 2026-07-27)

The mainline restoration stage (F12/F13) is closed. The next queued item is
OSC 52 **read** — letting the user paste the host/system clipboard over SSH
via the terminal, the read counterpart to the OSC 52 **write** (copy) dun
already ships. TODO.md "Deferred": *"OSC 52 paste/query support or
platform-specific clipboard command integration."* Platform-specific clipboard
commands (`pbpaste`/`xclip`/`wl-paste`) are rejected — they break dun's
no-external-clipboard-command policy and do not work over SSH. This line is
OSC 52 read only.

## What exists (read before planning)

- **The write side (the template):** `clipboard.osc52.{enabled,max_bytes}`
  (`crates/dun-config/src/config.rs:111` `Osc52Config`, default
  `enabled=false`, `max_bytes=16*1024`); `osc52_copy_sequence`
  (`crates/dun-cli/src/terminal/clipboard.rs:1`) emits
  `ESC ] 52 ; c ; <base64> BEL`, with a hand-rolled `base64_encode` right
  beside it (no external crate); `copy_text_external`
  (`crates/dun-cli/src/app/editing.rs:499`) gates on `enabled`, checks
  `max_bytes`, and sends via `RuntimeAction::WriteTerminal(String)`
  (`crates/dun-cli/src/terminal/action.rs`, drained in
  `crates/dun-cli/src/terminal/shell.rs:22`).
- **The input parser** (`crates/dun-cli/src/terminal/vt/parser/mod.rs`): a
  byte-at-a-time state machine. `State::Paste`/`DiscardPaste` accumulate
  bracketed-paste bytes into `self.paste` under a `PASTE_CAPACITY` cap, then
  `push_event(Event::Paste(text))` (mod.rs:372-419). The startup ambiguous-
  width probe already parses CPR/DA1 (the `cpr` field, mod.rs:59). OSC is
  `ESC ]` (0x5D); the read response is `ESC ] 52 ; c ; <base64> ST|BEL`
  (`ST` = `ESC \`, 0x1B 0x5C; `BEL` = 0x07). This response is the accumulate-
  to-terminator-then-emit shape `State::Paste` already models.
- **Event flow:** `Event::Paste(text)` → `app.handle_paste(&text)`
  (`crates/dun-cli/src/terminal/event_loop.rs:64`); `Event` is defined in
  `crates/dun-cli/src/terminal/vt/event.rs:119`. Buffer text is stored raw and
  neutralized at render by `DisplaySanitizer` (there is NO insert-time
  sanitizer in `editing.rs` — paste rides the display sanitizer like all
  buffer text).
- **The bounded input surface** is documented in
  `docs/dev/terminal-compatibility-checks.md:24-25` (CSI/SS3 keys, UTF-8, SGR
  mouse, bracketed paste, CPR + DA1). The OSC 52 read response is NOT in it —
  adding it extends the documented surface.
- The PTY harness answers the startup CPR/DA1 probe
  (`docs/dev/terminal-compatibility-checks.md:81`) — the model for a harness that
  answers an OSC 52 read query.

## Claude's decisions (bake these into the plan — not open questions)

- **No new security-review layer on input.** The decoded clipboard bytes are
  ordinary buffer text and ride the existing `DisplaySanitizer` at render,
  exactly like bracketed paste and file text. The only decode steps are
  base64-decode + UTF-8 validation (invalid → dun's existing escaped-fallback
  behavior, as with non-UTF-8 file loads), both under a byte cap. Do not add a
  bespoke vetting/scrubbing layer.
- **The terminal owns the read-gate.** Most terminals disable or prompt for
  OSC 52 read by default (kitty read-off/ask, iTerm2 preference, tmux
  `set-clipboard`, xterm `allowWindowOps`). dun does not replicate that
  policy; it must degrade cleanly when the terminal never answers.
- **Opt-in, default off**, matching the write side's posture.
- **Safe Rust, no new dependencies.** Hand-roll `base64_decode` beside the
  existing `base64_encode` in `clipboard.rs`.
- **Bounded like Paste.** The response accumulates under an explicit byte cap
  (reuse/mirror the `max_bytes`/`PASTE_CAPACITY` discipline); an overlong or
  malformed response is discarded, never partially applied.
- **Never block the editor indefinitely.** A read query that gets no response
  must fall back to the internal clipboard within a bounded time; the editor
  must stay responsive.

## The plan must address each

1. **Query emission.** The read query `ESC ] 52 ; c ; ? BEL`: reuse
   `RuntimeAction::WriteTerminal`? Gate on config. `path:line` for where it is
   triggered and sent.
2. **Parser extension.** The new OSC-read state machine mirroring
   `State::Paste`: entry from `ESC ]`, matching the `52;c;`/`52;p;` prefix,
   accumulating base64 to `ST`/`BEL` under a cap, base64-decode + UTF-8
   validate, and the new `Event` variant it emits. Enumerate every edge:
   `ST` vs `BEL` terminator, an unrecognized OSC (must be consumed and
   discarded, not leak to the buffer), truncation/EOF mid-sequence, oversize,
   non-base64 payload, empty payload, and interaction with the existing
   `Paste`/`cpr` states. Map each to the `mod.rs` function it mirrors.
3. **Response → paste application.** How the decoded text reaches the buffer
   (reuse `handle_paste`? a distinct path?), and the **synchronous-feel vs
   async** question: a short bounded wait after the query (like the CPR
   startup probe) for paste-at-cursor, versus an async event that applies
   whenever the report arrives. Weigh UX (a paste that lands "later" is
   surprising in an editor), implementation cost, and the event loop's shape.
   Recommend one; leave the final call to Claude.
4. **Config + trigger surface (open — propose, Claude decides).** A separate
   `clipboard.osc52.allow_read` (default false) versus reusing `enabled`;
   whether reads share `max_bytes`. And the trigger: a distinct
   `edit.paste_external` command + keybinding versus overloading the internal
   Paste (`Ctrl+V`). Give a recommendation with reasons (Claude leans: a
   separate read flag and a distinct command, so the internal clipboard stays
   the zero-surprise default — but inventory the implications).
5. **No-response / timeout fallback.** Exact semantics: how long dun waits,
   what the user sees (status message), and the guaranteed fall back to the
   internal clipboard. Tie this to the event-loop tick model.
6. **Ordered steps (likely 2–3), each its own implementation brief.** Sequence
   conservatively (parser + decode + fixture first, behind an event that
   nothing yet triggers; then config + command + wiring; then docs). Per step:
   files/functions, the gate tests, and what regresses if done wrong.
7. **Test plan.** Parser unit fixtures (well-formed `c`/`p`; `ST` and `BEL`
   terminators; truncated; oversize-capped; non-base64; empty; an unrelated
   OSC swallowed cleanly; a read response interleaved with a bracketed paste);
   base64-decode unit tests (independent oracle, round-trip against the
   existing encoder AND hardcoded vectors); a PTY harness case answering the
   query with a payload and asserting the paste, plus a no-response case
   asserting the fallback. Name the mutation targets for each invariant guard
   (the cap, the terminator match, the discard-on-malformed).
8. **Risks / open questions for Claude** — including: terminal/tmux read
   support reality (which of the SSH-target terminals actually answer, and
   whether tmux `set-clipboard` passthrough forwards the query and the reply);
   the async-response race with normal typing; any place the new OSC state
   could swallow or mis-frame a legitimate sequence; whether the bounded-input
   claim in `terminal-compatibility-checks.md` needs a caveat that read
   responses are best-effort.

## Scope

- Files you MAY modify: **NONE — design only.** Leave the tree clean
  (`git status --short` empty when done). Read anything; run read-only
  commands; do not `cargo build`/edit.

## Hard rules

- Do NOT edit any source file, commit, branch, push, or touch git.
- Base every claim on real files (`path:line`); do not hand-wave the parser
  states or the call-site inventory.
- Safe Rust only in the design — if some piece seems to require `unsafe` or a
  new dependency, that is an open question for Claude, not a design choice.

## Report format (final message)

The eight-part plan above, concrete enough that each step could become its own
implementation brief without further discovery.
