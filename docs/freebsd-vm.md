# FreeBSD Test VM

The FreeBSD VirtualBox VM is a portability test environment for `dun`: it runs
the full test suite (unit, protocol, tmux-backed terminal grid, PTY smoke) on a
non-Linux, LLVM-native Unix so cross-platform assumptions are exercised.

It is a **functional / portability** environment, not a size-budget platform.
The binding binary-size budget stays macOS + Debian (see
[release-size-audit.md](./release-size-audit.md)); a FreeBSD release binary is
only a reference point, not a gate — its toolchain (LLVM/lld, FreeBSD libc) and
`pkg` Rust version differ from the budget baseline.

## Connection

- VirtualBox VM: FreeBSD 15.1-RELEASE amd64.
- Reachable through a NAT port forward on the host: `localhost` port `2233`.
- User: `fft` (same as the macOS host user), with passwordless `sudo`.
- SSH keypair: the same `vm-test/dun-vm-test` used for the Debian VM (both are
  local, user-provisioned guests). The directory is gitignored; the private key
  is never committed.
- The VM is started manually from the VirtualBox UI by the project owner; ask
  for it to be started before a test run.

Use the tracked wrapper scripts with the `freebsd` target (they share their
connection settings through `vm-test/vm-common.sh`, which keeps a repo-local,
gitignored `vm-test/known_hosts` and accepts new keys — several VMs share
`localhost`, which would otherwise collide in the global known_hosts):

```text
# one-off command / interactive shell on FreeBSD
vm-test/vm-run -t freebsd sudo whoami
vm-test/vm-run -t freebsd 'cd ~/dun-abc1234 && cargo test --workspace'

# clean git-archive sync of a commit (default HEAD) -> ~/dun-<shortsha>
vm-test/vm-sync -t freebsd
```

`-t freebsd` can also be given as `DUN_VM_TARGET=freebsd`. The raw connection,
should the scripts be unavailable, is
`ssh -i vm-test/dun-vm-test -p 2233 fft@localhost` (add
`-o StrictHostKeyChecking=accept-new` on first use).

## Toolchain (install with pkg)

The base system ships `clang`/`ld.lld` and `pkg`; the rest is installed with
`pkg install` (the first run bootstraps pkg and fetches the ports catalogue —
it is slow, so install asynchronously and poll):

```text
sudo pkg install -y rust    # rustc + cargo (1.96.x); also ships rust-src
sudo pkg install -y tmux    # for the tmux_grid tests
```

- `git` is **not** needed on the VM: `vm-test/vm-sync` streams a host-side
  `git archive` through the VM's base `tar`.
- The `rust` package bundles `cargo` and installs `rust-src`
  (`/usr/local/lib/rustlib/src`), so `-Zbuild-std` works — but see the caveat
  below before running the size build.
- Run the suite with a UTF-8 locale: `env LANG=en_US.UTF-8 cargo test
  --workspace`. Without it FreeBSD defaults to `C`, and `dun` falls back to
  ASCII/English (by design).

## FreeBSD-specific gotchas

- **`/usr/bin/edit` is `ee` (easy editor), not Microsoft Edit.** The Microsoft
  Edit differential/reference tests gate on `microsoft_edit_on_path`, which
  verifies the `edit --help` signature, so they skip cleanly here instead of
  driving `ee`. Do not "fix" a skip by pointing them at `/usr/bin/edit`.
- **`scripts/release-build.sh` does not run as-is on FreeBSD:** its
  `#!/usr/bin/env bash` needs `bash` (a port, not base) and its size check uses
  GNU `stat -c%s`. For a reference size, run the build-std line directly under
  `sh` and use BSD `stat`:

  ```text
  triple=$(rustc -vV | sed -n 's/^host: //p')
  env RUSTC_BOOTSTRAP=1 cargo build --release --locked -p dun-cli \
      -Zbuild-std=std,panic_abort -Zbuild-std-features= \
      --target "$triple" --manifest-path "$PWD/Cargo.toml"
  stat -f%z "target/$triple/release/dun"
  ```

## Working Conventions

- Build from a clean `git archive` copy (`vm-test/vm-sync`), not a live
  checkout, so the tested source state is exactly one commit.
- Use one working directory per task and delete it after recording results; the
  repository is the record, the VM is disposable scratch space.
- Do not develop on the VM or leave uncommitted work there.
- The keypair grants access to local test VMs only; do not reuse it elsewhere.
