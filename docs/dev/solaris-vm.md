# Solaris Test VM

The Solaris VirtualBox VM is a second portability test environment for `dun`
(alongside [freebsd-vm.md](./freebsd-vm.md)): it exercises `dun` on Oracle
Solaris — a non-Linux Unix with a native (non-GNU, non-LLVM) link editor and a
multilib layout — so cross-platform assumptions are stressed further.

It is a **portability / functional** environment, not a size-budget platform.
The binding binary-size budget stays macOS + Debian
([release-size-audit.md](./release-size-audit.md)); a Solaris release binary is
a reference point only (its toolchain and `pkg` Rust version differ from the
budget baseline).

## Connection

- VirtualBox VM: Oracle Solaris 11.4 (SunOS 5.11), x86-64.
- Reachable through a NAT port forward on the host: `localhost` port `2244`.
- User: `<user>` — the guest account you created, with passwordless `sudo`.
- SSH keypair: the same `vm-test/dun-vm-test` used for the other VMs.
- The VM is started manually by the project owner; ask before a test run.

Use the tracked wrappers with the `solaris` target:

```text
vm-test/vm-run -t solaris uname -a
vm-test/vm-sync -t solaris            # clean git-archive of HEAD -> ~/dun-<sha>
```

`vm-sync` extracts with `cd … && tar -xf -` so it works with Solaris `tar`
(which reads a tape device without `-f -`, and whose `-C` differs from GNU/BSD).
Solaris `tar` also warns `pax_global_header: typeflag 'g' not recognized` and
drops a harmless stray file at the archive root — cargo ignores it.

## Toolchain (installed via `pkg`)

Already installed on the VM: `developer/rust/rustc` and `developer/rust/cargo`
(**1.87.0**), `gcc` (no `cc` alias), the native Solaris `ld`, `bash`, `gmake`,
`tmux` (3.4), `gtar`, `curl`/`wget`/`xz`. `git` is absent — not needed, since
`vm-sync` streams a host-side `git archive` through the VM's `tar`.

- `rustc --print sysroot` is `/usr`, but the actual rustlib is under
  `/usr/lib/amd64/rustlib` (multilib) — `rustc --print target-libdir` is
  `/usr/lib/amd64/rustlib/x86_64-pc-solaris/lib`.
- The system rust is *not* a Solaris fork: `rustc -Vv` reports
  `commit-hash: 17067e9ac6d7ecb70e50f92c1944e545188d2359`, identical to upstream
  1.87.0, so upstream `rust-src-1.87.0` matches it exactly.

### rust-src for build-std (`/opt` + a symlink)

The `pkg` rust ships no `rust-src`, and cargo's `-Zbuild-std` reads the std
source only from `$(rustc --print sysroot)/lib/rustlib/src/rust` with no env
override. Keep the source out of the system tree by putting it in `/opt` and
linking to it (the bulky source lives in `/opt`; `/usr` gets one symlink):

```text
# download + verify against the official checksum
curl -fsSLO https://static.rust-lang.org/dist/rust-src-1.87.0.tar.xz
curl -fsSLO https://static.rust-lang.org/dist/rust-src-1.87.0.tar.xz.sha256
openssl dgst -sha256 rust-src-1.87.0.tar.xz   # must match the .sha256 file

sudo gtar -xJf rust-src-1.87.0.tar.xz -C /opt
sudo mkdir -p /usr/lib/rustlib/src
sudo ln -s /opt/rust-src-1.87.0/rust-src/lib/rustlib/src/rust /usr/lib/rustlib/src/rust
```

`scripts/release-build.sh` does not run as-is on Solaris (its size check uses
GNU `stat -c%s`). Run the build-std line directly and size with `wc -c`:

```text
triple=$(rustc -vV | sed -n 's/^host: //p')   # x86_64-pc-solaris
env RUSTC_BOOTSTRAP=1 cargo build --release --locked -p dun-cli \
    -Zbuild-std=std,panic_abort -Zbuild-std-features= \
    --target "$triple" --manifest-path "$PWD/Cargo.toml"
wc -c < "target/$triple/release/dun"
```

Reference size (2026-07-19, HEAD `22b2dd4`): **1,050,560 bytes** — larger than
the other reference platforms (FreeBSD 655,704, macOS 678,708); attributed to
the Solaris `ld`/runtime, not a budget concern (Solaris is not a budget
platform).

## Solaris-specific findings

- **`dun` full test suite: 689 pass, 4 fail** (2026-07-19, `22b2dd4`) with
  `LANG=en_US.UTF-8`. The product compiles and links cleanly (crossterm 0.28.1
  builds against the native `ld`); all unit, protocol, PTY, plugin, and
  surface-write tests pass.
  (As of `bf48733` the suite is **709 pass, 6 fail**: the four `tmux_grid` tests
  plus two `tmux_logfilter` tests — see below.)
- **KNOWN ISSUE — the tmux failures are the same root cause. Root-caused
  2026-07-19: Solaris tmux 3.4 (via Solaris libc `wcwidth`) renders Unicode
  *ambiguous-width* characters as double-width** — the box-drawing block
  (U+2500 `─`, U+2502 `│`, corners) and geometric symbols (U+25C6 `◆`) each
  occupy 2 cells, where `unicode-width` (which `dun` uses) and every other
  tested terminal (Linux/FreeBSD/macOS) treat them as width 1. So `dun` emits a
  provably correct 80-column frame, tmux receives it byte-for-byte (verified
  with `pipe-pane`), but each box-drawing glyph consumes two columns, overflowing
  the row and clipping the visible border to ~46 columns.

  This is **not a `dun` code defect** — `dun`'s output is correct by Unicode's
  default (Western) width, and size detection is fine (`crossterm::size()`
  returns 80, verified). It is a Solaris terminal `wcwidth` policy (Ambiguous →
  wide), analogous in spirit to the FreeBSD `edit`=`ee` gotcha. Diagnosis method
  (for re-verification): a pane running `printf '\033[1;1H──────────'` leaves
  `#{cursor_x}` at 20, not 10.

  **Workaround (verified):** force `dun`'s ASCII border glyphs, which are
  width-1 everywhere — put `terminal.encoding = ascii` in the config (or run
  `dun --config <file>` with that line). The border then renders full-width
  (`+-- * Untitled --…--+`). Do not mask the tmux failures by skipping; they
  correctly report the platform's ambiguous-width policy.
- **The two `tmux_logfilter` failures (`execute`, `filters_command_output`) are
  the same quirk.** Those two assert on body content read across three tiled
  plugin windows, which the double-width box glyphs truncate; the other two
  `tmux_logfilter` tests (`menu`, `keybinding`→scratch) are width-insensitive
  (menu bar / short window title) and pass. Solaris has `/usr/bin/python3`
  (→3.11), so the log-filter host launches and the tests run rather than
  skip. FreeBSD and macOS pass all four.
- There is no `/usr/bin/edit` on Solaris, so the Microsoft Edit tests skip
  cleanly (they gate on `microsoft_edit_on_path`).

## Clock: enable NTP, do not install the Guest Additions

VirtualBox's "Default" paravirtualization interface resolves to **None** for a
Solaris guest — check any VM's `Logs/VBox.log` for `GIM: Using provider` — so
the guest gets no paravirtualized clock and its time can drift. Run `ntpd`:
this guest has `/etc/inet/ntp.conf` pointing at `pool.ntp.org`, which the
Solaris installer never offers to set up.

Do **not** install the VirtualBox Guest Additions for time sync. Measured on
2026-07-28, its `VBoxService` made the clock *worse* under sustained CPU load —
losing about eleven seconds where the same load with `ntpd` alone moved it by
under 0.15 s — and its kernel modules are unsigned as far as Solaris is
concerned, so every boot logs two signature warnings.

## Working Conventions

Same as the other VMs: build from a clean `vm-sync` archive, one working
directory per task, delete scratch after recording results, do not develop on
the VM. The keypair grants access to local test VMs only.
