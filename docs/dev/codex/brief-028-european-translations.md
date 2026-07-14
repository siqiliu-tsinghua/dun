# Brief 028 — European translations: `fr`, `de`, `it`, `es`, `pt`, `ru`

Translation brief. Produce six complete translation files:

| file | language | serves |
| --- | --- | --- |
| `i18n/fr.conf` | French | `fr_FR`, `fr_CA`, `fr_BE`, `fr_CH`, … |
| `i18n/de.conf` | German | `de_DE`, `de_AT`, `de_CH`, … |
| `i18n/it.conf` | Italian | `it_IT`, `it_CH`, … |
| `i18n/es.conf` | Spanish | `es_ES`, `es_MX`, `es_AR`, … (all of Latin America) |
| `i18n/pt.conf` | Portuguese | `pt_BR`, `pt_PT` |
| `i18n/ru.conf` | Russian | `ru_RU`, `ru_UA`, … |

**Bare language tags on purpose.** The locale chain's last step is the bare
language, so `es.conf` serves every Spanish-speaking region with one file. Only
Chinese needs a script tag (`zh-Hans`/`zh-Hant`), because Simplified and
Traditional are genuinely different writing systems; the regional differences
within these six are not remotely that large.

For **Portuguese**, write Brazilian Portuguese (`arquivo`, not `ficheiro`):
CLDR's bare `pt` is Brazilian, Brazil has ~20× Portugal's speakers, and a
European reader understands it. A separate `pt-PT.conf` can be contributed
later and would win for `pt_PT` automatically.

## Goal

Six files, each complete (all 499 keys), each passing
`every_shipped_translation_is_valid_and_complete` unchanged, each reading as
natural software UI text to a native speaker.

## Context pointers

- `i18n/zh-Hans.conf` and `i18n/zh-Hant.conf` — complete, reviewed examples of
  the format, the section structure, and how the rules below look in practice.
  Use them as the shape; the English defaults in the code are your source for
  *meaning*.
- `docs/i18n.md` — read "Menu mnemonics are invariant", "Reordering arguments",
  and "Validating a translation".
- `crates/dun-cli/src/tests/i18n.rs` —
  `every_shipped_translation_is_valid_and_complete` discovers every
  `i18n/*.conf`, so each file is validated the moment you create it, and it
  prints exactly which keys are missing. **That list is your worklist.**

## Rules (inherited; do not re-derive)

1. **The vocabulary rule.** Anything the user types back stays English and is
   passed as a `{}` argument, never translated: file paths, command ids
   (`edit.move_left`), theme names (`msedit|turbo|dark|dun`),
   config-diagnostics section tokens (`summary|paths|…`), key names (`Enter`,
   `Ctrl+X`, `PageUp`), and the command-name list inside the command-line help.
   If a value in `zh-Hans.conf` leaves something in English, yours does too.
2. **Placeholders.** Each value must have the same **arity** as its English
   default. Use positional `{}` when the English order works. When your language
   needs a different order — this is why the mechanism exists — use indexed
   `{0}`, `{1}`, …: use every index in `0..arity` at least once, never mix `{}`
   and `{N}` in one value. A template that breaks this is rejected by the
   validator (and would fall back to English at runtime).
3. **The destructive-action guard.** `confirm.button.save`,
   `confirm.button.discard`, `confirm.button.cancel` must be present and
   **pairwise distinct**. They are drawn beside the literal `(s)`, `(d)`, `(c)`
   keys the dialog answers to: if two of them read alike, a user presses `(d)`
   and loses unsaved work. This is the one place a translation error can destroy
   data — get these three right first, in every language.
4. **No mnemonics in values.** The editor appends `(F)`, `(N)` from the English
   labels. `menu.file = Fichier`, never `menu.file = Fichier (F)`.
5. **Loader limits.** One line per key, value ≤ 256 bytes, nothing the display
   sanitizer would escape (no control bytes, no bidi marks, no zero-width
   characters). Cyrillic and accented Latin are fine; they are ordinary text.

## Language traps — read before writing each file

These are the mistakes a competent-but-careless translator actually makes here.

**French.** *Undo* and *Cancel* are both "Annuler" in careless French. They must
not collide: use **Annuler** for Cancel (the dialog button) and
**Annuler la saisie / Rétablir** carefully — the standard pair is
Undo = *Annuler*, Redo = *Rétablir*. Since `confirm.button.cancel` also wants
*Annuler*, that is fine (different keys, different contexts) — but
`confirm.button.discard` must **not** be *Annuler*: use **Abandonner** or
**Ignorer les modifications**. Save = *Enregistrer*. File = *Fichier*.

**German.** German UI words are long, and the menu bar is width-constrained on
narrow terminals. Prefer the shorter standard term: *Speichern*, *Verwerfen*,
*Abbrechen*, *Rückgängig* (not *Rückgängig machen*), *Wiederherstellen*,
*Ausschneiden*, *Kopieren*, *Einfügen*, *Suchen*, *Ersetzen*, *Fenster*,
*Ansicht*, *Hilfe*, *Datei*, *Beenden*. Do not invent compounds longer than
they need to be.

**Italian.** Same collision as French: *Annulla* means both Undo and Cancel.
Cancel = **Annulla**, Undo = **Annulla** is acceptable in menus, but
`confirm.button.discard` must be distinct: use **Ignora** or **Scarta**. Save =
*Salva*, File = *File* (Italian keeps the English word), Find = *Trova*,
Replace = *Sostituisci*.

**Spanish.** Save = *Guardar*, Discard = **Descartar**, Cancel = *Cancelar* —
three clearly distinct words, no trap. File = *Archivo*, Undo = *Deshacer*,
Redo = *Rehacer*, Find = *Buscar*, Replace = *Reemplazar*, Window = *Ventana*.

**Portuguese (Brazilian).** File = **Arquivo** (not *Ficheiro*). Save =
*Salvar*, Discard = **Descartar**, Cancel = *Cancelar*. Undo = *Desfazer*,
Redo = *Refazer*, Find = *Localizar*, Replace = *Substituir*.

**Russian.** *Отменить* means both Undo and Cancel — the classic Russian UI
collision. Use Cancel = **Отмена** (noun, the button) and Undo = **Отменить**
(verb), and make `confirm.button.discard` clearly distinct: **Не сохранять** or
**Отклонить**. Save = *Сохранить*, File = *Файл*, Find = *Найти*, Replace =
*Заменить*, Window = *Окно*. Russian word order is free — this is a language
where indexed `{0}`/`{1}` placeholders will earn their keep in the multi-argument
status templates.

## File header

Begin each file with a comment block in this spirit (English, like every other
comment in the repo):

```text
# French (fr) UI translation for dun.
# Serves fr_FR, fr_CA, fr_BE, … via the locale chain (docs/i18n.md).
#
# Machine-translated and NOT reviewed by a native speaker. Corrections are
# welcome; see docs/i18n.md for the rules a value must satisfy.
```

Say the "not reviewed by a native speaker" part plainly. It is true, and a user
deciding whether to trust the UI deserves to know.

Keep the same section-comment structure as `i18n/zh-Hans.conf` so the files can
be read side by side.

## Scope

- Files you MAY create: `i18n/fr.conf`, `i18n/de.conf`, `i18n/it.conf`,
  `i18n/es.conf`, `i18n/pt.conf`, `i18n/ru.conf`.
- **Nothing else.** No code, no tests, no docs, no existing translation. The
  mechanism is in place and validated. If the validator rejects a file, the file
  is wrong, not the validator — and if you genuinely believe otherwise, STOP and
  report rather than editing anything outside the list above.

## Order of work, and stop-loss

Do the languages **one at a time**, in this order: `es`, `pt`, `it`, `fr`, `de`,
`ru`. Finish a file completely, get the validator green for it, then start the
next. If you run long, stop **at a file boundary** with everything green and say
how far you got — six good files and an honest report beats six rushed ones.

## Verification (MANDATORY — you run it; iterate to green)

After each file, and once at the end:

```
cargo test -p dun-cli --bin dun i18n
cargo test --workspace --no-fail-fast
```

Then paste, verbatim, the output of this check for **each** language — it is the
one that catches the failure mode that matters:

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
for path in sorted(glob.glob('i18n/*.conf')):
    cat = load(path)
    save = cat.get('confirm.button.save')
    discard = cat.get('confirm.button.discard')
    cancel = cat.get('confirm.button.cancel')
    distinct = len({save, discard, cancel}) == 3 and all([save, discard, cancel])
    print(f"{path:22} keys={len(cat):4}  save={save!r} discard={discard!r} cancel={cancel!r}  distinct={distinct}")
EOF
```

## Hard rules

- Do NOT `git commit`, branch, push, or touch git config. Leave the new files in
  the working tree; Claude reviews them and commits.
- Do NOT modify any file outside the six listed above.
- Full machine access, but touch NOTHING outside this repo, no network. Only the
  new files, `cargo`, and `python3`.
- You MUST paste the real verbatim verification output. If a run did not reach
  green, say so explicitly — never fake it.

## Report format (your final message)

1. The files: entry count each, and confirmation the validator passes for all.
2. Verification — verbatim cargo output and the destructive-action table above.
3. Per language: the translation choices a reviewer should check — especially
   anywhere you used indexed `{0}`/`{1}` placeholders, and how you resolved the
   Undo/Cancel collisions in French, Italian and Russian.
4. Stop-loss / open questions (empty if none).
