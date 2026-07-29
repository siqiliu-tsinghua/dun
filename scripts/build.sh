#!/bin/sh
# Build dun, after asking the two questions that actually change the outcome:
# which of the two builds you want, and whether to build the syntax
# highlighting plugin host along with it.
#
# Questions first, then a summary, then one confirmation, then the work. No
# command runs while there are still questions to answer, so Ctrl-C during
# the interview leaves nothing behind.
#
# Interactive when stdin is a terminal; silent and default-driven otherwise,
# so CI and pipes are unaffected. A question whose flag was given is not
# asked.
#
# POSIX sh with no `local`, like the other scripts here: FreeBSD's base
# system has no bash and Solaris /bin/sh is ksh93.
set -eu

repo_root=$(cd "$(dirname "$0")/.." && pwd)

opt_build=""      # budget | plain
opt_syntect=""    # yes | no
assume_yes=no

usage() {
    cat <<'USAGE'
Usage: scripts/build.sh [options]

Builds dun, and optionally the syntect syntax-highlighting plugin host.
Asks before doing either when stdin is a terminal, and shows everything it
is about to do before it does any of it.

The two builds:
  budget  scripts/release-build.sh — rebuilds the standard library
          (-Zbuild-std) for the smallest binary. Needs the rust-src
          component; this is the build the 1 MiB size budget is measured
          against.
  plain   cargo build --release — the ordinary build. Larger, no extra
          component, always available.

Options:
  --budget          use the size-optimised build
  --plain           use the ordinary cargo release build
  --syntect         also build hosts/rust-syntect (syntax highlighting)
  --no-syntect      do not build it
  -y, --yes         do not ask anything; take the defaults
  -h, --help        show this text

The default build is budget when the rust-src component is installed and
plain when it is not. The syntect host is built by default: it is the one
highlighting host that needs nothing on the target machine except the file
it produces (the Python and Lua hosts need those interpreters).
USAGE
}

die() {
    printf 'build.sh: %s\n' "$*" >&2
    exit 1
}

while [ $# -gt 0 ]; do
    case "$1" in
        --budget) opt_build=budget ;;
        --plain) opt_build=plain ;;
        --syntect) opt_syntect=yes ;;
        --no-syntect) opt_syntect=no ;;
        -y | --yes) assume_yes=yes ;;
        -h | --help) usage; exit 0 ;;
        *) die "unknown option $1 (--help for usage)" ;;
    esac
    shift
done

interactive=no
if [ "$assume_yes" = no ] && [ -t 0 ]; then
    interactive=yes
fi

# ask "question" default(yes|no) -> 0 for yes
ask() {
    if [ "$interactive" = no ]; then
        [ "$2" = yes ]
        return $?
    fi
    if [ "$2" = yes ]; then
        printf '%s [Y/n] ' "$1"
    else
        printf '%s [y/N] ' "$1"
    fi
    ask_reply=""
    read -r ask_reply || ask_reply=""
    case "$ask_reply" in
        [Yy] | [Yy][Ee][Ss]) return 0 ;;
        [Nn] | [Nn][Oo]) return 1 ;;
        *) [ "$2" = yes ]; return $? ;;
    esac
}

have_rust_src() {
    hrs_sysroot=$(rustc --print sysroot 2>/dev/null) || return 1
    [ -d "$hrs_sysroot/lib/rustlib/src/rust" ]
}

command -v cargo >/dev/null 2>&1 || die "cargo is not on PATH; install Rust 1.85 or newer"

# --- decide everything ------------------------------------------------------

if have_rust_src; then
    rust_src=yes
else
    rust_src=no
fi

if [ -z "$opt_build" ]; then
    if [ "$rust_src" = no ]; then
        if [ "$interactive" = yes ]; then
            printf 'The rust-src component is not installed, so the size-optimised\n'
            printf 'build is unavailable (rustup component add rust-src).\n\n'
        fi
        opt_build=plain
    elif ask "Size-optimised build (rebuilds std, smallest binary)?" yes; then
        opt_build=budget
    else
        opt_build=plain
    fi
elif [ "$opt_build" = budget ] && [ "$rust_src" = no ]; then
    die "--budget needs the rust-src component (rustup component add rust-src)"
fi

if [ -z "$opt_syntect" ]; then
    if ask "Also build the syntect syntax-highlighting plugin (a few minutes)?" yes; then
        opt_syntect=yes
    else
        opt_syntect=no
    fi
fi

# --- show the plan, then confirm --------------------------------------------

if [ "$opt_build" = budget ]; then
    build_text='scripts/release-build.sh (size-optimised, rebuilds std)'
else
    build_text='cargo build --release'
fi
if [ "$opt_syntect" = yes ]; then
    plugin_text='hosts/rust-syntect (syntax highlighting)'
else
    plugin_text='none'
fi

printf '\ndun build plan\n'
printf '  %-9s %s\n' "editor" "$build_text"
printf '  %-9s %s\n' "plugin" "$plugin_text"
printf '\n'

if [ "$interactive" = yes ]; then
    if ! ask "Start the build?" yes; then
        printf 'Nothing was built.\n'
        exit 0
    fi
    printf '\n'
fi

# --- build ------------------------------------------------------------------

if [ "$opt_build" = budget ]; then
    "$repo_root/scripts/release-build.sh"
    triple=$(rustc -vV | sed -n 's/^host: //p')
    dun_bin=$repo_root/target/$triple/release/dun
else
    cargo build --release --locked --manifest-path "$repo_root/Cargo.toml"
    dun_bin=$repo_root/target/release/dun
fi
[ -x "$dun_bin" ] || die "the build finished but $dun_bin is not there"

syntect_bin=""
if [ "$opt_syntect" = yes ]; then
    # After dun, and reported separately: a syntect failure (no network for
    # its crates, most often) must not look like a failed editor build.
    if cargo build --release --manifest-path "$repo_root/hosts/rust-syntect/Cargo.toml"; then
        syntect_bin=$repo_root/hosts/rust-syntect/target/release/dun-syntect-host
    else
        printf '\nbuild.sh: the syntect host failed to build; dun itself is fine.\n' >&2
        printf 'build.sh: install without it, or re-run with --no-syntect.\n' >&2
    fi
fi

printf '\ndun build\n'
printf '  %-9s %s\n' "editor" "$dun_bin"
if [ -n "$syntect_bin" ]; then
    printf '  %-9s %s\n' "plugin" "$syntect_bin"
fi
printf '  %-9s %s\n' "" "next: scripts/install.sh"
