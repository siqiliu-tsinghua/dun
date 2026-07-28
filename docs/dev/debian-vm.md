# Debian Measurement VM

The Debian VirtualBox VM `debvbox` is the binding measurement environment for
`dun`: release binary size gates, runtime resource audits, release smoke runs,
and the external SSH / low-capability terminal matrix all record their Debian
results from this machine.

It is the default target of the `vm-test/` wrapper scripts, which support
several local VMs through a target selector (`-t NAME` / `DUN_VM_TARGET`;
default `debian`). The FreeBSD portability VM is the other target today — see
[freebsd-vm.md](./freebsd-vm.md). When a change needs cross-platform testing,
ask the project owner to start every relevant VM (both Debian and FreeBSD
today), then run against each with its target.

## Connection

- VirtualBox VM name: `debvbox` (Debian 13 amd64, system `rustc`/`cargo`
  1.85 from Debian packages).
- Reachable through a NAT port forward on the host: `localhost` port `2222`.
- User: `<user>` — the guest account you created.
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
directory, never from `~/dun-worktree`. The wrappers keep a repo-local,
gitignored `vm-test/known_hosts` (accept-new) because every target shares
`localhost`. The raw connection, should the scripts be unavailable, is
`ssh -i vm-test/dun-vm-test -p 2222 <user>@localhost`; the target's port and
destination can be overridden with `DUN_VM_TARGET` / `DUN_VM_PORT` /
`DUN_VM_DEST`.

## Idle CPU versus clock accuracy

VirtualBox's "Default" paravirtualization interface resolves to `KVM` for a
Linux guest (`Logs/VBox.log`, `GIM: Using provider`). That gives the guest a
usable TSC — the interface reports the TSC frequency, so the kernel does not
have to calibrate it — but on this host an **idle** Debian guest with it
enabled burned close to a full core, against 3–15% for the FreeBSD and Solaris
guests, which get `None`.

If that matters to you, set the interface to **None**:

```sh
VBoxManage modifyvm "Debian 13.5" --paravirt-provider none
```

Idle host CPU for this guest dropped from 67–90% to 7–10%. The cost is the
clock: without the interface the kernel's TSC calibration fails at boot
(`tsc: Marking TSC unstable due to could not calculate TSC khz`) and it falls
back to `acpi_pm`, which is far slower to read. `ntpd` keeps the time correct
regardless, and nothing this VM is used for — release builds and the test
suite — depends on cheap timestamp reads, so the trade is worth taking. The
size measurements do not involve the clock at all.

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
