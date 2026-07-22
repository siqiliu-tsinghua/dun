# Release Size Audit

This document records lightweight release binary size checks for `dun`.
Results are machine-local baselines, not release claims.

## Hard Budget

The v0.1 release budget is strict:

```text
target/<host-triple>/release/dun <= 1,048,576 bytes
```

The gate must pass on both audited macOS x86_64 and Debian x86_64 builds.
Since 2026-07-10 the budget build is the build-std contract:

```text
scripts/release-build.sh
```

If either platform is above the limit, consult
[feature-triage.md](./feature-triage.md) and
[feature-budget.md](./feature-budget.md). Do not add runtime features while
the budget is failing.

## Build Profiles

The budget build is `scripts/release-build.sh` (build-std contract,
2026-07-10). Sections below dated before 2026-07-10 used the plain stable
build `cargo build --release --locked -p dun-cli`; their numbers remain
valid history for that recipe.

The checked-in `[profile.release]` (shared by both recipes):

```text
opt-level = "z"
lto = "fat"
codegen-units = 1
strip = "symbols"
panic = "abort"
```

This trades build time and possibly some runtime performance for a smaller
executable. Any future profile change must repeat the size and runtime-resource
audits.

## 2026-07-08 Baseline

Source commit: `4d89d07`

macOS host:

```text
rustc 1.85.0 (4d91de4e4 2025-02-17)
cargo 1.85.0 (d73d2caf9 2024-12-31)
Darwin fftmac.local 25.5.0 x86_64
```

Debian VM:

```text
rustc 1.85.0 (4d91de4e4 2025-02-17) (built from a source tarball)
cargo 1.85.0 (d73d2caf9 2024-12-31)
Linux debvbox 6.12.95+deb13-amd64 x86_64
```

| Platform | Profile | Size | File type | Dynamic dependencies |
| --- | ---: | ---: | --- | --- |
| macOS x86_64 | default release | 1,627,136 bytes | Mach-O 64-bit executable | `/usr/lib/libiconv.2.dylib`, `/usr/lib/libSystem.B.dylib` |
| macOS x86_64 | size-oriented release | 859,544 bytes | Mach-O 64-bit executable | `/usr/lib/libiconv.2.dylib`, `/usr/lib/libSystem.B.dylib` |
| Debian x86_64 | default release | 1,881,392 bytes | ELF 64-bit PIE, dynamically linked, not stripped | `libgcc_s.so.1`, `libc.so.6`, `/lib64/ld-linux-x86-64.so.2` |
| Debian x86_64 | size-oriented release | 1,034,840 bytes | ELF 64-bit PIE, dynamically linked, stripped | `libgcc_s.so.1`, `libc.so.6`, `/lib64/ld-linux-x86-64.so.2` |

Both default and size-oriented binaries printed `dun 0.1.0` with `--version`
on their host platform.

Observed reduction from the size-oriented profile:

- macOS x86_64: about 47 percent smaller than default release.
- Debian x86_64: about 45 percent smaller than default release.

## 2026-07-08 Architecture Conclusion

The size-oriented `dun` binary is currently about 0.8-1.0 MiB on the audited
dynamic-link targets. That is small for the intended SSH/server editor use
case. By contrast, the current planning assumption for `rum` is an
approximately 6 MiB runtime, which is larger than the whole size-oriented
editor.

The resulting architecture rule is:

- keep the default editor build independent of `rum`;
- implement simple and common editor features in Rust core code;
- use future `rum` integration only for workflows that justify the runtime
  cost, such as complex custom log filters, structured extraction, advanced
  text transformations, or semantic plugin logic;
- make any future `rum` adapter optional or late-loaded rather than required
  for startup, file editing, basic search, window management, or configuration
  diagnostics.

## Repeat Checklist

Use this checklist when refreshing the release-size baseline:

1. Record the source commit, local modifications, toolchain, host OS, and CPU
   architecture.
2. Build the release binary with `scripts/release-build.sh` (the build-std
   budget contract; prints the byte size).
3. Record executable type with `file` and dynamic dependencies with
   `otool -L` on macOS or `ldd` on Linux (binary under
   `target/<host-triple>/release/dun`).
4. Verify the size is no larger than `1,048,576` bytes.
5. Run the binary with `--version` to verify the measured executable starts.
6. Run `--dump-config` to verify the measured executable can emit defaults.
7. Run the runtime resource audit in
   [runtime-resource-audit.md](./runtime-resource-audit.md) when changing
   profile settings, dependency features, terminal backend behavior, or file
   loading paths.
8. Update the dependency audit in
   [dependency-audit.md](./dependency-audit.md) when adding or removing
   runtime dependencies.

## Notes

- The current workspace has a checked-in size-budget `[profile.release]`.
- The audit does not use UPX, static linking, musl, or platform-specific
  packaging.
- The Debian result was built from a clean git archive copied to
  `/tmp/dun-size-audit` on the VM, using the Debian system `rustc`/`cargo`.
- Future audits should record the commit, toolchain, host OS, exact build
  command, byte size, `file` output, dependency listing, and `--version`
  smoke result.

## 2026-07-09 Budget Gate Refresh

Source baseline: committed code at `60d45a2`.

Build command on both platforms:

```text
cargo build --release --locked -p dun-cli
```

macOS host:

```text
rustc 1.85.0 (4d91de4e4 2025-02-17)
cargo 1.85.0 (d73d2caf9 2024-12-31)
Darwin fftmac.local 25.5.0 x86_64
```

Debian VM:

```text
rustc 1.85.0 (4d91de4e4 2025-02-17) (built from a source tarball)
cargo 1.85.0 (d73d2caf9 2024-12-31)
Linux debvbox 6.12.95+deb13-amd64 x86_64
```

| Platform | Size | Budget margin | File type | Dynamic dependencies |
| --- | ---: | ---: | --- | --- |
| macOS x86_64 | 863,664 bytes | 184,912 bytes under | Mach-O 64-bit executable | `/usr/lib/libiconv.2.dylib`, `/usr/lib/libSystem.B.dylib` |
| Debian x86_64 | 1,038,936 bytes | 9,640 bytes under | ELF 64-bit PIE, dynamically linked, stripped | `libgcc_s.so.1`, `libc.so.6`, `/lib64/ld-linux-x86-64.so.2` |

Both binaries printed `dun 0.1.0` with `--version` and emitted the default
configuration with `--dump-config`.

Result: both audited builds pass the 1 MiB gate, but Debian has less than
10 KiB of margin. Treat any new runtime code or dependency as budget-sensitive
until a fresh size audit proves otherwise.

## 2026-07-10 Plugin-Client Spike Measurement

Source: branch `spike/plugin-client-size` at `c7f042c` (baseline `0404b18`
plus the spike prototype; `0404b18` is runtime-identical to `60d45a2`).

The spike adds a `dun-plugin` crate — framed stdio, hand-rolled JSON,
envelope/role/policy model, span validation, timeout/cancel/crash handling
for one role — wired into `dun-cli` behind a hidden `--plugin-spike` flag so
fat LTO keeps it live. Build command on both platforms:

```text
cargo build --release --locked -p dun-cli
```

| Platform | Baseline | With spike | Delta | Budget check |
| --- | ---: | ---: | ---: | --- |
| macOS x86_64 | 863,664 | 925,696 | +62,032 | 122,880 under |
| Debian x86_64 | 1,038,936 | 1,116,760 | +77,824 | **68,184 over** |

Both spike binaries passed `--version`; the macOS build also passed an
end-to-end `--plugin-spike` round trip against the fixture host example.

Conclusions:

- The required plugin client costs about 76 KiB on the binding Debian
  platform — a floor, excluding config integration and additional roles.
- Landing it requires trimming optional features first; the derived target
  is ~147–187 KiB freed on Debian (client cost + future-feature reserve −
  current margin). See [feature-triage.md](./feature-triage.md).
- Debian deltas ran ~1.25x macOS for identical code; usable for quick local
  estimates, never for gate claims.

## 2026-07-10 Slimming Batches 1–3

C/D removals from the feature triage, each built and measured on both
platforms with the locked release profile; Debian numbers are binding.
Deltas are per whole batch (fat LTO makes per-item deltas non-additive).

| Batch | Commit | Removed | macOS bytes | Debian bytes | Debian delta |
| ---: | --- | --- | ---: | ---: | ---: |
| — | `60d45a2` | (baseline) | 863,664 | 1,038,936 | — |
| 1 | `ce68f20` | F46 advanced Command Output family | 843,136 | 1,014,360 | −24,576 |
| 2 | `13d3ef7` | F20 Outline pane (→ plugin role) | 834,888 | 1,002,072 | −12,288 |
| 3 | `53fe7f8` | F12 bookmarks + F13 visible whitespace | 826,656 | 989,784 | −12,288 |

Cumulative: −49,152 bytes (48.0 KiB) on Debian; margin is 58,792 bytes.

The 2026-07-10 defect-fix commit (`06ed915`: panic terminal-restore hook,
run-command timeout, dirty-check caching) measured 826,768 bytes on macOS
(+112) and 997,976 bytes on Debian (+8,192 after page alignment); margin
50,600 bytes.

## 2026-07-10 Build Contract: build-std (Spike A)

Following the msedit size study (docs/msedit-reference.md), three build-std
variants were measured on the Debian VM at `1578aff` (RUSTC_BOOTSTRAP=1 on
the stable 1.85 toolchain, rust-src from the Debian package):

| Variant | Debian bytes | vs 997,976 | Panic behavior (verified) |
| --- | ---: | ---: | --- |
| stable baseline | 997,976 | — | hook + message + backtrace machinery |
| build-std, default std features | 780,728 | −217,248 | unchanged, std rebuilt with release profile |
| build-std, `-Zbuild-std-features=` | 620,928 | −377,048 | hook runs, message with location; no backtrace symbolization |
| + `panic_immediate_abort` | 534,912 | −463,064 | hook does NOT run, no output |

Panic behavior was verified with a standalone hook-and-panic experiment per
variant. The user ratified the empty-features variant as the release build
contract (2026-07-10); `panic_immediate_abort` was rejected because it
disables the terminal-restore panic hook (an A-level invariant).

Adopted recipe: `scripts/release-build.sh` (prints binary path and size).
Recorded at `b2510a3`:

| Platform | Size | Margin | Dynamic dependencies |
| --- | ---: | ---: | --- |
| macOS x86_64 | 575,460 | 473,116 | unchanged |
| Debian x86_64 | 620,928 | 427,648 | `libgcc_s.so.1`, `libc.so.6`, `ld-linux` (unchanged) |

Both binaries passed `--version` and `--dump-config` smoke. Dev builds and
`cargo test` remain on the plain stable path; rust-src is a one-time
prerequisite per machine (rustup component / Debian `rust-src` package).

## 2026-07-11 Plugin Client Wiring

`eb38c7b` makes `dun-plugin` a real `dun-cli` dependency (worker-thread
host lifecycle, per-buffer highlight cache, configured policies):

| Platform | Before | After | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 575,460 | 653,852 | +78,392 | 394,724 |
| Debian x86_64 | 620,928 | 702,848 | +81,920 | 345,728 |

Matches the spike's ~76 KiB floor plus wiring. `--version`/`--dump-config`
smoke and the full workspace suite passed on both platforms.

The render slice (`29c1d28`: syntax palette slots, span conversion, frame
mapping, painting) completed the SyntaxHighlight role end to end for
+4,112 macOS (657,964) / +4,096 Debian (706,944; margin 341,632). Full
protocol-client cost from the pre-client baseline: 86,016 bytes Debian.
End-to-end verified in a real tmux session against the fixture host.
Each batch passed fmt/clippy, the workspace test suite (13 suites; the
tmux 3.7 grid-harness failure recorded in TODO.md predates the slimming
stage and reproduces at `b03192d`), and `--version`/`--dump-config` smoke
on both platforms.

## 2026-07-11 Renderer Slice 2 (Surface diff emitter)

`74f6576` (brief-005) adds the `surface_emit` module to `dun-ui` as
`#[allow(dead_code)]` pending integration. Zero size delta on both
platforms, confirming the unreferenced module is fully stripped:

| Platform | Before | After | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 657,964 | 657,964 | 0 | 390,612 |
| Debian x86_64 | 706,944 | 706,944 | 0 | 341,632 |

Debian measured on a clean `git archive` of `74f6576`; `--version` and
`--dump-config` smoke passed. The integration slice (replacing the
ratatui `Terminal` in `dun-cli`) is where the real size movement — the
removal of the ratatui dependency — will show up.

## 2026-07-11 dun-cli Surface Cutover (slice 4a)

`9da0834` (brief-010) switches the dun-cli event loop from the ratatui
`Terminal` to `SurfaceBackend` (SurfaceRenderer + emit_diff). The ratatui
dependency still stands (dun-ui uses it), but the binary no longer reaches
the ratatui render path, so fat LTO strips that machinery — a net size win
despite adding the Surface renderer to the binary:

| Platform | Before | After | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 657,964 | 592,172 | -65,792 | 456,404 |
| Debian x86_64 | 706,944 | 637,312 | -69,632 | 411,264 |

Debian measured on a clean `git archive` of `9da0834`; `--version` and
`--dump-config` smoke passed. Verified on a real terminal via the
tmux/PTY/terminal-grid suites plus an interactive tmux smoke (initial
render, cursor tracking while typing, resize full-repaint, clean quit).
The ratatui-retirement slice (removing the dun-ui ratatui path and the
dependency) is expected to yield little further binary size — most of the
now-unreachable code is already stripped — but drops the dependency and
its lock packages.

## 2026-07-11 ratatui Retirement (slice 4b)

`858e876` deletes the dead dun-ui ratatui render path, migrates the Surface
path off `ratatui::layout::Rect` to `dun_core::Rect`, and drops the `ratatui`
dependency from both crates, the workspace table, and the lockfile. Binary
size barely moves (LTO already stripped the unreachable render path in 4a),
but the dependency and its transitive tree are gone:

| Platform | Before (4a) | After (4b) | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 592,172 | 588,060 | -4,112 | 460,516 |
| Debian x86_64 | 637,312 | 633,216 | -4,096 | 415,360 |

Lockfile packages: 76 → 42 (**-34**), removing ratatui and its transitive
deps (cassowary, compact_str, castaway, darling, strum/strum_macros,
itertools, lru, unicode-segmentation, unicode-truncate, and the
syn/proc-macro2/quote proc-macro stack). Debian measured on a clean
`git archive` of `858e876`; `--version`/`--dump-config` smoke passed. The
full renderer-replacement line (Phase 12) is complete: `dun` renders through
the in-house Surface backend with no ratatui.

## 2026-07-11 Plugin load/unload commands (brief-011)

`1d42229` adds the WorkerMessage control channel and `plugin load`/`unload`
prompt commands (on-demand host lifecycle for memory). Small runtime add:

| Platform | Before | After | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 588,060 | 592,180 | +4,120 | 456,396 |
| Debian x86_64 | 633,216 | 637,312 | +4,096 | 411,264 |

Debian measured on a clean `git archive` of `1d42229`; `--version`/
`--dump-config` smoke passed.

## 2026-07-11 Solarized themes + dun default (e61774a)

Two new 256-color themes (solarized-dark/light) and the default theme switch
to dun. Debian is unchanged (LTO folds the const theme builders); macOS moves
slightly:

| Platform | Before | After | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 592,180 | 596,284 | +4,104 | 452,292 |
| Debian x86_64 | 637,312 | 637,312 | 0 | 411,264 |

Debian measured on a clean `git archive` of `e61774a`; `--version` and
`--dump-config` (now `theme = dun`) smoke passed.

## 2026-07-13 Post-client re-audit + i18n slice 1 (fd31719)

One recorded span from `e61774a` (the last measured commit) to `fd31719`,
covering briefs 012-021 (palette warning role, config color overrides,
plugin status indicator, contract tests, panic restore tests, window.only,
screen snapshots, menu behaviour matrix), the dun/Solarized theme redesign,
sanitizer bidi/zero-width hardening, save/menu/dialog fixes, and i18n
slice 1 (mechanism + menus + zh-CN). This closes the plugin-protocol-client
stage's release gates and is the re-audit the plan required after the
client landed.

| Platform | e61774a | fd31719 | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 596,284 | 625,060 | +28,776 | 423,516 |
| Debian x86_64 | 637,312 | 670,080 | +32,768 | 378,496 |

Of the span, i18n slice 1 alone measured +12,344 on macOS
(612,716 -> 625,060); the earlier commits were not individually measured
(the unmeasured stretch this re-audit pays off). Debian measured on a clean
`vm-test/vm-sync` archive of `fd31719`; `--version`/`--dump-config` smoke
passed, `ldd` unchanged (libgcc/libm/libc only), and the release binary
rendered the zh-CN menus correctly under tmux on the VM with
`LANG=zh_CN.UTF-8`. Local smoke: tmux_grid 5 passed, msedit_diff 1 passed,
release `test-panic-hook` pty_smoke 10 passed, `strings | grep -c
DUN_TEST_PANIC` printed 0 on the budget binary.

## 2026-07-13 Dropdown shift-left clamp (a9ff7c8)

Menu dropdowns near the right edge shift left onto the screen instead of
being clamped away (translated labels widen the bar, so the rightmost menu
fell off narrow terminals sooner). Zero size cost on both platforms:

| Platform | Before | After | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 625,060 | 625,060 | 0 | 423,516 |
| Debian x86_64 | 670,080 | 670,080 | 0 | 378,496 |

Debian measured on a clean `vm-test/vm-sync` archive of `a9ff7c8`;
`--version` smoke passed.

## 2026-07-13 i18n slice 2: help window (28d0c97)

Help window text extraction (~125 catalog keys, display-width column
padding, '_' allowed in catalog keys) plus the shipped zh-CN help
translations. The key-enumeration helper is `#[cfg(test)]` and costs the
binary nothing; the delta is the catalog lookups and table restructuring.

| Platform | Before | After | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 625,060 | 629,164 | +4,104 | 419,412 |
| Debian x86_64 | 670,080 | 678,272 | +8,192 | 370,304 |

Debian measured on a clean `vm-test/vm-sync` archive of `28d0c97`;
`--version` smoke passed; VM scratch directory removed after recording.

## 2026-07-13 i18n slice 3: dialog chrome (89cd9e4)

Dialog/overlay chrome extraction (48 keys in ui_text.rs, the `{}` template
mechanism with mismatch fallback, zh-CN translations) plus the help-window
slice's follow-on. Cost per slice is tracking steady at ~8 KiB Debian per
extraction batch:

| Platform | Before | After | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 629,164 | 633,284 | +4,120 | 415,292 |
| Debian x86_64 | 678,272 | 686,464 | +8,192 | 362,112 |

Debian measured on a clean `vm-test/vm-sync` archive of `89cd9e4`;
`--version` smoke passed; VM scratch directory removed after recording.

## 2026-07-15 i18n slice 4 tail + Debian debt settlement (744c843 ≡ 1d03433)

The VM was unavailable for the second half of the i18n stage, leaving 19
commits (`89cd9e4..1d03433`) unmeasured on the binding platform. This audit
pays that debt off in one span. HEAD is `744c843`, a docs-only tip whose
release binary is byte-identical to the i18n stage-close commit `1d03433`,
so measuring HEAD settles the whole debt.

The span covers i18n slices 4a–4c (status-message extraction, the stateless
helper builders, the typed/stored text, and the `ui_text/status.rs` domain
split), the locale script-fallback + every-translation validator, the ten
shipped translations (briefs 027–029), and the dropdown shift-left clamp.

| Platform | 89cd9e4 | 744c843 | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 633,284 | 649,900 | +16,616 | 398,676 |
| Debian x86_64 | 686,464 | 715,136 | +28,672 | 333,440 |

Debian was measured this session on a clean `vm-test/vm-sync` archive of
`744c843` (`scripts/release-build.sh`, build-std, rust-src from the Debian
`rust-src` package; build finished in 46s). The macOS figure is the i18n
stage-close value tracked at `1d03433` (macOS was never in debt — only the
VM was unavailable). The +28,672 Debian growth is the i18n slice-4
mechanism tail; the translations themselves cost the binary nothing
(external `i18n/*.conf` files). Debian/macOS ratio for this span is ~1.73x,
consistent with non-additive fat-LTO deltas.

Smoke on the measured Debian binary: `file` reports ELF 64-bit PIE,
stripped; `ldd` unchanged (`libgcc_s.so.1`, `libm.so.6`, `libc.so.6`,
`ld-linux`); `--version` printed `dun 0.1.0`; `--dump-config` emitted the
181-line default config (`theme = dun`). VM scratch directory removed after
recording. The Distinctive Plugins stage now starts on a measured baseline
with 333,440 bytes of margin on the binding platform.

## 2026-07-16 Capability model spine (c0f610e)

First binding measurement of the capability-model stage. The span
`744c843..c0f610e` covers the role→capability redesign (docs), the `Capability`
vocabulary + `GrantedCapabilities` primitives (slice A), the `PluginWindows`
ownership registry (brief-030), and the live grant + enforcement + the
config↔handshake trust cross-check (`c0f610e`). The primitives and the registry
were dead-stripped until `c0f610e` first linked the grant into the client.

| Platform | 744c843 | c0f610e | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 649,900 | 654,004 | +4,104 | 394,572 |
| Debian x86_64 | 715,136 | 715,136 | +0 | 333,440 |

Debian measured on a clean `vm-test/vm-sync` archive of `c0f610e`
(`scripts/release-build.sh`, build-std, Debian `rust-src`; build finished in
49s; archive verified to contain `capability.rs` and the grant code before
trusting the byte-identical result). The whole capability machinery so far
costs the binding binary **nothing net**: the +4,104 macOS growth from linking
the grant did not translate to Debian, where it landed inside existing slack —
the non-additive fat-LTO behavior the budget notes call out (measure per batch,
never assume additivity or that a macOS delta implies a Debian one).

Smoke on the measured Debian binary: `file` reports ELF 64-bit PIE, stripped;
`ldd` unchanged (`libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, `ld-linux`);
`--version` printed `dun 0.1.0`; `--dump-config` emitted the default config. VM
scratch directory removed after recording. Margin on the binding platform is
unchanged at 333,440 bytes.

## 2026-07-18 C spine: menu contribution (bbc3fa7)

Binding measurement for the whole C-spine ("C — menu") integration milestone,
paying off the four commits owed on the VM since `c0f610e`: the handshake-carried
menu grant (`d2fe8df`), the host-layer generalization `PluginHost`/`PluginHosts`
(`01d184d`), the dun-core typed variants `WindowKind::PluginSurface` +
`EditorCommand::PluginMenuAction` (chunk 2, `ee876d2`), and the menu inject +
dispatch + plugin surface window path (chunk 3, `5a4fc06` + `bbc3fa7`).

| Platform | c0f610e | bbc3fa7 | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 654,004 | 670,492 | +16,488 | 378,084 |
| Debian x86_64 | 715,136 | 735,616 | +20,480 | 312,960 |

Debian measured on a clean `vm-test/vm-sync` archive of `bbc3fa7`
(`scripts/release-build.sh`, build-std, Debian `rust-src`; build finished in 50s;
archive verified to contain `plugin_surface.rs`, the `PluginMenuAction` variant,
and the new `status.plugin.window-limit` translation before trusting the result).
Toolchain `rustc 1.85.0 (4d91de4e4 2025-02-17)` (built from a source tarball),
`cargo 1.85.0`, `Linux debvbox 6.12.95+deb13-amd64`. The whole C spine costs the
binding binary +20,480 bytes; the ten `status.plugin.window-limit` translations
are external `i18n/*.conf` files and cost the binary nothing. Debian/macOS ratio
for this span is ~1.24x. Both platforms stay well under budget.

Smoke on the measured Debian binary: `file` reports ELF 64-bit PIE, stripped;
`ldd` unchanged (`libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, `ld-linux`);
`--version` printed `dun 0.1.0`; `--dump-config` emitted the 181-line default
config; `strings | grep -c DUN_TEST_PANIC` and `rust_begin_short_backtrace` both
0 (build-std still strips the panic-backtrace machinery). macOS gate this
session: `tmux_grid` (5), `msedit_diff` (1), release `test-panic-hook`
`pty_smoke` (10) all pass; macOS `strings` panic-trigger check 0; `--version` /
`--dump-config` clean. VM scratch directory removed after recording. Margin on
the binding platform is 312,960 bytes; the C (menu) stage is complete.

## 2026-07-18 D: keybinding capability (b7111ef)

Binding measurement for the `keybinding` capability slice, span
`bbc3fa7..b7111ef`: the `PluginMenuAction` -> `PluginAction` rename
(`1aa6bf8`), the `PluginKeybinding` contribution model + handshake parse
(`1273794`), and the dun-cli install + collision check + event-loop
pending-prefix integration (`b7111ef`).

| Platform | bbc3fa7 | b7111ef | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 670,492 | 674,596 | +4,104 | 373,980 |
| Debian x86_64 | 735,616 | 739,712 | +4,096 | 308,864 |

Debian measured on a clean `vm-test/vm-sync` archive of `b7111ef`
(`scripts/release-build.sh`, build-std, Debian `rust-src`; build finished in
50s; archive verified to contain `keybinding.rs`, `resolved_keybindings`, and
the `plugin_keymap` field before trusting the result). Toolchain `rustc 1.85.0`,
`cargo 1.85.0`, `Linux debvbox 6.12.95+deb13-amd64`. The whole keybinding slice
costs the binding binary +4,096 bytes; Debian/macOS ratio ~1.0x for this span.

Smoke on the measured Debian binary: `file` reports ELF 64-bit PIE, stripped;
`ldd` unchanged (`libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, `ld-linux`);
`--version` printed `dun 0.1.0`; `--dump-config` emitted the 181-line default
config; `strings | grep -c DUN_TEST_PANIC` and `rust_begin_short_backtrace`
both 0. macOS gate this session: `tmux_grid` (5), `msedit_diff` (1), release
`test-panic-hook` `pty_smoke` (10) all pass; macOS `strings` panic-trigger
check 0. VM scratch directory removed after recording. Margin on the binding
platform is 308,864 bytes; the D (keybinding) stage is complete, and with it
the four capability slices A–D of the Distinctive Plugins stage.

## 2026-07-19 surface-write capability (aa8b852)

Binding measurement for the `surface-write` capability slice (sw-1 `e319d0e`
dun-plugin request/validate path + sw-2 `22b2dd4` dun-cli action→request→render
wiring; `aa8b852` adds only Solaris docs, byte-identical runtime).

| Platform | b7111ef | aa8b852 | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 674,596 | 678,708 | +4,112 | 369,868 |
| Debian x86_64 | 739,712 | 743,808 | +4,096 | 304,768 |

Debian measured on a clean `vm-test/vm-sync` archive of `aa8b852`
(`scripts/release-build.sh`, build-std, Debian `rust-src`). Toolchain
`rustc 1.85.0`, `cargo 1.85.0`, `Linux debvbox 6.12.95+deb13-amd64`. The
surface-write slice costs the binding binary +4,096 bytes; Debian/macOS ratio
~1.0x. Both platforms stay well under budget.

Smoke on the measured Debian binary: ELF 64-bit PIE stripped; `ldd` unchanged
(`libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, `ld-linux`); `--version` printed
`dun 0.1.0`; `--dump-config` 181 lines; `strings` panic-trigger checks both 0.
macOS gate: `tmux_grid` (5), `msedit_diff` (1), release `test-panic-hook`
`pty_smoke` (10) all pass, `strings` 0.

Cross-platform functional runs this session (full `cargo test --workspace`,
reference sizes not budget gates): **macOS 693/0, FreeBSD 693/0, Solaris 689/4**
(the 4 Solaris failures are all `tmux_grid`, root-caused to the platform's
ambiguous-width `wcwidth` policy, not a `dun` defect — see docs/solaris-vm.md).
VM scratch removed after recording. Margin on the binding platform is 304,768
bytes.

## 2026-07-19 stream-read capability (e438a13)

Binding measurement for the `stream-read` capability slice (sr-1 `72d2d9e`
dun-plugin request/validate path + sr-2 `e438a13` dun-cli command-output feed →
verdict → surface wiring).

| Platform | aa8b852 | e438a13 | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 678,708 | 682,820 | +4,112 | 365,756 |
| Debian x86_64 | 743,808 | 747,904 | +4,096 | 300,672 |

Debian measured on a clean `vm-test/vm-sync` archive of `e438a13`
(`scripts/release-build.sh`, build-std, Debian `rust-src`). Toolchain
`rustc 1.85.0`, `cargo 1.85.0`, `Linux debvbox 6.12.95+deb13-amd64`. The
stream-read slice costs the binding binary +4,096 bytes; Debian/macOS ratio
~1.0x. Both platforms stay well under budget.

Smoke on the measured Debian binary: ELF 64-bit PIE stripped; `ldd` unchanged
(`libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, `ld-linux`); `--version` printed
`dun 0.1.0`; `--dump-config` 181 lines; `strings` panic-trigger checks both 0.
macOS gate (on an idle machine — the real-terminal `tmux_grid`/`msedit_diff`
tests flake under concurrent VM load, verified by re-running once idle):
`tmux_grid` (5), `msedit_diff` (1), release `test-panic-hook` `pty_smoke` (10)
all pass, `strings` 0.

Cross-platform functional runs this session (full `cargo test --workspace`):
**macOS 700/0, FreeBSD 700/0, Solaris 696/4** (the 4 Solaris failures are the
root-caused ambiguous-width `tmux_grid` quirk, not a defect). VM scratch removed
after recording. Margin on the binding platform is 300,672 bytes.

## 2026-07-19 scratch-input + execute capability (d9c380a)

Binding measurement for the `scratch-input`/`execute` capability slice (si-1
`32f0b52` dun-plugin execute path + si-2 `d9c380a` action-kind dispatch +
editable scratch window across dun-core/plugin/cli).

| Platform | e438a13 | d9c380a | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 682,820 | 691,028 | +8,208 | 357,548 |
| Debian x86_64 | 747,904 | 756,096 | +8,192 | 292,480 |

Debian measured on a clean `vm-test/vm-sync` archive of `d9c380a`
(`scripts/release-build.sh`, build-std, Debian `rust-src`). Toolchain
`rustc 1.85.0`, `cargo 1.85.0`, `Linux debvbox 6.12.95+deb13-amd64`. This slice
is the largest capability delta so far (+8,192 vs the ~4 KiB of surface-write
and stream-read): it adds the `PluginActionKind` machinery across four crates,
the editable scratch window path, and the execute worker path. Both platforms
stay well under budget.

Smoke on the measured Debian binary: ELF 64-bit PIE stripped; `ldd` unchanged
(`libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, `ld-linux`); `--version` printed
`dun 0.1.0`; `--dump-config` 181 lines; `strings` panic-trigger checks both 0.
macOS gate (idle machine): `tmux_grid` (5), `msedit_diff` (1), release
`test-panic-hook` `pty_smoke` (10) all pass, `strings` 0.

Cross-platform functional runs (full `cargo test --workspace`): **macOS 706/0,
FreeBSD 706/0, Solaris 702/4** (the 4 Solaris failures are the root-caused
ambiguous-width `tmux_grid` quirk, not a defect). With this, all v0 capability
data channels (`overlay-write`, `surface-write`, `stream-read`,
`scratch-input`/`execute`) are built and measured. VM scratch removed after
recording. Margin on the binding platform is 292,480 bytes.

## 2026-07-19 stream-read chunking fix (4a841e2)

Binding measurement for the stream-read chunking fix (the first API-review fix
from the log-filter acceptance: large command output is now fed as bounded
chunks instead of one oversized chunk the client rejects).

| Platform | 988902e | 4a841e2 | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 691,028 | 691,036 | +8 | 357,540 |
| Debian x86_64 | 756,096 | 756,096 | 0 | 292,480 |

Debian measured on a clean `vm-test/vm-sync` archive of `4a841e2`
(`scripts/release-build.sh`, build-std, Debian `rust-src`). The fix is
byte-neutral on the binding platform (opt-level=z + fat LTO absorb the added
chunk loop and FIFO queue); macOS grew 8 bytes. Debian smoke: ELF PIE stripped,
`ldd` unchanged, `--version` / `--dump-config` clean, `strings` panic-trigger 0.
macOS gate (idle): `tmux_grid` (5), `msedit_diff` (1), release `test-panic-hook`
`pty_smoke` (10) all pass, `strings` 0.

Cross-platform functional runs (full `cargo test --workspace`): **macOS 709/0,
FreeBSD 709/0, Solaris 705/4** (the 4 Solaris failures are the root-caused
ambiguous-width `tmux_grid` quirk). No measurement debt.

## 2026-07-19 keybinding-collision diagnostic (bf48733)

Binding measurement for the silent-collision fix (959915f: report a rejected
plugin keybinding instead of dropping it silently) plus the tmux live-test
additions (bf48733, test-only).

| Platform | 4a841e2 | bf48733 | Delta | Margin |
| --- | ---: | ---: | ---: | ---: |
| macOS x86_64 | 691,036 | 691,036 | 0 | 357,540 |
| Debian x86_64 | 756,096 | 756,096 | 0 | 292,480 |

Byte-neutral on both platforms: the diagnostic's runtime delta is absorbed by
opt-level=z + LTO, and the new i18n key's ten translations are external files
that cost the binary nothing. Debian smoke: ELF PIE stripped, ldd unchanged,
--version / --dump-config clean, strings panic-trigger 0. macOS gate (idle):
tmux_grid (5), tmux_logfilter (4), release test-panic-hook pty_smoke (10) all
pass.

Cross-platform functional runs (full cargo test --workspace): **macOS 715/0,
FreeBSD 715/0, Solaris 709/6**. FreeBSD now runs the four tmux_logfilter live
tests too (it has /usr/bin/python3) and passes them. The six Solaris failures
are all the root-caused ambiguous-width tmux quirk: the four tmux_grid tests
plus, new this run, two tmux_logfilter tests (execute and stream) whose
assertions read body content across several tiled plugin windows, which the
double-width box glyphs truncate — the menu and scratch-title tests are
width-insensitive and pass. Not a dun defect (see docs/solaris-vm.md). No
measurement debt.
