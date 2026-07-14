# Brief 027 — Traditional Chinese (`i18n/zh-Hant.conf`)

Translation brief. Produce `i18n/zh-Hant.conf`: a complete Traditional Chinese
translation, serving `zh_TW`, `zh_HK` and `zh_MO` (the locale chain resolves all
three to `zh-Hant`; the mechanism already works — only the file is missing).

**This is not a script conversion.** Read the terminology section before you
write a single line: a character-by-character 簡→繁 conversion produces
*mainland terminology written in Traditional characters*, which reads wrong in
Taipei and is the exact failure this brief exists to prevent.

## Goal

`i18n/zh-Hant.conf` exists, is complete (all 499 keys), passes
`every_shipped_translation_is_valid_and_complete` unchanged, and reads as
natural Taiwan-convention Traditional Chinese to a native speaker.

## Context pointers

- `i18n/zh-Hans.conf` — the Simplified translation, complete and reviewed. It is
  your source for *meaning* and for the key list. It is **not** your source for
  wording.
- `docs/i18n.md` — the rules. Read "Menu mnemonics are invariant", the `{}`
  template rule, and "Validating a translation".
- `crates/dun-cli/src/tests/i18n.rs` —
  `every_shipped_translation_is_valid_and_complete` discovers every
  `i18n/*.conf`, so your new file is validated the moment you create it. It will
  tell you exactly which keys are missing. Use it as your worklist.

## Terminology — the whole point of this brief

Taiwan software conventions differ from mainland ones in vocabulary, not just
characters. Use the right-hand column. This list is not exhaustive; it is the
set that appears most in this UI, and it should calibrate the rest.

| English | zh-Hans (mainland) | **zh-Hant (Taiwan) — use this** |
| --- | --- | --- |
| File (menu) | 文件 | **檔案** |
| View (menu) | 视图 | **檢視** |
| Help (menu) | 帮助 | **說明** |
| Edit (menu) | 编辑 | 編輯 |
| New | 新建 | **開新檔案** |
| Open | 打开 | **開啟** |
| Save | 保存 | **儲存** |
| Save As | 另存为 | **另存新檔** |
| Close | 关闭 | 關閉 |
| Quit | 退出 | **結束** |
| Reload | 重新加载 | **重新載入** |
| Undo | 撤销 | **復原** |
| Redo | 重做 | 重做 |
| Cut | 剪切 | **剪下** |
| Copy | 复制 | **複製** |
| Paste | 粘贴 | **貼上** |
| Clipboard | 剪贴板 | **剪貼簿** |
| Selection | 选区 | **選取範圍** |
| Find | 查找 | **尋找** |
| Replace | 替换 | **取代** |
| Search | 搜索 | **搜尋** |
| Match (n.) | 匹配 | **相符項目 / 符合** |
| Window | 窗口 | **視窗** |
| Pane | 窗格 | 窗格 |
| Split | 拆分 | **分割** |
| Menu | 菜单 | **選單** |
| Buffer | 缓冲区 | 緩衝區 |
| Directory | 目录 | 目錄 |
| Path | 路径 | 路徑 |
| Read-only | 只读 | **唯讀** |
| Not found | 未找到 | **找不到** |
| Permission denied | 权限被拒绝 | **存取被拒 / 權限不足** |
| Default | 默认 | **預設** |
| Config / Settings | 配置 / 设置 | **設定** |
| Run / Execute | 运行 | **執行** |
| Command | 命令 | 命令 |
| Program | 程序 | **程式** |
| Byte(s) | 字节 | **位元組** |
| Memory | 内存 | **記憶體** |
| Network | 网络 | **網路** |
| Timed out | 超时 | **逾時** |
| Load | 加载 | **載入** |
| Indent | 缩进 | **縮排** |
| Cursor | 光标 | **游標** |
| Hidden | 隐藏 | 隱藏 |
| Collapse / Expand | 折叠 / 展开 | **摺疊 / 展開** |
| Plugin | 插件 | **外掛** |
| Highlight | 加亮 | **標示 / highlight → 語法標示** |
| Truncated | 已截断 | **已截斷** |
| Line wrap | 自动换行 | **自動換行** |
| Toggle | 切换 | 切換 |

Punctuation: Traditional Chinese uses the same fullwidth marks as
`i18n/zh-Hans.conf` (`：`, `，`, `；`). Keep them.

## Rules you inherit (do not re-derive them)

1. **The vocabulary rule.** Anything the user has to type back stays English
   and is a `{}` argument, never translated: file paths, command ids
   (`edit.move_left`), theme names (`msedit|turbo|dark|dun`), config-diagnostics
   section tokens (`summary|paths|…`), key names (`Enter`, `Ctrl+X`), and the
   command-name list in the command-line help. Copy the shape from
   `i18n/zh-Hans.conf`: if a value there leaves something in English, yours does
   too.
2. **`{}` placeholders.** Every value must have **exactly** the same number of
   `{}` as its English default. You may reorder them freely to suit Chinese word
   order. A mismatch is rejected by the validator (and silently falls back to
   English at runtime, which is why it is rejected).
3. **The destructive-action guard.** `confirm.button.save`,
   `confirm.button.discard` and `confirm.button.cancel` must be present and
   **pairwise distinct**. These three words are drawn beside the literal `(s)`,
   `(d)`, `(c)` keys the dialog answers to — if two read alike, a user presses
   `(d)` and loses unsaved work. Suggested: `儲存` / `捨棄` / `取消`.
4. **No mnemonics in values.** The editor appends `(F)`, `(N)` etc. from the
   English labels. A value is base text only: `menu.file = 檔案`, never
   `menu.file = 檔案 (F)`.
5. **Loader limits.** Single line per key; value ≤ 256 bytes; nothing the
   display sanitizer would escape (no control bytes, no bidi marks, no
   zero-width characters).

## File header

Start the file with a comment block, in this spirit (English, like every other
comment in the repo):

```text
# Traditional Chinese (zh-Hant) UI translation for dun.
# Serves zh_TW, zh_HK and zh_MO via the locale chain (docs/i18n.md).
#
# Taiwan software conventions, not a character conversion of zh-Hans:
# 檔案/檢視/說明/儲存/復原/貼上/尋找/取代/視窗/預設 …
#
# Machine-translated; corrections from native speakers are welcome.
```

Keep the same section-comment structure as `i18n/zh-Hans.conf` (menus, help
window, dialog chrome, status messages) so the two files can be read side by
side.

## Scope

- Files you MAY modify: **`i18n/zh-Hant.conf` (new file) only.**
- Everything else is out of scope — no code, no tests, no docs, no
  `i18n/zh-Hans.conf`. The mechanism is already in place; if you believe a code
  change is needed, STOP and report it instead. (If the validator rejects your
  file, the file is wrong, not the validator.)

## Verification (MANDATORY — you run it; iterate to green)

```
cargo test -p dun-cli --bin dun i18n
cargo test --workspace --no-fail-fast
```

The first is your worklist: `every_shipped_translation_is_valid_and_complete`
names every missing key until the file is complete. Iterate until green, then
run the full workspace to be sure nothing else moved.

Then paste, verbatim, the output of:

```
python3 - <<'EOF'
import re
hans = dict(l.split('=',1) for l in open('i18n/zh-Hans.conf') if '=' in l and not l.strip().startswith('#'))
hant = dict(l.split('=',1) for l in open('i18n/zh-Hant.conf') if '=' in l and not l.strip().startswith('#'))
print("hans keys:", len(hans), "hant keys:", len(hant))
same = [k.strip() for k in hant if k in hans and hans[k].strip() == hant[k].strip()]
print("values identical to zh-Hans:", len(same))
for k in sorted(same)[:40]: print("  ", k, "=", hant[k].strip())
EOF
```

Some identical values are correct (numbers, `Dun`, terms that genuinely match).
A large count is a red flag that you converted characters instead of localising
terminology — review those keys and say in your report why the ones that
remain are right.

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave the new file in
  the working tree; Claude reviews it line by line and commits.
- Do NOT modify any file other than `i18n/zh-Hant.conf`.
- Full machine access, but touch NOTHING outside this repo, no network. Only the
  new file, `cargo`, and `python3`.
- You MUST paste the real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. The file: entry count, and confirmation the validator passes.
2. Verification — verbatim output of both cargo runs and the comparison script.
3. Terminology notes: the choices you made that are *not* in the table above and
   that a reviewer should check (this is the list Claude will focus on).
4. Stop-loss / open questions (empty if none).
