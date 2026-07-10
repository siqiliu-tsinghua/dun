# brief-003 — CI-grade fixture host + protocol failure-mode test matrix

## Goal

`crates/dun-plugin` has unit tests but no integration coverage of the
protocol failure modes the client must survive (docs/plugin-protocol.md
§Completion Criteria item 4). When you are done: the fixture host is a
proper package binary reachable from integration tests, it can be told to
misbehave in each named way, and `crates/dun-plugin/tests/protocol.rs`
proves the client's observable behavior for every row of the matrix below.
No client/source behavior changes.

## Context pointers

- Read `AGENTS.md` and `docs/plugin-protocol.md` (Transport, Process Launch
  Rules, Output Validation) first.
- Key files:
  - `crates/dun-plugin/src/client.rs` — `HostClient` under test
    (`launch`, `request_highlight`, `shutdown`, `PluginError` variants).
  - `crates/dun-plugin/src/proto.rs` — envelope/Policy (note
    `Policy { timeout, max_frame_bytes, max_spans, max_stderr_bytes,
    max_diagnostics, max_snapshot_lines }`); tests pass short timeouts.
  - `crates/dun-plugin/examples/fixture_host.rs` — current fixture; you
    will MOVE it to `src/bin/fixture-host.rs` (delete the examples/ copy)
    so integration tests can locate it via
    `env!("CARGO_BIN_EXE_fixture-host")`.
- Acceptance is mechanical: the new integration tests decide.

## Scope

- Files you MAY modify:
  - `crates/dun-plugin/src/bin/fixture-host.rs` (new; moved + extended),
  - `crates/dun-plugin/examples/fixture_host.rs` (delete),
  - `crates/dun-plugin/tests/protocol.rs` (new),
  - `crates/dun-plugin/Cargo.toml` (ONLY if a `[[bin]]` section is needed;
    `src/bin/` auto-discovery may make it unnecessary).
- You MUST NOT modify `crates/dun-plugin/src/*.rs` (the client under
  test). If a test exposes what you believe is a real client defect, mark
  that test `#[ignore = "records suspected defect: <one line>"]`, keep it,
  and report it prominently.
- Plus the standard MUST-NOT list from `docs/dev/codex/TEMPLATE.md`.

## Deliverable

1. Fixture host as `src/bin/fixture-host.rs`. Keep existing behavior
   (hello-ack, echo-revision response, `stale-test`, `slow-test`,
   `cancel-request` ignore, shutdown exit). Add misbehavior triggers:
   - handshake-level, selected by argv[1]: `bad-version` (hello-ack with
     `"v": 9`), `bad-trust` (unknown trust string), `no-ack` (reply
     `error` instead of hello-ack), `garbage-frame` (write 4-byte length
     prefix promising more bytes than it sends, then exit), default = 
     normal.
   - request-level, selected by the request payload `language` field:
     `crash-test` → `std::process::exit(2)` without replying;
     `flood-test` → a response with `max_spans + 1` well-formed spans
     (hardcode 4097 to exceed the default policy);
     `badcoord-test` → one span with `end_col` far beyond the line;
     `badstyle-test` → one span with style `"blink"`;
     `wrong-id-test` → a response whose `request_id` is `request_id + 1`;
     `bigframe-test` → a response payload padded (e.g. a long string
     field) so the frame exceeds the client's `max_frame_bytes`;
     `diag-flood-test` → `max_diagnostics + 1` diagnostics then a normal
     response; `stderr-test` → write ≥ 64 KiB to stderr, then reply
     normally; plus the existing `stale-test` and `slow-test`.
2. `tests/protocol.rs` integration tests (one behavior per test), each
   launching the fixture via `env!("CARGO_BIN_EXE_fixture-host")` with a
   `Policy` whose `timeout` is ≤ 500 ms where relevant:
   - happy path: launch + one validated span round trip + clean shutdown;
   - handshake: `bad-version` → `PluginError::Protocol(UnsupportedVersion)`;
     `bad-trust` → `Handshake`; `no-ack` → `Handshake`; `garbage-frame` →
     an error (Frame/HostClosed), not a hang;
   - `crash-test` → `HostClosed` (or the actual variant — assert what the
     client really returns and name it);
   - `flood-test` → `PolicyViolation`; `badcoord-test` → `PolicyViolation`;
     `badstyle-test` → `PolicyViolation`; `wrong-id-test` →
     `PolicyViolation`; `bigframe-test` → `Frame(Oversized…)`;
     `diag-flood-test` → `PolicyViolation`; `stale-test` →
     `StaleRevision { .. }`;
   - `slow-test` with a 100 ms timeout → `Timeout`, and the test returns
     promptly (well under the fixture's sleep);
   - `stderr-test` → request still succeeds; this documents that stderr
     flooding does not break the protocol path.
   Every test must assert the error VARIANT (use `matches!`), not just
   `is_err()`.

## dun pitfalls (read twice)

`docs/dev/codex/TEMPLATE.md` §dun pitfalls items 1, 5, 7. The fixture
host binary and tests do not ship in the release binary; still no new
dependencies. Integration tests must not depend on wall-clock generosity:
derive waits from the `Policy` timeout you set, and keep every test under
a few seconds.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dun-plugin
cargo test --workspace --no-fail-fast
```

Paste the `test result:` lines verbatim.

## Hard rules

All of `docs/dev/codex/TEMPLATE.md` §Hard rules apply verbatim.

## Report format (your final message)

Per `docs/dev/codex/TEMPLATE.md` §Report format; list each matrix row and
the error variant the client actually produced.
