# Brief 021 — Prove the sanitizer invariant instead of sampling it

Implementation brief. "Buffer text, pane titles, and status fields are sanitized
before rendering so file content and file names cannot emit terminal control
sequences" is an A-level invariant (AGENTS.md, README). It is what stops a
hostile file from hijacking the terminal of whoever opens it — the exact thing
that makes `cat`ing an untrusted file dangerous and an editor safe.

It is currently guarded by **nine hand-written `assert_no_raw_controls` calls**
in `dun-ui/src/tests/fallback.rs`. Hand-written cases only cover the attacks
somebody thought of.

## Exhaustive, not random. And no `proptest`.

The obvious reach is a property test. Do not take it.

`DisplaySanitizer::sanitize_line` (`crates/dun-core/src/display.rs`) walks the
input **one `char` at a time** — `for (index, ch) in line.char_indices()` then
`push_char(ch)` — and `push_char` carries no state between characters. So the
output of a string is the concatenation of the per-character outputs, and the
whole input space that matters is **every Unicode scalar value**: about 1.1
million, which a test can simply walk end to end in well under a second.

Exhaustive is *strictly stronger* than sampling a few hundred random strings,
and it needs no dependency. `proptest` would pull a dozen crates into a repo
that has **zero** dev-dependencies and two runtime ones, to do a weaker job.

Do not add `proptest`, `arbitrary`, `quickcheck`, or any other crate.

## Goal

1. Prove, by exhaustion, that no Unicode scalar value survives the sanitizer as
   a control character — in every sanitizer profile.
2. Prove the char-wise composition the exhaustion relies on.
3. Prove it end to end: a frame whose every text field is poisoned must emit no
   attacker-controlled escape sequence to the terminal.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-core/src/display.rs` — `DisplaySanitizer`, `sanitize_line`,
  `push_char`, `SanitizedLine::as_plain_text`, `DisplayClass`,
  `render_control`, `render_non_ascii`, the `ascii_only` and `max_bytes`
  settings. This is the thing under test; **do not change it**.
- `crates/dun-ui/src/shell.rs` — `UiShell::display_sanitizer`, built from the
  terminal profile, so there is more than one configuration to exhaust.
- `crates/dun-ui/src/render/chrome.rs` — `sanitize_chrome_text`, the wrapper
  every chrome string goes through.
- `crates/dun-ui/src/frame/text.rs` — `sanitize_buffer_body`.
- `crates/dun-ui/src/surface_emit.rs` — **this is what actually writes bytes to
  the terminal.** The end-to-end test must go through here: sanitizing perfectly
  is worthless if some field never reaches the sanitizer, and only the emitted
  bytes can prove it did.
- `crates/dun-ui/src/snapshot.rs` — `frame_snapshot`, added in brief 018; useful
  for building a frame, though the emitted bytes are what matter here.
- `crates/dun-ui/src/tests/support.rs` — `assert_no_raw_controls`, the existing
  (weak) check. Keep it; it is used elsewhere.

## Specification

### 1. Exhaustive over every scalar value

For every `char` in `'\u{0}'..='\u{10FFFF}'` (Rust's `char` range already
excludes surrogates), for **every** sanitizer configuration the shell can
produce (at minimum: UTF-8 and ASCII-only; plus a `max_bytes` small enough to
exercise truncation):

- `sanitizer.sanitize_line(&ch.to_string()).as_plain_text()` must contain
  **no** `char::is_control()` character, no `\u{1b}`, and no C1 control
  (`\u{80}`..=`\u{9f}`).
- The failure message must name the offending scalar value in hex and the text
  it produced. That is what makes exhaustion debuggable without a shrinker.

Note the cost: this is ~1.1M iterations per configuration. Keep the inner loop
allocation-light so the test stays fast; if it takes more than a couple of
seconds, say so in your report rather than silently marking it `#[ignore]`.

### 2. Composition — the assumption the exhaustion rests on

Exhaustion over single characters only proves the string case if the sanitizer
really is char-wise. Pin that:

- For a set of strings (include multi-byte, combining marks, wide CJK, emoji,
  and mixed control/printable), assert
  `sanitize_line(s).as_plain_text()` equals the concatenation of
  `sanitize_line(&ch.to_string()).as_plain_text()` over `s.chars()` — for inputs
  short enough that `max_bytes` truncation does not kick in.
- If this does **not** hold, STOP and report: it means the exhaustion argument
  is invalid and the brief needs rethinking. Do not paper over it.

### 3. End to end — the bytes that reach the terminal

The sanitizer can be perfect and the invariant still broken, if some string
reaches the surface without passing through it. Only the emitted bytes can rule
that out.

Build a frame in which **every** attacker-influenceable text field carries a
payload, then emit it through `surface_emit` and assert the payload's escape
sequences are absent from the output:

- buffer body text;
- the file name / window title;
- the status line (left and right);
- the plugin indicator text (`StatusBar::plugin`);
- overlay title, lines, input and list entries (drive a file dialog whose
  directory listing contains a poisoned file name if you can do so
  deterministically; otherwise construct the `UiOverlay` directly).

Payloads to use, at minimum — a curated list of what actually attacks a
terminal, not random bytes:

- `\u{1b}[2J` and `\u{1b}[H` (clear/home),
- `\u{1b}]0;pwned\u{7}` (OSC window title),
- `\u{1b}]52;c;…\u{7}` (OSC 52 clipboard write),
- `\u{1b}P…\u{1b}\\` (DCS),
- `\u{9b}2J` (C1 CSI, the single-byte form — a classic bypass when code only
  filters `\u{1b}`),
- `\u{7}` (BEL), `\r`, `\u{8}` (backspace, used to overwrite what the user saw),
- `\u{202e}` (RTL override, used to disguise file names).

The assertion is on the **emitted byte stream**: the only `\u{1b}` sequences in
it may be ones the renderer itself produced for styling and cursor placement.
Concretely: strip the renderer's own SGR/cursor sequences, then assert none of
the payloads survive — and assert specifically that `\u{9b}` and `\u{7}` appear
nowhere at all, since the renderer never emits either.

If any field turns out **not** to be sanitized, that is a real vulnerability.
**Do not fix it in this brief.** Report it, and leave the test failing or
`#[ignore]`d with a comment naming the hole, so the fix lands as its own change
with its own review.

## Scope

- Files you MAY modify:
  - `crates/dun-core/src/display.rs` — **tests only**, if it has a colocated
    test module. The implementation is not to change.
  - `crates/dun-core/src/**/tests.rs` and `crates/dun-ui/src/tests/**` — new
    test modules, registered in the relevant `tests/mod.rs`.
- Files/areas you MUST NOT touch:
  - **any implementation code, in any crate.** This brief tests an invariant; it
    does not change behaviour. A test that had to change the code to pass is a
    test that proved nothing.
  - `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `PROGRESS.md`, `TODO.md`, `docs/**`,
    `README.md`;
  - `.git`, git config, any `Cargo.toml`, `Cargo.lock` — **no new
    dependencies**. Not `proptest`. See the section above; this is a decision.
  - `vm-test/**`, `reference/**`, `hosts/**`.

## Deliverable

- The exhaustive per-scalar test, per sanitizer configuration.
- The composition test.
- The end-to-end poisoned-frame test against the emitted bytes.
- In your report: the runtime of the exhaustive test, and **any field you found
  that is not sanitized** — that is the finding this brief exists to surface.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **No new dependencies. Not `proptest`.** Exhaustion is stronger and free.
3. **Do not change the implementation.** If a test fails, that is the finding.
4. **The C1 form (`\u{9b}`) is the interesting case.** Code that filters only
   `\u{1b}` looks safe and is not. Make sure it is in the payloads.
5. **The emitted bytes are the only real evidence.** A test that checks
   `sanitize_chrome_text` in isolation cannot see a field that never calls it.
6. **Tests are layered and colocated.**
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Then prove the exhaustive test is load-bearing: temporarily make `push_char`
pass one control character through unchanged, confirm the test fails and names
that scalar value, and restore it. Paste verbatim output.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave changes in the
  working tree; Claude gates and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and report.
- Full machine access, but touch NOTHING outside this repo, no network. Only
  file edits within Scope, `cargo`, and `python3` for parsing output.
- Minimal diff; no drive-by reformatting or renames.
- Paste real verbatim verification output; if not green, say so.

## Report format (your final message)

1. What changed — per file, line ranges, one-line why.
2. Verification — each command with verbatim output, including the mutant run.
3. **The findings** — the exhaustive test's runtime, whether composition holds,
   and every text field that reaches the terminal without being sanitized. Do
   not bury this.
4. Stop-loss / open questions (empty if none).
