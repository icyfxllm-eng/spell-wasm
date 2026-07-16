# Chinese (pinyin) Script Path - sequence summary (DRAFT for native review)

**Scope: the writing system only - PINYIN ONLY.** This teaches a true beginner to
READ and SPELL Mandarin **in pinyin**: the Latin spelling system and its tone marks.
No grammar, no word meanings, no comprehension.

**NO hanzi. NO stroke order. NO drawing input** - that path is retired app-wide and
stays retired. This is exclusively the pinyin spelling system.

Strictly linear. Full JSON: `config/courses/zh.json`.

## >> BLOCKING FLAG: no audited word content for zh <<

`assets/words/zh/` **does not exist** - there is no audited Chinese word list. Every
other pilot spells real audited words after learning the elements; **zh cannot, yet.**

Therefore this draft delivers the pinyin **element inventory and structure in full**
(initials, finals, tones) but has **zero `spell` steps** - there are no real words to
reference, and we do NOT invent words or duplicate audio/metadata.

**Proposal for Eric:** commission an audited pinyin word list for zh (same pipeline and
audit terms as the other languages) - e.g. tiered by syllable complexity: simple
open syllables (ma, bo, ni) -> finals with codas (fan, hang) -> full tone-marked
syllables (mama, xiexie). Once that list exists and is audited, the spelling units drop
in under the existing element units with no structural change. Until then, zh ships (if
at all) as element-only, or not at all. Auditor: review the element order below on its
own merits.

## Element order and rationale

Pinyin syllable = **initial + final + tone**. The path follows exactly that shape.

1. **Initials** (21): b p m f  d t n l  g k h  j q x  zh ch sh r  z c s
   - Grouped by place of articulation (labial / alveolar / velar / palatal /
     retroflex / dental-sibilant), which is also how they're taught in China.
   - Note in cards: y and w are semivowel spellings of i/u finals, mentioned but not
     drilled as separate initials.
2. **Finals**: simple a o e i u u-umlaut, then the compound finals
   (ai ei ao ou, an en ang eng ong er, the i- u- and u-umlaut- groups).
   - Flag: `i` is a real "ee" after most initials but a buzz after z/c/s and
     zh/ch/sh/r; this is called out on the `i` card.
   - Flag: `u-umlaut` (u with two dots) is written plain `u` after j q x y.
3. **Tone marking**: tone 1 (high level), 2 (rising), 3 (dip-rise), 4 (falling),
   neutral (no mark), plus a **tone-placement rule** card (mark goes on a, else o,
   else e; in iu/ui on the last vowel).

## First words

**None in this draft** - see the blocking flag above. When an audited zh list exists,
the first words should be simple open CV syllables (e.g. ma bo ni tu) before adding
codas and before stacking all four tones.

## Notes for the auditor

- Confirm the initial grouping and the finals subset are a sane beginner order for
  pinyin, and note anything out of order.
- Confirm the tone-placement rule wording.

## Card strings

English source text, to be localized later. Budget: <=2 strings per element,
<=120 chars each, ~100 per language. **Chinese (pinyin) uses 64 strings** (within
budget). All element strings are in `zh.json` and shown to the auditor as a named
checklist section (see `README-REVIEW.md`).
