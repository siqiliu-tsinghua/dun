#!/usr/bin/env bash
# Inner runner for the SSH dimension of the gallery pass: pin the local window
# geometry, then run dun *on a VM* over a real SSH connection.
#
# This is the acceptance case the local captures cannot reach. dun's product
# goal is editing over SSH, and everything that makes that hard lives in the
# link, not in the editor: terminal type negotiated through ssh, colours and
# wide glyphs surviving the hop, and the remote PTY learning the local window
# size. `ssh -t` is what forces a PTY; without it dun sees no terminal at all.
#
# Usage: gallery-ssh.sh <debian|freebsd|solaris> <rows> <cols> [dun args...]
set -uo pipefail

target="${1:?gallery-ssh.sh: vm target}"
rows="${2:?rows}"
cols="${3:?cols}"
shift 3

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

case "$target" in
    debian) port=2222 ;;
    freebsd) port=2233 ;;
    solaris) port=2244 ;;
    *) echo "unknown vm target: $target" >&2; exit 2 ;;
esac

# Same sequence the local runner uses; sent twice because iTerm2 can still be
# laying the window out on the first one.
printf '\033[8;%d;%dt' "$rows" "$cols"
sleep 1.2
printf '\033[8;%d;%dt' "$rows" "$cols"
sleep 0.6

# The remote side needs the same throwaway-config treatment as the local one:
# its own config dir with the catalogs beside it, so the language is what this
# run asked for rather than whatever the VM account happens to have.
remote_setup='
  set -e
  d=$HOME/dun-5934810
  s=$(mktemp -d)
  printf "theme = %s\nterminal.encoding = utf8\nterminal.colors = 256\nmouse.enabled = false\n" "${DUN_THEME:-dun}" > "$s/config"
  cp -R "$d/i18n" "$s/i18n"
  cd "$d"
  # The capture must carry proof of which host it came from. Without it the
  # three VMs render byte-identically -- same fixture, same dun -- and the
  # screenshot cannot distinguish "connected to three machines" from
  # "connected to one machine three times". uname goes in the file, not just
  # the window title, because iTerm2 shows only "ssh" there.
  {
    echo "REMOTE HOST: $(uname -s) $(uname -r) $(uname -m)  [$(hostname)]"
    echo
    cat acceptance/fixture.txt
  } > "$s/fixture.txt"
  DUN_CONFIG="$s/config" LC_ALL="${DUN_LOCALE:-C}" \
    ./target/release/dun "$s/fixture.txt"
  rm -rf "$s"
'

exec ssh -t \
    -i "$repo_root/vm-test/dun-vm-test" \
    -p "$port" \
    -o ConnectTimeout=10 \
    -o StrictHostKeyChecking=accept-new \
    -o UserKnownHostsFile="$repo_root/vm-test/known_hosts" \
    -o SendEnv=DUN_THEME \
    fft@localhost \
    "DUN_THEME='${DUN_THEME:-dun}' DUN_LOCALE='${DUN_LOCALE:-C}' sh -c $(printf '%q' "$remote_setup")"
