# Code Organization Guidelines

This document defines the code organization standard for `dun`.

It is adapted from the neighboring `rum` project's code organization rules,
but tuned for `dun`'s terminal-editor shape. The goal is not to create many
small files for their own sake. The goal is to keep editor code reviewable,
testable, and ready for future crate extraction without changing behavior.

## Goals

`dun` should stay:

- readable in one coherent pass per module;
- friendly to focused unit tests;
- practical for incremental compilation and future crate boundaries;
- safe Rust by default;
- light enough for SSH and low-resource terminal use.

File and module boundaries should make ownership clearer. They should separate
policy, parsing, rendering, terminal I/O, file I/O, and state mutation where
those responsibilities are genuinely distinct.

## Safe Rust Policy

`dun` source code is safe Rust.

Rules:

- every crate root and Rust test/support entry point must use
  `#![forbid(unsafe_code)]`;
- no `unsafe` block, `unsafe fn`, `unsafe impl`, `unsafe trait`, or
  `unsafe extern` may be added to `dun` crates in normal development;
- if `unsafe` seems unavoidable, stop and record an explicit design decision
  before implementation;
- prefer safe standard-library and crate APIs, even when they require a small
  local wrapper;
- new dependencies that contain unsafe code become part of the trusted
  computing base and must be mentioned in the dependency audit when they affect
  the default runtime path.

Current status: `dun`'s own crates have zero real unsafe code. The `unsafe`
word appears only as ordinary text in a Rust-outline parser token list.
Third-party dependencies may contain unsafe internals; that is dependency TCB,
not permission to add unsafe to `dun` itself.

## File Size Policy

Use character counts as maintenance signals, not as blind commands.

Implementation files:

- under `10k`: preferred range;
- `10k` to `20k`: warning range; assess whether a split would improve
  reviewability;
- `20k` to `35k`: split-plan range; any change touching the file should state
  the split boundary or start the split;
- over `35k`: architecture debt unless an explicit temporary exception exists.

Test files:

- under `15k`: preferred range;
- `15k` to `25k`: warning range;
- `25k` to `45k`: split-plan range;
- over `45k`: debt; split by behavior family.

Measure implementation size after separating in-file `#[cfg(test)]` modules.
Large in-file test modules should usually be moved first, because that often
reveals the real implementation size and gives immediate review wins.

## Current Hotspots

As of 2026-07-08, these original hotspot files drove the staged split plan:

| File | Approx size | Status | Likely first boundary |
| --- | ---: | --- | --- |
| `crates/dun-cli/src/main.rs` | 558k chars | done | split into app, input, dialogs, files, terminal, command-output, help, and tests |
| `crates/dun-ui/src/lib.rs` | 143k chars | done | split into facade, shell, frame, render, hit, text, model, and tests |
| `crates/dun-config/src/lib.rs` | 84k chars | done | split into keys, parser, defaults, validation, commands, config, limits, and tests |
| `crates/dun-core/src/buffer.rs` | 74k chars | done | split into buffer storage, cursor, selection, edits, line ops, undo, search, and tests |
| `crates/dun-core/src/workspace.rs` | 31k chars | done | split into `workspace/{model,split,focus,resize,hit,tests}.rs` |
| `crates/dun-term/src/theme.rs` | 28k chars | done | split into `theme/{color,style,palette,builtins,tests}.rs` |

These original hotspot files are now split. The same thresholds still create
an assess-on-touch rule for any new file that reaches the warning or split-plan
ranges: new feature work in one of those files should either perform a focused
split or explain why the split is deferred and what boundary will be used
later.

Stage 10 removed the original `workspace.rs` and `theme.rs` files. The
remaining theme palette constructor file, `theme/builtins.rs`, is concentrated
data-style code; when adding new theme families, split that file by theme
family before extending it further.

Stage 12 split the i18n key table. `ui_text.rs` grew past `35k` while the i18n
slices landed and was split twice: first into `ui_text/{mod,chrome,status}.rs`,
then — once the extraction stopped adding keys — `status.rs` (42k, 265 keys)
into `ui_text/status/{window,file,edit,search,prompt,command,command_output}.rs`,
each under `10k`. The temporary size exception recorded here for `status.rs` is
retired.

Two properties of that table are load-bearing and should survive future edits:
`ALL` stays a **single flat enumeration** (assembled in `status/mod.rs` from the
domain modules, then in `ui_text/mod.rs` with `chrome`), because the
translation-completeness and key-uniqueness tests walk it; and the domain
modules are re-exported with globs, so every call site says `ui_text::SOME_KEY`
and never has to know which file a key lives in. Moving a key between domain
modules must therefore stay invisible to callers.

Stage 11 reduced `dun-ui/src/lib.rs` to a facade. The main remaining watch-list
implementation files are now feature-group files such as
`dun-cli/src/app/search_replace.rs`, `dun-cli/src/app/editing.rs`, and
`dun-term/src/theme/builtins.rs`; split them only when their behavior is next
touched or their responsibility boundaries become clearer.

## Preferred Module Shape

Crate roots should become facades:

- `lib.rs`: module declarations, public re-exports, short crate overview;
- `main.rs`: process entry point only, with app behavior delegated to modules.

Use normal Rust modules and directory modules, not `include!` trees.

Preferred future directory shapes:

```text
crates/dun-cli/src/
  main.rs              process entry only
  app/                 AppState and command dispatch
  input/               key, mouse, paste, menu dispatch
  dialogs/             prompt, file dialog, buffer switcher, confirmations
  files/               open/save/reload, snapshots, atomic save
  terminal/            lifecycle guard, SGR rewrite, shell escape
  command_output/      run command, output buffer, output navigation/save
  help/                help/status/diagnostics/outline/search-result text
  tests/               unit tests split by behavior family

crates/dun-ui/src/
  lib.rs               facade
  model/               UiFrame, UiWindow, UiOverlay, menu/status models
  render/              menu, status, window, overlay, chrome helpers
  hit/                 workspace/menu/overlay hit testing
  text/                width fitting, sanitized span conversion
  tests/               renderer and model tests by behavior family

crates/dun-core/src/
  buffer/              storage, cursor, selection, edits, undo, search
  workspace/           split tree, focus, resize, hit testing
  display.rs           sanitizer remains small unless it grows
  file_text.rs         decoding remains small unless more encodings arrive

crates/dun-config/src/
  keys/                Key, KeyStroke, KeySequence, keymap lookup
  parser/              line-based config parser
  defaults/            default config and keymap construction
  validation/          duplicate bindings and limit checks

crates/dun-term/src/
  profile.rs           terminal profile detection/overrides
  glyphs.rs            glyph profiles
  theme/               style primitives and built-in theme palettes
```

These are expected boundaries, not mandatory names. Use the names that best
fit the code when doing the split.

## Splitting Principles

Split by responsibility:

- model vs rendering;
- parsing vs validation;
- terminal I/O vs editor state mutation;
- file I/O vs buffer mutation;
- command parsing vs command execution;
- search/index planning vs UI display;
- test setup vs behavior assertions.

Avoid:

- arbitrary size chunks;
- tiny micro-modules with no domain meaning;
- broad `pub` exports just to make a split compile;
- moving and refactoring behavior in the same patch unless the behavior change
  is the actual task.

When splitting:

1. Write a short split map first: target module, responsibility, expected
   moved code.
2. Move bodies verbatim where possible.
3. Keep visibility as narrow as possible: `pub(super)` before `pub(crate)`,
   `pub(crate)` before `pub`.
4. Keep old public surfaces compiling through facade re-exports.
5. Run focused tests for the owning crate; run `cargo test --workspace` for
   cross-crate or command-routing changes.
6. If tests are moved, preserve behavior coverage and keep helper code in a
   local `support` module.

## Directory Fan-Out

Directory sprawl is also debt.

- a directory with more than about 20 direct Rust files should be reviewed for
  subdirectory grouping;
- group by domain, not alphabetically;
- if a new file would make an already-busy directory harder to scan, create a
  named subdirectory first.

## Test Organization

Prefer tests near the crate that owns the behavior:

- pure buffer/workspace behavior belongs in `dun-core`;
- terminal profile/theme behavior belongs in `dun-term`;
- config parsing and keymap behavior belongs in `dun-config`;
- renderer model and ratatui snapshot behavior belongs in `dun-ui`;
- process-level command routing, terminal lifecycle, file I/O, and PTY smoke
  behavior belongs in `dun-cli`.

Within large test modules, split by behavior family. Good examples for
`dun-cli` include file I/O, prompts, file dialogs, command line, command
output, mouse, menus, config reload, helper panes, and terminal lifecycle.

## Exception Policy

Exceptions are allowed only when explicit.

Acceptable temporary reasons:

- a subsystem is still moving too quickly for a stable boundary;
- a planned split would obscure an urgent correctness fix;
- a patch is intentionally documentation-only;
- a file is being split in staged, reviewable batches.

An exception should say:

- why the file remains large;
- what the future split boundary is;
- what test command covered the current change.
