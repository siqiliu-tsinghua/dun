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
cargo build --release --locked -p dun-cli    # budget measurement build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Runtime-code commits must pass fmt, clippy, tests, the release smoke
checklist ([docs/release-smoke-checklist.md](./docs/release-smoke-checklist.md)),
and the size gate below.

## Hard Size Budget (the binding constraint)

`target/release/dun` must be ≤ 1,048,576 bytes on macOS x86_64 AND Debian
x86_64 using the checked-in `[profile.release]`. Baseline 2026-07-09
(`60d45a2`):

- macOS: 863,664 bytes (184,912 under budget)
- Debian: 1,038,936 bytes (9,640 under budget — **Debian is the binding
  platform**; it runs ~175 KiB fatter than macOS)

Treat every runtime code or dependency addition as budget-sensitive. Decisive
size measurements happen on the Debian VM; macOS deltas are only a proxy.
With `opt-level = "z"` + fat LTO, size deltas are non-additive — measure
removals per batch, not per item.

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
  (4) I reproduce claimed evidence myself before committing.
- Watch Codex's recurring failure modes in review: fixing at the wrong
  layer, masking symptoms instead of causes, and drive-by edits outside
  scope.
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

Sequencing:

1. **Plugin-client size spike** — a minimal framed-stdio + JSON + one-role
   prototype, measured on Debian with the locked release profile. Its byte
   delta anchors how much the trim must free. No JSON/serde dependency exists
   in-tree today; adding one is a budget decision, not a convenience decision.
2. **Feature triage + proactive trim** — reclassify all features into
   A core / B optional (measured byte cost + trim order) / C remove now /
   D delegate-to-plugin. The paper inventory can proceed in parallel with the
   spike. Execute C/D removals in gated batches. Working document:
   [docs/feature-triage.md](./docs/feature-triage.md). This supersedes the
   lazy trim-on-failure order in
   [docs/feature-budget.md](./docs/feature-budget.md); rewrite that document
   as part of this stage.
3. **Land the real plugin protocol client** on the freed budget
   (TODO.md "Plugin Protocol Client" stage).
4. **Re-audit** both platforms and refresh
   [docs/release-size-audit.md](./docs/release-size-audit.md).

Budget target for stages 2–3: after the plugin client lands, Debian must
retain a reserve for future features (target figure to be set from the spike
measurement, ~80–120 KiB).

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
