#!/usr/bin/env bash
# Open one gallery capture in a chosen macOS terminal emulator.
#
# The three emulators take a command very differently: kitty accepts one after
# its options, while Terminal.app and iTerm2 only accept a *script file* via
# `open -a`. So the launch args are baked into a generated wrapper script and
# that wrapper is handed to whichever emulator was asked for. Window geometry
# is unified by gallery-run.sh's CSI 8 t sequence, not per-emulator options.
#
# Usage: gallery-open.sh <kitty|iterm|terminal> [rows cols] -- [launch.sh args]
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
emulator="${1:?gallery-open.sh: emulator}"
shift

rows=24
cols=80
if [ "${1:-}" != "--" ]; then
    rows="${1:?rows}"; cols="${2:?cols}"; shift 2
fi
[ "${1:-}" = "--" ] && shift

# BSD mktemp only substitutes X's at the END of the template, so build the
# name first and add the .command suffix Terminal.app needs afterwards.
wrapper_base="$(mktemp "${TMPDIR:-/tmp}/dun-gallery-XXXXXX")"
wrapper="$wrapper_base.command"
mv "$wrapper_base" "$wrapper"
{
    echo '#!/bin/bash'
    # Terminal.app and iTerm2 run a .command file from the user's home, not
    # from the repo, so relative --file arguments would not resolve.
    printf 'cd %q || exit 1\n' "$repo_root"
    # The locale must be BAKED IN, not inherited: `open -a` does not propagate
    # the caller's environment. Terminal.app then runs the .command through a
    # login shell that sets LANG from the user's profile, and kitty runs it
    # with no profile at all — so a caller-side `LC_ALL=xx gallery-open.sh`
    # silently produced the user's own language in one emulator and English in
    # another, never the requested one. Verified: inside a Terminal session
    # launched with `env LC_ALL=de_DE.UTF-8 open -a Terminal`, LC_ALL is unset
    # and LANG is the profile's.
    # DUN_PYGMENTS_PYTHON rides along for the same reason: launch.sh reads it,
    # and it would be dropped by `open -a` exactly like the locale was.
    for var in LC_ALL LC_MESSAGES LANG DUN_PYGMENTS_PYTHON; do
        eval "value=\${$var-}"
        if [ -n "${value:-}" ]; then
            printf 'export %s=%q\n' "$var" "$value"
        else
            printf 'unset %s\n' "$var"
        fi
    done
    printf 'exec %q %q %q -- ' "$repo_root/acceptance/gallery-run.sh" "$rows" "$cols"
    for a in "$@"; do printf '%q ' "$a"; done
    echo
} > "$wrapper"
chmod 755 "$wrapper"

case "$emulator" in
    kitty)
        open -na kitty --args \
            -o "initial_window_width=${cols}c" -o "initial_window_height=${rows}c" \
            -o remember_window_size=no -o font_size=14 \
            --hold "$wrapper"
        ;;
    iterm)
        # NOT `open -n`: that spawns a second iTerm *instance*. Plain `open -a`
        # hands the script to the running one, which opens it in a new window.
        open -a iTerm "$wrapper"
        ;;
    terminal)
        open -a Terminal "$wrapper"
        ;;
    *)
        echo "unknown emulator: $emulator (kitty|iterm|terminal)" >&2
        exit 2
        ;;
esac
echo "$wrapper"
