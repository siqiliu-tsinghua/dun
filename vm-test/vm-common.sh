# Shared connection settings for the local test/measurement VMs.
# Sourced by vm-run and vm-sync; not executable on its own.
#
# Multiple VMs are supported through a target selector: pick one with the
# wrappers' `-t NAME` flag or the DUN_VM_TARGET env var (default: debian).
# Each target is a VirtualBox guest on a localhost NAT port; they share one
# keypair and account (fft@localhost) with passwordless sudo. See
# docs/dev/debian-vm.md and docs/dev/freebsd-vm.md.

VM_TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$VM_TEST_DIR/.." && pwd)"

VM_KEY="$VM_TEST_DIR/dun-vm-test"
VM_TARGET="${DUN_VM_TARGET:-debian}"

case "$VM_TARGET" in
    debian) VM_DEFAULT_PORT=2222 ;;
    freebsd) VM_DEFAULT_PORT=2233 ;;
    solaris) VM_DEFAULT_PORT=2244 ;;
    *)
        echo "vm-test: unknown target '$VM_TARGET' (known: debian, freebsd, solaris)" >&2
        echo "vm-test: select one with '-t NAME' or DUN_VM_TARGET=NAME." >&2
        exit 1
        ;;
esac

VM_PORT="${DUN_VM_PORT:-$VM_DEFAULT_PORT}"
VM_DEST="${DUN_VM_DEST:-fft@localhost}"

# Every target is localhost on a different port, which collides in the user's
# global known_hosts; keep a repo-local one (gitignored) and accept new keys
# (a changed key still fails, guarding against a surprise host swap).
VM_KNOWN_HOSTS="$VM_TEST_DIR/known_hosts"
VM_SSH_OPTS=(-i "$VM_KEY" -p "$VM_PORT" -o ConnectTimeout=10 \
    -o StrictHostKeyChecking=accept-new \
    -o UserKnownHostsFile="$VM_KNOWN_HOSTS")

if [ ! -f "$VM_KEY" ]; then
    echo "vm-test: SSH key not found at $VM_KEY" >&2
    echo "vm-test: the keypair is machine-local and never committed;" >&2
    echo "vm-test: ask the project owner for it (docs/dev/debian-vm.md)." >&2
    exit 1
fi
