# Word-list sources & licenses

The built-in gameplay pools are compiled by `scripts/build-wordlists.py` from the
curated sources in `assets/words/{code}/{tier}.txt` into `src/word_data.rs`. This
file records where the data comes from and under what terms it ships.

## Current sources

| Data | Source | License | Notes |
|------|--------|---------|-------|
| Word banks — hand-authored core | **Original curation** for this app | Owned — ships freely | The Easy tiers are substantially this core. 16 locale directories under `assets/words/`. |
| Word banks — corpus volume (14 locales) | **Leipzig Corpora Collection** (wortschatz-leipzig.de), Wikipedia editions | **CC BY 4.0** (as recorded — see the verification note) | Build input only, cached under `.corpus-cache/` and never redistributed; only the filtered word lists ship. Pinned corpora listed below. |
| Keyboard layouts (`assets/keyboards/*.json`) | Original | Owned | Single source of truth for the runtime keyboard **and** the charset gate. |
| Exclusion roots (`assets/words/exclusions/_roots.txt`) | Seeded from `src/profanity.rs` | Owned | Extend per-language from LDNOOBW (see below). |

### Corrected 2026-10-07

This section used to read "Word banks (11 locales × 4 tiers) — Original curation
— Owned — ships freely", and the paragraph that followed it said there was **no
third-party lexicon license to satisfy for the shipped binary**. Both were
wrong, and they contradicted `NOTICES.md` in this same repository, which has
recorded the Leipzig Corpora Collection at CC BY 4.0 all along. Three separate
claims were false:

1. **The count.** 11 locales, when there are 16 bank directories.
2. **The sole source.** "Original curation", when 14 of the banks are grown from
   a pinned Leipzig Wikipedia corpus by `scripts/build-bigbank.py`. The
   hand-authored core is real — Leipzig was layered on top of it and this file
   never caught up.
3. **The tiering.** "Tiers reflect human spelling-difficulty judgement, not raw
   frequency." `build-bigbank.py` tiers corpus words by LENGTH (`TIER_LEN`).
   Tier membership is a length band over a hand-authored core, not a difficulty
   judgement.

The evidence that settled it is in the shipped data: seven banks carry localised
MediaWiki namespace words that only occur in reference text — `citation`,
`edition`, `retrieved` (en), `categoría`, `isbn` (es), `copyright`,
`référence` (fr), `kategorie`, `wikipedia` (de), `doi` (vi). That is a dump
fingerprint, not a coincidence. See `docs/census/bank_provenance.md`.

### The pinned Leipzig corpora

`scripts/build-bigbank.py` pins one corpus per language so a rebuild is
reproducible. The Wikipedia editions were chosen for diverse, neutral
vocabulary.

| | | | |
|---|---|---|---|
| en `eng_wikipedia_2016_100K` | es `spa_wikipedia_2021_100K` | fr `fra_wikipedia_2021_100K` | de `deu_wikipedia_2021_100K` |
| pt `por_wikipedia_2021_100K` | pl `pol_wikipedia_2021_100K` | vi `vie_wikipedia_2021_100K` | ko `kor_wikipedia_2021_100K` |
| fil `tgl_wikipedia_2021_100K` | ja `jpn_wikipedia_2021_100K` | ru `rus_wikipedia_2021_100K` | ar `ara_wikipedia_2021_100K` |
| sw `swa_wikipedia_2021_100K` | hi `hin_wikipedia_2021_100K` | | |

Chinese is the exception: the zh bank comes from `scripts/build-zh-bank.py`
(`cmn_wikipedia_2021_100K`) with CC-CEDICT glosses. Hindi also has
`scripts/build-hi-bank.py`. Both are recorded in `NOTICES.md`.

Each archive carries its own `-sources.txt` listing every source article URL and
retrieval date, which is first-hand confirmation of what the text is. Spot-checked
2026-10-07: en spans 2009-03-10 to 2016-08-20, es 2011-01-19 to 2021-05-20, fil
2007-03-11 to 2021-05-20.

### Verification note (open)

The CC BY 4.0 term above is carried from `NOTICES.md` and from
`scripts/build-bigbank.py`, which both state it. It could **not** be confirmed
against wortschatz-leipzig.de on 2026-10-07 because that site is behind
proof-of-work anti-bot protection, and the corpus archives contain no licence
file. Two things are therefore still worth a human check:

* The Leipzig terms page, to confirm CC BY 4.0 and the citation they ask for.
* Whether the Wikipedia underlay matters. The corpus text is Wikipedia, which is
  CC BY-SA, while Leipzig distributes the collection as CC BY. For single common
  words the question is probably moot — a word list is facts rather than
  expression, and no sentence or phrasing is redistributed — but this file
  should not be the place that decides it.

`NOTICES.md` is the canonical third-party attribution record for this project and
should be read alongside this file. Note that it is a repository document: it is
not currently copied into `dist/` or the app bundle, so none of the attributions
it holds reach a user.

## Intended expansion sources (not yet ingested)

The pools HAVE grown beyond hand curation -- that growth was Leipzig, recorded
above. These are the sources still unused, and the rule that was broken once
already stands: each must be recorded here with its license **before** any
derived data is committed, and per-language Hunspell licenses vary (GPL/LGPL/MPL/BSD) — substitute
an alternative open lexicon for any language whose license is incompatible with a
closed-binary release, and document the substitution.

- **Base lexicons:** Hunspell dictionaries (LibreOffice set) — license per language.
- **Frequency data:** hermitdave/FrequencyWords (OpenSubtitles-derived) and/or the
  `wordfreq` dataset — drives frequency-band tiering (§3.2).
- **Kid Mode vocab:** CEFR A1/A2 lists per language, intersected with the base
  lexicon.
- **Profanity / exclusions:** LDNOOBW per-language lists
  (github.com/LDNOOBW/List-of-Dirty-Naughty-Obscene-and-Otherwise-Bad-Words),
  used both to exclude gameplay words and to filter The Climb usernames (§4.4).

## Build gates (enforced by the pipeline)

1. **Charset** — every character of every word is reachable on that locale's
   keyboard (`assets/keyboards/{code}.json`).
2. **Exclusions** — no word matches a locale exclusion or a shared root.
3. **Balance** — each tier within ±20% of the English tier count.
4. **Determinism** — output is a canonical sorted function of the inputs.

## QA / launch checklist (§3.5)

- [ ] Cross-validate each pool against Hunspell spellcheck (100% pass) once
      Hunspell dictionaries are available in the build environment.
- [ ] Native-speaker spot-check of the shipped pools — flag archaic, offensive
      or bizarre entries. (This line used to name nl / sv / nb / tr, which are
      cut from the registry; `src/consts.rs` is the live list. The open
      mark-fold audit in `docs/census/mark_fold_audit.md` is the current form of
      this item.)
