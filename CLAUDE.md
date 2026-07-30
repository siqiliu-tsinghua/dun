# CLAUDE.md

Session guidance for Claude Code in this repository. The authoritative
contribution rules and invariants live in [AGENTS.md](./AGENTS.md); read it
before changing behavior. This file adds orientation, the binding constraint,
and the active working plan. Keep all three in sync.

## Quick Orientation

Rust 1.85 workspace, six crates under `crates/`:

- `dun-core`: buffers, undo/redo, search, tiled workspace state, and the
  typed `EditorCommand` enum (`src/command.rs` — 93 command variants across
  `App`/`Edit`/`File`/`Window`, every one with a command id, held equal by a
  test; the mechanical enumeration of user-visible features).
- `dun-term`: terminal capability profiles, color/glyph fallback, themes.
- `dun-config`: typed config, keymap, command-id parsing, validation.
- `dun-ui`: backend-neutral frame model rendered onto the in-house `Surface`
  grid (ratatui retired at `858e876`).
- `dun-plugin`: the protocol client, its hand-rolled JSON, and the per-role
  output validators.
- `dun-cli`: terminal lifecycle (in-house VT since `877b7ad`), event loop,
  command application (the largest crate, ~36k lines including its tests —
  most UX weight lives here).

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
checklist ([docs/dev/release-smoke-checklist.md](./docs/dev/release-smoke-checklist.md)),
and the size gate below.

## Hard Size Budget (the binding constraint)

The `scripts/release-build.sh` binary must be ≤ 1,048,576 bytes on macOS
x86_64 AND Debian x86_64.

- **Current: macOS 727,380 / Debian 788,784 at `4b362e7`** (2026-07-30, command
  capture deadline, G1 of the process-cleanup track) — margin **259,792**,
  four-platform matrix **935/0**. **No measurement debt.** +4,096 on macOS, **0
  on the binding platform**: `rustix::event::poll` was already linked for the
  event reader, so the deadline-aware read only added branches. G2 (kill the
  command's process group) needs rustix's `process` feature and is the first
  step of this track likely to cost pages.
- macOS 723,284 / Debian 788,784 at `8a3dddf` (2026-07-30,
  external-review parser response) — four-platform matrix **933/0**. Both parser fixes together are
  **byte-identical to `11f56a6`** on the binding platform: the config change
  replaced two passes (`strip_comment` then `unquote_value`) with one
  allocation-free `scan_config_value` returning a borrowed slice, and the JSON
  change added only an explicit grammar walk, a comparison over a `Vec` that
  already existed, and a `const`. The `HashSet` alternative for duplicate keys
  was rejected precisely because it was the one that would have cost pages.
  Gate platforms only; FreeBSD/Solaris not re-measured (parser-internal changes,
  no platform-specific code), their `11f56a6` figures stand — but all four ran
  the functional matrix.
- macOS: **719,100 bytes** / Debian: **784,688 bytes** at `260d850`
  (2026-07-29, folding complete) — margin 263,888. **No measurement debt.**
  Folding cost 16,384 total on the binding platform, attributed per step:
  seam +8,192 (`058447f`), fold state and edit remap +4,096 (`4efc4c5`),
  **placeholder rendering +0** (`c96fab0`), commands and ten catalogs +4,096
  (`bb62b20`). Brief 058 estimated 32–64 KiB for the feature; it came in at
  half the low end, and the render step was free because it added branches to
  paths that already existed. Reference platforms, same build-std
  contract but outside the budget: FreeBSD **698,344**, Solaris
  **744,880** — all four platforms are now under the line, Debian binding.
  Solaris needs one platform-specific link flag to get there:
  `scripts/release-build.sh` adds `-C link-arg=-znoldynsym` on SunOS (adopted
  2026-07-29), dropping ~343 KB of `.SUNW_ldynsym` symbol-table metadata the
  native linker keeps for `pstack`. Without it Solaris measures 1,087,760.
  **The gate stays macOS + Debian** — two platforms is what keeps per-step
  measurement affordable, and Solaris tracks Debian within 5% of `.text`, so it
  would never bind first. `strip` recovers nothing there and
  `-z strip-class=nonalloc` makes it 377 KB *larger*; mechanism in
  docs/dev/release-size-audit.md.
- Earlier: macOS 710,860 / Debian 776,496 at `058447f`. The +8,192 over
  v0.1.0 is attributed: `1d078cb` (bookmarks into `TextBuffer`, with the
  per-buffer `Vec<usize>` and the remap) measured **768,304, byte-identical**
  to the tag, so the whole page pair belongs to `058447f`, the line-level
  seam. Codex's plan estimated 32–64 KiB for all of folding; step 1 spent 8.
- Release baseline: macOS **706,748** / Debian **768,304** (tag `v0.1.0`).
- Debian: **768,304 bytes** at tag `v0.1.0` — **binding platform**, margin
  280,272 bytes (2026-07-28, v0.1.0 sign-off, clean git archive).
  **No measurement debt.** The +4,096 over the previously recorded 764,208
  is not from the v0.1 wrap-up stage: `63088ff`, the tip before it began,
  measures 768,304 too, and rebuilding `4f91b01` reproduces 764,208 exactly,
  so the environment is stable and the page belongs to `9200e5e` (the plugin
  leader disclosure), which the earlier entry assumed byte-neutral on Debian
  without measuring it. The wrap-up stage itself is byte-identical on both
  platforms — measured, not inferred from "docs only". Correcting the earlier
  entry: the 2026-07-27 stage cost ~8 KiB on Debian, not ~4 KiB — the
  diagnostic-window i18n keys (+4,096) *and* the plugin menu/leader work
  (+4,096, recorded then as "0 Debian" from a macOS delta of +8 rather than
  from a measurement).
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
  HEAD. (See docs/dev/release-size-audit.md 2026-07-23 entries.)

**Debian measurement debt: settled 2026-07-15.** The 19-commit debt span
(`89cd9e4..1d03433`) is paid off: HEAD (`744c843`, byte-identical binary to
`1d03433`) measures 715,136 bytes on the VM — +28,672 over `89cd9e4`, the
i18n slice-4 mechanism tail; the ten translations stayed free. The ~700 KiB
projection held. Smoke passed (ELF PIE stripped, `ldd` = libgcc/libm/libc/
ld-linux unchanged, `--version`, `--dump-config`). The plugin stage now
starts on a measured baseline (docs/dev/release-size-audit.md 2026-07-15).

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
  platform ([docs/dev/debian-vm.md](./docs/dev/debian-vm.md)); with macOS it is the
  1 MiB size budget.
- **`freebsd` (port 2233, FreeBSD 15.1)** — a **portability / functional** test
  env ([docs/dev/freebsd-vm.md](./docs/dev/freebsd-vm.md)), NOT a size-budget platform
  (LLVM/lld + `pkg` rust ≠ the 1.85 budget baseline; size is a reference only).
- **`solaris` (port 2244, Oracle Solaris 11.4)** — a **portability / functional**
  env ([docs/dev/solaris-vm.md](./docs/dev/solaris-vm.md)), NOT a size-budget platform
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
[docs/dev/release-size-audit.md](./docs/dev/release-size-audit.md) (clean git archive,
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
   Outcome in [docs/dev/feature-triage.md](./docs/dev/feature-triage.md).
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
   Option C). F46 stays removed; F20 was cancelled outright on 2026-07-28.

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
   docs/dev/real-terminal-acceptance.md.

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

9. ~~v0.1 wrap-up and public release~~ — **DONE, tagged `v0.1.0`
   2026-07-28** (`1ad72d4`..`029ea08`). One step remains and it is not
   engineering: the user creates the public GitHub repository, then
   `Cargo.toml`'s commented `repository`, README's `<repository-url>`, the
   remote, and the push of `main` + the tag all follow. The
   runtime is done: no active code stage, four-platform 845/0, binding Debian
   760,112 with 288,464 to spare. What is missing is everything around the
   code. **No runtime code changes in this stage** — therefore no size
   measurement and no VM work until the final sign-off.

   User decisions frozen 2026-07-27: publish as a **public GitHub
   repository** (no crates.io packaging work); process docs are **kept** and
   move under `docs/dev/`; 3–4 curated gallery screenshots enter the repo;
   LICENSE is MIT, `Copyright (c) 2026 Si-Qi Liu`. Deferred to stage C:
   whether the GitHub release attaches macOS/Debian binaries (lean: no — the
   build-std contract is not byte-reproducible for an outsider, and shipping
   binaries without a reproducible build recipe is a liability).

   Three serial stages:

   - **A — cleanup and relocation.** Today `docs/` mixes 4 user-facing pages
     with 18 process/measurement ones. Split `docs/` (user) from `docs/dev/`
     (process, measurement, method, VM manuals, the 58 codex briefs); move
     PLAN/PROGRESS/AUDIT there too so the root keeps only README, AGENTS,
     CLAUDE, TODO. Fix every cross-link behind a link checker — the briefs
     reference paths too. Then the **pre-publication scan** that a public repo
     requires: real IPs, hostnames, absolute local paths, emails, credentials
     (`hosts/sample-logs/` — `ssh-bruteforce.log`, `access.log`,
     `mcp-probes.log` — are the first suspects). Curate 3–4 gallery
     screenshots into `docs/images/`.
   - **B — documentation.** The corpus is written for dun's *builders*, not
     its users or its outside contributors. README still calls dun "planned"
     and "ratatui-based" (retired at `858e876`, README:5,7,14,294); there is
     no user guide at all; no plugin-**author** guide
     (docs/plugin-protocol.md is the spec, hosts/README.md only configures the
     shipped hosts); and no onboarding for the test tooling — the VM manuals
     document connection and conventions but assume the VM already exists, so
     nobody outside this machine can reproduce the harness. Write those four,
     fix README to describe what dun is today, demote docs/dev/PLAN.md to a historical
     plan (CLAUDE.md is the live one), close the stale TODO/AGENTS items.
   - **C — release artifacts.** LICENSE, CHANGELOG (user-visible changes
     distilled from PROGRESS's 1,360 lines), Cargo metadata, a decision on
     docs/dev/PLAN.md's still-unchecked plugin-policy security-audit item (do not ship
     v0.1 with an open security box), then the sign-off: release smoke +
     dual-platform size gate + four-platform matrix + tag `v0.1.0`. **The VMs
     are needed only here.**

10. ~~Folding~~ — **DONE, stage closed 2026-07-29** (`1d078cb`..`7c97fd9`).
    Manual folds that need no type knowledge and no plugin, so they work on
    the cold open that motivates the feature. Plan brief 058 → implementation
    briefs 059–063, every step Claude-gated. Four steps plus a prerequisite
    fix: bookmark remap repaired first (`1d078cb`, byte-identical) because
    folding needed the same remapping; then the line-level display seam
    (`058447f`), fold state and edit remap on `TextBuffer` (`4efc4c5`), the
    placeholder row (`c96fab0`, **+0 bytes**), and the commands with their
    full trail — keys, menu, help, status messages, ten catalogs (`bb62b20`).
    `Ctrl+X,F` toggles, `Ctrl+X,A` unfolds all.

    Cost 16,384 bytes on the binding platform against brief 058's 32–64 KiB
    estimate; four-platform matrix **906/0**. One recurring failure mode,
    named three times by Codex before it stopped recurring: **tests covered
    the path that does not execute** (step 1's identity mapping untested while
    its fold path was; step 2's degenerate-range guard asserted but never
    reached; step 3's maintained fold set was not the rendered one — a
    duplicate created by my own step split). Two briefs (059, 062) stopped on
    under-scoped MAY-modify lists, both times because I wrote scope from
    recollection instead of `grep`.

    Tail work, same stage: the Solaris binary's 1,087,760 was root-caused to
    linker metadata rather than code, `-znoldynsym` adopted as the platform's
    default link flag (`107557f`), and `release-build.sh` converted to POSIX
    `sh` so it runs on all four platforms (`7c97fd9`).

Queue: **F20 Outline is cancelled** (user decision 2026-07-28) — the need is
too small to carry, and a plugin-delivered navigation aid is absent exactly
when it would matter, on a cold open of an unfamiliar file on a strange host.
The `DocumentStructure` role need is withdrawn with it. rum evaluation stays
externally blocked on rum-ext's resource/type base; plugin dropdown entries
still have no mnemonic when a host declares none (deliberate — see
docs/plugin-protocol.md "Menu mnemonics"). Optional follow-up: live
real-terminal OSC 52 read acceptance (user-driven).

11. **Install experience — ACTIVE, opened 2026-07-29** by a user walking the
    published install flow. `main` carries v0.1.0 plus folding plus this,
    all unreleased; CHANGELOG's `Unreleased` is the list.

    The finding: building `dun` gives you an executable and nothing else, so a
    first run has no config file and — because catalogs loaded from `i18n/`
    *next to the active config file* — an English UI whatever `LANG` says.
    Round one answered it with `scripts/install.sh` + `scripts/uninstall.sh`
    and no runtime change (the catalogs are deliberately external, so a
    `--init` flag inside the binary could not have installed them). Round two
    added `scripts/build.sh`, interactive prompts on a TTY (`--yes` is the CI
    path), `--prefix` for `/usr/local` and `/opt/dun`, the syntect host
    installed *and enabled* by default, a `PATH` rc-file offer, and
    `--package` (a tarball whose layout is an install tree, so the same
    `install.sh` runs on both sides of an `scp`).

    Round three restructured all three scripts to **decide → show the plan →
    confirm once → act**, so an interrupted interview leaves nothing behind
    (`--dry-run` is now the same plan printer minus the confirmation), and
    moved the default prefix to `$HOME/.local` so the per-user and system
    layouts are one shape: `<prefix>/bin`, `<prefix>/share/dun/{config,i18n}`,
    and `~/.config/dun/config` for the user's own overrides.

    **Two runtime changes**, both load-bearing for that layout:

    - catalogs also load from `<bin>/../share/dun/i18n`
      (`current_exe`-relative). Without it `--prefix` is theatre — a system
      install cannot translate for a user with no config file. Ordering is by
      directory, not by candidate; a broken file in your directory is still
      reported rather than masked; `--no-config` disables both.
    - **configuration is two layers**: `<bin>/../share/dun/config` then the
      user's file overlaid key by key (`parse_config_overlay`, which already
      existed). Without it the same defect applies to settings: one personal
      line would discard every machine-wide one. An invalid *installed* file
      reports and steps aside; an invalid *user* file is still fatal.
      `ConfigSource` now means the user layer, and `LoadedConfig::base`
      carries the installed one; `F6` prints both.

    13 tests, 7 mutations, all caught. The end-to-end pair is the one to keep:
    installed config binds `F9` + `theme = turbo`, user file sets only
    `theme = dark`, and one tmux session asserts `F9` opens Help *and* `F6`
    says `theme: dark` — replace-instead-of-overlay cannot pass both.

    **Stage closed 2026-07-29 at `11f56a6`.** Gates on all four platforms from
    a clean `git archive`: Debian **788,784** binding, margin 259,792,
    **+4,096** — one page for both runtime changes together, because the
    catalogs and the configuration share the one path derivation. macOS
    723,276, FreeBSD 702,896, Solaris 750,064. Matrix **919/0** at the commit
    on all four. Release smoke on macOS + Debian. No measurement debt.

    Deployment acceptance ran on all three VMs in the three steps the owner
    specified: `$HOME` install/uninstall, `$PREFIX` install/uninstall (plus
    `sudo --prefix /opt/dun`), and the tarball carried to a fresh `duntest`
    user via `cp`/`chown`/`su -`. Every install was verified by *running* the
    editor and reading the menu bar, not by listing files. Two defects found
    that way, both fixed: the uninstall plan announced removing a catalog
    directory that did not exist, and Solaris packages were named `i86pc`
    (`uname -m`'s platform name) instead of `amd64` (`isainfo -k`).

Previously (still true): the last *folding-era* runtime-code commit is
`bb62b20`. Figures at `7c97fd9`, all four verified through
`scripts/release-build.sh`: macOS **719,100**, Debian **784,688** (binding,
margin 263,888), FreeBSD **698,344**, Solaris **744,880**.

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
is documented in docs/dev/terminal-compatibility-checks.md.

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
