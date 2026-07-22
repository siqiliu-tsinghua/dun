# Brief 034 — Wide geometry step 1: thread the width mode through text measurement

Implementation brief. This is **step 1 of the plan in
`docs/dev/codex/brief-033-wide-geometry-plan.md`** (read it first). Mechanical
change, named test gate. Steps 2–4 are separate briefs — do NOT do them here.

## Goal

Route `dun-ui`'s text-measurement helpers through the ambiguous-width authority
so measurement honors `AmbiguousWidth`, with **Narrow behavior byte-for-byte
identical** to today. This step changes *measurement only*; it does NOT change
border drawing (step 2), window geometry / `WindowGeometry` (step 3), or
`dun-cli` (step 4). When you are done, the four `text.rs` helpers take an
explicit `AmbiguousWidth` argument, every `dun-ui` call site passes the mode
from its `shell.profile.ambiguous_width` (or forwards a mode it was given), and
no `dun-ui` code calls `unicode-width` directly anymore — it calls
`dun_term::char_width` / `dun_term::str_width`.

## What already exists

`dun_term::char_width(ch, mode) -> Option<usize>` and
`dun_term::str_width(s, mode) -> usize` are the width authority (Narrow =
`unicode-width`'s `width`, Wide = `width_cjk`). `UiShell` has
`profile: TerminalProfile` with an `ambiguous_width` field.

## Exact change

Change the four helper signatures in `crates/dun-ui/src/text.rs`:

```rust
display_width(text, mode)
wrap_line_segments(line, width, mode)
fit_text_to_width(text, max_width, truncation, mode)
status_text_for_width(left, right, width, truncation, mode)
```

Their bodies use `dun_term::char_width` / `dun_term::str_width` under `mode`
exclusively (Narrow must call exactly today's narrow operations, so a Narrow run
is unchanged). An explicit `AmbiguousWidth` parameter is the chosen shape (a
`Copy` one-byte value; no context struct, no global/thread-local).

Also convert the two remaining **direct** `unicode-width` calls in `dun-ui` (not
through the helpers) to the authority, passing the local mode:

- `crates/dun-ui/src/frame/text.rs` (the `UnicodeWidthStr::width` calls, ~lines
  126/133/152)
- `crates/dun-ui/src/render/surface_window.rs` (`UnicodeWidthChar::width`, ~line
  171/174)

Remove the now-unused `use unicode_width::...` imports from every `dun-ui` file
you empty out. (Do NOT remove the `unicode-width` dependency from
`dun-ui/Cargo.toml` yet — `surface.rs` still imports nothing from it but other
step-2/3 code and tests may; leaving the dep is fine and is a step-4 cleanup.)

### Call-site inventory (pass the mode from these sources)

| Helper / direct call | Call sites | Mode source |
|---|---|---|
| `wrap_line_segments` | `frame/text.rs:60,83`; `frame/highlight.rs:460` | `self.profile.ambiguous_width` |
| `display_width` | `frame/highlight.rs:465`; `frame/scroll.rs:113` | `self.profile.ambiguous_width` |
| `display_width` | `render/menu.rs:9,35,86,90` (add `mode` to `menu_item_column_range`); `hit.rs:59` | `shell.profile.ambiguous_width` |
| `display_width` | `render/overlay.rs:63,65,68,71,74` (forward mode from `overlay_layout` into the private `overlay_layout_for_content`) | forwarded |
| `display_width` | `render/surface_layers.rs:28`; `render/surface_overlay.rs:148` | `shell.profile.ambiguous_width` |
| `fit_text_to_width` | `render/menu.rs:109`; `render/window.rs:41`; `render/surface_overlay.rs:68,83,99,124,145` | `shell.profile.ambiguous_width` |
| `status_text_for_width` | `render/menu.rs:112`; `render/status.rs:11` | `shell.profile.ambiguous_width` |
| direct | `frame/text.rs:126/133/152`; `render/surface_window.rs:171/174` | local `shell`/`self` profile mode |

Test call sites: `tests/model.rs:409,416,419,421` pass `AmbiguousWidth::Narrow`
(preserving the helper unit contract); `tests/model.rs:437` and `tests/i18n.rs:112`
pass `shell.profile.ambiguous_width`.

If a call site turns out not to have a `shell`/profile in scope without a large
signature cascade beyond the files listed in Scope, STOP and report it (do not
invent a global).

## Scope

- Files you MAY modify:
  - `crates/dun-ui/src/text.rs`
  - `crates/dun-ui/src/frame/text.rs`, `.../frame/highlight.rs`, `.../frame/scroll.rs`
  - `crates/dun-ui/src/render/menu.rs`, `.../render/overlay.rs`, `.../render/status.rs`,
    `.../render/surface_layers.rs`, `.../render/surface_overlay.rs`,
    `.../render/surface_window.rs`, `.../render/window.rs`
  - `crates/dun-ui/src/hit.rs`
  - `dun-ui` test modules that call these helpers (`src/tests/model.rs`,
    `src/tests/i18n.rs`, and any others the compiler flags)
- Files/areas you MUST NOT touch:
  - `crates/dun-ui/src/render/surface_draw.rs` (border drawing = step 2)
  - `crates/dun-ui/src/surface.rs` (the Surface itself = already done / step 2)
  - the `inner_area`/`body_area`/gutter **geometry** in `surface_window.rs` —
    change ONLY the measurement calls there, leave the `width - 2` / gutter math
    exactly as it is (step 3 reworks geometry)
  - anything in `crates/dun-cli/**`, `crates/dun-core/**`, `crates/dun-term/**`,
    `crates/dun-config/**`
  - any `Cargo.toml`, `Cargo.lock`, `.git`, `docs/**`, `i18n/**`, `vm-test/**`,
    `reference/**`, `hosts/**`

## Deliverable

- The signature change + call-site cutover above.
- One new test (colocated with the `text.rs` unit tests): a
  `wide_text_helpers_measure_fit_and_wrap_ambiguous_glyphs` test asserting that,
  under `AmbiguousWidth::Wide`, `display_width` of a box-drawing string counts 2
  per glyph, `fit_text_to_width` truncates accordingly, and `wrap_line_segments`
  splits by the wide width — versus the narrow counts under Narrow.
- Existing `status_text_is_clipped_by_display_width` and
  `window_title_is_clipped_by_display_width` still pass (update their calls to
  pass a mode, keeping Narrow).

## dun pitfalls (read twice)

1. Safe Rust only (`#![forbid(unsafe_code)]`).
2. Narrow is sacred: do not update any existing Narrow golden snapshot
   (`crates/dun-cli/src/tests/snapshots.rs`, the snapshot directory). If a Narrow
   snapshot changes, your Narrow path diverged — fix the code, not the snapshot.
3. `dun-cli/src/main.rs` is the prelude hub — but you are not touching dun-cli
   this step; if a change seems to require it, STOP and report.
4. Match each file's local test style.
5. Stop-loss: if the same step fails twice for the same reason, STOP and report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Also paste:

```
grep -rn 'UnicodeWidthChar\|UnicodeWidthStr\|unicode_width' crates/dun-ui/src --include=*.rs | grep -v '/tests/'
```

which must be empty after this step (dun-ui no longer calls unicode-width
directly; dun-cli still does until step 4).

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working tree.
- Do NOT modify files outside Scope. If you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format

1. What changed — per file, line ranges, one-line why.
2. Verification — each command's verbatim output (suite counts; tmux/python
   skips noted) plus the grep result.
3. Verdict.
4. Stop-loss / open questions (empty if none).
