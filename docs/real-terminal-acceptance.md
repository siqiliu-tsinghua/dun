# Real-Terminal Acceptance Checklist

A focused, human- (or computer-use-) driven acceptance pass for the parts of
dun's terminal behavior that the **byte-precise automated harness cannot
observe**, plus a gallery-screenshot pass for docs/website use.

This **complements, never replaces**, the automated real-terminal tests
(`crates/dun-cli/tests/{pty_smoke,terminal_grid,tmux_grid,tmux_logfilter}.rs`,
see [real-terminal-tui-testing.md](./real-terminal-tui-testing.md)). Those run
dun in a real PTY/tmux and assert on the normalized character grid at byte
precision, in CI, in seconds — that is the source of truth for rendering and
input dispatch. This checklist is for what a harness that *is* the terminal
structurally can't test: the real emulator's own policy/quirks, the visual
perception layer, and the real-SSH transport.

It is **not a CI gate**: it is screenshot-brittle, slow, single-machine, and
run by a human or a computer-use agent as a pre-release / periodic check.
Results are recorded as an "observed on `<emulator> <version>`" matrix — never
extrapolated into a universal support claim.

## Prerequisites

- **Emulators** (macOS): Terminal.app (built in); iTerm2
  (`brew install --cask iterm2`); kitty (`brew install --cask kitty`). Test on
  whichever are installed; record which.
- **A dun binary**: `scripts/release-build.sh` (preferred) or
  `cargo build --release`. `acceptance/launch.sh` finds or builds one.
- **Fixture + launcher**: `acceptance/fixture.txt` and `acceptance/launch.sh`
  (writes a throwaway `DUN_CONFIG`, never touches `~/.config/dun`).
- **One-time emulator config for OSC 52 read** (most disable it by default):
  - iTerm2: Settings → General → Selection → "Applications in terminal may
    access clipboard" (on; may still prompt).
  - kitty: `clipboard_control ... read-clipboard read-primary` (default is
    ask/deny for reads).
  - Terminal.app: no OSC 52 read support — expect the internal fallback path.

## Running

```
acceptance/launch.sh [dun|msedit|dark|turbo] [--osc52-read] [--osc52-write] \
                     [--mouse] [--ascii] [--16color] [--mono] [--file PATH]
```

Resize the emulator window to the size the item asks for (usually **80×24**)
before reading the result. Quit dun with `Ctrl+Q`.

## Section A — Cross-checks against the byte-precise harness (calibration)

Each item has a harness test that holds the **byte-truth**; the real-terminal
screenshot confirms the emulator renders the same thing. A divergence is an
**emulator/font finding** (e.g. a wide glyph the emulator draws differently
than `unicode-width` predicts — the class that bit Solaris). Do these first:
they calibrate that "what I see in the emulator" == "what the harness asserts".

| # | Scenario | Harness test that holds the truth |
| --- | --- | --- |
| A1 | Baseline layout at 80×24 (menu bar, bordered window, gutter, status bar) | `tmux_grid_renders_baseline_layout_80x24` |
| A2 | ASCII + 16-color fallback chrome (`--ascii --16color`) | `tmux_grid_ascii_16_fallback_uses_ascii_chrome_and_no_256_sgr` |
| A3 | The wide/ambiguous/box-drawing fixture line renders aligned, no overlap | `tmux_grid_normalizes_cursor_and_sgr_attributes` + the ambiguous-width suite |
| A4 | Cursor block and a `Ctrl+L` line selection land on the expected cells | `terminal_grid` cursor/selection assertions |
| A5 | An open dropdown menu matches the menu chrome | `menu_matrix` snapshot |

For each: run the harness test (`cargo test -p dun-cli --test tmux_grid`, etc.),
then reproduce the same state in the emulator and screenshot. Record agree /
diverge (+ what diverged).

## Section B — Emulator-dependent (the harness cannot observe)

| # | Check | How | Pass = |
| --- | --- | --- | --- |
| B1 | OSC 52 **write** round-trip | `--osc52-write`; select text, `edit.copy_external`; in another shell `pbpaste` | `pbpaste` shows the selection (or: emulator filtered it, internal clipboard still works) |
| B2 | OSC 52 **read** | `--osc52-read`; `printf hello \| pbcopy`; in dun `Ctrl+X,Ctrl+V` | `hello` pastes at the cursor; on an unsupporting terminal, the internal-fallback status appears within ~500 ms |
| B3 | Modifier-key delivery | Try each: `Ctrl+Shift+Left/Right` (word-select), `Alt+←/→/↑/↓` (window focus), `Ctrl+Backspace/Delete` (word delete), `Shift+PageUp/Down`, `F1/F3/F5/F6`, `BackTab` | each triggers its binding (the emulator actually sends the sequence dun expects) |
| B4 | Wide / combining / box-drawing **visual** | open the fixture, look at the CJK/ambiguous/combining lines | no overlap, no cell misalignment, cursor tracks correctly in this emulator's font |
| B5 | Themes + capability fallback | `dun`/`msedit`/`dark`/`turbo`; then `--16color`, `NO_COLOR=1 … --mono`, `--ascii` | colors/attributes look right; mono emits only bold/underline/reverse; ascii uses ASCII chrome |
| B6 | Mouse | `--mouse`; click-to-focus a split, drag-select, wheel-scroll, click a menu, drag the scrollbar | each interaction works via real emulator mouse reporting |
| B7 | Terminal restore | (a) `Ctrl+X,S` shell-escape then `exit`; (b) quit with `Ctrl+Q`; (c) a panic build (`--features test-panic-hook`, then `DUN_TEST_PANIC=1`) | the prompt returns clean: cursor visible, no stuck raw mode, mouse reporting off, alt-screen restored |
| B8 | Resize (SIGWINCH) | drag the window / change font size while dun runs | layout reflows; wrapped rows and the gutter recompute; no corruption |

## Section C — Real SSH end-to-end

| # | Check | How |
| --- | --- | --- |
| C1 | The seven-step remote loop (README "Product Goal") | from the emulator, `ssh` into a host/VM, run dun on a remote file, edit + save; confirm OSC 52 write/read pass through the local emulator's clipboard |

The `vm-test/` VMs are convenient SSH targets. This is the only path that
exercises the full local-emulator ↔ SSH ↔ remote-dun chain that the local PTY
harness does not.

## Section D — Gallery screenshots (docs / website)

Fixed fixture + fixed 80×24 (or a stated larger size) + per emulator, for a
consistent gallery. Save as `acceptance/gallery/<emulator>-<subject>.png`
(the `gallery/` dir is git-ignored; curate deliberately before committing any
into docs).

| # | Shot | Launch |
| --- | --- | --- |
| D1 | Each theme, full editor | `launch.sh dun` / `msedit` / `dark` / `turbo` |
| D2 | Tiled split layout | any theme, then `Ctrl+X,H` / `Ctrl+X,V` to split |
| D3 | Open dropdown menu | any theme, `Alt+F` / `Alt+V` |
| D4 | Visible whitespace + bookmarks (the restored F12/F13) | `Ctrl+X,.` then `Ctrl+X,K` on a couple of lines |
| D5 | Log-filter plugin surface | configure a `hosts/` log-filter host, trigger its menu |
| D6 | Wide-glyph / CJK rendering | the fixture's CJK lines, dun theme |

## Results matrix (fill per run)

Copy this per pass; one row per item, one column per emulator. Record
version + date at the top.

```
Emulators: Terminal.app __,  iTerm2 __,  kitty __        Date: ____-__-__

item        Terminal.app   iTerm2   kitty    notes
A1
A2
A3
A4
A5
B1
B2
B3
B4
B5
B6
B7
B8
C1
```

Legend: ✓ pass · ✗ fail (defect) · — unsupported-by-terminal (expected;
fallback verified) · n/a not applicable.

## Limits (read before trusting a result)

- Screenshot/visual observation is fuzzier than the harness's grid diff; use
  Section A to calibrate, and prefer the harness verdict when they disagree
  about a rendering the harness covers.
- Single machine, single font set, the installed emulator versions only. Never
  turn an "observed on iTerm2 3.6" into "all xterm-family terminals".
- Not a regression gate — timing/focus/DPI make it flaky. It is a periodic /
  pre-release human-grade check, and the source for the terminal-support notes
  in [terminal-compatibility-checks.md](./terminal-compatibility-checks.md).
