# Minimal-Pair CANDIDATES — Fiverr Native-Audit Handoff

**Feature:** CC-LEARNING-MODES / feature 3 (minimal-pair learning mode).
**This artifact:** the per-language, pre-gate minimal-pair **candidate** lists +
the one auditor-checklist line to add this round. Review-gated: nothing here
bundles or ships. No engine, no UI, no round variant, no whisper run — those are
deferred. This is generation + packaging only.

---

## ⚠️ These are PRE-GATE candidates (read this first)

A minimal pair here = **two words in the same difficulty tier whose
grapheme-cluster edit distance is exactly 1** (NFC-normalized). That is a pure,
deterministic **text** criterion. It is the **superset**.

The real feature then applies a **whisper.cpp loopback distinctness gate** at
build time: synthesize both words through the TTS cache, transcribe both, and
**drop the pair if whisper confuses them** — auto-removing homophones and
TTS-mushed pairs. That gate needs the TTS backend + whisper audio infra, which
is **not available in this environment**, so it has **not** been run. No whisper
results are invented or implied here.

```
  audited word lists ─► [THIS: grapheme distance-1] ─► candidate TSVs (superset)
                                                            │
                        (separate CI/device step) ─► [whisper.cpp gate] ─► shipped pairs
```

The native auditor reviews the **superset**. Their flags + the later whisper gate
both prune it. Expect over-generation (see Korean note below) — that is by design.

---

## ✅ The auditor-checklist line (add to each of the 14 language checklists)

> **Do these auto-generated word pairs sound genuinely confusable to a learner? Flag any nonsensical pairs.**

The per-language candidate TSV (`<lang>.candidates.tsv`) is the attachment the
auditor reviews against this line. Package it alongside the script-path curricula
as this Fiverr round's handoff. One checklist line, one TSV attachment, per
language.

---

## Files

- `generate_candidates.py` — the deterministic generator (only added dep:
  `grapheme`, a UAX #29 segmenter). Re-run: `python3 generate_candidates.py`.
- `<lang>.candidates.tsv` — one per language with a word list (16 files).

**TSV columns** (tab-separated, header row included):

| col | meaning |
|-----|---------|
| `word1`, `word2` | the pair (NFC), sorted so word1 < word2 |
| `tier` | shared difficulty tier: easy / medium / hard / expert |
| `diff` | the single edit, as `op@pos:detail` — `sub@2:라→무` (substitute cluster 2), `ins@1:+x` (insert), `del@1:-p` (delete). `pos` is a **1-based grapheme-cluster index**, not a code-point index. |

Rows are ordered per tier by **most shared context first** (tightest / most
genuinely-confusable pairs on top), then shortest, then lexicographic —
fully deterministic. Auditors see the best candidates first; single-cluster
"any two short words" noise sinks to the bottom of each tier.

---

## Grapheme clusters, NOT code points (the correctness thing)

Edit distance is computed over **user-perceived characters** (UAX #29 extended
grapheme clusters), never code points. Splitting on code points would over-count
edits inside a stacked Thai syllable, a Korean block, or a Vietnamese
tone-letter, and either miss real pairs or invent fake ones. Worked proof from
the actual generated data:

### Thai — stacked vowel/tone marks collapse into one cluster
- `เสือ` = **4 code points** `เ` + `ส` + `◌ื`(SARA UEE, above) + `อ` → **3
  graphemes** `['เ','สื','อ']`. The consonant `ส` and its above-vowel `◌ื` are
  **one** cluster `สื`.
- Pair `เรือ`/`เสือ` → `sub@2:รื→สื`: exactly one **cluster** (`รื`→`สื`)
  differs. Code-point counting would have mis-scored this.
- `ช้า` = 3 code points `ช` + `◌้`(MAI THO tone) + `า` → 2 graphemes `['ช้','า']`
  — base + tone mark = one cluster.
- (Note: Thai *leading* vowels `เ แ โ ใ ไ` are spacing glyphs and per UAX #29 are
  their own cluster — visible above as the standalone `เ`. That is correct Unicode
  behavior; the stacked above/below vowels and tone marks — the ones the brief
  called out — are the ones that must and do collapse.)

### Korean — the syllable block is one cluster
- `닭` = **1** code point in NFC (U+B2ED), or **3** code points when decomposed to
  conjoining jamo (ㄷ+ㅏ+ㄺ) — either way **1 grapheme**. Blocks are never split
  into jamo.
- Pair `나라`/`나무` → `sub@2:라→무`: the two 2-syllable words share block 1
  (`나`) and differ in block 2 — a genuine one-syllable minimal pair.

### Vietnamese — a tone/diacritic swap on a vowel is distance 1
- `trường` vs `trưởng` → `sub@4:ờ→ở`: the precomposed vowels
  `ờ` (U+1EDD, o-horn + grave) and `ở` (U+1EDF, o-horn + hook-above) are **one
  grapheme each**; swapping the tone is a single-cluster substitution. In NFD each
  word is 9 code points but still **6 graphemes** — the counting is identical
  either way because we segment, not split.

---

## Per-language candidate counts

| lang | easy | medium | hard | expert | total |
|------|-----:|-------:|-----:|-------:|------:|
| en | 4 | 0 | 0 | 0 | **4** |
| es | 5 | 0 | 0 | 0 | **5** |
| fr | 3 | 0 | 0 | 0 | **3** |
| de | 6 | 2 | 0 | 0 | **8** |
| pt | 8 | 0 | 0 | 0 | **8** |
| it | 2 | 1 | 0 | 0 | **3** |
| nl | 9 | 1 | 0 | 0 | **10** |
| pl | 3 | 0 | 0 | 0 | **3** |
| sv | 7 | 1 | 0 | 0 | **8** |
| nb | 3 | 2 | 0 | 0 | **5** |
| fil | 10 | 0 | 1 | 1 | **12** |
| vi | 32 | 10 | 7 | 7 | **56** |
| ko | 362 | 118 | 7 | 9 | **496** |
| ja | 95 | 36 | 5 | 1 | **137** |
| **total** | | | | | **813** |

**zh: SKIPPED.** `assets/words/zh` does not exist (empty/absent — no audited
Chinese word list to generate from). No `zh.candidates.tsv` is produced. When a
zh word list lands, re-run the generator.

### Why Korean is big (and why we did NOT cap it away)

Korean's total (496) dwarfs the others because a Hangul **syllable block is one
grapheme cluster** (as the brief mandates — jamo splitting is *not* allowed). The
`easy` tier has 27 single-syllable words, so **every** pair of them is trivially
"one cluster substituted" = distance 1 → C(27,2) = 351 pairs, almost all sharing
zero context.

Critically, grapheme distance **cannot** tell a great single-syllable minimal
pair from nonsense here: `소`/`코` (s→k, one phoneme apart — excellent) and
`발`/`밤` (final-consonant contrast — excellent) look **identical** to
`귀`/`빵` (unrelated — nonsense): all three are "one block substituted, shared
context 0." Distinguishing them requires phoneme/audio info that only the
**whisper gate + native auditor** have. So capping the single-cluster bucket
would arbitrarily discard genuine minimal pairs. We therefore **retain the full
superset** and rely on the downstream prune — which is exactly what the checklist
line ("flag any nonsensical pairs") is for. The tightest pairs are sorted to the
top of `ko.candidates.tsv`; the single-syllable bucket sits at the bottom for
fast bulk triage.

**Caps applied this round: none.** The generator keeps a per-tier safety valve
(`PER_TIER_CAP`, currently 1000) that would keep the most-shared-context pairs
and **log** any drop — it did **not** fire on today's lists (largest tier
ko/easy = 362). Nothing was silently truncated.
