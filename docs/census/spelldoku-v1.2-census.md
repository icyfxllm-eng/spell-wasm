# CC-SPELLDOKU v1.2 — §0 census (Word Mode)

Run 2026-09-19, read-only, on main after build 229 (Spell Search). Items 1–8
were answered in `docs/census/spelldoku-census.md`; only changes are noted.
Rulings: `docs/CC-SPELLDOKU-v1.2.md`.

## Stop-and-ask triggers

| Trigger | Fires? |
|---|---|
| 1 name collision | No |
| 3 no audio-unavailable state | No: `api::play_word` has `on_fail`, and `wordsearch::lexicon::SERVER_VOICE` lists the voiced languages |
| 7 no app-only mechanism | No: `build.rs` strip + `#[cfg(not(feature = "web"))]` + `"platforms": ["ios"]` |
| 8 no preview flag readable at runtime | No: the flag is `audit_preview`, read through `spelldoku::table::current_build()` |
| 9 no language clears 95% at 9×9 | No (see item 9) |
| **12 no reusable server-generation path** | **Yes.** Ruled R2: 12×12 generates on device |

## Items 1–8, changes only

- Number tables: `config/numbers/*.json` now cover all 15 languages, rows 0–12 (English 0–20, 30, 40), every row `sourced`, none `audited`.
- Jr resolver: `src/experience.rs` (`resolve`, `allowed_tiers`, `serve_tier`, `leaderboard_allowed`).
- Everything else as in the v1 census.

## Item 9 — bank depth and eligibility

Bank rows per band (`words::tier_for`):

| lang | easy | medium | hard | expert |
|---|---|---|---|---|
| en | 768 | 801 | 801 | 800 |
| es | 268 | 877 | 1974 | 2978 |
| fr | 279 | 889 | 1967 | 2974 |
| de | 272 | 954 | 1967 | 2951 |
| pt | 272 | 908 | 1966 | 2966 |
| pl | 267 | 961 | 1969 | 2978 |
| ru | 267 | 970 | 1977 | 2994 |
| vi | 246 | 756 | 1163 | 1929 |
| ko | 292 | 970 | 1980 | 2992 |
| ja | 253 | 967 | 1970 | 2970 |
| zh | 296 | 944 | 1958 | 2984 |
| fil | 247 | 701 | 1159 | 1976 |
| sw | 262 | 703 | 1180 | 700 |
| ar | 290 | 968 | 1987 | 2993 |
| hi | 293 | 781 | 800 | 800 |

- **Audited status:** bank words carry no per-row audited flag (language-level status only, in `consts::BUILTIN_LANGS`). Only number-table rows have `status`. See R1.
- **Length cap:** none exists in the fragment renderer. Assumed 12 graphemes (R6).
- **Distinct initial glyphs:** Latin and Cyrillic scripts 18–33 per band. Arabic 29–31, but ا leads most medium–expert rows (the ال article). Hindi 116–163. Japanese 58–76. Korean 200–605 by syllable, 18–19 by jamo. Chinese 272–1,393 by Hanzi, 25–26 by pinyin initial.
- **Data defect:** 113 hard and 215 expert Japanese rows begin with a small kana (ぁ, ょ…). Tracked as a separate fix.
- **Confusion gate (English):** 22 confusable bank pairs, 5 with different initials (cell/sell, knew/new, know/now, right/wright, rite/write).

Simulation (seed 20260919, 1,000 attempts per language, glyph view and F12
mix, up to 200 redraws per attempt; script kept outside the repo):

- **Redrawing only the rejected word:** 100% for every language, size and tier.
- **Redrawing the whole set:** below 95% at 12×12 for en 38.5, es 55.6, fr 70.0, de 93.4, pt 59.3, pl 70.5, ru 61.2, vi 60.5, fil 29.8, sw 0.9, ar 2.3, ko-by-jamo 5.9; at 9×9 for sw Expert 62.2, ar Hard 44.1 and Expert 54.4, ko-by-jamo Medium 91.8 and Hard 86.4. ja, hi, zh and ko-by-syllable clear 95% everywhere.

## Item 10 — confusion data

- No language-level matrix exists (CC-WORDGRID E5). English has a hand-authored list, `config/confusions/en.json`.
- `wordsearch::confusion::edits` is the word-to-word test (test-only today; R4 opens it). `decoys()` reads the shipped decoy table and cannot compare two bank words.
- `reports::confusion_pairs` is per-player and is not a source (CC-WORDGRID D6).

## Item 11 — My Words and missed words

- **My Words** (`word_lists.rs`, key `byear_word_lists_v1`): `ListEntry {text, lang, added_at, starred}` carries **no band**. A band can be derived by exact bank match (`translate_screen::bank_tier_of`, private and app-only; it misses zh rows stored as `pinyin|hanzi`).
- **Missed words** (`model::MissEntry {word, lang, tier, …}`, key `byear_misses_v1`, cap 300) **carries a tier**. API: `misses::load`, `due_misses`, `add_miss`.

## Item 12 — server generation

None. The backend serves speech, answer checks, meanings, auth, the Climb and matches; nothing generates puzzles or ships seed packs. CC-WORDGRID's Daily is generated on device (`wordsearch::serve::daily`); its server Phase C is unbuilt.
