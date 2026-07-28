# Brief 043 — crossterm replacement step 2: Unix sys shim, raw lifecycle, size, probe polling

Implementation brief. **Step 2 ("Brief 2") of the accepted plan for brief 041**
(`docs/dev/codex/brief-041-crossterm-replacement-plan.md`). Step 1 (own output
escapes) landed at `cf1a5b6`. This step moves raw mode, terminal size, and the
startup probe's readiness loop onto an in-house Unix sys shim built on rustix.
The INPUT path (crossterm `event::poll`/`event::read` and the event types)
stays on crossterm — steps 3–5 are separate.

## Claude's decisions (bake these in)

- **tty acquisition policy:** `sys::Terminal::open` uses **stdin when
  `isatty(stdin)`**, else opens `/dev/tty` read+write via safe
  `std::fs::OpenOptions`. This preserves crossterm's stdin-or-/dev/tty policy.
- **POLLNVAL is a hard error.** Empirically verified on macOS (2026-07-23):
  `poll(2)` on a real pty stdin reports `IN` correctly, but on a `/dev/tty` fd
  it always returns `POLLNVAL`. If any poll on the input fd reports
  NVAL/ERR/HUP with no readable data, fail with a clear `io::Error` (message
  must mention that a real terminal on stdin is required). Do NOT add a
  select() fallback or a macOS special case — on macOS a redirected-stdin
  session was already broken under crossterm's default mio backend (its
  `use-dev-tty` select feature is not enabled in dun), so nothing regresses on
  any platform: Linux/FreeBSD/Solaris can poll `/dev/tty` fine and keep
  working.
- **Size fallback (plan risk #10, accepted):** `Terminal::size()` =
  `tcgetwinsize`; if either dimension is zero, retry up to ten times with
  bounded 10–90 ms sleeps, then fall back to 80×24; propagate real ioctl
  errors. No `tput`, no process spawn.
- **rustix features:** `default-features = false, features = ["std", "stdio",
  "termios", "event"]` (workspace dep + dun-cli `workspace = true`). Remove
  the direct `mio` declarations from BOTH manifests — mio stays in the
  lockfile transitively via crossterm until step 5.
- **signal-hook is NOT added here** — SIGWINCH moves in step 4 with the event
  reader. Resize keeps flowing through crossterm's event source this step.

## Exact change

1. **New `crates/dun-cli/src/terminal/sys/mod.rs` + `sys/unix.rs`** —
   `sys/mod.rs` re-exports the Unix implementation under `cfg(unix)` (no
   Windows stub). `sys/unix.rs` owns, in safe Rust only:
   - `Terminal::open() -> io::Result<Arc<Terminal>>` per the acquisition
     policy above; expose the fd for probe/loop use and `is_tty` facts.
   - Raw mode: `enter_raw` = `tcgetattr` → clone + retain the EXACT original
     `Termios` → `make_raw` → `tcsetattr(OptionalActions::Now)` → mark the
     snapshot active only after success. `restore_raw` clones the snapshot
     outside any lock, applies it, clears it only after success. Poisoned
     locks are recovered (`unwrap_or_else(PoisonError::into_inner)`), never
     unwrapped — the panic hook calls this path.
   - `size()` per the decision above.
   - `poll_readable(deadline: Instant) -> io::Result<Readiness>`: build ONE
     fresh `rustix::event::PollFd` with `PollFlags::IN` per call, call
     `rustix::event::poll` with the remaining time, discard the PollFd after.
     On `EINTR`, recompute the remaining deadline and retry. NVAL/ERR/HUP →
     the hard-error path (after consuming any simultaneously readable bytes).
     No registration, no retained interest state of any kind.
   - `read(&mut [u8])` on the same fd.
2. **`terminal/lifecycle.rs`** — `TerminalGuard` and
   `install_panic_terminal_restore` operate on the shared `Arc<Terminal>`
   (restore handle) instead of crossterm's `enable_raw_mode`/
   `disable_raw_mode`. Preserve exactly: enter order (raw → alt screen →
   paste → optional mouse), failed-mode-entry rolls raw back before
   returning, suspend/drop/panic attempt every cleanup step and report only
   the first error, resume re-captures the then-current termios before
   re-entering. The panic hook must be non-panicking throughout (best-effort
   locks) and still restore before chaining to the default hook.
3. **`terminal/ambiguous_width.rs`** — replace the mio Poll/SourceFd loop
   with `terminal.poll_readable(deadline)` + `terminal.read(...)` on the
   SAME shared terminal. The probe bytes, 500 ms budget, bounded parser
   (256/32), CPR/DA1 logic, and every unit test's expectations stay
   IDENTICAL. Eligibility keeps today's rule (UTF-8 + stdin tty + stdout
   tty); redirected stdin must NOT newly enable the probe.
4. **`terminal/event_loop.rs`** — replace `crossterm::terminal::size()` with
   `Terminal::size()`; the `event::poll`/`event::read` calls stay.
5. **`main.rs` (`run_tui`)** — startup composition becomes: `Terminal::open`
   → install panic restore handle → `TerminalGuard::enter(terminal.clone(),
   mouse off)` → `detect_ambiguous_width(&terminal, encoding)` → profile
   composition (unchanged) → backend → loop. Keep the prelude hub imports
   consistent.
6. **Manifests** — workspace `Cargo.toml`: remove `mio`, add `rustix` as
   decided; `crates/dun-cli/Cargo.toml`: `mio` → `rustix`. `Cargo.lock`
   regenerates (mio remains via crossterm).
7. **Docs (same-change rule)** — `AUDIT.md`: update the bounded
   terminal-response invariant entry to name the in-house poll path instead
   of mio; `docs/dev/terminal-compatibility-checks.md`: document the tty
   acquisition policy + the macOS `/dev/tty` POLLNVAL limitation (clear
   startup error, stdin-tty required on macOS when redirected).

## Scope

- Files you MAY modify: the two new `sys/` files;
  `terminal/{mod,lifecycle,ambiguous_width,event_loop}.rs`; `main.rs`;
  `Cargo.toml` + `crates/dun-cli/Cargo.toml` + `Cargo.lock` (exactly the
  mio-remove/rustix-add); `AUDIT.md` + `docs/dev/terminal-compatibility-checks.md`
  (the two entries above); colocated unit tests (including
  `terminal/lifecycle.rs` tests — see Deliverable).
- Files/areas you MUST NOT touch: `terminal/vt/**` (step 1 is done),
  `terminal/input.rs`, the crossterm `event::poll`/`event::read` usage,
  `crates/dun-cli/tests/**`, any other crate, any other doc, `.git`,
  `i18n/**`, `hosts/**`, `vm-test/**`, `reference/**`.

If a change needs a file outside Scope, STOP and report.

## Deliverable

- The sys shim + migrated lifecycle/probe/size/startup + manifest swap +
  the two doc entries.
- Unit tests: a testable restore executor for the guard — inject a recording
  writer + recording raw-ops so tests assert (a) failed mode entry still
  rolls raw back; (b) suspend/drop attempt EVERY later cleanup step after an
  earlier one fails and return the first error; (c) the termios snapshot is
  captured before modification and restored exactly. Probe unit tests keep
  passing unchanged (pure parser tests).
- Prove load-bearing (run, paste, restore): (a) make restore_raw skip
  applying the snapshot → a test fails; (b) make the cleanup path return
  early on the first error (skipping later steps) → a test fails; (c) swap
  the first-error retention to last-error → a test fails.

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`) — rustix + std only; no libc.
2. The panic hook runs under `panic = "abort"` in release: `Drop` never
   fires there, so the hook itself must do the full restore, without
   panicking, without deadlocking (a panic while a lifecycle lock is held
   must still restore).
3. Capture termios BEFORE `make_raw` mutates it; restore the EXACT snapshot
   (not a freshly-made-raw-then-unraw approximation). Resume must re-capture
   (the user's shell may have changed settings while suspended).
4. The probe and the future event loop share ONE fd — do not open a second
   `/dev/tty` for the probe.
5. PTY suites are the behavioral oracle: pty_smoke panic/suspend/resume and
   all tmux suites must pass unchanged (they provide real ttys, so the
   stdin path is exercised; the /dev/tty path has no automated test — state
   what you verified manually).
6. Stop-loss: same failure twice, or an out-of-scope file needed → STOP,
   report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Note pty_smoke (all 9 cases incl. panic + suspend/resume) and tmux suites
explicitly. Claude runs the dual-platform size gate and release smoke at the
gate (macOS now; Debian batched — VM currently down).

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working
  tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why.
2. Verification — each command's verbatim output (suite counts; PTY/tmux
   noted).
3. Mutation evidence — the three load-bearing runs, verbatim.
4. Verdict.
5. Stop-loss / open questions (empty if none).
