# Feature Triage

Working document for the v0.1 slimming stage. It inventories every runtime
feature as a separately-removable unit, assigns each a class with evidence,
and replaces the lazy trim-on-failure order in
[feature-budget.md](./feature-budget.md) once complete. The active plan and
sequencing live in [CLAUDE.md](../CLAUDE.md).

## Budget Math

All figures are Debian x86_64, the binding platform.

```text
baseline (2026-07-09, 60d45a2):        1,038,936 bytes
budget:                                1,048,576 bytes
current margin:                            9,640 bytes

plugin client cost (spike, 2026-07-10):   77,824 bytes  (76.0 KiB, Debian)
future-feature reserve (target):      80–120 KiB
required freed bytes:                150,104–191,064 bytes  (~147–187 KiB)
```

The client cost comes from the `spike/plugin-client-size` branch
(`c7f042c`), measured on the Debian VM with the locked release profile; see
[release-size-audit.md](./release-size-audit.md). Treat 76 KiB as a floor:
the spike covers framing, hand-rolled JSON, envelope/role/policy, validation,
and timeout/cancel/crash handling for one role, but not config integration or
additional roles. Same-commit macOS delta was 62,032 bytes; Debian deltas run
roughly 1.25x macOS, which is usable for quick local estimates only.

## Rough Attribution (macOS, 2026-07-10)

`cargo bloat` over the release profile with `strip` disabled, at `0404b18`
(.text total 670.3 KiB): std 352.2 KiB, dun-cli 158.1 KiB, dun-ui 41.1 KiB,
dun-config 30.8 KiB, dun-core 25.5 KiB, crossterm 21.5 KiB, ratatui
15.8 KiB, dun-term 3.8 KiB.

Two structural findings:

- Roughly 90+ KiB of the std share is panic-backtrace symbolization (gimli,
  addr2line, rustc_demangle), linked even with `panic = "abort"`. It cannot
  be removed on the stable toolchain, so trims must come from the remaining
  ~580 KiB.
- Feature code is spread across many small methods; visible single functions
  (Command Output capture 5.0 KiB, Config Diagnostics text 5.6 KiB, command
  prompt run/complete 12.6 KiB combined) understate their units. Per-unit
  `Bytes` therefore comes from batch removal experiments, not attribution.

## Classes and Decision Rules

Apply in order; first hit wins.

| Class | Meaning | Rule |
| --- | --- | --- |
| A | Core; removal breaks the product | Safety/correctness invariant, or required by the seven-step remote editing loop (README "Product Goal") with no A-level workaround. |
| B | Optional; kept while budget allows | Everything not A/C/D; ranked by measured bytes per value; total B bytes are capped. |
| C | Remove now | Serves neither the editing loop nor SSH constraints; includes showcase-era leftovers. |
| D | Delegate to plugin | A plugin role can provide it once the protocol client exists; record the role need in [plugin-protocol.md](./plugin-protocol.md) at removal. |

Measurement rules: sizes come from removal experiments built with the locked
release profile; with `opt-level = "z"` and fat LTO deltas are non-additive,
so removals are measured per batch. macOS deltas are proxies only.

## Inventory

`Current` reflects [feature-budget.md](./feature-budget.md): `req` =
required table, `t1`–`t13` = optional trim position. `Hypothesis` is the
pre-measurement expectation and is not a decision. `Bytes` and `Decision`
are filled during the triage.

### File and buffers

| ID | Feature unit | Surface | Current | Hypothesis | Bytes | Decision |
| --- | --- | --- | --- | --- | ---: | --- |
| F01 | Core file ops: new/open/save/save-as/close, atomic writes, path diagnostics | `file.*` | req | A | | |
| F02 | Explicit reload + external-change overwrite refusal | `file.reload` | req | A | | |
| F03 | Buffer switcher overlay | `file.switch_buffer` | t7 | B | | |
| F04 | Unsafe-file handling: invalid-UTF-8 read-only fallback, large-file limit, unstable-read rejection | open path | req | A | | |
| F05 | Unsaved-change confirmations on quit/new/open/close/reload | confirm flow | req | A | | |

### Editing core

| ID | Feature unit | Surface | Current | Hypothesis | Bytes | Decision |
| --- | --- | --- | --- | --- | ---: | --- |
| F06 | Insert/delete/newline, undo/redo with transaction coalescing | `edit.*` core | req | A | | |
| F07 | Cursor movement: arrows, line/page/document/word | `edit.move_*` | req | A | | |
| F08 | Keyboard selection: shift-movement, select line/all, word/page extension | `edit.extend_*`, `edit.select_*` | req | A | | |
| F09 | Internal clipboard cut/copy/paste | `edit.cut/copy/paste` | req | A | | |
| F10 | Opt-in OSC 52 external copy | `edit.copy_external`, `clipboard.osc52.*` | req | B | | |
| F11 | Line commands: copy/delete/move/indent/outdent/trim | `edit.*_line`, `edit.trim_*` | t3 | B | | |
| F12 | Bookmarks: toggle/next/previous + gutter markers | `edit.*bookmark*` | t3 | C | | C, removed 2026-07-10 (batch 3) |
| F13 | Visible-whitespace markers | `edit.toggle_visible_whitespace` | t3 | C | | C, removed 2026-07-10 (batch 3) |
| F14 | Soft-wrap visual-row model: wrap toggle + wrapped scrolling/selection/highlights/paging | `edit.toggle_word_wrap` + display layer | t9 | B, likely largest single unit | | |
| F15 | Horizontal scrolling, explicit scroll commands, clip edge indicators | `edit.scroll_left/right` | req | A | | |

### Search and navigation

| ID | Feature unit | Surface | Current | Hypothesis | Bytes | Decision |
| --- | --- | --- | --- | --- | ---: | --- |
| F16 | Find with preview, next/previous, `/i` `/w` prefixes, match highlight + count | `edit.find*` | req | A | | |
| F17a | Replace: command-driven `replace` / `replace all` | `edit.replace` | req | A | | |
| F17b | Interactive replace confirmation modal (replace/skip/all/cancel) | replace flow | req | B | | |
| F18 | Go to line | `edit.go_to_line` | req | A | | |
| F19 | Search Results pane: `results`, `results N`, row navigation | `app.search_results` | t4 | B | | |
| F20 | Outline pane: section heuristics (Markdown/INI/TOML/Rust/shell), jumps | `app.outline` | t4 | D (structure role) | | D, removed 2026-07-10 (batch 2); structure-listing role recorded in plugin-protocol.md |

### Tiling windows

| ID | Feature unit | Surface | Current | Hypothesis | Bytes | Decision |
| --- | --- | --- | --- | --- | ---: | --- |
| F21 | Tiling core: split H/V, directional focus, resize, close, only | `window.*` core | req | A | | |
| F22 | Tiling extras: equalize, rotate, collapse/expand/toggle | `window.equalize/rotate/collapse/expand/toggle_collapse` | req | B | | |
| F23 | Small-pane degradation: gutter drop, title clipping | render path | req | A | | |

### Chrome and rendering

| ID | Feature unit | Surface | Current | Hypothesis | Bytes | Decision |
| --- | --- | --- | --- | --- | ---: | --- |
| F24 | Menu bar + grouped dropdowns, mnemonics, short-terminal scrolling | menu system | req | A | | |
| F25 | Status bar core fields: name, dirty/read-only, position | status bar | req | A | | |
| F26 | Status bar extended fields: selection, match count, scroll range, h-offset, encoding, profile, window index | status bar | req | B | | |
| F27a | `msedit` theme + 16-color/mono fallback colors | themes | req | A | | |
| F27b | Extra built-in themes: `turbo`, `dark`, `dun` | themes | t1 | B | | |
| F28 | Vertical scrollbar thumb + click/drag interaction | render + mouse | req/t2 | B | | |
| F29 | msedit fidelity polish: current-line highlight, gutter separator | render | req | B | | |
| F30 | Mouse support family: focus, cursor, selection drag, split drag, menu clicks, wheel, scrollbar, dialog clicks | `mouse.*` config | t2 | B | | |
| F31 | Bracketed paste routing + paste safety rules | paste path | req | A | | |
| F32 | Display sanitization: C0/C1 controls, escape payloads, long-line caps | sanitizer | req | A | | |
| F33 | Terminal profiles: UTF-8/ASCII glyphs, 256/16/mono colors, 16-color SGR rewrite | `dun-term` + sgr | req | A | | |

### Dialogs and prompts

| ID | Feature unit | Surface | Current | Hypothesis | Bytes | Decision |
| --- | --- | --- | --- | --- | ---: | --- |
| F34a | File dialog base: path input, match list, keyboard selection, directory nav, overwrite confirm | Open/Save As | req | A | | |
| F34b | File dialog Tab path completion | Open/Save As | req | B | | |
| F35 | File dialog polish: hidden-file toggle, parent row, overflow indicators, recent-directory memory, wheel | Open/Save As | t8 | B/C | | |
| F36 | Modal prompts for find/replace/go-to-line with UTF-8 cursor editing | prompts | req | A | | |
| F37a | Command prompt (`Ctrl+P`) executing typed command ids | `app.command_line` | req | A | | |
| F37b | Command prompt bounded history | prompt | req | B | | |
| F38 | Command prompt Tab completion: families, candidate cycling, path arguments | prompt | req | B | | |

### Config and helper panes

| ID | Feature unit | Surface | Current | Hypothesis | Bytes | Decision |
| --- | --- | --- | --- | --- | ---: | --- |
| F39 | Config loading, typed keymap, validation, `--dump-config` | `dun-config` | req | A | | |
| F40 | Runtime `reload-config`, runtime `theme`, live helper refresh | `app.reload_config` | t12 | B/C | | |
| F41 | Config Diagnostics pane + 9 section jumps | `app.config_diagnostics*` (10 ids) | t6 | B/C | | |
| F42 | Help / key reference pane | `app.help` | t13 | B | | |
| F43 | Status History pane | `app.status_history` | t5 | B/C | | |

### Shell and command output

| ID | Feature unit | Surface | Current | Hypothesis | Bytes | Decision |
| --- | --- | --- | --- | --- | ---: | --- |
| F44 | Shell escape: suspend TUI, run shell, resume | `app.shell_escape` | req | A | | |
| F45 | Run Command: prompt, bounded capture, base output pane, own history | `app.run_command` | t11 | B | | |
| F46 | Command Output advanced: search/next/prev, save + dialog, summary/status/stdout/stderr/body/index/truncation jumps, section nav, only-stdout/only-stderr derived panes | `app.command_output_*` (17 ids) | t10 | C/D (showcase leftover; LogFilter role overlap) | | C, removed 2026-07-10 (batch 1); stream filtering/derived views recorded as LogFilter plugin territory |

### CLI and plugin

| ID | Feature unit | Surface | Current | Hypothesis | Bytes | Decision |
| --- | --- | --- | --- | --- | ---: | --- |
| F47 | CLI contract: `--help`, `--version`, `--config`, `--no-config`, `--dump-config`, exit codes | `dun-cli` args | req | A | | |
| F48 | Plugin protocol client (incoming) | not yet implemented | req | A | | |

## Removal Trail Checklist

Removing a unit is a full-trail diff. For each removal batch:

1. Code paths and types.
2. `EditorCommand` variants and command-id parsing.
3. Default keymap entries and configurable bindings.
4. Menu entries and mnemonics.
5. Help/key-reference and status text.
6. Tests (unit, UI snapshot, PTY where applicable).
7. README feature paragraphs and affected docs.
8. `feature-budget.md` classification tables.
9. For D-class removals: record the plugin role need in
   [plugin-protocol.md](./plugin-protocol.md).
10. Gates: fmt, clippy, workspace tests, release smoke, both-platform size
    measurement recorded in [release-size-audit.md](./release-size-audit.md).
