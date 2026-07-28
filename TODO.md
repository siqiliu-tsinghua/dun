# TODO

This file tracks active and near-term work. Completed decisions and finished
items belong in [PROGRESS.md](./docs/dev/PROGRESS.md).

## Current Stage: v0.1 Wrap-Up and Public Release

Roadmap agreed 2026-07-27; execution starts 2026-07-28. The runtime is done
for v0.1 — no active code stage, four-platform matrix 845/0, binding Debian
760,112 bytes with 288,464 to spare. What is missing is everything *around*
the code: docs that describe a project which no longer exists in places, no
user-facing documentation of any kind, and none of the release artifacts.

**No runtime code changes in this stage.** That means no size measurement and
no VM work until C6. Stages run serially: B depends on A's paths, C's README
links depend on B.

User decisions frozen 2026-07-27: publish as a **public GitHub repository**
(no crates.io packaging work here); process docs are **kept** and move under
`docs/dev/`; 3–4 curated gallery screenshots enter the repo; LICENSE is MIT,
`Copyright (c) 2026 Si-Qi Liu`. One sub-decision is deferred to C: whether the
GitHub release attaches macOS/Debian binaries (lean: no, see C5).

### A. Cleanup and relocation

- [x] Split the doc tree: `docs/` becomes user-facing, `docs/dev/` holds
  process, measurement, and method. Moved the fourteen planned documents plus
  five the first pass had misfiled as user-facing — `crate-map`,
  `editor-baseline`, `release-smoke-checklist`,
  `terminal-compatibility-checks`, and `window-management` are all read only
  by someone working *on* `dun`. Nineteen documents in total; `docs/` now
  holds `configuration.md`, `i18n.md`, and `plugin-protocol.md`, the three a
  user or plugin author needs.
- [x] Move `PLAN.md`, `PROGRESS.md`, and `AUDIT.md` into `docs/dev/`. The
  repository root keeps `README.md`, `AGENTS.md`, `CLAUDE.md`, and `TODO.md`.
- [x] Fix every cross-link the move breaks and add `scripts/check-links.py` so
  the check is repeatable. The checker walks tracked Markdown, resolves inline
  links, reference definitions, and bare repository-path mentions, and exits
  non-zero on a broken local target. Source-tree mentions (`crates/`, `i18n/`)
  are checked only under `--strict`: the append-only log and the codex briefs
  name the tree as it stood when they were written, so a mention of a
  since-split module is a historical fact rather than a broken reference.
  Twenty-four files were rewritten; the target count held at 362 across the
  move, which is what proves no link was dropped rather than silently
  repointed.
- [x] Review what `scripts/check-links.py --strict` caught in live documents.
  Only one was a defect: `docs/i18n.md` sent translators to
  `crates/dun-cli/src/ui_text.rs`, which is a directory now — fixed. The
  matches in `docs/dev/code-organization-guidelines.md` and
  `docs/dev/file-splitting-plan.md` are *not* defects: they sit in
  completed-stage tables where the pre-split filename is the subject of the
  record ("`buffer.rs`, 74k chars, done, split into …"). That is why source
  mentions stay behind `--strict`.
- [x] Run the pre-publication scan a public repository requires. Clean on the
  categories that mattered: no absolute local paths, no email addresses, no
  key material or passwords in any tracked file, and `vm-test/` keys are
  untracked as expected. The `fft` username appears only as the `fft@localhost`
  example in the VM manuals — a throwaway local VM account. The sample logs
  are genuinely synthetic: `hosts/sample-logs/generate.py` is deterministic
  and its header says so, `probe-vm` is invented, and no test asserts on their
  contents.
- [x] Move the synthetic attacker sources off **real, routable, attributable**
  /24 prefixes (user decision 2026-07-28). `ATTACKER_NETS` now draws from the
  RFC 5737 documentation ranges, each source getting a disjoint 14-address
  block so "which source is most persistent" and "isolate one source's whole
  session" keep working, and the region labels (`CN-Telecom`, `Tor-exit`, …)
  became behavior labels (`botnet-1`, `anon-relay`, …). Regenerating shifted
  the whole corpus, not just the addresses: `randint` uses rejection sampling,
  so narrowing the range changed how much of the random stream each draw
  consumes (`ssh-bruteforce.log` 2293 → 2182 lines). Every figure quoted in
  `hosts/sample-logs/README.md` was therefore recomputed against the new set,
  not just the IP ones: the failregex now matches 505 lines / 70 distinct IPs
  and spares 26 allow-listed 4xx (was 499 / 89 / 23).
- [x] Shoot and curate four README screenshots (2026-07-28, user at the
  keyboard). The 51 existing shots were all of the acceptance fixture —
  deterministic comparison content, so the frames read "acceptance fixture"
  and "item A3, B4" — excellent evidence, poor showcase. These four were shot
  fresh against an **orthogonal design**: four themes are the floor (a theme
  is global, so one per frame), and every other dimension rides along.

  | Frame | Theme | Language | Highlight host | Also shows |
  | --- | --- | --- | --- | --- |
  | 1 | `dun` | en | syntect | split, both panes highlighted, clip indicators |
  | 2 | `msedit` | zh-Hans | pygments | dropdown menu, mnemonics, shortcut column |
  | 3 | `turbo` | en | — | plugin menu + author mnemonics, three-window filter workflow |
  | 4 | `dark` | zh-Hant | lua | search-match and current-line highlight |

  Constraints found while designing it, all verified rather than assumed:
  plugin windows are **never** syntax-highlighted (`app/highlight.rs` returns
  early unless the focused window is `WindowKind::Edit`), `.log` has no lexer
  in any shipped host, and the language hint is the bare file extension — so
  `sshd_config` (no extension) and `Cargo.toml`/`*.conf` (syntect returns no
  spans) would all have photographed as plain text. Source files only, which
  is why each host highlights its own source.

  zh-Hant is machine-translated like eight other catalogs, so frame 4 was
  composed for **minimal exposure**: the only translated text on screen is the
  four menu-bar words, each matching Microsoft's official zh-hant in
  `reference/msedit/i18n/edit.toml` character for character, plus the search
  status line. 18 of 23 comparable dun terms match that reference exactly; the
  five differences are defensible (dun's `復原` for Undo is Microsoft Office's
  own term, against msedit's `還原`).

  Curation: sips downscaling made every file *larger* (interpolation destroys
  the crisp colour set — 476 KB became 680 KB at 1200px), so the Retina
  originals are kept. Re-encoding with per-scanline filter selection and
  maximum deflate took 1290 KB → 924 KB losslessly, verified pixel-identical
  by hash rather than by trusting the size drop. Quantizing to 256 colours
  first (Wolfram `ColorQuantize`, no dithering) took it to 798 KB. That step
  is lossy, so it was checked by eye at high zoom on both extremes — the
  subtle `dun` palette and the high-contrast `turbo` frame — and is
  indistinguishable; max per-channel error is 5–9 of 255. Note that
  Wolfram's own PNG export was *worse* than the hand-written encoder
  (413 KB against 335 KB on the same image) and that dithering doubled the
  file, so the win came from combining its quantizer with our encoder.
- [x] Delete working-tree litter (`.DS_Store`) and check that every script in
  `acceptance/` is still referenced by a document.

### B. Documentation

- [x] Rewrite `README.md` to describe what `dun` is today. The old Status
  section was a 220-line accretion of implementation notes — sentences like
  "the advanced Command Output family was removed in the 2026-07 slimming
  stage" — which is a development log, not a front page. Replaced with what
  `dun` is, why remote editing is its case, what it does, plugins, themes,
  installing, and a document index split into user-facing and contributor
  sections. Detail moved into the user guide rather than being deleted.
- [x] Write `docs/user-guide.md`. Written against the running binary rather
  than the old docs: `--help`, `--dump-config`, and the source of truth for
  search flags and confirm-dialog keys were all read off the program.
- [x] Write `docs/plugin-authoring.md`, the author-facing counterpart to the
  protocol spec. Its minimal host is extracted and run through
  `hosts/check-host.py` — which rejected three defects in the first draft
  (missing `v` protocol version, missing `role` on the response, and span
  fields written as `start`/`end` where the wire format is
  `start_col`/`end_col`). The committed example passes.
- [x] Write `docs/dev/testing-guide.md` so the harness can be rebuilt
  elsewhere: the five test layers and what each is blind to, VM creation from
  scratch (NAT port forwards 2222/2233/2244, dedicated SSH key, passwordless
  sudo, per-OS toolchain), the `vm-test` wrappers, the size-measurement
  contract, and the acceptance tooling. Package names were verified on the
  three running guests, not recalled — Solaris IPS names are pathed
  (`developer/rust/rustc`), and the `grep -E` failure in that check is itself
  one of the documented quirks.
- [x] Document `acceptance/sweep-logfilter.sh` in both
  `docs/dev/real-terminal-acceptance.md` and the testing guide.
- [x] Fold a keybinding reference into the user guide, stating that
  `--dump-config` and the in-app Help are authoritative.
- [x] Decide the language of `docs/dev/real-terminal-tui-testing.md`: kept in
  Chinese with an English preface that says why, points English readers to the
  testing guide for the same ground, and notes that the note predates both the
  ratatui and crossterm replacements it mentions in passing. Translating a
  333-line design note that records reasoning, and whose operational content
  the new guide now covers in English, was not worth the cost.
- [x] Demote `docs/dev/PLAN.md` to a historical architecture plan, with a
  header saying so, why its Phase 9 boxes were never ticked, and where the
  live plan is. Maintaining two live plans was the actual defect.
- [x] Clear the remaining stale statements. `AGENTS.md`'s "Current Build
  Stage" section needed more than its `ratatui` mention removed — the whole
  section described the *first* implementation line ("do not start with plugin
  runtime integration") years of work later; it is now a crate-boundary
  section stating the dependency invariant. `CLAUDE.md` still said five
  crates and omitted `dun-plugin`. The i18n item is closed as provenance.
- [x] Fix the stale documentation paths the stage-A move left **outside**
  Markdown — 11 files, in shell scripts and Rust `//!` comments, which the
  link checker could not see because it only walked `*.md`. The checker now
  scans every tracked text file for bare repository-path mentions and only
  parses links in Markdown; that is what turned an invisible class of breakage
  into a gate. `cargo check` confirms the comment edits compile.

### C. Release artifacts

- [ ] Add `LICENSE`: MIT, `Copyright (c) 2026 Si-Qi Liu`. `Cargo.toml:17`
  has declared `license = "MIT"` all along with no such file in the tree.
- [ ] Write `CHANGELOG.md` for v0.1.0 by distilling *user-visible* changes
  out of docs/dev/PROGRESS.md's 1,360 lines of development record.
- [ ] Complete the `Cargo.toml` package metadata: `description`,
  `repository`, `authors`, `keywords`, `categories`, `readme`.
- [ ] Decide `docs/dev/PLAN.md:275` — "Security audit suite for plugin policy after
  plugin APIs exist" is still unchecked, and the plugin APIs now exist.
  Thirteen trust/policy tests live in `dun-config/src/tests/plugins.rs` and
  `dun-cli/src/tests/plugins/`, but there is no named audit suite the way
  control-byte rendering has one. Either build the suite or record why the
  existing coverage closes it. Do not tag v0.1 with an open security box.
- [ ] Decide whether the GitHub release attaches macOS/Debian binaries.
  Current lean: no. `scripts/release-build.sh` depends on `RUSTC_BOOTSTRAP`
  and rust-src, so an outsider cannot reproduce the bytes; publishing
  binaries without a reproducible recipe is a liability. Ship source-build
  instructions instead.
- [ ] Confirm with the user whether the public repository should carry the
  conventional `CONTRIBUTING.md` (a short pointer to AGENTS.md) and
  `SECURITY.md` (how to report a vulnerability).
- [ ] Release sign-off: run the release smoke checklist, the dual-platform
  size gate, and the four-platform functional matrix, then tag `v0.1.0`.
  This is the only step that needs the VMs — ask the user to start all three.

## Completed Stage: Feature Triage and Slimming (2026-07-10)

Outcome in [docs/dev/feature-triage.md](./docs/dev/feature-triage.md) and
[docs/dev/release-size-audit.md](./docs/dev/release-size-audit.md): C/D batches 1-3
removed the advanced Command Output family, Outline, bookmarks, and
visible-whitespace markers (-48 KiB Debian); the decisive lever was the
build-std release contract (`scripts/release-build.sh`, user decision
2026-07-10), which drops panic-backtrace machinery while keeping panic
hooks and messages (verified). Post-contract binaries: Debian 620,928 /
macOS 575,460 bytes — margin 427,648 on the binding platform. All
remaining B-class features are KEPT; no lazy trim order remains.

- [x] Classify inventory units and execute C/D removals (batches 1-3).
- [x] Measure removals and the toolchain lever on the Debian VM.
- [x] Adopt the build-std budget build contract (user decision 2026-07-10).
- [x] Rewrite docs/dev/feature-budget.md from the completed triage.

## Completed Stage: In-House Terminal I/O (2026-07-23)

The five-step crossterm-replacement track from briefs 041–046 is complete.
`dun-cli` now owns terminal lifecycle, its safe Unix sys shim, VT output and
events, the bounded input parser, and the SIGWINCH-aware event reader. The
direct-poll design passed the 780/0 workspace matrix on macOS, Debian, FreeBSD,
and Solaris; the final dependency closure reduced the lockfile from 42 to 26
packages without changing surviving versions.

- [x] Own fixed VT output and terminal lifecycle/sys operations.
- [x] Migrate application dispatch to owned event types.
- [x] Replace the input parser and event loop, including resize handling.
- [x] Remove the retired dependency family and close the named docs.

## Completed Stage: Plugin Protocol Client (closed 2026-07-13)

This stage built the required host-neutral plugin protocol client; all its
release gates closed 2026-07-13 at `fd31719` (see "Release Gates for This
Stage" below). It did not wait for `rum`; `rum` remains a future optional
pure-sandbox host that must speak the same protocol. The client's measured
cost (~76 KiB Debian, spike branch `spike/plugin-client-size`) fit the
post-build-std margin with room to spare.

The protocol client is a required runtime feature under
[docs/dev/feature-budget.md](./docs/dev/feature-budget.md). The budget gate is the
`scripts/release-build.sh` binary on both audited platforms.
External plugin hosts and future runtime packages are separate artifacts and do
not count toward the default `dun` executable size.

Implementation reference: [docs/plugin-protocol.md](./docs/plugin-protocol.md).

### Protocol Specification

Reconciled against the implementation 2026-07-13; these were built with the
client but never checked off.

- [x] Freeze protocol v0 message envelope: protocol version, `request_id`,
  `plugin_id`, `role`, optional buffer/stream `revision`, and payload
  (`PROTOCOL_VERSION = 0` and `Envelope` in `crates/dun-plugin/src/proto.rs`;
  version mismatch is a structured rejection).
- [x] Define length-prefixed stdio framing:
  `u32 little-endian payload_length` plus UTF-8 JSON payload
  (`crates/dun-plugin/src/frame.rs`).
- [x] Define frame and payload caps before allocation (`read_frame` checks
  the cap before allocating; per-plugin `Policy::max_frame_bytes`).
- [x] Define structured protocol errors for malformed frame, unsupported
  version, unknown role, policy rejection, timeout, cancellation, host crash,
  oversized output, and stale revision (`FrameError` + `ProtocolError` +
  `PluginError`; cancellation surfaces as the timeout path that triggers it).
- [x] Define stderr handling as bounded human diagnostics, never protocol
  (reader thread caps the tail at `Policy::max_stderr_bytes`; stderr is
  never parsed as frames).

### Role and Policy Model

- [x] Define `PluginRole` — v0 ships `SyntaxHighlight` only (plan decision
  2026-07-13): additional variants (`LogFilter`, `TextTransform`,
  `ConfigHelper`, …) land with the "Distinctive Plugins" stage below, driven
  by real hosts rather than speculatively. The envelope carries the role as
  a string id, so new variants are a local change.
- [x] Define `TrustClass`: `pure-sandbox` and `user-trusted-external`;
  `unsupported-unsafe` is deliberately not modeled — unknown trust classes
  are rejected at config parse and handshake instead (documented on the type
  in `proto.rs`).
- [x] Define `PluginPolicy` (`Policy`: max frame/snapshot-lines/spans/stderr/
  diagnostics caps + timeout). Allowed outputs are enforced by construction —
  validated `StyleSpan`s are the only output channel — and user confirmation
  is the explicit `plugin.<id>.trust` opt-in in config. Revisited 2026-07-23
  when `LogFilter` landed: the existing per-plugin caps sufficed unchanged
  (plus `max_surface_lines` added by the surface-write slice); nothing
  per-role was needed.
- [x] Define plugin manifest/config fields (baseline 2026-07-10:
  `plugin.<id>.command/trust/roles/timeout_ms/max_frame_bytes`, typed and
  validated in dun-config without a dun-plugin dependency). Per-role policy
  overrides and a runtime field were revisited 2026-07-23 at the Distinctive
  Plugins closure and stay unadopted — `LogFilter` landed on the existing
  per-plugin fields; reopen only if a real host demonstrates the need.
- [x] Reject unknown trust classes, unknown roles, missing command paths, and
  any direct filesystem/process/network/terminal/editor authority request
  (config parser rejects unknown trust/role values and unknown fields with
  tests; `validate_plugin_entries` requires command/trust/roles; the protocol
  has no authority-request fields, so there is nothing to grant by
  construction).

### Transport and Host Lifecycle

- [x] Add a small Rust-owned protocol client module or crate without adding
  `rum` or heavy runtime dependencies (`crates/dun-plugin`: hand-rolled JSON,
  no external dependencies).
- [x] Launch configured external hosts directly, not through a shell
  (worker-thread lifecycle in `crates/dun-cli/src/plugins.rs`; lazy launch,
  failure cooldown, relaunch on next request).
- [x] Pass only stdin/stdout/stderr plus a minimal environment or explicit
  whitelist (env is cleared in `HostClient::launch`).
- [x] Implement `Hello`/`HelloAck`, role `Request`/`Response`, `Diagnostic`,
  `CancelRequest`, `Error`, and `Shutdown` paths. `LoadPlugin`/`UnloadPlugin`
  message kinds are deliberately absent from v0: the model is one plugin per
  host process, so host lifecycle is plugin lifecycle (editor-level
  `plugin load`/`unload` commands, brief-011). Protocol-level load messages
  return only if the Distinctive Plugins stage produces a real multi-plugin
  host need.
- [x] Add per-request timeout and cancellation (configured
  `plugin.<id>.timeout_ms` maps onto the client policy).
- [x] Kill or quarantine a host after malformed frames, oversized output,
  timeout, failed cancellation, EOF during frame, or process crash. Every
  error path in `request_highlight`/`handshake` calls `HostClient::kill`
  (`child.kill()` + `child.wait()`, reaping the process), and `Drop` does the
  same; the failure matrix exercises each path.
- [x] Ensure plugin host failure never corrupts buffers, file state, terminal
  state, or workspace layout. The wiring `Err` branch only sets a bounded
  status message and never touches buffer state; pinned by
  `highlight_failure_leaves_buffer_and_prior_highlight_untouched`.

### First Applied Role

- [x] Implement one visible low-risk role end to end (`SyntaxHighlight`;
  verified in a real tmux session against the fixture host: spans render
  with the theme syntax palette).
- [x] Send bounded visible-window snapshots with buffer revision and a
  file-extension language hint (focused Edit pane, coalesced on the worker).
- [x] Validate returned style spans: known style ids, in-range coordinates,
  normalized ranges, bounded count, matching revision (client validation +
  defensive re-checks at editor conversion).
- [x] Discard stale results when the buffer revision has changed (client
  layer and again at cache application in the editor).
- [x] Apply validated results through the UI highlight pipeline (shared
  body-span geometry with selection/search; syntax < search < selection
  paint order) without granting plugins UI or terminal access.
- [x] Keep plugin diagnostics sanitized and visible through the status
  surface (bounded error text; status rendering is already sanitized).

### Fixture Hosts and Tests

- [x] Add a Rust fixture host for CI-grade protocol tests
  (`crates/dun-plugin/src/bin/fixture-host.rs`, mode via argv or program
  name).
- [x] Add optional example hosts outside the required CI path: `hosts/`
  carries syntax-highlight hosts in Rust (syntect), Python (Pygments), and
  dependency-free Lua, plus `hosts/check-host.py`, a language-agnostic
  conformance checker. All three pass the checker on Debian (2026-07-11).
- [x] Test handshake success and protocol-version rejection.
- [x] Test normal request/response for the highlight role (client-level;
  editor application pending).
- [x] Test malformed/truncated frames, oversized frame, span flood,
  forbidden output (coords/style/request-id), timeout, host crash, stderr
  flooding, and a correctly framed non-JSON payload (16-test matrix in
  `crates/dun-plugin/tests/protocol.rs`). The explicit cancellation-path case
  is intentionally not added: cancellation only fires on timeout (a
  best-effort `CancelRequest` immediately followed by kill), already covered
  by the timeout test; a dedicated test would be racy with marginal value.
- [x] Test stale revision rejection at the client level (editor-state
  invariance is checked again at wiring).
- [x] Rejected plugin output cannot request file I/O, process spawn, terminal
  writes, direct buffer mutation, or raw control-byte rendering. Guaranteed by
  construction: `request_highlight` returns only a validated
  `Vec<StyleSpan>` (or an `Err`), so a host has no channel to those effects;
  applied spans are re-sanitized at render. No behavioral test can exercise a
  capability the type does not expose.

### Open Investigations

- [x] Handshake latency spikes (resolved 2026-07-11): the >500 ms spikes were
  the macOS first-exec malware scan on the old shebang-script launchers — each
  run created fresh scripts (fresh inodes), each triggering a hundreds-of-ms
  scan. The hard-linked launcher fix (brief-003) reuses the already-scanned
  fixture inode and removed the cause. Re-measured with the `#[ignore]`
  `measure_handshake_latency_sequential_vs_parallel` diagnostic: launch
  (spawn + reader threads + hello/hello-ack) is ~3 ms sequential and ~16 ms
  worst-case at 24-way parallelism, in debug, even with the full protocol
  suite running concurrently — modest scheduler contention, not a protocol
  bug. Tight per-host timeouts are safe in production, where a host does not
  compete with dozens of concurrent test spawns. The diagnostic stays in
  `protocol.rs` for re-measurement if spikes ever recur on other hardware.

### Release Gates for This Stage

All gates closed 2026-07-13 at `fd31719`; the Debian run doubled as the
post-client re-audit (docs/dev/release-size-audit.md "Post-client re-audit").

- [x] `cargo fmt --all -- --check` (clean, 2026-07-13).
- [x] `cargo clippy --workspace --all-targets -- -D warnings` (clean,
  2026-07-13).
- [x] `cargo test --workspace` (606 passed / 0 failed, 2026-07-13).
- [x] Release smoke checklist passes (2026-07-13: tmux_grid, msedit_diff,
  release `test-panic-hook` pty_smoke, `strings` panic-trigger check 0,
  `--version`/`--dump-config` on both platforms).
- [x] macOS `scripts/release-build.sh` binary stays within `1,048,576` bytes
  (625,060 bytes at fd31719, 2026-07-13).
- [x] Debian VM `scripts/release-build.sh` binary stays within `1,048,576`
  bytes (670,080 bytes at fd31719, margin 378,496, 2026-07-13).
- [x] Neither binary exceeds budget; no triage consultation needed.

Do not add runtime features while either audited release binary exceeds the
1 MiB budget.

## Completed Stage: UI Text Internationalization (i18n; closed 2026-07-13)

Closed 2026-07-13; the one open item below (native-speaker corrections) is
additive and adopted as contributions arrive.
Approach decided 2026-07-11 (user decision): do NOT compile all
translations into the binary. English stays compiled in as the `&'static`
fallback so the single binary keeps working on any remote host with no
resource directory; other languages load at runtime from optional external
per-language resource files (key → string) selected by `LC_MESSAGES`/`LANG`.
Only the mechanism plus English count against the size budget. Starts after
the plugin-client stage closes (release gates + dual-platform re-audit).

Slice 1 (mechanism + menus) landed 2026-07-13; design of record is
[docs/i18n.md](./docs/i18n.md).

- [x] Define the i18n key model and the `i18n/<lang>.conf` resource format
  (same `key = value` line format as the config file), lookup order
  (`LC_ALL`/`LC_MESSAGES`/`LANG` → English fallback), and the search
  location (`i18n/` next to the active config file). Documented in
  docs/i18n.md.
- [x] Change UI label types from `&'static str` to `Cow<'static, str>`
  (`MenuItem`/`MenuEntry`) so the built-in language stays zero-cost while
  loaded translations are owned strings.
- [x] Extract menu labels to keys (`menu.*` in
  `crates/dun-ui/src/frame/menu.rs`) with mnemonic-preserving composition:
  translations supply base text only; the mnemonic letter always comes from
  the compiled English label, so uniqueness and keyboard navigation hold by
  construction in every language.
- [x] Load resource files with bounded file size (64 KiB cap before
  allocation), and reject any value the display sanitizer would escape
  (control bytes, ESC, bidi formatting, invisible zero-width): the load
  check literally runs the sanitizer, so it can never drift from what
  rendering enforces. Broken files reject whole with a line-numbered
  status diagnostic; the editor stays English.
- [x] Fall back to English on ASCII terminals (`EncodingProfile::Ascii`):
  non-ASCII translated text would only be sanitizer-escaped there. Wide/CJK
  display itself already works (Surface + unicode-width).
- [x] Ship the first reference translation: `i18n/zh-CN.conf` (menus), with
  a completeness test binding it to the menu keys. Translator guide in
  docs/i18n.md.
- [x] Extract help window fixed strings — slice 2, landed 2026-07-13:
  ~125 keys (title, section headers, command descriptions keyed by command
  id as `help.command.<id>`, prompt/selection/navigation/menu/notes rows),
  all translated in `i18n/zh-CN.conf` with a completeness test that fails
  listing any missing key (`help_translation_keys`). Key-cap columns stay
  English; column padding is by display width, not char count (the
  translated "(未绑定)" key column exposed the misalignment).
- [x] Extract dialog titles, buttons, and prompt labels — slice 3, landed
  2026-07-13: 48 keys in `crates/dun-cli/src/ui_text.rs` (single source;
  the completeness test enumerates `ui_text::ALL` and also rejects
  placeholder-count drift). Covers prompt modal titles, unsaved/replace
  confirm dialogs, buffer switcher, file dialog chrome, and helper-window
  titles. Introduced the `{}` template mechanism (`tr_fmt`): translations
  can reorder arguments, and a template whose placeholder count mismatches
  falls back to English so runtime values are never dropped. Confirm-button
  letters (s)/(d)/(c)/(r)/(a) stay functional English keys appended by
  code. Stateful `self.message` strings (dialog event messages) move to
  slice 4 with the status work.
- [x] Extract status messages into parameterized templates — slice 4a,
  landed 2026-07-13 via Codex brief-022 (gated: scope/fmt/clippy/617 tests
  reproduced, zh reviewed, mutation-checked): 172 call sites in `app/*.rs`
  converted, 158 keys (148 `status.*` + 10 `prompt.*.label/name`), table
  now 206 unique keys with a uniqueness test; three behavior tests pin the
  exact English baseline and the zh output through real command paths.
  Debian size measurement pending the next VM session (macOS 645,580,
  +12,296).
- [x] Slice 4b-1: the stateless helper text builders — landed 2026-07-13 via
  Codex brief-023 (gated: scope/fmt/clippy/619 tests reproduced, zh reviewed,
  mutation-checked). All 50 helper-composed call sites brief-022 deferred are
  converted: `buffer_error_text`, `workspace_error_text`, `axis_name`,
  `replacement_status_text`, `command_line_parse_error_text`,
  `COMMAND_LINE_HELP`, `command_run_status`/`exit_status_text`,
  `opened_file_status`/`reloaded_file_status`,
  `status_with_atomic_temp_report`, `ConfigSource::status_text`, and
  `PromptCompletionState::status_text` (13 builders, 81 keys). Established
  the **vocabulary rule**: text the user types back — theme names, config
  section tokens, command ids — stays English and is passed as a `{}`
  argument; only the prose around it translates. `ui_text.rs` was split into
  `ui_text/{mod,chrome,status}.rs` on the way.
- [x] Slice 4b-2: the stored / type-erased text — landed 2026-07-13 via Codex
  brief-024 (gated: scope/fmt/clippy/624 tests reproduced, zh reviewed,
  mutation-checked twice). `FileDialogState::message` became a typed
  `FileDialogMessage` enum rendered at `overlay()`; dun-authored path errors
  became a typed `PathIoError` carried *inside* `io::Error` via its custom
  payload API (`io::Error::new` + `get_ref`/`downcast_ref`), so the
  `io::Result` plumbing never changed; `"Untitled"` is overwritten by dun-cli
  at every creation point (dun-core stays catalog-free); Command Output buffer
  content is translated and the English `exit_status_text` is retired. 36 keys.
- [x] Slice 4c: split `ui_text/status.rs` — landed 2026-07-13 via Codex
  brief-025. 42k/265 keys became `ui_text/status/{window,file,edit,search,
  prompt,command,command_output}.rs`, each under 10k, with `ALL` still a single
  flat enumeration and glob re-exports keeping every call site untouched. Gate
  verified the move was byte-faithful by comparing the pre/post key tables as
  sorted sets (265 keys, English defaults identical), and the release binary is
  unchanged to the byte. The size exception in
  docs/dev/code-organization-guidelines.md is retired.
- [x] Extend `i18n/` to more languages — done 2026-07-13, ten shipped:
  `zh-Hans`, `zh-Hant`, `fr`, `de`, `it`, `es`, `pt`, `ru`, `ja`, `ko`
  (briefs 027–029). Bare language tags except Chinese, which needs a script
  tag; the locale chain's script step means one file serves every region of
  its language. Every file is validated by
  `every_shipped_translation_is_valid_and_complete`, which discovers the
  directory rather than naming files, so a contributed language is covered
  the moment it lands. Each shipped file states in its header that it is
  machine-translated and unreviewed by a native speaker.
- [x] State the translation provenance rather than tracking a review that will
  never happen. The nine non-Chinese catalogs are machine-translated and
  unreviewed; there is no native-speaker reviewer, so this stopped being a task
  and became a fact recorded in docs/i18n.md and the header of every shipped
  catalog. What *is* mechanically enforced stays enforced: key completeness,
  placeholder shape, English mnemonics and key caps by construction, and the
  pairwise-distinct test guarding the one destructive case (Save/Discard/Cancel
  beside the literal `(s)`/`(d)`/`(c)` keys). A wording correction from a
  speaker of the language is welcome as a contribution, not awaited as a gate.
- [x] Measure the size delta per batch; the mechanism must stay lean since
  hand-rolled parsing (no serde) remains the rule. Slice 1 measured both
  platforms at `fd31719`: macOS +12,344 (625,060); Debian span including
  briefs 012-021 lands at 670,080, margin 378,496. Debian debt for slices
  4a-4c settled 2026-07-15 at `744c843` (≡ `1d03433`): 715,136 bytes,
  +28,672 over `89cd9e4`, margin 333,440 (docs/dev/release-size-audit.md
  2026-07-15). Translations stayed free.

## Completed Stage: Distinctive Plugins (Capability Model First; closed 2026-07-23)

**Stage closed 2026-07-23.** The mechanism, the open capability APIs (slices
A–D below), and the three v0 data channels are all built and Debian-measured.
The first real consumers — the dependency-free Python and Lua `log-filter`
hosts under `hosts/` — served as the ergonomics acceptance test and passed
live tmux acceptance on macOS, Debian, FreeBSD, and Solaris
(`crates/dun-cli/tests/tmux_logfilter.rs`: menu injection, keybinding →
scratch, execute → surface, command → stream → surface; green at `877b7ad`,
2026-07-23). All three acceptance findings are fixed: oversized stream feeds
now chunk with a FIFO pending queue (`4a841e2`), the hosts' leader moved from
`Ctrl+L` (built-in SelectLine collision) to `Ctrl+T` (`c560379`), and a
collision-rejected keybinding contribution surfaces a status diagnostic
instead of vanishing silently (`959915f`). The v0 capability surface is
frozen in docs/plugin-protocol.md. Closure decisions: the slice-A "sum-typed
validator dispatch" (still listed as pending under B below) is retired as
superseded by construction — validators are keyed by capability in
`validate.rs` and each typed request method dispatches statically; per-role
policy overrides and the runtime config field stay unadopted (see the Role
and Policy Model item above). The B-inherited window/scratch open path and
ownership reaping landed with C chunk 3 and si-2.

Plan decided 2026-07-13; **reframed capability-model-first 2026-07-16.**
`role` was still described in embedded-`rum` permission terms; across a
protocol boundary that meaning is dead (`dun` cannot grant an external
process OS authority). The redesign makes `role` a **named bundle of
inward capabilities** — typed, validated channels into `dun`-owned objects
(buffer, stream, overlay, surface, window, scratch-input/execute, menu,
keybinding). Design lives in docs/plugin-protocol.md ("Capability Model",
"Capability Infrastructure"). Trust class becomes the capability grant gate.

Decision: **build the mechanism and the open capability APIs first, driven
by fixture hosts, before any concrete product plugin.** A `log-filter`-shaped
host (`{ stream-read, surface-write, window, scratch-input, menu, keybinding }`)
is the intended first real consumer and the ergonomics acceptance test — it
is deferred until the APIs exist, and the surface is not frozen until it has
been built against once. Each slice ships a minimal fixture host + protocol
tests + a Debian size measurement (capability cost attributed per batch;
deltas non-additive).

- [x] **A — mechanism spine (primitives; landed 2026-07-16, hand-written).**
  In `dun-plugin`, zero binary delta (dead-stripped until B links it; macOS
  budget build byte-identical at 649,900): the `Capability` vocabulary as
  types, `Role` as a named capability bundle (`Role::capabilities`) plus
  `Role::output_capability`, `GrantedCapabilities` (trust-gated grant,
  unit-tested including the denial branch, mutation-proven), and `validate.rs`
  reframed as the `overlay-write` capability's validator. **Deferred into B**
  (tautological/untestable until a gated capability or owned surface exists,
  and linking them early adds binary weight for no functional gain): the live
  handshake grant + `dun-cli` role→capability wiring, `plugin_id` ownership
  tagging + unload reaping, and the sum-typed per-capability validator
  dispatch (one validator today).
- [x] **B — windows + scratch input.** `window` lifecycle (≤2/plugin +
  aggregate/terminal fallback, own-only destroy); `scratch-input` = a
  `dun`-native editable buffer + `execute` submit (snippet runs in the host
  interpreter, never in `dun`; no keystroke routing). **Also inherits from A:**
  the live handshake trust-gated grant + `dun-cli` role→capability wiring,
  `plugin_id` ownership tagging + unload reaping, and the sum-typed validator
  dispatch — all first become non-tautological here (the `window`/`scratch`
  gated capabilities and owned surfaces), and this is where the first Debian
  size measurement of the capability machinery is taken.
  - In progress: the `PluginWindows` ownership registry primitive (≤2/plugin
    cap, own-only destroy, reap) landed via brief-030 (Codex-executed,
    Claude-gated; own-only mutation-proven; zero binary delta, still unwired).
    Workspace wiring + `WindowKind::PluginSurface` pending.
  - Landed `c0f610e` (Claude-authored): the live grant is wired and enforced —
    `HostClient` computes `GrantedCapabilities` from roles+config trust after
    the handshake and refuses `request_highlight` without `overlay-write`; the
    config↔handshake trust cross-check rejects an over-claiming host. Two new
    protocol tests, both mutation-proven. First binding Debian measurement of
    the capability stage taken: 715,136 bytes (+0 from `744c843`; macOS +4,104
    to 654,004). Still pending in B: the `window`/`scratch` open path (needs a
    trigger — menu/keybinding or a protocol window-action), ownership reaping
    wired to real windows, and the sum-typed validator dispatch.
- [x] **C — menu.** One top-level subtree per plugin, label i18n (`en_US`
  required + optional tags), menu-invoke dispatch, structural bounds,
  menu-bar width handling.
  - In progress: the `PluginMenu` contribution model + validator + label
    resolution (`dun-plugin/src/menu.rs`) landed via brief-031 (Codex-executed,
    Claude-gated; `en_US`-required and control-char guards mutation-proven;
    pure `pub` module, zero binary delta, unwired).
  - Spine chunk 1 landed (Claude-authored): `Role::LogFilter` bundle
    (`{ stream-read, surface-write, window, scratch-input, menu, keybinding }`,
    trust-gated) makes `menu`/`window` grantable, and `HostClient` parses the
    handshake-carried menu contribution — honored only when the host holds
    `menu`, ignored otherwise (capability gate mutation-proven). macOS +8 to
    654,012.
  - Ordered chunk 1 landed 2026-07-17 (Claude-authored): **host-layer
    generalization (option A, atomic)** — `PluginHighlighter` → `PluginHost`
    (per configured entry: worker channel, client lifecycle, `plugin_id`,
    `granted`, `menu: Option<PluginMenu>`), collected in `PluginHosts`
    replacing `AppState.highlighter`. Highlight scheduling is one facet
    (routed to the first `syntax-highlight`-role host); the worker ships the
    launched client's menu to the main thread via a `Started` event (installed
    on the host, cleared on unload, reinstalled by relaunch);
    `PluginHosts::menus()` gathers contributions for chunk-3 injection
    (`#[allow(dead_code)]` until then). **Hybrid launch built as decided**:
    highlight-only hosts stay lazy, `menu`/`window`-granted hosts launch
    eagerly at startup and on `plugin load`. `plugin`/`load`/`unload` and the
    status indicator now address hosts by `plugin_id` (bare load/unload stays
    valid for a single host; two new status keys + two reworded, all ten
    translations updated). Five mutations killed (eager gate, menu install,
    menu clear-on-unload, StartFailed flag, get_mut addressing).
  - Ordered chunk 2 landed 2026-07-18 (Claude-authored): **dun-core typed
    variants** — `WindowKind::PluginSurface` and
    `EditorCommand::PluginMenuAction { plugin_id, action_id }` added with every
    exhaustive-match arm. `dun-config::command_id` maps every `PluginMenuAction`
    to the generic `plugin.menu_action` id (deliberately absent from
    `ALL_COMMAND_IDS`, no `command_from_id` round-trip — plugin actions are
    never user-bindable); the top-level `handle_command` dispatch arm is a no-op
    placeholder until chunk 3. No other exhaustive match needed touching
    (`WindowKind` sites all carry a wildcard; editability is a `BufferKind`
    decision and highlight already skips non-`Edit` panes). One new
    mutation-proven test pins the generic-id / non-bindable contract
    (`plugin_menu_action_has_a_generic_non_bindable_id`). macOS budget build
    +4,120 to 662,252 (owned-`String` payload adds drop/clone glue across
    `EditorCommand`'s pervasive use; proxy — folded into the owed chunk-4 Debian
    measurement).
  - Ordered chunk 3 landed 2026-07-18 (Claude-authored), two commits:
    - Part 1 — **menu injection**: `UiShell` gains a `plugin_menu_items:
      Vec<MenuItem>` that `menu_bar()` appends after the built-in menus, so
      rendering, hit testing, and keyboard/mouse dispatch see one consistent
      list. dun-cli resolves each host's `PluginMenu` into a `MenuItem` whose
      entries carry `EditorCommand::PluginMenuAction { plugin_id, action_id }`
      (`PluginHosts::resolved_menu_items`, labels resolved against the active
      locale chain — empty/`en_US` on ASCII or `--no-config`).
      `refresh_plugin_menus` recomputes each pump (a handshake is absorbed
      inside `poll` without surfacing) and on `plugin load`/`unload`,
      reassigning only on change.
    - Part 2 — **dispatch + window path** (also completes B's window path):
      `PluginMenuAction` dispatches to `dispatch_plugin_menu_action`, gated on
      the invoked host holding `window` (`PluginHost::holds_window`); each
      invoke opens a read-only `WindowKind::PluginSurface` split owned by the
      plugin, subject to the ≤2/plugin cap (`PluginWindows`, now wired — the
      `#![allow(dead_code)]` is gone). Ownership is reaped on `plugin unload`
      and config reload (`reconcile`/`reap_all_plugin_windows`) and released
      when the user closes a surface (`window.close`/`only` → `release`). Every
      invoke opening a fresh surface (vs focus-existing) is an interim; the
      per-action request round-trip is deferred to the first real consumer.
      One new i18n key (`status.plugin.window-limit`, all ten translations).
      Five mutations killed (window gate, ≤2 cap ×2, unload reap, close
      release). macOS budget build +8,240 over chunk 2 to 670,492.
  - Ordered chunk 4 landed 2026-07-18: **binding Debian measurement + release
    smoke for the whole C spine.** On a clean `vm-sync` archive of `bbc3fa7`,
    `scripts/release-build.sh` measured **735,616 bytes** (margin 312,960),
    +20,480 over the last binding baseline `c0f610e` (715,136) for the whole
    span (handshake menu grant, host-layer generalization, dun-core typed
    variants, menu inject + dispatch + window path); the ten
    `status.plugin.window-limit` translations were free. Debian smoke: ELF PIE
    stripped, `ldd` unchanged, `--version`/`--dump-config` clean, `strings`
    panic-trigger 0. macOS gate: `tmux_grid`/`msedit_diff`/release
    `pty_smoke` pass, `strings` 0. Recorded in docs/dev/release-size-audit.md
    (2026-07-18). **The C (menu) stage is complete; no Debian measurement debt.**
- [x] **D — keybinding.** Code landed 2026-07-18 (Claude-authored), three commits;
  one binding Debian measurement + smoke owed (see below).
  - D-1 (`1aa6bf8`): renamed `EditorCommand::PluginMenuAction` -> `PluginAction`
    (`plugin.menu_action` -> `plugin.action`) — a menu item and a leader chord
    produce the same "invoke plugin action" command.
  - D-2 (`1273794`, dun-plugin): `keybinding.rs` `PluginKeybinding` model (one
    leader keystroke spec + bounded, distinct chords; keys are opaque strings
    parsed by dun) + validator; `HostClient` parses a `keybinding` HelloAck
    field alongside `menu`, gated on the `keybinding` grant; fixture host
    advertises `Ctrl+J`/`p`->`ping`; 2 protocol + 8 model tests, gate + dup
    guard mutation-proven.
  - D-3 (dun-cli): the worker ships the keybinding to the main thread via
    `Started`; `PluginHost` stores it (cleared on unload); `resolved_keybindings`
    parses each leader/chord into a `[leader, chord] -> PluginAction`
    `plugin_keymap` on `UiShell`, **collision-checked** — a leader that is a
    built-in binding or prefix, another plugin's leader, or unparseable drops
    the whole contribution. `handle_key_stroke` consults `plugin_keymap` after
    the built-in keymap (reusing the existing `pending_keys` +
    `has_sequence_prefix` machine — the default keymap already uses `Ctrl+X,*`
    multi-stroke leaders, so it was live), so built-ins can never be shadowed.
    `refresh_plugin_menus` became `refresh_plugin_contributions` (menus +
    keymap). Five tests (dispatch, pending-then-unbound cancel, built-in
    collision, two-plugins-same-leader, unload clears); collision + both
    event-loop consultations mutation-proven. macOS budget build +4,104 over
    the C spine to 674,596.
  - D-4 (`b7111ef` measured): binding Debian measurement + smoke for the D span.
    On a clean `vm-sync` archive of `b7111ef`, `scripts/release-build.sh`
    measured **739,712 bytes** (margin 308,864), +4,096 over `bbc3fa7`. Debian
    smoke: ELF PIE stripped, `ldd` unchanged, `--version`/`--dump-config` clean,
    `strings` panic-trigger 0. macOS gate: `tmux_grid`/`msedit_diff`/release
    `pty_smoke` pass, `strings` 0. Recorded in docs/dev/release-size-audit.md
    (2026-07-18). **The D (keybinding) stage is complete; no measurement debt.**
    With A–D done, the capability mechanism + open APIs are all built.
- [x] Wire trust as the grant gate + the config↔handshake trust cross-check —
  landed `c0f610e`: `Capability::trust_gate_cleared` gates `GrantedCapabilities`,
  and `HostClient::launch` rejects a host whose declared trust exceeds the
  configured trust (tested at protocol.rs "exceeds configured trust"). Protocol
  enhancements continue to be recorded in docs/plugin-protocol.md as they land.

- [x] **Remaining v0 capability data channels** (the trigger/UI layer — menu,
  keybinding, window open/close, overlay highlight — is done; these three
  output/input channels give a host actual content, and none were wired by
  A–D). Build API-first, fixture-driven, one at a time (user decision
  2026-07-19), each with tests + a Debian measurement:
  - **surface-write** (host fills its own window). DONE — sw-1/sw-2 landed,
    sw-3 measured. Binding Debian measurement (`aa8b852`, 2026-07-19):
    **743,808 bytes**, margin 304,768, +4,096 over the D baseline `b7111ef`;
    smoke passed (ELF PIE stripped, `ldd` unchanged, `--version`/`--dump-config`,
    `strings` panic-trigger 0). Cross-platform functional runs: macOS 693/0,
    FreeBSD 693/0, Solaris 689/4 (the 4 = the root-caused Solaris ambiguous-width
    `wcwidth` quirk, not a defect). No measurement debt.
    - sw-1 `e319d0e` (dun-plugin): `Policy::max_surface_lines`,
      `validate_surface` beside `validate_spans`,
      `HostClient::request_surface(action_id)` (gated on `surface-write`, reuses
      the request/response transport, no role/revision), fixture answers an
      `action_id` request with lines. 2 protocol + 2 validator tests; gate and
      line-cap mutation-proven.
    - sw-2 (dun-cli): worker gains `WorkerMessage::Surface`/`HostEvent::Surface`
      (host_worker refactored into `serve_job`/`serve_surface`/
      `report_launch_failure` for clean per-item borrows). `dispatch_plugin_action`
      now opens-or-**reuses** the plugin's surface (resolving the fresh-window
      interim: one surface per plugin) and, when the host holds `surface-write`,
      sends a surface request; `apply_surface_outcome` fills the window with the
      host's validated lines on the next pump (resolving the per-action
      round-trip interim). A `window`-only host still gets an empty surface.
      Three chunk-3 window tests reworked for reuse + three new tests
      (fill-on-response, window-only-no-request, reuse); the surface-write
      dispatch gate, the render, and the reuse are each mutation-proven. macOS
      budget build 678,708 (+4,112 over the D baseline `b7111ef`).
  - **stream-read** (feed command-output stream chunks to a host). DONE —
    sr-1/sr-2 landed, sr-3 measured (`e438a13`, 2026-07-19): binding Debian
    **747,904 bytes**, margin 300,672, +4,096 over the surface-write baseline;
    smoke passed. Cross-platform functional: macOS 700/0, FreeBSD 700/0,
    Solaris 696/4 (the 4 = the root-caused Solaris ambiguous-width quirk). No
    measurement debt.
    - sr-1 `72d2d9e` (dun-plugin): `StreamChunk { stream_id, chunk_index, lines,
      final_chunk }`, `validate_stream_verdict` (one keep/drop boolean per input
      line), `HostClient::request_stream_filter` (gated on `stream-read`, reuses
      the transport, input bounded by `max_snapshot_lines`), `Json::as_bool` /
      `json::bool`, fixture keeps non-empty lines. 2 protocol + 2 validator
      tests; gate and verdict-length check mutation-proven.
    - sr-2 (dun-cli): worker `Stream` message / `StreamVerdict` event /
      `serve_stream`; `PluginHost` remembers the fed lines (`pending_stream`).
      A finished command's stdout is fed to every `stream-read` host
      (`feed_command_output_to_filters` → `feed_stream_to_filters`, additive —
      the normal output window still opens); `apply_stream_verdict` keeps the
      marked lines and shows them in the host's surface window (shared
      `fill_plugin_surface` with surface-write), dropping a length-mismatched
      verdict. Three tests (feed gate, kept-lines-in-surface, mismatch-dropped);
      the feed gate, the keep filter, and the length guard mutation-proven.
      macOS budget build 682,820 (+4,112 over the surface-write baseline).
  - **scratch-input + execute** (dun-native editable buffer + submit its text to
    the host). DONE — si-1/si-2 landed, si-3 measured (`d9c380a`, 2026-07-19):
    binding Debian **756,096 bytes**, margin 292,480, +8,192 over the
    stream-read baseline; smoke passed. Cross-platform functional: macOS 706/0,
    FreeBSD 706/0, Solaris 702/4 (the 4 = the root-caused ambiguous-width
    quirk). No measurement debt. **All v0 capability data channels are now
    built and measured.**
    - si-1 `32f0b52` (dun-plugin): `HostClient::request_execute(snippet)` sends
      `{ snippet }` (gated on `scratch-input`, reuses the transport), returns
      the host's result lines via `validate_surface`; fixture echoes a summary.
      2 protocol tests, gate mutation-proven.
    - si-2 (dun-core/plugin/cli): plugin actions gain a **kind**
      (`PluginActionKind { Surface, Scratch, Execute }`) — dun-core on
      `EditorCommand::PluginAction`, dun-plugin's `PluginMenuItem`/`PluginChord`
      parse an optional `kind` field (default Surface; unknown rejected),
      dun-cli maps wire→core. Dispatch routes by kind: Surface (unchanged),
      Scratch opens the plugin's editable `WindowKind::PluginScratch` window
      (BufferKind::Untitled, user edits with dun's engine), Execute submits the
      scratch buffer's whole text via `send_execute_request` →
      `WorkerMessage::Execute` → `serve_execute` → the result fills the surface
      window (reusing the surface path). Scratch/execute gated on
      `scratch-input`. Tests: menu kind parse/reject, scratch opens editable
      only with grant, execute submits scratch text + shows result, execute
      with no scratch window sends nothing. Four mutations killed (kind
      validator, scratch gate, execute submit, plus si-1 gate). macOS budget
      build 691,028 (+8,208 over the stream-read baseline for si-1+si-2).

## Completed Stage: OSC 52 Clipboard Read (2026-07-27)

**Closed 2026-07-27 at `42774ec`.** OSC 52 read (paste the host clipboard over
SSH via the terminal) is the read counterpart to the OSC 52 write dun already
ships. Landed over two gated steps — the armed parser seam + strict base64
decoder (`110aa08`, byte-neutral to user behavior) and the user-facing wiring
(`42774ec`: `clipboard.osc52.allow_read` opt-in, `edit.paste_external` /
`Ctrl+X,Ctrl+V` / Edit-menu Paste External, the typed query action, the 500 ms
synchronous-feel wait, the internal-clipboard fallback, i18n × ten, PTY tests,
docs). Codex hit one clean stop-loss on the 052/053 scope boundary (a forced
non-exhaustive match in the out-of-scope `shell.rs`), resolved by deferring the
RuntimeAction variant to step 2. Binding Debian 760,112 (+4,096 over
`b9ac165`, margin 288,464); macOS 698,524; four-platform functional matrix
845/0 (macOS/Debian/FreeBSD/Solaris), PTY 11/11. No new input security layer:
decoded bytes ride the existing `DisplaySanitizer`; the terminal owns the
read-gate; real-terminal read support is best-effort (documented, not a
measured matrix — the mechanism is PTY-verified). The `editing.rs` →
`app/clipboard.rs` split was deliberately not bundled and remains available if
the file crosses the size guideline. Platform-specific clipboard commands
stay rejected.

User decision 2026-07-27: implement OSC 52 **read** (paste the host/system
clipboard over SSH via the terminal) — the read counterpart to the OSC 52
write dun already ships. Platform-specific clipboard commands are rejected.
Design-of-record decisions: no new input security layer (decoded bytes ride
the existing `DisplaySanitizer` like any paste; only base64-decode + UTF-8
validation under a byte cap); the terminal owns the read-gate (most disable or
prompt by default) so dun must degrade cleanly on no response; opt-in and
default-off like the write side; safe Rust, no new deps (hand-roll
`base64_decode` beside the existing encoder); the response parses as a bounded
accumulate-to-terminator state mirroring `State::Paste`.

- [x] Design-only brief 051
  (`docs/dev/codex/brief-051-osc52-read-plan.md`): the plan — query emission,
  parser extension, response application, config/trigger surface, no-response
  fallback, ordered steps, tests, risks. Delivered 2026-07-27.
- [x] Claude reviewed/adapted the plan 2026-07-27; decisions frozen: separate
  `clipboard.osc52.allow_read` (default false, shares `max_bytes`); distinct
  `edit.paste_external` command bound `Ctrl+X,Ctrl+V` + menu `Paste External
  (E)` (internal Paste untouched); synchronous-feel 500 ms bounded wait (not
  async — a delayed reply could paste into the wrong buffer/selection, and
  OSC 52 has no request id); empty response = valid empty clipboard (no stale
  fallback); typed `RuntimeAction::QueryOsc52Clipboard { max_bytes }`.
  **Refinement beyond the plan: OSC 52 framing is armed-gated** — when no read
  is pending, `ESC ]` keeps today's `Alt+]` behavior; OSC consumption happens
  only in the ~500 ms armed window, so default input stays byte-identical.
  Late-response ambiguity accepted + documented (no quarantine).
- [x] Implementation steps per the adapted plan, each its own gated brief:
  - [x] Step 1 — decoder + armed parser framing + event seam (brief 052,
    `110aa08`). Byte-neutral; one clean stop-loss (deferred RuntimeAction to
    step 2). 834 tests; armed-gate mutation-proven by Claude; macOS +24.
  - [x] Step 2 — config + command + keymap + menu + 500 ms wait + fallback +
    PTY + behavior docs (brief 053, `42774ec`). 845 tests, PTY 11/11;
    write-grants-read gate and empty-no-stale mutation-proven by Claude; macOS
    698,524. `editing.rs`→`app/clipboard.rs` split deliberately not bundled.
  - [x] Step 3 — closure (this stage): binding Debian measurement + smoke +
    four-platform matrix + docs. (No separate brief 054 needed — the closure
    is measurement + docs, done directly.)
- [x] Dual-platform size measurement + release smoke; bounded-input-surface
  and terminal-compatibility docs updated. Debian 760,112 (+4,096 over
  `b9ac165`, margin 288,464); macOS 698,524; smoke clean (ELF PIE, ldd
  unchanged, DUN_TEST_PANIC=0). Four-platform functional 845/0. Real-terminal
  read support documented best-effort (terminal-owned gate; not a measured
  matrix).

## Completed Stage: Restoration Review — F12/F13 (2026-07-23 → 2026-07-26)

**Closed 2026-07-26 at `b9ac165`.** F12 (bookmarks) and F13 (visible
whitespace) are restored full-trail over three gated steps — display seam
`3b69844`, F13 `5914467`, F12 `b9ac165` — re-landed from the `53fe7f8^` spec
against today's architecture (i18n across ten catalogs, the shared
`EditorTextDisplay` seam, `Ctrl+X` keymap family). Binding Debian 756,016
(+16,384 over `877b7ad`, margin 292,560); four-platform functional matrix
817/0 (macOS/Debian/FreeBSD/Solaris). Codex hit one clean stop-loss on a
stale gutter-render assumption (the Surface renderer paints the separator
over the old marker cell) — resolved with Option C (the `*` replaces the
separator at the gutter edge, no width change). Full-trail docs updated
(README, feature-triage, feature-budget, release-size-audit). F46 stays
removed; F20 still returns as a plugin role.

User decision 2026-07-23: with the Distinctive Plugins stage closed, the
restoration review runs next (see the "Deferred" item, now decided): restore
F12 (bookmarks) + F13 (visible whitespace), removed 2026-07-10 at `53fe7f8`
(batch 3). A plain `git revert` is dead — a `git merge-tree` simulation
conflicts in 10 of the commit's 26 files, and clean hunks would reintroduce
pre-i18n hardcoded English — so the track is plan-first: `53fe7f8^` is the
behavior specification, re-landed against today's architecture (i18n keys +
ten translations, wide-aware dun-ui render layer, the `Ctrl+X` keymap
family, current menu/help pipelines), full-trail per AGENTS.md. Note the old
default chords do not transplant: `Ctrl+X,M`/`Ctrl+X,P` are taken by Window
Collapse/Expand today; the design brief owes a collision inventory and a
keymap proposal.

- [x] Design-only brief 047 (`docs/dev/codex/brief-047-f12-f13-restoration-plan.md`):
  the restoration plan — spec extraction from `53fe7f8^`, per-piece mapping
  to today's code, i18n key plan, keymap proposal, ordered steps, test plan,
  risks. Delivered 2026-07-23: three ordered steps — (1) a shared
  display-coordinate seam (`EditorTextDisplay` in dun-ui + a mapped
  sanitizer entry in dun-core + `GlyphSet` whitespace glyphs) with the
  oversized `editing.rs`/`buffer_state.rs` splits, behavior-identical;
  (2) F13 full-trail; (3) F12 full-trail. 18 i18n keys, full keymap
  collision inventory, named mutation targets per invariant.
- [x] Claude reviewed/adapted the plan; decisions frozen 2026-07-23:
  keymap `Ctrl+X,.` (whitespace), `Ctrl+X,K` (toggle bookmark), `Ctrl+X,N`
  (next), `Ctrl+X,L` (previous), menu mnemonics `.`/K/N/L (all verified
  free); bookmark edit semantics are the exact old behavior (only Delete
  Line / Move Line / Reload remap — general marker-edit tracking is out of
  scope, a possible future design item); status brackets keep the old
  index-4 insertion and priority, with narrow-width tests instead of
  shortened labels; sanitizer caps in raw source coordinates before
  mapping; all wide-mode geometry through `dun_term::char_width`
  (`·`/`→`/`¶` are East Asian Ambiguous = two cells in Wide). Measurement
  cadence adapted from the plan: macOS budget build gates every step; the
  binding Debian measurement + smoke runs after step 1 and after step 3
  (C-spine/crossterm batching precedent), not three times.
- [x] Implementation steps per the adapted plan, each its own gated brief
  (scope check, fmt/clippy/tests, mutation-proofs on invariant guards):
  - [x] Step 1 — display-coordinate seam (brief 048, `3b69844`). Gated: 790
    tests, fmt/clippy clean. Two gate-found defects fixed (long-line render
    5→101→4ms via the raw-width fast path; a duplicate untested `scroll_status`
    fork removed). Measured both platforms — macOS 682,044, Debian 743,728,
    margin 304,848. No user-visible change.
  - [x] Step 2 — F13 visible whitespace, full trail (brief 049, `5914467`).
    800 tests; macOS +16 (682,060). Default-off byte-identity mutation-proven.
  - [x] Step 3 — F12 bookmarks, full trail (brief 050, `b9ac165`). One clean
    Codex stop-loss on the gutter-render assumption → Option C. 817 tests;
    macOS 690,292. Strict-circular nav, Move Line remap, and separator-over-
    marker all mutation-proven by Claude.
- [x] Dual-platform size measurement + docs/dev/release-size-audit.md entries;
  release smoke. Debian binding 756,016 (+16,384 over `877b7ad`, margin
  292,560); macOS 690,292; smoke clean (ELF PIE, ldd unchanged,
  DUN_TEST_PANIC=0). Four-platform functional matrix 817/0 (macOS, Debian,
  FreeBSD, Solaris).
- [x] Full-trail docs: README feature paragraphs, docs/dev/feature-triage.md
  restoration record, feature-budget classification for the restored units.

## Completed Stage: v0.1 Release Hardening

- [x] Set the hard runtime budget: `target/release/dun` must be no larger than
  `1,048,576` bytes on both audited macOS and Debian builds.
- [x] Make the checked-in release profile the size-budget profile.
- [x] Classify implemented runtime features as required or optional.
- [x] Define the optional runtime trim order.
- [x] Record the current macOS release binary size.
- [x] Record the current Debian release binary size.
- [x] Run the release smoke checklist.

## Active Baseline

- [x] Create the Rust `1.85` workspace structure.
- [x] Add ignored local reference area for studying Microsoft Edit and Turbo
  Vision.
- [x] Create initial crates: `dun-core`, `dun-term`, `dun-ui`, `dun-config`,
  `dun-cli`.
- [x] Add crate boundary documentation.
- [x] Commit the current workspace/documentation baseline.

## `dun-core`

- [x] Replace placeholder id types with an allocation strategy.
- [x] Define first real text buffer representation.
- [x] Define cursor and selection types.
- [x] Define edit transaction type.
- [x] Implement insert/delete/newline.
- [x] Implement undo/redo.
- [x] Coalesce continuous ordinary character input into undo transactions while
  keeping paste, movement, selection, delete, and replace boundaries separate.
- [x] Coalesce continuous same-direction Backspace/Delete runs into undo
  transactions.
- [x] Add UTF-8-safe word movement, word selection, and word delete commands.
- [x] Keep the horizontal cursor position visible in long editor lines.
- [x] Implement dirty-state tracking.
- [x] Implement split focused window.
- [x] Implement close focused window and tree repair.
- [x] Implement directional focus movement.
- [x] Implement split ratio resize.
- [x] Implement collapse/expand.
- [x] Add unit tests for buffer edits.
- [x] Add unit tests for split-tree transitions.

## `dun-term`

- [x] Define full `TerminalProfile`.
- [x] Detect UTF-8 vs ASCII rendering mode.
- [x] Detect or configure 256-color vs 16-color vs mono.
- [x] Define Microsoft Edit-like default palette.
- [x] Define ASCII border and indicator glyphs.
- [x] Define 16-color fallback theme.
- [x] Add tests for glyph fallback selection.

## `dun-ui`

- [x] Build backend-neutral frame model from config, workspace, and buffers.
- [x] Resolve theme, glyph, keymap, and display sanitizer in `UiShell`.
- [x] Sanitize buffer lines before they enter the UI frame model.
- [x] Select `ratatui` and backend versions compatible with Rust `1.85`.
- [x] Render menu bar.
- [x] Render grouped File/Edit/View/Help dropdown menus.
- [x] Add keyboard navigation for grouped dropdown menus.
- [x] Render status bar.
- [x] Render single editor area with line-number gutter.
- [x] Render Microsoft Edit-style single-line borders for tiled windows.
- [x] Render ASCII fallback borders.
- [x] Render multiple tiled windows from resolved layout rectangles.
- [x] Render focused buffer cursor inside the active window body.
- [x] Render selected text ranges in the active window body.
- [x] Add UI hit testing for optional mouse focus and cursor placement.
- [x] Add submenu hit testing for optional mouse command dispatch.
- [x] Polish tiled rendering for small terminals and narrow panes.
- [x] Keep rendering free of file I/O.
- [x] Tune `msedit` theme colors against local Microsoft Edit screenshots.
- [x] Add Microsoft Edit-like active top-menu color and gray dropdown panel.
- [x] Add menu mnemonics and right-aligned shortcut column rendering.
- [x] Add current-line row highlight and persistent gutter separator.
- [x] Add lightweight modal prompt rendering for Go To Line, Find, Replace,
  and confirmations.
- [x] Add larger Open/Save As file dialog baseline after lightweight modals.
- [x] Add Tab path completion to the Open/Save As file dialog baseline.
- [x] Add mouse hit testing for file dialog entries.
- [x] Add file dialog list scrolling and PageUp/PageDown navigation.
- [x] Add parent-directory and hidden-file polish to file dialogs.
- [x] Add file dialog path-input cursor movement and Home/End editing.
- [x] Add file dialog empty/no-match diagnostics and tighter Open/Save visuals.
- [x] Add file-dialog overlay structure tests for Microsoft Edit-like visual
  fields.
- [x] Refine selection and search highlight geometry for soft-wrapped lines.
- [x] Add visual-row scrolling for soft-wrapped editor panes.

## UI Polish Backlog

Scope: these are non-`rum`, non-manual polish tasks that should be handled with
automated tests only. Manual screenshot comparison and external terminal
inspection stay outside this section.

- [x] Add automated text-snapshot coverage for Microsoft Edit-like menu,
  window, status, and modal chrome.
- [x] Keep long dropdown menus usable on short terminals, including visible
  overflow indicators and correct mouse hit testing.
- [x] Add visible overflow indicators for scrollable modal lists such as
  Open/Save As and Switch Buffer.
- [x] Polish command prompt completion display so candidates are visible in
  the prompt overlay, not only status history.
- [x] Strengthen ASCII/16-color fallback rendering tests for menus, dialogs,
  scrollbars, and viewport markers.
- [x] Tighten small-terminal and narrow-pane rendering assertions beyond
  no-panic smoke tests.
- [x] Keep helper-window and modal text layout covered by automated rendering
  assertions for Help, Config Diagnostics, Status History, Outline, Search
  Results, Command Output, and file/dialog overlays.
- [x] Keep mouse hit testing aligned with rendered menu/dialog/scrollbar
  geometry after every UI polish change.

## Code Hygiene

- [x] Document the safe Rust policy and code organization rules.
- [x] Document the staged oversized-file splitting plan.
- [x] Split `crates/dun-cli/src/main.rs` tests by behavior family before the
  next large CLI feature batch.
- [x] Split `dun-cli` pure status/help/file-dialog/text-width helpers out of
  `crates/dun-cli/src/main.rs`.
- [x] Extract `dun-cli` AppState window, editing/clipboard, and view-state
  method groups.
- [x] Extract `dun-cli` AppState mouse interaction and command-dispatch method
  groups.
- [x] Split `crates/dun-cli/src/main.rs` implementation into app, input,
  dialogs, files, terminal, command-output, and helper-text modules.
- [x] Continue Stage 4 with AppState prompts/dialogs, helper
  panes/search-replace, command output, and file I/O method groups.
- [x] Complete Stage 4 by moving remaining AppState construction, frame/view
  sync, menu state, command-line runner, and status/path display methods out
  of `crates/dun-cli/src/main.rs`.
- [x] Continue the CLI split with remaining `main.rs` responsibilities:
  process entry/runtime loop, CLI argument parsing, startup config loading,
  pure command-line parsing/completion helpers, help text assembly, command
  output text formatting, terminal profile detection, and residual text
  formatters.
- [x] Start Stage 5 by moving terminal raw/alternate-screen lifecycle and
  16-color SGR output rewriting into `crates/dun-cli/src/terminal/`.
- [x] Continue Stage 5 with terminal input dispatch, shell/run-command host
  process boundaries, and file open/save/snapshot/atomic I/O modules.
- [x] Split `crates/dun-ui/src/lib.rs` into model, render, hit-testing, text,
  and test modules.
- [x] Start Stage 7 by moving `dun-ui` unit tests into
  `crates/dun-ui/src/tests/` behavior modules.
- [x] Continue Stage 7 with `dun-ui` pure model type extraction.
- [x] Move `dun-ui` text width, truncation, wrapping, and visible-whitespace
  helpers into `text.rs`.
- [x] Move `dun-ui` workspace/menu/overlay hit-testing methods into `hit.rs`.
- [x] Continue Stage 7 with render function extraction by visual layer.
- [x] Finish the `dun-ui` facade split by moving `UiShell` and frame model
  construction out of `crates/dun-ui/src/lib.rs`.
- [x] Split `crates/dun-config/src/lib.rs` into keys, parser, defaults, and
  validation modules.
- [x] Start Stage 8 by moving `dun-config` unit tests into
  `crates/dun-config/src/tests/` behavior modules.
- [x] Move `dun-config` config model and limits model into `config.rs` and
  `limits.rs`.
- [x] Continue Stage 8 with key/keymap/file-dialog-keymap extraction.
- [x] Continue Stage 8 with parser/default-config/validation extraction.
- [x] Split `crates/dun-core/src/buffer.rs` into buffer storage, cursor,
  selection, edit, undo, search, and tests.
- [x] Start Stage 9 by moving `dun-core` buffer tests into behavior modules,
  moving buffer model/storage into `model.rs`, and moving search/replace-all
  logic into `search.rs`.
- [x] Continue Stage 9 with cursor/selection movement extraction.
- [x] Continue Stage 9 with edit, line-ops, and undo extraction.
- [x] Split `crates/dun-core/src/workspace.rs` and
  `crates/dun-term/src/theme.rs` when they are next touched for substantive
  work.

## Terminal Test Extensions

The tmux-backed real-terminal baseline is complete; see
[PROGRESS.md](./docs/dev/PROGRESS.md) and
[docs/dev/real-terminal-tui-testing.md](./docs/dev/real-terminal-tui-testing.md). Keep
this section focused on post-baseline extensions only.

- [ ] Add normalized-grid assertions for selection attributes and richer
  semantic color output only when a concrete diff case or regression risk needs
  those projections.

## `dun-config`

- [x] Define typed config defaults.
- [x] Define typed keybinding schema.
- [x] Define default keymap.
- [x] Load config files through Rust-owned parsing.
- [x] Apply configured command keybindings in the runtime input path.
- [x] Reload runtime configuration without restarting the editor.
- [x] Show active config diagnostics inside the editor.
- [x] Support multi-stroke key sequence prefix matching.
- [x] Define theme selection config.
- [x] Define terminal override config.
- [x] Define optional mouse enablement config.
- [x] Define opt-in OSC 52 external-copy config with a payload byte limit.
- [x] Define configurable file-dialog modal keybindings.
- [x] Define large-file and display limits.
- [x] Expose built-in defaults through `--dump-config`.
- [x] Add config validation tests.
- [x] Add MacBook-friendly `Ctrl+W` window focus/resize aliases while keeping
  `Alt` compatibility bindings where terminals deliver them.

## `dun-cli`

- [x] Add argument parsing.
- [x] Add terminal setup and restoration guard.
- [x] Create initial untitled workspace.
- [x] Open file path passed on command line.
- [x] Save focused buffer back to its loaded file path.
- [x] Wire config/profile/workspace/UI construction.
- [x] Apply editor commands to the focused buffer/window.
- [x] Add runtime config reload command.
- [x] Add config diagnostics screen.
- [x] Add command-line prompt baseline.
- [x] Add command-line prompt history.
- [x] Add runtime theme selection command.
- [x] Add interactive file dialogs for open/save-as and modal prompts for
  find/replace/go-to-line entry.
- [x] Route printable text input into the focused buffer.
- [x] Track pending multi-stroke key sequences.
- [x] Keep the focused cursor line visible while drawing.
- [x] Implement find result navigation.
- [x] Implement replace current/next match baseline.
- [x] Implement go-to-line prompt.
- [x] Implement read-only help/key reference window.
- [x] Generate help/key reference content from the active keymap.
- [x] Implement status history window.
- [x] Expand status bar detail fields.
- [x] Show focused buffer name, dirty state, and line/column status.
- [x] Confirm before quit/new/open/close would discard dirty buffers.
- [x] Return stable exit codes.
- [x] Report visible success/failure status for tiling window commands.
- [x] Run full command ids from the command-line prompt, including
  `window.*` commands.
- [x] Add optional mouse capture, left-click window focus, and body cursor
  placement.
- [x] Add mouse text selection, menu clicks, and split dragging.
- [x] Document right-click paste and clipboard safety policy.
- [x] Add UTF-8-safe prompt cursor editing for command/find/replace/go-to-line
  prompts.
- [x] Add bracketed paste routing for editor buffers, prompts, and file
  dialogs.
- [x] Add right-click paste status handling without invoking external clipboard
  commands.
- [x] Implement internal Cut/Copy/Paste baseline for active selections without
  using the OS clipboard.
- [x] Add keyboard selection baseline with `Shift+Arrow` and `Shift+Home/End`.
- [x] Add editor PageUp/PageDown movement and viewport synchronization.
- [x] Add `Shift+PageUp/PageDown` page-wise selection.
- [x] Report Undo/Redo status feedback.
- [x] Add scroll range and horizontal offset status fields.
- [x] Add optional mouse wheel scrolling for editor panes.
- [x] Add cached search match highlighting and focused match status fields.
- [x] Add command-line `replace all QUERY TEXT` with one undo transaction.
- [x] Add explicit horizontal viewport scroll commands.
- [x] Add a lightweight vertical scrollbar indicator for long buffers.
- [x] Add incremental Find and Replace query preview in modal prompts.
- [x] Add an interactive Replace confirmation flow with Replace, Skip, All,
  and Cancel actions.
- [x] Add mouse click/drag support for the editor scrollbar when mouse support
  is enabled.
- [x] Add horizontal viewport edge indicators and ratatui visual smoke tests
  for viewport polish.
- [x] Add current-line selection command and keyboard binding.
- [x] Add ignore-case and whole-word search prefixes for Find and Replace.
- [x] Add mouse selection edge scrolling.
- [x] Add file-dialog overwrite confirmation, inline error retention, and
  session recent-directory reuse.
- [x] Add stable ratatui text snapshot coverage for baseline UI layout.
- [x] Add buffer switcher overlay for already-open buffers.
- [x] Detect external file changes and reject unsafe Save overwrites.
- [x] Add explicit Reload from disk for focused file buffers.
- [x] Add line commands for copy/delete/move/indent/outdent/trim.
- [x] Add word-wrap, visible-whitespace, and bookmark baseline commands.
- [x] Add Turbo Pascal-style shell escape that suspends and resumes the TUI.
- [x] Add Run Command prompt with bounded read-only output buffer.
- [x] Add Run Command history/output polish and read-only output pane reuse.
- [x] Add PTY smoke coverage for shell escape suspend/resume behavior.
- [x] Add opt-in OSC 52 external copy for active selections while preserving
  the internal clipboard fallback.
- [x] Add Command Output clear/copy/stderr/save commands.
- [x] Add Command Output summary/stdout/stderr navigation and output find.
- [x] Add Command Output Save dialog integration.
- [x] Add Command Output status/truncated quick jumps and View-menu coverage.
- [x] Strengthen Command Output Save dialog overwrite/error coverage.
- [x] Add Command Output index, body-line jumps, and next/previous search
  repeat.
- [x] Polish Config Diagnostics summaries for source, clipboard, limits, and
  keymap coverage.
- [x] Improve Config Diagnostics grouping with top-level Summary and Paths.
- [x] Add Config Diagnostics section jump commands.
- [x] Make soft-wrap PageUp/PageDown movement and selection use visual rows.
- [x] Strengthen soft-wrap paging tests for wide characters, tabs, and control
  bytes.
- [x] Add document start/end navigation for read-only and editable panes.
- [x] Keep menu mnemonics unique within each menu.
- [x] Add read-only outline/section list and section jump commands.
- [x] Add read-only search result list and numbered result jumps.
- [x] Add Command Output section navigation and stdout/stderr-only views.
- [x] Add command-line Tab completion for built-in command families.
- [x] Expand Outline detection for common Markdown, INI/TOML, Rust, and shell
  section lines.
- [x] Add `n`/`p` and `Enter` row navigation for Outline and Search Results.
- [x] Add command-line completion candidate cycling and path completion.
- [x] Make Command Output only-views searchable/saveable as the current output.
- [x] Restore focus from closable read-only helper panes to their source where
  a source exists.

## File and Display Safety

- [x] Implement UTF-8-first file loading behavior.
- [x] Open invalid UTF-8 files as read-only escaped fallback buffers.
- [x] Define invalid-byte fallback behavior.
- [x] Track file-text encoding and expose escaped-byte fallback state.
- [x] Reject unstable/corrupt reads when file metadata changes during Open.
- [x] Prevent save from silently corrupting lossy/fallback buffers.
- [x] Save files through same-directory temporary files and atomic rename.
- [x] Refuse normal Save when the loaded file's metadata snapshot no longer
  matches the current path.
- [x] Add readable path diagnostics for common open/save failures.
- [x] Define large-file soft limit behavior.
- [x] Add large-file performance baselines.
- [x] Add lightweight release binary size audit for macOS and Debian builds.
- [x] Add lightweight runtime memory and startup baseline for macOS and Debian
  builds.
- [x] Add release size repeat checklist.
- [x] Add dependency/feature audit and minimal default-build feature policy.
- [x] Implement display sanitizer for C0/C1 controls.
- [x] Render `ESC`, OSC, BEL, NUL, DEL, CR, and backspace visibly.
- [x] Add tests for terminal-injection payloads.
- [x] Add control-byte rendering audit suite for buffer text and UI chrome.
- [x] Cap display work for very long lines.

## Terminal Compatibility Testing

- [x] Add PTY smoke tests for common SSH-style terminal profiles.
- [x] Expand PTY tests into a broad terminal compatibility harness.
- [x] Document manual SSH terminal checks and current-environment verification.
- [x] Define the external SSH and low-capability terminal release matrix.
- [x] Fix strict VT100/16-color output so low-capability profiles do not emit
  256-color-style `38;5;n` or `48;5;n` SGR sequences.
- [x] Add automated event-level coverage for common modified terminal keys.
- [x] Run the external SSH and low-capability Debian VM matrix for `d2c832f`.
- [x] Add static Microsoft Edit reference tests for source-visible menu,
  status, color, and terminal setup markers.

## Deferred

- [x] Restoration review after the plugin protocol client lands — review
  decided 2026-07-23 (user): restore F12/F13 (bookmarks, visible
  whitespace); F46 stays removed (the LogFilter-plugin overlap rationale
  still holds); F20 (Outline) still returns as a plugin role. Execution
  tracked in "Current Stage: Restoration Review — F12/F13".

- [x] OSC 52 paste/query support — **DONE 2026-07-27** (`42774ec`), OSC 52
  read only; platform-specific clipboard commands stay rejected (break the
  no-external-command policy, fail over SSH). See "Completed Stage: OSC 52
  Clipboard Read".
- [x] Crash recovery and orphaned atomic-save temp-file cleanup.
- [ ] `rum` configuration evaluation.
- [ ] `dun-plugin-rum`.
- [ ] Syntax highlighting plugins backed by `rum`.
- [ ] Full log viewing and filtering product after the plugin protocol client
  is working; untrusted third-party defaults wait for a pure `rum` host.
- [ ] Memory watchdog design for long-running plugin evaluation.
- [x] Broad terminal compatibility test harness.
