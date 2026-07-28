#!/usr/bin/env bash
# The budget release build for dun (build contract decided 2026-07-10).
#
# Rebuilds std with the workspace release profile and NO default std
# features: panic-backtrace symbolization (gimli/addr2line/rustc_demangle)
# drops out of the binary while panic hooks and panic messages keep working
# (verified by experiment; see docs/dev/release-size-audit.md 2026-07-10).
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

# Solaris only: drop .SUNW_ldynsym. The native link editor keeps local function
# names in the dynamic symbol table so pstack and dtrace can name frames; in a
# stripped release binary that section, its two sort tables and the .dynstr
# entries they need come to ~343 KB — a third of the whole budget, and none of
# it code. Panic messages carry file:line and this build does not link
# backtrace symbolization anyway, so little is lost. Measured 2026-07-29:
# 1,087,760 -> 744,880 bytes, suite 906/0, release panic path intact.
# Set DUN_SOLARIS_KEEP_LDYNSYM=1 to keep the section — needed if you link with
# GNU ld, which does not know the option.
if [ "$(uname -s)" = "SunOS" ] && [ -z "${DUN_SOLARIS_KEEP_LDYNSYM:-}" ]; then
    # Appended, so a caller's own RUSTFLAGS survive; set only here, so on every
    # other platform an unset RUSTFLAGS keeps deferring to the user's config.
    RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-znoldynsym"
    export RUSTFLAGS
fi

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
