# Brief 046 — crossterm replacement step 5: dependency removal + docs closure

Implementation brief. **Step 5 ("Brief 5") of the accepted plan for brief 041**
— the final step. Steps 1–4 landed (`cf1a5b6`, `919a98f`, `d8f17c4`,
`5ffd477`): dun's terminal I/O is fully in-house and `crossterm` has ZERO
source references; the four-platform matrix is green (780/0 ×4). This step
removes the now-unused dependency and closes the documentation trail.

## Exact change

1. **Manifests** — remove `crossterm` from the workspace `Cargo.toml` dep
   table and from `crates/dun-cli/Cargo.toml`. Regenerate `Cargo.lock`
   (`cargo update` is NOT allowed — a plain build regenerates it; versions of
   surviving deps must not drift). Record the exact resulting package count
   in the report (projection was 26; the regenerated lockfile is
   authoritative).
2. **Docs closure** (AGENTS.md same-change rule; update each named doc where
   it names crossterm/mio as a runtime dependency or describes the old input
   path — surgical edits, not rewrites):
   - `README.md` — any dependency/architecture claims naming crossterm.
   - `AUDIT.md` — the terminal-safety wording that names crossterm (the
     restore/panic-hook invariants are now the sys shim's; keep the
     invariants, fix the attribution).
   - `docs/dev/dependency-audit.md` — the dependency table: crossterm family and
     mio out; rustix + signal-hook in with their feature sets and why; note
     unicode-width unchanged; record the new lockfile package count.
   - `docs/dev/terminal-compatibility-checks.md` — remove/replace any remaining
     claim that input parsing is crossterm's or that the PTY harness "does
     not answer the probe" (it does since brief 040); the supported input
     surface is the bounded matrix (xterm-family keys, SGR mouse, bracketed
     paste, CPR/DA1, SIGWINCH resize; kitty/modifyOtherKeys/X10/rxvt
     explicitly out of scope).
   - `docs/dev/crate-map.md` — dun-cli's terminal module now owns lifecycle +
     sys shim + VT core (output/event/parser) + event reader; no external
     terminal backend.
   - `docs/dev/file-splitting-plan.md` — only if it names the old event-loop/
     crossterm layout in a way the split made stale.
   - `docs/dev/runtime-resource-audit.md` — only the dependency-related
     wording; do NOT re-run resource measurements (Claude handles
     measurement at the gate).
   - `PLAN.md` / `TODO.md` / `PROGRESS.md` — mark the crossterm-replacement
     track complete per each file's existing conventions.
3. **Greps (paste in the report):**
   - `grep -rn crossterm --include='*.rs' crates/` → zero hits.
   - `grep -rn 'crossterm\|[^a-z]mio[^a-z]' README.md AUDIT.md docs/ | grep -v dev/codex | grep -v release-size-audit` →
     only intentional historical references remain (the size-audit history
     and the codex briefs are the archive — do not edit those).
   - `grep -n 'name = ' Cargo.lock | wc -l` → the exact package count.

## Scope

- Files you MAY modify: `Cargo.toml`, `crates/dun-cli/Cargo.toml`,
  `Cargo.lock`; `README.md`, `AUDIT.md`, `PLAN.md`, `TODO.md`,
  `PROGRESS.md`; `docs/{dependency-audit,terminal-compatibility-checks,crate-map,file-splitting-plan,runtime-resource-audit}.md`.
- Files/areas you MUST NOT touch: ANY `.rs` file (if removal breaks a build,
  STOP and report — it means step 4 left a reference), `docs/dev/codex/**`,
  `docs/dev/release-size-audit.md` (Claude records measurements), CLAUDE.md,
  AGENTS.md, `.git`, `i18n/**`, `hosts/**`, `vm-test/**`, `reference/**`.

If a change needs a file outside Scope, STOP and report.

## Deliverable

- The manifest/lockfile removal + the docs closure + the three grep outputs.
- No source change, no behavior change: the binary must build identically
  (Claude verifies size is ≈ step 4 on both platforms at the gate).

## dun pitfalls (read twice)

1. `cargo update` is forbidden — surviving dependency versions must not
   drift in the lockfile regeneration.
2. Docs edits are surgical: fix what is now false, keep the history. The
   size-audit doc and the codex briefs are intentionally historical.
3. Feature removal discipline (AGENTS.md): this is a dependency removal —
   the trail is manifests + lockfile + the named docs; there are no command
   ids/keymaps/help texts involved. If you find one that mentions crossterm,
   STOP and report instead of editing it.
4. Stop-loss: same failure twice, or an out-of-scope file needed → STOP,
   report.

## Verification (MANDATORY — run and paste verbatim)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

Plus the three greps. Claude runs the final dual-platform size gate, release
smoke, the dependency/duplicate audit reproduction, and the four-platform
re-verify at the gate.

## Hard rules

- Do NOT commit, branch, push, or touch git. Leave changes in the working
  tree.
- Do NOT modify files outside Scope; if you think you must, STOP and report.
- Minimal diff; no drive-by reformatting.
- Paste real verbatim verification output; if not green, say so.

## Report format

1. What changed — per file, one-line why.
2. Verification — verbatim outputs + the three greps + the exact package
   count.
3. Verdict.
4. Stop-loss / open questions (empty if none).
