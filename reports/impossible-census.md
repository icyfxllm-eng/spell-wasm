# CC-IMPOSSIBLE v1 §0 census

Run 2026-10-15 on `main` (local checkout; HEAD 08da2864, behind `origin/main` 1facad06). Read-only: no bank row, registry entry, flag or app code changed.
Tooling: a throwaway script (not committed) that reuses `tools/difficulty-score/retier.py`'s `composite()` and `tools/build_collisions.py`'s key functions, the PURITY ledgers in `assets/words/purity/`, and the bank at `assets/words/<lang>/<tier>.txt`.

## Headline: what Eric needs to decide before Phase A

1. **C2: there is no stored per-row difficulty score.** The nearest thing is `retier.composite()`, a tool-time proxy (extractor features + bigram rarity + vowel runs + length). It covers 10 of the 15 registry languages (en es fr de pt pl vi ko ja fil). It does not cover **ru ar hi sw zh**. Per §0's exit gate, those five are ineligible unless Eric signs a score. I have not invented one.
2. **C4/C5: pronunciation data barely exists, and F2 fails closed.** Only en (CMUdict), es and ru (rule-derived keys), and zh (pinyin inside the row) have a key. Of the 10 scored languages, **8 resolve to an empty pool**: fr de pt pl vi ko ja fil.
3. **C6: English, the free language, is locked at its default.** At 10% English yields 80 words, below D3's 150. It reaches 150 only at 20%, the D1 cap, which D1 forbids raising "just to reach D3's minimum." Spanish clears it at the 10% default (263; 5% would give only 131).
4. **C11: The Climb has no per-answer score.** It ranks chain length, with a separate board per difficulty. F7's "×1.5 score" and "single universal leaderboard" don't match the code. See C11.
5. **C10: the trap is not a "top trap."** Traps are drawn uniformly from 5 kinds. A12's wording needs adjusting.
6. **Quality flag (§8 trigger 3):** the English top slice is dominated by long inflected words. See C6 notes.

Net: as built today, Impossible v1 could ship in **Spanish only** (English only by breaking D1/D3).

## C1: difficulty enum and tier registry

There is no enum. Tiers are bare strings, and "single source of truth" does not hold for tiers today. Switch sites:

| Place | File:line |
|---|---|
| Ordered tier list | `src/consts.rs:1` (`TIER_ORDER`), `src/experience.rs:29` (`TIERS`) |
| Level picker options | `src/consts.rs:22` (`LEVEL_OPTS`), `src/game.rs:904` (`offered_levels`) |
| Per-mode allowed tiers (Junior vs Standard) | `src/experience.rs:86` (`allowed_tiers`), `:108` (`serve_tier`); data `config/modes.json`, golden `config/allowed-tiers.golden.json` |
| Word pool by tier | `src/words.rs:68` (`simple_tier`), `:6288` (`tier_for`) |
| Time per tier | `src/consts.rs:3` (`tier_time`) |
| Climb band to tier | `src/game.rs:211-295` (`serve_climb_tier`, `band_to_tier`, streak ladders) |
| Climb ranked set (client / server) | `src/climb.rs:17` / `backend/climb.py:23` |
| Daily arc | `src/daily.rs:17` (`ARC`) |
| Board game | `src/boardgame/engine.rs:371,415` (`Tier::Expert`) |
| Other hard-coded tier lists | `wordpic.rs:37,588`, `wordpic_screen.rs` (many), `defmatch.rs:157`, `bee.rs:34`, `online_spelloff.rs:129`, `practice.rs:571`, `word_data_audit.rs:26` |

Consequence: adding `impossible` means a real registry has to be introduced first. Several of the sites above (Bee, WordPic, Defmatch, Spell Off) would silently treat an unknown tier in their own way. A3/A5/I3 enforcement needs that registry.

## C2: per-language difficulty score

| Language | Stored field on rows | Computable proxy (`retier.composite`) |
|---|---|---|
| en es fr de pt pl vi ko ja fil | none (rows are bare strings) | yes; range roughly 0 to 10, float, not persisted |
| zh ru ar hi sw (fa unregistered) | none | **no**; not in `retier.LANGS` (ineligible per §0 gate) |

`data/<lang>/lexicon.jsonl` has `freq_rank`/`grade`/`pron` fields, but they are null (a 2.7k-row seed, not the shipped bank). `retier.py` swaps only within hard∪expert and keeps Expert's count fixed. Expert membership is therefore already "top by this composite within hard∪expert", which makes "top 10% of Expert" a slice of a slice.

## C3: expert pool after the PURITY gate

PURITY-passing = in the ledger as `dictionary`/`exception`, or `pending` and grandfathered, and not in `awaiting-c6`. vi has no ledger (it is not in `purity_gate.py` LANGS, so the gate does not apply), and its figure is the raw bank.

| Lang | Expert rows | PURITY-passing | Lang | Expert rows | PURITY-passing |
|---|---|---|---|---|---|
| en | 797 | 797 | ko | 1455 | 1453 |
| es | 2630 | 2629 | ja | 2344 | 2342 |
| fr | 2556 | 2553 | fil | 1472 | 1470 |
| de | 2650 | 2648 | zh | 2984 | 2975 |
| pt | 2567 | 2564 | ru | 2884 | 2884 |
| pl | 2785 | 2783 | ar | 2525 | 2525 |
| vi | 2500 | 2500 (no ledger) | hi | 287 | 287 |
| | | | sw | 683 | 683 |

Open item: I1 says the pool is a subset of rows passing PURITY's build gate, but vi is outside that gate, so vi cannot satisfy I1 as written.

## C4: pronunciation data per row

| Lang | Source | Coverage of full bank | Caveat |
|---|---|---|---|
| en | CMUdict (`tools/build_collisions.py` `key_en`) | 99.8% | heteronyms and unlisted words get no key (4 expert rows) |
| es | orthographic rule key (seseo, yeísmo, h, b/v) | 100% | not true pronunciation; approximates "audio can't carry it" |
| ru | rule key: devoicing and assimilation only | 100% | no vowel reduction (stress data is dark), so it under-collides, which is the unsafe direction for F2 |
| zh | pinyin with tone digits, inside the row (`pinyin\|hanzi`) | 100% | zh is out of scope (D4) |
| fr de pt pl vi ko ja fil ar hi sw | none | 0% | fail-closed leaves nothing. ja rows are kana, so the row is its own reading, but there is no data to detect cross-kanji homophones. ko spelling is not phonemic. |

The existing `assets/words/{en,es,ru}/homophones.txt` are computed from the same keys. For en they add unbanked homophones (a few expert rows: colonel, gauge, principal, kowtow, and others).

## C5: F2 homophone filter dry run

Counts are within the slice. "No key" means fail-closed exclusion.

| Lang | Bank-wide expert rows with a homophone | Excluded in top 10% slice | No key (fail closed) in slice |
|---|---|---|---|
| en | 2 by key; 10 in `homophones.txt` | 0 by key; 1 more via `homophones.txt` | 0 |
| es | 6 | 0 | 0 |
| fr de pt pl vi ko ja fil | not computable | 0 | **all** (255, 265, 256, 278, 250, 145, 234, 147) |
| ru | 3 | n/a (unscored) | 0 |
| zh | 250 | n/a (unscored, D4) | 0 |
| ar hi sw | not computable | n/a | all |

Caveat: collisions are measured within the language's banked rows, as F2 step 4 specifies. A word whose homophone is unbanked passes unless `homophones.txt` (en/es/ru only) lists it.

## C6: eligible Impossible pool by slice

Slice = top N% of PURITY-passing Expert by proxy score, then homophone-filtered, then fail-closed. D3 floor = 150.

| Lang | 5% | 10% | 15% | 20% |
|---|---|---|---|---|
| en | 40 | 80 | 120 | 159 (156 after `homophones.txt`) |
| es | 131 | **263** | 394 | 526 |
| fr de pt pl vi ko ja fil | 0 | 0 | 0 | 0 |
| zh ru ar hi sw | no score (C2) | no score | no score | no score |

Reading against D1 and D3:
- **es** passes the floor at the default 10%.
- **en** is locked at the default 10% (80 words). It reaches 150 only at 20%. Choosing 20% *because* of D3 is the thing D1 and I6 forbid, so Eric has to decide it explicitly.
- Everything else is empty or ineligible today.

**Pool-pattern flag (§8 trigger 3).** The English top-10 by proxy: sufficiently, communications, significantly, communication, quintessential, classification, significance, sufficient, occasionally, transmission. Spanish: yuxtaposición, exhibición, deshidratación, exageración, haciéndole, ejecución. The composite's length and bigram-rarity terms favour long, suffixed forms and keep inflection pairs (communication / communications). I'm reporting it rather than tuning around it. It is arguably not "hardest to spell from audio", and F2 has no inflection dedupe.

## C7: replay mechanics

There is a replay control (`replayBtn`, `game.rs:1771`) and `note_replay` (`game.rs:1335`). It only sets `aids.replayed`, which downgrades a clean answer from Good to Hard in spaced repetition (`game.rs:2092`). **No cap and no per-tier setting exists**, so D7's "1 replay" is new code, including a "No replays left" state. Prior art: `wordpic_screen.rs:2709-2740` has per-tier `audio_gate` (expert loses Slow but keeps Replay). Extra attempts: `settings.rs:34,228` (`extra_attempts` pref, gated by `flags::attempts_shields`); one retry per word via `aids.retry_used` (`game.rs:2641-2749`).

## C8: answer leniency by tier

There is no per-tier difference. Leniency is by **Kid Mode** only (`src/norm.rs:89`): Kid uses `fold_lenient` (strips combining marks, ß/æ/œ/ł/ø/ı, whitespace). Everyone else uses `fold_strict`: NFC + Unicode lowercase + whitespace and Arabic silent marks removed.

| Lang | Tier | Standard | Kid |
|---|---|---|---|
| all | easy/medium/hard/expert | strict fold: **case-insensitive**, accent-strict | accent-lenient |

Consequences for F4: `fold_strict` lowercases, so German-noun capitalisation is **not** checked today ("no case folding where the language stores case meaningfully" would be a new rule, not existing behaviour). The Vietnamese tone-placement equivalence lives elsewhere (a `norm`/Vietnamese path; not located in this pass, and A6 depends on it).

## C9: Daily Challenge seeding and storage

`daily.rs:225 build_words(lang, date, kid)`: seed = FNV/splitmix over `ns|locale|tier|cycle|pool_hash|salt`, `DAILY_NS = "daily-v2"` (`:259`; "never changes"). It runs a per-tier cycle walk with a retry salt (cap 8) to avoid yesterday's words. Fixed arc `2 easy / 3 medium / 3 hard / 2 expert` (`:17`). Results are **local only**: `Record{last_completed, streak, best_streak, history: date→correct}` in key `spell_daily_v1` (`:22`). There is **no leaderboard and no per-mode board key** for Daily. F8's "its own per-mode board (O1)" is therefore new infrastructure, not a choice between two existing boards. The Spell Off (`spelloff-v1`) is the only server-seeded variant.

## C10: BoardGame "Double Expert" trap

Defined at `src/boardgame/mod.rs:85` (`Trap::DoubleExpert`; `ALL` at `:90`, label key `bg.trap.double` = "Double Expert", `i18n/locales/en.json:877`). Placement: `boardgame/rules.rs:199` assigns each trap tile `Trap::ALL[rng % 5]`, **uniform over 5 kinds**, so there is no "top" or hardest trap tile. Spell: `engine.rs:414-417` draws one Expert word, and the second is drawn on success (`:361-376`). Fail path: `engine.rs:535-543` sets `skip = true` on either failure, the lose-next-roll rule. NPCs use `self.chance(p)` per word with no per-tier input I found, so O3 is moot unless `p` varies by tier (not checked).

## C11: The Climb scoring

`src/climb.rs:336 submit_run(difficulty, chain, duration_ms)` posts to `/api/climb/submit-chain`; `backend/climb.py:249-304` stores `best_chain` per `(user, difficulty, locale)` and ranks by it. `VALID_DIFFICULTIES = (medium, hard, expert)` (`:23`). **There is no per-answer score, and so no multiplier input.** Also, the leaderboard is already one board per difficulty and locale, so "single universal board" is not today's design. And the Climb's tier comes from the player's band (`game.rs:211-295`: a streak ladder up to Expert), not from a chosen difficulty. "A correct answer at Impossible during a Climb run" has no current hook.

Per F7's stop-and-ask, I am **not** drafting the diff. Candidate approaches for Eric: (a) a chain-weighting scheme (an Impossible word counts as 1.5 chain units, which changes the integer ranking and the server's `chain`/`wordCount`/`MIN_MS_PER_WORD` sanity checks); (b) a new top Climb band that serves Impossible with a normal +1; (c) drop F7 from v1.

## C12: entitlement per language

`src/entitlements.rs:164-215` `resolve_entitlements` / `resolve_for_edition`: `free_tier()` (`:125`) = English Full, every other language Preview (tier 1 only, `preview_allows` `:321`); regional grants (`:341-364`, map `config/country-language-map.json`) raise named languages to Full; a Complete purchase raises all to Full; the education edition adds an audit-passed grant (`:249`). `AccessLevel` is per language, and Preview excludes Daily and Climb. A language-level `Full` grant is exactly what F11/O2 propose to follow. The RTL gate is `consts.rs:147,229,343` (`RTL_SUPPORTED = true`, so ar is open today).

## Other findings worth knowing

- The referenced **CC-BANK-REBUILD** and **CC-BOARD-GAME** files are not in this repo (`docs/`). The shared no-repeat mechanism I can see is `src/deck.rs`: a persisted per-pool shuffled deck keyed by language and difficulty (plus a 5-word recent window), not per profile. F5 says "per profile and language" and "CC-BANK-REBUILD", so please confirm that is the same thing.
- Flags: `src/flags.rs` resolves from locally stored values with defaults. I did not verify that a *remote* flag channel exists. F12 and the "turn on per language without resubmission" claim depend on it.
- `Expert` is also in the Jr/Kid allow-lists via `config/modes.json` (Junior caps at Medium), so I3 for Kid Mode and Spell Jr is plausible through `allowed_tiers`. Randospell and Little Speller were not inspected here.

## Census exit gate

Not passed automatically. The items needing Eric's call before Phase A: the five languages with no score (C2), English's lock at 10% (C6), the Climb model (C11), the Daily leaderboard (C9), the case-folding gap (C8), and whether the top-slice pattern (C6) is acceptable.
