# Testing Guide

How to run `dun`'s tests, and how to build the parts of the harness that do not
come with the repository. The VM manuals next to this file
([debian-vm.md](./debian-vm.md), [freebsd-vm.md](./freebsd-vm.md),
[solaris-vm.md](./solaris-vm.md)) document connection details and working
conventions for the machines this project already has; this guide is what you
need if you are standing the harness up somewhere else.

- [The layers, and what each one catches](#the-layers-and-what-each-one-catches)
- [Running the everyday suite](#running-the-everyday-suite)
- [PTY tests](#pty-tests)
- [tmux grid tests](#tmux-grid-tests)
- [Building a test VM from scratch](#building-a-test-vm-from-scratch)
- [The vm-test wrappers](#the-vm-test-wrappers)
- [Measuring the binary](#measuring-the-binary)
- [Real-terminal acceptance and screenshots](#real-terminal-acceptance-and-screenshots)
- [Known per-platform quirks](#known-per-platform-quirks)

## The layers, and what each one catches

Five layers, in increasing cost and decreasing determinism. The point of the
list is what each layer **cannot** see, because that is what justifies the next
one.

| Layer | Runs where | Catches | Blind to |
| --- | --- | --- | --- |
| Unit and integration tests | `cargo test`, no terminal | Editor logic, config parsing, layout maths, the protocol client | Anything about a real terminal |
| PTY tests | A pseudo-terminal in-process | Terminal lifecycle, raw mode, escape output, input parsing, shell escape, OSC 52 | Anything about how a *terminal emulator* interprets that output |
| tmux grid tests | A detached `tmux`, output read back as a character grid | What actually lands on screen: geometry, wide glyphs, box drawing, menu layout, plugin windows | Colors as a human sees them, font behavior, emulator policy |
| Cross-platform VM matrix | Debian, FreeBSD, Solaris guests | Portability: libc differences, terminal database differences, ambiguous-width behavior, byte size on the binding platform | The same blind spots as the layer being run |
| Real-terminal acceptance | Human or automation in kitty / iTerm2 / Terminal.app | Emulator policy (clipboard prompts, modifier delivery), font and glyph approximation, visual judgement | Nothing is asserted; it is looking, not testing |

The 2026-07-27 acceptance pass is the argument for the last row: it found five
plugin-UI defects, none of which any automated layer could have caught, because
each was about what a person sees rather than what a byte says.

## Running the everyday suite

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

That is the gate for any change. Everything in it runs on a developer machine
with no VM, no tmux, and no network. Documentation changes additionally need:

```sh
scripts/check-links.py            # every local link and doc path resolves
scripts/check-links.py --strict   # also checks source-path mentions
```

`--strict` reports mentions of source files inside historical documents — the
append-only log and the codex briefs name the tree as it stood the day they
were written — so it is a review aid rather than a gate.

## PTY tests

`crates/dun-cli/tests/pty_smoke.rs` drives `dun` through a real pseudo-terminal:
it starts the editor, feeds bytes, and asserts on what comes back. This is where
terminal lifecycle, the shell escape's suspend/resume, and the OSC 52 clipboard
round trip are covered.

They run as part of `cargo test` and need nothing installed. If one hangs, it is
usually a genuine deadlock in the event loop rather than a flaky test — the
harness has bounded reads throughout.

## tmux grid tests

`tmux_grid.rs`, `terminal_grid.rs`, and `tmux_logfilter.rs` start `dun` inside a
detached `tmux` session, send keystrokes with `tmux send-keys`, and read the
screen back with `tmux capture-pane`, normalising it into a character grid the
assertions run against.

**Requirement: `tmux` on `PATH`.** No GUI terminal is involved and no display is
needed; this works over SSH and in a headless VM.

```sh
cargo test -p dun-cli --test tmux_grid
cargo test -p dun-cli --test tmux_logfilter
```

Why `tmux` rather than a bespoke emulator: it is the terminal these tests are
about — it is what people actually run `dun` inside on a server — and its
`capture-pane` gives a stable, scriptable view of the final rendered cells,
including the wide-character decisions that a byte-level assertion cannot see.
The rationale in full is in
[real-terminal-tui-testing.md](./real-terminal-tui-testing.md).

## Building a test VM from scratch

The project uses local VirtualBox guests reached over SSH on forwarded ports.
Nothing about that is special to VirtualBox — any hypervisor with NAT port
forwarding works — but the wrappers assume the port convention below.

**1. Create the guest.** Install the OS normally. Give it enough disk for a Rust
toolchain and a release build (16 GB is comfortable; the `target/` directory
dominates).

**2. Network: NAT plus a port forward.** Leave the adapter on NAT and forward a
host port to guest port 22. The wrappers know these ports:

| Target | Host port | Guest |
| --- | --- | --- |
| `debian` | 2222 | Debian 13 amd64 — the binding size-measurement platform |
| `freebsd` | 2233 | FreeBSD 15.1 amd64 — portability only |
| `solaris` | 2244 | Oracle Solaris 11.4 — portability only |

In the VirtualBox UI this is Settings → Network → Advanced → Port Forwarding;
on the command line:

```sh
VBoxManage modifyvm <vm-name> --natpf1 "ssh,tcp,127.0.0.1,2222,,22"
```

Bind the host side to `127.0.0.1`, not `0.0.0.0`: this is a passwordless-sudo
development guest and it should not be reachable from your network.

**3. SSH in, with a dedicated key.** Generate a keypair used for nothing else
and install it in the guest:

```sh
ssh-keygen -t ed25519 -f vm-test/dun-vm-test -N '' -C dun-vm-test
ssh-copy-id -i vm-test/dun-vm-test.pub -p 2222 <user>@localhost
```

`vm-test/` is gitignored apart from the three wrapper scripts, so the private
key stays out of the repository. Keep it that way.

**4. Passwordless sudo**, so a test run can install a missing package without
an interactive prompt:

```sh
echo '<user> ALL=(ALL) NOPASSWD:ALL' | sudo tee /etc/sudoers.d/dun-test
```

**5. Toolchain.** Rust `1.85` or newer *plus the `rust-src` component* — the
size-budget build rebuilds the standard library and cannot work without it —
and `tmux`, `rsync`, and `git`:

```sh
# Debian 13 — rustc/cargo 1.85, rust-src lands in /usr/lib/rustlib/src/rust/
sudo apt install rustc cargo rust-src tmux rsync git

# FreeBSD 15 — one `rust` package carries the toolchain
sudo pkg install rust tmux rsync git

# Solaris 11.4 — IPS names are pathed
sudo pkg install developer/rust/rustc developer/rust/cargo \
                 terminal/tmux network/rsync developer/versioning/git
```

`rust-src` only matters where you intend to run `scripts/release-build.sh` —
that is Debian and macOS, the two size-budget platforms. FreeBSD and Solaris
are portability guests and run ordinary `cargo build`, so a missing `rust-src`
there costs you nothing. On Solaris the component is not where `cargo` expects
it and has to be linked from `/opt`; the recipe for the existing guest is in
[solaris-vm.md](./solaris-vm.md).

The guests these instructions were checked against carry `rustc` 1.85 on
Debian, 1.96 on FreeBSD, and 1.87 on Solaris — the floor is 1.85 and newer is
fine.

**6. Verify.**

```sh
vm-test/vm-run -t debian 'rustc --version && tmux -V && sudo whoami'
```

## The vm-test wrappers

Three tracked scripts share their connection settings through
`vm-test/vm-common.sh`, so no raw `ssh` invocation has to be remembered:

```sh
vm-test/vm-run                            # interactive shell on the default target
vm-test/vm-run -t freebsd sudo whoami     # one command on a chosen target
vm-test/vm-run 'cd ~/dun-abc1234 && cargo test --workspace'

vm-test/vm-sync                           # clean git archive of HEAD -> ~/dun-<shortsha>
vm-test/vm-sync v0.1.0                    # ... of a named ref
vm-test/vm-sync --worktree                # rsync the dirty tree -> ~/dun-worktree
```

Target selection is `-t NAME` or `DUN_VM_TARGET`; the default is `debian`. Host
keys go in a repo-local, gitignored `vm-test/known_hosts`, so a rebuilt guest
does not touch your personal `~/.ssh/known_hosts`.

`vm-sync` without `--worktree` syncs a **clean git archive**, which is what a
measurement must use — an uncommitted file changes the binary and invalidates
the number. `--worktree` exists for iteration only.

## Measuring the binary

The 1 MiB budget is measured on Debian x86-64 (binding) and macOS x86-64, using
the build-std contract rather than a plain release build:

```sh
scripts/release-build.sh
```

It sets `RUSTC_BOOTSTRAP=1` and passes `-Zbuild-std=std,panic_abort`
`-Zbuild-std-features=` on a stable toolchain, which is why `rust-src` is
required. A plain `cargo build --release` is a development build and its size
means nothing against the budget.

The repeatable procedure — clean git archive, locked build, `stat`, `file`,
`ldd`, and a `--version` / `--dump-config` smoke — is in
[release-size-audit.md](./release-size-audit.md), together with every
measurement this project has taken. Record new ones there; a size claim with no
recorded measurement behind it is how a budget quietly stops binding.

## Real-terminal acceptance and screenshots

The checklist is [real-terminal-acceptance.md](./real-terminal-acceptance.md).
It is deliberately **not** a CI gate: it covers what the automated layers
cannot observe, and it ends in human judgement.

`acceptance/launch.sh` starts `dun` deterministically for a checklist item. It
writes a throwaway config to a scratch directory and points `dun` at it through
`DUN_CONFIG`, so your real `~/.config/dun` is never touched, and it copies the
`i18n/` catalogs next to that config — without which every launch would be
silently English.

```sh
acceptance/launch.sh                          # default theme, the fixture file
acceptance/launch.sh msedit --mouse
acceptance/launch.sh dun --ascii              # ASCII fallback
acceptance/launch.sh dun --file README.md     # real content instead of the fixture
LC_ALL=ja_JP.UTF-8 acceptance/launch.sh       # language comes from the environment
acceptance/launch.sh dun --syntax pygments    # with a highlight host
acceptance/launch.sh dun --logfilter python   # with the log-filter host
```

Three headless sweeps produce text grids rather than screenshots, and need only
`tmux`:

```sh
acceptance/sweep-states.sh      # dialog and editor states across languages
acceptance/sweep-menus.sh       # every menu, opened
acceptance/sweep-logfilter.sh   # the log-filter plugin's full layout
```

`sweep-logfilter.sh` is the one worth knowing about: it is the only tool that
captures a plugin's complete UI — its injected menu, its two plugin-owned
windows, and an editor split alongside them — and it does so in a detached
`tmux` at 100x30 with no GUI terminal involved. Two plugin windows plus a split
do not fit in 80 columns, which is why it uses a wider geometry than the rest of
the harness. Its chords come from `tmux_logfilter.rs`, which is the authority
for them.

Screenshots are a different matter. `gallery-run.sh`, `gallery-open.sh`, and
`gallery-ssh.sh` drive a real GUI terminal, and a screenshot needs a human or a
desktop-automation tool to press the shutter — there is no headless path. The
gallery directory is gitignored; curate deliberately before committing any
image.

## Known per-platform quirks

- **Do not enable VBoxService on the Solaris guest.** Its time sync *causes*
  clock drift there rather than correcting it: the same 90-second CPU load
  moves the clock by eleven seconds with VBoxService running and by less than
  0.15 s without it, and the drift reproduces on re-enabling. Solaris runs
  `ntpd` instead. Measured four ways in
  [solaris-vm.md](./solaris-vm.md#do-not-run-vboxservice-on-this-guest).
  Debian and FreeBSD run neither VBoxService nor Guest Additions time sync and
  show no drift; Debian additionally gets a paravirtualized clock because
  VirtualBox's "Default" paravirtualization interface resolves to `KVM` for a
  Linux guest and to `None` for FreeBSD and Solaris (check any VM's
  `Logs/VBox.log` for `GIM: Using provider`).
- **Solaris renders ambiguous-width glyphs double-wide** under `tmux` — box
  drawing and `◆` included. This is a real property of that terminal, not a
  `dun` defect. `dun` probes for it at startup and adapts; if the probe cannot
  run, `terminal.ambiguous-width = wide` or `terminal.encoding = ascii` is the
  override.
- **Solaris command flags differ.** `grep -E`, `tail -n`, `stat -c`, and `tar`
  all behave differently from GNU. Prefer portable forms in anything that runs
  on all three guests.
- **FreeBSD ships `/usr/bin/edit`**, which is `ee`, not Microsoft Edit. The
  reference tests that look for an `edit --help` contract skip accordingly.
- **`vm-sync --worktree` needs `rsync` in the guest.** It is not installed by
  default on FreeBSD or Solaris.
