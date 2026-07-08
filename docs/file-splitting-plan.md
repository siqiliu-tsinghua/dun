# File Splitting Plan

This document turns the code organization policy into an executable staged
plan. It is intentionally conservative: each stage should be reviewable,
behavior-preserving, and independently testable.

The current goal is code hygiene, not new editor behavior. A split stage should
move code across module boundaries with minimal visibility widening and no
semantic edits unless that stage explicitly says otherwise.

## Global Rules For Every Stage

Every stage must:

1. Start from a clean git worktree.
2. Write a short split map before moving code.
3. Move code verbatim where possible.
4. Prefer `pub(super)` to `pub(crate)`, and `pub(crate)` to `pub`.
5. Keep crate public surfaces compiling through facade re-exports.
6. Run `cargo fmt --all`.
7. Run `cargo test --workspace`.
8. Run `git diff --check`.
9. Record the result in `PROGRESS.md`.

If a stage cannot pass `cargo test --workspace`, stop and fix that stage before
starting another one.

No stage may introduce unsafe code. Existing `#![forbid(unsafe_code)]`
attributes must remain at crate/test-support entry points.

## Current Hotspot Order

As of 2026-07-08:

| Rank | File | Approx size | Reason for order |
| ---: | --- | ---: | --- |
| 1 | `crates/dun-cli/src/main.rs` | 558k chars | largest file, mixed app state, tests, I/O, terminal lifecycle, dialogs, command output |
| 2 | `crates/dun-ui/src/lib.rs` | 143k chars | mixed model/render/hit testing/tests |
| 3 | `crates/dun-config/src/lib.rs` | 84k chars | key model, defaults, parser, validation in one file |
| 4 | `crates/dun-core/src/buffer.rs` | 74k chars | core buffer behavior and tests in one file |
| 5 | `crates/dun-core/src/workspace.rs` | 31k chars | split-plan range, lower risk after larger files |
| 6 | `crates/dun-term/src/theme.rs` | 28k chars | split-plan range, can wait until theme work resumes |

The sequence below follows that risk order. Do not start with the smallest
files; the immediate maintenance cost is dominated by `dun-cli`.

## Stage 0: Baseline And Split Harness

Purpose: make later moves measurable.

Actions:

- capture current file sizes with `find crates -name '*.rs' ... | xargs wc -c`;
- capture current test names with `cargo test --workspace -- --list`;
- record current largest files in `PROGRESS.md`;
- keep this as documentation/setup only.

Gate:

```text
cargo fmt --all
cargo test --workspace
git diff --check
```

Expected behavior change: none.

## Stage 1: Split `dun-cli` Unit Tests By Behavior Family

Purpose: reduce the largest file before touching implementation ownership.

Target shape:

```text
crates/dun-cli/src/main.rs
crates/dun-cli/src/tests/
  mod.rs
  support.rs
  cli_args.rs
  config.rs
  editing.rs
  file_io.rs
  file_dialog.rs
  prompts.rs
  command_line.rs
  command_output.rs
  search_replace.rs
  windows.rs
  menus_mouse.rs
  helper_panes.rs
  terminal_io.rs
```

Rules:

- move the existing `#[cfg(test)] mod tests` body out first;
- keep test helper functions in `tests/support.rs` or narrow behavior modules;
- preserve all test names modulo module path prefixes;
- do not edit app implementation in this stage except for `mod tests;`.

Gate:

```text
cargo fmt --all
cargo test --workspace -- --list
cargo test --workspace
git diff --check
```

Expected behavior change: none.

Stop condition: if the test-name set changes beyond module path prefixes,
pause and reconcile before continuing.

## Stage 2: Extract `dun-cli` Pure Model Types

Purpose: move low-dependency structs/enums before stateful app methods.

Target shape:

```text
crates/dun-cli/src/
  main.rs
  app/
    mod.rs
    state.rs
    buffer_state.rs
    search.rs
    history.rs
  dialogs/
    mod.rs
    line_input.rs
    prompt.rs
    file_dialog.rs
    buffer_switcher.rs
    confirm.rs
  command_output/
    mod.rs
    model.rs
  terminal/
    mod.rs
    action.rs
```

Likely moves:

- `BufferState`, `LoadedTextBuffer`, file encoding/status structs;
- `LineInput`;
- prompt, file-dialog, buffer-switcher, confirm state structs;
- command-output section/model structs;
- `RuntimeAction`, `MouseDragState`, small enums.

Rules:

- prefer private modules under `dun-cli` first;
- expose only what `main.rs` still needs;
- do not move terminal setup or file I/O yet.

Gate:

```text
cargo fmt --all
cargo test --workspace
git diff --check
```

Expected behavior change: none.

## Stage 3: Extract `dun-cli` Pure Helpers

Purpose: separate branch-heavy pure logic from process-level code.

Target modules:

```text
app/status.rs
app/navigation.rs
dialogs/completion.rs
dialogs/file_dialog_listing.rs
help/text.rs
command_output/format.rs
```

Likely moves:

- status text formatting;
- command-line parsing/completion tables;
- file-dialog path/listing helpers;
- help/config diagnostics/status-history text generation;
- command-output buffer text formatting and section parsing;
- outline/search-results text generation.

Rules:

- these modules should have focused unit tests where practical;
- avoid moving functions that directly mutate `AppState` until Stage 4.

Gate:

```text
cargo fmt --all
cargo test --workspace
git diff --check
```

Expected behavior change: none.

Completion note:

- status/help text helpers now live under `crates/dun-cli/src/help/`;
- file-dialog path/listing helpers and text wrapping/width helpers now live
  under `crates/dun-cli/src/files/`;
- command-output section lookup helpers moved with other read-only text helper
  functions;
- AppState-mutating behavior remains in `main.rs` for Stage 4.

## Stage 4: Extract `dun-cli` App State Method Groups

Purpose: turn `AppState` into an `app` module with method groups rather than a
single file.

Target modules:

```text
app/mod.rs
app/commands.rs
app/editing.rs
app/windows.rs
app/bootstrap.rs
app/frame.rs
app/menus.rs
app/search_replace.rs
app/helper_panes.rs
app/command_output.rs
app/file_io.rs
app/file_dialogs.rs
app/buffer_switcher.rs
app/prompt_dialogs.rs
app/status_view.rs
app/view_state.rs
```

Likely moves:

- command dispatch methods;
- edit/window command application;
- find/replace/go-to-line methods;
- helper pane open/jump/close logic;
- clipboard and selection command logic;
- scroll/view synchronization.

Rules:

- keep `AppState` definition in `app/state.rs`;
- use `impl AppState` blocks per module;
- keep behavior tests from Stage 1 green after each module group move.

Progress note:

- `app/windows.rs` owns window command application, split/focus/resize/close,
  focused-buffer window lookup, and unreferenced buffer cleanup;
- `app/editing.rs` owns edit command application, text input, internal
  clipboard, OSC52 copy request preparation, line commands, bookmarks, undo,
  redo, paging, and horizontal scrolling;
- `app/view_state.rs` owns focused buffer accessors, buffer lookup, status
  recording, and buffer view-context calculation;
- `app/mouse.rs` owns mouse interaction state and mouse-driven workspace,
  selection, split, scrollbar, menu, and file-dialog hit behavior;
- `app/commands.rs` owns editor/app/file command dispatch, configured key
  sequence dispatch, auxiliary-window key dispatch, runtime-action requests,
  and config reload application;
- `app/file_io.rs` owns focused buffer open/save/save-as/reload/new-file
  mutation paths on top of the `files/` I/O helpers;
- `app/helper_panes.rs` owns Help, Config Diagnostics, Status History, Outline,
  and read-only helper-pane refresh/jump behavior;
- `app/command_output.rs` owns command-output AppState behavior: output pane
  open/refresh/clear/copy/navigation/search/section views/save dialog;
- `app/search_replace.rs` owns find preview commit, replace confirmation,
  search-results pane jumps, replace/replace-all, numbered helper-row
  navigation, and go-to-line;
- `app/prompt_dialogs.rs` owns prompt lifecycle, paste routing into prompts,
  prompt history/completion/submission, and active modal overlay assembly;
- `app/file_dialogs.rs` owns file-dialog event handling plus unsaved-change
  confirmation flows because confirmation can chain into Save As;
- `app/buffer_switcher.rs` owns buffer-switcher keyboard/mouse navigation and
  overlay text;
- `app/bootstrap.rs` owns AppState construction from loaded config and optional
  startup file paths;
- `app/frame.rs` owns buffer-view assembly, search-cache refresh, mouse-enabled
  access, and workspace-area view synchronization;
- `app/menus.rs` owns active menu state, keyboard/mouse menu opening, menu
  movement, and selected-menu dispatch;
- `app/command_line.rs` owns command-line command runners over the typed editor
  command surface;
- `app/status_view.rs` owns focused path/name/status/detail display helpers;
- Stage 4 is complete: `crates/dun-cli/src/main.rs` no longer contains an
  `impl AppState` block. Remaining `main.rs` code is process entry, startup
  config loading, pure command-line parsing/completion, help text assembly,
  command-output text formatting, terminal profile detection, and the runtime
  event loop for Stage 6.

Gate after each method-group move, not only after the whole stage:

```text
cargo fmt --all
cargo test --workspace
git diff --check
```

Expected behavior change: none.

## Stage 5: Extract `dun-cli` Process I/O Boundaries

Purpose: isolate host side effects from editor state.

Target modules:

```text
files/
  mod.rs
  open.rs
  save.rs
  snapshot.rs
  atomic.rs
terminal/
  mod.rs
  lifecycle.rs
  input.rs
  sgr.rs
  shell.rs
```

Likely moves:

- CLI argument parsing can remain in `main.rs` until the end or move to
  `app/args.rs`;
- file open/read validation/save/atomic temp cleanup;
- terminal guard, raw/alternate screen lifecycle;
- SGR 16-color rewriter;
- key/mouse/paste event conversion and dispatch;
- shell escape and run-command host process boundary.

Rules:

- side-effect modules should expose typed results, not mutate global state
  behind broad APIs;
- preserve all existing path diagnostics and terminal restoration tests;
- do not add new dependencies.

Progress note:

- `terminal/lifecycle.rs` owns raw mode, alternate-screen, bracketed paste,
  mouse capture, suspend/resume, and drop-time terminal restoration;
- `terminal/sgr.rs` owns the 16-color SGR rewriter and `TerminalWriter`;
- `terminal/input.rs` owns key/mouse event dispatch and crossterm-to-config
  key/text conversion;
- `terminal/shell.rs` owns shell escape, one-shot command execution, bounded
  stdout/stderr capture, and command-run status formatting;
- `files/snapshot.rs`, `files/open.rs`, `files/save.rs`, and
  `files/atomic.rs` own stable-read validation, UTF-8/fallback loading, save
  snapshot checks, path diagnostics, same-directory atomic writes, and stale
  atomic-temp cleanup.

Stage 5 is complete for the planned process I/O boundaries. AppState file
command flow and dialog methods remain in `main.rs` until the later AppState
method-group and entry-point reduction stages.

Gate:

```text
cargo fmt --all
cargo test --workspace
git diff --check
```

Expected behavior change: none.

## Stage 6: Reduce `dun-cli/src/main.rs` To Entry Point

Purpose: make the binary root small and stable.

Target:

```text
crates/dun-cli/src/main.rs      process entry, CLI top-level only
crates/dun-cli/src/lib.rs       optional facade only if integration tests need it
```

Do not create `lib.rs` merely for aesthetics. Create it only if tests or
future CLI integration benefit from using shared app modules without a binary
target.

Gate:

```text
cargo fmt --all
cargo test --workspace
git diff --check
```

Expected result:

- `main.rs` below `10k` to `20k` chars;
- app behavior lives in named modules.

Completion note:

- CLI argument parsing and startup config discovery/loading now live in
  `crates/dun-cli/src/{cli,config_loading}.rs`;
- command-line prompt parsing/completion now lives in
  `crates/dun-cli/src/command_line.rs`;
- help content, read-only helper buffers, and config-diagnostics section types
  now live under `crates/dun-cli/src/help/`;
- command-output formatting now lives in
  `crates/dun-cli/src/command_output/format.rs`;
- terminal profile detection, event loop, and OSC52 clipboard sequence
  formatting now live under `crates/dun-cli/src/terminal/`;
- small shared editor constants and wrapping index math live in
  `crates/dun-cli/src/util.rs`;
- `crates/dun-cli/src/main.rs` is now process entry plus CLI/TUI startup
  orchestration only and remains below the target size.

## Stage 7: Split `dun-ui`

Purpose: separate backend-neutral UI model, rendering, hit testing, and text
helpers.

Target shape:

```text
crates/dun-ui/src/
  lib.rs
  model.rs
  shell.rs
  menu.rs
  overlay.rs
  window.rs
  render/
    mod.rs
    menu.rs
    overlay.rs
    status.rs
    window.rs
    chrome.rs
  hit.rs
  text.rs
  tests/
    mod.rs
    model.rs
    rendering.rs
    hit.rs
    fallback.rs
```

Split order:

1. move tests out;
2. move pure model types;
3. move text width/sanitization helpers;
4. move hit testing;
5. move render functions by visual layer;
6. keep `UiShell` and public exports in `lib.rs`/`shell.rs`.

Gate after each sub-step:

```text
cargo fmt --all
cargo test --workspace
git diff --check
```

Expected behavior change: none.

Progress note:

- Stage 7.1 moved inline `dun-ui` unit tests out of `lib.rs` into
  `tests/{model,hit,rendering,fallback,support}.rs`. Implementation code
  remains in `lib.rs` for the next sub-step.
- Stage 7.2 moved backend-neutral UI data types into `model.rs` and preserved
  the existing public API through `lib.rs` re-exports.
- Stage 7.3 moved text width, truncation, wrapping, and visible-whitespace
  helpers into `text.rs`.
- Stage 7.4 moved workspace, menu, and overlay hit-testing methods into
  `hit.rs`. Shared menu/overlay layout helpers remain in `lib.rs` until the
  render-layer extraction decides their final home.
- Stage 7.5 moved ratatui rendering into `render/{mod,chrome,menu,overlay,
  status,window}.rs`. `lib.rs` keeps `UiShell`, frame model construction,
  menu/status model construction, and public facade exports.

## Stage 8: Split `dun-config`

Purpose: make key model, defaults, parser, and validation independently
reviewable.

Target shape:

```text
crates/dun-config/src/
  lib.rs
  commands.rs
  config.rs
  limits.rs
  keys/
    mod.rs
    key.rs
    sequence.rs
    keymap.rs
    file_dialog.rs
  defaults.rs
  parser.rs
  validation.rs
  tests/
    mod.rs
    keys.rs
    parser.rs
    validation.rs
```

Rules:

- public types should remain re-exported from `lib.rs`;
- parser behavior and default config text must stay byte-for-byte stable unless
  an intentional config change is separately approved.

Gate:

```text
cargo fmt --all
cargo test --workspace
git diff --check
```

Expected behavior change: none.

Progress note:

- Stage 8.1 moved inline `dun-config` unit tests into
  `tests/{config,keys,parser,validation,support}.rs`.
- Stage 8.2 moved the typed config model into `config.rs` and the limits model
  into `limits.rs`, preserving public exports from `lib.rs`.
- Stage 8.3 moved key parsing, key sequence display, editor keymap defaults,
  and file-dialog modal keymap/action mapping into
  `keys/{key,sequence,keymap,file_dialog}.rs`. `lib.rs` keeps the command id
  mapping, parser, default config text, validation error text, and facade
  exports until the next parser/defaults split.
- Stage 8.4 moved command id mapping into `commands.rs`, default config text
  into `defaults.rs`, line-based config parsing into `parser.rs`, and error
  text conversion into `validation.rs`. `lib.rs` is now only the crate-level
  facade plus test module hook.

## Stage 9: Split `dun-core::buffer`

Purpose: make core editing logic reviewable without changing buffer semantics.

Target shape:

```text
crates/dun-core/src/buffer/
  mod.rs
  model.rs
  cursor.rs
  selection.rs
  edit.rs
  line_ops.rs
  search.rs
  undo.rs
  tests/
    mod.rs
    cursor.rs
    edit.rs
    selection.rs
    search.rs
    undo.rs
    line_ops.rs
```

Rules:

- preserve `dun_core::TextBuffer` and related public exports;
- avoid storage representation changes during the split;
- do not combine this with rope/gap-buffer/large-file work.

Gate:

```text
cargo fmt --all
cargo test --workspace
git diff --check
```

Expected behavior change: none.

## Stage 10: Split Remaining Split-Plan Files

Purpose: finish the lower-risk organization work after the largest files are
stable.

Targets:

```text
crates/dun-core/src/workspace/
  mod.rs
  model.rs
  split.rs
  focus.rs
  resize.rs
  hit.rs
  tests.rs

crates/dun-term/src/theme/
  mod.rs
  color.rs
  style.rs
  palette.rs
  builtins.rs
  tests.rs
```

Gate:

```text
cargo fmt --all
cargo test --workspace
git diff --check
```

Expected behavior change: none.

## Commit Strategy

Each stage should normally be one commit. Stage 4, Stage 5, Stage 7, and Stage
9 may become multiple commits because they contain natural sub-steps.

Commit messages should name the moved boundary:

```text
Split dun-cli file dialog tests
Extract dun-ui overlay rendering
Move dun-config keymap model
Split dun-core buffer undo logic
```

Avoid mixed commits that both move modules and change behavior. If a behavior
bug is found during a split, fix it in a separate commit before or after the
move.

## Release And Risk Notes

This plan is not a feature roadmap. It should be paused when urgent editor
functionality, terminal compatibility, or security work is more important.

The first large split stage should be done before the next broad CLI feature
batch. Otherwise `dun-cli/src/main.rs` will keep absorbing unrelated behavior,
making later splits riskier.
