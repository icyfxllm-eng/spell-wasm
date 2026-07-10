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
| CC-CEDICT | CC BY-SA 4.0 | Yes | data files | _TODO verbatim_ |
| kaikki.org / Wiktionary | CC BY-SA | Yes | data files | _TODO verbatim_ |
| CMUdict | BSD-style | Notice only | — | _TODO verbatim_ |
| WordNet | Princeton | Notice only | — | _TODO verbatim_ |
| wordfreq | verify | _TODO_ | _TODO_ | _TODO_ |
| Leipzig Corpora | research | verify redistribution | _TODO_ | _TODO_ |
| NECTEC Lexitron (th) | registration | **ASK ERIC — do not ingest without approval** | — | blocked |

## In-app credits

CC-BY-SA sources require **visible attribution**. When derived data ships, add a
Settings → About → **Data Sources / Licenses** screen listing each source's
required attribution string. (Not built yet — follow-up once a source is
actually ingested and Eric approves the posture.)
