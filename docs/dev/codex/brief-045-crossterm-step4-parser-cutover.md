# Brief 045 — crossterm replacement step 4: VT parser, SIGWINCH reader, event-loop cutover

Implementation brief. **Step 4 ("Brief 4") of the accepted plan for brief 041**
— the decisive step. Steps 1–3 landed (`cf1a5b6`, `919a98f`, `d8f17c4`):
output, sys shim, and owned event types are in-house; crossterm remains only
as the live input parser behind the temporary adapter in `event_loop.rs`.
This step replaces it: dun parses its own input bytes and reads through the
sys shim's direct `poll(2)`. After it, **no crossterm name appears anywhere
in `crates/dun-cli/src`** (the dependency leaves the manifests in step 5).

This is the step that fixes the Solaris input defect (the second `tmux
send-keys` batch lost through crossterm+mio's poll-fallback interest
clearing): the two timing-out Solaris `tmux_logfilter` tests going green is
part of the acceptance.

## Claude's decisions (bake these in — already recorded, do not reopen)

- Bare-ESC disambiguation deadline: **100 ms**, one named internal constant.
- Paste cap: **16 MiB**; an over-cap paste is discarded through its exact
  terminator and produces NO event (never a truncated paste).
- Parser promises the classic **F1–F20** tables only.
- Parser-originated key events are always `Press`.
- Malformed non-paste UTF-8 / unknown-or-excluded CSI: consumed and dropped,
  NEVER leaked as text. `ESC [ M` (X10) discards its three data bytes.
  kitty CSI-u/colon forms, modifyOtherKeys, rxvt mouse: consumed, dropped.
- `CSI … R` is context-sensitive: **Probe mode → CPR; Input mode → F3**
  (`CSI R`/`CSI 1;mR`), so Shift+F3 keeps working.
- Resize: `signal_hook::flag::register(SIGWINCH, Arc<AtomicBool>)`; the
  reader consumes the flag, queries `Terminal::size()`, emits
  `Event::Resize`. Coalescing + ≤250 ms observation delay is accepted; NO
  self-pipe. Retain the `SigId` and unregister on reader drop.
- Read ≤**1,024 bytes** per readiness; do not read again until the event
  queue is drained. CSI body cap 30 bytes (32 with `ESC [`); probe response
  cap stays 256 bytes.

## Exact change

1. **New `terminal/vt/parser/{mod.rs,keys.rs,mouse.rs,tests.rs}`** — one
   incremental, bounded, allocation-stable parser with modes
   `Input | Probe`, state machine per the accepted plan:
   `Ground / Escape{deadline} / Ss3 / Csi{bytes,len} / OversizedCsi /
   Utf8{bytes,len,expected} / Paste{…} / DiscardPaste{…} / DiscardX10{…}`.
   Implement the full coverage matrix (plan part 3): plain UTF-8 (fragmented
   scalars buffered; uppercase carries SHIFT), controls (CR→Enter, TAB→Tab,
   DEL→Backspace, NUL→Ctrl+Space, 01–1A→Ctrl+a..z, 1C–1F→Ctrl+4..7),
   bare/Alt-Escape (100 ms deadline; `ESC ESC` emits one Esc and re-arms),
   unmodified + modified navigation (CSI/SS3 A–D/H/F, masks 1–8, legacy
   single-mask forms), tilde editing keys (1/7 Home, 2 Insert, 3 Delete,
   4/8 End, 5/6 Page, `;m` modifiers, `CSI Z` → Shift+BackTab), function
   keys (SS3 P–S; CSI P–S incl. `1;m`; legacy `CSI [[A–E`; tilde
   11–15/17–21/23–26/28–29/31–34 → F1–F20, with modifiers), SGR mouse
   (`CSI <Cb;Cx;CyM|m`, checked u8/u16, nonzero coords → zero-based,
   buttons/drag/move/scroll ×4, modifier bits), bracketed paste (only the
   exact fragmented-safe `ESC [ 201 ~` ends it; lossy string within cap),
   CPR + DA1 (Probe mode; keep today's fail-closed semantics), explicit
   discards per the decisions above. Reuse/merge the detector's existing
   bounded CSI framing rather than duplicating it.
2. **New `terminal/event_reader.rs`** — `EventReader` owning the parser, the
   SIGWINCH flag, and the shared `Arc<sys::Terminal>`:
   `next_event(timeout) -> io::Result<Option<Event>>`. Wait order per call:
   drain parser queue → consume resize flag (size + Resize event) → poll
   until the earlier of caller deadline and any pending bare-ESC deadline →
   one bounded read → feed parser. EINTR: recheck resize, recompute
   deadline. Register SIGWINCH at construction (after the startup probe),
   unregister on drop.
3. **`terminal/ambiguous_width.rs`** — the probe uses the shared parser in
   `Probe` mode through the same fd/poll path (behavior identical: probe
   bytes, 500 ms, CPR col 2/3, DA1 sentinel, fail-closed Narrow; every
   existing unit test keeps its expectations).
4. **`terminal/event_loop.rs`** — cut over to
   `EventReader::next_event(Duration::from_millis(250))`; delete the
   temporary crossterm adapter and every `crossterm::` use. One event per
   iteration, same dispatch order.
5. **`main.rs`** — construct the `EventReader` after the probe, thread it to
   the loop; imports stay consistent.
6. **Manifests** — add `signal-hook` (workspace:
   `{ version = "0.3", default-features = false }`; dun-cli
   `workspace = true`). Touch NOTHING else in the manifests — crossterm
   stays declared until step 5. `Cargo.lock` regenerates for signal-hook
   only.
7. **New tmux resize test** (in `crates/dun-cli/tests/tmux_grid.rs`): start
   dun in a fixed-size pane, `tmux resize-pane` (e.g. 80×24 → 100×30), and
   assert the next captured frame adopts the new dimensions (border box
   spans the new width under the pane's measured ambiguous-width mode).

## Scope

- Files you MAY modify: new `terminal/vt/parser/*` and
  `terminal/event_reader.rs`; `terminal/{mod,event_loop,ambiguous_width}.rs`;
  `terminal/vt/mod.rs`; `main.rs`; `Cargo.toml` +
  `crates/dun-cli/Cargo.toml` + `Cargo.lock` (signal-hook add ONLY);
  `crates/dun-cli/tests/tmux_grid.rs` (the new resize test only); colocated
  unit tests.
- Files/areas you MUST NOT touch: `terminal/vt/{output,event}.rs` beyond
  re-exports, `terminal/sys/**` (its API is sufficient; if it truly is not,
  STOP and report), `terminal/{lifecycle,input}.rs`, `app/**`,
  `crates/dun-cli/tests/**` except the one new resize test,
  `crates/dun-cli/src/tests/**`, other crates, docs, `.git`, `i18n/**`,
  `hosts/**`, `vm-test/**`, `reference/**`.

If a change needs a file outside Scope, STOP and report.

## Deliverable

- Parser + reader + cutover + the resize test.
- **Parser unit corpus with independent oracles** (literal input bytes →
  literal owned events; never derive expecteds from the implementation):
  every mapping row above; every accepted sequence split at EVERY byte
  boundary; multiple events in one 1,024-byte batch; fake-clock tests for
  99/100/101 ms bare-ESC (no wall-clock sleeps); paste terminator split at
  every byte, exactly-at-cap, over-cap discard, embedded ESC/CSI, invalid
  UTF-8; SGR mouse zero-coordinate rejection and numeric overflow; CPR/DA1
  valid/malformed/fragmented/missing-sentinel; Input-vs-Probe `R`; discard
  cases emit nothing; bounded-state assertions.
- Grep gate in the report: `grep -rn crossterm crates/dun-cli/src` → ZERO
  hits.
- Prove load-bearing (run, paste, restore each): (a) Input-mode `CSI 1;2R`
  routed to CPR/dropped instead of Shift+F3 → a test fails; (b) the reader
  reads again before the queue drains (or retains poll state) → the
  repeated-batch test fails; (c) ESC deadline changed to fire immediately →
  the 99 ms fake-clock test fails; (d) paste terminator accepts a prefix
  fragment (`ESC [ 201` without `~`) → a split-point test fails; (e) resize
  flag never consumed → the resize test fails.

## dun pitfalls (read twice)

1. Safe Rust only. No busy-wait: every wait goes through `poll` with a
   computed timeout (sub-ms remainders round up, as `sys::poll_readable`
   already does).
2. The PTY harness answers the probe (Narrow/Wide/no-response cases at
   `tests/support/pty.rs`) and sends input in separate batches — it is the
   behavioral oracle for the cutover. The no-response case must still reach
   the 500 ms Narrow fallback.
3. tmux_logfilter on kqueue/epoll platforms must stay green THROUGH the
   cutover — those four tests catch chunking/ordering regressions in the
   new reader immediately.
4. Do not regress startup ordering: probe before SIGWINCH registration and
   before the first frame (`main.rs:run_tui` sequencing from step 2).
5. An `Err` from `next_event` must propagate (NVAL/ERR/HUP policy from
   step 2), not be swallowed into a timeout.
6. Stop-loss: same failure twice, or an out-of-scope file needed → STOP,
   report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Note pty_smoke (all 9), tmux_grid (5 + the new resize test), and
tmux_logfilter (4) explicitly, plus the probe-latency observation (the PTY
responder path must NOT pay the 500 ms fallback). Claude runs the
dual-platform size gate, release smoke, and the four-platform VM round
(including the Solaris acceptance) at the gate.

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working
  tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why.
2. Verification — verbatim outputs (suite counts; PTY/tmux/probe-latency
   noted) + the zero-crossterm grep gate.
3. Mutation evidence — the five load-bearing runs, verbatim.
4. Verdict.
5. Stop-loss / open questions (empty if none).
