# CC-VI-BANK-GROWTH §0 census

**Run 2026-10-06. Read-only: no bank row, registry entry or app code was changed.**
**STOP — Phases A–D are blocked on Eric's review of this document.**

**Six HALTs fired: C5, C7, C8, C10, C11, C12.** Three census items (C8, C9 and
most of C6's projection) could not be completed because they depend on sources
that are not local and on a licence verdict only Eric can give.

---

## Before the items: this file's premises have moved

The spec says the vi bank "holds about 8,000 rows" and that growth requires "a
change of unit (syllable → word)". Both were true when it was written. Neither is
true now — the bank was cleaned and rebuilt on 2026-10-05/06, and the change of
unit has already happened.

| the spec assumes | measured today |
|---|---|
| ~8,000 rows | **5,680** |
| rows are single tokens, at the syllable ceiling | **77% are multi-syllable** |
| multi-syllable answers need enabling (D1) | already allowed; the vi keyboard has a space key |
| a Vietnamese-only gate must be built (F4) | a syllable-structure gate is already in `build-wordlists.py` |
| the vi mic is in scope to keep absent | already withdrawn, with a test |

**This matters most for F8.** Its lint says legacy rows — pre-existing, fewer
than two sources — are "counted, not failed", and that "the legacy count may
never increase". Every one of tonight's 5,680 rows comes from ONE source
(Leipzig). Adopting F2 as written makes the entire bank legacy and freezes that
count at 5,680, which is a stricter starting point than the spec anticipated.
Eric should decide whether tonight's rows are legacy or are re-derived through
F2's two-source rule.

---

## C1 — Unit

5,680 rows.

| syllables | rows |
|---|---|
| 1 | 1,262 |
| 2 | 4,067 |
| 3 | 346 |
| 4 | 5 |

Capitals 0 · digits 0 · hyphens 0 · non-Latin 0.

**D1 caps: zero breaches.** The five 4-syllable rows are all in Expert, which
permits 4. No Easy/Medium/Hard row exceeds 3.

Multi-syllable share **77%**, already above D6's provisional ≥60%.

| tier | rows | multi-syllable |
|---|---|---|
| easy | 840 | 0 (0%) |
| medium | 840 | 419 (49%) |
| hard | 1,500 | 1,499 (99%) |
| expert | 2,500 | 2,500 (100%) |

## C2 — Provenance · HALT did not fire

Script: `scripts/build-bigbank.py`. Source: `vie_wikipedia_2021_100K` — the
Leipzig Corpora Collection, CC BY 4.0, Wikipedia-derived, cached at
`.corpus-cache/`. Fourteen languages share this script: ar de en es fil fr hi ja
ko pl pt ru sw vi. **Reported only; nothing else was touched.**

**Did it tokenize on whitespace? No — and this refutes part of the spec's
hypothesis.** Leipzig ships Vietnamese already WORD-segmented: of 142,237
entries in its words file, 76,700 contain a space (`đầu tiên`, `có thể`, `khoa
học`). The list was never a syllable list.

So the contamination did not come from splitting on spaces. It came from the
absence of a Vietnamese-only gate: the corpus is Wikipedia, Wikipedia contains
English, and nothing rejected it. That is precisely what F4 proposes, and what
the syllable filter added on 2026-10-06 already does.

**HALT check:** no other instruction file owns vi bank admission.
`docs/CC-MASTER-PARITY.md` owns the same pipeline for ru/ar/sw only (its own
title scopes it), and `build-bigbank.py` names those three in a comment. No
second pipeline is being built.

## C3 — Residual contamination · clean

All 5,680 rows swept with the draft F4 Layer-1 parser.
**Zero structurally illegal syllables.** `reports/vi-quarantine-candidates.csv`
is written and contains only its header. No row was removed or edited.

Layer 2 (attested) could not be run: it is defined as "occurs in a two-source
headword", and the two sources are not local — see C8.

## C4 — Space readiness · no surface splits, but D8 conflicts with the matcher

| surface | behaviour with one internal space |
|---|---|
| answer matcher | **drops whitespace entirely** (`norm.rs` fold_strict/fold_lenient) |
| vi keyboard | space key present (added 2026-10-06) |
| bank builder | accepts a space for vi |
| answer rendering | already handled — `<span class="ltr space">` |
| From a photo | does **not** split on space; its splitters are `, ; / \| ，；、` |
| photo line cleanup | collapses whitespace runs to one space — matches F1 |

**No surface strips, splits on, or rejects an internal space, so C4's HALT does
not fire on that wording.** But there is a direct conflict to settle:

> **D8 (DEFAULT): "A missing or extra space is a spelling error like any other
> character. No auto-accept of `họcsinh` for `học sinh`."**

Today `họcsinh` IS accepted, because `fold_strict` filters whitespace for every
language. Honouring D8 means changing the universal matcher, which affects all
fifteen languages and is outside this file's stated ownership. **Eric's call,
and it should be explicit:** the current behaviour was a deliberate property —
it is what let the space key be an affordance rather than a new way to be wrong.

## C5 — Voice dialect · **HALT**

Layer-1 vi voice: **`vi-VN-Wavenet-A`** (Google, `backend/app.py:87`).

**The dialect cannot be established.** Google's voice list publishes the locale
`vi-VN` and no dialect field, and nothing in this repository records one. The
spec forbids guessing from listening, so the sound-alike grouping in C5, every
"active trap" determination in F6, and the Easy-tier projection in C6 are all
blocked behind it.

What would settle it: a statement from the provider's documentation, or Eric's
own determination recorded in the source registry.

## C6 — Tier depth · partial

Rows per tier today: easy 840 · medium 840 · hard 1,500 · expert 2,500.

The projections cannot be computed: both readings of D13 depend on trap tags,
which depend on the dialect (C5 HALT). Reported as blocked rather than guessed.

The tier calibrator's acceptance of trap input for vi was not determined.

## C7 — Tone convention · **HALT**

The convention used by "the vi dictionary authority" cannot be established:
no vi dictionary source is present in the repository, and the spec names none
beyond "the authority" (C8's sources are the candidates).

Existing rows were not counted by convention, because without an authority the
counts have no reference to be counted against.

## C8 — Sources and licences · **HALT (not run)**

Neither F2 source is local:

* `kaikki-vi` — absent. (`tools/lexicon-ingest/sources/` holds only
  `kaikki-ru.jsonl`, 893MB, gitignored.)
* `hnd-wordlist` — absent.

`data/vi/SOURCES.md` already exists and lists a DIFFERENT candidate set —
Hunspell Vietnamese, kaikki.org (CC BY-SA), wordfreq — each with its licence
`_TODO_`, under a standing instruction: **"STOP GATE: verify each licence's
current text at ingest time (do not trust this template). Present to Eric before
ingesting."**

Downloading was not done. D5 makes the licence check Eric's to record, and the
census's own HALT is "if a licence differs from D5's description", which cannot
be evaluated without the files and a human reading them. Combined with
`docs/census/bank_provenance.md` — which found CC BY attribution already missing
for fourteen banks — this is the wrong week to acquire two more sources without
the paperwork first.

## C9 — Yield · blocked on C8

Cannot be measured without both sources. D6's 20,000-row target therefore
remains an estimate.

For scale, from the source that IS local: the Leipzig vi corpus yields **31,544**
entries passing the syllable filter, of which 27,003 are multi-syllable. That is
one source and cannot satisfy F2's two-source rule on its own, but it bounds the
order of magnitude.

## C10 — Profanity coverage · **HALT**

`assets/words/exclusions/vi.txt` contains **zero entries** — one comment line:
"Vietnamese exclusions (accent-lenient, lowercased match). Extend from LDNOOBW
vi."

The seed is empty, which is exactly the HALT condition. The spec's reasoning
holds and is worth repeating: a compound can be vulgar when its syllables are
not, so this matters more as multi-syllable rows grow, not less.

## C11 — Equivalence layer · **HALT**

`CC-PLAYER-CONTRACT.md` is **not present in this repository**, though five other
spec files reference it. No equivalence layer by that name exists in `src/`.

The nearest existing mechanism is `src/homophones.rs` — the data-driven
accept-any-homophone layer consulted at the submit path — which may be the
intended home for F5's registration, but that is an inference, not a reading of
the contract.

CC-SENSE-CUE, by contrast, IS merged: `valid_other_sense` exists in `game.rs`
and `feedback.rs`.

## C12 — Unaccented input at Easy · **HALT**

Cannot be reported: the rule lives in `CC-PLAYER-CONTRACT`, which is not in the
repository (C11). Its interaction with F6's "zero active-trap rows at Easy" is
therefore unevaluable, and it is also gated behind C5's dialect.

---

## What Eric is being asked to decide

1. **C5** — the dialect of `vi-VN-Wavenet-A`. Blocks F6 entirely, and C6 and C12 with it.
2. **C8** — acquire and licence-clear `kaikki-vi` and `hnd-wordlist`, or name different sources. Blocks C9, F2 and everything downstream.
3. **C10** — the vi profanity seed is empty and must be filled before growth.
4. **C11/C12** — supply `CC-PLAYER-CONTRACT`, or name what replaces it.
5. **C7** — name the dictionary authority whose tone convention is canonical.
6. **D8** — whether `họcsinh` should stop matching `học sinh`. This reverses shipped behaviour across all fifteen languages, not just vi.
7. **F8 legacy** — whether tonight's 5,680 single-source rows count as legacy (freezing the count there) or must be re-derived through F2.

Nothing in Phases A–D was started.
