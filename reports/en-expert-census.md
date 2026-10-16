# CC-EN-EXPERT-GROWTH §0 census

Run 2026-10-15, local `main` (HEAD 08da2864). Read-only: no code or data changed. The scripts were throwaway and live outside the repo.

## Root-cause verdict

| Cause | Verdict | Evidence |
|---|---|---|
| R1 bank sourced from a frequency-capped list | **Confirmed** | `scripts/build-bigbank.py` draws from Leipzig `eng_wikipedia_2016_100K` in frequency order. About 94% of current Expert rows sit at Zipf ≥ 4.0 (C3). |
| R2 tiers assigned by letter count | **Confirmed (origin), partly superseded** | The build gate was `TIER_LEN`: easy 2–4, medium 5–6, hard 7–8, expert 9–15 letters. Two later retiers (`aa76d14c`, `retier.py`) swapped some rows by a composite score. Tier medians are still 4 / 6 / 7 / 10 letters (C4). |
| R3 English was frozen | **Confirmed** | `config/bank_floors.json`: en `frozen: true`, "frozen-reference per D5 ... NEVER red". Last Expert growth was `64219e3f` on 2026-07-27, "banks to 800/tier everywhere" (+600 rows). Earlier: +50 (`893ed7a6`), +150 (`97c214fc`). |
| R4 filters removed disproportionately many rare words | **Not supported** | PURITY quarantined only 7 English rows in total, 1 of them Expert (C1). Proper-noun and profanity gates acted at corpus level (C6), but that is not why Expert is small. |
| **Additional: R5 a per-tier cap** | **Confirmed** | Each tier was grown to ~800 and stopped. Other languages were later raised to 2,500–3,000 Expert rows in the August generation wave. English was not (it is frozen). |

Summary: English Expert is small because it was deliberately capped at ~800 per tier. It holds *common* long words, not hard-to-spell ones, because it was built from the top of a frequency list and tiered by length.

## C1: rows per tier after PURITY

| Tier | Bank rows | Quarantined | In ledger as passing |
|---|---|---|---|
| easy | 763 | 4 | 763 |
| medium | 798 | 2 | 798 |
| hard | 800 | 0 | 800 |
| expert | 797 | 1 | 797 |
| total | 3,158 | 7 | 3,158 |

## C2: how tiers are assigned today

- Origin: `scripts/build-bigbank.py` `TIER_LEN` (letters) over Leipzig frequency order.
- Retier A: `tools/difficulty-score/retier.py` (`LANGS` includes en). Composite = 3.0·extractor features + 2.5·bigram rarity vs easy+medium + 1.2·vowel-run + 0.5·length. It swaps only within hard∪expert and keeps Expert's count fixed.
- Retier B: `aa76d14c` "judgment-based Expert re-tier" (25 rows).
- Net: a mix, with length still the strongest signal. Tier is stored as the file a row lives in (`assets/words/en/<tier>.txt`), with no score field. The served pool is built at `src/words.rs:68` `simple_tier` and `:6288` `tier_for`. A single "tier assignment lives in one place" rule would have to be created.

## C3: frequency (Zipf) per tier

**Caveat that matters:** no frequency source with usable resolution in the 2.5–4.0 band is available locally (see C7). These Zipf values come from the Leipzig sample file, which is only 2.49M tokens, so Zipf below ~3.5 means "seen 1 to ~10 times" and is noisy. Treat the low bins as indicative only.

| Zipf bin | easy | medium | hard | expert |
|---|---|---|---|---|
| 2.0–2.5 | 0 | 0 | 0 | 0 |
| 2.5–3.0 | 0 | 1 | 3 | 10 |
| 3.0–3.5 | 2 | 7 | 6 | 8 |
| 3.5–4.0 | 11 | 11 | 12 | 14 |
| 4.0–4.5 | 232 | 16 | 53 | 240 |
| 4.5–5.0 | 264 | 466 | 522 | 433 |
| ≥ 5.0 | 254 | 297 | 201 | 79 |
| absent from corpus | 0 | 0 | 3 | 13 |
| median | 4.75 | 4.89 | 4.78 | 4.58 |

Bank-wide: **14 of 3,158 rows are below Zipf 3.0, none below 2.5** (16 rows absent from the corpus). The whole bank, Expert included, is essentially a ≥ 4.0 list.

## C4: length per tier (letters)

| Tier | min | median | max | 2–4 | 5–6 | 7–8 | 9–10 | 11–12 | 13–15 |
|---|---|---|---|---|---|---|---|---|---|
| easy | 2 | 4 | 6 | 755 | 8 | 0 | 0 | 0 | 0 |
| medium | 4 | 6 | 9 | 1 | 747 | 41 | 9 | 0 | 0 |
| hard | 4 | 7 | 14 | 1 | 5 | 764 | 13 | 14 | 3 |
| expert | 5 | 10 | 16 | 0 | 5 | 12 | 525 | 197 | 57 (+1 at 16) |

Tiers are nearly perfect length bands. This is the R2 signature.

## C5: source and growth history

- Source: Leipzig `eng_wikipedia_2016_100K` (CC BY 4.0), cached at `.corpus-cache/`, plus a curated seed. Built by `scripts/build-bigbank.py`, with `--target N` per tier.
- `git log` on `assets/words/en/expert.txt`: `893ed7a6` (+50), `97c214fc` (+150), `aa76d14c` (25 swapped), `64219e3f` (+600, 2026-07-27), `6ea2feae` (variety fix), `3e54e8d6` (PURITY, −1). **No rows added after the freeze.** There was no "REBUILD unfreeze" for English. BD-G4 (`e3dd6f46`, 2026-08-08) rebuilt other banks from the 300-concept core; English expert was not touched by it.
- CC-BANK-COMPLETE and CC-BANK-REBUILD themselves are not in `docs/`, so D5 and the "two-source rule" are known to me only through code comments and the `bank_floors.json` `$doc` string.

## C6: removed by each filter, split by Zipf band

The original build did not log per-filter removals, so this is a recomputation over the Leipzig types (ASCII alphabetic, length ≥ 2). "Proper-like" = lowercase form is under 90% of occurrences.

| Zipf band | Types | Proper-like | In SCOWL dictionary | Admissible (dictionary ∧ not proper-like ∧ has CMU key) | Already in bank |
|---|---|---|---|---|---|
| 2.5–3.0 | 48,455 | 28,124 | 7,521 | 4,430 | 14 |
| 3.0–3.5 | 15,197 | 8,245 | 5,386 | 2,952 | 23 |
| 3.5–4.0 | 8,790 | 4,331 | 4,177 | 2,296 | 48 |
| 4.0–4.5 | 4,254 | 1,652 | 2,361 | 1,404 | 541 |
| 4.5–5.0 | 1,962 | 672 | 1,289 | 807 | 1,685 |
| ≥ 5.0 | 867 | 287 | 669 | 438 | 831 |

Actual in-repo removals: PURITY quarantine 7 English rows (`manifest.csv`), none of them an obscurity filter. The bank's thinness is from where it stopped sampling, not from what the filters removed.

## C7: sources, licence and coverage

| Source | In repo / cached | Licence | Coverage / limits |
|---|---|---|---|
| CMUdict | `tools/wordpipe/sources/cmudict.dict` + `cmudict.LICENSE` | BSD-style, licence file present | 126,052 words. Bank coverage **3,153 / 3,158** (null: circumlocution, amanuensis, chiaroscuro, fuchsia, onomatopoeia) |
| SCOWL `en_US.dic` | `.corpus-cache/dicts/en/` (pinned in `config/bank-purity-sources.json`) | Permissive (per the sources file) | 38,406 lowercase ASCII headwords. **No POS or sense tags**, so F3's archaic / offensive / dialectal sense exclusions cannot be applied from it |
| Wiktionary (kaikki English) | **not cached** (only Tagalog); `defs/en.jsonl` holds 3,669 bank-word definitions | CC BY-SA 4.0 | Attribution approach still unapproved (`data/en/SOURCES.md` F5 gate). A full English dump would have to be fetched |
| WordNet | **not present** | Princeton (permissive) | would need fetching |
| web2 (Webster 1934) | vendored | Public domain | 235k words, heavy on archaic/obscure, no tags. Does not meet "non-archaic" |
| Frequency, OpenSubtitles `freq_en.txt` | vendored | hermitdave/FrequencyWords, **CC BY-SA 4.0** (NOTICES.md) | **Top 50,000 words only** (lowest count 159 of 725M tokens, so Zipf ≥ 5.3). It cannot see the 2.5–4.0 band at all |
| Frequency, Leipzig sample | cached | CC BY 4.0 | 2.49M tokens: too small to place words in 2.5–4.0 reliably |
| `wordfreq` / SUBTLEX-US | **not installed** | wordfreq code is Apache-2.0 (`data/LICENSES.md`); its English data licence not verified here | This is the source the spec names, and it is not available locally |

**Exit-gate check:** CMUdict is a licence-cleared pronunciation source, so the spec's hard stop does not trigger. But **no adequate, cleared frequency source for Zipf 2.5–4.0 exists locally**, and no tagged headword source for the sense exclusions. Both block a trustworthy Phase B. Please decide whether to approve fetching wordfreq (and its English data licence check) and a Wiktionary English dump.

## C8: F3 dry run (indicative only: see C3 and C7 caveats)

SCOWL lowercase headwords → funnel:

| Step | Rows |
|---|---|
| Zipf 2.5–4.0 (Leipzig-sample Zipf) | 17,084 |
| not mostly capitalised in corpus | 11,517 |
| not already in bank / quarantine | 11,472 |
| not on profanity / blocklist | 11,425 |
| has CMUdict key (fail closed) | 9,591 |
| length ≥ 4 | 9,484 |
| not an obvious inflection of a dictionary lemma | **8,680** |

The remaining 8,680 are far larger than D1's 3,000 target would need, but quality is unproven. Sample: claw, accommodation, fissure, grog, agglomerate, gruesome, methodical, approve, asset, brush, grandson, endpoint, quasar, barrister, aforesaid, scuba, sclerosis, perfidy, genomics. That sample mixes genuinely hard words (perfidy, sclerosis) with ordinary ones (claw, brush, approve, grandson) and obscure ones (grog, aforesaid, quasar). Zipf from a 2.5M-token sample cannot tell them apart. Of the 8,680, 7,886 are also in web2. The raw yield is not a usable estimate of how many rows would survive "an educated adult recognises it".

## C9: F2 dry run (proxy; **failed sanity, so do not rely on it**)

The real F2 (grapheme-phoneme alignment with conditional entropy) is Phase A work. As a stand-in I used letters-versus-phonemes mismatch, schwa count, `extractors.py` features and doubled consonants, weighted 0.6 / 0.3 / 0.1 with the specified rarity and length terms, banded by quartile of the current bank.

- Current Expert rows that would drop a tier: 347 of 797. Lower-tier rows that would rise into Expert: 343.
- Drop sample: particularly, resulting, experience, interface, providing, documentary, requiring, engineering, established, universal, technologies.
- Rise sample: supplies, middle, inner, skills, add, privilege, latter, office, argument, evidence, football, pressure, vision, correct.
- F6 seed check: 9 of 13 keyed Expert-expected words landed in Expert (rhythm and minuscule landed Hard). 2 of 8 Not-Expert seeds landed in Expert (information, environment). That is 25%, against a 5% limit.

The proxy fails A3's bar, and its "rise" list is plainly wrong (*add*, *middle*). Numbers above are for completeness only. The honest finding is that a simple mismatch heuristic is not enough, and F2 needs the real aligner plus calibration. Also note that rhythm, pharaoh and onomatopoeia are classic Expert words that only a real irregularity measure will place correctly (pharaoh and onomatopoeia lack a usable key in this run).

## C10: definitions

- `backend/def_pools/en.json` carries 2,968 definition rows for English: easy 760, medium 768, hard 729, expert 711. Of these, `prompt_grade` is true for 534 / 590 / 558 / 542.
- That leaves 797 − 711 = at least **86 current Expert rows with no definition row**, and about 190 rows across the bank.
- Audit status: `en.json` has no `audit_pass` field. Definition Match reads `audit_pass` and `prompt_grade` from separate audit artifacts (`src/defmatch.rs:21-30,179`), so I could not determine which English rows are audited. I did not find that artifact in this pass.
- Surfaces that read definitions: Definition Match (audit-gated, tier-filtered), Word Stories (flag off, attribution unapproved) and the post-answer meaning panel (`game.rs:914`).

## Items for your review

1. **Frequency source (C7).** F3 and F2's rarity term need real Zipf data for 2.5–4.0. Nothing cleared and adequate is present. Approve fetching wordfreq (and verifying its English data licence), or name another.
2. **Sense tags (C7).** The "no archaic / offensive / dialectal-only" exclusion needs a tagged source. Wiktionary (CC BY-SA, attribution unapproved) is the usual route.
3. **Honest yield versus D1.** The 8,680 candidates are an upper bound of unknown quality. D1's 3,000 can't be judged until items 1–2 are resolved.
4. **Convention check for D9.** The current bank holds one variety per language (`config/bank-variety.json`); `6ea2feae` says English follows the voice. I did not confirm American spelling is the stored convention, though the sample (theater, traveled, center, gray) in that commit message suggests it.
5. **CC-BANK-COMPLETE / REBUILD** are not in `docs/`, so I could not verify D5's text or the "two-source rule" beyond code comments.
6. **Impossible dependency.** Nothing here changes the Impossible decisions already made: English stays locked until a grown, re-tiered Expert tier lands.

## Census exit gate

Not auto-passed. Root-cause verdict is above. The stop-and-ask on C7 does not trigger for pronunciation (CMUdict is cleared) but does apply, in spirit, to frequency and sense-tag sources.
