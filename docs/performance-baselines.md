# Performance Baselines

`dun` keeps large editable files conservative: files above
`limits.editable_file_soft_limit_bytes` are rejected before they become normal
editable buffers. The baseline here measures the current in-memory editor path
below that limit so future changes can be compared against a repeatable local
reference.

These are not release claims. They are machine-local regression baselines.

## Run

Default test runs compile the performance tests but skip them:

```text
cargo test -p dun-cli
```

Run the local baseline explicitly:

```text
cargo test -p dun-cli --release large_file_perf -- --ignored --nocapture
```

The default fixture is about 8 MiB plus a 512 KiB single-line case. Override the
sizes when testing closer to the editable soft limit:

```text
DUN_PERF_LARGE_FILE_BYTES=16777216 \
DUN_PERF_LONG_LINE_BYTES=1048576 \
cargo test -p dun-cli --release large_file_perf -- --ignored --nocapture
```

## Covered Paths

The ignored `large_file_perf_*` tests cover:

- startup file open through the same loader used by the CLI;
- `TextBuffer::find_all` for sparse matches and missing matches;
- cursor movement to the end of a large buffer and scroll synchronization;
- UI frame construction for a visible editor window;
- ratatui drawing through `TestBackend`;
- long-line rendering with `limits.line_display_soft_limit_bytes` enforced.

The tests assert functional invariants but do not enforce timing thresholds.
Timing gates can be added later once representative low-end SSH hosts are part
of the release matrix.

## Current Local Sample

Generated on 2026-07-05 with Rust 1.85 release test profile in the current
development workspace:

```text
large_file_perf ui_frame_long_line_display_cap: 0 ms
large_file_perf ratatui_draw_long_line_display_cap: 0 ms
large_file_perf fixture: bytes=8388643 lines=121506 error_lines=473
large_file_perf startup_open: 22 ms
large_file_perf find_all_sparse_match: 7 ms
large_file_perf find_all_missing_match: 12 ms
large_file_perf sync_view_to_eof: 0 ms
large_file_perf ui_frame_visible_window: 3 ms
large_file_perf ratatui_draw_visible_window: 0 ms
```

When comparing future results, rerun the same command on the same host after a
clean build or after warming the build cache consistently. Do not compare these
numbers directly across unrelated machines.
