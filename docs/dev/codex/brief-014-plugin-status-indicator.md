# Brief 014 — Plugin status-bar indicator (theme warning color)

Implementation brief. Add an opt-in status-bar indicator showing the loaded
plugin host and flagging it — in the theme's `warning` color — when it is
resident but idle, or when its last request failed. Uses the `warning` palette
role from brief-012. Touches `dun-config`, `dun-ui`, and `dun-cli`.

## Goal

On a small remote box, a plugin host is a resident process. If it sits there
doing nothing, the user should be able to see that and `plugin unload` it. The
indicator makes that visible without any new UX surface.

1. Config (opt-in, off by default):
   - `plugins.status_bar = true | false` — show the indicator.
   - `plugins.idle_after_ms = 300000` — flag a loaded host as idle after this
     long with no activity. `0` disables idle flagging (errors still flag).
2. The indicator renders at the right edge of the status bar:
   - loaded, recently active → `[<id>]`, normal status styling;
   - loaded, idle past the threshold → `[<id> idle]`, **alert** styling;
   - last request failed → `[<id> error]`, **alert** styling;
   - unloaded (via the `plugin unload` command) → `[<id> off]`, normal styling.
3. Alert styling is derived from the theme, never hardcoded: it is the theme's
   `warning` pair **swapped** (see below).

Explicit non-goal: we do NOT measure the host's memory. Reading RSS needs
per-OS code (procfs / task_info) and costs bytes. "Resident and idle" is the
proxy the indicator reports; do not add any memory probing.

## Context pointers

- Read `AGENTS.md` first.
- `crates/dun-term` (do NOT modify): `Palette.warning: Style` exists, defined
  in every theme as `Style::new(<warning fg>, <editor bg>, BOLD)`.
- `crates/dun-cli/src/plugins.rs` — `PluginHighlighter`: one syntax-highlight
  host on a worker thread. It already has `plugin_id()`, `is_loaded()`,
  `load()`, `unload()`, `schedule(job) -> bool`, `poll() -> Vec<HighlightOutcome>`.
  `HighlightOutcome.result` is `Result<Vec<StyleSpan>, String>`.
  `PluginHighlighter::for_tests()` (cfg(test)) builds one without a worker.
- `crates/dun-cli/src/terminal/event_loop.rs` — after
  `frame_for_workspace_with_menu_selection(...)` the loop already overwrites
  `ui_frame.status.left` and `ui_frame.status.right`. Set the new plugin field
  there the same way. Do NOT change any `frame_for_workspace*` signature.
- `crates/dun-ui/src/model.rs` — `StatusBar { left, right, focused_window }`.
- `crates/dun-ui/src/frame/status.rs` — `UiShell::status_bar(...)` builds it.
- `crates/dun-ui/src/render/surface_layers.rs` — `draw_status` is the ONLY
  place the status row is painted (`fill_rect` + `set_text`).
- `crates/dun-ui/src/render/status.rs` — `sanitized_status_text_for_width`.
- `crates/dun-ui/src/render/chrome.rs` — `sanitize_chrome_text`.
- `crates/dun-ui/src/text.rs` — `display_width`, `status_text_for_width`.
- `crates/dun-config/src/config.rs`, `parser.rs`, `defaults.rs` — the config
  pattern to mirror (see `MouseConfig` / `mouse.enabled`).

## Specification

### 1. `dun-config`

Add:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginStatusConfig {
    pub status_bar: bool,
    pub idle_after_ms: u64,
}

impl Default for PluginStatusConfig {
    fn default() -> Self {
        Self { status_bar: false, idle_after_ms: 300_000 }
    }
}
```

- New `Config` field `pub plugin_status: PluginStatusConfig` (defaulted).
- Parser arms in `apply_config_entry` (note: these do NOT collide with the
  existing `plugin.` prefix, which is stripped earlier — `plugins.` has no dot
  after `plugin`):
  - `"plugins.status_bar"` → `parse_bool`, error text `expected true or false`.
  - `"plugins.idle_after_ms"` → `value.parse::<u64>()`, error text
    `expected an idle threshold in milliseconds`.
- `defaults.rs`: emit both under a `\n# Plugin status indicator\n` heading,
  uncommented, with the default values (mirror the Mouse section).
- Re-export `PluginStatusConfig` from `lib.rs`.

### 2. `dun-ui`

In `model.rs`:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginIndicator {
    pub text: String,
    pub alert: bool,
}
```

- `StatusBar` gains `pub plugin: Option<PluginIndicator>`.
- `UiShell::status_bar(...)` sets `plugin: None` (dun-cli fills it in).
- Re-export `PluginIndicator` from the crate root next to `StatusBar`.
- Every existing `StatusBar { .. }` literal in dun-ui (including tests) needs
  the new field; use `plugin: None`.

In `render/surface_layers.rs::draw_status`, paint the indicator at the right
edge, in its own style:

```rust
pub(crate) fn draw_status(surface: &mut Surface, shell: &UiShell, status: &StatusBar, area: TuiRect) {
    let style = shell.theme.palette.status_bar;
    let width = area.width as usize;

    let indicator = status
        .plugin
        .as_ref()
        .map(|plugin| (sanitize_chrome_text(shell, &plugin.text), plugin.alert));
    let indicator_width = indicator
        .as_ref()
        .map(|(text, _)| display_width(text))
        .unwrap_or(0);
    // Reserve the indicator plus one separating space; if it cannot fit, drop it.
    let reserved = if indicator_width == 0 || indicator_width + 1 > width {
        0
    } else {
        indicator_width + 1
    };
    let bar_width = width - reserved;

    surface.fill_rect(area.x, area.y, area.width, area.height, ' ', style);
    let text = sanitized_status_text_for_width(shell, status, bar_width);
    surface.set_text(area.x, area.y, &text, style);

    if reserved > 0 {
        let (text, alert) = indicator.expect("reserved implies an indicator");
        let indicator_style = if alert {
            alert_style(shell)
        } else {
            shell.theme.palette.status_text
        };
        let x = area.x + (area.width - indicator_width as u16);
        surface.set_text(x, area.y, &text, indicator_style);
    }
}

/// The theme's own warning pair, swapped: the warning foreground becomes the
/// chip background. The theme already guarantees those two colors contrast
/// (that is what `warning` means), so this reads on every palette — unlike
/// painting the warning foreground onto the status bar's own background, which
/// on `dun` would put red on cyan. Under `mono` the swap is a no-op and the
/// bold chip still stands out against the reverse-video bar.
fn alert_style(shell: &UiShell) -> Style {
    let warning = shell.theme.palette.warning;
    Style { fg: warning.bg, bg: warning.fg, attrs: warning.attrs }
}
```

Import `Style` from `dun_term` and `display_width` / `sanitize_chrome_text` as
needed. `sanitized_status_text_for_width` already takes an explicit width — do
not change its signature.

### 3. `dun-cli`

In `plugins.rs`, track activity on `PluginHighlighter`:

- New fields: `last_activity: Instant` (init `Instant::now()`), `failed: bool`
  (init `false`).
- `schedule(...)`: when it actually sends a job (the non-duplicate path), set
  `self.last_activity = Instant::now()`.
- `poll(...)`: collect outcomes as today; if any outcome arrived, set
  `self.last_activity = Instant::now()`, and set `self.failed` from the LAST
  outcome (`result.is_err()`).
- `load()` / `unload()`: reset `last_activity = Instant::now()` and
  `failed = false`.
- `for_tests()`: initialize the new fields the same way.

Add a pure, clock-injected status query (so tests need no sleeping):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PluginActivity { Off, Active, Idle, Error }

impl PluginHighlighter {
    pub(crate) fn activity_at(&self, now: Instant, idle_after: Option<Duration>) -> PluginActivity {
        if !self.is_loaded() {
            return PluginActivity::Off;
        }
        if self.failed {
            return PluginActivity::Error;
        }
        match idle_after {
            Some(threshold)
                if now.saturating_duration_since(self.last_activity) >= threshold =>
            {
                PluginActivity::Idle
            }
            _ => PluginActivity::Active,
        }
    }
}
```

In `AppState` (add the method in the module that already owns plugin helpers,
e.g. `app/highlight.rs`):

```rust
pub(crate) fn plugin_indicator(&self) -> Option<PluginIndicator> {
    if !self.config.plugin_status.status_bar {
        return None;
    }
    let highlighter = self.highlighter.as_ref()?;
    let idle_after = match self.config.plugin_status.idle_after_ms {
        0 => None,
        ms => Some(Duration::from_millis(ms)),
    };
    let id = highlighter.plugin_id();
    let (suffix, alert) = match highlighter.activity_at(Instant::now(), idle_after) {
        PluginActivity::Off => (" off", false),
        PluginActivity::Active => ("", false),
        PluginActivity::Idle => (" idle", true),
        PluginActivity::Error => (" error", true),
    };
    Some(PluginIndicator { text: format!("[{id}{suffix}]"), alert })
}
```

Use whatever field name `AppState` actually uses for the loaded config (check
`app/state.rs`; do not invent one). In `terminal/event_loop.rs`, after the
existing `ui_frame.status.right = …` line, add:

```rust
ui_frame.status.plugin = app.plugin_indicator();
```

## Scope

- Files you MAY modify:
  - `crates/dun-config/src/config.rs`, `parser.rs`, `defaults.rs`, `lib.rs`;
  - `crates/dun-config/src/tests/` (parser/config tests);
  - `crates/dun-ui/src/model.rs`, `lib.rs`, `frame/status.rs`,
    `render/surface_layers.rs`;
  - `crates/dun-ui/src/tests/` (fix `StatusBar` literals; add render tests);
  - `crates/dun-cli/src/plugins.rs`, `crates/dun-cli/src/app/highlight.rs`,
    `crates/dun-cli/src/app/state.rs` (only if a field/import is needed),
    `crates/dun-cli/src/terminal/event_loop.rs`;
  - `crates/dun-cli/src/tests/` (new indicator tests).
- Files/areas you MUST NOT touch:
  - `crates/dun-term/**` — the `warning` role is already correct;
  - `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `PROGRESS.md`, `TODO.md`, `docs/**`,
    `README.md` (Claude writes the docs);
  - `.git`, git config, any `Cargo.toml`, `Cargo.lock` (no new deps);
  - `dun-core`, `dun-plugin`;
  - `vm-test/**`, `reference/**`, `hosts/**`.

## Deliverable

- `PluginStatusConfig` + the two config keys + dump entries.
- `PluginIndicator`, `StatusBar.plugin`, and the `draw_status` painting.
- `PluginActivity` + `activity_at` + `AppState::plugin_indicator` + the
  event-loop line.
- Tests:
  1. `dun-config`: defaults are `status_bar = false` / `idle_after_ms =
     300_000`; `plugins.status_bar = true` and `plugins.idle_after_ms = 60000`
     parse; a non-numeric idle threshold is a line error; `default_config_text`
     contains both keys and still round-trips through `parse_config`.
  2. `dun-cli` (`plugins`): `activity_at` returns `Active` right after a
     scheduled job; `Idle` once `now` is past the threshold; `Error` after a
     failed outcome is polled; `Off` after `unload()`; and `Active` (never
     `Idle`) when `idle_after` is `None`. Drive the clock by passing a
     synthetic `now` (`Instant::now() + Duration::from_secs(…)`); do not sleep.
  3. `dun-cli`: `plugin_indicator()` returns `None` when
     `plugins.status_bar = false`, and `Some` with `alert = true` and text
     ending in `idle]` when the host is idle and the toggle is on.
  4. `dun-ui`: `draw_status` paints the indicator cells with the swapped
     warning style when `alert`, with `status_text` when not, leaves the rest
     of the row in `status_bar` style, and renders the row unchanged when
     `plugin` is `None`. Add a narrow-width case (e.g. width 4 with a long
     indicator) asserting no panic and no indicator drawn.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe.** `#![forbid(unsafe_code)]` is in force.
2. **The 1 MiB dual-platform size budget is real.** Claude gates size. No new
   dependencies, no memory probing, no per-OS code.
3. **Sanitized rendering is an invariant.** The indicator text goes through
   `sanitize_chrome_text` before it reaches the surface, like all chrome text.
4. **Never hardcode a color.** The alert style is derived from
   `palette.warning` exactly as specified. Do not introduce a new palette role,
   and do not edit `dun-term`.
5. **Do not change `frame_for_workspace*` signatures** — the event loop already
   post-processes `ui_frame.status`; follow that pattern.
6. **Width arithmetic.** `area.width` is `u16`, widths are `usize`. Use
   `saturating_*`; a status row narrower than the indicator must drop the
   indicator, not panic or wrap.
7. **Tests are layered and colocated.** No sleeping in tests — the clock is a
   parameter.
8. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Loop: edit → test → fix → rerun, until green. Paste verbatim output.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave changes in the
  working tree; Claude gates and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and report.
- Full machine access, but touch NOTHING outside this repo, no network. Only
  file edits within Scope, `cargo`, and `python3` for parsing output.
- Minimal diff; no drive-by reformatting or renames.
- Paste real verbatim verification output; if not green, say so.

## Report format (your final message)

1. What changed — per file, line ranges, one-line why.
2. Verification — each command with verbatim output lines.
3. The finding / verdict.
4. Stop-loss / open questions (empty if none).
