# Brief 039 — Auto-detect step 1: startup ambiguous-width probe

Implementation brief. **Step 1 of the plan in
`docs/dev/codex/brief-038-ambiguous-width-autodetect-plan.md`** (read it first).
Steps 2–3 (mode-aware test harness; cross-platform + measurement) are separate;
do NOT do them here.

## Goal

At startup, on a real terminal, probe whether ambiguous-width glyphs render 1 or
2 columns and set `TerminalProfile.ambiguous_width` from the result — so an
unconfigured user on a wide terminal gets correct rendering. The explicit config
`terminal.ambiguous-width` still wins. Detection failure / non-tty / ASCII ⇒
Narrow (today's default). This step implements the **detector + startup
composition + its unit tests only**; it does NOT change the tmux/PTY test
harness or the grid parser (step 2).

## Claude's decisions (bake these in)

- **Probe glyph: `─` (U+2500)**, not `…` — it is the actual UI border glyph and
  is verified 2-wide on Solaris. Probe bytes: `b"\r\xe2\x94\x80\x1b[6n\x1b[c"`
  (`\r`, `─`, DSR cursor-position request, DA1 sentinel).
- **Readiness: a local `mio::Poll` + `mio::unix::SourceFd` around stdin**, used
  before crossterm initializes its event reader. `mio` is already in `Cargo.lock`
  via crossterm; promote it to a **direct** dependency (approved). No unsafe FFI,
  no blocking orphan thread, no global/thread-local.
- **Deadline: one 500 ms overall budget** (not per-read).
- **ASCII startup skips the probe** (returns Narrow).

## Design (implement exactly the plan's detection design)

- **Where:** in `run_tui` (`main.rs`), after `TerminalGuard::enter(false)` (raw
  mode + alternate screen active, mouse capture off so reports can't interfere)
  and **before** building `TerminalColorRewrite`/writer/backend and the event
  loop. This precedes the first frame and the first crossterm event read, so
  there is no competing reader. Enter the guard with mouse disabled here; the
  event loop enables configured mouse capture as it already does.
- **Probe (new `crates/dun-cli/src/terminal/ambiguous_width.rs`):**
  - Return `Narrow` immediately if stdin OR stdout is not a tty, or if the
    effective encoding is ASCII.
  - Write + flush the probe bytes.
  - Read with a local `mio::Poll`/`SourceFd` loop under one 500 ms deadline.
    Parse a **bounded** stream: ≤256 bytes total, ≤32 bytes per CSI. Handle
    fragmented/coalesced input; ignore unrelated bounded bytes.
  - Accept CPR only as `ESC [ row ; col R` with `row >= 1`: **col 2 ⇒ Narrow,
    col 3 ⇒ Wide**, other cols invalid.
  - Keep the CPR candidate but only finish on a syntactically valid **DA1**
    (`ESC [ ? … c`). DA1-without-valid-CPR, malformed input, buffer exhaustion,
    timeout, or I/O error ⇒ `Narrow`.
  - On every path after writing, queue `cursor::MoveToColumn(0)` +
    `terminal::Clear(ClearType::CurrentLine)` and flush (the first render is a
    full repaint on the alternate screen, so nothing lingers).
  - Return an `AmbiguousWidth`.
- **Profile composition:** set the probe result into the stored **detected**
  profile's `ambiguous_width` (the base config reload rebuilds from), then
  recompute `app.shell.profile = terminal_overrides.apply_to(detected)` so the
  config option keeps precedence. Preserve the `TerminalOverrides` from
  `LoadedConfig` before it moves into `AppState`. Keep
  `TerminalProfile::new`/`from_capabilities` returning Narrow.

## Scope

- Files you MAY modify:
  - `Cargo.toml` (workspace: add `mio` with the `os-poll`/`os-ext` features
    crossterm already enables) and `crates/dun-cli/Cargo.toml` (add the direct
    `mio` dep); `Cargo.lock` as `cargo` regenerates it — **the allowed manifest
    edits, for this dependency only**
  - new `crates/dun-cli/src/terminal/ambiguous_width.rs`
  - `crates/dun-cli/src/terminal/mod.rs` (module wiring + re-export)
  - `crates/dun-cli/src/main.rs` (`run_tui` sequencing) and a small typed
    `AppState`/shell helper in `crates/dun-cli/src/app/` for the composition
  - `crates/dun-cli/src/terminal/lifecycle.rs` only if `enter` needs the
    mouse-off variant it already supports
  - detector unit tests (colocated)
  - `AUDIT.md` (record the bounded terminal-response invariant) and
    `docs/dev/terminal-compatibility-checks.md` (startup probe behavior) — required
    by AGENTS.md for a behavior change
- Files/areas you MUST NOT touch:
  - `crates/dun-cli/tests/**` and `crates/dun-cli/tests/support/**` — the
    tmux/PTY harness and grid parser are **step 2**. (This means PTY/tmux tests
    that spawn the editor are not yet probe-aware; see "Expected interim" below.)
  - `crates/dun-ui/**`, `crates/dun-core/**`, `crates/dun-config/**`,
    `crates/dun-term/**` logic (you only read `TerminalProfile`/`AmbiguousWidth`
    and `TerminalOverrides`)
  - any other `Cargo.toml`; `.git`, `docs/**` except the two files listed,
    `i18n/**`, `vm-test/**`, `reference/**`, `hosts/**`

If composition needs a file not listed, STOP and report.

## Expected interim (not a failure)

Because the harness is untouched, an editor spawned in a PTY/tmux without a probe
responder will hit the 500 ms deadline and fall back to Narrow. That keeps the
existing Narrow tests passing (just slower by ≤500 ms each); step 2 adds the
responder + mode-aware parsing. If any spawn-the-editor test *fails* (not merely
slows), STOP and report — that's a real regression.

## Deliverable

- The detector, startup composition, and `mio` dependency.
- Colocated unit tests (pure, no real terminal): exact probe output bytes;
  fragmented Narrow (`ESC[1;2R`) and Wide (`ESC[1;3R`) CPR each followed by DA1;
  DA1 with no CPR ⇒ Narrow; CPR with no DA1 (deadline) ⇒ Narrow; malformed /
  out-of-range col / oversized ⇒ Narrow; and the three precedence cases (config
  Narrow / config Wide / config unset with detected Wide). A reload test proving
  the detected base's ambiguous_width survives a config reload.
- Prove load-bearing: mutation that swaps col 2/3, or finishes on CPR before
  DA1, must fail the relevant test.
- `AUDIT.md` + `docs/dev/terminal-compatibility-checks.md` updated.

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`) — `mio` is safe; do not reach for
   FFI/global/thread-local.
2. Narrow is sacred: no Narrow golden snapshot may change; config precedence and
   `from_capabilities`-Narrow are unchanged.
3. `dun-cli/src/main.rs` is the prelude hub — keep its imports consistent.
4. 1 MiB dual-platform budget: `mio` is already compiled via crossterm, but
   Claude runs the size gate; keep the detector small, add nothing else.
5. Stop-loss: same failure twice, or an out-of-scope file needed → STOP, report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Note in the report if any spawn-the-editor suite got slower (the 500 ms
fallback) — that is expected until step 2; a *failure* there is not.

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why.
2. Verification — each command's verbatim output (suite counts; note any
   slowdown and skips).
3. Verdict.
4. Stop-loss / open questions (empty if none).
