# CC-HUMAN-AUDIO Phase A: Lingua Libre coverage census

Run 2026-09-18 on branch `cc-human-audio`, on Eric's Mac (the Commons API is a
network call, so this is a tool run, not CI). **Report only. Phases B–D stay
blocked until Eric reviews this.**

- Bank: `cargo test --lib human_audio_census -- --ignored` dumps all 15 languages
  × 4 tiers (86,872 entries) through `words::tier_for`, the accessor the game
  serves from.
- Commons side: `tools/human-audio-census/census.py` (fetch + score) and
  `render.py` (tables). Fetched data is cached outside the repo
  (`~/repos/ha-census-cache`); a rerun resumes from the cache.
- Every number below is a **ceiling**: usable means "passes every filter the
  census can check". Auditor verification (F3) can only lower it.

## Naming convention: verified, with two traps

Real titles are `LL-Q<wikidata-id> (<iso-639-3>)-<uploader>-<word>.wav`, e.g.
`LL-Q7737 (rus)-1Apollinariya1-молоко.wav`. Two traps the spec's shape hides:

1. **Hyphens are ambiguous.** Usernames contain hyphens
   (`Jérémy-Günther-Heinz Jähnick-capable.wav`) and so do titles that are not
   the bare word (`…-Arlo Barnes-it-it.wav`, `…-Vealhurl--able.wav`, the suffix
   *-able*). Matching the longest bank word as a suffix counted those as
   recordings of *it* and *able*. The census now binds each title to its
   uploader (read from Commons): a clip counts only when the title is exactly
   `<uploader>-<word>`.
2. **47 English files use em-dashes** (`LL-Q1860—User—word.wav`). Skipped.

Commons, not Lingua Libre's own database, is the source of truth for what
exists: it holds 34,359 Russian files where Lingua Libre's query service knows
16,604. All 17 category counts matched Commons exactly.

## Rulings signed during the run (Eric, 2026-09-18)

- **Tagalog counts as Filipino.** Lingua Libre files Filipino almost entirely
  under `tgl` (2,034 files; 7 under `fil`).
- **Swahili is sw-TZ**, per the voice table. The speech-to-text map said sw-KE;
  that is fixed (PR #2) and already live on the Mac mini.
- **Arabic stays MSA (ar-XA), and auditors may accept Lingua Libre Arabic clips
  as MSA.** No speaker's place of learning can declare MSA, so for Arabic the
  variety check moves from speaker metadata to F3: a native Arabic speaker's
  clip is a candidate, accepted only as a correct MSA citation form. **This
  amends I8 for Arabic.** Arabic's usable figure is therefore an upper bound on
  what auditors will accept, with an unknown dialect-rejection rate.
- **Arabic speech-to-text stays ar-SA.** Google's v1 recognizer (what the
  backend calls) has no MSA code; ar-XA exists only on v2 Chirp 3. Changing the
  map to ar-XA would break the Arabic mic. Do not "fix" it.
- **D13. The pilot's clips ship bundled in the app, not from a server.**
  This amends F8, which said clips are "served from SpellGame's own backend".
  For the pilot language, the verified clips are built into the app binary and
  hash-checked at build time, and the resolver plays the bundled clip before TTS.
  Nothing is fetched at runtime, so I9 holds without a hosting dependency.
  Consequences:
  - **App size:** about +8 MB for an English pilot, +15 MB for French (≈3–4 KB
    per compressed one-second clip).
  - **Acceptance test 12** becomes "the bundled clips play identically offline,
    hash-verified at build". Offline packs (CC-OFFLINE-PACKS) are the route for
    later languages, when bundling every language (~300 MB) stops fitting.
  - **D10 parity:** spellgame.net serves the same files as static assets, so
    web and app still resolve the same clip.
  - **Licenses:** a bundled BY-SA clip is distributed inside the binary, so D1's
    BY-SA legal check lands before the first build that contains one. That
    applies to a French pilot (mostly BY-SA) and hardly at all to an English one
    (62% of the bank is covered by CC0 clips).

- **R1 → I8 stays strict** (Eric, 2026-09-18). Variety comes from the speaker's
  declared place of learning only; a declared residence does not stand in. The
  "usable if residence counts" column is kept for the record, not for use.
- **R3 → English is the pilot** (Eric, 2026-09-18). This is Eric's pick over
  D6's coverage rule, which would have named French (64.7% vs 62.4%). English
  is one dominant US speaker and nearly all CC0, so with D13 the pilot ships no
  BY-SA clip and needs no D1 legal check.

### What the English pilot needs before any tier switches on

Under strict I8, **no English tier reaches D3's 80% even as a ceiling**, so the
pilot cannot enable a tier from Lingua Libre alone. Gig C recordings (F4) fill
the rest. The counts are the *minimum*, assuming every usable clip passes F3;
each auditor rejection adds one.

| Tier | Entries | Usable (I8) | Short of 80% | Held out by D5 (Gig C only) | Whole gap |
|---|---:|---:|---:|---:|---:|
| easy | 768 | 603 (78.5%) | 12 | 70 | 165 |
| medium | 801 | 617 (77.0%) | 24 | 38 | 184 |
| hard | 801 | 459 (57.3%) | 182 | 51 | 342 |
| expert | 800 | 299 (37.4%) | 341 | 21 | 501 |

Easy and Medium are a few dozen Gig C recordings from the gate, so they are
the natural first pilot tiers. Hard and Expert need hundreds.

## Filters applied, in order

1. **Exact match**: NFC, case-sensitive, title word = bank entry (zh: hanzi half
   of `pinyin|hanzi`). Exact match also satisfies I3's inflection rule: a lemma
   title can only match a bank entry spelled identically.
2. **D5 held out** before any clip is considered:
   ru 280 stress homographs (kaikki-ru, headwords *and* inflected forms: берега,
   бегом, вести); zh 225 polyphones (CC-CEDICT, ≥2 readings); ja 1,036 kana
   entries with ≥2 common JMdict senses; en 180 heteronyms (CMUdict prons that
   differ in stress position or stressed vowel: record, present, read, wind).
   The English rule also catches accent splits (cost, drop), so English is a
   floor. Other languages have no local homograph source and are unmeasured.
3. **License** read from each file's Commons metadata (D1). Only 1 matched file
   of 55,454 failed the allowlist.
4. **Speaker** (Lingua Libre item via the Artist field) must declare the
   language at level *native*.
5. **Variety (I8)**: the speaker's declared *place of learning* for that
   language must resolve (Wikidata) to the shipping country. Undeclared =
   excluded.

## Summary per language

| Lang | Variety | LL categories | Commons files | Bank | Raw | **Usable (I8)** | Usable if residence counts | Gap | D5 held out | Usable speakers | Top speaker share |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| fr | fr-FR | fra | 438,295 | 6,109 | 81.9% | **64.7%** | 77.4% | 2,158 | — | 64 | 26% (Sartus85) |
| en | en-US | eng | 108,989 | 3,170 | 80.9% | **62.4%** | 66.6% | 1,192 | 180 | 17 | 73% (Grendelkhan) |
| pl | pl-PL | pol | 97,544 | 6,175 | 16.6% | **11.3%** | 15.8% | 5,478 | — | 2 | 98% (Olaf) |
| ar | ar-XA (MSA) | ara | 13,753 | 6,238 | 10.2% | **8.8%** | 8.8% | 5,687 | — | 9 | 42% (Zinou2go) |
| hi | hi-IN | hin | 3,431 | 2,674 | 12.9% | **4.6%** | 8.8% | 2,551 | — | 3 | 83% (AryamanA) |
| fil | fil-PH | fil, tgl | 2,041 | 4,083 | 8.2% | **3.0%** | 3.0% | 3,962 | — | 3 | 84% (Kunokuno) |
| pt | pt-BR | por | 9,587 | 6,112 | 31.2% | **1.0%** | 2.6% | 6,049 | — | 2 | 89% (Ed Guimaraes) |
| es | es-ES | spa | 19,127 | 6,097 | 42.4% | **0.4%** | 1.5% | 6,075 | — | 3 | 91% (Akhram) |
| ru | ru-RU | rus | 34,359 | 6,208 | 4.8% | **0.1%** | 3.5% | 6,203 | 280 | 3 | 40% (ZarinkaIM) |
| de | de-DE | deu | 26,112 | 6,144 | 12.1% | **0.0%** | 10.2% | 6,142 | — | 2 | 50% (Lucy Fermented) |
| zh | cmn-CN | cmn, zho | 4,126 | 6,182 | 21.6% | **0.0%** | 6.7% | 6,180 | 225 | 1 | 100% (KhawaChenpo) |
| vi | vi-VN | vie | 3,100 | 4,094 | 4.3% | **0.0%** | 4.3% | 4,093 | — | 1 | 100% (Khangtran2008) |
| ko | ko-KR | kor | 1,024 | 6,234 | 1.8% | **0.0%** | 1.8% | 6,234 | — | 0 | — |
| ja | ja-JP | jpn | 1,043 | 6,160 | 0.9% | **0.0%** | 0.0% | 6,160 | 1036 | 0 | — |
| sw | sw-TZ | swa | 102 | 2,845 | 0.7% | **0.0%** | 0.0% | 2,845 | — | 0 | — |

**Total Gig C workload under I8: 71,009 entries** (every gap row, all languages).
Gap lists, one `tier<TAB>entry` file per language, are in
`~/repos/ha-census-cache/gaps/` (not committed; regenerated by `render.py`).

## Per tier: raw / usable (I8) / usable if residence counts

D3 enables a tier at ≥80% usable *verified*. These are ceilings; ✓ marks ≥80%
under I8.

| Lang | easy | medium | hard | expert |
|---|---|---|---|---|
| fr | 100.0% / 99.3% ✓ / 100.0% | 95.3% / 82.6% ✓ / 92.0% | 88.4% / 69.8% / 83.5% | 72.0% / 52.7% / 66.8% |
| en | 94.0% / 78.5% / 80.5% | 90.4% / 77.0% / 79.5% | 77.5% / 57.3% / 61.5% | 62.1% / 37.4% / 45.2% |
| pl | 34.8% / 25.5% / 29.2% | 27.2% / 19.0% / 25.7% | 13.9% / 9.6% / 13.2% | 13.4% / 8.6% / 13.2% |
| ar | 41.4% / 32.8% / 32.8% | 23.5% / 21.7% / 21.7% | 8.7% / 7.6% / 7.6% | 4.0% / 3.1% / 3.1% |
| hi | 66.6% / 29.0% / 50.9% | 14.6% / 3.8% / 8.7% | 3.8% / 0.6% / 1.8% | 0.6% / 0.4% / 0.5% |
| fil | 59.1% / 14.6% / 14.6% | 12.8% / 7.6% / 7.6% | 4.7% / 1.8% / 1.8% | 2.3% / 0.6% / 0.6% |
| pt | 97.1% / 0.7% / 12.1% | 70.8% / 1.2% / 6.2% | 30.1% / 1.3% / 2.0% | 13.7% / 0.8% / 0.9% |
| es | 99.3% / 1.9% / 5.6% | 66.6% / 0.3% / 1.9% | 44.0% / 0.2% / 1.1% | 29.2% / 0.3% / 1.2% |
| ru | 34.8% / 1.5% / 24.7% | 5.6% / 0.1% / 4.6% | 2.8% / 0.0% / 2.1% | 3.3% / 0.0% / 2.1% |
| de | 48.5% / 0.0% / 40.4% | 23.3% / 0.0% / 20.3% | 11.3% / 0.1% / 9.9% | 5.6% / 0.0% / 4.4% |
| zh | 57.8% / 0.7% / 10.1% | 32.6% / 0.0% / 14.6% | 25.0% / 0.0% / 8.1% | 12.3% / 0.0% / 2.9% |
| vi | 38.6% / 0.0% / 38.6% | 6.2% / 0.1% / 6.2% | 1.2% / 0.0% / 1.2% | 1.1% / 0.0% / 1.0% |
| ko | 18.2% / 0.0% / 17.8% | 2.1% / 0.0% / 2.1% | 1.1% / 0.0% / 1.1% | 0.6% / 0.0% / 0.6% |
| ja | 5.5% / 0.0% / 0.4% | 1.0% / 0.0% / 0.0% | 0.4% / 0.0% / 0.0% | 0.7% / 0.0% / 0.0% |
| sw | 2.7% / 0.0% / 0.0% | 1.4% / 0.0% / 0.0% | 0.3% / 0.0% / 0.0% | 0.0% / 0.0% / 0.0% |

Only two (language, tier) pools clear 80% even as a ceiling: **fr Easy** and
**fr Medium**. English Easy (78.5%) is close; with residence counted it
crosses (80.5%).

## Voice consistency and license, for the leading languages

| Lang | Usable (I8) | Top speaker alone | Top 3 speakers | CC0 clips only |
|---|---:|---:|---:|---:|
| fr | 64.7% | 41.4% | 54.3% | 14.1% |
| en | 62.4% | 55.9% | 61.8% | 62.2% |
| pl | 11.3% | 11.0% | 11.3% | 10.9% |
| ar | 8.8% | 6.4% | 8.2% | 0.7% |
| hi | 4.6% | 4.5% | 4.6% | 0.9% |
| fil | 3.0% | 2.5% | 3.0% | 0.5% |

## License mix (every matched clip, before variety and D5 filters)

| Lang | CC0 | CC BY | CC BY-SA | Excluded |
|---|---:|---:|---:|---:|
| fr | 5,374 | 1,383 | 16,187 | 1 |
| en | 4,736 | 102 | 3,014 | 0 |
| pl | 993 | 3 | 78 | 0 |
| ar | 109 | 70 | 906 | 0 |
| hi | 289 | 12 | 218 | 0 |
| fil | 19 | 1 | 326 | 0 |
| pt | 570 | 4 | 2,553 | 0 |
| es | 553 | 491 | 2,450 | 0 |
| ru | 52 | 3 | 276 | 0 |
| de | 455 | 9 | 860 | 0 |
| zh | 412 | 1 | 1,074 | 0 |
| vi | 177 | 0 | 0 | 0 |
| ko | 106 | 0 | 6 | 0 |
| ja | 11 | 0 | 23 | 0 |
| sw | 20 | 0 | 0 | 0 |

## Why matched clips were excluded (clip counts)

| Lang | D5 | license | no level for language | speaker item unreadable | not native | variety mismatch | no place of learning | place has no country |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| fr | · | 1 | 65 | 657 | 239 | 852 | 11,213 | 31 |
| en | 791 | · | 25 | 557 | 669 | 332 | 3,805 | 43 |
| pl | · | · | 59 | 1 | · | · | 317 | · |
| ar | · | · | 47 | 70 | 25 | · | · | · |
| hi | · | · | 65 | · | 136 | · | 173 | 145 |
| fil | · | · | · | 210 | 14 | · | · | · |
| pt | · | · | 68 | 8 | 1,317 | 29 | 1,642 | · |
| es | · | · | 43 | 379 | 142 | 1,447 | 1,451 | 10 |
| ru | 14 | · | 3 | 7 | · | · | 316 | 5 |
| de | · | · | · | 16 | 319 | · | 987 | · |
| zh | 69 | · | · | 35 | 154 | 3 | 1,293 | · |
| vi | · | · | · | · | · | · | 176 | · |
| ko | · | · | · | · | 1 | · | 111 | · |
| ja | 26 | · | · | 16 | 17 | · | 1 | · |
| sw | · | · | · | · | · | · | · | 20 |

"Place has no country" is mostly answers that are not places: *school* (476
clips), *home* (266), *university* (225), *self* (164), *family* (152).

Where the variety-excluded speakers learned the language:

- **fr** (fr-FR): Switzerland 744, Canada 89, Ivory Coast 12, Belgium 7
- **en** (en-US): South Africa 176, Canada 69, Ireland 52, United Kingdom 17, India 12
- **es** (es-ES): Colombia 979, Uruguay 273, Mexico 186, Peru 4
- **pt** (pt-BR): Portugal 29
- **zh** (cmn-CN): Taiwan 3

## Findings

1. **Lingua Libre is a French-and-English source.** Only fr and en reach
   meaningful usable coverage. Every other language is overwhelmingly Gig C:
   pl 11%, ar ≤9%, and the rest under 5%.
2. **Missing variety data, not missing recordings, is the biggest filter.**
   Spanish has a recording for 99.3% of Easy but 1.9% usable; German 48.5%
   raw, 0.0% usable. Most speakers never declared *where they learned* the
   language (11,213 French clips alone). Many did declare where they *live*;
   that alternate reading is the third column above. It is not I8 as written.
3. **Russian gets no stress payoff from Lingua Libre.** Raw coverage is 4.8%:
   the bank is frequency-ranked surface forms, many inflected, and Lingua Libre
   Russian is mostly lemma lists. With I3's exact-surface rule, the stress
   benefit that motivated this spec arrives only through Gig C (6,203 entries).
4. **Mandarin, Japanese, Korean, Vietnamese, Swahili are Gig C only.** zh is
   21.6% raw but no speaker declares a mainland place of learning. ja
   titles are mostly kanji while the bank is kana, so only 0.9% match at all.
   Swahili has 102 files in total.
5. **Spanish's Lingua Libre speakers are Latin American** (Colombia, Uruguay,
   Mexico), against an es-ES voice. Even with residence counted, Spanish stays
   under 6% at every tier.
6. **English is the cleanest source.** One US speaker (Grendelkhan,
   Connecticut) covers 55.9% of the bank alone, and CC0 clips cover 62.2%, so
   English needs neither attribution nor the D1 BY-SA legal check.
7. **French has more coverage but less consistency.** Its 64.7% comes from 64
   speakers (top speaker 26%, 41.4% alone), and CC0 clips cover only 14.1%, so
   shipping French means the BY-SA legal check and a Credits screen from day one.
8. **Lingua Libre's wiki is archived.** Entity pages now redirect to
   `archive.lingualibre.org`, which refused connections during the run. The
   census reads speakers through Lingua Libre's SPARQL service instead. 10 of
   637 speakers are missing from that index, excluding 1,956 clips as
   "speaker item unreadable". F1's harvester will need the same route.

## D6: the pilot pick

D6's rule: highest usable coverage among languages with an active auditor,
tie-break toward Russian.

- **By coverage: French (64.7%), then English (62.4%).** Russian (0.1%) is not
  in contention, so the tie-break does not apply.
- **The repo records no active auditor for any language.** Gig A/B are specced
  (CC-MASTER-PARITY C1) but nothing logs who is engaged. D6 cannot be closed
  without Eric saying which languages have one.
- If both have auditors, the rule picks **French**, the only language with any
  tier at ≥80% (Easy, Medium). Findings 6 and 7 are the trade-off Eric may want
  to weigh: English is one voice and needs no BY-SA review.

## Open for Eric

- **R2. Is there an active English auditor?** R3 settled the pilot, so this
  now asks only about English. F3 verification and the second-auditor check on
  Gig C recordings (D7) both need one.
- **D11, D12** remain open, as in the spec. D12's inputs are the gap column.

## Not measured

Background suitability, recording quality, stress/tone correctness and actual
MSA-ness are auditor checks (F3); none can be read from metadata. Homographs
are unmeasured outside ru, zh, ja and en for want of a local source.
