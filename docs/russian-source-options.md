# Russian word-list source options — review artifact for Eric

**Status: DECISION REQUESTED. No source is chosen and no Russian content exists.**

Prepared 2026-07-16 for CC-NEW-LANG-CONTENT F1 (Russian to fully playable). This
document proposes candidates and names what only you can settle. It deliberately
does **not** pick a source: `sources/registry.json` demands a human license
verdict, and the CC-WORDLIST-SOURCES license addendum says `verified_by` must
name a person. Every license below is **reported, not verified** — treat each as
`UNKNOWN` until someone reads the artifact's own LICENSE file.

---

## 1. What the pipeline requires before a single Russian word may ship

From `sources/registry.json` and `scripts/license-gate.mjs` (both on
`feature/wordlist-sources`, unmerged):

- **Exactly ONE source per language. No merging** (D2). This is the constraint
  that makes the choice below matter — we cannot hedge by combining a lexicon
  with a frequency list.
- A complete registry entry: name, url, SPDX license, tier, version, **pinned
  commit**, artifact path, tarball URL, **tarball sha256**, retrieved date.
- A complete `sources/ru/` directory: `fetch.sh` (pinned + sha256-verified,
  never scrapes), `LICENSE`, `PROVENANCE.md`.
- Tier **A** (permissive/public-domain) or **B** (attribution/share-alike) only.
  Tier **C** is stop-and-ask.
- Only then may `wordlists/ru.txt` exist.

**How Spanish actually works, because it sets the standard.** The source does not
supply the word list. `rla-es` is unmunched into a **951,893-form surface index**;
the shipped list is **202 hand-curated words** *validated against* that index —
196 backed by source, 6 reviewed exceptions, **0 genuine misses**. So the source's
job is to **prove every curated word is a real word**. Curation and tiering stay
human. Russian needs the same: a lexicon broad enough to back ~200 curated words.

---

## 2. The Russian-specific problem: ё

**This is the finding that should drive the choice, and it has no Spanish analogue.**

CC-LINEUP-SWAP D4 requires canonical forms to **store ё** where dictionaries write
it (`ёж`, `лёд`, `всё`), while accepting typed `е` and displaying `ё` on reveal.
The keyboard already supports this — `ё` is a long-press on `е` (`50f0909`).

But **everyday Russian writes `е` for `ё`**, and many corpora and dictionaries
follow suit. If the chosen source's surface index is ё-less, then every canonical
ё-form is a **genuine miss** in provenance validation — and `genuine_misses` must
be `0`. The fallback is the exceptions allowlist, but Spanish needed 6 exceptions
out of 202. Russian could need dozens, which stops being an allowlist and starts
being an admission the source doesn't back the content.

**So the first question of any candidate is: does it preserve ё?** A source that
scores well on license and size and fails here is the wrong source.

Secondary, cheaper wrinkles:

- **Alphabet.** Eligibility must be `абвгдеёжзийклмнопрстуфхцчшщъыьэюя` (33,
  **including ё**), mirroring `es`'s `abcdefghijklmnopqrstuvwxyzáéíñóúü`.
- **Length rules mismatch — worth fixing while we're here.** `es.manifest.json`
  ingests at `min_len 3 / max_len 15`; `scripts/build-wordlists.py` gates at
  `2..16`. Harmless for `es`, but Russian's inflection makes the boundary
  livelier. Pick one before ingesting.
- **Inflection volume.** Spanish unmunched to ~1.2M forms. Russian is far more
  inflected (6 cases × number × gender, full verb paradigms); an unmunch could be
  several million forms. Fine for a build-time index, but `fetch.sh` and the
  index file will be materially larger than Spanish's.

---

## 2a. Size baseline — what "parity" actually means

Measured 2026-07-16 from `assets/words/<lang>/<tier>.txt` (comments/blanks
excluded). This is the CC-WORDLIST-RAFU F1 baseline, computable today because it
needs no source:

| tier | median | min | max |
|--------|-------:|----:|----:|
| easy | 41.0 | 40 | 50 |
| medium | 42.5 | 40 | 50 |
| hard | 41.0 | 40 | 50 |
| expert | 44.0 | 40 | 52 |

Per-language totals run 160 (`fr`, `pt`, `tr`) to 202 (`es`). **Russian's target
is therefore ~40–50 words per tier, ~165–200 total** — the same order as the 202
curated Spanish words. That is a hand-curatable quantity, which is the point:
the source backs curation, it does not replace it.

Two caveats on the baseline:

- **`zh` has no source files at all.** Mandarin's banks live directly in
  `words.rs` as `pinyin|hanzi` pairs, explicitly "not run through the pipeline"
  because its charset gate assumes single-string words. So the "eleven ready
  languages" are really **ten pipeline languages + one special case**. Any
  tooling that computes a baseline by globbing `assets/words/*/` will silently
  score `zh` as zero, as mine did until I checked.
- **ru/ar/fa/ur currently have no source directory and zero words in every tier.**

## 3. Candidates

Licenses below are **reported and must be verified against the artifact itself.**
I have not fetched any of these.

### A. Hunspell `ru_RU` (LibreOffice dictionaries) — *recommended for evaluation first*

- **What:** the standard Russian Hunspell dictionary (historically Alexander
  Lebedev's), the direct structural analogue of `rla-es`.
- **Reported license:** BSD-style. **Needs verification** — LibreOffice ships
  dictionaries under mixed terms per language, and the Russian one's history is
  not something to take on trust.
- **Tier if confirmed:** likely **A**.
- **ё:** believed to distinguish ё. **Must be confirmed by inspecting the `.dic`.**
- **Why first:** identical pipeline shape to `es` — `fetch.sh` pulls a pinned
  tarball, unmunch `.dic`/`.aff` into a surface index. `scripts/` already does
  this; no new machinery. If its license is genuinely BSD, it is **Tier A**,
  which sidesteps the copyleft posture problem in §4 entirely.

### B. OpenCorpora

- **What:** open Russian corpus + morphological dictionary (the basis of
  `pymorphy2`). Rich, actively maintained, explicitly marks ё.
- **Reported license:** CC BY-SA. → **Tier B** (attribution + share-alike).
- **ё:** strong — ё is marked in the morphology.
- **Cost:** not a Hunspell artifact, so unmunching doesn't apply; it needs its own
  extraction step. That is new machinery the `es` path doesn't have.

### C. Wiktionary via kaikki.org

- **What:** machine-readable Wiktionary extraction; `LICENSES.md` already names
  Wiktionary/kaikki as an intended expansion source.
- **Reported license:** CC BY-SA (Wiktionary's terms). → **Tier B**.
- **ё:** headwords generally written with ё, which suits D4.
- **Cost:** extraction + quality variance; Wiktionary contains inflected forms,
  archaisms, and proper nouns that the eligibility rules would need to strip.

### D. hermitdave/FrequencyWords, `wordfreq` — **not a lexicon**

Named in `LICENSES.md` for frequency-band tiering. They tell you a word is
**common**, not that it is **correctly spelled**, so neither can back provenance.
And D2's one-source rule means we cannot pair a frequency list with a lexicon
without a decision from you. Listed to rule out.

---

## 4. The decision that outranks the source choice

`sources/registry.json` already records this, unresolved, for Spanish:

> **NOTE: shipping copyleft-derived data inside a closed binary is a posture
> decision for Eric, not this pipeline.**

`es` is **Tier B** (GPL/LGPL/MPL tri-license) and ships inside a closed App Store
binary. **That question is already open and Russian does not create it** — but
Russian is the moment to answer it, because the addendum's whole purpose is the
claim *"every word list traces to a documented source whose license permits this
use, and the build fails otherwise"*, made to institutional buyers who carry
copyright liability. That claim and an unresolved copyleft posture cannot both
stand.

If the answer is *"avoid copyleft for new languages"*, that **selects option A**
(if BSD confirms) and demotes B and C. If Tier B is acceptable, all three are live
and ё should decide.

---

## 5. What I need from you

1. **Copyleft posture** (§4): is Tier B data acceptable inside the closed binary?
   Answering this may collapse the choice to one candidate.
2. **Source pick** — A, B, or C — pending license verification.
3. **A verifier.** The addendum requires `verified_by` to name a person. I cannot
   fill it, and the gate fails closed on `UNKNOWN` for active/audit-ready
   languages. Russian is `ComingSoon`, so today it is a *warning*, not a failure —
   but it hard-blocks Russian ever reaching audit-ready.
4. **Length rules** (§2): reconcile ingest `3..15` vs build `2..16`.

## 6. What happens once you answer

1. `sources/ru/` — `fetch.sh` (pinned tarball + sha256), `LICENSE`,
   `PROVENANCE.md`.
2. `sources/registry.json` — the `ru` entry, with your verdict and verifier.
3. Extract → `sources/ru/surface-index.txt`; **report the ё coverage** before
   curating anything.
4. Curate ~200 words tiered per CC-NEW-LANG-CONTENT D3 (easy = spelled as
   pronounced; medium = hard/soft signs, final devoicing, и/ы; hard = unstressed
   о/а and е/и reduction, -тся/-ться, doubles; expert = long morphology, prefixed
   verbs, loanwords). **This needs a native speaker — it is not something I should
   do unreviewed**, and D3's done-criteria say each tier's sample passes your
   eyeball review.
5. Provenance-validate to `genuine_misses: 0`.
6. Add `ru` to `LANGS` in `scripts/build-wordlists.py`, add the `RU` arm to
   `tier_for` (it currently returns an **empty** bank by design — see `50f0909`),
   wire the `ru-RU` backend voice.
7. Russian stays `ComingSoon` until its Gig A/B audit passes. Per your call on
   2026-07-16, it does **not** activate before audit, despite
   CC-NEW-LANG-CONTENT F1's "activates into every game mode now" — that ordering
   is circular (the same file commissions the audit only *after* F1 is green).

---

### Already done (`50f0909`, branch `feature/ru-parity`)

- Cyrillic ЙЦУКЕН keyboard: `assets/keyboards/ru.json` + the `RU` layout, all 33
  letters reachable, ё as long-press on е. JSON↔Rust SSOT test green.
- `tier_for` no longer falls through to the **English** bank for ru/ar/fa/ur —
  they return empty, with two tests pinning it. The charset gate caught this the
  moment Russian had a real keyboard (`char 'b' in "bed" not reachable`); before
  that, "Russian" would have served English words.

---

## 7. This document is CC-WORDLIST-RAFU's precondition

CC-WORDLIST-RAFU (2026-07-16) asks to "expand and strengthen" these four lists.
It cannot start, and the reason is structural rather than a matter of effort:

- It requires every word to come from **CC-WORDLIST-SOURCES' per-language source
  of record** and states **"No new sources may be introduced by this work."**
  `sources/registry.json` contains exactly **one** source: `es`. There is no
  source of record for ru, ar, fa, or ur. So RAFU forbids introducing a source
  while requiring words to come from one that does not exist. **Choosing
  Russian's source (§5 above) is the unblock** — it is CC-WORDLIST-SOURCES'
  work, not RAFU's.
- It describes the four as having **"bootstrap-thin word lists"** and asks for
  "duplicate/normalization anomalies in the existing bootstrap list". There is no
  bootstrap list. All four are at **zero words**, with no source directory (§2a).
  Nothing exists to measure anomalies in, deduplicate, or strengthen.

What RAFU asks for that **is** deliverable now, and is in §2a: the ready-language
size baseline. The rest of its gap report — trap-class coverage, TTS cache
coverage, dedupe anomalies — all measure lists that do not exist yet.

Its ZWNJ observation is correct and already tracked: CC-RTL D7 marks the Persian
ZWNJ policy "proposed", so Persian words requiring ZWNJ must be held pending that
decision. Worth noting it mirrors the ё policy exactly — store the canonical
form, accept the form users actually type — so deciding ё's precedent well makes
ZWNJ's decision easier.

### Not started, and blocked elsewhere

- **ar/fa/ur sources.** Same license questions, but their pipeline additionally
  needs keyboard layouts, which CC-NEW-LANG-CONTENT lists as a non-goal and
  CC-RTL D8 owns behind its unfinished P0.3 prototype gate.
