# Brief 057 — step B: give plugin menus a keyboard mnemonic

Step B of the brief 055 plan, on top of step A (`fdda513`, already committed).
This is the behavior change.

## Goal

A plugin-contributed top-level menu must open on the same `Alt+<letter>` chord
in every language, the way built-in menus already do. Today the composition
rule is applied only to built-ins, so a host that supplies a translated label
becomes keyboard-unreachable:

| locale | menu bar | `Alt+L` opens it? |
| --- | --- | --- |
| `C` | `File  Edit  View  Help  Log Filter` | yes |
| `zh_CN.UTF-8` | `文件 (F)  编辑 (E)  视图 (V)  帮助 (H)  日志过滤` | **no** |

English works only by accident: `Log Filter` starts with `L`.

## Read step A first

`fdda513` added the primitives you must reuse — do **not** write a second copy
of these rules:

- `dun_ui::english_menu_mnemonic(label) -> Option<char>` — first character,
  accepted only when ASCII alphabetic, uppercased. This is the rule to apply
  to a plugin's `en_US` label.
- `dun_ui::compose_translated_menu_label(base, mnemonic) -> String` — exactly
  `"{base} ({mnemonic})"`.
- `dun_ui::built_in_menu_mnemonics()` — the built-in set, derived from `MENUS`.
  **Seed the claimed set from this. Never hard-code F/E/V/H in implementation
  code**; a hard-coded list is allowed only inside a test, as the oracle.

## Decisions — already frozen by Claude, do not redesign

1. **Composition happens in dun-cli's plugin resolver**, not in
   `UiShell::menu_bar`. dun-cli owns the `LabelSet`, the English fallback and
   the locale tags; dun-ui only ever sees a finished string.
2. **Mnemonic = `english_menu_mnemonic(en_US label)`.** Not an ASCII letter →
   reject that menu subtree and say why. Do not scan later characters and do
   not auto-assign a different key.
3. **Collision with a built-in → reject the subtree.** Auto-picking another
   letter would make a plugin's chord depend on load order.
4. **Two plugins colliding → configuration order wins**; the later subtree is
   rejected and diagnosed. No stale reservation: if the earlier host unloads,
   the next refresh may accept the later one.
5. **Rejections are never silent.** Mirror `resolved_keybindings`
   (`crates/dun-cli/src/plugins.rs:499`), which already drops a whole
   contribution and reports it. Menus need a richer reason because they have
   two distinct failures.
6. **English output is unchanged**: a plugin's English label stays
   `Log Filter`, never `Log Filter (L)` — exactly as built-ins are `File`, not
   `File (F)`. Only the translated form gains the suffix.
7. **Dropdown entries are out of scope.** Plugin dropdown entries carry no
   mnemonic even in English; that needs its own policy and its own change.
   Leave them exactly as they are.

## Two corrections to the plan you produced

- **There is no `crates/dun-cli/src/plugins/menu.rs`.** The module split you
  proposed as step 1 was deliberately dropped: the code-organization guideline
  for a >35k file permits *stating* the split boundary instead of starting it,
  and mixing a refactor into a behavior fix is how this repo has twice lost
  coverage silently. Put the work in `crates/dun-cli/src/plugins.rs` and
  `crates/dun-cli/src/tests/plugins.rs` as they stand. **Do not split any
  file.**
- **`PROGRESS.md` is not yours to touch.** Claude maintains it.

## Scope

- Files you MAY modify:
  - `crates/dun-plugin/src/menu.rs` (the `LabelSet` accessors)
  - `crates/dun-cli/src/plugins.rs` (resolver + rejection types)
  - `crates/dun-cli/src/app/highlight.rs` (refresh + reporting)
  - `crates/dun-cli/src/app/state.rs` (rejection tracking)
  - `crates/dun-cli/src/ui_text/` (the two new status keys)
  - `crates/dun-cli/src/tests/plugins.rs`, `crates/dun-cli/src/tests/i18n.rs`
  - `crates/dun-cli/tests/tmux_logfilter.rs`, and
    `crates/dun-cli/tests/support/tmux.rs` **additively only** — see below
  - `crates/dun-plugin/tests/protocol.rs`
  - all ten `i18n/*.conf`
  - `docs/i18n.md`, `docs/plugin-protocol.md` (these two docs only)
- MUST NOT touch: `AGENTS.md`, `CLAUDE.md`, `README.md`, `PROGRESS.md`,
  `TODO.md`, any other `docs/**`, `.git`, `Cargo.toml`, `Cargo.lock`,
  `vm-test/**`, `reference/**`, `acceptance/**`, `hosts/**`,
  `crates/dun-core/**`, `crates/dun-config/**`, `crates/dun-ui/**`.

Note `crates/dun-ui/**` is closed: step A put everything you need there
already. If you believe you must change dun-ui, STOP and report why.

## Implementation

1. `LabelSet` gains `fallback()` and `resolve_translation(tags)`; keep
   `resolve` as a compatibility wrapper. The resolver must be able to tell
   "an active translation was selected" from "fell back to English" **even
   when the two strings are equal**.
2. `resolved_menu_items` returns accepted items *plus* typed rejections —
   shape it like `resolved_keybindings`. Suggested:
   `ResolvedPluginMenus { items, rejections }`,
   `PluginMenuRejection { plugin_id, reason }`, with reasons
   `InvalidEnglishMnemonic` and `MnemonicConflict(char)`.
3. Seed claimed mnemonics from `built_in_menu_mnemonics()`, then walk plugin
   menus in configuration order: derive from `en_US`; require an ASCII letter;
   **reject a raw English label whose trailing `(X)` would make the existing
   matcher pick a different mnemonic than the first-character rule** (only
   when it actually contradicts — `Log Filter (L)` does not); reject if the
   mnemonic is already claimed; otherwise claim it, leave the English label
   untouched, and compose the translated form.
4. Accepted menus stay in configuration order and still trail all built-ins.
5. Track rejections in `AppState`; report each newly rejected contribution
   once, keep them in status history, and allow re-reporting after the
   rejection clears. Two new translated status keys, roughly:
   - `Plugin {} menu ignored: en_US label has no valid mnemonic`
   - `Plugin {} menu ignored: mnemonic {} conflicts`
   Add them to `ui_text::ALL` **and all ten catalogs** — the completeness test
   at `crates/dun-cli/src/tests/i18n.rs:24` enforces it.
6. Document the rule in `docs/i18n.md` (next to "Menu mnemonics are
   invariant") and in the `menu` capability section of
   `docs/plugin-protocol.md`: derivation, the ASCII constraint, collision
   order, subtree-only rejection, and the diagnostic.

### The tmux harness change must be additive

`crates/dun-cli/tests/support/tmux.rs` deliberately pins the locale so tests
are environment-independent. Add an **opt-in** way for one test to request a
locale; do not change the default for existing tests. If that turns out to
require restructuring the harness, STOP and report — a shared test harness
change that silently alters other tests is worse than a missing live test.

## Tests

At minimum, and named as in the plan:

- `translated_plugin_menu_opens_on_same_alt_chord_as_english` — **this must
  fail before your change**; say so in your report with the failure output.
- `translated_plugin_menu_composes_the_english_mnemonic` — exact labels
  `Log Filter` and `日志过滤 (L)`.
- `plugin_menu_rejects_non_ascii_english_mnemonic`
- `plugin_menu_rejects_digit_english_mnemonic`
- `plugin_menu_rejects_conflicting_embedded_english_mnemonic`
- `plugin_menu_colliding_with_builtin_is_rejected_and_reported` — an `F...`
  label; assert File stays index 0, no plugin menu installs, and the status
  names the plugin and `F`.
- `later_plugin_with_duplicate_mnemonic_is_rejected_and_reported`
- `unloading_first_claimant_promotes_later_plugin_menu`
- `accepted_plugin_menu_reports_no_rejection`
- `plugin_menu_resolution_preserves_builtin_labels_and_indices` — across every
  shipped catalog, the built-in prefix must be byte-identical before and after
  an accepted plugin is added.
- `translated_plugin_menu_is_mouse_hittable_at_its_rendered_columns` — use
  hard-coded expected labels/columns, **not** values derived from
  `menu_index_at_column`, or the test proves nothing.
- `tmux_translated_logfilter_menu_opens_with_english_alt_l` — real host, both
  locales, same `M-l`.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe** (`#![forbid(unsafe_code)]`).
2. **The 1 MiB dual-platform size budget is real.** Claude gates this with
   release builds on macOS AND Debian. Translations are external files and
   cost nothing; new runtime code does. Keep the resolver concrete — no new
   generic layers, no new dependency.
3. **No panics on a render or refresh path.** Step A's review specifically
   replaced an `expect` in `menu_label` with a graceful fallback because it
   runs every frame. Do not reintroduce that pattern: a malformed plugin
   contribution must degrade to a rejection with a status message, never a
   panic.
4. **All untrusted text goes through the sanitizer.** Plugin labels are
   untrusted input. They are already validated on the protocol side; do not
   add a path that prints them raw.
5. **`crates/dun-cli/src/main.rs` is the prelude hub** — update its imports in
   the same change if you add or move a symbol.
6. **Terminal-detection env is pinned in harnesses** — see the additive-only
   rule above.
7. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report. Also stop if the change starts requiring files outside
   Scope.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Paste verbatim. tmux-backed tests skip cleanly without tmux — say so rather
than reporting them green.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify files outside Scope. If you believe you must, STOP and report.
- Full machine access, but touch NOTHING outside this repo, no network.
- Minimal diff: no drive-by reformatting, renames, or module splits.
- You MUST paste real verbatim verification output. If a run did not reach
  green, say so — never fake it.

## Report format (your final message)

1. What changed — per file, line ranges, one-line why.
2. The pre-change failure output of
   `translated_plugin_menu_opens_on_same_alt_chord_as_english`.
3. The two status key names and their eleven values for one of them.
4. What you did in `support/tmux.rs` and why it cannot affect existing tests.
5. Verification — verbatim.
6. Stop-loss / open questions.
