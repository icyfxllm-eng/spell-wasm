# CC-ORTHO-SWEEP-ALL v1: §0 census

Run 2026-11-05 on `origin/main` bf2342c0 (worktree `spell-ortho-census`, branch `census/ortho-all`). Read-only over banks, grader and profiles.
Script: `tools/ortho_census.py`. Raw output: `reports/ortho-census/` (`summary.json`, `c3-collisions-<lang>.tsv`, `c7-<lang>.csv`).

**Status: HALT.** Six stop-and-ask conditions fired (next section). Nothing was edited and no phase was started.

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
