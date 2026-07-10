# Debian Measurement VM

The Debian VirtualBox VM `debvbox` is the binding measurement environment for
`dun`: release binary size gates, runtime resource audits, release smoke runs,
and the external SSH / low-capability terminal matrix all record their Debian
results from this machine.

## Connection

- VirtualBox VM name: `debvbox` (Debian 13 amd64, system `rustc`/`cargo`
  1.85 from Debian packages).
- Reachable through a NAT port forward on the host: `localhost` port `2222`.
- User: `fft` (same as the macOS host user).
- SSH keypair: `vm-test/dun-vm-test` and `vm-test/dun-vm-test.pub` in this
  repository. The directory is gitignored; the private key is not in git
  history and must stay untracked.
- The user account has passwordless `sudo` for installing required packages.
- The VM is started manually from the VirtualBox UI by the project owner; ask
  for it to be started before a binding measurement.

Use the tracked wrapper scripts in `vm-test/` instead of remembering raw
`ssh` invocations; they share their connection settings through
`vm-test/vm-common.sh`:

```text
# interactive shell
vm-test/vm-run

# one-off command (quote metacharacters as for plain ssh)
vm-test/vm-run sudo whoami
vm-test/vm-run 'cd ~/dun-abc1234 && cargo test --workspace'

# clean git-archive sync of a commit (default HEAD) -> ~/dun-<shortsha>
vm-test/vm-sync
vm-test/vm-sync v0.1-rc1

# working-tree rsync for iteration only -> ~/dun-worktree
vm-test/vm-sync --worktree
```

Binding measurements must build from a `vm-test/vm-sync` clean-commit
directory, never from `~/dun-worktree`. The raw connection, should the
scripts be unavailable, is `ssh -i vm-test/dun-vm-test -p 2222 fft@localhost`;
port and destination can be overridden with `DUN_VM_PORT` / `DUN_VM_DEST`.

## Working Conventions

- Build from a clean `git archive` copy, not from a live checkout, so the
  measured source state is exactly one commit. Follow the repeat checklist in
  [release-size-audit.md](./release-size-audit.md).
- Use one dated working directory per task (`~/dun-work-YYYYMMDD` or similar)
  and delete it after the results are recorded in the repository documents.
  The repository is the record; the VM is disposable scratch space.
- Do not develop on the VM or leave uncommitted work there. On 2026-07-10 two
  leftover checkouts (one with an uncommitted, already-landed diff) had to be
  verified against main history before they could be cleaned up.
- The keypair grants access to a local test VM only; do not reuse it for any
  other host.
