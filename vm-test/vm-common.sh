# Shared connection settings for the debvbox measurement VM.
# Sourced by vm-run and vm-sync; not executable on its own.
# See docs/debian-vm.md for the VM description and working conventions.

VM_TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$VM_TEST_DIR/.." && pwd)"

VM_KEY="$VM_TEST_DIR/dun-vm-test"
VM_PORT="${DUN_VM_PORT:-2222}"
VM_DEST="${DUN_VM_DEST:-fft@localhost}"
VM_SSH_OPTS=(-i "$VM_KEY" -p "$VM_PORT" -o ConnectTimeout=10)

if [ ! -f "$VM_KEY" ]; then
    echo "vm-test: SSH key not found at $VM_KEY" >&2
    echo "vm-test: the keypair is machine-local and never committed;" >&2
    echo "vm-test: ask the project owner for it (docs/debian-vm.md)." >&2
    exit 1
fi
