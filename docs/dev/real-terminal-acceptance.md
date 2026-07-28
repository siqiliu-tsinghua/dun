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
                     [--mouse] [--ascii] [--16color] [--mono] [--file PATH] \
                     [--lang TAG] [--syntax syntect|pygments|lua]
```

Resize the emulator window to the size the item asks for (usually **80×24**)
before reading the result. Quit dun with `Ctrl+Q`.

**Language is env-driven**, exactly as it is for a real user: dun reads the
first nonempty of `LC_ALL`/`LC_MESSAGES`/`LANG`, and `launch.sh` passes the
ambient environment through untouched — `LC_ALL=ja_JP.UTF-8 acceptance/launch.sh`
is the normal way to select one. What `launch.sh` must always do is copy `i18n/`
next to its throwaway config, because dun resolves catalogs relative to the
active config file; without that copy every launch is silently English.
`--lang TAG` is only sugar for sweeping all ten catalogs. It exists because a
catalog tag is **not** usable as a locale: `locale_candidates()` upper-cases the
region subtag, so `LC_ALL=zh-Hans` yields `["zh-HANS","zh"]` and matches
`zh-Hans.conf` only on a case-insensitive filesystem (macOS) — silently English
on Linux. `--ascii` forces English by design, so `--lang` + `--ascii` is
rejected rather than silently ignored.

### Helper scripts

| Script | What it does |
| --- | --- |
| `acceptance/gallery-run.sh R C -- ARGS` | pins the window to R×C with the `CSI 8;R;C t` sequence (kitty, iTerm2 and Terminal.app all honour it), then hands off to `launch.sh`. One geometry mechanism for all three emulators. |
| `acceptance/gallery-open.sh EMU R C -- ARGS` | opens that in `kitty`, `iterm` or `terminal`. kitty takes a command directly; the other two only accept a **script file** via `open -a`, so the args are baked into a generated `.command` wrapper (which also `cd`s to the repo, since those two run a `.command` from the user's home). |
| `acceptance/sweep-menus.sh` | headless: opens each top-level menu in each shipped language, saves the 80×24 grid. |
| `acceptance/sweep-states.sh` | headless: opens every dialog and editor state (via `Alt+<mnemonic>` then the entry's bare mnemonic) in each language. Never sends Enter, so Save As / Run Command are shown but never executed. |
| `acceptance/sweep-logfilter.sh` | headless: the log-filter plugin's full layout — its injected menu, both plugin-owned windows, and those windows beside a dun split — at 100×30, because two plugin windows plus a split do not fit in 80 columns. The only capture of a *foreign* window in the tiled layout. Chords come from `tmux_logfilter.rs`, which is their authority; `LOGFILTER_HOST=lua` selects the other host. |

The three sweeps drive dun in a **detached tmux session** — no GUI terminal is
involved and nothing is injected into any on-screen application. This is the
same mechanism the harness tests use (`crates/dun-cli/src/tests/tmux_*.rs`).
Text grids beat screenshots for i18n review: they diff, they grep, and every
row's display width can be checked mechanically. Mnemonics are stable Latin
characters in every catalog (the translated label carries them in trailing
parens), so one key sequence drives all languages.

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
| C2 | Rendering over SSH, every emulator x every VM | `acceptance/gallery-ssh.sh <debian\|freebsd\|solaris> 24 80`, launched through `gallery-open.sh` like a local capture |

The `vm-test/` VMs are convenient SSH targets. This is the only path that
exercises the full local-emulator ↔ SSH ↔ remote-dun chain that the local PTY
harness does not.

`gallery-ssh.sh` needs `ssh -t`: without a forced PTY the remote dun sees no
terminal at all. Window geometry is still set locally with `CSI 8 t` and
travels to the remote PTY over the link, so the remote editor lays out at the
size the local emulator was told to be.

**Make the remote host identify itself in the captured content.** The three VMs
run the same dun over the same fixture, so their captures are byte-identical —
a first pass produced three iTerm2 PNGs with the same MD5, and the only reason
kitty's and Terminal.app's differed was that their title bars happen to carry
the per-run wrapper filename. A screenshot that cannot distinguish "connected
to three machines" from "connected to one machine three times" proves nothing,
so the runner prepends `uname` + `hostname` to the remote fixture and every
capture carries its own provenance. The window title is not enough: iTerm2
shows only `ssh`.

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
| D7 | Syntax highlighting | `launch.sh --syntax syntect --file acceptance/fixture-code.rs` (also `lua`; `pygments` needs Pygments installed) |
| D8 | Capability fallbacks | `--ascii` / `--16color` / `--mono` |
| D9 | UI language | `LC_ALL=<locale> launch.sh` — one per script family (Han, Kana/Hangul, Cyrillic, long Latin) |

### Capturing a window

`screencapture -x -o -l<CGWindowID> out.png` grabs **one window by id**, from
its own backing store, so occlusion and stacking do not matter. This is what
makes the pass work at all: the Claude Code terminal window covers most of the
screen, and a full-screen grab is useless when the target is behind it. Get the
id from a window enumerator built on `CGWindowListCopyWindowInfo` (owner name +
title + bounds); match the window a launch created by diffing the id list
before and after.

**Do not time a `--syntax` shot with a guessed sleep.** The plugin status
indicator means the host *connected*, not that spans have been applied, so it
is not a readiness signal for a screenshot. Measure readiness instead: run the
same launch headlessly in a detached tmux pane and poll `capture-pane -p -e`
until an SGR colour change actually appears on a keyword line (`38;5;173m` on
`use std::collections::HashMap;` in the code fixture), then take the GUI shot
with that time plus a margin. Measured here: **both syntect and lua paint spans
1.0 s after dun is up** — the engine is not the slow part. What broke a 2 s
GUI wait was the whole launch chain in front of it (emulator start, the
launcher's own settle sleeps, the catalog copy, dun start, host spawn); lua's
extra interpreter spawn was only what tipped it over. A 5 s wait covers both.

## Pass of 2026-07-27 — i18n sweep + gallery

**Headless i18n sweep (220 grids, `acceptance/gallery/text/`).** 11 tags
(`en` + the ten catalogs) x 4 menus, plus 16 dialogs/states x 11 tags.
Mechanically checked: **no row exceeds 80 display columns**, **no dropdown
panel is clipped**, **entry counts match English everywhere** (file 10, edit 18,
view 19, help 1), and **no entry is left untranslated** in any catalog.

Widest dropdown panel, in display columns (English in brackets):

| Menu | Widest languages | English |
| --- | --- | --- |
| edit | **pt 65**, fr 61, es 57, ru 56 | 36 |
| view | it 46, fr 46, pt 43 | — |
| file | ko 42, ja 40, fr 40, de 39 | 31 |

Counter-intuitive but measured: the Romance languages, not German, are what
push the panels widest. Portuguese `Copiar para a área de transferência
externa (X)` plus `Ctrl+X,Ctrl+C` takes the Edit panel to **65 of 80 columns**,
leaving 15 columns of headroom; German is only 39. **The pt Edit menu is the
first thing that will overflow** if an Edit entry grows or a native-speaker
review lengthens that string — check it at 80 columns before landing either.

Translation-quality note (not a mechanism bug): `pt.conf` uses Brazilian
forms (`Arquivo`), while the tag maps to `pt_PT`; European Portuguese would be
`Ficheiro`. One for the native-speaker review.

**Gallery — 39 shots, 13 per emulator, 0 failures.** Capture sizes: kitty
1362x850, Terminal.app 1800x1198, iTerm2 1140x994.

**iTerm2 needs its own capture path.** `open -a iTerm <script>` opens a new
*tab*, not a new window, so the window-id-diff that works for kitty and
Terminal.app finds nothing after the first launch (a naive sweep scored 1/13)
and every shot carries tab-bar chrome. The fix is to **quit iTerm before each
shot**: the relaunch opens the script in a brand-new window with a single tab,
and iTerm hides the tab bar when a window has only one tab.

**Prerequisite: turn off iTerm2's startup windows** (its "restore windows" /
`OpenNoWindowsAtStartup` behaviour). Measured on a cold launch with the
default setting, iTerm creates **two windows of its own before the document
even opens** — so a quit-per-shot sweep flashes two junk windows per capture
and the run looks broken:

| cold launch | default setting | startup windows off |
| --- | --- | --- |
| no document | 2 windows | **0** |
| with document | 3 windows | **1** (the document) |

The junk windows are self-cleaning and never corrupted a capture, but one of
them carries a tab bar, which is what makes the run look like the sweep is
accumulating tabs. Note how to tell them apart **without opening a single
image**: at this font the document window is `570x462` logical and a
tab-barred one is `570x497`. A 35-point (70-pixel) height difference is a tab
bar. Read the geometry column, not the window count — counting windows is what
hid this for three rounds of testing.

Verify this mechanically rather than by eye: the tab bar is ~70 px tall, so a
clean iTerm capture is **1140x924** and a tabbed one **1140x994**.

**Height alone is not enough — check file size too.** Height only proves the
tab bar is absent; a window that was captured before dun painted is the right
height and completely blank, and will be recorded as a pass. PNG size catches
it: a blank frame compresses to a fraction of a rendered one. Flag any capture
under ~50% of that emulator's median size. In this pass exactly one shot of 39
was blank that way (`iterm-fallback-mono`, 29 KB against a 183 KB median, all
others 87–113%); a re-capture with a size assertion and retry fixed it. Bake
both oracles into the sweep — capture, assert `size > threshold` and
`height != tabbed`, retry otherwise.

**iTerm2 ignores `CSI 8 t` by default**, so its windows stay at the profile's
geometry — **25 text rows, not the requested 24** (visible as `View 1-21/36` in
the status bar). Sending the sequence a second time after the window settles
does not help. `DisableWindowSizeChangeEscapeCodes` and friends are unset here,
so this is iTerm's built-in default of treating window-resize sequences as
potentially insecure; enabling them is a user preference change, deliberately
not made for this pass. kitty and Terminal.app both honour the sequence and
give exactly 24 rows, so read the iTerm column as one row taller by design.

**D8 `--ascii` is a positive result, not a rendering failure.** Under
`terminal.encoding = ascii` the CJK fixture line renders as
`\u{5bbd}\u{5b57}\u{6d4b}\u{8bd5}` and the box-drawing chrome falls back to
`|`/`#`. That is the sanitizer doing its job: on a terminal declared
ASCII-only, every non-ASCII codepoint becomes a **visible, unambiguous,
lossless** escape instead of bytes the emulator would mangle. The UI is English
even under `LC_ALL=zh_CN.UTF-8`, which is the documented ASCII-forces-English
rule. Read these shots as evidence for the sanitize-controls invariant holding
in a real emulator — do not file them as missing-glyph bugs.

Engine difference visible in the D7 shots: syntect marks type names
(`Budget`, `new`) with the **emphasis** class and the lua mini-lexer does not,
so the same file renders with bold type names under one host and not the other.
Both are correct — the lua host is a deliberately minimal example.

### SSH pass of 2026-07-27

Nine captures, 3 emulators x 3 VMs, all at HEAD (`5934810`) built on each VM.
Verified mechanically rather than by eye — three rounds of this session were
lost to reading the wrong quantity, so the oracle is a headless 80x24 grid per
VM, not the images: **24 rows each, no row over 80 display columns, and the
only line that differs between the three is the host-identity line itself.**

Solaris's historical ambiguous-width problem did not reappear over SSH:
`◆ ○ ● │ ─ ┼ ┐ └ ± × ÷` and the CJK line render as they do locally, so the
stage-B width auto-detection survives the link.

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
