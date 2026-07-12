# CLAUDE.md

Session guidance for Claude Code in this repository. The authoritative
contribution rules and invariants live in [AGENTS.md](./AGENTS.md); read it
before changing behavior. This file adds orientation, the binding constraint,
and the active working plan. Keep all three in sync.

## Quick Orientation

Rust 1.85 workspace, five crates under `crates/`:

- `dun-core`: buffers, undo/redo, search, tiled workspace state, and the
  typed `EditorCommand` enum (`src/command.rs`, ~112 variants — the
  mechanical enumeration of user-visible features).
- `dun-term`: terminal capability profiles, color/glyph fallback, themes.
- `dun-config`: typed config, keymap, command-id parsing, validation.
- `dun-ui`: backend-neutral frame model and `ratatui` rendering.
- `dun-cli`: terminal lifecycle, event loop, command application (the
  largest crate, ~18k LOC — most UX weight lives here).

Docs are load-bearing in this repo: any behavior or architecture change must
update the matching document in the same change (AGENTS.md lists document
responsibilities). Feature removals are full-trail diffs: code + command ids +
keymap defaults + menu entries + help text + tests + README/docs paragraphs.

## Build, Test, Gates

```text
scripts/release-build.sh                     # budget measurement build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

`scripts/release-build.sh` is the build-std budget contract (decided
2026-07-10): RUSTC_BOOTSTRAP=1 + `-Zbuild-std=std,panic_abort`
`-Zbuild-std-features=` on stable 1.85; needs rust-src (present on both
platforms). Panic hooks and messages verified working under it. Plain
`cargo build --release` stays a dev build only.

Runtime-code commits must pass fmt, clippy, tests, the release smoke
checklist ([docs/release-smoke-checklist.md](./docs/release-smoke-checklist.md)),
and the size gate below.

## Hard Size Budget (the binding constraint)

The `scripts/release-build.sh` binary must be ≤ 1,048,576 bytes on macOS
x86_64 AND Debian x86_64. Baseline 2026-07-10 (`b2510a3`, build-std
contract):

- macOS: 575,460 bytes (`target/x86_64-apple-darwin/release/dun`)
- Debian: 620,928 bytes — **binding platform**, margin 427,648 bytes

Reserve plan on the margin: plugin client ~76 KiB + future-feature reserve
120 KiB still leaves ~230 KiB. Decisive measurements happen on the Debian
VM; macOS deltas are proxies (~1.25x rule of thumb). With `opt-level = "z"`
+ fat LTO, size deltas are non-additive — measure per batch.

## Codex Delegation (grunt-work packages)

Method imported from the rum project (2026-07-10). Role split: I own
direction, diagnosis, brief writing, the authoritative gate, and the commit;
Codex executes exactly one brief, self-verifies to green, and reports with
verbatim evidence — it never commits, branches, or pushes.

- Briefs live in `docs/dev/codex/`: `TEMPLATE.md` is the format,
  `brief-NNN-<slug>.md` are the packages. Commit the brief FIRST, then
  dispatch: `scripts/dispatch-brief.sh NNN` (resolves the brief, runs
  `cdx exec -s danger-full-access`, logs to `/tmp/dun_cdx_brief_NNN.log`;
  `DRY_RUN=1` previews). Launch via Bash `run_in_background: true` — never a
  hand-typed `&`-detach.
- Do not mutate the repo while a brief is in flight.
- The gate before crediting any brief: (1) scope check — the diff touches
  only the MAY-modify list; (2) `cargo fmt --all -- --check`, clippy
  `-D warnings`, `cargo test --workspace --no-fail-fast` reproduced by me;
  (3) for runtime-code briefs, the dual-platform size gate and release smoke;
  (4) I reproduce claimed evidence myself before committing; (5) for any test
  guarding a correctness or safety invariant, I mutate the implementation and
  confirm the test fails (see AGENTS.md "Prove a test load-bearing"). Two
  Codex-authored A-level tests this session passed against a broken
  implementation and only mutation caught them — a brief saying "verified" is
  not the same as a test that can fail.
- Watch Codex's recurring failure modes in review: fixing at the wrong
  layer, masking symptoms instead of causes, drive-by edits outside scope,
  and vacuous tests (an oracle that reuses the implementation's own predicate;
  asserting an escape sequence is present when something other than the code
  under test emitted it).
- Known dispatch failures around a new codex release (tree stays clean in
  both; re-dispatch after the user updates codex interactively): (a) log
  stalls forever at "Reading additional input from stdin..." — pending
  self-update prompt with no TTY; safe to kill; the `< /dev/null` in the
  dispatcher usually prevents this. (b) fast fail with API 400 "model
  requires a newer version of Codex" — the dispatcher prints the remedy.
  First seen in rum 2026-07-01 (hang) and dun 2026-07-10 (fast fail).

## Debian Measurement VM

VirtualBox VM `debvbox`; connection details and working conventions are in
[docs/debian-vm.md](./docs/debian-vm.md). Use the tracked wrapper scripts —
`vm-test/vm-run [command]` to execute on the VM and `vm-test/vm-sync [ref]`
to sync a clean commit archive (`--worktree` rsyncs the dirty tree for
iteration only) — instead of raw ssh. Passwordless sudo is available on the
VM. The VM is started manually by the user — ask when a binding measurement
is needed. Measurement procedure: repeat checklist in
[docs/release-size-audit.md](./docs/release-size-audit.md) (clean git archive
to the VM, locked release build, `stat`/`file`/`ldd`, `--version` and
`--dump-config` smoke). Use dated scratch dirs on the VM and delete them
after recording results.

## Active Plan (decided 2026-07-10)

Positioning: `dun` is a lightweight TUI editor in its own right. Embedding
`rum` in the runtime is dead; plugin support is protocol-first with external
hosts ([docs/plugin-protocol.md](./docs/plugin-protocol.md)). The plugin
protocol client is required core; `rum-host` is a future separate artifact.

Sequencing (stages 1–2 completed 2026-07-10):

1. ~~Plugin-client size spike~~ — done: 76 KiB Debian floor, hand-rolled
   JSON; implementation reference on branch `spike/plugin-client-size`.
2. ~~Feature triage + trim~~ — done: C/D batches 1-3 (-48 KiB) plus the
   decisive build-std contract (spike A; all remaining B features KEPT).
   Outcome in [docs/feature-triage.md](./docs/feature-triage.md).
3. **Land the real plugin protocol client** (ACTIVE — TODO.md "Plugin
   Protocol Client" stage). Margin is ample; keep the client hand-rolled
   JSON, no serde.
4. **Re-audit** both platforms after the client lands.

Parallel line: renderer replacement (drop ratatui for the in-house Surface
backend) as dependency hygiene, sliced into small Codex briefs (brief-002
landed the Surface grid). No longer size-critical; correctness gates and
the tmux/PTY suites are the fence.

Triage decision rules (apply in order, first hit wins):

1. Safety/correctness invariant (sanitized rendering, atomic save, dirty
   confirm, terminal restore)? → A, non-negotiable.
2. Required for the seven-step remote editing loop (README "Product Goal")
   with no A-level workaround? → A.
3. Could a plugin role provide it once the protocol exists? → D; record the
   role need in docs/plugin-protocol.md when removing.
4. Serves neither the editing loop nor SSH constraints (including
   showcase-era leftovers)? → C, remove now.
5. Otherwise → B, ranked by measured bytes per value; B has a total byte cap.
