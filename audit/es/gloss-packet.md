# Spanish (es) gloss review packet

**What you are checking.** Each row pairs a Spanish word from the app's word bank with the
English meaning it stands for. Spell Translate shows a word in another language only through
that meaning, so a wrong or ambiguous meaning would show a child the wrong word. Nothing here
is live yet: Spanish stays hidden in Translate until you sign the file.

**Rows:** 296  ·  **by tier:** easy 268, medium 3, hard 16, expert 9  ·  **shared with Japanese:** 266

## How to review

Open `gloss-rows.csv` and fill the last four columns for every row:

- **keep** — the word and meaning match, and the meaning is the sense a child would expect.
- **fix** — right idea, wrong detail. Put the better word in `fix_word` and/or the better English
  meaning in `fix_meaning`. A fixed word must already be in the Spanish bank, and a fixed meaning
  must be a common English word; if neither is possible, choose cut.
- **cut** — the row should not exist (no good match, or the word is wrong for children).
- **note** — anything else worth knowing.

The flagged sections below are where a second look matters most. They come from mechanical
checks, so an unflagged row can still be wrong.

## Sense check  (40)

The English meaning has more than one common sense (a ring on a finger, or a phone ringing). Confirm the word matches the sense a child would read first. Hint list only — not exhaustive.

- **abrigo** → coat  _(row 1, easy)_
- **abrir** → open  _(row 2, easy)_
- **anillo** → ring  _(row 11, easy)_
- **bien** → well  _(row 26, easy)_
- **cabeza** → head  _(row 33, easy)_
- **caja** → box  _(row 35, easy)_
- **carta** → letter  _(row 45, easy)_
- **cima** → top  _(row 52, easy)_
- **correr** → run  _(row 66, easy)_
- **cuadrado** → square  _(row 68, hard)_
- **derecho** → right  _(row 76, easy)_
- **difícil** → hard  _(row 78, easy)_
- **espalda** → back  _(row 92, easy)_
- **estrella** → star  _(row 96, hard)_
- **frío** → cold  _(row 104, easy)_
- **fuego** → fire  _(row 105, easy)_
- **historia** → story  _(row 125, hard)_
- **izquierda** → left  _(row 138, expert)_
- **jugar** → play  _(row 140, easy)_
- **largo** → long  _(row 142, easy)_
- **libro** → book  _(row 147, easy)_
- **llave** → key  _(row 148, easy)_
- **luz** → light  _(row 153, easy)_
- **línea** → line  _(row 155, easy)_
- **mano** → hand  _(row 160, easy)_
- **mesa** → table  _(row 167, easy)_
- **ola** → wave  _(row 200, easy)_
- **oso** → bear  _(row 204, easy)_
- **pato** → duck  _(row 211, easy)_
- **pie** → foot  _(row 222, easy)_
- **planta** → plant  _(row 226, easy)_
- **primavera** → spring  _(row 233, expert)_
- **punto** → point  _(row 237, easy)_
- **roca** → rock  _(row 246, easy)_
- **romper** → break  _(row 248, easy)_
- **sonido** → sound  _(row 265, easy)_
- **tiempo** → time  _(row 273, easy)_
- **tren** → train  _(row 280, easy)_
- **vaso** → glass  _(row 284, easy)_
- **viento** → wind  _(row 292, easy)_

## Same spelling as the English meaning  (5)

Confirm each is the natural Spanish word, not a loan kept only because it matched.

- **animal** → animal  _(row 12, easy)_
- **color** → color  _(row 59, easy)_
- **hospital** → hospital  _(row 129, hard)_
- **metal** → metal  _(row 168, easy)_
- **no** → no  _(row 188, easy)_

## Not kid-safe  (3)

On the kid exclusion list; Spell Jr never shows these. Confirm, or say if they are fine.

- **muerte** → death  _(row 175, easy)_
- **sangre** → blood  _(row 256, easy)_
- **vino** → wine  _(row 293, easy)_

## English meaning not kid-safe  (2)

The English meaning itself is on the English kid exclusion list.

- **sangre** → blood  _(row 256, easy)_
- **vino** → wine  _(row 293, easy)_

## Mechanical sanity checks (expect none)

## English meaning not in the English bank  (0)

Translate resolves English across all four tiers, so every meaning in the English bank can answer; gloss-check forbids a meaning outside it.

- none

**Not in the Spanish bank:** 0 (gloss-check forbids this, so expect 0).

## Signing off

When the review comes back: apply the fixes to `config/gloss/es.json`, run
`node scripts/gloss-check.mjs`, then set `"audited": true` and `"auditor"` to the reviewer's
full name. The build refuses an audited file with no named auditor.
