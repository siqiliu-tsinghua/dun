# Brief 066 — JSON: strict RFC 8259 numbers, unique object keys, honest integer errors

Implementation brief. Step 2 of 2 from brief 064's approved plan. The design
questions are **already decided below** — implement the specification, do not
re-open it. Step 1 (config quote-aware comments) landed at `f5891a0`.

## Goal

`crates/dun-plugin/src/json.rs` accepts a superset of RFC 8259 number syntax and
accepts duplicate object keys with first-wins semantics. Both are compatibility
debt that gets expensive once third-party hosts exist: every other JSON
implementation a plugin author will use takes the **last** duplicate, not the
first. Make the parser strict, bound the cost of enforcing it, and stop
reporting an out-of-range integer as a *missing* field. Keep `Json::Num(f64)`.

## Verified current behaviour (measured — treat as given, do not re-derive)

Through `dun_plugin::json::parse`:

```
01     -> ACCEPTED as Num(1.0)      (RFC 8259: invalid, leading zero)
1.     -> ACCEPTED as Num(1.0)      (RFC 8259: invalid, no fraction digit)
-.5    -> ACCEPTED as Num(-0.5)     (RFC 8259: invalid, no integer digit)
00.5   -> ACCEPTED as Num(0.5)      (RFC 8259: invalid)
1e     -> rejected
1.2.3  -> rejected
+-1    -> rejected
1e+2   -> ACCEPTED as Num(100.0)    (VALID JSON — must stay accepted)
-0     -> ACCEPTED as Num(-0.0)     (VALID JSON — must stay accepted)
1E5    -> ACCEPTED as Num(100000.0) (VALID JSON — must stay accepted)

{"request_id":1,"request_id":2}  -> ACCEPTED, get("request_id") == Num(1.0)
```

Cause: `Parser::number` (`json.rs:188-203`) scans a run of `[0-9-+.eE]` and
delegates to Rust's `f64::from_str`, which is more permissive than JSON.
`Json::get` (`json.rs:25-34`) is a `find` over `Vec<(String, Json)>`.

Integers fail **closed**, which is why the representation is not changing:
`as_u64` (`json.rs:44-51`) returns `None` above `2^53-1` rather than a wrong
value. But `field_u64` (`proto.rs:199-204`) maps that `None` to
`ProtocolError::MissingField`, so an out-of-range `request_id` is reported as a
field that is *absent*. That misdiagnosis is cheap to fix and worth fixing.

Brief 064 verified, and you may rely on it: **no shipped host emits a number
form the strict scanner would reject, a non-finite number, or a duplicate
object member** — checked across `fixture-host.rs`, `hosts/rust-syntect`, both
Python hosts (`json.dumps`), and both Lua hosts (`%d` formatting with unique
`__order` lists). **No existing test asserts the lenient behaviour**, so no
existing test should need editing.

## Specification (decided — implement exactly this)

### 1. Strict numbers

Replace the permissive scan with the RFC 8259 grammar, implemented directly:

- optional `-` (a leading `+` is invalid);
- integer part: `0` alone, or `[1-9][0-9]*` (so `01` and `00.5` are invalid);
- optional fraction: `.` followed by **at least one** digit (so `1.` is invalid;
  `-.5` is invalid because the integer part is missing);
- optional exponent: `e`/`E`, optional `+`/`-`, **at least one** digit.

Keep `Json::Num(f64)`, keep delegating the final conversion to `f64::from_str`
once the grammar has accepted the span, and keep the existing non-finite
rejection.

### 2. Unique object keys, with a bounded cost

Reject duplicate keys in `Parser::object` — the parse layer, so it covers
envelopes and nested payload objects alike. Compare each newly parsed key
against the members already collected in the existing `Vec`; **do not** add a
`HashSet` or any other allocation. Error message: static, `"duplicate object
key"`.

**This part is not optional:** a plain linear scan is O(n²), and
`max_frame_bytes` defaults to 256 KiB (`proto.rs:117`), which admits roughly
43,000 members in one flat object — on the order of 10^9 string comparisons,
seconds of a frozen editor from a single malformed frame. So also add a
**`MAX_OBJECT_MEMBERS` constant** alongside the existing `MAX_DEPTH`
(`json.rs:10`) and reject an object with more members than that, with a static
error. This bounds the quadratic by construction and matches the limit
vocabulary the parser already uses.

**Choose the constant from evidence, not from taste.** Before picking it,
inventory the largest object actually produced anywhere in the protocol —
envelopes in `proto.rs`, validator payloads in `validate.rs`, the client's
constructions in `client.rs`, `fixture-host.rs`, and every host under `hosts/`.
Report that maximum, then set the constant with generous headroom over it
(round number, at least 8x the observed maximum). If you find an object whose
member count is unbounded by design, STOP and report — that would invalidate
this approach and is exactly the kind of thing worth stopping for.

### 3. Honest integer errors

`field_u64` returns `MissingField` only when `Json::get` returns `None`, and
`BadField` when a value is present but fails `as_u64`.

### 4. Conformance checker

`hosts/check-host.py` currently parses with `json.loads`, which is last-wins and
therefore does not mirror dun's new policy. Give it an `object_pairs_hook` that
rejects duplicates, so a plugin author testing against the checker sees the same
answer dun would give. Keep it standard-library only.

## Scope

- Files you MAY modify:
  - `crates/dun-plugin/src/json.rs`
  - `crates/dun-plugin/src/proto.rs`
  - `crates/dun-plugin/src/bin/fixture-host.rs`
  - `crates/dun-plugin/tests/protocol.rs`
  - `hosts/check-host.py`
  - `docs/plugin-protocol.md`
- Files/areas you MUST NOT touch:
  - `crates/dun-plugin/src/menu.rs` — the manually constructed duplicate-label
    test at `menu.rs:390-398` is defence in depth for in-memory `Json::Obj`
    values that never went through the parser. Leave it exactly as it is; parser
    rejection does not make it redundant.
  - `crates/dun-config/**` (step 1 is done and committed);
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `TODO.md`, and all of `docs/**`
    except `docs/plugin-protocol.md`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock`;
  - `vm-test/**` (contains local SSH keys), `reference/**`.

## Deliverable

- The strict number scanner, the bounded duplicate-key rejection with its
  `MAX_OBJECT_MEMBERS` constant, and the `field_u64` fix.
- The `check-host.py` duplicate hook.
- `docs/plugin-protocol.md` updated to state, for plugin authors: numbers must
  be RFC 8259 (no leading zeros, no bare `1.`, no `-.5`), object keys must be
  unique, there is a maximum member count per object, and integers must stay
  within 2^53-1 or the field is rejected as malformed rather than missing.
- Tests, per the requirements below.

## Test requirements (this is what the gate checks)

- **Independent oracle, spelled out by hand.** For the number grammar, write
  explicit valid and invalid byte tables. Never derive the expectation from the
  scanner's own accept set — a test whose oracle is the implementation cannot
  fail. Cover at minimum every case in the measured table above, plus `1e+`,
  `-`, `.5`, `0123`, `1e1e1`.
- **Name the executing path.** Unit tests on `json::parse` are necessary but not
  sufficient: add at least one test that drives a **real framed message** through
  `Envelope::from_json_bytes` / `HostClient::recv`, so the rejection is proven on
  the path the editor actually runs, not only on the parser in isolation. Extend
  `fixture-host.rs` with a mode that emits a malformed response (a non-RFC number
  and, separately, a duplicate key) and assert the client surfaces a protocol
  error.
- **`field_u64`**: assert the exact `ProtocolError` variant for absent, string,
  fractional, negative, `9007199254740993`, and `18446744073709551615` request
  ids. Absent must still be `MissingField`; the rest must be `BadField`.
- **`MAX_OBJECT_MEMBERS`**: one object at the limit parses, one over it is
  rejected.
- **No existing test may be edited.** Brief 064 verified none asserts the old
  behaviour. If you find yourself needing to change an existing assertion, STOP
  and report it.
- **Report your own mutation runs**: separately weaken the leading-zero rule,
  the fraction-digit rule, the exponent-digit rule, and the duplicate-key
  comparison, and show each trips its corresponding test. Restore by reversing
  each edit — never with `git checkout`, the tree is dirty with your own work.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe** (`#![forbid(unsafe_code)]` in every crate
   root).
2. **The 1 MiB dual-platform size budget is real.** `json.rs` and `proto.rs`
   ship. Debian is the binding platform at **788,784** with **259,792** to
   spare; step 1 came in byte-identical there, and this step should stay in the
   same neighbourhood. No new dependencies, no `HashSet`, no new generic
   instantiations. Claude measures on macOS + Debian before committing.
3. **Static error messages only.** Never interpolate host bytes into a parse
   error: protocol errors reach the status line, and the sanitizer is not a
   substitute for not putting attacker bytes there in the first place.
4. **Do not touch frame bounds or depth accounting** — `frame.rs:37`,
   `json.rs:162`.
5. **Tests are colocated**: `json.rs` uses an inline `#[cfg(test)] mod tests`
   (line 375); cross-crate protocol tests live in
   `crates/dun-plugin/tests/protocol.rs`. Match the local style.
6. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report — do not keep tuning.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
python3 -c "import ast,sys; ast.parse(open('hosts/check-host.py').read())"
```

The workspace baseline at `f5891a0` is **923 passed / 0 failed / 0 ignored**
with tmux present; report your own totals and say explicitly whether tmux was
available, because the PTY suite skips cleanly without it and a smaller total
reported as green would hide that.

Loop: edit → test → fix → rerun until green. Never claim a result without the
verbatim lines.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave changes in the
  working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network.
- Minimal diff: no drive-by reformatting, renames, or comment changes outside
  the task.
- You MUST paste real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. The `MAX_OBJECT_MEMBERS` evidence — the largest real object you found, where
   it is, and the constant you chose.
3. Verification — each command run, with exact verbatim output lines, plus the
   tmux availability statement.
4. Your four mutation runs, each with the test it tripped.
5. Stop-loss / open questions — where you stopped and why (empty if none).
