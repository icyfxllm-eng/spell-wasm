# es game-eligibility filter — RULES FOR ERIC'S REVIEW

**STATUS: REVIEW-GATED. Nothing here has been wired into the shipped app.**
The list this produces (`wordlists/es.txt`) is a *candidate* for review, not a
replacement for the live `assets/words/es/*.txt`. Eric signs off before any
generated list enters a build.

These are the rules applied to the `unmunch`-expanded rla-es v2.9 `es_ANY`
dictionary. Each word that is removed is attributed to **exactly one** rule (the
first it violates, in the order below), so the drop counts fully explain the
`1,231,013 in -> 909,728 out` delta. Counts are read straight from
`wordlists/es.manifest.json`.

## Pipeline order & drop counts (from the actual run)

| # | Rule | What it removes | Rationale | Dropped |
|---|------|-----------------|-----------|--------:|
| 0 | `nfc_renormalized` | (not a drop) words re-normalized to NFC | D3: ingest is NFC; log any change | 0 |
| 1 | `has_digit` | tokens containing 0-9 | not spellable words | 0 |
| 2 | `has_punct_or_space` | whitespace / hyphen / apostrophe / period / slash / punctuation | multi-token & abbreviated forms | 6 |
| 3 | `not_lowercase` | any token with an uppercase letter | drops proper nouns, acronyms, capitalized-only forms | 281 |
| 4 | `out_of_charset` | any char outside `a-z á é í ó ú ü ñ` | keeps to the Spanish keyboard charset (no ç etc.) | 0 |
| 5 | `too_short` | length < 3 (incl. single letters) | too trivial to spell | 119 |
| 6 | `too_long` | length > 15 | oversized agglutinated forms | 51,481 |
| 7 | `duplicate` | already-emitted after normalization | dedupe | 269,398 |
| | **Total dropped** | | | **321,285** |
| | **Kept -> `wordlists/es.txt`** | | | **909,728** |

Allowed alphabet: `abcdefghijklmnopqrstuvwxyzáéíóúüñ`  ·  length 3–15  ·  NFC.

## Why the counts look the way they do

- **`out_of_charset` = 0 / `has_digit` = 0**: rla-es is a clean orthographic
  dictionary, so almost all noise is capitalization, length, and duplication —
  not stray symbols. Good sign for source quality.
- **`duplicate` = 269,398**: `unmunch` emits the same surface form via multiple
  affix paths; dedupe (after NFC) collapses them.
- **`not_lowercase` = 281**: proper nouns / acronyms carried in the dictionary.

## ⚠️ THE THING ERIC ACTUALLY NEEDS TO DECIDE

The mechanical rules above are *correct* but **not sufficient for a spelling
game**. `unmunch` expands every inflection **and every enclitic-pronoun
combination**, so the kept list is dominated by valid-but-obscure agglutinated
verb forms. Real examples from the 50-word audit sample:

```
descalzárnoslas   repetiéndooslas   perforándotelos   especificármela
acordándomelas    coscachearíais    bailádnosla       diluídselos
```

These pass every rule (lowercase, in-charset, 3–15 chars) yet are terrible game
words. **Recommended follow-up passes for Eric to approve (NOT implemented here,
because they change what counts as "eligible" and that is his call):**

1. **Lemma / base-form reduction** — keep dictionary stems, drop generated
   inflections + clitic clusters (needs a lemmatizer or restricting affix
   expansion; the biggest quality lever).
2. **Frequency band gate** — intersect with a licensed frequency list
   (`hermitdave/FrequencyWords`, CC BY-SA — would need its own registry entry)
   to keep only common words and assign difficulty tiers.
3. **Profanity / kid-exclude** — run the existing `assets/words/profanity/es.txt`
   and `kid-exclude/es.txt` filters (deliberately NOT touched here — that
   architecture is out of scope per the work order).

Until (1)–(3) are decided, `wordlists/es.txt` is a **provenance-clean raw
candidate**, not a shippable pool.

## Determinism & non-goals

- Output is a pure, C-locale (codepoint) sorted, deduped, NFC function of the
  pinned source + these rules. Two runs are byte-identical (proven).
- This filter does **not** touch scoring, difficulty tiers, or the profanity
  architecture, and it does **not** write to `assets/words/` or `src/`.
