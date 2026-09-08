# Data licenses & obligations (STOP GATE — Eric decides shipping posture)

This file summarizes the licensing obligations of every data source that
contributes to a shipped lexicon/pool. **It states each license's requirements;
it does not give a legal conclusion.** Fill in per-source verbatim requirements
(with citations) as each `data/<lang>/SOURCES.md` is completed, then Eric
decides the shipping posture.

## Current shipped data

The lists shipping **today** (`assets/words/<lang>/*`, `src/words.rs`) are
**original curation** owned by this project — no third-party obligation.

## Sources pending ingestion (obligations to record before shipping derived data)

Several primary sources are **CC BY-SA**. Shipping word lists **derived** from
them inside the app likely triggers attribution + share-alike considerations for
the *data files* (not the app code). Record exact requirements here before any
derived lexicon is promoted to `assets/words/`:

| Source | License | Attribution string required? | Share-alike scope | Recorded |
|--------|---------|------------------------------|-------------------|----------|
| JMdict (EDRDG) | CC BY-SA | Yes — EDRDG notice | data files | _TODO verbatim_ |
| EDICT2 (EDRDG) | CC BY-SA | Yes — EDRDG notice (recorded below) | data files | ingested 2026-09-07, build-time only |
| CC-CEDICT | CC BY-SA 4.0 | Yes | data files | _TODO verbatim_ |
| kaikki.org / Wiktionary | CC BY-SA | Yes | data files | _TODO verbatim_ |
| CMUdict | BSD-style | Notice only | — | _TODO verbatim_ |
| WordNet | Princeton | Notice only | — | _TODO verbatim_ |
| wordfreq | **Apache-2.0** (verified 2026-09-07, wheel METADATA) | No | — | build-time only; distilled to tools/wordpipe/sources/freq_{ko,ja}.txt |
| Leipzig Corpora | research | verify redistribution | _TODO_ | _TODO_ |
| NECTEC Lexitron (th) | registration | **ASK ERIC — do not ingest without approval** | — | blocked |

## In-app credits

CC-BY-SA sources require **visible attribution**. When derived data ships, add a
Settings → About → **Data Sources / Licenses** screen listing each source's
required attribution string. (Not built yet — follow-up once a source is
actually ingested and Eric approves the posture.)

## Ingested 2026-09-07 — CJK frequency

Korean and Japanese had no difficulty signal except word length — which is
also what the calligram packer needs diverse inside each tier, so the two
requirements were in direct conflict. Frequency breaks that tie.

**Neither source ships.** Both are consumed at build time and distilled to
`tools/wordpipe/sources/freq_ko.txt` and `freq_ja.txt` — two-column files
(~100K each) covering only the words already in the bank, in the same shape as
the `freq_en.txt` that has been in the tree all along. Regenerate with
`tools/build_freq_cjk.py`.

| Source | License | Used for | Coverage |
|---|---|---|---|
| wordfreq 3.1.1 | Apache-2.0 | frequency bands, ko + ja | ko 63.7%, ja 38.2% direct |
| EDICT2 | CC BY-SA (EDRDG) | ja kana->kanji join | lifts ja to 78.1% |

The Japanese join is why EDICT2 is here: the bank spells Japanese in kana,
wordfreq indexes kanji, and the two do not meet without a reading table.

### EDRDG attribution (required)

> This publication has included material from the JMdict/EDICT dictionary
> files in accordance with the licence provisions of the Electronic Dictionary
> Research and Development Group. See http://www.edrdg.org/

### STILL OPEN — Eric decides

This file's header makes shipping posture his call, and the lists shipping
today are recorded above as original curation with no third-party obligation.
Whether a tier ORDERING derived from CC BY-SA data counts as derived data is a
judgement, not a certainty: frequency counts and reading pairs are facts, and
the shipped artifact is this project's own word list in a different order. If
it does count, the Settings → About → Data Sources screen named above becomes
a shipping prerequisite, and it is not built.
