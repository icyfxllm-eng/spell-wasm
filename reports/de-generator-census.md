# German sound-alike generator: section 0 census (read-only), 2026-11-04

Question: what would a `de` generator (v1.1 F3) find, and what does it need?

## Inputs today

- **No German lexicon.** English has cmudict plus web2, so a word outside the bank still gets a key. German has neither. `tools/wordpipe/sources` holds no German pronunciation data, `espeak-ng` is not installed, and `phonemizer` is not available.
- Bank: 6,259 rows (Easy 272, Medium 1,000, Hard 2,000, Expert 2,987). Voice `de-DE-Neural2-A`; the table is `missing`, with only the five seed sets.
- `VOICE_IN_KEY` includes `de`, so the voice is part of the cache key.

## Prototype: spelling-only key

Rules: ß→s; ph→f; v→f; w→v; ck/tz/dt→doubled consonants; ie/ieh/ih→i; vowel+h before a consonant dropped; aa/ee/oo→single; äu/eu→oi; ä→e; final b/d/g→p/t/k. Doubled consonants are kept, because they mark a short vowel.

Result over the whole bank: **21 candidate sets, 9 Expert words (0.30% of 2,987)**. Far below MG5's 10% line.

It finds 4 of the 5 seed sets (wahren/waren, wider/wieder, Stadt/statt, booten/boten) but misses endstand/entstand. The seed pair Ligen/liegen is a seed-file candidate, not in the five.

## Why a spelling key is not enough

Vowel length cannot be read from spelling, and it is what separates many German near-pairs. At least these prototype sets are **false** (different vowel length or quality):

- Welt/wählt, Welten/wählten, zelt/zählt, Feld/fehlt: short /ɛ/ against long /ɛː/ or /eː/.
- Dänen/denen, Sätzen/setzen, beenden/bänden: the same ä/e issue, and ä-e is exactly the unsigned M3 merger.
- Armeen/armen, Gerd/geehrt: different syllable counts or vowel quality.

The rest (fielen/vielen, Mohr/Moor, reist/reißt, reisen/reißen, weisen/weißen, leeren/lehren, Ligen/liegen) look genuine. So the prototype's precision is about two thirds, and **a false set is the harmful kind** (it accepts a wrong answer).

## What a real generator needs

1. A German pronunciation source with IPA, such as Wiktionary extracts or an espeak-ng install. This is the real blocker.
2. A key over that IPA, with final devoicing and the ich/ach distinctions folded the way the de-DE-Neural2-A voice speaks.
3. The usual F5 loopback and F6 auditor pass before the status can be `verified`.

## Expected payoff

About 15 to 20 true sets in the bank, around 0.3% of Expert. That is small. It matters for correctness more than volume: it keeps German grading from rejecting heard-identical spellings, and it is the only route to unlocking German Impossible.

## Open questions for Eric

- G1: Pick the pronunciation source (Wiktionary IPA vs installing espeak-ng). Either needs a licence and size check I have not done.
- G2: Is a small, hand-audited seed list (the five plus Mohr/Moor, reist/reißt, weisen/weißen, leeren/lehren, fielen/vielen, Ligen/liegen) enough for now? It costs nothing and misses unbanked homophones.

HALT. Nothing was built.
