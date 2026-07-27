#!/usr/bin/env bash
# Inner runner for the gallery-screenshot pass: pin the window to an exact
# character geometry, then hand off to acceptance/launch.sh.
#
# Geometry is set with the xterm window-manipulation sequence CSI 8 ; rows ;
# cols t, which kitty, iTerm2 and Terminal.app all honour. That keeps the
# three emulators comparable without needing per-emulator window options —
# and it is ordinary program output, the same channel dun itself draws on.
#
# Usage: gallery-run.sh [rows cols] -- [launch.sh args...]
set -uo pipefail

rows=24
cols=80
if [ "${1:-}" != "--" ]; then
    rows="${1:?gallery-run.sh: rows}"
    cols="${2:?gallery-run.sh: cols}"
    shift 2
fi
[ "${1:-}" = "--" ] && shift

# Sent twice: kitty and Terminal.app honour the first one, but iTerm2 can still
# be laying the window out that early and drops it. Re-sending after the window
# settles is idempotent for the two that already applied it.
printf '\033[8;%d;%dt' "$rows" "$cols"
sleep 1.2
printf '\033[8;%d;%dt' "$rows" "$cols"
sleep 0.6          # let the emulator finish resizing before dun measures

exec "$(dirname "$0")/launch.sh" "$@"
