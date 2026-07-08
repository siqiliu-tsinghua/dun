# Runtime Resource Audit

This document records lightweight startup and memory checks for `dun`.
Results are local baselines for regression spotting, not cross-machine
performance claims.

## Method

The 2026-07-08 checks used the existing size-oriented release binaries from
[release-size-audit.md](./release-size-audit.md):

- macOS binary: `target/size-audit/macos-size/release/dun`;
- Debian binary:
  `/tmp/dun-size-audit/target/size-audit/linux-size/release/dun`.

Each TUI run was launched in an `expect` pseudo-terminal with an 80x24
`xterm-256color`-style screen. The script waited for a visible startup marker,
sampled the child process RSS with `ps -o rss= -p PID`, then sent `Ctrl+Q`.

The medium fixture was about 1.1 MiB of repeated text. The large fixture was a
17 MiB byte file, intentionally above the current 16 MiB editable-file soft
limit. The large-file path exits before the normal TUI is entered, so only
elapsed rejection time and exit code are recorded.

Command Output RSS was measured after a short output pane appeared. Its elapsed
time includes scripted prompt/input overhead and is not used as a startup
baseline.

RSS values are operating-system-specific. Compare macOS to macOS and Linux to
Linux; do not treat the two RSS columns as the same accounting model.

## 2026-07-08 Baseline

Code baseline: `4d89d07`. Later commit `2dbfdca` changed only audit and policy
documentation, so the measured executable code is still the same baseline.

macOS host:

```text
rustc 1.85.0 (4d91de4e4 2025-02-17)
cargo 1.85.0 (d73d2caf9 2024-12-31)
Darwin fftmac.local 25.5.0 x86_64
```

Debian VM:

```text
rustc 1.85.0 (4d91de4e4 2025-02-17) (built from a source tarball)
cargo 1.85.0 (d73d2caf9 2024-12-31)
Linux debvbox 6.12.95+deb13-amd64 x86_64
```

| Platform | Scenario | Startup or elapsed | RSS |
| --- | --- | ---: | ---: |
| macOS x86_64 | empty TUI startup | 27 ms | 1,328 KiB |
| macOS x86_64 | open 1.1 MiB UTF-8 file | 47 ms | 5,016 KiB |
| macOS x86_64 | reject 17 MiB over-limit file | 23 ms, exit 1 | not sampled |
| macOS x86_64 | Command Output pane after short command | elapsed excluded | 1,968 KiB |
| Debian x86_64 | empty TUI startup | 15 ms | 2,872 KiB |
| Debian x86_64 | open 1.1 MiB UTF-8 file | 32 ms | 6,500 KiB |
| Debian x86_64 | reject 17 MiB over-limit file | 16 ms, exit 1 | not sampled |
| Debian x86_64 | Command Output pane after short command | elapsed excluded | 3,116 KiB |

## Interpretation

The current editor baseline starts quickly and stays in the low-megabyte RSS
range for empty editing and small-to-medium text files. Opening a 1.1 MiB file
adds a few MiB of RSS, which is expected because the editor currently owns
UTF-8 text, display metadata, search state, and undo-capable buffer state in
Rust.

Large files above the editable soft limit are rejected before becoming editor
buffers. That keeps accidental large log opens from becoming an unbounded
memory path in the first editor line.

Future checks should repeat this audit after changes to:

- file loading or buffer storage;
- undo/redo transaction storage;
- soft-wrap or display caching;
- terminal backend features;
- dependency feature sets;
- future optional plugin/runtime adapters.

