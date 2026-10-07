# CC-BANK-PURITY §0 census — IN PROGRESS (C1 HALT open)

Run 2026-10-07 on `origin/main` da040556. Tooling: `tools/bank/purity_check.py` (F1, read-only),
`tools/bank/census_c6.py`. Row-level output: `reports/bank-purity-rows.csv`. No bank row, registry entry or app code changed.

| Item | Status |
|---|---|
| C1 recompute | **HALT** (5 languages beyond 5%), see below |
| C2 dictionary registry | done, one source weaker than the spec assumed |
| C3 consumers | done (provisional on C1) |
| C4 pool gates | not started (needs C1 resolved) |
| C5 Jr paths | done, report only |
| C6 profanity sheet | done |
| C7 residual proper nouns | done for ko ja ar hi sw ru (the languages with no proper-noun signal); report only |
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
| **fil** | 858 | 755 | **+13.6%** | HALT (was +31% before a bug fix: words that are also surnames were being dropped from the headword set) |
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
- **fil +103.** Mine uses the Wiktionary extract plus a frequency test for English. Kaikki is richer than the evidence's scrape, so unconfirmed falls (556 vs 1,221) while `wrong_language_english` is 587 (evidence ~604 incl. names). Net removal is higher mainly through `proper_noun` (200), from capitalised English-dictionary hits.
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

## C3 — consumers of a bank word  (`tools/bank/census_c3.py`, `reports/bank-purity-c3.csv`; provisional until C1 is settled)

Counts of entries that reference a row the current run would quarantine (Q), respell (R) or re-capitalise (D).

| Consumer | Entries | Q | R | D | Evidence run |
|---|---|---|---|---|---|
| definition pools `backend/def_pools/*.json` | 40,917 | 534 | 30 | 55 | 530 |
| gloss `config/gloss/*.json` | 3,796 | 15 | 0 | **109** | 202 |
| practice curricula `config/practice/*.json` | 406 | 1 | 0 | 0 | 8 |
| audio verdicts `config/audio-verdicts.json` | 3,465 | 53 | 1 | 14 | not listed |
| `src/ru_stress_data.rs` | 973 | 1 | 0 | 0 | not listed |
| human-audio manifests (en only) | 3,171 | 0 | 0 | 0 | not listed |
| zh sandhi / polyphone / tone-probe configs | 132 | 0 | 0 | 0 | not listed |

- The practice hit is Arabic `والتي` (attached conjunction), a real curriculum word. Fil `isda` was a false hit from a checker bug, now fixed.
- **German capitalisation is the big one (D):** 109 gloss keys and 55 definition-pool rows are keyed by the lowercase noun and must be re-keyed, not removed. Gloss keys are "the bank's EXACT stored form".
- **Word IDs.** `word_id` is FNV-1a-64 of the NFC word (`src/wordid.rs`), so every respelled (109) or re-capitalised (1,824 de) word gets a new ID, and every changed tier gets a new `TIER_HASHES` entry. A Spell Racing ghost or saved deck that stores an old ID resolves to nothing, and the loader aborts rather than substituting. Those stores live on players' devices (`spell_decks_v1`, ghosts, the Daily pool hash), so I cannot count them from the repo. Racing is hidden today, which limits the exposure, but saved decks and the Daily are not.
- No unknown consumer found in the repo that would break. HALT condition for C3 not met.

## C5 — Spell Jr paths  (report only; nothing changed)

Every mode with a Jr policy in `config/modes.json`, and whether its words pass the kid exclusion list (`kid_filter`).

| Mode | Passes `kid_filter`? | Evidence |
|---|---|---|
| standard / climb (Jr) | yes | `game.rs:124` via `active_word_list` |
| daily (Jr) | yes | `daily.rs:235` |
| bee_sim | yes | `bee.rs:145` |
| word_chains | yes | `chains.rs:406` |
| letter_forge | yes | `forge.rs:66` (`kid_only`) |
| impostor | yes | `impostor.rs:544` |
| word_picture | yes | `wordpic.rs:417` |
| translate | yes | `translate_screen.rs:97` |
| **practice** | **no** | fixed curriculum `config/practice/*.json`; **en includes `die` and `died`**, both on the en kid list |
| **def_match** | **no** | uses a `kid_register` flag in the definition pools, not the list; **en `dead die kill dies`, es `muerte sangre cuchillo`, fr `mort sang vin couteau`, de `wein Messer`, pt `morte sangue vinho cerveja` are flagged kid-register and sit on the kid list** |
| **spelldoku, spell_search, spell_cross** | **no** | only the global profanity check (`profanity::is_blocked`); no kid list |
| **void-round rescue** (`game.rs:1479`) | **no** | draws a replacement from the raw tier |
| my_words, misses, reports, calendar | n/a | the family's own words; my_words is parent-curated |
| ghost_racing, versus, spell_aloud, word_stories, syllable_replay, online_spelloff | hidden for Jr | not served |

Under-exclusion today, by language, is bounded by the lists: en 14 and es/fr/de/pt 4 each of the Easy+Medium rows are on a kid list. pl, ko, ja, fil and zh have empty kid lists, so nothing is excluded for them anywhere. This is CC-ONBOARD-JR and each mode's own file; Eric decides who fixes it.

## C7 — residual proper nouns (Wiktionary part-of-speech filter)  (`tools/bank/census_c7.py`, `reports/bank-purity-c7.csv`)

Report only; F3 applies nothing until Eric reads this. Extracts from kaikki.org (CC BY-SA), cached in `.corpus-cache/wikt/`.
Run for the six languages where the dictionary gives no proper-noun signal. es, fr, de, pt and pl already classify names through capitalised dictionary entries (C1); en and zh were skipped on size (en has 7 rows; zh has no proper-noun class).
A row counts when every Wiktionary entry yielding it (headword or form) has part of speech `name`, and C1 has not already quarantined it.

| Lang | Rows | Name-only in Wiktionary | Would add to quarantine |
|---|---|---|---|
| ko | 6,233 | 147 | 92 |
| ja | 6,160 | 52 | 48 |
| ar | 6,238 | 171 | 171 |
| hi | 2,674 | 67 | 67 |
| sw | 2,845 | 45 | 15 |
| ru | 6,208 | 92 | 24 |

Full lists in the CSV. Samples (every nth row, up to 50 each, shown here shortened):
- ko: 오스트리아 캘리포니아 스코틀랜드 아르헨티나 크리스마스 인도네시아 아프가니스탄 우크라이나 유고슬라비아 크로아티아 제주 울산 춘천 몽골 이슬람 히틀러 독도 광화문 보스턴 가톨릭
- ja: しまね とうしば とくしま くまがや けんたろう こいずみ さいとう だいすけ しょうた にしむら やまざき **たこ はやぶさ みなみ もも まゆ**
- ar: الأرض يناير أبريل الكتاب أوروبا فرنسا روسيا باريس المغرب إسرائيل نيويورك السعودية إبراهيم الاحتلال البحرين البرازيل اليونان سويسرا جنيف
- hi: **सूरज** अमेरिका पाकिस्तान इंग्लैंड अफ्रीका ब्रिटेन राजस्थान जर्मनी मुहम्मद इस्लाम श्रीलंका गुजरात कश्मीर अर्जुन हिमालय कोलकाता
- sw: kenya ulaya julai china kongo marekani tanzania januari zanzibar jumapili moroko nehemia uropa uswidi brazili
- ru: нато цска уильямс шевченко лукашенко путину марио пермь джеффри рогозин кира мэтт юнеско евровидение роскомнадзор шекспира алматы

**Precision caveat (bold above).** Most hits are real countries, cities, months and given names. But the filter also flags common words whose only Wiktionary entry is a name: hi `सूरज` (sun), ja `たこ` (octopus), `はやぶさ` (falcon), `もも` (peach), `みなみ` (south), `まゆ`, ar `الأرض` (the earth), `الكتاب`, `الأمم`. Japanese kana rows are matched by reading, so a surname reading collides with a common noun. This list needs a native pass before it is applied; it should not be applied mechanically. Months and weekdays (sw `julai januari jumapili`, ar `يناير أبريل الاثنين`) are proper nouns in some languages but good game words in others, which is a policy question for Eric, not a data one.

## Open for Eric

1. C1 HALT: save the evidence CSV to `reports/bank-check-2026-10-07.csv` so the five differences can be settled row by row, or tell me which side to follow per language.
2. Filipino second source (see C2).
3. C7 policy: are months and weekdays (and country names that are also common words) out of the banks? The filter cannot decide that.
4. Who owns the C5 gaps (practice, def_match, the three grid modes, the void-round rescue).
