# Brief 065 — Config: quote-aware comment scanning

Implementation brief. Plan approved from brief 064; the design questions are
**already decided below** — implement the specification, do not re-open it.

## Goal

`crates/dun-config/src/parser.rs` strips comments before honouring quotes, so a
`#` inside a quoted value silently corrupts it. Make comment scanning
quote-aware per the specification below, reject unterminated quotes with a
line-numbered error, and quote the path `scripts/install.sh` generates so an
install prefix containing `#` produces a working config. When you are done,
all three verified failures below behave correctly and no currently-valid
config changes meaning.

## Verified current behaviour (measured — treat as given, do not re-derive)

Three live instances of the one defect. `strip_comment` (`parser.rs:91-95`)
splits the raw line at the **first** `#`; `unquote_value` (`parser.rs:451-463`)
only strips quotes when the trimmed value starts and ends with the same quote
character and is at least 2 bytes long.

1. **Plugin command path.** Measured through `dun_config::parse_config`:

   ```
   plugin.log.command = "/opt/dun#tools/host"   ->  OK, command == "\"/opt/dun"
   ```

   Not merely truncated — the unbalanced leading `"` survives into a string
   that is then used as an **executable path**. An unterminated quote raises no
   error at all.

2. **Keybindings.** `parse_char_key` (`crates/dun-config/src/keys/key.rs:230-248`)
   accepts any single character, so `#` is a legitimate key. Measured:

   ```
   key.edit.find = Alt+#     ->  ERR  line 1: invalid key sequence: missing key
   key.edit.find = "Alt+#"   ->  ERR  line 1: invalid key sequence: unknown key `"Alt
   ```

   `#` is therefore unbindable today, and both error messages misdirect.

3. **Install prefix.** `scripts/install.sh:759` emits the host path unquoted:

   ```sh
   printf 'plugin.syntect.command = %s\n' "$(abs_path "$syntect_dest")"
   ```

   A prefix containing `#` yields a config line truncated at parse time. Fixing
   the parser alone leaves this broken, which is why it is in scope.

## Specification (decided — implement exactly this)

Comment and quote handling for `crates/dun-config/src/parser.rs` only:

1. A line whose first non-whitespace character is `#` is entirely a comment.
2. Otherwise split the line at the **first `=`**. (Today the split happens
   after comment stripping; this reorders the two operations.)
3. In the **key** part, a `#` starts a comment. Keys never contain `#`.
4. In the **value** part, let `v` be the value with surrounding whitespace
   trimmed:
   - **If `v` is exactly one character, it is literal.** No quote processing.
     This preserves today's behaviour for `key.<cmd> = "` and `key.<cmd> = '`,
     which bind the quote characters themselves and currently work because
     `unquote_value` requires `len >= 2`. A regression here is a silent loss of
     a working binding — cover it with a test.
   - Else if `v` begins with `"` or `'`, the value runs to the **matching
     closing quote**. `#` inside is literal. After the closing quote only
     whitespace and then an optional `#` comment may follow; anything else is
     an error. An unterminated quote is an error.
   - Else (`v` does not begin with a quote), an unquoted `#` starts a comment
     exactly as today, and any quote characters inside `v` are **literal**.
     `plugin.x.command = /opt/it's/host` must keep working — do not treat an
     apostrophe in an unquoted value as syntax.
5. New errors use `ConfigParseError::line` with the line number and a **static**
   message. Do NOT interpolate the offending bytes: fatal startup config errors
   are printed straight to stderr at `crates/dun-cli/src/main.rs:136`, before any
   UI sanitizer exists.

`scripts/install.sh`: emit the command path double-quoted. If the resolved path
contains a `"` or a newline, fail with a clear message rather than write a
config line that cannot parse.

`docs/configuration.md:9` currently reads "Blank lines and text after `#` are
ignored." Replace it with an accurate statement of the rules above, and say
plainly that a value containing `#` must be quoted. Note the one remaining
limitation: a value containing `#` **and both** quote characters is not
representable — v0.2 deliberately adds no escape syntax.

## Scope

- Files you MAY modify:
  - `crates/dun-config/src/parser.rs`
  - `crates/dun-config/src/tests/parser.rs`
  - `scripts/install.sh`
  - `docs/configuration.md`
- Files/areas you MUST NOT touch:
  - **`crates/dun-config/src/i18n.rs`.** It has its own `strip_comment`
    (`i18n.rs:198`) with the same shape, called from `parse_catalog`
    (`i18n.rs:129`). This is deliberate and **out of scope**: the catalog format
    has no quoting concept at all (values are `raw_value.trim()`), none of the
    ten shipped `i18n/*.conf` uses `#` in a value (verified: zero matches), and
    giving translators a new quoting syntax is a separate design decision. Do
    **not** unify, share, or "helpfully" refactor the two `strip_comment`
    functions. Duplication here is intentional.
  - `AGENTS.md`, `CLAUDE.md`, `README.md`, `TODO.md`, and all of `docs/**`
    except `docs/configuration.md`;
  - `.git`, git config, `Cargo.toml`, `Cargo.lock`;
  - `vm-test/**` (contains local SSH keys), `reference/**`.

## Deliverable

- The quote-aware scanner in `parser.rs`, replacing `strip_comment`'s role.
- The `install.sh` quoting plus its guard.
- The `docs/configuration.md` rewrite of the syntax rule.
- Tests in `crates/dun-config/src/tests/parser.rs`, table-driven, covering at
  minimum: quoted value containing `#`; unquoted value containing `#` (still a
  comment); trailing comment after a quoted value; single-character `"` and `'`
  values; apostrophe inside an unquoted path; unterminated quote (error, with
  the line number asserted); garbage after a closing quote (error); full-line
  comment; `#` as a keybinding via `key.edit.find = "Alt+#"` **and** the
  currently-working `Alt+z` control case.

## Test requirements (this is what the gate checks)

- **Independent oracle.** Write the expected value for each case out by hand.
  Never compute the expectation with a helper that reuses the scanner's own
  logic — a test whose oracle is the implementation cannot fail.
- **Name the executing path.** Every test must go through the public
  `dun_config::parse_config` / `parse_config_overlay`, not a private helper, so
  it exercises the path the editor actually runs.
- **No existing test may be edited.** Brief 064 verified that no current test
  asserts the broken behaviour (`tests/parser.rs:209-273` uses balanced quotes
  only). If you find yourself needing to change an existing assertion, STOP and
  report it — that means the change altered behaviour it should not have.
- Add a regression test that an invalid **user** config is fatal while an
  invalid **installed** config is reported and stepped over, using the new
  unterminated-quote error. Both layers run through
  `crates/dun-cli/src/config_loading.rs:247-255`; this keeps that split
  load-bearing. If that test does not belong in `dun-config`'s test module,
  say so in your report rather than reaching outside Scope.

## dun pitfalls (read twice)

1. **Safe Rust is forbidden-unsafe** (`#![forbid(unsafe_code)]` in every crate
   root).
2. **The 1 MiB dual-platform size budget is real.** `parser.rs` ships. Keep the
   scanner allocation-free and borrowing (`&str` slices) as it is today; do not
   introduce owned `String` values per config entry. No new dependencies.
   Claude measures on macOS + Debian before committing.
3. **Static error messages only** — see specification item 5.
4. **Tests are colocated**: `dun-config` uses `src/tests/*.rs` behaviour
   modules. Match the local style of the file you extend.
5. **Stop-loss is real.** If the same step fails twice for the same reason,
   STOP and report — do not keep tuning.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
sh -n scripts/install.sh
```

Also run `scripts/install.sh --dry-run` into a temporary prefix and paste the
generated plugin stanza, showing the path is now quoted.

Loop: edit → test → fix → rerun until green. Never claim a result without the
verbatim lines. The tmux-backed suite requires tmux; if it is unavailable those
tests skip cleanly — say so explicitly rather than reporting them green.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave changes in the
  working tree; Claude runs the authoritative gate and commits.
- Do NOT modify files outside Scope. If you believe you must, STOP and write
  that in the report instead.
- Full machine access, but touch NOTHING outside this repo, no network.
- Minimal diff: no drive-by reformatting, renames, or comment changes outside
  the task.
- You MUST paste real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. What changed — per file, with line ranges and a one-line why.
2. Verification — each command run, with exact verbatim output lines.
3. The specification items you implemented, each mapped to the test that
   proves it.
4. Stop-loss / open questions — where you stopped and why (empty if none).
