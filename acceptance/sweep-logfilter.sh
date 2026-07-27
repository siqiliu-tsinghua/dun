#!/usr/bin/env bash
# Headless appearance sweep for the log-filter plugin: its injected menu, its
# two plugin-owned windows, and those windows combined with dun's own splits.
#
# The log-filter host is the UI-invasive one — menu + keybinding + window +
# scratch-input + surface-write — so it is the only host that exercises the
# tiled layout with foreign windows in it. Driven in a DETACHED tmux session;
# no GUI terminal is involved.
#
# Chords come from crates/dun-cli/tests/tmux_logfilter.rs, which is the
# authority: the host's leader is Ctrl+T (NOT the Ctrl+L in hosts/README.md),
# `Ctrl+T e` opens its scratch window, `Ctrl+T a` submits it, and dun's own
# `Ctrl+X o` Run Command feeds the host's stream surface.
#
# Geometry is 100x30 rather than the usual 80x24: two plugin windows plus a
# dun split do not fit in 80 columns, and the point here is the layout.
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
outdir="${1:-$repo_root/acceptance/gallery/text}"
host="${LOGFILTER_HOST:-python}"
mkdir -p "$outdir"

COLS=100
ROWS=30
session=dunlf

start() {
    tmux kill-session -t "$session" 2>/dev/null
    tmux new-session -d -s "$session" -x "$COLS" -y "$ROWS" \
        "LC_ALL=$1 $repo_root/acceptance/launch.sh --logfilter $host 2>/dev/null" 2>/dev/null
    # Wait for the handshake rather than sleeping blind. The readiness signal
    # must be the status bar's `[logfilter]` plugin id, NOT the menu label:
    # the host translates its top-level label (`Log Filter` -> `日志过滤`), so
    # matching on the English text silently never fires in other languages.
    for _ in $(seq 1 40); do
        sleep 0.3
        tmux capture-pane -p -t "$session" 2>/dev/null | tail -1 | grep -q "\[logfilter\]" && {
            # `[logfilter]` lights up at handshake, which is BEFORE the menu
            # contribution is installed — grabbing here caught a menu bar
            # without the plugin entry. Wait for row 0 to actually grow.
            local base cur
            base="$(tmux capture-pane -p -t "$session" | sed -n '1p' | sed 's/ *$//' | wc -c)"
            for _ in $(seq 1 20); do
                sleep 0.25
                cur="$(tmux capture-pane -p -t "$session" | sed -n '1p' | sed 's/ *$//' | wc -c)"
                [ "$cur" -gt "$base" ] && return 0
            done
            return 0
        }
    done
    return 1
}

grab() {
    tmux capture-pane -p -t "$session" > "$outdir/lf-$1.txt" 2>/dev/null
    printf '  %-28s %s\n' "lf-$1" "$(sed -n '1p' "$outdir/lf-$1.txt" | cut -c1-52)"
}

for entry in en:C zh-Hans:zh_CN.UTF-8; do
    tag="${entry%%:*}"
    locale="${entry##*:}"
    echo "=== $tag ==="

    # 1. menu bar carries the injected top-level entry
    start "$locale" || { echo "  MISS: host never handshook"; continue; }
    grab "menubar-$tag"

    # 2. the plugin's own dropdown, opened by its mnemonic
    tmux send-keys -t "$session" M-l 2>/dev/null; sleep 0.6
    grab "menu-open-$tag"
    tmux send-keys -t "$session" Escape 2>/dev/null; sleep 0.3

    # 3. plugin-owned scratch window (window + scratch-input capabilities)
    tmux send-keys -t "$session" C-t 2>/dev/null; sleep 0.2
    tmux send-keys -t "$session" -l e 2>/dev/null; sleep 1.0
    grab "scratch-$tag"

    # 4. scratch + surface: type a pattern and submit it (execute -> surface)
    tmux send-keys -t "$session" -l "needle" 2>/dev/null; sleep 0.4
    tmux send-keys -t "$session" C-t 2>/dev/null; sleep 0.2
    tmux send-keys -t "$session" -l a 2>/dev/null; sleep 1.2
    grab "surface-$tag"

    # 5. both plugin windows plus a dun split on top
    tmux send-keys -t "$session" M-v 2>/dev/null; sleep 0.4
    tmux send-keys -t "$session" -l h 2>/dev/null; sleep 0.8
    grab "split-$tag"
    tmux kill-session -t "$session" 2>/dev/null

    # 6. command output streamed into the host's surface
    start "$locale" || { echo "  MISS: host never handshook (stream)"; continue; }
    tmux send-keys -t "$session" C-x 2>/dev/null; sleep 0.2
    tmux send-keys -t "$session" -l o 2>/dev/null; sleep 0.8
    tmux send-keys -t "$session" -l "seq 5" 2>/dev/null; sleep 0.3
    tmux send-keys -t "$session" Enter 2>/dev/null; sleep 1.5
    grab "stream-$tag"
    tmux kill-session -t "$session" 2>/dev/null
done
echo "done -> $outdir/lf-*.txt"
