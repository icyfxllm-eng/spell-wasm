# Vietnamese word-bank options — review artifact for Eric

**Status: DECISION REQUESTED. Nothing changed by this document.**

Prepared 2026-10-06, after the vi bank was found to be 73% not Vietnamese and
cleaned down to 1,122 words, which cost Vietnamese its Daily (forge ceiling fell
from 17 to zero) and left tiers that had to be re-derived by length.

This was commissioned as "which corpus should refill the Vietnamese bank". The
measurements say that is the wrong question, so it is answered first and briefly.

## No source will fix this

The Leipzig Vietnamese corpus is already here — `.corpus-cache/
vie_wikipedia_2021_100K.tar.gz`, 21MB, the same CC BY 4.0 collection behind every
other bank. Run through the Vietnamese syllable filter it yields **31,544**
genuine Vietnamese entries, frequency-ordered, headed by và, của, được, là, năm,
một, các, đã, có, trong. The source is not the problem and no licence gate is in
the way.

**86% of it cannot be used.** 27,003 of those 31,544 entries are multi-syllable:
đầu tiên, có thể, khoa học, sử dụng, miêu tả. Vietnamese writes a word as
space-separated syllables, and the bank format is one token per entry with no
spaces — enforced by the builder's "non-alphabetic" filter, and by every
keyboard in `assets/keyboards/`, none of which has a space key, in any language.
A multi-word answer is untypeable app-wide by construction.

What survives is **4,541 single syllables**, and they do not tier:

| tier | length band | words from the corpus |
|---|---|---|
| easy | 2–4 | 3,990 |
| medium | 5–6 | 547 |
| hard | 7–8 | **3** |
| expert | 9–15 | **1** |

A Vietnamese syllable is two to six characters. It is essentially never seven or
more, so the two hard tiers cannot be filled from any source, at any size,
while tiers are defined by length.

This is also the explanation for the mess that started all of it. The bank was
padded with English because English words are long and share bare Latin letters
— exactly what the length tiers and the honeycomb need, and exactly what
Vietnamese does not provide. The contamination was not careless ingestion so
much as the format demanding something the language does not supply.

## The three real options

**A. Accept Vietnamese as a short-tier language.** Ship easy and medium from the
corpus (~4,500 words, a four-fold increase on today) and declare vi structurally
2-tier, as `bank_floors.json` already does for Swahili's missing T4. The Daily
stays off. Honest, cheap, and permanently caps Vietnamese at syllables rather
than words.

**B. Let the bank hold multi-syllable entries.** Allow a space for vi in the
builder's alphabetic filter — the same shape as the existing `fil` hyphen and
`fa` ZWNJ exceptions — and add a space key to the vi keyboard. That unlocks
27,003 real Vietnamese words, fills all four tiers, and probably restores the
Daily. It touches the answer field, the keyboard, the charset gate and grading,
and it is the only option that lets a Vietnamese player spell a Vietnamese
*word* rather than a syllable.

**C. Tier Vietnamese by something other than length.** Syllable length does not
vary enough to be a difficulty axis, but tone and vowel-quality marks do: `ba` is
not `bả` is not `bưởng`. This needs `every_tier_ladder_climbs` to accept a
per-language difficulty measure, which today is mean NFD length for everyone.
Smaller than B, and it fixes the ladder without making Vietnamese words typeable.

B and C are not exclusive; B alone would also fix the ladder, because
multi-syllable words are long.

## Recommendation

B, and C if B is refused. A is the only one that needs no engineering, and it
locks Vietnamese to spelling fragments of words indefinitely — which is a
reasonable thing to decide deliberately and a poor thing to arrive at by default,
which is roughly how it happened the first time.

## What is NOT in the way

* **Licence.** Leipzig is CC BY 4.0, already relied on for fourteen banks. The
  attribution gap is real but general, not Vietnamese — see
  `docs/census/bank_provenance.md`.
* **Data.** The corpus is cached locally; nothing needs downloading.
* **Filtering.** `build-wordlists.py` already rejects non-Vietnamese by syllable
  structure, so a refill will not reintroduce `dreadnought`.
