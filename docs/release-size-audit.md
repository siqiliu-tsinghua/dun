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
