#!/usr/bin/env bash
# Headless sweep of every dialog and editor state that the click-driven pass
# cannot reach: open the menu with Alt+<mnemonic>, run the entry with its bare
# mnemonic letter, capture the 80x24 grid.
#
# Detached tmux only — no GUI terminal is involved and nothing is injected into
# any on-screen application (same mechanism as crates/dun-cli/src/tests/tmux_*).
#
# Mnemonics are stable Latin characters in every catalog (the translated label
# carries them in trailing parens), so one key sequence drives all languages.
#
# Nothing is ever confirmed: no Enter is sent, so the Save As / Run Command
# prompts are only shown, never executed. The fixture is never written back.
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
outdir="${1:-$repo_root/acceptance/gallery/text}"
mkdir -p "$outdir"

langs="en:C de:de_DE.UTF-8 es:es_ES.UTF-8 fr:fr_FR.UTF-8 it:it_IT.UTF-8 \
ja:ja_JP.UTF-8 ko:ko_KR.UTF-8 pt:pt_PT.UTF-8 ru:ru_RU.UTF-8 \
zh-Hans:zh_CN.UTF-8 zh-Hant:zh_TW.UTF-8"

# name:menu-mnemonic:entry-mnemonic
targets="\
dlg-open:f:o dlg-saveas:f:a dlg-runcmd:f:r \
dlg-find:e:f dlg-replace:e:b dlg-gotoline:e:g dlg-help:h:h \
st-selectall:e:a st-whitespace:v:. st-wordwrap:v:z st-bookmark:v:k \
st-splith:v:h st-splitv:v:v st-statushist:v:s st-configdiag:v:d \
st-searchresults:v:w"

session=dunstate
count=0
fail=0
for entry in $langs; do
    tag="${entry%%:*}"
    locale="${entry##*:}"
    for t in $targets; do
        name="${t%%:*}"
        rest="${t#*:}"
        mkey="${rest%%:*}"
        ekey="${rest##*:}"

        tmux kill-session -t "$session" 2>/dev/null
        tmux new-session -d -s "$session" -x 80 -y 24 \
            "LC_ALL=$locale $repo_root/acceptance/launch.sh 2>/dev/null" 2>/dev/null
        sleep 1.2
        tmux send-keys -t "$session" "M-$mkey" 2>/dev/null
        sleep 0.4
        tmux send-keys -t "$session" -l "$ekey" 2>/dev/null
        sleep 0.6
        out="$outdir/$name-$tag.txt"
        tmux capture-pane -p -t "$session" > "$out" 2>/dev/null
        tmux kill-session -t "$session" 2>/dev/null
        if [ -s "$out" ]; then count=$((count + 1)); else fail=$((fail + 1)); echo "EMPTY: $out" >&2; fi
    done
done
echo "captured $count grids ($fail empty) into $outdir"
