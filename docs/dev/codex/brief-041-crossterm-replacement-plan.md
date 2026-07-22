# Brief 041 — Design plan for replacing crossterm with an in-house terminal layer (design only)

**Diagnostic/design brief. NO source change.** Produce a step-by-step
implementation plan; Claude evaluates it, then dispatches the steps as
implementation briefs and gates each. (Plan-first workflow — see CLAUDE.md.)

## Why (decided 2026-07-23)

1. **A root-caused upstream defect breaks dun on poll-fallback platforms.**
   mio's `poll(2)` fallback backend (`selector/poll.rs`, used on
   `target_os = "solaris"` and other non-epoll/kqueue platforms) clears a fd's
   interest after every readiness event (`poll_fd.events &= !poll_fd.revents`)
   and relies on mio's `IoSource::do_io` wrapper to reregister it. crossterm
   registers stdin as a raw `SourceFd` once, reads with its own `FileDesc::read`,
   and never reregisters — so after the first input batch, stdin's READABLE
   interest is gone forever and every later batch is silently lost. Verified
   end-to-end: truss on Solaris shows `pollsys` never reporting fd 0 again; a
   raw-mode `dd` control on the same tmux proves the OS path is fine; forcing
   `mio_unsupported_force_poll_poll` reproduces the identical two
   `tmux_logfilter` timeouts on macOS. Unfixed in crossterm 0.29/master.
2. **Upstream is semi-active** (no release since 0.29, 159 issues / 65 PRs,
   only small fixes merged), so a fix landing there is not a plan.
3. **Strategic fit:** ratatui is already retired for the in-house Surface
   backend; the terminal I/O layer is the last third-party piece of the UI
   stack. Replacing it drops crossterm/crossterm_winapi/winapi/mio/
   signal-hook-mio/parking_lot from the lockfile (42 → ~36), likely shrinks the
   binary (1 MiB dual-platform budget), and makes dun's whole terminal stack
   self-owned.

## Claude's decisions (bake these into the plan — not open questions)

- **Safe Rust only.** `#![forbid(unsafe_code)]` stays in every dun crate. Unix
  syscalls go through **rustix** (raw mode via `rustix::termios`, size via
  `tcgetwinsize`/TIOCGWINSZ, readiness via `rustix::event::poll`, plus
  read/write) and **signal-hook** for SIGWINCH. rustix and signal-hook are
  already in the lockfile (via crossterm); they stay, everything else crossterm
  pulled goes.
- **No Windows now, door stays open (msedit-style layering).** The design MUST
  split a **platform-neutral VT core** (escape emission, the input escape
  parser, event-loop semantics, the public event types) from a **small sys
  shim** (today: one Unix implementation; a future `windows.rs` console-mode
  shim could slot in without touching the core — msedit's `sys/unix.rs` 563
  lines vs `sys/windows.rs` 634 lines proves the shape). Do not implement or
  stub Windows; just keep the boundary clean.
- **The event loop polls the tty fd directly with level-triggered `poll(2)`.**
  No readiness abstraction layer, no interest bookkeeping. This kills the
  Solaris defect by construction; the two timing-out `tmux_logfilter` tests
  becoming green on Solaris (747/2 → 749/0, all four platforms 749/0) is the
  track's final acceptance criterion.
- **Own event types, crossterm-shaped.** Provide our own
  `KeyCode`/`KeyModifiers`/`KeyEvent`/`MouseEvent`/`MouseButton`/`Event` etc.
  as a compatible subset so the ~24 dun-cli files that consume them mostly just
  change import paths (`main.rs` is the prelude hub — re-export from there).
  Enumerate any semantic differences explicitly.
- **Bounded protocol surface.** Parse: classic xterm-family key sequences
  (CSI/SS3, modifiers, UTF-8 text), SGR mouse (1006), bracketed paste
  (200~/201~), CPR + DA1 (the startup probe already parses these — reuse or
  merge that bounded parser), and SIGWINCH-driven resize. Explicitly NOT
  supported: kitty keyboard protocol, modifyOtherKeys, legacy X10 mouse,
  win32-input-mode. The supported-terminal claim is the xterm family + tmux +
  screen (the SSH reality dun targets).

## What exists (read before planning)

- The crossterm surface is tiny and already isolated: 5 runtime files —
  `crates/dun-cli/src/terminal/{lifecycle,event_loop,surface_backend,ambiguous_width}.rs`
  + the `main.rs` re-exports. No other crate touches crossterm. Output usage is
  fixed escapes only (alt screen ?1049, bracketed paste ?2004, mouse
  ?1000/1002/1006, cursor hide/show/moveto, clear); input usage is
  `event::poll(250ms)`/`event::read`, `terminal::size()`, and raw-mode
  enable/disable. ~24 dun-cli files consume the event types.
- The startup ambiguous-width detector
  (`crates/dun-cli/src/terminal/ambiguous_width.rs`) has its own bounded CSI
  parser and currently uses `mio::Poll` — the new layer must let it use the
  same in-house readiness path so **mio is dropped entirely**.
- The test safety net: 749 workspace tests; the PTY harness answers the CPR/DA1
  probe and covers 8 TERM profiles; the tmux harness is ambiguous-width-aware;
  golden frame snapshots; three VMs (debian/freebsd/solaris).
- References available locally: `reference/msedit` (`crates/edit/src/vt.rs`
  parser, `crates/edit/src/sys/unix.rs` — sigaction SIGWINCH + poll + read +
  termios, including real-world quirks like retrying TIOCGWINSZ); crossterm
  0.28.1 source in `~/.cargo/registry/src/*/crossterm-0.28.1/` (its
  `event/sys/unix/parse.rs` is the reference for sequence coverage).

## The plan must address each

1. **Module/crate layout.** Where the VT core and the Unix sys shim live
   (inside `dun-cli/src/terminal/`? the platform-neutral parser in `dun-term`?),
   with the msedit-style boundary explicit. Propose with pros/cons and a
   recommendation.
2. **Complete call-site inventory.** Every crossterm item used, `path:line`,
   and what replaces it (escape constant, rustix call, new event type, new
   loop). Include the `Cargo.toml`/lockfile delta and what happens to the
   detector's mio usage.
3. **The input parser.** Coverage matrix: every sequence dun must parse (keys
   incl. modifiers/arrows/function/home-end-pgup-pgdn/tab-backtab/enter-esc-
   backspace, UTF-8 text, SGR mouse incl. drag/scroll, bracketed paste, CPR,
   DA1, focus events if dun uses them — check), the state machine shape
   (bounded buffers like the detector's), Esc-vs-Esc-sequence disambiguation
   under `poll`, and how partial reads across poll ticks are buffered. Map each
   row to the crossterm parser function and/or msedit vt.rs location it mirrors.
4. **The event loop + lifecycle.** Raw-mode enter/exit via rustix with the same
   panic-safety the current `TerminalGuard` has; SIGWINCH → Resize via
   signal-hook (self-pipe or flag+poll-wake — msedit uses a flag; we need the
   250ms tick semantics of `event::poll(Duration)` preserved); `terminal::
   size()` replacement; how the ambiguous-width probe integrates (same fd,
   before the loop starts — today's sequencing).
5. **Ordered steps (likely 4–6), each its own implementation brief.** Sequence
   conservatively: output/escape side and lifecycle first (mechanical,
   Narrow-byte-identical-style gates), event types + import migration next,
   input parser + event-loop cutover last. For each step: files/functions, how
   existing behavior stays byte-identical where it must, the gate tests, and
   what can regress if done wrong. State where the dual-platform size
   measurements land.
6. **Test plan.** How the existing PTY/tmux suites gate each step; what new
   parser unit fixtures are needed (sequence corpus — consider cribbing
   crossterm's own test vectors); the Solaris acceptance (the two timeouts
   going green); any harness changes (should be none — the harness speaks VT,
   not crossterm).
7. **Risks / open questions** for Claude to decide — including any crossterm
   behavior dun currently relies on that is NOT in the bounded protocol surface
   above (search for it, don't assume), and any place where the event-type
   subset would change observable behavior (e.g. key repeat, release events,
   modifier quirks in tmux).

## Scope

- Files you MAY modify: **NONE — design only.** Leave the tree clean
  (`git status --short` empty when done). Read anything, including
  `~/.cargo/registry` sources and `reference/msedit`; run read-only commands;
  do not `cargo build`/edit.

## Hard rules

- Do NOT edit any source file, commit, branch, push, or touch git.
- Base every claim on real files (`path:line`); do not hand-wave the parser
  coverage matrix or the call-site inventory.
- Safe Rust only in the design — if some piece seems to require `unsafe`, that
  is an open question for Claude, not a design choice.

## Report format (final message)

The seven-part plan above, concrete enough that each step could become its own
implementation brief without further discovery.
