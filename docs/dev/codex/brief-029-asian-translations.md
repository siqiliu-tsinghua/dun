# Brief 029 — Japanese and Korean (`ja`, `ko`)

Translation brief, and the last of the language set. Produce two complete files:

| file | language | serves |
| --- | --- | --- |
| `i18n/ja.conf` | Japanese | `ja_JP` |
| `i18n/ko.conf` | Korean | `ko_KR` |

Bare language tags: neither language has a regional split that needs one.

## Why these two are last, and different

Japanese and Korean are **verb-final**. English says
`Find: {}/{} matches: {}` — count, total, query — and no natural Japanese
sentence can carry the arguments in that order. This is exactly why indexed
placeholders exist (they landed in `058e0ff`, days before this brief), and
these are the two languages that most need them.

**Use them.** A translation that contorts itself to keep English argument order
is a worse translation than one that reorders. Russian used 15 of the 33
multi-argument templates; expect Japanese and Korean to use at least as many.

```text
# English default (positional, filled left to right)
status.find.match = Find: {}/{} matches: {}

# Japanese: the query first, the counts after — impossible with positional {}
status.find.match = 検索：{2} — {0}/{1} 件目
```

Rules for indexed templates (enforced by the validator):

- a value is positional **or** indexed, never both;
- an indexed value must use every index in `0..arity` at least once (repeats
  are allowed), and none beyond it — skipping an index would silently drop a
  runtime value;
- get the arity wrong and the value is ignored, so English renders instead.

## Goal

Two files, each complete (all 499 keys), each passing
`every_shipped_translation_is_valid_and_complete` unchanged, each reading as
natural software UI text to a native speaker.

## Context pointers

- `i18n/ru.conf` — the closest model: read its indexed templates first, they
  show the shape.
- `i18n/zh-Hans.conf`, `i18n/zh-Hant.conf` — CJK examples of the format, the
  section structure, and how the vocabulary rule looks in a CJK language.
- `docs/i18n.md` — "Menu mnemonics are invariant", "Reordering arguments",
  "Validating a translation".
- `crates/dun-cli/src/tests/i18n.rs` —
  `every_shipped_translation_is_valid_and_complete` discovers every
  `i18n/*.conf` and prints exactly which keys are missing. **That list is your
  worklist.**

## Rules (inherited; do not re-derive)

1. **The vocabulary rule.** Anything the user types back stays English and is
   passed as a `{}` argument: file paths, command ids (`edit.move_left`), theme
   names (`msedit|turbo|dark|dun`), config-diagnostics section tokens, key names
   (`Enter`, `Ctrl+X`), and the command-name list inside the command-line help.
   If a value in `zh-Hans.conf` leaves something in English, yours does too.
2. **The destructive-action guard.** `confirm.button.save`,
   `confirm.button.discard`, `confirm.button.cancel` must be present and
   **pairwise distinct** — they are drawn beside the literal `(s)`, `(d)`, `(c)`
   keys the dialog answers to, and two of them reading alike is how a user
   presses `(d)` and loses unsaved work. Suggested: Japanese
   **保存 / 破棄 / キャンセル**; Korean **저장 / 버리기 / 취소**. Do this first.
3. **No mnemonics in values.** The editor appends `(F)`, `(N)` from the English
   labels: `menu.file = ファイル`, never `menu.file = ファイル (F)`.
4. **Loader limits.** One line per key, value ≤ 256 bytes (note: CJK characters
   are 3 bytes each in UTF-8 — a 90-character value is 270 bytes and will be
   rejected; keep status messages tight), nothing the display sanitizer would
   escape.

## Terminology

**Japanese** — standard software conventions, katakana where the industry uses
it: ファイル (File), 編集 (Edit), 表示 (View), ヘルプ (Help), 新規 (New), 開く
(Open), 保存 (Save), 名前を付けて保存 (Save As), 閉じる (Close), 終了 (Quit),
元に戻す (Undo), やり直し (Redo), 切り取り (Cut), コピー (Copy), 貼り付け
(Paste), クリップボード (Clipboard), 検索 (Find), 置換 (Replace), ウィンドウ
(Window), バッファ (Buffer), 選択範囲 (Selection), 読み取り専用 (Read-only),
見つかりません (Not found), 権限がありません (Permission denied), プラグイン
(Plugin), 設定 (Config), タイムアウト (Timed out), 上書き (Overwrite).

**Korean** — 파일 (File), 편집 (Edit), 보기 (View), 도움말 (Help), 새로 만들기
(New), 열기 (Open), 저장 (Save), 다른 이름으로 저장 (Save As), 닫기 (Close),
끝내기 (Quit), 실행 취소 (Undo), 다시 실행 (Redo), 잘라내기 (Cut), 복사
(Copy), 붙여넣기 (Paste), 클립보드 (Clipboard), 찾기 (Find), 바꾸기 (Replace),
창 (Window), 버퍼 (Buffer), 선택 영역 (Selection), 읽기 전용 (Read-only), 찾을
수 없음 (Not found), 권한 없음 (Permission denied), 플러그인 (Plugin), 설정
(Config), 시간 초과 (Timed out), 덮어쓰기 (Overwrite).

Punctuation: use the language's own conventions (Japanese `：`/`、`/`。`;
Korean uses ASCII `:` and `,` normally). Do not copy Chinese punctuation
blindly.

## File header

```text
# Japanese (ja) UI translation for dun.
# Serves ja_JP via the locale chain (docs/i18n.md).
#
# Machine-translated and NOT reviewed by a native speaker. Corrections are
# welcome; see docs/i18n.md for the rules a value must satisfy.
```

Say the "not reviewed by a native speaker" part plainly — it is true, and a user
deciding whether to trust the UI deserves to know. Keep the same
section-comment structure as the other files.

## Scope

- Files you MAY create: `i18n/ja.conf`, `i18n/ko.conf`.
- **Nothing else.** No code, no tests, no docs, no existing translation. If the
  validator rejects a file, the file is wrong, not the validator — and if you
  genuinely believe otherwise, STOP and report rather than editing anything else.

## Order of work, and stop-loss

`ja` first, then `ko`. Finish one file completely and get the validator green for
it before starting the other. If you run long, stop at the file boundary with
everything green and say how far you got.

## Verification (MANDATORY — you run it; iterate to green)

```
cargo test -p dun-cli --bin dun i18n
cargo test --workspace --no-fail-fast
```

Then paste, verbatim, the output of:

```
python3 - <<'EOF'
import glob, re
def load(p):
    d = {}
    for line in open(p):
        line = line.strip()
        if '=' in line and not line.startswith('#'):
            k, v = line.split('=', 1)
            d[k.strip()] = v.strip()
    return d
for path in ['i18n/ja.conf', 'i18n/ko.conf']:
    cat = load(path)
    s, d, c = (cat.get(f'confirm.button.{k}') for k in ('save', 'discard', 'cancel'))
    indexed = [k for k, v in cat.items() if re.search(r'\{\d+\}', v)]
    longest = max((len(v.encode()), k) for k, v in cat.items())
    print(f"{path}: keys={len(cat)} distinct={len({s,d,c})==3} "
          f"save={s!r} discard={d!r} cancel={c!r}")
    print(f"  indexed templates: {len(indexed)}")
    print(f"  longest value: {longest[0]} bytes ({longest[1]}) — cap is 256")
EOF
```

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave the new files in
  the working tree; Claude reviews them and commits.
- Do NOT modify any file outside the two listed above.
- Full machine access, but touch NOTHING outside this repo, no network. Only the
  new files, `cargo`, and `python3`.
- You MUST paste the real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. The files: entry count each, validator green.
2. Verification — verbatim cargo output and the script above.
3. Per language: which multi-argument templates you reordered with indexed
   placeholders and why, plus any terminology choice a reviewer should check.
4. Stop-loss / open questions (empty if none).
