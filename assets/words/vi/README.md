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

With the junk gone, the frequency ranks that had defined these tiers described
nothing — hard measured EASIER than medium (4.87 against 6.52), and
`words.rs::every_tier_ladder_climbs` rightly refuses a ladder that does not
climb. Re-deriving by length uses the same measure that test uses. Means now
climb 3.50 / 4.72 / 5.77 / 7.23.

Unmarked words stay in easy and medium, where they are genuinely the common ones
(anh, ba, cho, con, nhanh); hard and expert are entirely diacritic-bearing.

## What this cost

The Daily (letter forge) is no longer offered for Vietnamese. Its measured
ceiling fell from 17 to ZERO: no gate, not even four, yields a puzzle every day
of a year. The honeycomb needs many words sharing seven characters, and the
English contamination was supplying exactly that. Vietnamese words are short
syllables carrying tone and vowel-quality marks, so few fit inside any seven
characters. See `forge.rs::min_pool`.

The Spell It mic is withdrawn for Vietnamese as well, for a separate reason —
the lexicon cannot produce a tone mark. See `consts.rs::VOICE_SPELL_LANGS`.

## What would fix it

A real Vietnamese frequency corpus, replacing all four files. That restores
meaningful ranks, and it is the only thing that can bring the Daily back. The
dropped words are still recoverable from this repository's history, and
`build-wordlists.py` reports every drop with its reason when it runs.
