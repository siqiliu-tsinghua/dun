# Release Smoke Checklist

This checklist is the lightweight pre-release pass for the current Rust-owned
editor line. It is not a substitute for the broader manual terminal matrix, but
it gives each release candidate a bounded automated gate.

## Automated Gate

Run these from the repository root:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p dun-cli --test tmux_grid
cargo test -p dun-cli --test msedit_diff
cargo build --release --locked -p dun-cli
target/release/dun --version
target/release/dun --dump-config
```

Expected results:

- formatting has no changes to apply;
- clippy reports no warnings;
- the workspace test suite passes;
- tmux-backed tests pass when `tmux` is installed and skip cleanly otherwise;
- Microsoft Edit differential tests pass when `edit` is installed and skip
  cleanly otherwise;
- the release binary builds with locked dependencies;
- the release binary is no larger than `1,048,576` bytes on both audited macOS
  and Debian builds, using the checked-in release profile;
- `--version` exits successfully and prints the package version;
- `--dump-config` exits successfully and prints parseable default
  configuration.

Measure the release binary with `stat -f%z target/release/dun` on macOS and
`stat -c%s target/release/dun` on Debian/Linux. If either platform exceeds the
limit, follow the trim order in
[feature-budget.md](./feature-budget.md) before adding any features.

## Size And Resource Drift

Repeat the release size and runtime-resource audits when any of these change:

- dependency set or enabled features;
- release profile settings;
- terminal backend, command-output capture, file I/O, or plugin-adapter shape;
- anything expected to affect startup memory or binary size.

Record updated results in
[release-size-audit.md](./release-size-audit.md) and
[runtime-resource-audit.md](./runtime-resource-audit.md).

## Terminal Matrix

Before a tagged release, record the external SSH and low-capability terminal
matrix in [terminal-compatibility-checks.md](./terminal-compatibility-checks.md).
The automated PTY, tmux, and Microsoft Edit tests are the default local smoke
path; real external terminal results remain the release signoff path. Release
notes should claim only the terminal paths recorded in that matrix.

## Completion Criteria

This smoke pass is complete when the automated gate has passed or documented
clean skips for optional tools, any required size/resource audits are updated,
and the external terminal matrix has either current results or an explicit
release-blocking note.
