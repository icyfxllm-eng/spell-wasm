# CC-ORTHO-SWEEP-ALL: auditor addendum rows (es, fr, pt)

For the native-speaker auditor. Sheets: `reports/ortho-gigb-es.csv`, `-fr.csv`, `-pt.csv` (from `tools/ortho/make_gigb_sheet.py`).
Two new columns on every row: **Same sound in your accent?** (yes / no / unsure) and **Correct spelling where you live?**
(write the spelling you would use; leave blank if the stored spelling is right for you). Nothing on the sheet is live until it
returns; the `ortho.profile.<lang>` flags stay off until then.

## es (Spanish)
- **The voice is from Spain** (es-ES-Neural2-G). Judge sameness against that voice: c/z against s is a *different* sound here
  (distinción), so casa/caza is not a same-sound pair. If your accent is Latin American, answer for the voice, and write your
  own accent in the notes.
- b/v, silent h, ll/y (yeísmo) and pairs that differ only by a written tilde (tu/tú) are treated as the same sound.
- n/ñ, r/rr (pero/perro) are near-sounds: a miss, never accepted.
- Leaving the tilde off a vowel (está/esta) is a "close", not correct; ñ is a letter, so ano for año is a miss.

## fr (French)
- Voice fr-FR-Neural2-F. Homophones such as ver/vert/verre/vers, sang/sans/cent are listed; confirm each.
- oe for œ is accepted on Easy and Spell Jr, "close" above. Left-out accents are "close". 1990 spelling-reform forms
  (maitre, évènement) are accepted spellings of the same word.

## pt (Portuguese)
- Voice pt-BR-Neural2-B; stored forms are Brazilian, AO90. European forms (secção, facto) are accepted variants.
- ç is a letter (caça is not caca). Left-out tilde/accents on other vowels are "close".
- Treated as the same sound: ç/ss/c/s sibilants (caça/cassa), ch/x (cheque/xeque), silent h, g/j. Treated as accent-only, never accepted: unstressed-vowel and final-l/u differences. If your region differs on any of these, say so in the second column.
