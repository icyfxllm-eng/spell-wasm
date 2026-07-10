# Difficulty & Expert-Depth Audit — Phase 1 findings

Audit-first, per the work order's STOP GATE. No scoring code changed.

## 1. Pool census (words per tier, all languages)

| lang | easy | medium | hard | expert | Expert vs required (≥500) |
|------|-----:|-------:|-----:|-------:|---------------------------|
| en | 50 | 50 | 50 | 50 | **8% of minimum** (short 450) |
| es | 50 | 50 | 50 | 52 | 10% |
| fr–tr (9 EU langs) | 40 | 40–43 | 40 | 40–46 | ~8% |
| vi | 47 | 46 | 46 | 46 | 9% |
| ko | 45 | 44 | 44 | 44 | 9% |
| ja | 42 | 42 | 42 | 41 | 8% |
| fil | 46 | 46 | 46 | 46 | 9% |

**Total across all 15 languages × 4 tiers: 2,574 words.** The spec requires
≥500 Expert + ≥700 Hard + ≥1000 Medium = **≥2,200 per language for Climb alone**
— i.e. one language's requirement exceeds the entire current corpus.

**Central finding:** every pool is ~40–52 words/tier. No language is within an
order of magnitude of the depth minimums. This is the defect that dominates all
others — feature-model sophistication is moot until the pools are ~10× larger.

## 2. Current scoring inspection

There is no runtime difficulty-scoring module in `spell-core`. Difficulty is a
**build-time tier assignment**: `scripts/build-wordlists.py` reads curated
per-tier source files; a re-tiering pass sorts each language by a per-language
score and partitions into easy/medium/hard/expert:

- **Latin (en/es/fr/…):** length + diacritic/digraph weight.
- **vi:** length + diacritic count (tone load).
- **ko:** syllable count + batchim/compound-vowel complexity.
- **ja:** kana count + dakuten/small-kana density.
- **fil:** length + ñ + hyphen.

So per-language feature signals **do** exist (not one generic formula), but they
are coarse (a handful of features) and there is no frequency axis, no grade
calibration (HSK/JLPT/TOPIK), and no distribution-separation enforcement.

## 3. Distribution / separation

With only ~40 words/tier the re-tiering guarantees monotonic **mean length**
per tier (verified elsewhere) but cannot support percentile separation tests
(Expert p10 > Hard p50) meaningfully at n=40. Cannot be fixed by re-scoring —
needs depth.

## 4. Spot-check

Expert pools are real but shallow (e.g. en: `onomatopoeia, sacrilegious,
idiosyncrasy…`; ja: `おうだんほどう, きゅうきゅうしゃ…`; ko multi-syllable +
double-batchim words). They *feel* expert — there simply aren't 500 of them.

## Conclusion & what's actually blocking Phase 2+

The depth minimums cannot be met with hand-authored lists. They require the
**lexicon infrastructure** (the sibling work order): ingesting JMdict, CC-CEDICT,
CMUdict, HSK/JLPT/TOPIK lists, kaikki, wordfreq — thousands of words/language
with frequency + POS + readings + grade tags.

Those are **external prerequisites this environment cannot satisfy**:
- GB-scale dictionary downloads (no build-time network; the lexicon spec itself
  says ingestion runs offline, not in CI).
- **License verification** (CC-BY-SA obligations) — a hard human/legal gate.
- **Native-speaker review** of Kid-Mode + Expert pools per language — a hard gate.

**Recommendation:** treat this + the lexicon + Mandarin specs as one program.
The self-contained *engines* are done and tested (Vietnamese tone, Hangul
automaton, kana long-press transforms, pinyin normalizer). The *data* pieces are
gated on sourcing + licensing + native review, which are Eric's to unblock —
per the specs' own STOP GATES. This report is that Phase-1 gate.
