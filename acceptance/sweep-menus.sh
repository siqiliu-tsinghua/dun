#!/usr/bin/env bash
# Headless i18n sweep: open each top-level menu in each shipped language and
# save the 80x24 character grid.
#
# This drives dun in a DETACHED tmux session — no GUI terminal is involved and
# nothing is injected into any on-screen application. It is the same mechanism
# the repo's own acceptance tests use (crates/dun-cli/src/tests/tmux_*.rs).
# Text grids beat screenshots for i18n review: they diff, they grep, and the
# display width of every row can be checked mechanically.
#
# Usage: acceptance/sweep-menus.sh [outdir]
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
outdir="${1:-$repo_root/acceptance/gallery/text}"
mkdir -p "$outdir"

# tag:POSIX locale — same mapping as launch.sh's locale_for_tag.
langs="en:C de:de_DE.UTF-8 es:es_ES.UTF-8 fr:fr_FR.UTF-8 it:it_IT.UTF-8 \
ja:ja_JP.UTF-8 ko:ko_KR.UTF-8 pt:pt_PT.UTF-8 ru:ru_RU.UTF-8 \
zh-Hans:zh_CN.UTF-8 zh-Hant:zh_TW.UTF-8"

# menu mnemonic:name — Latin mnemonics are stable across all catalogs.
menus="f:file e:edit v:view h:help"

session=dunsweep
count=0
for entry in $langs; do
    tag="${entry%%:*}"
    locale="${entry##*:}"
    for m in $menus; do
        key="${m%%:*}"
        name="${m##*:}"
        tmux kill-session -t "$session" 2>/dev/null
        tmux new-session -d -s "$session" -x 80 -y 24 \
            "LC_ALL=$locale $repo_root/acceptance/launch.sh 2>/dev/null" 2>/dev/null
        sleep 1.3
        tmux send-keys -t "$session" "M-$key" 2>/dev/null
        sleep 0.7
        out="$outdir/menu-$name-$tag.txt"
        tmux capture-pane -p -t "$session" > "$out" 2>/dev/null
        tmux kill-session -t "$session" 2>/dev/null
        if [ -s "$out" ]; then
            count=$((count + 1))
        else
            echo "EMPTY: $out" >&2
        fi
    done
done
echo "captured $count grids into $outdir"
