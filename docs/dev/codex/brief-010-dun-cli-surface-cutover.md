# Brief 010 — dun-cli live-render cutover to the Surface backend

Implementation brief. Renderer-replacement slice 4a: switch the dun-cli event
loop from driving a ratatui `Terminal` to driving `dun_ui::SurfaceRenderer`
(the public backend added in commit `b132059`). The ratatui **dependency
stays** — `dun-ui` still uses it, and its parity harness proves the Surface
output equals what ratatui would have produced, so this swap is safe. Retiring
the ratatui render path in `dun-ui` and dropping the dependency is a separate,
later slice (not this brief).

This brief touches the live terminal I/O boundary. The mechanical change is
well-scoped, but the real acceptance oracle is the tmux/PTY integration suite
(`cargo test -p dun-cli`), which launches the actual `dun` binary in a real
terminal and asserts on-screen text and colors. Those tests — not unit tests —
decide correctness. You MUST run them and confirm they actually ran (not
skipped for a missing tmux/expect); if they skip, say so explicitly and do not
claim the gate is green.

## Goal

`dun-cli` renders through `SurfaceRenderer` instead of ratatui. Introduce a
small `SurfaceBackend` that bundles the terminal writer and the renderer and
owns the per-frame byte protocol (hide cursor, write the diff, reposition and
show the cursor, flush). The event loop, `main.rs`, and `shell.rs` drive that
backend. No `use ratatui::` remains in `crates/dun-cli/src`. Behavior on
screen is unchanged: the full `cargo test -p dun-cli` suite (tmux + PTY +
terminal-grid) passes with the tests actually running.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-cli/src/terminal/event_loop.rs` — the render site. The
  `terminal.draw(|frame| { … build ui_frame … app.shell.render(frame,
  &ui_frame) })` closure builds `ui_frame` in renderer-agnostic code that must
  be **preserved verbatim**; only the surrounding draw call and the frame size
  source change. `frame.area()` becomes `crossterm::terminal::size()`.
- `crates/dun-cli/src/main.rs` (~lines 168–175) — constructs
  `TerminalGuard`, `TerminalColorRewrite`, `TerminalWriter`,
  `CrosstermBackend`, `Terminal`. Replace the backend/Terminal construction
  with the `SurfaceBackend`; keep the guard and color-rewrite. The trailing
  `terminal.show_cursor()?` becomes `backend.show_cursor()?`.
- `crates/dun-cli/src/terminal/shell.rs` — `handle_runtime_action` and
  `run_shell_escape` take `terminal: &mut Terminal<…>` and call
  `terminal.show_cursor()` / `terminal.clear()`. Retarget them to
  `&mut SurfaceBackend`, mapping `show_cursor()` → `backend.show_cursor()` and
  `clear()` → `backend.clear()` (which clears the screen AND invalidates the
  renderer so the next frame is a full repaint).
- `crates/dun-cli/src/terminal/sgr.rs` — `TerminalWriter` is the color-rewrite
  `io::Write` over stdout; the diff bytes and cursor sequences are written
  through it (SGR rewrite for low-capability terminals still applies).
- `crates/dun-cli/src/terminal/lifecycle.rs` — `TerminalGuard` owns raw
  mode / alternate screen / mouse; leave it as is (the backend does not manage
  terminal mode).
- `dun_ui::{SurfaceRenderer, RenderedFrame}` — `SurfaceRenderer::new()`,
  `.render(shell, ui_frame, width, height) -> RenderedFrame { bytes, cursor }`,
  `.invalidate()`. First frame and post-invalidate / size-change frames are
  full repaints; others diff.
- `crates/dun-cli/tests/{tmux_grid,terminal_grid,pty_smoke,msedit_*}.rs` — the
  acceptance oracle. They use `env!("CARGO_BIN_EXE_dun")`, so
  `cargo test -p dun-cli` rebuilds the binary and drives it.

## Specification

Add `crates/dun-cli/src/terminal/surface_backend.rs`:

```rust
pub(crate) struct SurfaceBackend {
    writer: TerminalWriter,
    renderer: SurfaceRenderer,
}
```

- `new(writer: TerminalWriter) -> Self` — `renderer: SurfaceRenderer::new()`.
- `draw(&mut self, shell: &UiShell, ui_frame: &UiFrame, width: u16, height: u16)
  -> io::Result<()>`:
  1. `let frame = self.renderer.render(shell, ui_frame, width, height);`
  2. Hide the cursor (`crossterm::cursor::Hide`) so it does not skate across
     the screen as cells paint.
  3. Write `frame.bytes` through `self.writer` (`write_all`).
  4. If `frame.cursor` is `Some((x, y))`, `MoveTo(x, y)` then
     `cursor::Show`; if `None`, leave it hidden.
  5. Flush the writer.
  Use `crossterm::queue!` for the cursor/`MoveTo` commands targeting
  `self.writer`, then one flush — do not `execute!` per command.
- `clear(&mut self) -> io::Result<()>` — `queue!(self.writer,
  terminal::Clear(ClearType::All))`, flush, then `self.renderer.invalidate()`
  so the next `draw` is a full repaint over the cleared screen.
- `show_cursor(&mut self) -> io::Result<()>` — `queue!(self.writer,
  cursor::Show)`, flush.
- `invalidate(&mut self)` — delegate to `self.renderer.invalidate()`.

Event loop (`event_loop.rs`):

- Replace the `terminal` parameter with `backend: &mut SurfaceBackend`.
- Each iteration: `let (width, height) = crossterm::terminal::size()?;` then
  build `ui_frame` exactly as today (the whole status/overlay assembly is
  unchanged — keep it verbatim, using `width`/`height` where `area.width`/
  `area.height` were used), then `backend.draw(&app.shell, &ui_frame, width,
  height)?;`.
- `Event::Resize(_, _)` must now act: `backend.clear()?;` (clears + invalidates
  so the next frame is a clean full repaint at the new size). The other event
  arms are unchanged.
- `handle_runtime_action(action, backend, app, guard)?` — thread the backend.

`main.rs`:

- Build `let writer = TerminalWriter::new(io::stdout(), color_rewrite.clone());`
  then `let mut backend = SurfaceBackend::new(writer);`. Remove the
  `CrosstermBackend`/`Terminal` construction and their imports. Pass
  `&mut backend` to `run_event_loop`. Replace the trailing
  `terminal.show_cursor()?` with `backend.show_cursor()?`.

`shell.rs`:

- Change the two function signatures from `terminal: &mut Terminal<…>` to
  `backend: &mut SurfaceBackend`; `terminal.show_cursor()?` →
  `backend.show_cursor()?`; `terminal.clear()?` → `backend.clear()?`. Remove
  the ratatui imports.

Do not touch `crates/dun-cli/Cargo.toml`: ratatui is still pulled transitively
by `dun-ui`, and the dependency line is retired in the later slice. Just remove
the now-unused `use ratatui::…` lines from the three dun-cli source files.

## Scope

- Files you MAY modify:
  - `crates/dun-cli/src/terminal/surface_backend.rs` (new);
  - `crates/dun-cli/src/terminal/mod.rs` (declare the module; export
    `SurfaceBackend` alongside the existing `pub(crate) use`s);
  - `crates/dun-cli/src/terminal/event_loop.rs`;
  - `crates/dun-cli/src/terminal/shell.rs`;
  - `crates/dun-cli/src/main.rs`.
- Files/areas you MUST NOT touch (defaults for every brief):
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`, `TODO.md`, `PLAN.md`,
    `docs/**`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock` (NO dependency changes —
    ratatui stays);
  - `crates/dun-ui/**` (the renderer and its parity harness are done),
    `crates/dun-cli/tests/**` (the oracle must stay unmodified — if a test
    fails, fix the code, never the test), the other dun-cli modules;
  - `vm-test/**`, `reference/**`, `hosts/**`, every other crate.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force;
   if you think `unsafe` is unavoidable, STOP and report.
2. **The 1 MiB dual-platform size budget is real.** Claude runs the
   dual-platform size gate on this change (it is runtime code). Keep the diff
   minimal; no new dependencies. ratatui is still linked via dun-ui, so expect
   roughly neutral size — do not attempt to drop it here.
3. **All untrusted text goes through the sanitizer.** You are not adding text
   paths — `ui_frame` is already built by sanitized code. Do not reroute or
   bypass it; keep the `ui_frame` assembly verbatim.
4. **`crates/dun-cli/src/main.rs` is the prelude hub.** Modules use
   `use crate::*`; if you add `SurfaceBackend` to the terminal module exports,
   the prelude picks it up — keep the export list consistent.
5. **Tests are layered and colocated.** The tmux/PTY/terminal-grid suites in
   `crates/dun-cli/tests/` are the acceptance oracle for this brief; do not
   modify them.
6. **Terminal-detection env is pinned in harnesses.** The tmux/PTY support
   already pins TERM/COLORTERM/LANG/LC_CTYPE/NO_COLOR; do not add environment
   assumptions in the backend.
7. **Stop-loss is real.** If a tmux/PTY test fails for the same reason twice
   after a genuine fix attempt, STOP and report the captured screen diff — do
   not weaken or edit the test.

## Verification (MANDATORY — you run it; iterate to green)

Run exactly these and paste results verbatim:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dun-cli --no-fail-fast
cargo test --workspace --no-fail-fast
```

The `-p dun-cli` run is the real gate. In your report, quote the individual
`tmux_grid`, `terminal_grid`, and `pty_smoke` test result lines showing they
ran and passed. If any of those suites SKIPPED (tmux or expect unavailable in
your environment), state that plainly — a skipped oracle is not a pass, and
Claude will run it on a machine that has tmux.

Loop: edit → test → fix → rerun, until green. Never claim a result without the
verbatim lines.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave all changes
  in the working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead. In particular, do NOT edit any file under
  `crates/dun-cli/tests/`.
- Full machine access, but touch NOTHING outside this repo, no network. The
  only commands you run are file edits within Scope, `cargo`, and `python3`
  for parsing output.
- Minimal diff: no drive-by reformatting, renames, or comment changes
  outside the task.
- You MUST paste the real verbatim verification output. If a run did not
  reach green (or a suite skipped), say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command run, with the exact verbatim output lines;
   explicitly call out whether the tmux/PTY suites ran or skipped, with their
   per-test result lines.
3. The finding / verdict.
4. Stop-loss / open questions — where you stopped and why (empty if none).
