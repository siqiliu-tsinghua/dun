# Contributing

Thanks for looking. `dun` is small on purpose, and most of the rules below
exist to keep it that way.

## Before you start

Read [AGENTS.md](AGENTS.md). It carries the project invariants — the ones a
change must not break — and it is short. [CLAUDE.md](CLAUDE.md) has the
current working plan and the size baseline.

## The gate

Every change must pass:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Documentation changes also need `scripts/check-links.py` to report zero broken
targets. Changes that touch runtime code need the release smoke checklist and
the size gate as well — see below.

[docs/dev/testing-guide.md](docs/dev/testing-guide.md) explains the five test
layers, what each one can and cannot catch, and how to stand up the VM and
tmux harnesses from scratch if you want to run the cross-platform matrix.

## The constraints that will surprise you

**The binary must stay under 1 MiB** on macOS and Debian x86-64, measured with
`scripts/release-build.sh`. This is not a guideline. A feature that costs
measurable bytes needs to justify them against
[docs/dev/feature-budget.md](docs/dev/feature-budget.md), and "measure per
batch" is the rule — with `opt-level = "z"` and fat LTO, size deltas are not
additive.

**The shell scripts are POSIX `sh`.** `scripts/build.sh`, `install.sh`,
`uninstall.sh` and `release-build.sh` run on all four supported platforms, so
no `bash` (FreeBSD's base system has none), no `local` (Solaris `/bin/sh` is
ksh93), no GNU-only flags, and no `tar -z` (Solaris `tar` has neither). The
install scripts also hold one behavioural rule: decide everything, print the
plan, confirm once, and only then write — an interrupted run must leave the
machine as it was.

**Safe Rust only.** Every crate root and test entry point carries
`#![forbid(unsafe_code)]`. Adding `unsafe` requires an explicit design
decision, not a pull request that happens to contain it.

**No new dependencies without a measurement.** The tree has three external
direct dependencies. Terminal I/O and rendering are in-house; do not
reintroduce a framework to solve a rendering or input problem.

**Documentation is part of the change.** A behaviour change that does not
update the matching document is incomplete. AGENTS.md lists which document owns
what. Feature removals are full-trail: code, command ids, keymap defaults, menu
entries, help text, tests, and the prose that describes them.

**Untrusted text stays sanitized.** Anything derived from a file, a file name,
a command's output, or a plugin must reach the display sanitizer before it
reaches the terminal. [docs/dev/AUDIT.md](docs/dev/AUDIT.md) is the security
boundary and maps each invariant to the test that carries it.

## Tests that can fail

A test guarding a correctness or safety invariant should be proven
load-bearing: break the implementation on purpose and confirm the test fails.
A guard that passes against a broken implementation is worse than no guard,
because it is trusted. If you add one, say in the pull request how you proved
it.

## Translations

Corrections to any `i18n/*.conf` catalog are welcome and uncontroversial —
nine of the ten are machine-translated and unreviewed, and
[docs/i18n.md](docs/i18n.md) says so plainly rather than implying review is
pending. The mechanical gates (key completeness, placeholder shape, column
overflow) will catch a structurally broken file; wording is what needs a human.

## Plugins

You do not need to change `dun` to write a plugin — hosts are separate
programs. See [docs/plugin-authoring.md](docs/plugin-authoring.md).
