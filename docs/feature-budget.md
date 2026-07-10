# Feature Budget

This document is the hard runtime-size gate for `dun` v0.1. It exists so
feature decisions are made by visible rules instead of taste.

## Hard Budget

The final release executable must be no larger than 1 MiB on both audited
platforms:

```text
maximum size: 1,048,576 bytes
platforms: macOS x86_64 and Debian x86_64
binary: target/release/dun
build command: cargo build --release --locked -p dun-cli
```

The budget applies to the uncompressed executable. It does not count source,
tests, documents, `target/` intermediates, local reference checkouts, external
plugin host executables, future optional runtime packages, or bundled example
plugins that are not linked into `target/release/dun`.

The checked-in `[profile.release]` is the release-size profile. Do not use a
different profile to claim v0.1 budget compliance.

Measure with:

```text
# macOS
stat -f%z target/release/dun

# Debian/Linux
stat -c%s target/release/dun
```

## Budget Failure Rule

If either audited platform is above `1,048,576` bytes:

1. Do not add features.
2. Confirm the checked-in release profile was used.
3. Rebuild from a clean enough release target if the result is surprising.
4. Remove optional runtime features in the trim order below.
5. Re-run tests and both platform size measurements after each trim.
6. Stop trimming only when both platforms are within budget.

If all optional runtime features have been trimmed and the binary is still too
large, stop and rewrite the v0.1 product scope. Do not silently cut required
features.

## Required Runtime Features

These features define v0.1. They are not eligible for size trimming unless the
product scope is explicitly rewritten.

| Feature group | Required behavior |
| --- | --- |
| Terminal lifecycle | Enter and restore raw mode, alternate screen, cursor state, mouse/bracketed-paste state, and exit codes predictably. |
| Safe display | Sanitize buffer text, titles, status text, controls, escape sequences, and long lines before rendering. |
| Terminal fallback | Support UTF-8/ASCII glyph profiles and 256-color/16-color/mono color profiles, including strict 16-color SGR output. |
| Microsoft Edit baseline chrome | Render menu bar, editor body, window borders, status fields, and the default `msedit` theme. |
| Core text buffer | UTF-8-safe cursor movement, selection, insert, delete, newline, dirty tracking, line endings, undo, and redo. |
| File open/save | Open UTF-8 files, Save, Save As, explicit Reload, atomic same-directory writes, stale temp cleanup, recovery diagnostics, and external-change overwrite refusal. |
| Unsafe file handling | Invalid-byte read-only fallback, large-file soft limit, unstable-read rejection, and clear path diagnostics. |
| Unsaved-change safety | Confirm quit/new/open/close/reload paths that would discard dirty buffer state. |
| Search/edit navigation | Find, replace, go to line, page movement, document start/end, word movement/delete, and current selection rendering. |
| Tiling workspace | Split, focus, resize, close, collapse/expand, equalize, rotate, and small-pane degradation. |
| Configured keys | Rust-owned config loading, typed command ids, keymap validation, terminal/theme/mouse/clipboard/limit config, and `--dump-config`. |
| Basic file dialog and prompts | Prompt editing, Open/Save As path entry, Tab completion, keyboard list selection, and overwrite confirmation. |
| Internal clipboard | Cut, copy, and paste through a process-local clipboard without requiring an OS clipboard. |
| Shell escape | Suspend the TUI, run the user's shell, and resume without embedding a terminal emulator. |
| CLI contract | `--help`, `--version`, `--config`, `--no-config`, `--dump-config`, stable usage/runtime exit codes. |
| Plugin protocol client | Host-neutral framed-stdio protocol client, role and policy model, bounded snapshots, output validation, timeout/cancel/crash handling, stale revision rejection, and fixture-host test path. This excludes the `rum` runtime and other external hosts. |

The plugin protocol client is required even though actual plugin runtimes are
optional. If the required client causes the 1 MiB gate to fail, trim optional
editor features in the order below before cutting the client.

## Optional Runtime Trim Order

These features are useful, implemented, and currently kept only while both
audited release binaries stay within 1 MiB. Lower trim numbers are removed
first when the budget is exceeded.

| Trim | Optional feature | Keep only if budget allows |
| ---: | --- | --- |
| 1 | Extra built-in themes | Keep `turbo`, `dark`, and `dun`; retain `msedit` and fallback colors if trimmed. |
| 2 | Optional mouse interaction | Mouse focus, cursor placement, selection drag, split drag, menu clicks, scrollbar drag, and file-dialog mouse clicks. |
| 3 | Editor convenience markers | Nonessential line commands beyond core cut/copy/paste/editing. Bookmarks and visible-whitespace markers were removed 2026-07-10 in the slimming stage (feature-triage F12/F13). |
| 4 | Read-only helper panes | Search Results, helper-pane row navigation, and source-return polish. Core Find/Replace stays required. Outline was removed 2026-07-10 in the slimming stage (feature-triage F20). |
| 5 | Status History pane | In-editor status/error history screen. Current status line stays required. |
| 6 | Config Diagnostics pane | In-editor config diagnostics, section jumps, and keymap summaries. Config loading/validation stays required. |
| 7 | Buffer switcher overlay | Switch Buffer UI. Tiled windows and loaded buffers remain, but direct switcher polish may be cut. |
| 8 | Advanced file-dialog polish | Hidden-file toggle, parent-directory row, scroll overflow indicators, recent directory memory, mouse wheel, and larger visual polish. Basic Open/Save As path entry stays required. |
| 9 | Soft-wrap visual-row model | Display-layer word wrap and visual-row paging. Horizontal scrolling and safe long-line display stay required. |
| 10 | Advanced Command Output polish | Removed 2026-07-10 in the slimming stage (feature-triage F46); the base Run Command output pane stays under trim 11. |
| 11 | One-shot Run Command capture | `run` prompt and bounded captured output pane. Shell escape stays required. |
| 12 | Runtime config/theme convenience | `reload-config`, runtime `theme`, and live refresh of helper screens. Startup config and `--dump-config` stay required. |
| 13 | Help/key reference pane | In-editor Help window. The CLI `--help` output and documented default config stay required. |

Tests, documentation, and local reference/differential harnesses do not count
toward runtime size unless they add runtime dependencies or code to the final
binary.

External plugin hosts, including future `dun-rum-host`, are separately sized
artifacts. They may have their own size budgets later, but they must not be
linked into the default `dun` executable.

## Change Control

A new runtime feature may enter the active plan only when all of these are
true:

1. The feature is classified as required or assigned an optional trim position.
2. Its acceptance behavior is written down before implementation.
3. Its automated test path is written down before implementation, or it is
   explicitly marked as a manual release-matrix item.
4. The release binary still passes the 1 MiB macOS and Debian gates after the
   feature lands.

Backlog items are only records. They are not permission to implement.
