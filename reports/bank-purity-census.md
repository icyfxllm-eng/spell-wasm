# CC-BANK-PURITY §0 census — IN PROGRESS (C1 HALT open)

Run 2026-10-07 on `origin/main` da040556. Tooling: `tools/bank/purity_check.py` (F1, read-only),
`tools/bank/census_c6.py`. Row-level output: `reports/bank-purity-rows.csv`. No bank row, registry entry or app code changed.

| Item | Status |
|---|---|
| C1 recompute | **HALT** (5 languages beyond 5%), see below |
| C2 dictionary registry | done, one source weaker than the spec assumed |
| C3 consumers | done (provisional on C1) |
| C4 pool gates | provisional: size gate and Definition Match done; Rust gates in the table below |
| C5 Jr paths | done, report only |
| C6 profanity sheet | done |
| C7 residual proper nouns | done for ko ja ar hi sw ru (the languages with no proper-noun signal); report only |
| C8 completeness conflict | done, provisional on C1; no `launched: true` language, so no HALT |

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

## C4 — pool gates after the sweep  (PROVISIONAL on C1)

Method: `tools/bank/dryrun_sweep.py` applied the current Phase A rules to a **scratch worktree** (`spell-wasm-purity-dry`, never merged): quarantine classes, P11 duplicates and hi unconfirmed (P8) removed; misspelled rows corrected; German nouns capitalised. Then the real gates ran against that copy. No threshold, floor or test was edited (I6); the scratch tree's diff is bank data and the regenerated `src/word_data.rs` only.

Rows per tier, before to after:

| Lang | Easy | Medium | Hard | Expert |
|---|---|---|---|---|
| en | 767 to 763 | 800 to 798 | 800 to 800 | 798 to 797 |
| es | 268 to 268 | 877 to 857 | 1,974 to 1,878 | 2,980 to 2,630 |
| fr | 279 to 279 | 889 to 862 | 1,967 to 1,846 | 2,974 to 2,556 |
| de | 272 to 272 | 954 to 950 | 1,967 to 1,897 | 2,955 to 2,746 |
| pt | 272 to 272 | 907 to 887 | 1,965 to 1,799 | 2,966 to 2,567 |
| pl | 267 to 267 | 961 to 949 | 1,969 to 1,876 | 2,979 to 2,802 |
| ko | 292 to 291 | 970 to **217** | 1,979 to 933 | 2,992 to 1,455 |
| ja | 253 to **209** | 967 to 947 | 1,970 to 1,531 | 2,970 to 2,380 |
| fil | 247 to 247 | 701 to 579 | 1,159 to 927 | 1,976 to 1,472 |
| zh | 296 to 296 | 944 to 944 | 1,958 to 1,958 | 2,984 to 2,984 |
| ru | 267 to 267 | 970 to 949 | 1,977 to 1,900 | 2,994 to 2,884 |
| ar | 290 to 288 | 968 to 898 | 1,987 to 1,806 | 2,993 to 2,511 |
| hi | 293 to 292 | 781 to 776 | 800 to 740 | 800 to **287** |
| sw | 262 to 262 | 703 to 673 | 1,180 to 1,105 | 700 to 683 |

English totals 3,158, exactly the spec's expected figure. Korean Medium is 217, the case the spec predicted.

| Gate | Result on the swept bank |
|---|---|
| `build-wordlists.py` size gate (floor 30, ceiling from `bank_floors.json`) | **green**, 73,140 words (was 82,261) |
| Definition Match `POOL_FLOOR` = 40, adult and Jr view, definitions re-keyed to corrected spellings | **green**; lowest tiers are hi Expert 50, ja Easy 58, zh Easy 73, de Easy 145; no tier crosses 40 |
| Letter Forge `every_ready_language_generates_a_full_year` and Jr twin, wordgrid `pools_meet_e4`, SpellDoku Word Mode fill, `every_tier_ladder_climbs`, Daily arc fill | RUST_RESULT_PENDING |

Notes:
- Only hi Expert (287) and ko Medium (217) fall low enough to matter to ladder-style gates, and neither crosses the Definition Match floor. The `bank_floors.json` pool floors (for example 500/1000/2000/3000) are above many post-sweep tiers, but every language is `launched: false`, so F4's red-build rule does not apply yet. That is a CC-BANK-COMPLETE concern, not a gate failure today.
- zh traditional corrections were applied to `assets/words/zh` only. The zh bank's source of record for `practice-check` is `src/words.rs`, which I did not edit in the scratch tree.
- A swept bank changes `TIER_HASHES` for every changed tier (see C3), so any pinned hash test needs the regenerated table; that is expected output of the Phase A regeneration step, not a gate failure.

## C8 — U-engine conflict  (PROVISIONAL on C1; `tools/bank/census_c8.py`, `reports/bank-purity-c8.csv`)

No language in `config/bank_floors.json` has `launched: true`, so the HALT does not fire and the red-build rule is dormant today.

U is rebuilt with the engine's first three gates (rank at or below the T4 floor, orthographic, profanity) from the Leipzig lists the banks were built from. Gates 4 and 5 are Pending in the engine, so this is the provisional set.

| Lang | T4 floor | U size (100K corpus ceiling) | Quarantine rows | In U |
|---|---|---|---|---|
| en | 30,000 | 16,734 | 7 | 2 |
| es | 30,000 | 19,054 | 437 | 8 |
| fr | 30,000 | 21,392 | 550 | 10 |
| de | 25,000 | 8,227 | 282 | 1 |
| pt | 25,000 | 16,027 | 580 | 17 |
| pl | 25,000 | 18,452 | 259 | 0 |
| ko | 15,000 | 14,242 | 3,337 | **771** |
| ja | 18,000 | 16,885 | 1,093 | 67 |
| fil | 8,000 | 5,297 | 858 | 158 |
| ru | 25,000 | 19,222 | 208 | 0 |
| ar | 15,000 | 13,913 | 735 | **526** |
| hi | 10,000 | 9,306 | 0 | 0 |
| sw | 10,000 | 6,152 | 122 | 8 |
| **Total** | | | | **1,568** |

Thirty examples (random draw, seed 1): ar `بالقيام بوقف فهذا لخلق وآخر والاهتمام والجماعات والحياة والعراق والمتوسطة والمملكة ولها`; es `ranking`; fil `debut district million species status`; ko `구성하였다 국가에서는 들어올 요구하였다 이탈리아와 일반적이다 일방적으로 조지 카운티 클라우드 통합되었다 프로듀서이다`.

Reading: U is built from a frequency list with no "word of the language" gate, so once a language is marked `launched`, F4's red-build rule would require back the 1,568 rows above, almost all Korean particle and ending forms and Arabic attached-prefix forms. Both are frequent in text and are exactly what P4 and P7 remove. This is the P16 question: add a purity gate directly after the rank gate. Recommendation unchanged: yes. It needs a signature because gate order is asserted by tests.
Two engine facts worth knowing: the orthography gate drops every capitalised token, so German nouns (all capitalised in corpus text) never enter U, which is why de has only 8,227 even before this file; and U here is capped by the 100K-line Leipzig corpus, so real U for languages whose T4 floor exceeds the corpus (es, fr, en) is bounded by corpus size, not by the floor.

## Open for Eric

1. C1 HALT: save the evidence CSV to `reports/bank-check-2026-10-07.csv` so the five differences can be settled row by row, or tell me which side to follow per language.
2. Filipino second source (see C2).
3. C7 policy: are months and weekdays (and country names that are also common words) out of the banks? The filter cannot decide that.
4. Who owns the C5 gaps (practice, def_match, the three grid modes, the void-round rescue).
