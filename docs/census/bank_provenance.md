# Word-bank provenance: the licence file no longer describes the banks

**Status: PARTLY REMEDIED 2026-10-07 on Eric's "do the provenance remedy".**
The paperwork corrections are applied to `assets/words/LICENSES.md`. One item
from the proposed remedy below was DROPPED as misdiagnosed and replaced by a
larger finding; see "What the remedy actually found", at the end.

Found 2026-10-06 while cleaning the Vietnamese bank. The Vietnamese list turned
out to be 73% not Vietnamese — English, brand names, place names, and the word
`retrieved`, which is a Wikipedia citation artefact. That prompted a check of
every other bank, and the question stopped being about Vietnamese.

## The claim

`assets/words/LICENSES.md`, "Current sources (v1)":

> Word banks (11 locales × 4 tiers) — **Original curation** for this app —
> **Owned — ships freely** — Hand-authored common vocabulary; tiers reflect human
> spelling-difficulty judgement, not raw frequency.
>
> Because the v1 pools are original curation, there is **no third-party lexicon
> license to satisfy for the shipped binary**.

Its "Intended expansion sources (not yet ingested)" section lists Hunspell,
hermitdave/FrequencyWords, wordfreq and LDNOOBW. It does not mention Leipzig.

## What is actually in the banks

`scripts/build-bigbank.py`, in its own words:

> Grow `assets/words/<lang>/<tier>.txt` from the **Leipzig Corpora Collection**.
> Leipzig (wortschatz-leipzig.de) is monolingual, frequency-ranked, **CC BY 4.0**
> … augments the hand-authored core … with corpus words for volume.
> Verified Leipzig files (**Wikipedia** = diverse/neutral vocab).

Fourteen of fifteen banks name a pinned Leipzig Wikipedia corpus — every language
but Chinese:

    en  eng_wikipedia_2016_100K      ko  kor_wikipedia_2021_100K
    es  spa_wikipedia_2021_100K      fil tgl_wikipedia_2021_100K
    fr  fra_wikipedia_2021_100K      ja  jpn_wikipedia_2021_100K
    de  deu_wikipedia_2021_100K      ru  rus_wikipedia_2021_100K
    pt  por_wikipedia_2021_100K      ar  ara_wikipedia_2021_100K
    pl  pol_wikipedia_2021_100K      sw  swa_wikipedia_2021_100K
    vi  vie_wikipedia_2021_100K      hi  hin_wikipedia_2021_100K

`src/consts.rs` says it too, in passing: "Real Leipzig CC BY bank."

## The evidence in the shipped data

Seven banks still carry vocabulary that only occurs in reference text. The
localised MediaWiki namespace words are the clearest: they are a dump
fingerprint, not a coincidence.

    en   citation, edition, retrieved
    es   categoría, isbn
    fr   copyright, référence
    de   isbn, kategorie, wikipedia
    pt   categoria, copyright, edition, wikipedia
    sw   wikipedia
    vi   doi

The second half of the claim does not hold either. "Tiers reflect human
spelling-difficulty judgement" — `build-bigbank.py` tiers by LENGTH
(`TIER_LEN`), and the Vietnamese ladder had inverted so far that
`every_tier_ladder_climbs` failed on it.

## So what

Leipzig is **CC BY 4.0**. That is a permissive licence and the corpora are
explicitly fine to use — it asks for attribution, and nothing else here is a
problem. Two things follow:

1. `LICENSES.md` states the opposite of the truth: that no third-party lexicon
   licence applies to the shipped binary. It is the document someone would read
   to answer an App Store or legal question.
2. The app already has the machinery for this. `src/credits.rs` renders a credits
   screen for every shipped human-audio clip "whose license requires attribution
   (CC BY, CC BY-SA)". The word banks are not in it.

This is a paperwork gap, not a rights problem: the licence file says no
third-party lexicon ships, while CC BY 4.0 content does.

**Correction 2026-10-07: "being credited nowhere" was wrong.** `NOTICES.md` at
the repository root has recorded the Leipzig Corpora Collection at CC BY 4.0 all
along, naming the scripts that use it and the attribution line. This document
missed that file when it was written, and the error made the problem look like a
missing attribution when it is really a CONTRADICTION between two files in one
repository -- `LICENSES.md` denying what `NOTICES.md` records. The real
attribution gap is a different one, below.

## Proposed remedy, NOT applied here

* Record Leipzig in `LICENSES.md` as an ACTUAL source with its licence, the
  pinned corpus files above, and the retrieval year — moving it out of "intended,
  not yet ingested".
* Strike the "no third-party lexicon license to satisfy" sentence.
* Add the Leipzig attribution to the existing credits screen, next to the audio
  clips that are already credited for the same reason.
* Correct "tiers reflect human spelling-difficulty judgement": they are tiered by
  length, with a hand-authored core.

The first three are corrections to shipped claims rather than feature work, but
the third changes what a player sees, so none of it is applied here. Eric's call.

## What this does not say

It does not say the banks are unlicensed, that anything must be removed, or that
the hand-authored core is not original. The core is real; Leipzig was layered on
top of it and the paperwork never caught up.


## What the remedy actually found (2026-10-07)

Applying this document's remedy turned up two things it had wrong and one thing
it missed, so the record is here rather than in the commit alone.

**Applied to `assets/words/LICENSES.md`:** Leipzig recorded as an actual source
with its licence, the 14 pinned corpora and their retrieval spans; the "no
third-party lexicon license to satisfy" claim struck; the length-tiering
correction made; "11 locales" corrected to 16 bank directories; and two stale
items fixed in passing (an "intended expansion" preamble that still said the
pools had not grown, and a QA line naming nl/sv/nb/tr, cut from the registry).

**DROPPED: "add the Leipzig attribution to the existing credits screen."** The
diagnosis was wrong on two counts. `src/credits.rs` renders only the generated
`assets/human-audio/credits.json`, whose `clips` array is currently EMPTY, and
its player-facing copy is written about voice recordings: the settings subtitle
is "The people whose recordings you hear", the intro names Lingua Libre, and
the empty state reads "Every real-voice recording in this version is in the
public domain". Adding corpora there would mean new player-facing strings in 15
locales, to describe a list that exists for a different purpose.

**MISSED, and it is the bigger one: `NOTICES.md` does not ship.** It is not in
`dist/`, not in the iOS bundle, and not served by the web app; `index.html`
mentions it only in a CSS comment. So every attribution it carries -- Leipzig
CC BY 4.0, CC-CEDICT CC BY-SA 4.0, Wiktionary CC BY-SA 4.0, KANJIDIC2 CC BY-SA,
JMdict-derived JLPT meanings, OpenDyslexic CC BY 3.0 -- reaches no user. The
fonts are the one part already covered: `fonts/OFL.txt` IS copied into
`dist/fonts/`, so the SIL OFL obligation to ship the licence alongside the font
files is met.

That reframes the player-facing item. It is not "put Leipzig in the voice
credits"; it is "ship the notices file and reach it from the UI", which covers
every data source at once instead of one of them. That is a product change with
an i18n cost, so it is NOT applied here and remains Eric's call.

Still open, unchanged by this pass: whether the Wikipedia CC BY-SA underlay
matters for single-word extracts, and a human read of the Leipzig terms page --
which could not be fetched on 2026-10-07 because the site is behind
proof-of-work anti-bot protection, and the corpus archives carry no licence file.
