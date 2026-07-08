# Release Size Audit

This document records lightweight release binary size checks for `dun`.
Results are machine-local baselines, not release claims.

## Build Profiles

Default release build:

```text
CARGO_TARGET_DIR=target/size-audit/PLATFORM-default \
cargo build --release --locked -p dun-cli
```

Size-oriented release build, without changing repository profile settings:

```text
CARGO_TARGET_DIR=target/size-audit/PLATFORM-size \
CARGO_PROFILE_RELEASE_OPT_LEVEL=z \
CARGO_PROFILE_RELEASE_LTO=fat \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
CARGO_PROFILE_RELEASE_STRIP=symbols \
CARGO_PROFILE_RELEASE_PANIC=abort \
cargo build --release --locked -p dun-cli
```

The size-oriented profile is intended as an audit reference for small binary
targets. It trades build time and possibly some runtime performance for a
smaller executable. Do not make it the default release policy without a
separate performance pass.

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

## Architecture Conclusion

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

## Notes

- The current workspace has no checked-in custom `[profile.release]`.
- The audit does not use UPX, static linking, musl, or platform-specific
  packaging.
- The Debian result was built from a clean git archive copied to
  `/tmp/dun-size-audit` on the VM, using the Debian system `rustc`/`cargo`.
- Future audits should record the commit, toolchain, host OS, exact build
  command, byte size, `file` output, dependency listing, and `--version`
  smoke result.
