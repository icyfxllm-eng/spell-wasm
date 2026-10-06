# Vietnamese word bank — re-tiered 2026-10-06

These four files are sorted by length, not by frequency, and that is deliberate
repair work rather than the intended end state.

## What happened

The bank held 4,094 entries and 2,972 of them were not Vietnamese: English
(dreadnought, marketing, cholesterol), brand names (youtube, facebook, samsung),
place names (nigeria, jamaica, argentina), Italian (frazioni), and `retrieved`,
which is a Wikipedia citation artefact. 82% of the hard tier and 84% of expert.
A Vietnamese learner was mostly being asked to spell English.

The other languages' corpora are filtered by SCRIPT, which is why every
non-Latin bank is clean at 0% ASCII. Latin-script Vietnamese has no such signal,
so `build-wordlists.py` now filters by the structure of the language: a word must
be made of possible Vietnamese syllables, and at the two hard tiers it must also
carry a diacritic (short foreign words like `cat` and `bot` are valid Vietnamese
syllable SHAPES).

## Why length, not frequency

The frequency ranks that had defined these tiers were computed over the polluted
corpus and described nothing once the junk was gone — hard measured EASIER than
medium — so tiers are derived by length, the same measure
`words.rs::every_tier_ladder_climbs` uses.

## The space key — why hard and expert exist at all

A Vietnamese word is space-separated syllables: học sinh, thành phố, cà phê. The
bank format was one token with no spaces, so 86% of the vocabulary was
unrepresentable — 27,003 multi-syllable entries in the corpus against 4,541
single ones. And a single Vietnamese syllable is two to six characters, so hard
(7–8) and expert (9–15) could not be filled from any source at any size. That is
why the bank was padded with English in the first place: the format wanted long
tokens sharing bare Latin letters, which is what English supplies and Vietnamese
does not.

`build-wordlists.py` now allows a space for vi, the same shape as the fil hyphen
and fa ZWNJ exceptions beside it, and the vi keyboard carries a space key. The
bank is 5,680 words, and hard and expert are real Vietnamese words rather than
fragments.

Grading never required the space: `fold_strict` drops whitespace, so `họcsinh`
already matched `học sinh`. The key lets a player type what they are shown.

## State

    easy      840   single syllables
    medium    840   419 multi-syllable
    hard     1500   1499 multi-syllable
    expert   2500   all multi-syllable

Means climb 4.44 / 7.50 / 10.12 / 12.89. The Daily is offered again at a gate of
8, the same puzzle size Polish ships, with zero margin against its measured
ceiling — see `forge.rs::min_pool`.

The Spell It mic stays withdrawn for Vietnamese: it cannot produce a tone mark.
See `consts.rs::VOICE_SPELL_LANGS`.

## What is still owed

The bank is capped at 5,680 by `poolFloors` in `config/bank_floors.json`. Raising
those caps would let it grow and give the Daily real margin. The corpus has far
more to give: 31,544 filtered Vietnamese entries, of which only 5,680 are used.
