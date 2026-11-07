# CC-ORTHO-SWEEP-ALL v1: §0 census

Run 2026-11-05 on `origin/main` bf2342c0 (worktree `spell-ortho-census`, branch `census/ortho-all`). Read-only over banks, grader and profiles.
Script: `tools/ortho_census.py`. Raw output: `reports/ortho-census/` (`summary.json`, `c3-collisions-<lang>.tsv`, `c7-<lang>.csv`).

**Status (updated 2026-11-05, after Eric's resolutions): Phase 1 cleanups applied on `census/ortho-all`; one stop-and-ask open (D7, zh monosyllables). The original census text below is kept as run; the section "Resolutions applied" at the end supersedes it where they differ.**

## Stop-and-ask conditions that fired

| # | Condition in the spec | What the census found |
|---|---|---|
| S1 | C2 voice region differs from D1 default | **es is `es-ES-Neural2-G` (Spain), not Latin American.** The auditor addendum tells the Spanish auditor "the voice is Latin American". It is wrong as written. The es sound-alike table is already `verified` for this Spain voice (7 true sets), and the `es_scz` rule in F-es depends on whether the voice is seseante (a Spain voice is not). |
| S2 | A single check flags more than 5% of rows | Citation form (heuristic, C4): es 35.9%, fr 29.7%, pt 38.3%, pl 27.2%, ru 40.4%. fil 9.0%, sw 10.5%, hi 6.5%, ko 5.4%. The banks are Leipzig Wikipedia corpus words and hold plurals, conjugations and case forms. PURITY's ledger has no inflection class for es/fr/pt/pl/ru/sw/fil, so nobody has removed them. D5 and I7 ("no inflected form in any bank") would quarantine thousands of rows per language, which is a bank-shrinking decision for Eric. |
| S3 | Collision count exceeds 1% of rows | zh 10.13% (626 rows in 254 groups). This is homophones by design under pinyin-key comparison (no sandhi applied here), so it is the F-zh homophone lane, not duplicates. It still breaks I1 as written for pinyin modality. Other languages are under 1% (ru 0.82%, vi 0.04%). |
| S4 | A zh check flags more than 5% | F-zh-1 monosyllables: 931 rows, 15.1%. D7 would drop them unless their toned-pinyin key is unique. |
| S5 | Anything here conflicts with a signed file, including EN-VARIANTS F2 | `src/norm.rs` `fold_apostrophes` (F2, 71d62995) says "No lineup language spends these marks as letters... the census checked". That is true only because the bank cannot hold them: the build filter (`scripts/build-wordlists.py:266`) drops every non-letter, the sw keyboard has no apostrophe key (`assets/keyboards/sw.json`), and so sw has **0 ng' words** (`ng'ombe`, `ng'ambo` are absent; the 9 `ng-` rows are ordinary ngV words such as ngoma). The fold itself is compatible with F-sw. The F2 premise is not. |
| S6 | Prerequisite check | None. De/ja variants mechanism, SENSE-CUE table API and BANK-PURITY ledger are all present. Listed only so the absence is explicit. |

**S1 resolved (Eric, 2026-11-05): keep the Spain voice (`es-ES-Neural2-G`).** Consequences for the spec: D1's es default becomes Spain Spanish; `es_scz` is an accent rule (a Spain voice distinguishes c/z from s), not a true-homophone rule; casa/caza and cocer/coser are therefore not same-sound pairs for this voice; the es addendum must say "The voice is from Spain", and Latin American auditors answer for the voice they hear. S2 to S5 remain open.

## C1: Locations

| Item | Path |
|---|---|
| Banks (11 languages) | `assets/words/<lang>/{easy,medium,hard,expert}.txt` (sw has no expert in the 3-tier sense but 683 rows; zh rows are `pinyin|hanzi`), compiled to `src/word_data.rs` by `scripts/build-wordlists.py` |
| Lexicon layer (build-time only, git-ignored) | `data/<lang>/lexicon.jsonl` (present only in the main checkout) |
| Answer normalization | `src/norm.rs` (`fold_strict`, `fold_lenient`, `fold_apostrophes`, `answer_matches_in`), `src/pinyin.rs` (zh `PinyinKey`), `src/hangul.rs`, `src/ja_canon.rs`, `src/fa_canon.rs` |
| De/ja variants mechanism | `src/accepted.rs`; `data/{de,ja,en}/accepted-variants.tsv`; `docs/CC-ORTHO-SWEEP-DE-JA-decisions.md` |
| SENSE-CUE API | `src/soundalikes.rs` (`soundalikes(lang, voice)`, `status_for_voice`); tables `assets/words/<lang>/soundalikes.json`; generator `tools/build_collisions.py`; spec `docs/CC-SENSE-CUE-v1.1.md`, `v1.2-mergers.md` |
| SENSE-CUE status of the eleven | verified: **es** (7 true sets, voice es-ES-Neural2-G), **ru** (29 true, 4 accent). missing, "no generator for this language yet": fr, pt, pl, vi, ko, fil, zh, sw, hi |
| VOICE-CLARITY per-language intake | **None for these eleven.** Only `data/de/near-sound-candidates.tsv` and `data/ja/near-sound-candidates.tsv` exist. No `near-sound-rules.json`, `sound-alike-rules.json` or `ortho-profile.json` exists anywhere. Per T7 the state is "no intake exists": report and continue. |
| EN-VARIANTS F2 | Landed, 71d62995 (Nov 4), "all languages", folds before NFC on both sides |
| BANK-PURITY | Ledger `assets/words-quarantine/manifest.csv` (9,655 rows) plus `assets/words/purity/<lang>.tsv` for all eleven. Quarantined rows per language: es 467, fr 573, pt 598, pl 315, ru 211, ko 3,337, fil 887, sw 122, hi 579, vi 0, zh 0. Classes: proper nouns, wrong-language, missing-diacritic; ko also `particle_attached` / `inflected_sentence_form`. The sweep looks merged. I did not confirm that with its owner. |

## C2: Rows, tiers, voices, zh modality

| Lang | Rows | easy / medium / hard / expert | Voice in use (backend/app.py `LANG_VOICES`) | D1 default | Match |
|---|---|---|---|---|---|
| es | 6,267 | 268 / 1000 / 2000 / 2999 | es-ES / es-ES-Neural2-G | Latin American | **NO (S1)** |
| fr | 6,272 | 279 / 1000 / 2000 / 2993 | fr-FR / fr-FR-Neural2-F | fr-FR | yes |
| pt | 6,259 | 272 / 1000 / 2000 / 2987 | pt-BR / pt-BR-Neural2-B | pt-BR | yes |
| pl | 6,211 | 267 / 949 / 2000 / 2995 | pl-PL / pl-PL-Wavenet-B | pl-PL | yes |
| ru | 6,830 | 833 / 1000 / 1999 / 2998 | ru-RU / ru-RU-Wavenet-A | ru-RU | yes |
| vi | 5,601 | 774 / 832 / 1497 / 2498 | vi-VN / vi-VN-Wavenet-A | Hanoi | not checked: Google's vi-VN voices are not labelled by region; the spec needs a Northern voice confirmed by ear |
| ko | 3,163 | 558 / 217 / 933 / 1455 | ko-KR / ko-KR-Wavenet-D | ko-KR | yes |
| fil | 4,035 | 247 / 600 / 1200 / 1988 | fil-PH / fil-PH-Wavenet-A | fil-PH | yes |
| zh | 6,182 | 296 / 944 / 1958 / 2984 | cmn-CN / cmn-CN-Wavenet-A (spoken text is the hanzi) | Putonghua | yes |
| sw | 2,818 | 262 / 673 / 1200 / 683 | sw-TZ / sw-TZ-RehemaNeural (Azure, not Google) | Tanzanian | yes |
| hi | 2,095 | 292 / 776 / 740 / 287 | hi-IN / hi-IN-Neural2-A | hi-IN | yes |

**zh answer modality.** The bank stores `pinyin|hanzi`; grading compares `PinyinKey` (`src/pinyin.rs`), which already accepts diacritics, tone digits, `v`/`u:` for ü and optional separators. So typed toned pinyin is the live modality, with drawn entry (Quick Bee Draw) alongside. I found no typed-hanzi grading path. This is read from `pinyin.rs` and `game.rs`, not from running the app. If that holds, the profile's character lane (OpenCC conversion, `variant_char`) has nothing to grade today and the spec's own stop rule applies in the opposite direction (typed pinyin exists, so the pinyin profile is the one that matters).

## C3: Collisions under the proposed `normalize` chain

OpenCC and pypinyin are not installed, so zh was run on the pinyin side only (`fold_pinyin` without sandhi), and `script_convert` was not run.

| Lang | Rows in collision | % | Notes |
|---|---|---|---|
| es, fr, pt, pl, ko, fil, sw, hi | 0 | 0 | |
| ru | 56 (28 pairs) | 0.82 | All are ё/е under `fold_yo`. Two kinds, see below. |
| vi | 2 (1 pair) | 0.04 | `được` and `đựơc` (a typo with ơ and ̣ transposed) |
| zh | 626 (254 groups) | 10.13 | Same toned-pinyin key, different hanzi. Group sizes: 187 pairs, 39 triples, up to 9. 2 groups also share hanzi. |

**ru, the finding to read.** `fold_yo` merges true minimal pairs that are different words, not just duplicates: все/всё, всем/всём, вел/вёл. Listed: боёв/боев, ведёт/ведет, все/всё, всем/всём, вел/вёл, даёт/дает, ещё/еще, and 21 more in `c3-collisions-ru.tsv`. Some are the same word stored twice (ещё/еще, даёт/дает, ведёт/ведет), which is a bank duplicate. Others are different words that the spec's own seed (`ещё/еще` correct) would make indistinguishable. D4 and I1 as written cannot both hold for ru.

## C4: Citation form (heuristic)

Method: a row is flagged when stripping a language-specific inflection suffix and adding a lemma ending lands on another row in the same bank (list in the script). This finds real inflections, and also false positives where two real headwords look related (see ko samples). It cannot find an inflected form whose lemma is absent. Treat counts as an upper bound on one side and a lower bound on the other. Nothing here is a verdict.

| Lang | Flagged | % | Samples (row ← row it looks derived from) |
|---|---|---|---|
| es | 2,249 | 35.9 | amigo←amiga, indica←indicar, malas←malo, mayoría←mayor*, usuarios←usuario, llevaron←lleva, errores←error |
| fr | 1,863 | 29.7 | portes←porte, sœurs←sœur, souvenirs←souvenir, troupes←troupe, sociaux←social, apporte←apporter |
| pt | 2,398 | 38.3 | viram←virar, decisões←decisão, entrega←entregar, passaram←passo |
| pl | 1,687 | 27.2 | filmu←film, edycji←edycja, rodzaju←rodzaj, systemem←system, obecnych←obecny |
| ru | 2,654 + 106 `-тся` | 40.4 | идеи←идея, начале←начал, музыку←музыка, компании←компания, выборах←выбор |
| vi | n/a | n/a | Isolating language, no inflection. 4,413 rows (79%) contain a space, which is the signed D1/D2 multi-syllable form. |
| ko | 71 + 99 pattern | 5.4 | many false positives (강한←강, 바로←바다, 최고←최다). Only 51 rows end in -다 because the bank is headword nouns plus adverbs; the PURITY ledger already removed 3,337 inflected/particle rows. |
| fil | 362 | 9.0 | mahigit←higit, malinaw←linaw, nagpunta←punta, katulong←tulong. The "reduplication" hits (babae, tatay) are false positives. |
| zh | n/a | n/a | rows are words from a frequency list |
| sw | 295 | 10.5 | Tensed-verb pattern; 400 rows are ku- infinitives. Samples include false positives (ukubwa, mtaalamu are nouns). |
| hi | 137 | 6.5 | गर्मी←गर्म, पुस्तकें←पुस्तक, राष्ट्रों←राष्ट्र. Only 45 rows end in -ना. |

(*mayoría←mayor is a false positive of the heuristic. Spot-checking 20 samples per language is the auditor's job, not mine.)

Per-row lists: `reports/ortho-census/c7-<lang>.csv`, check ids `F-<lang>-cit`.

## C5: Characters

| Lang | Outside allow-list | Apostrophe | Hyphen | Space | ZWJ/ZWNJ | Not NFC |
|---|---|---|---|---|---|---|
| es, fr, pt, pl, ru, ko, zh, sw, hi | 0 | 0 | 0 | 0 | 0 | 0 |
| vi | 0 | 0 | 0 | 4,413 | 0 | 0 |
| fil | 0 | 0 | 67 | 0 | 0 | 0 |

Allow-lists: language letters plus listed diacritics (fr includes œ/æ, pt includes ã õ, pl the nine marks, vi base letters plus đ; hi Devanagari; ru Cyrillic; ko Hangul; zh ASCII letters and digits for pinyin). Two consequences to flag:

* **fr and sw cannot hold their apostrophe or hyphen words.** The build filter drops non-letters and neither keyboard has the key. There are 0 hyphen or elision rows in fr, so `week-end`, `porte-monnaie`, `l'eau` cannot be in the bank at all, and the F-fr rect1990 variants for compounds have nothing to attach to. Hyphens exist only in fil (67 rows, and the fil keyboard has a hyphen key).
* **hi keeps no ZWJ/ZWNJ**, so the `strip_zwj_zwnj` step is for typed input only.

## C6: Sources and licences

I did not fetch licence terms at any source this run, and I did not infer any. What is recorded in the repo is quoted. Everything else is `unknown`.

| Source named in the spec | Verdict | Basis |
|---|---|---|
| Leipzig Wikipedia corpora (the banks today) | check-only until confirmed | `assets/words/LICENSES.md` records CC BY 4.0 "as recorded" and says it was not confirmed against wortschatz-leipzig.de on 2026-10-07; underlying Wikipedia text is CC BY-SA. Unresolved. |
| OpenCC (zh) | unknown | spec says Apache-2.0; not verified |
| CC-CEDICT (zh) | check-only | spec says CC BY-SA; repo `NOTICES.md` records its use for glosses (D-NOBYSA applies to variants and pairs, not glosses) |
| RAE Ortografía 2010; 1990 rectifications; AO90 text; Rada Języka Polskiego; Central Hindi Directorate; KWF Ortograpiyang Pambansa 2014; NIKL 복수 표준어 announcements; Vietnamese spelling regulations | unknown (rules, not data) | official rule texts. The spec treats them as rules only, so the licence question is the pairs file, not the rule. |
| Hunspell es/fr/pt/pl/ru etc. | unknown | `LICENSES.md` itself warns the licence varies per language (GPL/LGPL/MPL/BSD) |
| Lexique 3, OpenCorpora, 우리말샘 | check-only | spec says BY-SA |
| sjp.pl, 표준국어대사전, Academic orthographic dictionary (ИРЯ РАН), VOLP/VOP/VOCP, BAKITA, Kamusi, KWF Diksiyonaryo, IIT Bombay Hindi WordNet, 现代汉语常用词表, 通用规范汉字表, HSK 3.0, Unihan, jieba, pypinyin | unknown | not checked |

`data/<lang>/SOURCES.md` exist for es, fil, ko, vi, zh. I did not read them for this section; they should be read before any variants file is built. This is the one C item I am marking not done (reason: no network licence lookups were run).

## C7: F-checks (per-row CSVs in `reports/ortho-census/c7-<lang>.csv`)

| Check | Count | Notes |
|---|---|---|
| F-es-1 pre-2010 tilde form stored | 0 | |
| F-es-2 plurals/verb forms | 2,249 | = C4 |
| F-es-3 ¿ ¡ | 0 | |
| F-fr-1 conjugated/plural | 1,863 | = C4 |
| F-fr-2 elision | 0 | filter removes them |
| F-fr-3 1990 pair stored in one form | 8 | événement **and** évènement are both stored (two rows for one word, will break I2). Also île, maître, plaît, connaître stored in traditional form only. |
| F-pt-1 pre-AO90 | 3 | pára, dêem, sequência (the last is a false positive: sequência is AO90-correct) |
| F-pt-2 PT-PT watch list | 9 | my list is broad; real hits are facto, comboio, rapariga, secção, aspecto. aceitar, setembro, espetáculo, exceção are correct in BR and should be ignored. |
| F-pt-3 inflected | 2,398 | = C4 |
| F-pl-1 inflected | 1,687 | = C4 |
| F-pl-2 stripped form is also a row | 150 | Mostly case forms next to their lemma (droga/drogą, matka/matką), plus real pairs such as piec/pięć. The latter are the los/łoś class. |
| F-pl-3 casefold collisions | 0 | |
| F-ru-1 ё missing from stored form | could not run | no ё-headword dictionary in the repo (`ru_stress_data.rs` has 973 stress entries). 64 rows contain ё. |
| F-ru-2 inflected | 2,760 | = C4 |
| F-ru-3 Latin characters | 0 | |
| F-vi-1 not NFC | 0 | |
| F-vi-2 tone-placement duplicate | 2 | được / đựơc |
| F-vi-3 i and y both stored | 2 | hí / hý |
| F-ko-1 not NFC | 0 | |
| F-ko-2 spaces | 0 | |
| F-ko-3 inflected | 170 | = C4, heuristic |
| F-ko-4 Hanja/Latin | 0 | |
| ko dual-standard words in bank | 2 | 손자, 마을. The partners 손주, 마실 are absent. |
| F-fil-1 raw English residue | 156 | heuristic on spelling shape. Real hits unlikely among doon/noon/maging (Filipino); likely hits among aircon, kanser, anaphylaxis. Needs a read. |
| F-fil-2 tuldik marks | 0 | |
| F-fil-3 r-/contracted/legacy | 6 | babae, lalaki **and** lalake, kwento, pwede, nyan. lalake is the legacy form stored beside the KWF form. |
| F-zh-1 monosyllables | 931 | 15.1% |
| F-zh-2 wrong script | 0 | test uses a short Traditional-only character list; not a conversion check |
| F-zh-3 Latin/punct in hanzi | 0 | |
| F-zh-4 homophone set sizes | 5,556 singletons; 187 pairs; 39 triples; 17 four; 5 five; 3 six; singles up to nine. Groups touching each tier: easy 72, medium 94, hard 132, expert 179. Sandhi not applied. |
| F-sw-1 bare stems | 0 | |
| F-sw-2 U+2019 stored | 0 | |
| F-sw-3 English residue | 29 | spelling-shape heuristic; mostly Swahili loans (kioo, hadithi, thamani, thelathini). `anaphylaxis` is a real hit. |
| F-hi-1 ZWJ/ZWNJ | 0 | |
| F-hi-2 not NFC | 0 | |
| F-hi-3 Devanagari English loans | 0 | by the ॉ/ॅ marker only; weak test |
| F-hi-4 nukta/no-nukta stored as separate rows | 14 rows (7 pairs) | बर्फ/बर्फ़, अंग्रेजी/अंग्रेज़ी, डिजाइन/डिज़ाइन, फरवरी/फ़रवरी, अफगानिस्तान/अफ़ग़ानिस्तान, जबरदस्त/ज़बरदस्त, स्विट्जरलैंड/स्विट्ज़रलैंड |

## C8: Auditor reach

**No answer available.** No auditor email is on file in the repo (`audit/`, `cloudflared/`, `docs/`, `reports/`), and `~/spellgame-server` logs and Caddyfile hold no Access challenge record. The Cloudflare Access policy and its login log live in the Cloudflare dashboard, which I did not open. So C8 is "unknown" for every auditor, including Paul (fil). The spec's rule that fil is not done until his email shows a yes is therefore not met. Fixed nothing.

## C9: Cross-language summary

| Lang | Sound-alike table | Citation-form heuristic | Collisions | Biggest finding |
|---|---|---|---|---|
| es | verified (Spain voice) | 35.9% | 0 | Voice is Spain, not Latin American (S1) |
| fr | missing | 29.7% | 0 | évènement and événement both stored; no hyphen or elision words possible |
| pt | missing | 38.3% | 0 | 3 real pre-AO90/PT-PT forms; the rest of the watch list was noise |
| pl | missing | 27.2% | 0 | 150 rows whose stripped form is another row |
| ru | verified | 40.4% | 56 | `fold_yo` merges real minimal pairs (все/всё) and exposes 28 duplicate pairs |
| vi | missing | n/a | 2 | được/đựơc typo row; Northern voice unconfirmed |
| ko | missing | 5.4% (mostly false positives) | 0 | Already headword-mostly after PURITY; only 2 NIKL dual words present |
| fil | missing | 9.0% | 0 | Legacy lalake stored beside lalaki; 156 possible English rows |
| zh | missing | n/a | 626 (pinyin homophones) | 15% monosyllables; typed pinyin is the live modality |
| sw | missing | 10.5% | 0 | Bank and keyboard cannot hold ng'; F2 premise is only true because of that |
| hi | missing | 6.5% | 0 | 7 nukta/no-nukta pairs stored as separate rows |

Seven things the spec should settle before Phase 1:

1. Which Spanish voice is the product, and does the es addendum say "Spain" or does the voice change (S1)?
2. D5/I7: quarantine inflected rows in es/fr/pt/pl/ru at 27–40%, or relax I7? This changes pool sizes; `bank_floors.json` floors are already above several post-sweep tiers.
3. ru: how `fold_yo` treats real ё/е pairs (все/всё), and which of the 28 pairs are duplicates to quarantine.
4. sw and fr: do we add an apostrophe (and hyphen) key and let the build filter keep them, or leave ng' words out? Without that, the sw golden seeds `ng'ombe` have no row.
5. zh: pinyin-key collisions will never meet I1 (zero collisions). I1 needs a pinyin-modality carve-out, or the lane is hanzi-only.
6. D7 monosyllables: 931 zh rows are affected.
7. C6 and C8 need a human or network pass.

HALT. Phases 1 to 5 are not started.


---

# Resolutions applied (Eric, 2026-11-05)

Resolutions are in `docs/CC-ORTHO-SWEEP-ALL-resolutions.md`. Every moved row is in `reports/ortho-census/quarantine-batch1.tsv` and in `assets/words-quarantine/manifest.csv`; signed rows are in `tools/bank/purity_overrides.csv`. Rows are moved, never deleted. Batch script: `tools/ortho/sweep_all_batch1.py`.

## Open: D7 zh monosyllables. STOP, not applied

931 zh rows are single syllables. Applying the rule as written ("drop a monosyllable unless its toned key is unique in the bank"; the second list, the 现代汉语常用词表, is not in the repo and no new dictionaries are allowed) drops **505 rows (8.2% of zh)**. That is over your 500-row line and over the 2% quarantine line. Nothing was dropped. By tier: easy 76, medium 89, hard 147, expert 193. The other 426 monosyllables have a unique key and stay. Per-row list: `reports/ortho-census/c7-zh.csv`, check `F-zh-1` (rows whose finding says "shared with N other bank rows" with N above 0).

**Decision (Eric, 2026-11-05): D7 not applied.** The 505 rows are tagged `monosyllable_shared_key` in `assets/words/purity/monosyllable-shared-key/zh.tsv`; nothing is quarantined. Apply D7 only if a character-typed zh mode ships, and revisit then with the 现代汉语常用词表 cross-check.

## C6 and D-LIC
- `assets/words/LICENSES.md`: "as recorded" is now "confirmed 2026-11-05", the open verification note is replaced, and D-LIC is written into the file. `grep -ri "unknown\|as recorded" assets/words/LICENSES.md` returns nothing.
- `data/<lang>/SOURCES.md` for es, fil, ko, vi, zh are unfilled templates (every version and attribution cell is "TODO"). Sources that fail D-LIC as written there:

| Lang | Source | Why it fails |
|---|---|---|
| es | kaikki.org Wiktionary (word stories) | CC BY-SA 4.0. The template says it must not ship until you approve an attribution approach. |
| fil | "license-verified GitHub Tagalog lists" | licence "per repo", never recorded |
| fil | kaikki.org Wiktionary (tl) | CC BY-SA |
| ko | NIKL 한국어 학습용 어휘 | "verify redistribution", never recorded |
| ko | 우리말샘 | CC BY-SA |
| ko | kaikki.org Wiktionary (ko) | CC BY-SA |
| vi | Hunspell Vietnamese wordlist | "verify variant", never recorded |
| vi | kaikki.org Wiktionary (vi) | CC BY-SA |
| vi | wordfreq (vi) | "verify", never recorded |
| zh | CC-CEDICT | CC BY-SA 4.0 |
| zh | HSK word lists | "official list", licence not stated |
| zh | wordfreq / SUBTLEX-CH | SUBTLEX is research-licensed (not permissive) |

  The Leipzig Tagalog line ("research, verify redistribution") now passes under your C6 ruling.
- A wider problem outside this task's scope, not touched: `NOTICES.md` says the definitions the app shows are served from CC BY-SA sources (Wiktionary glosses for every language, CC-CEDICT, JMdict, KANJIDIC2, and the hermitdave FrequencyWords list). Under D-LIC those do not ship. I did not remove anything. Whether they are "shipped data under assets/" or server-side material served from `backend/` is your call.
- GPL/LGPL Hunspell dictionaries are still used by `tools/bank/gen_ledger.py` for PURITY verdicts. That is a cross-check, consistent with D-LIC ("never a source of rows"). I used the pt_BR and sw_TZ ones the same way below.

## S2: inflected candidates
No row was quarantined on the C4 heuristic. 11,906 rows are tagged `inflected_candidate` in `assets/words/purity/inflected-candidates/<lang>.tsv` (es 2,249; fr 1,862; pt 2,400; pl 1,687; ru 2,746; ko 170; fil 361; sw 295; hi 136). **Deviation:** the tag lives in a sibling file, not in the ledger itself, because the ledger's verdict set is closed (`dictionary | exception | pending`) and `tools/bank/purity_gate.py` rejects anything else. Changing the gate is a signed-file change. I7 is now "no auditor-confirmed inflected form". Bank floors untouched.

## S3: Russian ё/е, all 28 pairs
`src/norm.rs` `yo_one_way`: typed е matches stored ё, typed ё never matches stored е; Spell Jr is unchanged (it strips accents both ways, D14). 22 same-word е-rows are quarantined as `yo_duplicate` (0.32% of ru). 6 pairs are different words and both rows stay.

| ё row | е row | Verdict |
|---|---|---|
| боёв | боев | quarantine боев |
| ведёт | ведет | quarantine ведет |
| всё | все | keep both: different words |
| всём | всем | keep both: different words |
| вёл | вел | quarantine вел |
| даёт | дает | quarantine дает |
| ещё | еще | quarantine еще |
| её | ее | quarantine ее |
| идёт | идет | quarantine идет |
| лёд | лед | quarantine лед |
| населённых | населенных | quarantine населенных |
| неё | нее | quarantine нее |
| нём | нем | keep both: different words (нём is "in him", нем is "mute") |
| объём | объем | quarantine объем |
| остаётся | остается | quarantine остается |
| перёд | перед | keep both: different words |
| провёл | провел | quarantine провел |
| путём | путем | quarantine путем |
| самолёт | самолет | quarantine самолет |
| своё | свое | quarantine свое |
| своём | своем | quarantine своем |
| счёт | счет | quarantine счет |
| трёх | трех | quarantine трех |
| узнаёт | узнает | keep both: different words (imperfective vs perfective) |
| ушёл | ушел | quarantine ушел |
| чём | чем | keep both: different words |
| четырёх | четырех | quarantine четырех |
| чёрный | черный | quarantine черный |

Side effects: in 21 of the 22 pairs the kept ё row sits in the same or an easier tier than the quarantined е row, so nothing gets harder. The exception is населенных (Hard), whose ё partner населённых is Expert; I did not retier anything (tier assignments are a non-goal). The ru sound-alike table fell from 33 sets to 12, because the e/ё pairs no longer both exist. Since the typed-side fold now accepts е for ё, the six kept pairs behave as follows: typing все for target всё is correct, and typing всё for target все is not. Typing "нем" for target нём is therefore correct too.

## S5: Swahili apostrophe
- `assets/keyboards/sw.json` and the matching `SW` layout in `src/keyboard.rs` now end the last row with an apostrophe key (the same shape as the Filipino hyphen key).
- `scripts/build-wordlists.py` keeps U+0027 for sw only.
- Eight ng' rows were added from the Leipzig `swa_wikipedia_2021_100K` corpus (curly forms folded), each confirmed by Hunspell sw_TZ as a cross-check only: ng'ombe, ng'ambo, kung'aa, mng'aro, king'ora (hard), mng'ao (medium), kung'amua, kunung'unika (expert). They are singular nouns or ku- infinitives. Ledger lines are `dictionary sw_TZ`. Other corpus ng' forms were skipped: plurals, tensed verbs, and words Hunspell sw_TZ does not list.
- The F2 note in `src/norm.rs` now says sw spends the apostrophe as a letter. `fold_apostrophes` maps the curly forms and deletes nothing, so it does not break ng' grading. Tests: ng'ombe is correct typed with U+0027, U+2019 and U+2018; ngombe is not.
- Not done: audio for the eight new words is not warmed in the cache (server task), and fr stays a non-goal.

## zh I1 carve-out
Same toned pinyin with different hanzi is exempt (254 groups, up to nine rows). The two groups with identical hanzi are fixed: `hu2li2|狐狸` and `you3zhu4|有着` are quarantined as `duplicate_hanzi`, keeping the standard readings hú·li and yǒu·zhe. The census script now measures zh I1 on the hanzi side.

## Row cleanups
- fr: événement quarantined, évènement (the 1990 form) kept, événement an accepted variant.
- vi: đựơc quarantined (`misspelled_nonstandard`). hí kept, hý quarantined and accepted. **Assumption:** you did not say which side to keep for hí/hý. I kept the i form. Change it if you want y.
- fil: lalake quarantined, lalaki kept, lalake accepted.
- hi: the seven no-nukta rows quarantined; the nukta rows kept; the no-nukta spellings accepted.
- pt: **I changed four rows, not three.** The resolution says "the 3 pre-AO90 / PT-PT forms"; the census found pára, dêem, facto and secção as real cases, and the third F-pt-1 hit (sequência) is correct AO90, so I left it. pára became para, dêem became deem, facto became fato (renamed in place, tiers kept, gloss and definition rows re-keyed), and secção was quarantined as a duplicate of seção. The old forms are accepted variants. This contradicts the earlier D-pt1 line ("pre-AO90 forms are close, not correct"); I followed the resolution. The renamed deem was `pending` in the ledger; Hunspell pt_BR accepts it, so I set it to `dictionary` and removed it from the grandfathered list. The other watch-list hits (comboio, rapariga) are lexical regionalisms, not spelling variants; untouched.
- Variant files: new `data/{fr,pt,vi,fil,hi}/accepted-variants.tsv`, wired into `src/accepted.rs` (same one-way format as de and ja). Every row has a source: Rectifications 1990, AO90 Base IV / IX, KWF 2014, Central Hindi Directorate, and for vi the 1984 Ministry of Education spelling rule (Quyết định 240/QĐ). I did not verify any citation text against the source this session (no network lookups allowed), and the vi decision number in particular is from memory. **Deviation:** the variant mechanism has no tier gate, so the Filipino legacy form lalake is accepted at every tier. The original spec accepted it at Easy and Spell Jr only.

## Tests and gates
- `cargo test` (lib): baseline before any change 1,657 passed, 0 failed. After the cleanups the derived artifacts needed regeneration, which I did by their documented commands: the Guardian purity file, `tools/build_collisions.py` (sound-alike tables), `tools/soundalikes_report.py`, `tools/impossible/build_exclusion.py` and `build_pools.py`, and the Spell Search golden digest (re-pinned from 0x81bb_1d05_6e8b_1863 to 0xfde8_d0a0_00cf_50ba with a reason beside it, and the same constant in `tests/e2e/specs/spell-search.mjs`).
- ru sound-alike table: regenerating it dropped it from `verified` to `missing` because the quarantine created a new set ([лет, лёд]) that had no voice-loopback result, and that emptied Russian's Impossible pool. I re-ran `tools/soundalike_loopback.py --lang ru` against the local server (clips from the warmed cache; same calibration bar 0.6495; 8 true sets measured, 2 flagged `voice-may-distinguish`, down from 7 of 34). That tool only measures currently-true sets, so it dropped the four sets an auditor had demoted to accent (всем/всём, перед/перёд, узнает/узнаёт, чем/чём), and `build_collisions.py` still requires an entry for them. I copied their 2026-10-17 entries back from the previous file. The tool should keep demoted sets' entries; that is a latent bug, not fixed here. The two flagged sets go to the auditor sheet as usual.
- Final result: `cargo test`, all targets, 0 failed (lib: 1,661 passed, 27 ignored; baseline 1,657 plus my 4 new tests).
- New tests: `ru_yo_fold_is_one_way`, `sw_ng_apostrophe_is_a_letter_and_curly_forms_fold_to_it`, `ortho_sweep_all_cleanup_variants_grade_both_spellings_of_one_row`, `every_new_variant_row_cites_a_rule_and_resolves_to_one_stored_row`.
- Gate checks run and passing: purity gate, collision tables, soundalikes report, Impossible pools and exclusions, composite pins, ru identity, lexicon load, sound-alike API. **Not run:** the e2e/Playwright half of `scripts/gate.sh` and the wasm build, so the e2e golden edit is unverified.

## I1 after the changes
`tools/ortho_census.py` re-run: I1 = 0 for all eleven languages under the rules above (ru measured under the one-way fold, zh on identical hanzi).

## Quarantined or moved rows (36 moved, 3 renamed, 8 added)
| Lang | Class | Rows |
|---|---|---|
| ru | yo_duplicate | 22 (list above) |
| fr | variant_duplicate | événement |
| vi | misspelled_nonstandard | đựơc |
| vi | variant_duplicate | hý |
| fil | variant_duplicate | lalake |
| hi | variant_duplicate | बर्फ, अंग्रेजी, डिजाइन, फरवरी, अफगानिस्तान, स्विट्जरलैंड, जबरदस्त |
| pt | variant_duplicate | secção |
| zh | duplicate_hanzi | hu2li2\|狐狸, you3zhu4\|有着 |

Full per-row table with tier and reason: `reports/ortho-census/quarantine-batch1.tsv`. No language lost more than 0.4% of its rows.

## Still open for you
1. D7: drop 505 zh monosyllables, drop fewer, or change the rule?
2. hí versus hý: confirm the i form.
3. The pt count (4 changed, not 3) and the lalake tier gate.
4. NOTICES.md shows CC BY-SA definition sources already in the app; D-LIC as written bans them.
5. The e2e half of the gate has not been run on this branch.

## Phase 1 proper: es, fr, pt (2026-11-06)

Built: src/ortho_profile.rs (data-driven, no language branches), data/{es,fr,pt}/ortho-profile.json, schema + tools/ortho/profile_check.py, accepted-variants with sources + tools/ortho/provenance-gate.py (selftest plants a BY-SA row), golden files tests/ortho/golden/{es,fr,pt}.tsv (214 / 338 / 325 rows, every lane at least 30), sound-alike and near-sound rule files for SENSE-CUE / VOICE-CLARITY, auditor addendum and Gig B sheets (reports/ortho-gigb-*.csv). The three ortho_profile flags ship OFF (test asserts it); with a flag off the old grading path runs unchanged.

Gate: cargo test green (1,667 lib). E2E app 597/614 before one fix, which makes the vi picture test correct: it typed d-bar as a direct tap although it is the long-press of d (removing two vi bank rows changed the first picture word from "tung" to one containing d-bar, exposing the latent bug). Remaining red, none from this change: 15 online-room specs need target/debug/spell-server (cargo build -p spell-server --features test-modes) and A13.3 needs a signed kill document; site boardgame_wall (ar) fails identically on clean origin/main; site picture-wall failed once in the gate and passes on rerun (timing).

Open for Eric: es casa/caza and cocer/coser (Spain voice makes them accent pairs; table still lists them true), es periodo/periodo-accented and video/video-accented duplicate rows, fr and pt tables missing (seeds are pending_table), close-feedback strings for non-German languages not wired, hi vs hy, fil lalake tier gate, NOTICES.md BY-SA wording, soundalike_loopback.py drops demoted sets.

## Follow-up cleanups (2026-11-06, Eric "Yes, do them")

- es: periodo and video quarantined; período and vídeo kept; the bare forms are accepted variants (tools/ortho/sweep_all_batch2.py). Citations are the RAE's general statement that both forms are valid; I did not verify the text against the source.
- fil: lalakeng quarantined, lalaking kept, lalakeng an accepted variant (the inflected sibling of Eric's lalake/lalaki decision). A tier gate for lalake was NOT added: Eric's resolution says accepted variant, and the variant file accepts at every tier by design. My earlier "lalake lacks a tier gate" was not grounded in the spec. reports/ortho-census/fil-vowel-pairs.tsv lists 24 more fil e/i and o/u pairs for Eric; none was touched, several are different words.
- NOTICES.md: BY-SA content IS shipped (definitions), so the note was extended, not reversed: BY-SA feeds definitions only; spelling data for the eleven sweep languages cites no BY-SA source. It states that earlier banks (Persian) are not covered by that rule.
- tools/soundalike_loopback.py: carries forward prior results for sets it did not re-measure (demoted sets), only when the voice is unchanged; --selftest added and wired into the gate.
- Regenerated: guardian-purity.json, es/fil soundalikes.json and report, wordsearch GOLDEN (re-pinned in both places). Not re-run: the full gate (its e2e half has the known environment failures listed above).

## fil e/i, o/u same-word pairs (2026-11-06, Eric "go")

Rule (tools/ortho/sweep_all_batch3.py): keep the spelling with at least 20 hits and at least 3x the other in Leipzig tgl_wikipedia_2021_100K (CC BY 4.0). Quarantined, with the kept form first: babaeng over babaing (412 vs 26), kabuuan over kabuoan (238 vs 28), koponan over kuponan (258 vs 60), legal over ligal (122 vs 38), politiko over pulitiko (111 vs 28). The loser of each is an accepted variant. 15 pairs under the bar are untouched and listed with counts in reports/ortho-fil-vowel-auditor-sheet.csv. Excluded as probably different words: bola/bula, noon/noong with nuon/nuong, pares/paris. Frequency is evidence of usage, not of the KWF-preferred form; the auditor may overrule any choice. Cargo test green (1,667); word-search digest unchanged.

## fil reviewer verdict sheet applied (2026-11-06, pasted by Eric)

tools/ortho/sweep_all_batch4.py. Merged (kept form first, other quarantined and an accepted variant): istasyon/estasyon, istilo/estilo, istruktura/estruktura, iskultura/eskultura, kemikal/kimikal, kompanya/kumpanya, kombinasyon/kumbinasyon, kompetisyon/kumpetisyon, tradisyonal/tradisyunal, inhinyeriya/inhenyeriya. Quarantined without a variant: espirito, kuponan, kabuoan (misspellings). Cut as non-headwords or non-words: babaeng, babaing, lalakeng (the -ng linker; babae and lalaki are stored), dae, dai. The earlier frequency-based kept forms for babaeng, kabuuan, koponan, legal, politiko were replaced or confirmed by the sheet. HELD, untouched: komplikado/kumplikado and politika/pulitika (reviewer: Low, Paul decides; politiko/pulitiko was already merged to politiko and follows politika), sunod-sunod/sunud-sunod (sheet says accepted at Easy and Spell Jr only; variants carry no tier gate, so that needs a design decision). Open: lalaking and the other -ng linker rows are the same class as the ones cut and were not touched; "legal" needs Paul's confirmation under the PURITY raw-English rule. Cargo test green (1,667).
