# Word-bank provenance: the licence file no longer describes the banks

**Status: DECISION REQUESTED. Nothing changed by this document.**

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

This is a paperwork gap, not a rights problem: CC BY 4.0 content is shipping and
being credited nowhere, while the licence file says none is shipping at all.

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
