# Release Size Audit

This document records lightweight release binary size checks for `dun`.
Results are machine-local baselines, not release claims.

## Hard Budget

The v0.1 release budget is strict:

```text
target/release/dun <= 1,048,576 bytes
```

The gate must pass on both audited macOS x86_64 and Debian x86_64 builds. The
checked-in `[profile.release]` is the size-budget profile, so the release
measurement command is simply:

```text
cargo build --release --locked -p dun-cli
```

If either platform is above the limit, follow the feature trim order in
[feature-budget.md](./feature-budget.md). Do not add runtime features while
the budget is failing.

## Build Profiles

The checked-in release build:

```text
cargo build --release --locked -p dun-cli
```

The previous audit used a size-oriented profile through environment variables.
That profile is now checked in as `[profile.release]`:

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
2. Build the release binary with `cargo build --release --locked -p dun-cli`.
3. Record byte size with `stat`, executable type with `file`, and dynamic
   dependencies with `otool -L` on macOS or `ldd` on Linux.
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

Source baseline: working tree based on `4626c7f`, after adding the checked-in
release-size profile and `docs/feature-budget.md`.

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
