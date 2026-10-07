# CC-BANK-PURITY §0 census — IN PROGRESS (C1 HALT open)

Run 2026-10-07 on `origin/main` da040556. Tooling: `tools/bank/purity_check.py` (F1, read-only),
`tools/bank/census_c6.py`. Row-level output: `reports/bank-purity-rows.csv`. No bank row, registry entry or app code changed.

| Item | Status |
|---|---|
| C1 recompute | **HALT** (5 languages beyond 5%), see below |
| C2 dictionary registry | done, one source weaker than the spec assumed |
| C3 consumers | not started |
| C4 pool gates | not started (needs C1 resolved) |
| C5 Jr paths | not started |
| C6 profanity sheet | done |
| C7 residual proper nouns | not started |
| C8 completeness conflict | not started |

## C1 — recompute vs the evidence run

The row-level evidence CSV exists only in chat. The comparison below is therefore **by count against the spec table**
(rows removed = rows − expected rows after Phase A). A set-level comparison needs the CSV saved at
`reports/bank-check-2026-10-07.csv`.

"Removed" = quarantine-class rows + P11 duplicates (misspelled rows whose corrected form is already a row) + hi unconfirmed (P8).

| Lang | Removed now | Expected | Diff | |
|---|---|---|---|---|
| en | 7 | 7 | 0.0% | |
| es | 466 | 477 | −2.3% | |
| fr | 565 | 570 | −0.9% | |
| **de** | 282 | 406 | **−30.5%** | HALT |
| pt | 585 | 591 | −1.0% | |
| pl | 282 | 293 | −3.8% | |
| ko | 3,337 | 3,337 | 0.0% | |
| **ja** | 1,093 | 679 | **+61.0%** | HALT |
| **fil** | 991 | 755 | **+31.3%** | HALT |
| **zh** | 0 | 1 | n/a | 3 traditional rows found; evidence has 2 (`藉由`, `藉口`) |
| ru | 208 | 216 | −3.7% | |
| **ar** | 735 | 689 | **+6.7%** | HALT |
| hi | 579 | 579 | 0.0% | |
| sw | 122 | 119 | +2.5% | |

Agreement elsewhere is close: 72 P11 duplicates (spec 69), 109 corrected in place (spec 106), German nouns to re-capitalise 1,824 (spec 1,721).
Bank sizes differ from the spec table only by a few rows (es +2, de +4, pl +1, zh +0, en/ja/fil/ko/ar/hi/ru/sw/fr/pt equal).

### Why the five differ (both lists not picked; Eric decides)

- **de −124.** The evidence removed about 124 more rows than the German dictionary check does. Likely first names and brand names the dictionary accepts capitalised (`carlos`, `petra`, `disney`); the evidence lists these as `noun_shown_lowercase`, which would not remove them, so the evidence's own removal count does not follow from its own classes. Cannot be settled without the evidence CSV row set.
- **ja +414.** Mine uses JMdict readings and finds 560 `loanword_in_hiragana` rows (evidence: roughly 100 visible, from a weaker SKK dictionary). The spec itself says Japanese should move to JMdict (C2). Proper nouns (`あふがにすたん`, `なぽれおん`) are not detectable from JMdict, so those stay `not_in_reference_dictionary` (746 now, spec 318).
- **fil +236.** Mine uses the Wiktionary extract plus a frequency test for English. Kaikki is richer than the evidence's scrape, so unconfirmed falls (577 vs 1,221) while `wrong_language_english` is 589 (evidence ~604 incl. names). Net removal is higher because my proper-noun count (331) comes from capitalised English-dictionary hits.
- **ar +46.** The prefix rule: single-letter prefixes count when the remainder is more frequent in Wikipedia; multi-letter ones (`بال لل وال كال`) when the remainder is a dictionary word. Evidence flagged `بذرة` (seed), a real word; the frequency test decides it the other way in some cases. Needs the Arabic auditor.
- **zh.** Unihan alone over-flags `著 蒙 覆`; CC-CEDICT word-level is used instead. It finds `有著 接著 牠` and not `藉口 藉由`. Two different judgments of orthography, not a data gap; native check advised.

## C2 — dictionary registry

All Hunspell files from LibreOffice/dictionaries commit `32b006a2c22a4ac7e8ed3f03346f7b3d85a970a4`, cached in `.corpus-cache/dicts/` (gitignored, never shipped, I7).

| Lang | File | Licence stated in the package | sha256 (first 16) |
|---|---|---|---|
| en | en_US.dic | SCOWL (permissive, BSD-like) | f0b1a234bd178bdd |
| es | es_ES.dic | GPL-3+ / LGPL-3+ / MPL-1.1+ | 6975dddec3d5d2c6 |
| fr | fr.dic | MPL 2.0 | b78a868e31dd6e37 |
| de | de_DE_frami.dic | GPL 2 or 3 | 4ca3c958b0e55459 |
| pt | pt_BR.dic | LGPL-3 / MPL | a38bfb26b68ece28 |
| pt-PT (variety only) | pt_PT.dic | GPL 2+ | e29ba2d7aa8a2ad4 |
| pl | pl_PL.dic | GPL2 / LGPL2.1 / MPL1.1 / Apache2 / CC BY 4.0 | c0848440599eb88e |
| ru | ru_RU.dic | BSD-style | f6047416a0204adb |
| sw | sw_TZ.dic | LGPL 2.1+ | e17d7c89fc547919 |
| ko | ko_KR.dic | GPL-3.0 only | 14117a72811ed6a0 |
| ar | ar.dic | GPL2 / LGPL2.1 / MPL1.1 tri-licence | 2a3e5367f61c1583 |
| hi | hi_IN.dic | GPL | 1e01f962a02638ef |
| ja | JMdict_e.gz (EDRDG) | CC BY-SA 4.0 | a84c9598 |
| zh | cedict.txt.gz, Unihan.zip | CC BY-SA 4.0; Unicode licence | 00e6c188; 4c93ea9c |
| fil | kaikki.org Tagalog extract (Wiktionary) | CC BY-SA | 81685696 |

Findings:
- No licence forbids offline build-time filtering. No C2 HALT.
- Korean `.dic` stores **decomposed jamo** and conjugated stems; read as NFC stems it removes exactly 3,337 rows, matching the evidence.
- **Filipino is the weakest source.** Only one source is in hand. A Tagalog Hunspell dictionary (`hunspell-tl`, GPLv2+, 2005, Crúbadán crawl-derived) exists but I have no verified download URL; PanLex (CC0) hosts did not resolve from this machine. The two-sources-agree rule for fil is not yet possible.
- Sw dictionary is small and old; expect it to keep over-reporting unconfirmed.

## C6 — profanity matches

`reports/bank-profanity-matches.csv`: **59 bank rows** match the repo's profanity lists (evidence: 59).
Per language: en 2, es 3, fr 7, de 5, pt 12, pl 4, ko 7, ja 4, fil 2, zh 13, ru/ar/hi/sw 0.

| Lang | Profanity list | Jr (kid-exclude) list |
|---|---|---|
| en es fr de pt | present | present (34, 9, 9, 9, 9 entries) |
| pl ko ja fil zh | present | **file exists, zero entries** |
| ru | present (80) | **none** |
| ar hi sw | **none** | **none** |

Note the spec says six empty and four missing Jr lists; the repo has five empty (`pl ko ja fil zh`; `vi` is out of scope) and four missing (`ru ar hi sw`).

## Open for Eric

1. C1 HALT: save the evidence CSV to `reports/bank-check-2026-10-07.csv` so the five differences can be settled row by row, or tell me which side to follow per language.
2. Filipino second source (see C2).
