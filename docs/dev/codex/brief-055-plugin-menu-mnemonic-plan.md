# Brief 055 — plan: plugin menus lose their keyboard mnemonic under translation

**Design-only brief. Scope: NONE — produce a plan, write no code.**

## Goal

A plugin-contributed top-level menu is keyboard-unreachable in every language
for which its host supplies a translated label. Produce a concrete,
step-by-step implementation plan that fixes it, with a call-site inventory and
the decisions that the fix forces made explicit. Claude reviews and adapts the
plan, then dispatches the steps as separate implementation briefs.

Do **not** change any source file. The deliverable is the plan.

## The defect, with evidence

Reproduced live (log-filter host loaded, same keystroke, two locales):

| locale | menu bar | `Alt+L` opens the dropdown? |
| --- | --- | --- |
| `C` | `File  Edit  View  Help  Log Filter` | yes |
| `zh_CN.UTF-8` | `文件 (F)  编辑 (E)  视图 (V)  帮助 (H)  日志过滤` | **no** |

Note what the menu bar itself shows: dun's own translated labels carry `(F)`,
`(E)`, `(V)`, `(H)`; the plugin's translated label carries nothing. English
works only by accident — `Log Filter` happens to start with `L`.

## Mechanism (already traced — confirm, do not re-derive)

- `crates/dun-ui/src/hit.rs:287` `mnemonic_matches` resolves a label's
  mnemonic as `entry_mnemonic(label).or_else(|| label.chars().next())`. For
  `日志过滤` that yields `日`, which no `Alt+<letter>` chord can produce.
- `crates/dun-ui/src/frame/menu.rs:325` `menu_label` composes dun's own
  translated top-level labels as `format!("{base} ({mnemonic})")`, taking the
  mnemonic from the **compiled English label's first letter** — the rule
  `docs/i18n.md` documents under "Menu mnemonics are invariant".
- `crates/dun-ui/src/frame/menu.rs:368` appends the plugin items with
  `items.extend(self.plugin_menu_items.iter().cloned())` — as-is. That rule is
  never applied to them.
- The plugin items are resolved on the dun-cli side:
  `crates/dun-cli/src/app/highlight.rs:87`
  `self.plugin_hosts.resolved_menu_items(&self.plugin_menu_tags)`.
- Host-supplied labels look like
  `hosts/python-logfilter/dun-python-logfilter-host.py:73`:
  `"top_label": {"en_US": "Log Filter", "zh-CN": "日志过滤"}`.

## A second, related gap — decide whether it is in or out

Plugin **dropdown entries** carry no mnemonic at all, even in English: the
reference host's items render as `Edit Pattern`, `Apply Pattern`,
`Show Status`, with no trailing `(M)`. dun's own entries always have one
(`Open... (O)`), and `crates/dun-cli/src/terminal/input.rs` dispatches a bare
letter inside an open dropdown via that mnemonic. So bare-letter dispatch
inside a plugin menu does not work in any language.

State in the plan whether you fold this into the same change or keep it
separate, and why. Either answer is acceptable if argued.

## Questions the plan MUST answer

1. **Where does the composition belong** — in `dun-ui`'s `menu_bar`, or where
   dun-cli resolves the host label (`resolved_menu_items`)? Give the
   consequence for each: who owns the English label at that point, and which
   layer already has the catalog.
2. **Where does the mnemonic come from** for a plugin? The host's `en_US`
   label's first letter is the obvious analogue of the built-in rule — say so
   explicitly, and say what happens when a host supplies **no** `en_US` label,
   or one starting with a non-ASCII character or a digit.
3. **Collisions.** dun's own set (F/E/V/H) is unique and tested. A plugin
   whose English label starts with `F` would shadow `File`. What is the
   policy — reject the contribution, keep it but unreachable, or pick another
   letter? There is precedent to follow: plugin **keybindings** already have a
   rejection path with a user-visible reason (`resolved_keybindings` returns
   rejections; a silent rejection was treated as a bug and fixed). Say whether
   menus should mirror it.
4. **Two plugins colliding with each other**, same question.
5. **What must NOT change**: dun's own menus must stay byte-identical in
   English and in every shipped catalog; menu indices must stay stable for
   dispatch and hit testing (`menu.rs:368` says plugin menus trail the
   built-ins for exactly that reason); mouse hit testing goes through the
   same labels.

## Deliverable

A plan, in the report, containing:

1. **Call-site inventory** — every place that reads a top-level menu label,
   composes one, or matches a mnemonic against one, with `path:line` and a
   one-line role. Include the mouse path, not just the keyboard one.
2. **Ordered steps**, each one an implementable unit: files touched,
   functions changed, what stays invariant, and the specific test that gates
   it. Steps must be independently gateable — Claude runs the full gate per
   step.
3. **The decisions** from the questions above, each with the reason.
4. **Tests to add**, named, including at least one that fails today: a
   plugin menu with a translated label must open on the same chord that opens
   it in English.
5. **Risks / open questions** — anything you could not settle from the code,
   and what evidence would settle it.

Everything with `path:line` evidence. Where you are unsure, say so rather
than guessing.

## Scope

- Files you MAY modify: **NONE**. This is a planning brief.
- Do not create, edit, or delete any file in the repo. Report only.

## Context pointers

- `AGENTS.md` — invariants and engineering rules.
- `docs/i18n.md` — "Menu mnemonics are invariant" is the rule being extended.
- `docs/plugin-protocol.md` — the capability model; the `menu` capability says
  "label i18n required" and the ownership section says dun owns the UI object
  and the plugin only supplies data.
- `crates/dun-ui/src/frame/menu.rs`, `crates/dun-ui/src/hit.rs`,
  `crates/dun-cli/src/app/highlight.rs`, `crates/dun-cli/src/terminal/input.rs`.
- The working tree has unrelated uncommitted changes; ignore them and do not
  revert, stash, or commit anything.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config.
- Do NOT modify ANY file. If you believe the plan requires a spike to answer a
  question, describe the spike instead of running it.
- Full machine access, but touch NOTHING outside this repo, no network. You
  may run read-only `cargo` commands (`cargo tree`, `cargo test --list`) and
  `python3`/`grep` for inspection.
- If a question cannot be answered from the code, say so in "Risks / open
  questions" rather than inventing an answer.

## Report format (your final message)

1. Call-site inventory.
2. Ordered steps with per-step gate tests.
3. Decisions, with reasons.
4. Tests to add.
5. Risks / open questions.
