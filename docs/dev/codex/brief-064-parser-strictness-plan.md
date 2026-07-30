# Brief 064 — Parser strictness: config comments + JSON protocol (PLAN ONLY)

**This is a design-only brief. `Scope: NONE — no source change.`** You produce
a plan; Claude reviews it, decides the open questions, and dispatches the
implementation steps as separate gated briefs.

## Goal

Produce a concrete, step-by-step implementation plan for two parser-level
correctness defects, both confirmed by measurement (evidence below, already
verified — do not spend effort rediscovering it):

1. `crates/dun-config/src/parser.rs` strips comments before honouring quotes,
   so a quoted value containing `#` is silently corrupted.
2. `crates/dun-plugin/src/json.rs` accepts a superset of RFC 8259 number
   syntax, accepts duplicate object keys with first-wins semantics, and models
   every number as `f64`.

The plan must let Claude dispatch each step independently, with a named test
gate per step, and must state the byte cost of each option because both files
are runtime code under the 1 MiB dual-platform budget (current margin on the
binding platform, Debian: 259,792 bytes).

## Verified current behaviour (measured 2026-07-30 — treat as given)

### Defect 1 — config comment stripping is not quote-aware

`parser.rs:26` calls `strip_comment(raw_line)` before `parser.rs:108` calls
`unquote_value(raw_value)`. `strip_comment` (`parser.rs:91-95`) splits on the
**first** `#` in the whole line; `unquote_value` (`parser.rs:451-463`) only
strips quotes when the trimmed value both starts and ends with the same quote
character.

Measured, through the public `dun_config::parse_config`:

```
input : plugin.log.command = "/opt/dun#tools/host"
result: OK,  command == "\"/opt/dun"      <-- note the retained leading quote
input : plugin.log.command = /opt/dun#tools/host
result: OK,  command == "/opt/dun"
input : plugin.log.command = "/opt/tools/host"
result: OK,  command == "/opt/tools/host"
```

So the failure is worse than "the value is truncated": the surviving leading
`"` becomes part of the string, and for `plugin.*.command` that string is an
executable path. An unterminated quote raises **no error at all**.

`docs/configuration.md:9` currently promises "Blank lines and text after `#`
are ignored", which is the behaviour a fix changes. That doc is in scope for
the implementation steps you plan (name it explicitly in the step that changes
behaviour), not for this brief.

### Defect 2 — JSON is a lenient superset

`json.rs:188-203` scans a run of `[0-9-+.eE]` and delegates to Rust's
`f64::from_str`, which is more permissive than JSON. Measured, through
`dun_plugin::json::parse`:

```
01     -> ACCEPTED as Num(1.0)      (RFC 8259: invalid, leading zero)
1.     -> ACCEPTED as Num(1.0)      (RFC 8259: invalid, no fraction digit)
-.5    -> ACCEPTED as Num(-0.5)     (RFC 8259: invalid, no integer digit)
00.5   -> ACCEPTED as Num(0.5)      (RFC 8259: invalid)
1e     -> rejected
1.2.3  -> rejected
1e+2   -> ACCEPTED as Num(100.0)    (valid JSON — must stay accepted)
-0     -> ACCEPTED as Num(-0.0)     (valid JSON — must stay accepted)
1E5    -> ACCEPTED as Num(100000.0) (valid JSON — must stay accepted)
```

Duplicate keys, via `Json::get` (`json.rs:25-34`, `Vec<(String, Json)>` +
`find`):

```
{"request_id":1,"request_id":2}  -> ACCEPTED, get("request_id") == Num(1.0)
```

First-wins. `serde_json`, JavaScript `JSON.parse`, and Python `json` all take
the **last** value, so a third-party host and `dun` can disagree about the same
bytes. That is the compatibility risk worth closing before the protocol goes
v1.

Integers: `Json::Num(f64)` (`json.rs:18`), `json::num` encodes `value as f64`
(`json.rs:75`), and `as_u64` (`json.rs:44-51`) guards on
`MAX_SAFE_INTEGER = 2^53-1`. Measured:

```
{"id":9007199254740993}     -> as_u64() == None
{"id":18446744073709551615} -> as_u64() == None
```

Note this **fails closed** rather than corrupting a value — the practical
urgency is lower than the other two items, and the plan should say so rather
than assume a big refactor is warranted. But note the error quality: `proto.rs`
`field_u64` (`proto.rs:199-204`) maps a failed `as_u64` to
`ProtocolError::MissingField`, so an out-of-range `request_id` is reported as a
*missing* field. That misdiagnosis is cheap to fix regardless of the
representation decision.

## Context pointers

- Read `AGENTS.md` (invariants, engineering rules) first, then
  `docs/plugin-protocol.md` (the wire contract these parsers implement) and
  `docs/configuration.md` (the promised config syntax).
- Key files:
  - `crates/dun-config/src/parser.rs` (547 lines) — the config parser; tests
    in `crates/dun-config/src/tests/parser.rs` (346 lines).
  - `crates/dun-plugin/src/json.rs` (417 lines) — hand-rolled JSON, inline
    `#[cfg(test)] mod tests` at line 375.
  - `crates/dun-plugin/src/proto.rs` — the only in-tree consumer of
    `json::parse` for protocol messages (`proto.rs:157`); integer fields at
    `proto.rs:177`, `proto.rs:199-204`.
  - `crates/dun-plugin/src/validate.rs:168` — the other `as_u64` call site.
  - `crates/dun-plugin/src/bin/fixture-host.rs` — the test fixture host; it
    both parses and emits protocol JSON, so a strictness change can break it.
  - `hosts/` — five shipped hosts (`rust-syntect`, `python-pygments`,
    `python-logfilter`, `lua-logfilter`, `lua-highlight`) plus
    `hosts/check-host.py`. These **emit** JSON that `dun` now parses
    leniently; if any of them emits a form that strict parsing would reject,
    that is a shipped-host breakage and must appear in your plan.

## Deliverable

A plan document as your report (no files written). It must contain:

1. **Step list.** Each step: the files and functions it touches, what it
   changes, the named test gate that decides it, and whether it is
   behaviour-changing or byte-neutral by construction. Order the steps so each
   is independently committable and green on its own.
2. **Call-site inventory**, with `path:line` evidence:
   - every construction and consumption of `Json::Num` / `json::num` /
     `as_u64` / `as_f64`, in `crates/**` **and** `hosts/**`;
   - every config key whose value can legitimately contain `#` (grep the
     value-parsing arms in `parser.rs` and say which are free-form strings vs
     enumerated tokens);
   - every existing test that asserts today's lenient behaviour and would
     therefore need to change — list them, because a test that must be
     *edited* to pass is exactly where a silent regression hides.
3. **Shipped-host compatibility check.** Read what `hosts/**` actually emit.
   State whether strict numbers or duplicate-key rejection would break any of
   them, with evidence. If you cannot determine it by reading, say so and name
   the fixture that would answer it — do not guess.
4. **Byte-cost estimate per option**, since both files ship. In particular,
   contrast: (a) keeping `Json::Num(f64)` and fixing only the scanner, vs
   (b) adding a separate integer representation to the `Json` enum — option
   (b) touches every `match` on `Json` and is the one most likely to cost
   pages. Give a reasoned estimate, not a number you cannot support, and say
   which option you recommend.
5. **Invariant preservation.** State how each step keeps: the `#![forbid(
   unsafe_code)]` rule, the no-new-dependency rule, the sanitizer path for any
   new error text that can reach the terminal, and the existing protocol
   framing/depth/size limits.
6. **Risks and open questions** — see the list below; answer what the code can
   answer, and flag what only Claude can decide.

## Open questions the plan must address

Answer with evidence where the code decides it; flag clearly where it is a
judgement call for Claude.

1. **Config: fix or reject?** Two candidate semantics for `#` inside a quoted
   value: (a) a quote-aware scanner where only an unquoted `#` starts a
   comment; (b) keep "`#` always starts a comment" but *reject* the line with
   a clear error when the result is an unbalanced quote. Which is less
   surprising, and which is smaller? Note that (a) is a superset of today's
   accepted inputs and (b) turns some currently-accepted configs into startup
   errors.
2. **Config: escapes.** Does a minimal escape set (`\\`, `\"`, `\'`, maybe
   `\#`) need to exist at all for the keys that are actually free-form? If no
   shipped key needs it, saying so is a valid and cheaper answer.
3. **Config: unterminated quote.** Should it become an error with a line
   number? What breaks if it does — check `scripts/install.sh`'s generated
   config and any config in `hosts/**` or `docs/**` examples.
4. **Config: which layer errors.** Recall the two-layer load
   (`crates/dun-cli/src/config_loading.rs`): an invalid **installed** config
   reports and steps aside, an invalid **user** config is fatal. Confirm a new
   parse error rides that existing split correctly and needs no new plumbing.
5. **JSON: where to reject duplicates.** At the `json.rs` parse layer (affects
   every parse, simplest) or at the protocol object layer in `proto.rs` (the
   review's suggestion, narrower)? Which is smaller, and does the parse layer
   need an allocation (a key set) that the object layer would not?
6. **JSON: integer representation.** Given `as_u64` already fails closed, is
   changing the representation worth its bytes now, or is the right v0.2 scope
   just the strict scanner + duplicate rejection + the `field_u64` error-kind
   fix? Recommend one.
7. **Fuzzing.** The review proposes coverage-guided fuzzing for these parsers.
   State whether that can be added without a new runtime dependency and
   without a nightly toolchain, and if it needs either, say so — it is then
   out of scope for v0.2 and belongs in a separate proposal.

## Explicitly out of scope

- **No source changes.** Not one file. If you believe a change is needed to
  answer a question, describe the experiment instead.
- The `run_command_capture` descendant-process hang in
  `crates/dun-cli/src/terminal/shell.rs` is a **separate, later brief**. Do not
  plan it, do not fold it in. Mention it only if it genuinely constrains a
  decision here.
- Crate metadata drift and `TODO.md` hygiene are Claude's, not this brief's.

## Scope

- Files you MAY modify: **NONE — design only, no source change.**
- Files/areas you MUST NOT touch:
  - everything (this is a read-only brief), and specifically `AGENTS.md`,
    `CLAUDE.md`, `README.md`, `TODO.md`, `docs/**`, `.git`, git config,
    `Cargo.toml`, `Cargo.lock`, `vm-test/**` (local SSH keys), `reference/**`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** Every crate root has
   `#![forbid(unsafe_code)]`.
2. **The 1 MiB dual-platform size budget is real.** Both files in this brief
   are runtime code. No new dependencies. Byte cost is a first-class part of
   the plan, not an afterthought.
3. **All untrusted text goes through the sanitizer.** New parse-error messages
   can carry file bytes to the terminal; they must ride the existing sanitized
   paths.
4. **Tests are layered and colocated.** `dun-config` uses `src/tests/*.rs`
   behaviour modules; `dun-plugin/src/json.rs` uses an inline
   `#[cfg(test)] mod tests`. Match the local style of the file you extend.
5. **Plan for tests that can fail.** Claude gates every invariant test by
   mutating the implementation and confirming the test fails. A test whose
   oracle reuses the implementation's own predicate is worthless — for the
   number scanner especially, the oracle must be the RFC's grammar written out
   independently, not the scanner's own accept set. State the intended oracle
   for each proposed test.
6. **Name the path that actually executes.** The recurring failure on the
   folding stage was tests covering a path that does not run. For each
   proposed test, say which production call site it exercises.
7. **Stop-loss is real.** If the same question defeats you twice, STOP and
   report it as an open question.

## Verification

This is a design-only brief: there is no build to run and no green to reach.
Verification is that every claim in your plan carries `path:line` evidence you
actually read. Do not run `cargo` except read-only inspection commands if you
genuinely need them (`cargo tree`, `cargo metadata`); do not build, do not
edit, do not format.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify any file. This brief produces a report only.
- Full machine access, but touch NOTHING outside this repo, no network.
- You MUST NOT claim you read something you did not. Every `path:line` in the
  plan must be real. If you are unsure of a fact, mark it explicitly as
  unverified rather than asserting it.

## Report format (your final message)

1. **Plan** — the numbered step list per Deliverable item 1.
2. **Inventories** — Deliverable items 2 and 3, as tables with `path:line`.
3. **Byte-cost analysis** — Deliverable item 4, with your recommendation.
4. **Open questions** — answers to all seven above, each marked
   `[evidence]` or `[judgement call for Claude]`.
5. **Stop-loss** — where you stopped and why (empty if none).
