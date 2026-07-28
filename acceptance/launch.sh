#!/usr/bin/env bash
# Launch dun deterministically for the real-terminal acceptance checklist and
# the gallery-screenshot pass (docs/dev/real-terminal-acceptance.md).
#
# It writes a throwaway config to a scratch dir and points dun at it with
# DUN_CONFIG, so your real ~/.config/dun is never touched. It opens the
# committed acceptance fixture. Resize the terminal window to the size the
# checklist item asks for (usually 80x24) BEFORE reading the result.
#
# Usage:
#   acceptance/launch.sh [theme] [flags]
#     theme            dun (default) | msedit | dark | turbo
#   flags (repeatable, any order):
#     --osc52-write    clipboard.osc52.enabled = true   (edit.copy_external)
#     --osc52-read     clipboard.osc52.allow_read = true (edit.paste_external)
#     --mouse          mouse.enabled = true
#     --ascii          terminal.encoding = ascii
#     --16color        terminal.colors = 16
#     --mono           terminal.colors = mono
#     --file PATH      open PATH instead of the default fixture
#     --lang TAG       Optional sugar for the i18n gallery sweep only.
#
# LANGUAGE IS ENV-DRIVEN. dun picks the UI language itself from the first
# nonempty of LC_ALL / LC_MESSAGES / LANG (crates/dun-cli/src/i18n_loading.rs),
# and this script passes the ambient environment through untouched. So the
# normal way to select a language is the normal way:
#
#   LC_ALL=ja_JP.UTF-8 acceptance/launch.sh
#
# What the script must do regardless of language is copy the catalogs: dun
# resolves `i18n/` NEXT TO THE ACTIVE CONFIG FILE, and this script's config
# lives in a throwaway scratch dir, so without the copy every launch would be
# silently English. That copy always happens.
#
# --lang TAG is a convenience for sweeping all ten catalogs in one pass; it
# only sets LC_ALL for that run. TAG is a catalog name from i18n/ (de es fr it
# ja ko pt ru zh-Hans zh-Hant) or `en`. It exists because the tag is NOT usable
# as a locale directly: locale_candidates() upper-cases the region subtag, so
# LC_ALL=zh-Hans yields ["zh-HANS","zh"] and matches zh-Hans.conf only on a
# case-insensitive filesystem (macOS) — silently English on Linux. Each tag
# therefore maps to a real POSIX locale below. --ascii forces English by design
# (the sanitizer would escape non-ASCII labels), so --lang + --ascii is
# rejected rather than silently ignored.
#
# Examples:
#   acceptance/launch.sh msedit --mouse
#   acceptance/launch.sh dun --osc52-read --osc52-write
#   NO_COLOR=1 acceptance/launch.sh dun     # capability-fallback mono path
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
theme="dun"
file="$repo_root/acceptance/fixture.txt"
osc52_write="false"
osc52_read="false"
mouse="false"
encoding="utf8"
colors="256"
lang=""          # empty = inherit the ambient locale, like a real user
syntax=""        # empty = no highlight host; else syntect|pygments|lua
logfilter=""     # empty = no log-filter host; else python|lua

# Catalog tag -> a POSIX locale whose locale_candidates() actually resolves to
# that catalog file. Keep in sync with i18n/ and dun-config's locale_script().
locale_for_tag() {
    case "$1" in
        en)      echo "C" ;;
        de)      echo "de_DE.UTF-8" ;;
        es)      echo "es_ES.UTF-8" ;;
        fr)      echo "fr_FR.UTF-8" ;;
        it)      echo "it_IT.UTF-8" ;;
        ja)      echo "ja_JP.UTF-8" ;;
        ko)      echo "ko_KR.UTF-8" ;;
        pt)      echo "pt_PT.UTF-8" ;;
        ru)      echo "ru_RU.UTF-8" ;;
        zh-Hans) echo "zh_CN.UTF-8" ;;
        zh-Hant) echo "zh_TW.UTF-8" ;;
        *)       return 1 ;;
    esac
}

while [ $# -gt 0 ]; do
    case "$1" in
        dun|msedit|dark|turbo) theme="$1" ;;
        --osc52-write) osc52_write="true" ;;
        --osc52-read) osc52_read="true" ;;
        --mouse) mouse="true" ;;
        --ascii) encoding="ascii" ;;
        --16color) colors="16" ;;
        --mono) colors="mono" ;;
        --file) shift; file="$1" ;;
        --lang) shift; lang="${1:-}" ;;
        --syntax) syntax="${2:-syntect}"; case "${2:-}" in syntect|pygments|lua) shift ;; esac ;;
        --logfilter) logfilter="${2:-python}"; case "${2:-}" in python|lua) shift ;; esac ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

locale=""
if [ -n "$lang" ]; then
    if ! locale="$(locale_for_tag "$lang")"; then
        echo "unknown --lang tag: $lang" >&2
        echo "known tags: en $(ls "$repo_root/i18n" | sed 's/\.conf$//' | tr '\n' ' ')" >&2
        exit 2
    fi
    if [ "$lang" != "en" ] && [ "$encoding" = "ascii" ]; then
        echo "--lang $lang with --ascii: dun forces English on ASCII terminals," >&2
        echo "so this combination cannot show translated text. Drop one of them." >&2
        exit 2
    fi
fi

# Prefer the size-budget build; fall back to a plain release build; build if neither exists.
bin=""
for candidate in \
    "$repo_root/target/x86_64-apple-darwin/release/dun" \
    "$repo_root/target/aarch64-apple-darwin/release/dun" \
    "$repo_root/target/release/dun"; do
    if [ -x "$candidate" ]; then bin="$candidate"; break; fi
done
if [ -z "$bin" ]; then
    echo "no dun binary found; building (cargo build --release)..." >&2
    ( cd "$repo_root" && cargo build --release >&2 )
    bin="$repo_root/target/release/dun"
fi

scratch="$(mktemp -d "${TMPDIR:-/tmp}/dun-acceptance.XXXXXX")"
config="$scratch/config"
cat > "$config" <<EOF
# Generated by acceptance/launch.sh — throwaway config, safe to delete.
theme = $theme
terminal.encoding = $encoding
terminal.colors = $colors
mouse.enabled = $mouse
clipboard.osc52.enabled = $osc52_write
clipboard.osc52.allow_read = $osc52_read
EOF

# Optional syntax-highlight host (hosts/README.md). dun launches the command
# directly — no shell, no arguments, CLEARED ENVIRONMENT — so interpreter-based
# hosts get an absolute-path wrapper written next to the config.
if [ -n "$syntax" ]; then
    case "$syntax" in
        syntect)
            host_cmd="$repo_root/hosts/rust-syntect/target/release/dun-syntect-host"
            if [ ! -x "$host_cmd" ]; then
                echo "syntect host not built: $host_cmd" >&2
                echo "build it with: (cd hosts/rust-syntect && cargo build --release)" >&2
                exit 2
            fi
            ;;
        pygments)
            # Pygments is not in the standard library and is usually absent
            # from the system python. Point DUN_PYGMENTS_PYTHON at one that
            # has it (a throwaway venv is fine) instead of installing into the
            # system interpreter. The check is up front and fatal on purpose:
            # a host that starts but cannot import pygments still connects and
            # still shows up in the status bar, and returns no spans — in a
            # screenshot that is indistinguishable from a file with nothing to
            # highlight.
            pyg_python="${DUN_PYGMENTS_PYTHON:-$(command -v python3)}"
            if ! "$pyg_python" -c "import pygments" 2>/dev/null; then
                echo "--syntax pygments: $pyg_python cannot import pygments" >&2
                echo "point DUN_PYGMENTS_PYTHON at an interpreter that can, e.g." >&2
                echo "  python3 -m venv /tmp/pygvenv && /tmp/pygvenv/bin/pip install Pygments" >&2
                echo "  DUN_PYGMENTS_PYTHON=/tmp/pygvenv/bin/python acceptance/launch.sh --syntax pygments" >&2
                exit 2
            fi
            host_cmd="$scratch/pygments-wrapper"
            printf '#!/bin/sh\nexec %s %s\n' \
                "$pyg_python" \
                "$repo_root/hosts/python-pygments/dun-pygments-host.py" > "$host_cmd"
            chmod +x "$host_cmd"
            ;;
        lua)
            lua_bin="$(command -v lua5.4 || command -v lua5.3 || command -v lua || true)"
            if [ -z "$lua_bin" ]; then
                echo "no lua interpreter found for --syntax lua" >&2
                exit 2
            fi
            host_cmd="$scratch/lua-wrapper"
            printf '#!/bin/sh\nexec %s %s\n' \
                "$lua_bin" "$repo_root/hosts/lua-highlight/dun-lua-highlight-host.lua" > "$host_cmd"
            chmod +x "$host_cmd"
            ;;
        *) echo "unknown --syntax engine: $syntax (syntect|pygments|lua)" >&2; exit 2 ;;
    esac
    cat >> "$config" <<EOF
plugins.status_bar = true
plugin.$syntax.command = $host_cmd
plugin.$syntax.trust = user-trusted-external
plugin.$syntax.roles = syntax-highlight
EOF
    echo "syntax : $syntax -> $host_cmd" >&2
fi

# Optional log-filter host. Unlike the highlight hosts this one is UI-invasive:
# it contributes a top-level "Log Filter" menu, a Ctrl+T keybinding leader, and
# up to two windows of its own, so it is what the split/menu appearance pass
# needs. Same cleared-environment rule — script hosts get a wrapper.
if [ -n "$logfilter" ]; then
    case "$logfilter" in
        python)
            lf_cmd="$scratch/logfilter-python-wrapper"
            printf '#!/bin/sh\nexec %s %s\n' \
                "$(command -v python3)" \
                "$repo_root/hosts/python-logfilter/dun-python-logfilter-host.py" > "$lf_cmd"
            ;;
        lua)
            lf_lua="$(command -v lua5.4 || command -v lua5.3 || command -v lua || true)"
            if [ -z "$lf_lua" ]; then
                echo "no lua interpreter found for --logfilter lua" >&2
                exit 2
            fi
            lf_cmd="$scratch/logfilter-lua-wrapper"
            printf '#!/bin/sh\nexec %s %s\n' \
                "$lf_lua" \
                "$repo_root/hosts/lua-logfilter/dun-lua-logfilter-host.lua" > "$lf_cmd"
            ;;
        *) echo "unknown --logfilter host: $logfilter (python|lua)" >&2; exit 2 ;;
    esac
    chmod +x "$lf_cmd"
    cat >> "$config" <<EOF
plugins.status_bar = true
plugin.logfilter.command = $lf_cmd
plugin.logfilter.trust = user-trusted-external
plugin.logfilter.roles = log-filter
EOF
    echo "logfilt: $logfilter -> $lf_cmd (leader Ctrl+T; Ctrl+T e edit, Ctrl+T a apply)" >&2
fi

# dun resolves catalogs relative to the active config file, so they have to
# travel with the throwaway config — always, so that the ambient locale can
# resolve exactly as it would against a real ~/.config/dun/i18n.
cp -R "$repo_root/i18n" "$scratch/i18n"

echo "dun    : $bin" >&2
echo "config : $config (theme=$theme encoding=$encoding colors=$colors mouse=$mouse osc52_write=$osc52_write osc52_read=$osc52_read)" >&2
echo "file   : $file" >&2
echo "catalog: $scratch/i18n (10 files)" >&2

# Language is env-driven, exactly as it is for a real user: dun reads the first
# nonempty of LC_ALL/LC_MESSAGES/LANG itself. Without --lang the ambient
# environment is passed through untouched, so the launcher exercises the real
# path instead of a launcher-only override. --lang is only sugar for the
# gallery sweep and is equivalent to setting LC_ALL yourself.
if [ -n "$lang" ]; then
    echo "lang   : --lang $lang -> LC_ALL=$locale (overriding the ambient locale)" >&2
    DUN_CONFIG="$config" LC_ALL="$locale" "$bin" "$file"
else
    echo "lang   : inherited from the environment (LC_ALL=${LC_ALL:-unset} LC_MESSAGES=${LC_MESSAGES:-unset} LANG=${LANG:-unset})" >&2
    DUN_CONFIG="$config" "$bin" "$file"
fi
status=$?
rm -rf "$scratch"
exit $status
