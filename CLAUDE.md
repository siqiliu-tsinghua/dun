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
x86_64 AND Debian x86_64.

- macOS: **706,748 bytes** (2026-07-27, `9200e5e`).
- Debian: **764,208 bytes** at `4f91b01` — **binding platform**, margin
  284,368 bytes (2026-07-27, binding measurement from a clean git archive).
  The last two commits are refactor/doc-only and byte-neutral on macOS, so
  Debian is measured through them by construction; re-measure on the next
  runtime change. The 2026-07-27 stage added ~4 KiB total: the diagnostic-
  window i18n keys (+4,096 Debian) and the plugin menu/leader work (+8 macOS,
  0 Debian).
- Earlier baseline for context: macOS 677,940 / Debian 739,632 at `877b7ad`
  (2026-07-23, crossterm replacement complete). The crossterm-replacement track
  (five steps, `cf1a5b6`..`877b7ad`) made dun's terminal I/O fully in-house
  and shrank the binding binary 24,656 bytes net from the 764,288 stage-B
  baseline: step 1 own output escapes (−4,096), step 2 rustix sys shim
  (+4,096 transitional), steps 3–4 owned event types + in-house VT parser /
  direct-poll(2) event reader (−24,656 as LTO shed crossterm+mio), step 5
  manifest removal byte-identical. Lockfile 42 → 26 packages; external
  direct deps = rustix + signal-hook + unicode-width. The historic Solaris
  input defect (second `tmux send-keys` batch lost via mio's poll(2)-fallback
  interest clearing + crossterm's never-reregistered `SourceFd`) is fixed by
  construction — first fully green four-platform matrix (780/0 on macOS/
  Debian/FreeBSD/Solaris). **No Debian measurement debt:** measured through
  HEAD. (See docs/release-size-audit.md 2026-07-23 entries.)

**Debian measurement debt: settled 2026-07-15.** The 19-commit debt span
(`89cd9e4..1d03433`) is paid off: HEAD (`744c843`, byte-identical binary to
`1d03433`) measures 715,136 bytes on the VM — +28,672 over `89cd9e4`, the
i18n slice-4 mechanism tail; the ten translations stayed free. The ~700 KiB
projection held. Smoke passed (ELF PIE stripped, `ldd` = libgcc/libm/libc/
ld-linux unchanged, `--version`, `--dump-config`). The plugin stage now
starts on a measured baseline (docs/release-size-audit.md 2026-07-15).

Note the ten shipped translations cost the binary **nothing**: they are
external resource files, not code. Decisive measurements happen on the
Debian VM; macOS deltas are proxies (~1.1-1.25x rule of thumb). With
`opt-level = "z"` + fat LTO, size deltas are non-additive — measure per
batch.

## Codex Delegation (grunt-work packages)

Method imported from the rum project (2026-07-10). Role split: I own
direction, plan review, the authoritative gate, and the commit; Codex executes
exactly one brief, self-verifies to green, and reports with verbatim evidence —
it never commits, branches, or pushes.

**Plan-first is the default (decided 2026-07-22).** For any non-trivial or
cross-cutting task, do NOT hand-write a from-scratch implementation brief.
Instead: (1) dispatch a **design-only brief** (`Scope: NONE — no source
change`) that states the problem + current state and asks Codex to produce a
concrete step-by-step *plan* — files/functions per step, how invariants stay
intact, the gate tests, a call-site inventory, and risks/open questions, all
with `path:line` evidence; Codex plans and writes no code. (2) I review and
adapt the plan (decide the open questions, correct anything). (3) I dispatch
each step as its own implementation brief, in order, and run the gate on each.
This moves the codebase spelunking + architecture inventory onto Codex, cutting
my session-quota burn, and Codex-authored plans enumerate the reach up front so
a cross-cutting change isn't silently under-scoped (two clean Codex stop-losses
on the wide-geometry work proved the hand-written brief hazard). Template
sequence: brief 033 (design-only plan) → briefs 034/035/… (per-step
implementation, each gated). Trivial mechanical jobs may still go straight to an
implementation brief.

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
- **Restore a mutation by reversing the edit, never with `git checkout`.**
  The tree is dirty during a gate: checkout restores from HEAD/index and
  wipes the brief's uncommitted work with it. This bit twice on 2026-07-13;
  recovery came from the final `git diff` dump in
  `/tmp/dun_cdx_brief_NNN.log`, plus a `cargo fmt` pass.
- **Aim the mutation at a path the test actually covers.** A mutation that
  survives is either a vacuous test or a misaimed mutation — check which
  before accusing the test. On 2026-07-13 a survivor turned out to be my own
  bad aim (I broke a `WorkspaceError` variant the test never exercised).
- **A refactor can silently remove coverage.** Twice this session, a
  correct-looking refactor left a live code path with no test on it (the
  `PathIoError::Display` used only by the CLI's startup `eprintln!`; the 47
  menu keys the generic validator could not see). Codex's own mutation runs
  could not detect either. After a refactor, ask which paths *used* to be
  covered and mutate those.
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

## Test / Measurement VMs

Three local VirtualBox VMs, all started manually by the user and reachable
through the tracked `vm-test/` wrappers with a target selector (`-t NAME` /
`DUN_VM_TARGET`; default `debian`):

- **`debian` (port 2222, `debvbox`)** — the **binding** size/measurement
  platform ([docs/debian-vm.md](./docs/debian-vm.md)); with macOS it is the
  1 MiB size budget.
- **`freebsd` (port 2233, FreeBSD 15.1)** — a **portability / functional** test
  env ([docs/freebsd-vm.md](./docs/freebsd-vm.md)), NOT a size-budget platform
  (LLVM/lld + `pkg` rust ≠ the 1.85 budget baseline; size is a reference only).
- **`solaris` (port 2244, Oracle Solaris 11.4)** — a **portability / functional**
  env ([docs/solaris-vm.md](./docs/solaris-vm.md)), NOT a size-budget platform
  (native Solaris `ld`, multilib, `pkg` rust 1.87 with `rust-src` linked from
  `/opt`). Known quirk (root-caused, not a `dun` defect): Solaris tmux renders
  Unicode ambiguous-width glyphs (box-drawing, `◆`) double-width, so `tmux_grid`
  fails there; workaround is `terminal.encoding = ascii`. More targets may be
  added.

Use `vm-test/vm-run [-t NAME] [command]` and `vm-test/vm-sync [-t NAME] [ref]`
(`--worktree` rsyncs the dirty tree for iteration only) instead of raw ssh;
passwordless sudo is available on all three. The wrappers keep a repo-local,
gitignored `vm-test/known_hosts`. **When a change needs VM testing, ask the
user to start every relevant VM (Debian, FreeBSD, Solaris today), then run
against each with its target** — do not assume any VM is up. Binding size
measurement (Debian) uses the repeat checklist in
[docs/release-size-audit.md](./docs/release-size-audit.md) (clean git archive,
locked release build, `stat`/`file`/`ldd`, `--version`/`--dump-config` smoke).
Use dated scratch dirs and delete them after recording results.

## Active Plan (decided 2026-07-10; resequenced 2026-07-13)

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
3. ~~Land the real plugin protocol client~~ — DONE, stage closed
   2026-07-13: all release gates passed at `fd31719`; Debian re-audit
   recorded (670,080 bytes, margin 378,496).
4. ~~UI text i18n~~ — DONE, stage closed 2026-07-13 (design in
   docs/i18n.md). The whole UI translates (menus, help, dialogs, every
   status message); English is compiled in as the `&'static` fallback and
   ten languages ship as external `i18n/<tag>.conf` files, which cost the
   binary nothing. Nine of the ten are machine-translated and unreviewed —
   there is no native-speaker reviewer, so that is stated as provenance in
   docs/i18n.md rather than tracked as outstanding work.
5. ~~Distinctive plugins~~ — DONE, stage closed 2026-07-23
   (capability-model-first, reframed 2026-07-16): `role` is a **named bundle
   of inward capabilities** — typed, validated channels into `dun`-owned
   objects — with trust class as the grant gate. Slices A–D plus the three v0
   data channels (surface-write, stream-read, scratch-input + execute) are
   built, fixture-driven, and Debian-measured; the first real consumers
   (Python + Lua log-filter hosts under `hosts/`) passed live tmux acceptance
   on all four platforms and their three findings are fixed. The v0
   capability surface is frozen; the sum-typed validator dispatch is retired
   as superseded by construction and per-role policy overrides stay unadopted
   (closure notes in TODO.md and docs/plugin-protocol.md).

6. ~~Restoration review — F12/F13~~ — DONE, stage closed 2026-07-26
   (`b9ac165`): bookmarks + visible whitespace restored full-trail over
   three gated steps (seam `3b69844`, F13 `5914467`, F12 `b9ac165`) from
   the `53fe7f8^` spec (a plain revert was dead — 10/26 files conflicted),
   re-landed on today's architecture (i18n ten catalogs + shared
   `EditorTextDisplay` seam + `Ctrl+X` keymap family). Binding Debian
   756,016 (+16,384 over `877b7ad`, margin 292,560); four-platform matrix
   817/0. One clean Codex stop-loss (stale gutter-render assumption →
   Option C). F46 stays removed; F20 still returns as a plugin role.

7. ~~OSC 52 clipboard read~~ — DONE, stage closed 2026-07-27 (`42774ec`):
   paste the host clipboard over SSH via the terminal (the read counterpart
   to the OSC 52 write). Two gated steps — the armed parser seam + strict
   base64 decoder (`110aa08`, byte-neutral), then the wiring (`42774ec`:
   `clipboard.osc52.allow_read` opt-in, `edit.paste_external` /
   `Ctrl+X,Ctrl+V`, 500 ms synchronous-feel wait, internal-clipboard
   fallback). Plan brief 051 → briefs 052/053; one clean stop-loss (052/053
   scope). No new input security layer (decoded bytes ride the display
   sanitizer; terminal owns the read-gate); real-terminal read is
   best-effort, PTY-verified, not a measured matrix. Binding Debian 760,112
   (+4,096 over `b9ac165`, margin 288,464); four-platform 845/0.

8. ~~Real-terminal acceptance + the plugin-UI defects it found~~ — DONE,
   stage closed 2026-07-27 (`d3eacda`..`4149566`, 17 commits). The pass ran
   for real: 51 screenshots (3 emulators x 14 local scenarios, plus 3
   emulators x 3 VMs over live SSH) and 232 headless 80x24 text grids
   (11 languages x 20 dialog/editor states, plus the log-filter surface).
   Tooling is in `acceptance/` and documented in
   docs/real-terminal-acceptance.md.

   **The screenshots were not the product — the defects they exposed were.**
   Five were found by looking at output, none by reading code: the two
   diagnostic windows were half-translated in all eleven languages; a
   plugin's top-level menu was keyboard-unreachable in every translated
   language (English worked only because `Log Filter` starts with `L`);
   plugin dropdown entries had no mnemonic even in English; a plugin's second
   window split the layout into three unreadable columns; and plugin leaders
   used a runtime check where a structural guarantee belonged. Plus two gaps
   of my own: the reserved `Ctrl+T` was never disclosed to users, and the
   gallery's locale dimension had never actually taken effect.

   Protocol changes: menu items take an optional `mnemonic` and menus a
   `top_mnemonic` (authors choose; no derivation rule survives a real
   plugin), and `Ctrl+T` is reserved for every plugin instead of hosts
   picking their own leader. A plugin's own settings live with the plugin,
   not in dun's config — install is unpacking a folder, uninstall is
   deleting one.

Next stage: none active — pick from the queue with the user. Remaining:
F20 Outline as a plugin `DocumentStructure` role; rum evaluation stays
externally blocked on rum-ext's resource/type base; plugin dropdown entries
still have no mnemonic when a host declares none (deliberate — see
docs/plugin-protocol.md "Menu mnemonics"). Optional follow-up: live
real-terminal OSC 52 read acceptance (user-driven).

Inserted track — **crossterm replacement: COMPLETE 2026-07-23**
(`cf1a5b6`..`877b7ad`, plan brief 041, implementation briefs 042–046,
plan-first Codex with every step Claude-gated). dun's terminal I/O is fully
in-house: `terminal/vt/{output,event,parser}` platform-neutral core +
`terminal/sys/unix.rs` rustix shim + `terminal/event_reader.rs` on direct
level-triggered `poll(2)`, safe Rust throughout, msedit-style layering keeps
the Windows door open. The acceptance was met: the Solaris second-batch input
defect is gone by construction and all four platforms run green (780/0 —
first time in the project's history). Lockfile 42 → 26; binding binary
−24,656 net. The bounded input surface (xterm-family keys, SGR mouse,
bracketed paste, CPR/DA1, SIGWINCH; kitty/modifyOtherKeys/X10/rxvt excluded)
is documented in docs/terminal-compatibility-checks.md.

Renderer replacement is DONE: ratatui was fully retired at `858e876`
(2026-07-11) — dropped from every crate, the workspace table, and the
lockfile (76 → 42 packages); `dun` renders through the in-house Surface
backend, with only a doc-comment mention of the old snapshot helper left in
test support. No longer a parallel line.

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
