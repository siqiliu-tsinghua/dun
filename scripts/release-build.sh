#!/usr/bin/env bash
# The budget release build for dun (build contract decided 2026-07-10).
#
# Rebuilds std with the workspace release profile and NO default std
# features: panic-backtrace symbolization (gimli/addr2line/rustc_demangle)
# drops out of the binary while panic hooks and panic messages keep working
# (verified by experiment; see docs/release-size-audit.md 2026-07-10).
#
# RUSTC_BOOTSTRAP=1 unlocks -Z flags on the stable toolchain — the compiler
# itself stays the pinned stable 1.85 (the pattern microsoft/edit's README
# recommends for stable builders). Prerequisite, once per machine:
#   rustup toolchains:  rustup component add rust-src
#   Debian system rust: sudo apt-get install rust-src
#
# Dev builds and `cargo test` keep the plain stable path; only this budget
# build uses build-std.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
triple=$(rustc -vV | sed -n 's/^host: //p')

RUSTC_BOOTSTRAP=1 cargo build --release --locked -p dun-cli \
    -Zbuild-std=std,panic_abort -Zbuild-std-features= \
    --target "$triple" --manifest-path "$repo_root/Cargo.toml"

bin="$repo_root/target/$triple/release/dun"
case "$(uname -s)" in
    Darwin) size=$(stat -f%z "$bin") ;;
    *) size=$(stat -c%s "$bin") ;;
esac
echo "binary: $bin"
echo "size:   $size bytes (budget 1048576)"
