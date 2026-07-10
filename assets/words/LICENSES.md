# Word-list sources & licenses

The built-in gameplay pools are compiled by `scripts/build-wordlists.py` from the
curated sources in `assets/words/{code}/{tier}.txt` into `src/word_data.rs`. This
file records where the data comes from and under what terms it ships.

## Current sources (v1)

| Data | Source | License | Notes |
|------|--------|---------|-------|
| Word banks (11 locales × 4 tiers) | **Original curation** for this app | Owned — ships freely | Hand-authored common vocabulary; tiers reflect human spelling-difficulty judgement, not raw frequency. |
| Keyboard layouts (`assets/keyboards/*.json`) | Original | Owned | Single source of truth for the runtime keyboard **and** the charset gate. |
| Exclusion roots (`assets/words/exclusions/_roots.txt`) | Seeded from `src/profanity.rs` | Owned | Extend per-language from LDNOOBW (see below). |

Because the v1 pools are original curation, there is **no third-party lexicon
license to satisfy for the shipped binary** — the App Store / MAS / MSIX
closed-binary concern in the work order (§3.1) does not apply to what ships today.

## Intended expansion sources (not yet ingested)

When the pools grow beyond hand curation, the pipeline is built to consume these.
Each must be recorded here with its license **before** any derived data is
committed, and per-language Hunspell licenses vary (GPL/LGPL/MPL/BSD) — substitute
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
- [ ] Native-speaker spot-check of nl / pl / sv / nb / tr pools (see
      `i18n/REVIEW_NEEDED.md`) — flag archaic/offensive/bizarre entries.
