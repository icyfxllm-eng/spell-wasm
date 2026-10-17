# CC-IMPOSSIBLE v1.1: Sound-alike filter pointer

**Status:** REVIEW-GATED. Pointer amendment atop CC-IMPOSSIBLE v1. Changes only the homophone filter (I2, test A2) and one dependency. Everything else in v1 stands.
**Source:** Eric's text as pasted in chat on 2026-10-16, recorded in the repo on 2026-10-17. CC-IMPOSSIBLE v1.2 (`docs/CC-IMPOSSIBLE-v1.2-MIX.md`) builds on it.

---

## 0. Intent

Impossible promises every word is solvable from audio alone. A word with a sound-alike twin breaks that promise. So does a word with an **accent** twin, because some players hear the two identically. Impossible must therefore be stricter than the bonus, while still reading the same single table.

## 1. Changes

### A1. Filter source
v1's homophone filter (I2) reads **only** `soundalikes(lang, voice)` from CC-SENSE-CUE v1.1. Delete any homophone list, scan or key built inside Impossible.

### A2. Strictness
Exclude every member of every set with `kind: true` **or** `kind: accent`. (The bonus and SENSE-CUE use `true` only; this difference is intentional.)

### A3. Status handling

| Table status | Impossible in that language |
|---|---|
| `missing` | locked (fail closed, unchanged from v1) |
| `none-by-construction` | available; the filter removes nothing |
| `verified` | available; filter per A2 |

### A4. Pool sizing
Use the CC-SENSE-CUE v1.1 shared census report for "words removed by the sound-alike filter". Impossible's lock at fewer than 150 eligible words (v1) is computed **after** that removal. No separate scan.

### A5. English ordering
Phase A for English waits for: CC-EN-EXPERT-GROWTH Phase A, then its staged rows live, then the English table regenerated (CC-SENSE-CUE v1.1 H5).

**Status note, 2026-10-17:** EN-EXPERT-GROWTH Phase A is merged; the re-tiered rows exist behind `en.retier` (off); English has a flag-aware table (`soundalikes-retier.json`) covering them.

## 2. Invariants (added)

- **I2a** Impossible contains no homophone data or scan code (symbol scan).
- **I2b** No Impossible word shares a set of either `kind` with any other bank word.

## 3. Acceptance tests (replace v1 A2)

- **A2.1** en: neither `sink` nor `sync` appears in the Impossible pool.
- **A2.2** en: neither `cot` nor `caught` appears in the Impossible pool.
- **A2.3** A language with a `missing` table: Impossible is absent.
- **A2.4** A `none-by-construction` language: the Impossible pool equals v1's pool before the homophone step.
- **A2.5** The removed-word count matches the shared census report exactly.
- **A2.6** A deliberate homophone list added inside Impossible fails CI (I2a).

## 4. Decisions

Recommended defaults, unsigned:
- **IM-1** Accent pairs excluded (A2).
- **IM-2** `missing` keeps Impossible locked for that language.

## 5. Non-goals

No change to the hardest-slice rule, replay cap, surfaces, Climb multiplier, or any other v1 decision. (v1.2 later replaces the hardest-slice rule for Mix with 100% of Expert.)
