# CC-SPELLUZZLE §4 census

**Run 2026-10-09 against `origin/main` c052f07c. Read-only: no repo code, bank or registry changed.**
**Phases A–C are built (2026-10-09). v1.1 additions are in the section at the end.**

Compiled from three read-only surveys (appendices). C7 ran a Python prototype generator and an independent checker on the real English bank; it is a feasibility measurement, not the Rust implementation.

## Decisions recorded (Eric, 2026-10-09)

1. **Launch scope: English only, Easy through Expert** (plus Spell Jr). Every other language waits.
2. **F10 amendment: Expert word length is 6–10 cells** (was 6–9). English Expert eligible words rise from 327 to 540, clearing the 400 floor. Check in Phase A: at 10 cells the C8 width limit falls to about 32.8 pt with no row gutter and about 30 pt with a 28 pt gutter, so A15 must be re-run at 10 cells and the layout may need a narrower gutter or tighter gaps.
3. **C7:** fix the generator's search (Jr and Easy fail on G5) rather than any gate; re-run the full 10,000 seeds per tier; measure iPhone p95 on a device in Phase A. If Jr still cannot reach 10,000/10,000, use F9's word-overlap relaxation. Never relax a gate.
4. **Fixtures:** replace with boards generated from the real bank. Do not add words to the bank for them.
5. **History:** use the F11 `spelluzzle.history` store as written (per profile). Do not fold into the wordgrid ledger; register as a consumer of the CC-PERSIAN-FOUNDATION F6 ledger if it is built. Keep off the shared 20/14/90 word window (I8).
6. **Audio amendment (narrow, named):** add one new function that prefetches a set of words and reports per-word success, built on `audio_verdict::servable` and `human_audio::clip_url`. `play_chain` is untouched. This is an exception to the "calls the resolver only" row in §2.
7. **O1–O3:** defaults stand: app-only, no "can't hear it" rescue, 🗝️.
8. **Missing spec files:** the Spelluzzle spec is authoritative; read the nearest existing docs.
9. **Later languages (Phase D):** Spanish and Russian first (need C7 and an Easy bank large enough); French, German, Portuguese, Polish, Filipino and Swahili wait for collision tables and bank growth; Vietnamese never in v1.

The offered set is therefore English Jr, Easy, Medium, Hard and Expert (6–10). The C4 and C7 HALTs are answered by items 2–4; C9 by item 5; C10 by item 6.

## Phase A results (2026-10-09, branch `spelluzzle-phase-a`)

Pure Rust core in `src/spelluzzle/` (app only): `types`, `view` (F3/F4 display), `gates` (the independent G1-G11 checker and its G8 solver), `gen` (generator), `lex`/`bank` (word data). No UI.

**A1-A8 are green** (`cargo test spelluzzle`, 14 tests, 1.8 s in debug). The full-size run, with the 217,720-word stand-in validity list (`SPZ_N=10000 SPZ_VALIDITY_FILE=...`, release, 18 s for all five tiers):

| Tier | success | attempts p50/p95/max | runes avg | earned p10/p50/p90 | cascade p10/p50/p90 |
|---|---|---|---|---|---|
| Jr | 10000/10000 | 13/53/242 | 9.9 | .475/.526/.579 | .314/.383/.475 |
| Easy | 10000/10000 | 4/15/42 | 9.1 | .444/.491/.526 | .273/.334/.416 |
| Medium | 10000/10000 | 5/20/74 | 14.1 | .435/.481/.531 | .292/.349/.417 |
| Hard | 10000/10000 | 8/32/87 | 15.5 | .448/.500/.557 | .361/.424/.490 |
| Expert (6-10) | 10000/10000 | 18/73/267 | 15.8 | .488/.550/.623 | .431/.503/.575 |

- Every seed gave a distinct board (10,000 of 10,000 per tier). Cap hits: none. At about 0.4 ms per board on this Mac, the 1.5 s iPhone p95 limit has a wide margin, but it still has to be measured on a device.
- The generator's fix for the Jr and Easy failures was structural, not a gate change: the last pick (the secret) must have every rune already on the board and must cover every spoken rune still held by one word. That removed the G5 rejections that were driving the earlier 2-5x attempt counts.
- A4: the checker's solver and a separate linear-scan solver agree on exactly one solution (checked on 15 boards per tier at full validity size, 6 by default).
- A5: 972,240 single-cell substitutions and adjacent swaps across 300 boards per tier; every one produced a clash.
- A8: four frozen toy-lexicon fixtures are each rejected with their own gate: G3, G5, G6b, G8.
- A6: 100 golden seeds per tier are pinned in `tests/fixtures/spelluzzle/golden-en.json` against a fingerprint of the English bank. When the bank changes, re-pin with `SPZ_BLESS=1 cargo test a6_deterministic_boards`. Cross-platform parity (WASM/iOS/Android/x86 CI) is not run here: only the host was exercised.

**Open for Eric, found while building:**
1. **The validity list for G8 does not ship to the device.** On the host the default list is the bank itself, so G8 is weaker than in the table above. A served board needs the same check on the device, which means shipping a dictionary-grade English list (or a precomputed, compact table) and a licence sign-off (`data/LICENSES.md`). Until then, device-side generation would use the bank-only list.
2. **The fixtures from the spec** (`fixture-easy-1`, `fixture-medium-1`) are used only by the view tests, since they fail G1 on the real bank (census C7). A14 needs replacement boards generated from the real bank.
3. The generator's final-pick rule narrows how the secret is chosen. Variety is unaffected by the numbers above, but E6 ("ten boards in a row feel different") is a human check for Phase B.

## Phase C: verified seeds (2026-10-09)

Medium, Hard and Expert are now playable, using the build-time seed verification Eric approved.

- `scripts/spelluzzle-seeds.sh /path/to/en_US.dic` generates boards against the bank, keeps the first 4,000 seeds per tier whose board still has exactly one answer under a 469,516-word list, and writes `assets/spelluzzle/seeds-en.json` (57 KB). The dictionary never ships.
- Seeds tried to reach 4,000: Medium 6,000 (1,741 rejected by the large list), Hard 6,000 (904), Expert 6,000 (297). Nothing failed to generate.
- The device regenerates each board from its seed with the bank alone and gets the same board. A seed file is trusted only while the bank, the collision sets and `GEN_VERSION` fingerprint the same as when it was built; otherwise the tier is absent (I11).
- `cargo test spelluzzle::tests::seeds_are_current` fails when the file is stale. `audit_seeds` (ignored, needs the dictionary) re-checked 924 sampled seeds against the large list with the checker and the independent solver: all have one answer.
- Browser tests: 15 of 15, including the Medium/Hard/Expert shapes (7 rows), Listen costing the Codebreaker star, the second explainer card, and Expert at 375x667 with 10-cell rows at 32 pt or more and no scrolling while composing.

## HALT summary

| Item | Result | Detail |
|---|---|---|
| C1 | pass | One mode list (`config/modes.json`, parsed only by `modes::all()`); `group` (required) and `platforms` exist; "Word puzzles" group exists (letter_forge, word_chains, spelldoku, spell_search, spell_cross). A new mode also needs a flag id equal to its registry id, a `juniorPolicy`, name/desc keys in 13 locales and a `launch_for` entry (a test enforces it). |
| C2 | pass, with a note | Jr resolver is merged (`experience::resolve`, `allowed_tiers` in `src/experience.rs`). The spec file CC-ONBOARD-JR.md is not in the repo, only `docs/onboard-jr-inventory.md`. |
| C3 | informational | No script field in the registry; derived Latin: en es fr de pt pl vi fil sw, Cyrillic: ru. Match key is `fold_strict`; Jr uses `fold_lenient` (strips accents, ß→ss). **vi fails** (tones need a separate row). No ё/е fold exists in the match key. Accents are long-press on a base key; digraphs are 2 units. |
| C4 | **HALT** | **English Expert has 323 eligible words, below the 400 floor** (470 of 797 fail the 6–9 length gate). Also below floor: Easy in every non-English language (210–242; the banks hold only 247–279 Easy rows) and sw Expert (284). |
| C5 | partial | Collision tables exist for en (263 sets), es (11), ru (15) only. fr, de, pt, pl, fil, sw, vi have none, so they have no collision check and cannot be offered. |
| C6 | partial | No validity list ships to the device for any language. Hunspell dictionaries (build-time, Mac-local) meet the 20,000 gate for en es fr de pt pl ru sw; fil is Leipzig-only (weak); vi fails. New wordlists are a licence sign-off gate (`data/LICENSES.md`). Device-side validity can only be "is a bank word". |
| C7 | **HALT** | See below: not 10,000/10,000 in any tier (ran 2,500), Jr succeeded 2,494/2,500, and both fixtures fail G1 on the real bank. |
| C8 | pass, with caveats | Max cell at 375×667 is 36.8 pt (width-limited), above 32. Caveats below. |
| C9 | **HALT** | More than one mechanism could own Spelluzzle's history (see below). |
| C10 | **HALT** | The audio path cannot prefetch a set and report per-word success before play. |
| C11 | pass | 🗝️ is used by no mode row (only as the WordPicture icon for "ankh"). Note: calendar 🗓️ and daily 🗓 differ only by the variation selector. |

**Census outcome per the §4 rule (offered = C3, C4, C5 and C7 pass):** only English can be offered, and only Jr, Easy, Medium and Hard (Expert fails C4). es and ru pass C3 and C5 and pass C4 for every tier except Easy; their C7 has not been run. fr de pt pl fil sw fail C5; vi fails C3 and C5. Absent languages and tiers are not shown, never shown locked (I11).

## Correction to the CC-BANK-REBUILD census

The CC-BANK-REBUILD census (branch `cc-bank-rebuild-census`) said CC-BANK-PURITY was unmerged. That is no longer true: Phase A (commit 3e54e8d6, 9,340 quarantined rows), `tools/bank/purity_check.py` and `config/bank-purity-sources.json` are ancestors of `origin/main`. The banks measured here are post-sweep (the quarantine filter removes 0 further rows in the Latin/Cyrillic banks). Remaining purity risk is `pending` rows in `assets/words/purity/<lang>.tsv` (fil: 83 Medium, 288 Hard, 236 Expert). The CC-BANK-REBUILD C1 HALT should be re-checked against main.

## C7 detail

2,500 seeds per tier (not 10,000: a 5,000/tier run was projected at ~40 minutes). Validity list: a 217,720-word stand-in (bank + hunspell en_US stems + web2 + homophone members); a 39,375-word variant gave the same success. Collision table: `assets/words/en/homophones.txt` (266 CMUdict-derived sets).

| Tier | success | attempts p50/p95/max | desk model p50/p95/max | s/board p50/p95 (Mac, 10 procs) | runes avg/max | earned p10/p50/p90 | cascade p10/p50/p90 |
|---|---|---|---|---|---|---|---|
| Jr | 2494/2500 | 239/991/1973 | 49/179/481 | 0.84/3.44 | 9.7/13 | .465/.519/.571 | .321/.395/.484 |
| Easy | 2500/2500 | 81/336/960 | 26/113/314 | 0.17/0.71 | 9.0/11 | .438/.475/.526 | .276/.340/.415 |
| Medium | 2500/2500 | 44/189/779 | 46/176/496 | 0.16/0.71 | 14.2/16 | .434/.478/.528 | .293/.346/.408 |
| Hard | 2500/2500 | 24/105/291 | 44/196/465 | 0.084/0.36 | 15.7/16 | .451/.500/.552 | .357/.419/.480 |
| Expert | 2500/2500 | 33/144/495 | 39/162/376 | 0.047/0.21 | 15.9/16 | .481/.554/.630 | .415/.488/.562 |

- **Spec stop condition met:** any offered tier below 10,000/10,000. Jr failed 6 seeds at the 2,000-draw cap. Jr and Easy need 2–5× the desk model's attempts; almost all rejects are G5 (small word pools).
- **Time:** the 1.5 s p95 limit is on the oldest supported iPhone, which cannot be measured here. Jr's 3.44 s Mac p95 (in Python, under parallel load) is not comparable but is the number to watch.
- G8 and G9 do reject boards (G8: 12 Easy, 646 Medium, 220 Hard, 48 Expert draws; G9: 183/79/22), so they are not vacuous. Neither list holds inflections beyond the bank, so a real list would be stricter; re-run G8 and G9 with it.
- **Fixtures:** both fail G1 (bank band) on the real bank. fixture-easy-1: `global` and `mobile` are Medium-only; `worst agree mirror waste` are in no bank file. fixture-medium-1: none of its 7 words is in any bank file. With only the band check disabled, both pass G2–G11 (easy: 12 runes, 33 cells, earned 41/84, cascade 1241/3780; medium: 15 runes, 44 cells, earned 43/96, cascade 18541/45695). The spec forbids weakening a gate, so the fixtures need replacing or the words adding to the bank.

## C8 detail

Computed from the CSS and measured on the real style blocks in a static page; the wasm app was not built. Keyboards: main 207 pt (+14 margin), compact 172 pt. Chrome above the board ≈129 pt. Height limit for 7 rows: 49.7 pt (compact) or 42.7 pt (main). Width limit for 9 cells: 36.8 pt. Caveats: a 44 pt row gutter drops it to 31.2 pt (fails 32); Big Text on the main keyboard leaves no spare height (37.0); at 320 pt wide the limit is 30.7 pt.

## C9 detail

Live stores, all device-level, none keyed by profile: `spell_wordgrid_seen_v1` (word history, 20 puzzles/14 days, Daily 90 days; shared by Spell Search, Spell Cross, SpellDoku Word Mode), `spell_spelldoku_seen_v1` (500 board hashes), `spell_ws_daily_seen_v1` (500), `XW_SEEN` (500). Spelluzzle's 500-hash per-profile history would be a fourth copy. CC-PERSIAN-FOUNDATION F6 (per-profile ledger per mode/language/tier) and the CC-BANK-REBUILD F8 shared history are specs only. Spelluzzle's bank words would also fall under the shared 20/14/90 word window, which `CC-SPELLDOKU-v1.3` warns is already near exhaustion for English Easy (764 words).

## C10 detail

`api::play_word_with` → `play_chain` is fire-and-forget per word; failure fires only after every source has failed; there is no success callback, no batch endpoint and no pre-play "can this be voiced" query. `game::preload_pool` and `native_audio::prefetch` swallow errors. Statically checkable per word: `human_audio::clip_url`, `audio_verdict::servable`; device TTS cannot be checked. There are several audio paths, not one resolver; CC-BUILD219-FIXES.md is not in the repo. I10 (no board shown without audio in hand) needs new resolver work that this file says it must not add.

## Questions for Eric

1. **English Expert (323 < 400):** lower the Spelluzzle Expert length range, accept English as offered through Hard only, or wait for bank growth (the CC-BANK-REBUILD work)?
2. **Easy outside English:** Easy banks are 247–279 rows, so every non-English Easy is absent until CC-BANK-REBUILD grows them. Is "English only at launch" acceptable?
3. **C7:** run the full 10,000/tier, and fix the Jr cap failures (G5 under small pools) first? Replace the fixtures?
4. **History owner (C9):** which single mechanism owns Spelluzzle's history, and is it a new key or the F6 ledger?
5. **Audio (C10):** I10 and A17 need a prefetch-with-result path the audio resolver doesn't have. Who builds it, and does that count as amending the resolver (the spec forbids it)?
6. **O1–O3,** defaults still stand: app-only; no "can't hear it" rescue; 🗝️ (free, per C11).
7. Missing spec files (CC-ONBOARD-JR, CC-BUILD219-FIXES, CC-SPELLDOKU-RULES, CC-SENSE-CUE, CC-HUB-NAV as files) are cited by this spec; the standing rule says stop and ask if a named file is missing.


---

# Appendix: c1-c2-c9-c10-c11

# Spelluzzle census C1/C2/C9/C10/C11 (worktree spell-wasm-spelluzzle-census, c052f07c)
Paths relative to the worktree. Missing spec files (not tracked): docs/CC-ONBOARD-JR.md, CC-BANK-PURITY.md, CC-BANK-REBUILD.md, CC-BUILD219-FIXES.md, CC-SPELLDOKU-RULES.md. Only docs/onboard-jr-inventory.md, docs/CC-SPELLDOKU{,-v1.2,-v1.3}.md, docs/CC-PERSIAN-FOUNDATION.md, docs/CC-WORDGRID.md exist. reports/bank-purity-census.md and reports/build219-forensics.md exist; reports/bank-rebuild-census.md is not on main (only on branch cc-bank-rebuild-census).

## C1 Mode registry and drawer
- ONE list. config/modes.json (536 lines, 25 entries) is bundled via build.rs into OUT_DIR/modes.json and included at src/modes.rs:157; `modes::all()` (src/modes.rs:162) is the only parser. Drawer, Play hub, experience::policies (src/experience.rs:80), hub_tiles (src/hub_tiles.rs:55,102,116) all call `modes::all()`. scripts/modes-check.mjs and tests/e2e/specs/drawer.mjs also read the JSON.
- `group` EXISTS: required, no serde default (src/modes.rs:86); enum Group {SpellIt, WordPuzzles, Meaning, YourWords, Unlisted} (src/modes.rs:96-112). A mode without group does not parse (test src/modes.rs:558).
- `platforms` EXISTS: Vec<String> "ios"/"web" (src/modes.rs:58); enforced in `permitted` (src/modes.rs ~245: platform = native ? "ios":"web"). Also: status, kidSafe, entitlementLevel, requiresPremium, languages, exitStyle, juniorPolicy (required), hubTile.
- "Word puzzles" group EXISTS: i18n key hub.group.word_puzzles = "Word puzzles" (src/i18n/locales/en.json:280; other locales too); mapped at src/drawer.rs:63. Members now: letter_forge, word_chains, spelldoku, spell_search, spell_cross (5 live).
- Drawer row building:
  - src/drawer.rs:62-64 PLAY table: (SpellIt,"hub.group.spell_it"),(WordPuzzles,...),(Meaning,...) = signed order (test src/drawer.rs:1240).
  - drawer::render (src/drawer.rs:607): `modes::all()` -> `play_hub::ctx(app)` (src/play_hub.rs:136; builds HubCtx kid/native/lang/level/premium/enabled flags) -> `modes::catalog(&all,&ctx)` (src/modes.rs:267; `permitted(..., Surface::Catalog)`: flag on, kid-safe, platform, languages, entitlement, premium) -> `play_sections` (src/drawer.rs:91-123: filter by group, then `play_hub::unavailable_reason(m,lang).is_none()` (src/play_hub.rs:218), map to Row {id, icon, name_key, target}) -> `body()` (src/drawer.rs:426) -> `row_html` (src/drawer.rs:398). YourWords rows: `your_words_rows` (src/drawer.rs:140).
  - Row target: `route_for` (src/drawer.rs:355) / `target_for` (src/drawer.rs:330); launch ids in src/play_hub.rs `launch_for` (:103), test launch_table_covers_every_registered_mode (:278). Per-mode playability gates in play_hub.rs: spelldoku_playable (:184), spell_search_playable (:194), spell_cross_playable (:204).
  - Gate: `crate::flags::is_on(id)` (src/flags.rs) -- a new mode needs a flag id equal to its registry id.
  - A new mode also needs: nameKey/descKey in all 13 locales (src/i18n/locales/*.json), juniorPolicy, group, icon, and an entry in play_hub LAUNCH.
- Icon dedupe: src/drawer.rs:127 `dedupe_icon` (name that already starts with its own glyph).

## C2 CC-ONBOARD-JR Phase A resolver: MERGED
- src/experience.rs: `resolve(kid, age_locked) -> Resolved{experience: Junior|Standard, locked, source}` (:60); `of_kid(kid)` (:73); `allowed_tiers(exp, mode_id) -> Vec<&str>` (:86) = tier ceiling from modes.json `juniorPolicy` (ceiling:<tier> | variant:<id> | hidden | ungated; unknown mode fails closed to Easy+Medium). TIERS const :28. JR_CLIMB_EASY_WORDS=20 :31.
- Also: drawer.rs:183 `account(signed_in, jr, name)`, :257 `nudge_allowed(exp, age_known)`; src/game.rs:902 selector offers what resolver allows (test :4284). Phase A noted started in docs/onboard-jr-inventory.md:~319. Spec file CC-ONBOARD-JR.md itself: not found.
- New mode must carry juniorPolicy; policy for puzzle modes today: spell_search/spell_cross ceiling:easy; spelldoku/letter_forge/word_chains ceiling:medium.

## C9 No-repeat / freshness stores
| Store | Key | Where | Status |
|---|---|---|---|
| Wordgrid ledger (word-level) | spell_wordgrid_seen_v1 | src/wordsearch_ui.rs:23; src/wordsearch/ledger.rs (WINDOW_PUZZLES=20 :10, WINDOW_DAYS=14 :11, DAILY_WINDOW_DAYS=90 :14; `Ledger{counter,seen[{k "lang:band",w,n,d}],relaxations}`; record/record_many/select) | LIVE, shared by Spell Search, Spell Cross, SpellDoku Word Mode (docs/CC-SPELLDOKU-v1.3.md:288-296). Device-level, no profile. |
| Spell Search board hashes | spell_ws_daily_seen_v1 (+ spell_ws_daily_words_v1) | src/wordsearch_ui.rs:198-215 `remember_puzzle` cap 500; WS_SEEN :213 | LIVE, Daily path (serve_daily :221-257) |
| Spell Cross board hashes | XW_SEEN (spell_xw_*), spell_xw_daily_words_v1 | src/wordcross_ui.rs:155-175, cap 500 | LIVE |
| SpellDoku canonical board hashes | spell_spelldoku_seen_v1 | src/spelldoku_ui.rs:26,202-277; play::NO_REPEAT_BOARDS=500 (src/spelldoku/play.rs:39); canon::hash | LIVE (F8 D-R14: last 500 boards, replaced 14-day window). CC-SPELLDOKU.md F8 :206, I6 :233 (365-day window originally). CC-SPELLDOKU-RULES.md: not found. |
| Decks | spell_decks_v1 | src/model.rs:154; read src/lib.rs:229; written src/game.rs:1723 | NOT an exposure window; saved word decks. |
| Solo selection | spell_served_v1 | src/selection.rs:90 (push_served :101, called only from pick_solo :193); pick_solo (:168) has no caller outside selection.rs/tests | DEAD (never written in prod). spell_seltrace_v1 (:~103, 200 entries) is written via note_outcome (game.rs:2087,2278,2699). |
| CC-PERSIAN-FOUNDATION F6 | none | docs/CC-PERSIAN-FOUNDATION.md:22 (D5), :90-100 (F6), :113 simulator, :129-138 conflict w/ small easy pools | SPEC ONLY, no code, no freshness simulator. Wants per (mode,language,tier) per-profile ledger, window W from pool floors. Queued, not started (:3). |
| CC-BANK-REBUILD shared history | n/a | reports/bank-rebuild-census.md (branch cc-bank-rebuild-census) C9 section lines 69-88 | NOT BUILT. Census says F8 "one history keyed by mode,language,tier, per profile" text not in repo; would need to absorb spell_wordgrid_seen_v1 and remove dead spell_served_v1. |
- In-session only, nothing persisted: Bee (src/bee.rs:136), Word Chains (src/chains_screen.rs:49), Impostor, Def Match (src/defmatch_screen.rs:92) (per rebuild census).
- Profile scoping: NONE of these keys include a profile id (rebuild census; src/storage.rs has no profile prefix). entitlements.rs:101 has `multiple_profiles` bool (default false :134), no profile-scoped storage exists.
- Could more than one own Spelluzzle's history (500 board hashes per profile+language)? YES, three candidate owners exist today with the same 500-cap hash pattern: wordsearch_ui (spell_ws_daily_seen_v1), wordcross_ui (XW_SEEN), spelldoku_ui (spell_spelldoku_seen_v1), plus the shared word-level ledger spell_wordgrid_seen_v1, plus a would-be unified CC-PERSIAN-FOUNDATION F6 / CC-BANK-REBUILD history. All are device-level (not per profile) and mostly per mode, not per language (hash lists are language-agnostic keys except ledger "lang:band"). A Spelluzzle board-hash list would be a 4th copy of the pattern; spec must name ONE owner and say whether it is a new key or folds into the F6 per-profile ledger. Note the shared word ledger doubles as a conflict: Spelluzzle words (if bank words) would also be subject to the 20/14/90 window already shared by 3 modes (v1.3 doc warns English Easy 764-word bank exhausts).

## C10 Audio resolver
- Single router for words: src/api.rs. `play_word` (:201) / `play_word_with(word, py, variant, rate, lang, on_fail)` (:233) -> `play_chain` (:261) iterating `source_order()` (:98; default Human, Pack, ServerCache, NativeTts; override localStorage spell_audio_src via parse_source_order; zh drops NativeTts :241). Each source: play_human (:336, human_audio::clip_url), play_pack (:313, packs::src_for), play_server_cache (:397; /api/speak URL via speak_url :165; native_audio::play_word or HtmlAudioElement), play_native_tts (:433). FAIL verdicts skipped before play (:283-290, audio_verdict::servable, src/audio_verdict.rs:92).
- API is fire-and-forget, SINGLE word, callback only on total failure: `on_fail: FnOnce()` runs only after every source failed (play_chain None branch -> set_source("none") ). No success callback, no promise. Outcome observable only after the fact: `api::last_audio_source()` (:25) and telemetry observe_resolution (:~30-50). There is NO pre-play "can this word be voiced" query and no per-word result.
- Preload/caching that exists: `api::preload_word(word, lang)` / `preload_word_with(word, py, lang)` (api.rs ~:567-598): fire-and-forget, skips words with a human clip, native -> native_audio::prefetch (src/native_audio.rs:59, JS SpellAudio.prefetch audio-native.js, which resolves success AND failure to undefined: `.then(function(){}, function(){})` ~:237 so errors are swallowed), web -> `fetch` to warm HTTP cache (result discarded). game::preload_pool (src/game.rs:152, PRELOAD_AT_OPEN=20) and warm_target/voice_target (:~1140-1180). JS side: MAX_PRELOADED=8 LRU of native-player preloads (audio-native.js:95), on-disk Directory.Cache files, ensureCached/ensurePreloaded single-flight maps (audio-native.js:135,192). Packs: src/packs.rs (offline pack). Human clips: src/human_audio.rs (clip_url :60, bundled, synchronous lookup, per-word existence knowable offline). audio_verdict::servable is sync per (lang,word,variant) -> usable as a static pre-check for Human/Pack/Server clips (not for device TTS). Service worker sw.js caches only the static shell, not audio.
- Server: backend/app.py:670 /api/speak (long-lived Cache-Control). No batch endpoint (grep: none).
- Several paths exist beyond the router: (1) `api::play_device_tts` (:426) on-device only (My Words OOD imports, COPPA), (2) `speech_out::speak(text, rate, code)` browser SpeechSynthesis (src/speech_out.rs:73, 100 lines, fire-and-forget, no result except noVoice message) used as last resort/fallback closures in game.rs:1184+, (3) native_lang::speak / speak_syllables (src/native_lang.rs:66,86) AVSpeech via plugin returning Promise, (4) api::play_sentence_audio (:652), (5) callers wrap play_word (bee_screen.rs:248-285 routed through resolver per CC-AUDIO-CLARITY; spelldoku_ui.rs:600-615; wordcross_ui.rs:375; impostor_screen.rs:156). So: one word-router (api::play_chain) with several non-router siblings. CC-BUILD219-FIXES.md: not found (referenced src/audio_verdict.rs:11, reports/build219-forensics.md exists). CC-SPELLDOKU-v1.2.md:139 item 3 asked the same "no audio for this item" question.
- Gap for Spelluzzle: no set prefetch, no per-word success report, no "no audio" state to branch on before play. Needs a new API (e.g., async `resolve_set(words) -> Vec<(word, Source|None)>` resolving the same order incl. audio_verdict + human_audio + prefetch promises) or a pre-check using human_audio::clip_url + audio_verdict::servable + a fetch/HEAD of speak_url; native prefetch currently swallows errors.

## C11 Key emoji and mode row emoji
- U+1F5DD key (🗝️): NOT used by any mode row, config/modes.json, src, index.html or i18n. Only occurrence in repo: config/wordpic/pictures.json:148 (id "ankh", WordPicture icon, not a mode).
- Mode row icons (config/modes.json), all unique; group in parens:
  practice 🌱 (spell_it); ghost_racing 👻 (unlisted/hidden); syllable_replay 🔉 (unlisted/hidden); photo_list 📷 (your_words); spell_aloud 🗣 (unlisted/hidden); word_stories 📖 (unlisted/hidden); online_spelloff ⚔️ (spell_it/hidden); def_match 🎴 (meaning); letter_forge 🔨 (word_puzzles); word_chains 🔗 (word_puzzles); impostor 🕵 (meaning); bee_sim 🐝 (spell_it); word_picture 🖼️ (meaning); my_words ＋ (your_words/core); misses ↻ (your_words/core); reports 🗺️ (your_words); calendar 🗓️ (your_words); translate 🌐 (your_words); spelldoku 🔢 (word_puzzles); spell_search 🔎 (word_puzzles); spell_cross ✚ (word_puzzles); boardgame 🎲 (spell_it); standard 🔊 (unlisted/core); climb 🏔 (spell_it/core); daily 🗓 (spell_it/core); versus ⚡ (spell_it/core).
- Collision notes: calendar 🗓️ (U+1F5D3 U+FE0F) vs daily 🗓 (no VS16) are visually the same glyph. Account row uses 👤 (drawer.rs, not a mode).

---

# Appendix: c3-c6

# Spelluzzle census C3-C6 (read-only, worktree spell-wasm-spelluzzle-census, origin/main c052f07c)

## Premise correction: the banks are NOT pre-sweep
CC-BANK-PURITY Phase A (commit 3e54e8d6, 2026-10-07, "quarantine sweep, 9,340 rows") IS an ancestor of origin/main c052f07c. The banks measured here are post-Phase-A. The quarantine filter therefore removes 0 further rows (reports/bank-purity-rows.csv, quarantine classes, matched against the current banks: 0 hits in the 10 Latin/Cyrillic languages). Branch cc-bank-purity-census (5bbf8f98) differs from main only by one test-golden commit plus main being ahead; no bank text differs. If "the swept bank" means some later sweep, it is not in any ref I can see. Residual purity risk is the ledger `pending` rows (assets/words/purity/<lang>.tsv), reported per cell below; fil is heavy (969 pending bank rows).
Script: /private/tmp/claude-501/-Users-eric/8254f994-5f46-4b6b-ae7a-7d6b37e79fca/scratchpad/spz/census.py (output census.out).

## C3 unit model
Registry: src/consts.rs BUILTIN_LANGS = 15 langs (en es fr de pt pl vi ko ja fil zh ru ar sw hi), (code, name, status, direction) only. There is NO script field; script is only in comments. Derived: Latin = en es fr de pt pl vi fil sw; Cyrillic = ru; excluded = ko ja zh ar hi (fa has a bank + keyboard but is not in the registry). All Active in the app; the web build is English only (is_active_lang, `web` feature), so web Spelluzzle = en only.
Base match key (src/norm.rs): fold_strict = NFC + Unicode lowercase, whitespace/Arabic marks/ZWNJ dropped, accents KEPT (normal play). fold_lenient (Spell Jr / Kid Mode only) = NFD, strip ALL combining marks, plus ß->ss, æ->ae, œ->oe, ł->l, ø->o, ı->i. No ё/е fold in the match key (ru: ё is distinct from е in strict play; typing е for ё is only accepted where the pair is in ru/homophones.txt via homophones::accepts). Case is folded, so de capitalised nouns are fine.
Keyboards (assets/keyboards/*.json == src/keyboard.rs): a unit = one char of the folded word; every char of every bank word in en es fr de pt pl ru fil sw is reachable on the in-app keyboard (0 unreachable char types). Reachability is base key OR long-press variant:
- en, sw, fil(+ñ key, hyphen key): plain keys. fil/sw/en need no long-press.
- es: ñ is a base key; á é í ó ú ü are long-press. fr: é è ê ç î ô œ û à ï â... long-press (11 accented types in the bank). de: ä ö ü are base keys; ß is long-press on s (96 bank rows). pt: ç base; ã á í õ ê é ó ú â ô long-press. pl: ALL 9 diacritics long-press (ł 900 rows, ą 461, ó 421, ę 402, ś 352, ż 268, ć 257, ń 101, ź 36). ru: ЙЦУКЕН, ё is long-press on е (42 bank rows).
- Digraphs: none have keys. es ch/ll/rr, fil ng, sw ch/sh are typed as 2 letters = 2 units = 2 runes (consistent with one rune = one key). sw ng' and fil mag-aaral are removed by the no-apostrophe/hyphen gate. fr œ and de ß are ONE unit and ONE long-press key (ß folds to ss = 2 units in Jr).
- CAVEAT for "one rune = one key": accented units are one long-press SELECTION on a base key (2 gestures, 1 character). Pass if long-press variants count as keys; if the cipher needs a visible key per rune, es/fr/pt/pl/ru(ё) need those long-press variants surfaced.
- Jr (lenient) caveats: units are the accent-stripped letters, so the Jr rune alphabet is smaller and rune-to-key is trivially one base key; but ß=2 units (de), ё->е and й->и (ru: NFD strips the breve, so Jr ru treats й as и), ñ->n, ł->l, ą->a etc. A Jr cipher on the stored word would show different unit counts than the lenient match key; compute Jr lengths on the lenient fold (done below).
- vi: FAILS C3. Bank has 60 char types the keyboard rows/long-press cannot produce (á à ạ ả ế ệ ộ ố ờ ự ậ ấ ...): tones are applied post-fix by a tone row (src/viet.rs), so a toned letter = base key + tone key, not one key. Bank is also space-separated syllables. CC-WORDGRID E2 already ruled vi out for the same reason (20.7% E2 fail). vi Jr only "works" because lenient fold strips tones and đ stays, which makes unrelated words collide; do not offer.
C3 verdict: PASS en sw fil (plain keys); PASS-with-long-press-caveat es fr de pt pl ru; FAIL vi.

## C4 eligible counts (G1), floor 400
Method: bank rows of assets/words/<lang>/<tier>.txt (identical to the word_data.rs constants), dedupe, strict fold (Jr: lenient fold on Easy union Medium, kid-exclude applied as kid_filter does), all chars alphabetic and on the keyboard, length in cells within tier range, minus profanity/<lang>.txt + exclusions/<lang>.txt + exclusions/_roots.txt (lenient exact / root substring), minus kid-exclude (Jr), minus audio Fail (config/audio-verdicts.json; "Fail" is the only withheld verdict, src/audio_verdict.rs), minus quarantine (0 left, see above). The in-code layers of src/profanity.rs (BANNED_ROOTS, RU stems) were not replicated; the files were. sw has no profanity file (E7), only a kid-exclude list.
Audio caveat: audio-verdicts.json `measured` is EMPTY. Full-bank verdicts exist only for en (3,165 clips); de fil hi ko ru vi have ~60-clip samples; es fr pt pl sw have none, so their audio filter removes nothing and the counts there are upper bounds.

| lang | Jr (E+M, 3-6) | Easy 3-6 | Medium 4-8 | Hard 5-9 | Expert 6-9 | cells below 400 |
|---|---|---|---|---|---|---|
| en | 1445 | 717 | 783 | 773 | **323** | Expert |
| es | 986 | **238** | 829 | 1568 | 1394 | Easy |
| fr | 999 | **236** | 833 | 1563 | 1392 | Easy |
| de | 1035 | **242** | 896 | 1482 | 1201 | Easy |
| pt | 1029 | **240** | 856 | 1517 | 1347 | Easy |
| pl | 1036 | **218** | 904 | 1550 | 1463 | Easy |
| ru | 1060 | **219** | 892 | 1584 | 1492 | Easy |
| fil | 782 | **210** | 577 | 866 | 690 | Easy |
| sw | 908 | **242** | 672 | 1034 | **284** | Easy, Expert |
| vi (fails C3) | 1062 (tone-stripped, meaningless) | 132 | 118 | 1 | 0 | all but Jr |
ENGLISH IS BELOW FLOOR AT EXPERT: 323 eligible (797 rows, 470 dropped by the 6-9 length gate, 4 audio Fail). That is the only English shortfall; Easy 717, Medium 783, Hard 773, Jr 1445 are fine. Easy is structurally short everywhere else: every non-en Easy bank is only 247-279 rows BEFORE any gate (same finding as docs/census/wordgrid-census.md E4), so no non-en language can offer Easy at floor 400 without growing the bank. sw Expert bank is 683 rows (sw is a structural 3-tier language per config/bank_floors.json), so its Expert cell is short by bank size.
Gate losses: not-units (space/hyphen) vi 4,000+, fil 26 (hyphens); length gate dominates Expert (en 470, es 1235, fr 1161, de 1447, pt 1219, pl 1321, ru 1391, fil 763, sw 399 of 683). Profanity/exclusion removals are 0-4 per tier (Jr 2-16, en Jr 16). Audio Fail removals: en Easy 18, Medium 5, Hard 2, Expert 4; fil Easy 3, Hard 1; de Medium 3; ru 1 per tier; vi small. Ledger-pending rows still inside the eligible set (purity risk): fil Medium 83, Hard 288, Expert 236, Jr 84; sw Hard 45, Expert 18; ru Expert 29, Hard 11; es Expert 23; fr Expert 15; de Expert 13; others <=8; en 0.
Jr resolver: Jr tier = Easy then Medium (experience.rs jr_climb_tier), served through game::active_word_list -> kid_filter::filter_kid (kid-exclude lists, lenient fold; never empties a pool). There is no separate Jr bank file.

## C5 CC-SENSE-CUE collision tables
Exist for en, es, ru only (tools/build_collisions.py handles exactly en/es/ru; output assets/words/<lang>/homophones.txt; consumed by src/homophones.rs, which shipped via include_str!). Sizes: en 263 sets / 615 member words; es 11 sets / 22 members; ru 15 sets / 30 members. config/sense-cue.json lists en es ru pl de fr pt ko, all cueMode "off" (no named auditor), but pl de fr pt ko have NO table; fil, sw, vi are absent from both.
Eligible words that are collision members (covered / eligible): en Easy 182/717, Medium 74/783, Hard 17/773, Expert 6/323, Jr 252/1445. es 3/238, 4/829, 4/1568, 0/1394, Jr 7/986. ru 3/219, 13/892, 6/1584, 2/1492, Jr 14/1060. All other languages: 0 (no table).
Reading: a table is complete-by-construction over the bank (rule-derived), so "covered" is just the colliding subset. en/es/ru are CHECKED; fr de pt pl fil sw are UNCHECKED (collisions such as fr homophones, de final devoicing Rad/Rat, pl ó/u rz/ż ch/h, pt s/z/x, fil are real but undetected). C5 PASS: en es ru. FAIL (no table): fr de pt pl fil sw (vi out anyway).

## C6 validity list (bank + >=20,000 external; CC-WORDGRID E6)
The E6 gate text: "a word list large enough to prove a decoy is not a real word; bank plus an external list of at least 20k entries." In the app only the bank ships as a word set; en also ships assets/wordsearch/en-decoys.txt (2,008 lines, a proven-non-word decoy table, not a validity list). NO validity list ships to the device for any language. All external lists below are build-time, gitignored, Mac-local:
- Hunspell dicts (LibreOffice set), ~/repos/spell-wasm-purity/.corpus-cache/dicts, STEM counts (affix-expanded forms are far more; /opt/homebrew/bin/hunspell is installed and tools/bank/purity_check.py already drives it): en_US 49,568; es_ES 58,221; fr 84,782; de_DE_frami 258,200; pt_BR 312,368 (pt_PT 44,476); pl_PL 348,892; ru_RU 146,269; sw_TZ 67,900; also ar 465,928, hi 207,694, ko 101,598. NONE for fil or vi.
- en: /usr/share/dict/web2, 235,976 entries (macOS system file, no plurals; WORDGRID ruled it "weak" on its own); tools/wordpipe/sources/freq_en.txt 50,000 (gitignored).
- Leipzig Wikipedia 100K corpora, ~/repos/spell-wasm/.corpus-cache/*_wikipedia_2021_100K.tar.gz (same source family the banks were built from, corpus-attested NOT dictionary-vetted; includes typos, proper nouns, loanwords). Lowercase-alpha word types / freq>=2: en 46,177/27,983; es 65,825/35,659; fr 51,188/29,541; de 37,842/17,201; pt 59,139/32,868; pl 104,561/51,530; ru 120,249/56,610; fil (tgl) 64,745/29,949; sw 68,057/27,128; vi 11,961/4,887.
E6 status by language: PASS (dictionary-grade >=20k): en, es, fr, de, pt, pl, ru, sw. WEAK (corpus-only, no dictionary): fil (29,949 forms at freq>=2, clears 20k numerically). FAIL: vi (no dictionary, 4,887 forms; also out on C3). This reverses the WORDGRID census (Sep 18: "no external list for es/ru"): the Hunspell dicts landed Oct 7 for the purity work. But licences: assets/words/LICENSES.md says per-language Hunspell licences vary (GPL/LGPL/MPL/BSD) and data/LICENSES.md makes any new wordlist source a sign-off stop gate; none of these dicts is shipped or licence-cleared for shipping.
Consequence: a device-side "is this decode a real word?" check can only use the bank (post-sweep ~2.7k-6k rows per language), which cannot prove a non-bank string is not a word. Any Spelluzzle rule that depends on proving non-wordhood (alternate-decoding uniqueness, decoy runes forming words, ambiguity checks) must be done at BUILD time against the Hunspell/web2 lists and the result shipped as a table (like en-decoys.txt), which needs the licence sign-off for the languages used; or restricted to bank-only validity. fil and vi cannot do it to dictionary grade.

## Final table (C7 pending)
| lang | script | C3 | C4 tiers at floor 400 | C5 | C6 consequence |
|---|---|---|---|---|---|
| en | Latin | pass | Jr, Easy, Medium, Hard pass; **Expert 323 FAIL** | pass (263 sets) | full E6 (web2 + hunspell 49.6k); build-time proof possible; only en has a shipped decoy table |
| es | Latin | pass (long-press caveat) | Jr, Med, Hard, Exp pass; Easy 238 fail | pass (11 sets) | hunspell 58k stems, build-time only; licence sign-off needed to ship derived data |
| fr | Latin | pass (caveat) | Jr, Med, Hard, Exp pass; Easy 236 fail | **no table** | hunspell 84.8k, build-time only |
| de | Latin | pass (caveat; ß=ss in Jr) | Jr, Med, Hard, Exp pass; Easy 242 fail | **no table** | hunspell 258k, build-time only |
| pt | Latin | pass (caveat) | Jr, Med, Hard, Exp pass; Easy 240 fail | **no table** | hunspell pt_BR 312k, build-time only |
| pl | Latin | pass (caveat; all diacritics long-press) | Jr, Med, Hard, Exp pass; Easy 218 fail | **no table** | hunspell 349k, build-time only |
| ru | Cyrillic | pass (ё long-press; Jr й->и, ё->е) | Jr, Med, Hard, Exp pass; Easy 219 fail | pass (15 sets) | hunspell 146k, build-time only |
| fil | Latin | pass | Jr 782, Med 577, Hard 866, Exp 690 pass; Easy 210 fail; heavy ledger-pending (Hard 288) | **no table** | WEAK: Leipzig-only, no dictionary; cannot prove non-words to dictionary grade |
| sw | Latin | pass | Jr 908, Med 672, Hard 1034 pass; Easy 242 and Expert 284 fail | **no table** | hunspell 67.9k, build-time only; no profanity list (E7) |
| vi | Latin | **FAIL** (tones need base+tone press) | not offerable | none | FAIL: no dictionary |
Not offered by rule: ko zh ja ar hi (and fa, unregistered).
Clean three-way pass today (C3+C4 at the tier+C5): English Jr/Easy/Medium/Hard only. es and ru pass C3+C5 and C4 for Jr/Medium/Hard/Expert (Easy short). Everything else lacks a collision table. Easy is the structural gap for 9 of 10 languages.

---

# Appendix: c7-c8

# Spelluzzle census C7 + C8 (worktree c052f07c, read-only, no network)
Files: spz/spz.py (checker G1-G11, Fractions), gen.py (generator), ana.py, fix.py, main_web2.jsonl (12,500 rows), sens_hun.jsonl (2,500 rows), c8/kb.html.

## Inputs
- Bank: assets/words/en/{easy,medium,hard,expert}.txt (763/798/800/797 rows). Jr = easy+medium.
- Collision table: assets/words/en/homophones.txt (266 CMUdict-derived sets, generated by tools/build_collisions.py; CMUdict itself is at tools/wordpipe/sources/cmudict.dict). Used the table directly, not re-derived.
- Validity list (STAND-IN): bank (all tiers) + hunspell en_US.dic lowercase-alpha stems + /usr/share/dict/web2 + homophone-set members = 217,720 words. No inflections beyond what the bank holds. Sensitivity list without web2: 39,375 words.
- Units = single letters. Gate readings: G9 applies to silent words only; G10 = sum_r(spoken cells of r / spoken words containing r) / spoken cells; G11 = mean over spoken words of (other-word cells whose rune is in that word / other-word cells), "others" includes silent and secret.

## Generator (what "attempt" means)
One attempt = one randomized board construction (first word uniform from the tier pool; each next word uniform-weighted among pool words that keep union <=16 runes and share >=2 runes with the union, weighted (1+k)^3 where k = number of spoken runes still in only one word that the candidate covers), then ALL gates G1-G11 via the independent checker. The G5 bias is my design; an unbiased pick (5 seeds/tier test) needed ~600-2000+ draws and often hit the cap, so the spec's desk-model attempt counts are only reachable with a G5-aware picker. Cap 2000 draws. Seeds: random.Random("<tier>:<seed>"), seeds 0..N-1. Spoken/silent/secret roles are assigned in draw order.

## What I ran
Main: 2,500 seeds per tier x 5 tiers = 12,500 boards (not 10,000: 5,000/tier was projected at ~40 min, so I cut it), 10 processes, web2-inclusive validity list, ~4,570 cpu-s, ~14 min wall. Sensitivity: 500 seeds/tier with the 39k hunspell list.
Times are this Mac (10 cores, Python 3.9, 10 procs in parallel). The iPhone p95 cannot be measured here; Rust/WASM on device will differ, treat these as relative only.

## Results (2,500 seeds/tier, web2 list)
| Tier | success | attempts p50/p95/max | desk model p50/p95/max | s/board p50/p95 | runes avg/max | earned p10/p50/p90 | cascade p10/p50/p90 |
|---|---|---|---|---|---|---|---|
| Jr | 2494/2500 (99.76%; 6 hit cap) | 239/991/1973 | 49/179/481 | 0.84/3.44 | 9.72/13 | .465/.519/.571 | .321/.395/.484 |
| Easy | 2500/2500 | 81/336/960 | 26/113/314 | 0.17/0.71 | 8.97/11 | .438/.475/.526 | .276/.340/.415 |
| Medium | 2500/2500 | 44/189/779 | 46/176/496 | 0.16/0.71 | 14.22/16 | .434/.478/.528 | .293/.346/.408 |
| Hard | 2500/2500 | 24/105/291 | 44/196/465 | 0.084/0.36 | 15.68/16 | .451/.500/.552 | .357/.419/.480 |
| Expert | 2500/2500 | 33/144/495 | 39/162/376 | 0.047/0.21 | 15.86/16 | .481/.554/.630 | .415/.488/.562 |
(Attempts quoted over successes; the 6 Jr cap-hits make the all-seed Jr p95 1006, max 2000.)
- Jr and Easy are 2-5x worse than the desk model on attempts (Jr p50 239 vs 49, Easy 81 vs 26); Medium/Hard/Expert match or beat it. Jr/Easy attempts are dominated by G5 rejects (788k of ~833k Jr rejects); the small-word pools make "every spoken rune in >=2 words" the binding gate. A better G5-aware picker would probably close this; I did not tune further.
- Cheap-gate reject mix (all draws): Jr G5/G6a/G6b; Easy G5/G6a/G6b/G3; Medium G5/G6a/G6b/G7/G8/G9; Hard G5/G6b/G6a/G7/G8; Expert G6b/G5/G6a/G7/G8/G9. G8 rejects: Easy 12, Medium 646, Hard 220, Expert 48 (of all draws); G9: Medium 183, Hard 79, Expert 22. So uniqueness and solvability are not vacuous but are rare rejections.
- Earned share p50 .475-.554 (desk ~.48-.62: Easy/Medium a touch low, Hard/Expert in range). Cascade p50 .340-.488 (desk ~.34-.41: Expert is above it, ~.49).
- Rune counts: Medium/Hard/Expert run at the 16-cap on average (14.2/15.7/15.9); G2 is tight there, Jr/Easy never get near it.
- Sensitivity (39k list, 500 seeds/tier): 100% success all tiers; attempts p50 Jr 224, Easy 82, Medium 38, Hard 23, Expert 32 (p95 992/343/175/103/139). G8 rejects drop ~3x (Medium 71/500 seeds-worth vs 646/2500), i.e. a bigger validity list is stricter but doesn't move success. Caveat: neither list includes inflections (plurals, -ed, -ing) beyond the bank, so a real validity list with inflections would make G8/G9 stricter than measured here; rerun with the real list before trusting the G8 numbers.

## Fixtures (G1-G11, strict, against real bank / collision table / both validity lists)
Both FAIL G1 (bank band); neither word list matters.
- fixture-easy-1 (global worst mobile agree mirror | waste): Easy band contains none of them. global and mobile are in medium.txt only; worst, agree, mirror, waste are in NO bank file (not in src/word_data.rs either). First failing gate G1.
- fixture-medium-1 (glucose neutral blank castle battery | geometry | mock): none of the 7 words is in any bank file (checked all four bands and word_data.rs). First failing gate G1.
- Diagnostic with the band check disabled only (everything else unchanged): both PASS G2-G11. Easy: 12 runes, 33 cells, earned 41/84 (.488), cascade 1241/3780 (.328). Medium: 15 runes, 44 cells, earned 43/96 (.448), cascade 18541/45695 (.406). Rune/cell counts match the expected 12/33 and 15/44. G8 unique and G9 solvable on both validity lists. None of the fixture words is in a collision group.
Fix options: add the 11 missing words to the bank bands, or have fixtures declare that G1's band rule is waived for them.

## C8 layout at 375x667pt (computed from index.html CSS + measured in browser pane on a static page using the app's real <style> blocks; the wasm app was not built, dist-test-web has no pkg)
Measured keyboard heights at 375 wide (excluding margins):
- In-app main keyboard (.game-kb, keys min-height 46, 3 rows + delete row, gaps 7): 207 pt, plus margin-top 14 = 221. Key width 32.1.
- Compact mode-screen keyboard (.sd-keys at max-width:480px, the one Spell Cross/Spell Search/Spelldoku use, keys 40pt, 3 rows + clear/delete row): 172 pt (matches the 172 quoted in index.html's own comment), width 355.
- Safe area: keyboard has padding-bottom env(safe-area-inset-bottom) (0 on a 375x667 iPhone SE; ~34 on notch phones, not this viewport). Top inset 0 on SE, 20 if status bar overlays; I tested both.
- Header: .sd-top measured 44 high (the ghost buttons are 44) + 6 margin = 50. Note line 20.8, typed line 25.2, screen padding 8 top / 10 bottom, ws-wrap gaps ~5 x3.
Budget for the board with keyboard up, rune key hidden: chrome ~129 pt (pad 8 + header 50 + note 21 + typed 25 + gaps 15 + pad 10) +0/20 top inset.
- Vertical: compact kb -> grid height 366 (346 with 20 inset); main kb -> 317 (297). 7 rows with 3pt gaps: cell <= (H-18)/7 = 49.7 / 46.9 (compact), 42.7 / 39.9 (main).
- Horizontal: 9 cells in 355 pt (10 pt side padding), 3 pt gap: cell <= (355-24)/9 = 36.8 pt (37.2 with 8 pt padding, 37.7 with 2 pt gaps).
- Max cell that fits = WIDTH-bound 36.8 pt in every case (height has >=3 pt spare even in the worst case). >=32: YES. >=28: YES.
- Caveats: if each row also carries a left gutter (play button / role marker), usable width shrinks: 28 pt gutter+6 gap -> 33.0 pt, 44 pt gutter+6 gap -> 31.2 pt (fails 32, passes 28). Big Text/Kid on the main keyboard (keys 56 high, ~261 pt with margin) leaves grid height 277 -> height-bound cell 37.0, so still ~36.8 overall (zero spare; with a 20 pt top inset it drops to ~34). On a 320x568 (SE 1st gen): width-bound 30.7, still >=28 but <32.

---


## v1.1 decisions recorded (Eric, 2026-10-10: "The fixes seem reasonable")

Eric accepted the recommended fix for every v1.1 census finding. Recorded here so the build has a source:

1. **Expeditions (C15):** verify runs offline and ship the word indexes (about 2,000 runs per tier), so the device only picks one. Keep the 24-rune cap and make the guided generator part of the spec. Fallback if the run lists are not wanted: Medium and Hard only.
2. **My Words (C17, O5):** the bank match is computed when the board is built (`word_index::canonical`); no stored id. The kid filter is applied inside Spelluzzle for Jr profiles.
3. **Ripple sound (C19):** a small oscillator tick beside `audio_boost::chime()`, no asset file, following the silent switch.
4. **Duel NPC (C20):** extract a public `npc_accuracy_milli(variant, difficulty, hits, attempts)` and public clamp constants in the Board Game engine, keeping its golden digest. Another session is working in that module, so this edit is coordinated before it is made, at Phase H.
5. **Layout (C21):** one shared action bar (Listen, Spell unheard) above the keyboard replaces per-row Listen heads on Par and Duel; the glyph shrinks to 52% or less in a pencilled cell so an 11 pt mark fits; the rune strip is stacked (glyph over a count-and-letter caption, 44 pt) and hidden while the keyboard is up.
6. **Par (C14):** Expert Par requires par 3 (Hard stays par 2 or 3), re-measured when built. "Most cells first" means the Listen that decodes the most hidden cells.
7. **Accepted as is:** Easy Duel draws (up to 12.6%); Spanish Easy below the floor (Par and Duel Easy stay absent for it).
8. **C12:** the stale STOP line at the top of this report is corrected, and this section is the v1.1 sign-off record.

Next: Phase E (pencil runes, rune strip, ripple) when Eric says start.

# CC-SPELLUZZLE v1.1 census additions (run 2026-10-10, origin/main 27280a81)

**Read-only: no repo code changed.** Trials ran as throwaway ignored tests in scratch copies of `origin/main`, using the real v1 lexicon, `bank::load` and `gates::count_solutions`. English only. **STOP: no v1.1 feature phase starts until Eric has reviewed this section.**

## Summary

| Item | Result | Detail |
|---|---|---|
| C12 | pass, with gaps | v1 is in the repo and built (phases A to C, 2026-10-09). The v1 census section header still says "STOP", which is stale. The CC-SPELLUZZLE spec files themselves are not in the repo. v1.1 rev 2 signs D26 to D32, but no file in the repo records that sign-off. |
| C13 | pass | English clears 400 at every tier (lowest: Expert 529, Easy 533). ru clears every tier since its bank was regrown. **es Easy is 232, below the floor.** fr de pt pl fil sw vi have no collision table, so P1 cannot be evaluated for them. |
| C14 Par | pass, no stop | 10,000 of 10,000 at Hard and Expert; Mac time p95 5.1 ms and 3.3 ms (the iPhone cannot be measured here). **Expert is par 2 on 97.8% of boards**, so it plays no harder than Hard. "Most cells first" reaches par 47% to 48% if it means the longest word and 67% to 71% if it means the Listen that decodes most hidden cells; the spec's 74% matches only the second reading. Which reading is meant is Eric's call. |
| C15 Expeditions | **STOP (strict)** | Success 10,000/10,000 only with a "guided" generator (silent picks weighted toward runes not yet carried). **Unguided Expert is 8,920 of 10,000.** Runes per run have a median of 24 at Expert, the X1 cap, so it has no headroom. Expert redraws 72% of runs, and its Vault is a window of about 3 percentage points. Mac run time p95: 0.85 s (one thread) to 1.45 s (nine threads) at Expert; if the iPhone is about 4x slower that lands near the 3 s limit. The Mac was heavily loaded, so the times are pessimistic. |
| C16 Duel | pass, no stop | All 12 equal-NPC rows hold the leg-1 starter at 48.7% to 50.7%. Single-board starter edge 63.6% to 85.1%, which is why D27 chose two legs. Draws 2.6% to 12.6%; **Easy draws up to 12.6%**, above the desk 3.5% to 7.6%, because Easy boards are small. No double claims in 20,000 legs per row. |
| C17 My Words | **STOP** | A stored row carries no bank match or id (`ListEntry {text, lang, added_at, starred}`). The match would have to be computed at board-build time through `word_index::canonical`. That is feasible (app-only) but it is a decision, not an assumption. The base game does not apply `kid_filter` to My Words, so a Jr profile sees every list word; Spelluzzle would have to filter itself (this is O5). My Words is free (preview) and Jr policy is ungated. |
| C18 My Words | pass | A fair board for essentially every list of 5 or more words (0 to 0.8% unserved); size 3 is 0.9% to 5.6% unserved depending on the draw budget. Lists of 8 or more words average 5 list words per board. Jr is as good: 91% to 100% fair. |
| C19 Sound | **STOP** | There is no per-rune tick, no pitch ladder and no combo state anywhere. The only synthesized sound is a fixed 880/1320 Hz completion chime. A tick would be a new oscillator function (no asset file), but it is new sound code, not reuse. |
| C20 NPC reuse | **STOP** | The NPC accuracy is a method on `BoardGameState` and the clamp constants are private. Reuse as it stands means copying it, or a small visibility edit in `src/boardgame/engine.rs` (extract a public `npc_accuracy_milli`). There is no NPC turn length concept and no NPC names. |
| C21 Layout | **STOP (conditional)** | Cells never drop below 28 pt (minimum 32.9 pt at 10 cells). **A clean pencil mark is 9 pt on the v1 glyph**, under the 11 pt floor; 11 pt needs the glyph at 52% or smaller, or the mark replacing the glyph. **With the keyboard up, Par and Duel boards overflow** (the board scrolls 57 to 126 pt) because every word carries a Listen head. The measured fix is a single shared action bar above the keyboard. A 16-entry rune strip fits only stacked, scrolling, or hidden while typing. |
| C22 Long-press | note | Nothing binds long-press on Spelluzzle cells today. `user-select` is not set, so a long-press on a typed or decoded letter would start the iOS selection callout. A cell long-press needs `user-select:none`, `-webkit-touch-callout:none`, a contextmenu guard, click suppression after the hold, and a button route for VoiceOver and Switch Control. |

## What needs Eric

1. **C15, Expert Expeditions:** the 24-rune cap is binding at Expert and the generator needs guidance to pass. Options: raise the cap (needs more glyphs; the sheet has 28), drop Expert Expeditions, or accept the redraw load and measure on a device first.
2. **C17 and O5:** how a My Words row finds its bank match (compute at build time, or add a stored id), and who applies the kid filter.
3. **C19:** is a new tick function acceptable (D37 assumed an existing sound)? Or drop the ripple sound.
4. **C20:** edit the Board Game module (extract the accuracy function) or copy it?
5. **C21:** pencil mark design (shrink the glyph in marked cells, or replace it), and the shared action bar for Par and Duel.
6. **C14:** which "most cells first" reading is meant, and whether Expert par should be raised (97.8% are par 2).
7. **C16:** Easy Duel draws up to 12.6%. Accept, or score tie-breaks.
8. **C13:** es Easy is below the floor, so Par and Duel on Easy are not offered for Spanish.

## Notes on the data

- The earlier v1 trials' Python prototype directory was not available to the agents that ran C14, C15, C16 and C18. They rebuilt the large validity list: C14 used `/usr/share/dict/words` plus the bank with seedgen's suffix rules (2,586,198 strings, stricter than the Hunspell list); C15 and C16 used the Hunspell `en_US.dic` with the same expansion (469,516 words, the list `seeds-en.json` records). The C14 re-run on the Hunspell list is one environment variable (`PAR_VALIDITY_FILE`).
- Two rules in the spec were ambiguous and the Duel agent assumed readings: a wrong Listen marks the slot heard but leaves it open to Listen again; a wrong unheard spell leaves the slot unheard and open.
- C18 results depend on the generator's draw budget, which the spec does not state (the headline used 150 draws per list-word count and secret kind).


---

## v1.1 appendix: c12-13-17-19-20-22

# Census C12, C13, C17, C19, C20, C22  (origin/main 27280a81, worktree spell-wasm-spz11, read-only)

## C12 v1 in repo, signed, census reviewed
- In repo: src/spelluzzle/{bank,fresh,gates,gen,lex,mod,offer,play,render,seedgen,seeds,store,tests,types,view}.rs (2,764 lines), src/spelluzzle_screen.rs, assets/spelluzzle/{runes.svg,seeds-en.json}, tests/fixtures/spelluzzle/golden-en.json, tests/e2e/specs/spelluzzle.mjs, tests/ios-ui/flows/spelluzzle.yaml, reports/spelluzzle_census.md (297 lines), reports/spelluzzle-device-pass.md. Phases A, B, C landed 2026-10-09 (d61e03d9, bae81849, 14ac1fec; Maestro flow 4b235a50). Registry row config/modes.json "spelluzzle": status live, ios only, languages ["en"], entitlementLevel preview. Flag src/flags.rs:126 defaults ON.
- Signed: reports/spelluzzle_census.md:8-20 "Decisions recorded (Eric, 2026-10-09)", 9 items (English only Easy-Expert + Jr; Expert 6-10 cells; fix generator not gates; history store; audio prefetch amendment; O1-O3 defaults; later languages).
- GAPS (report these, none block):
  1. reports/spelluzzle_census.md:4 still carries "STOP - Phases A-D are blocked on Eric's review and on O1-O3". Stale: the decisions section below it answers it. Needs an edit when you append.
  2. The spec file itself (CC-SPELLUZZLE.md) is NOT in the repo (decision 8 says so; git ls-files finds none). v1.1 spec exists only in the excerpt/chat.
  3. Table at census.md:~230-243 (C4/C5 by language) predates the 2026-10-09 ru/ko bank growth (4303e52f); ru Easy there is 219, now 746 (see C13).
  4. Decisions cover v1 only. Nothing in the report records sign-off of v1.1 (F15 Par, F19 Expeditions, F20 Duel, F21 My Words). That is the thing the new census is for.

## C13 eligible words with NO collision group, per language and tier
Method: throwaway ignored test in a scratch copy (repo untouched): per tier, bands from Tier::bands() (Jr = easy+medium), length range from Tier::shape() (Jr 3-6, Easy 3-6, Medium 4-8, Hard 5-9, Expert 6-10 chars), not profanity::is_blocked, audio_verdict::servable(lang, w, "normal"), Jr also kid_filter::kid_allowed; words distinct; "no collision group" = homophones::group_members(lang,w) empty (same test bank::load uses). Format: no-collision / eligible-before-collision-filter.
Caveat: counts chars, not keyboard-reachability or Lexicon::is_word; matches bank::load semantics. Run time 0.7 s.

| lang | Jr | Easy | Medium | Hard | Expert (6-10) | collision table |
|---|---|---|---|---|---|---|
| en | 1191/1443 | 533/715 | 709/783 | 756/773 | 529/536 | 266 lines |
| es | 975/981 | 232/234 | 824/828 | 1563/1567 | 1810/1810 | 14 |
| ru | 1536/1587 | 708/746 | 879/892 | 1578/1584 | 1897/1900 | 37 |
| fr | (999) | (235) | (833) | (1563) | (1783) | none, count unchecked |
| de | (1045) | (240) | (895) | (1481) | (1644) | none |
| pt | (1025) | (238) | (854) | (1515) | (1771) | none |
| pl | (1036) | (218) | (904) | (1550) | (1870) | none |
| fil | (779) | (207) | (577) | (872) | (992) | none |
| sw | (906) | (241) | (671) | (1034) | (480) | none |
| vi | (1480) | (663) | (826) | (1500) | (1983) | none, fails C3 anyway |

- English: ALL tiers at or above 400. Lowest is Expert 529 (margin 129), then Easy 533. No STOP on English. Jr 1191 is on the union of Easy+Medium.
- es: Easy 232 below floor (bank holds only 268 Easy rows). Jr/Medium/Hard/Expert clear.
- ru: clears every tier now, incl. Easy 708. Reason: ru easy.txt grew 2026-10-09 (4303e52f, now 833 rows) after the census table was computed (it said 219). Flag as a change to the earlier C4 verdict.
- fr de pt pl fil sw vi: no collision table, so P1 cannot be evaluated; numbers in brackets are only an upper bound. Easy is under 400 in all of them (207-241) except vi (663, fails C3). sw Expert 480 only just clears.
- en Easy 533 means 182 of 715 Easy words are collision members (25%); P1 removes them from Par boards (Hard 17, Expert 7 members) so Par is unaffected in practice.

## C17 My Words storage, bank anchor, Jr serving, entitlement
- Storage: src/word_lists.rs. Key byear_word_lists_v1 (LISTS_KEY, line ~17). struct ListEntry {text, lang, added_at, starred} (lines ~49-58). WordList {id, profile (empty, reserved), name, entries, source Photo|Translate|Manual|Migrated, order, deleted_at}. Per DEVICE (signed 2026-09-17 in docs/CC-MYWORDS-LISTS.md), no profile scoping. Legacy flat set is model::CustomSet (src/model.rs:74) {words, speak_lang, word_lang, word_batch, custom_marks}.
- Bank anchor: NONE. docs/CC-SNAP-DOC1-census.md sections 2-3: "bank_id appears zero times in code", ListEntry has "No anchor field, and no second structure holding one". CC-SNAP-ROADMAP.md:265-270 says the same. What exists is src/word_index.rs::canonical(lang, word) -> Option<String> (app-only: lib.rs `#[cfg(not(feature="web"))] mod word_index`), a validity index that returns the bank's spelling, with a known identity flaw for zh (372/6182 rows unreachable) and fa (70). snap_clean.rs:459 bank_lookup wraps it, returns None on web.
- STOP flag: a stored My Words row carries NO bank match. F21's "bank match to an audited row" must be computed at board-build time (canonical lookup, English, app-only, which fits Spelluzzle being ios-only) and cannot be read from the row. If F21 wants a stored id, that is a schema addition no file owns (DOC-1 census section 5). Also note custom_marks flags photo words that are OUT of dictionary.
- Kid/Jr serving in the base game: game.rs:112-130 active_word_list: when lang == MINE the pool is state.list_words / custom.words (pool_for_tier game.rs:90-110, filtered by length_tier), and the kid_filter is NOT applied (comment: "My Words is parent-curated (and still runs the global profanity filter)"). A Jr/kid profile therefore sees every list word. Jr cannot delete a list without parent gate (D4/D7), but can create lists and add words. Spelluzzle itself ignores My Words today: spelluzzle_screen.rs:123 and boardgame_screen.rs:248 map lang MINE -> EN.
- Consequence for F21 Jr: the base game gives no Jr kid-filter precedent for My Words, so "Jr: also v1 I12 and top-ups the Jr resolver serves" needs kid_filter::kid_allowed applied by Spelluzzle itself to list words (the function exists, src/kid_filter.rs).
- Entitlement: config/modes.json:277 my_words: entitlementLevel "preview" (free), requiresPremium null, languages null, kidSafe true, platforms ios+web, juniorPolicy "ungated", status core (a surface, not a tile). Photo import is the paid piece: entitlements.rs:99 photo_ocr (Complete parent-premium); docs/CC-MYWORDS-LISTS.md D6: no cap on lists for payers; free-tier word cap still open, FREE_CUSTOM_LISTS_CAP unenforced. spelluzzle row is entitlementLevel preview, so a My Words board in Spelluzzle needs no new entitlement.

## C19 CC-FEEDBACK sound / pitch ladder
- src/feedback.rs is ONLY an outcome->State mapping (Success/Close/Miss/Neutral; From<game::Outcome>, From<spelldoku Verdict>, From<spelluzzle::play::Outcome> at line 91). It produces no sound.
- Sound code that exists: src/audio_boost.rs only. chime() (line 118): fixed two-partial Web Audio chime, 880 Hz + 1320 Hz, 20 ms attack, <1 s, routed through the shared gain/limiter (ensure_ctx line ~38); parameters are const CHIME_PARTIALS, no frequency argument. Callers: bee_screen.rs:393, wordpic_screen.rs:2537, calendar.rs:226, c3_probe.rs. src/haptics.rs: correct(), key_tap() (LIGHT impact), incorrect(kid): Capacitor haptics, native only, no sound. Word audio is clips (api.rs).
- NO pitch ladder, NO combo state, NO tick, NO sfx assets: grep for pitch/ladder/combo finds nothing relevant (only game.rs/bee ladders of word tiers); no .mp3/.wav/.caf/.ogg in the repo except human-audio fixtures. The file docs/CC-FEEDBACK is not in the repo.
- Usable as a per-rune tick as it stands: nothing audible. haptics::key_tap() is a tactile tick today (and a stated "tactile substitute" because "the game has no key sound effect"). chime() is a completion chime, wrong shape for a tick and fixed pitch.
- STOP flag: a pitch-ladder step needs new code; to keep it asset-free it can be a new oscillator function next to chime() with a frequency parameter (no .caf/.mp3 needed), but that is a new sound (new code, new sign-off per the "soft" bounds that chime_tests pin). Asking for a ladder step "without touching combo state" is trivially possible only because no combo state exists. If CC-FEEDBACK spec assumes an existing ladder, it does not exist in this tree.

## C20 CC-BOARD-GAME NPC code reuse
- Module: src/boardgame/ (mod, engine, rules, board, golden, balance, tests), `mod boardgame` in lib.rs:25 is private to the crate and `#[cfg(not(feature = "web"))]` (app only); other modes in the crate can reach it as crate::boardgame::...
- Public pieces: boardgame::Difficulty {Easy, Normal, Tough} + ALL + ix() (mod.rs:130-143); boardgame::rules::Variant {Full, Sprint, Jr}.cfg() -> &'static VariantCfg; VariantCfg::delta(&self, d: Difficulty) -> i64 (rules.rs:233; delta_milli Full/Sprint [300,200,100], Jr 250 flat, rules.rs:88,116,141); VariantCfg::prior(&self) -> (i64,i64) (3000/2000; Sprint 750/500); VariantCfg::npc_stretch(&self) -> (i64,i64,i64); GameConfig::solo(...) (mod.rs:204).
- The accuracy formula is NOT a free function. It is `BoardGameState::npc_acc_milli(&self) -> i64` (engine.rs:560) = (human_acc_milli - variant.delta(difficulty)).clamp(NPC_FLOOR_MILLI, NPC_CEIL_MILLI), and `BoardGameState::human_acc_milli(&self) -> i64` (mod.rs:486) = (prior_hits + hits*1000)*1000 / (prior_attempts + attempts*1000), reading the human's players[].hits/attempts. NPC_FLOOR_MILLI = 300 and NPC_CEIL_MILLI = 950 are PRIVATE consts (engine.rs:10-11). The roll is a private `fn chance(&mut self, milli)` using the state's own RNG (engine.rs:193); NPC turn logic `npc_turn`/`npc_roll` are private (engine.rs:568,579) and inseparable from board movement, traps and stretch.
- NPC turn length: there is no such concept. An NPC turn is one d6 roll plus one spelling roll (plus one extra roll on Extra Roll boost); driven by Action::AdvanceNpc. NPC names: none; seats carry only a piece icon index (Seat {piece, npc}), no names.
- Verdict: usable from another mode only by (a) building a whole BoardGameState (new_game needs a TierPools and a Grader) to call npc_acc_milli, or (b) re-implementing the 3-line formula with VariantCfg::delta and ::prior (pub) plus the 300/950 clamp.
- STOP flag: yes. As it stands reuse means COPYING (the formula, the two clamp constants, the (hits,attempts) prior bookkeeping) or making a small visibility change (a pub free fn `npc_accuracy_milli(variant, difficulty, hits, attempts)` and pub consts) in src/boardgame/engine.rs. Spec F20's "Easy/Normal/Tough adaptive" matches D13/D18 exactly. Recommend the pub-fn extraction (golden-tested engine, change must keep digest golden stable), which is an edit to boardgame code, so ask Eric.

## C22 long-press on Spelluzzle cells
What the page does today:
- Long-press binding in the app is KEYBOARD ONLY: src/keyboard.rs:367 wire_long_press attaches a window pointerdown that acts only when target.closest(".kb-key[data-acc]") matches (350 ms HOLD_MS, line 360; accent popover #kbPop). Spelluzzle keys are `.sz-key` (spelluzzle_screen.rs:384) and cells are `<span class="sz-cell">` (render.rs:39); neither matches `.kb-key`, so there is no long-press accent popover on the Spelluzzle keyboard (relevant to non-en: es fr de pt pl ru accents are reachable only through the main keyboard) and none on cells.
- Board handlers: a single delegated "click" on #szBoard (spelluzzle_screen.rs:116, board_tap), #szKb click (117), #szTiers click (115). Nothing on pointerdown/touchstart/contextmenu/pointerup in this screen. Rows have `cursor:pointer` (index.html .sz-row). A new long-press on a cell would add a pointer timer; the existing click would also fire on release unless suppressed (keyboard.rs does this with suppress_click).
- OS/page bindings that a cell long-press can hit:
  - -webkit-touch-callout: NOT set anywhere in index.html, ios/, or src (grep: zero hits). Default iOS WKWebView behaviour applies: long-press on a non-link, non-image span gives no callout.
  - user-select: NOT set on .sz-screen, .sz-board, .sz-row, .sz-cell. Only .game-kb (index.html:1238), .orb-wrap, voice-spell-mic and the wordsearch grid (line ~1812) set user-select:none. So text in .sz-cell (and .sz-k, .sz-msg) is selectable: on iOS a long-press on a cell that holds text (typed letter, decoded letter) starts the word-selection loupe/callout (Copy, Look Up, Share). Cells showing runes are inline SVG with aria-hidden and give no text to select, but the decoded/typed letter cells are text spans.
  - contextmenu: no handler anywhere (grep contextmenu: zero hits). On Mac Catalyst/iPad pointer, right-click/long-press gives the system menu.
  - touch-action: only `touch-action:manipulation` on button, .kb-key, .pill, .ghost, .btn (index.html:158). `.sz-cell`/`.sz-row` are spans/divs, not covered, so double-tap-zoom is not suppressed for them (the viewport lock is a separate matter).
  - -webkit-tap-highlight-color: transparent only on .kb-key (line 1254); not on .sz-*.
  - Scrolling: #szBoard is overflow-y:auto (.sz-board), so a long-press that drifts becomes a scroll; pointercancel fires.
  - iOS accessibility: no doc covers it. docs has no Spelluzzle accessibility note; reports/spelluzzle-device-pass.md has no long-press item. Known OS bindings: VoiceOver double-tap-and-hold gesture, Switch Control and AssistiveTouch "long press" are system-level and would reach the element as a pointer long press only if the control is focusable; cells are non-focusable spans (aria-label set, role none), so a VoiceOver user cannot long-press a cell at all, which means anything on long-press MUST also have a button route.
- Verdict: no app binding conflicts, but there IS an OS binding to defeat: selectable text in .sz-cell gives the iOS selection callout on hold. A long-press feature would need `-webkit-user-select:none; user-select:none; -webkit-touch-callout:none` on .sz-cell (or .sz-board) and preventDefault on contextmenu, plus a click-suppression after the hold fires, plus a visible non-gesture route for VoiceOver/Switch Control. Not a STOP; a CSS-only + small JS addition. Not measured on a device (cannot here).

---

## v1.1 appendix: c21

# C21 layout census, 375x667 pt (Chromium, real index.html CSS, DOM mocked to match render.rs)
Harness: run.mjs / pencil.mjs / pm.mjs / ov.py in this folder. Screenshots: shot-expert-kbup-par-stripB.png, shot-expert-idle-duel-stripA.png, shot-base-expert9-kbup.png, shot-pencil-marks.png.

## v1 baseline (measured)
- Cell width is set by screen width, not fixed: 9 cells = 36.8 pt, 10 cells = 32.9 pt (screen pad 8, board pad 2, row pad 4, gap 2). Both >= 28. 10-cell cell is only 4.9 pt above 32.
- The keyboard is the in-app .sz-kb (3 rows x 40 + gaps = 120 pt, bottom at 659). It is NOT the OS keyboard, so "keyboard up" is deterministic. The v1 rune key (.sz-runekey, 16 entries = 2 wrapped rows, 58 pt, entries 35/42.6 wide x 26 tall) is HIDDEN while typing.
- HUD is Exit 48 | title | New board 91 | ? 48. New board is visible during play (begin() unhides it), the streak span is display:none while .playing. Title gets 147.7 pt.
- Board area: 471 pt with keyboard up, 533 pt with keyboard down (top inset 0). Top inset 20 takes 20 off both.
- Content height of the board column (7 rows): v1 Expert (3 silent rows with Listen head, 10 cells) 423 pt; Hard 9 cells x2 silent 415; 9 cells x3 silent 450. All FIT with keyboard up (21-56 pt spare).

## (a) Rune strip, 16 entries (glyph + count + letter)
- One row at 375 pt: 359 usable / 16 = 22.4 pt per entry (21.5 with gaps). Only a STACKED entry fits (16 pt glyph over 11 px "3 a" caption, 44 pt tall). Width 21.5 is below any touch target: display-only. Inline glyph+count+letter needs ~34-50 pt per entry (measured 40.6-50.4), so it does not fit one row.
- Two rows of 8 (inline, 18 pt glyph, 12 px text): entry 43 pt wide, strip 62 pt tall. Legible, touch-able.
- Single row scrolling horizontally: 32 pt tall, 825 pt wide (2.3 screens): 7 of 16 visible at once. Legible but hides state.
- Folded to a one-line toggle bar: 36 pt.
- Cost with keyboard up (strip kept visible while typing): A (1-row stacked) 44 pt, B (2x8) 62, C (scroll) 32, D (2x8 at 44 tall touch size) 92, E (fold) 36.
  Expert 3-silent, 10 cells, no other additions: A scrolls 4 pt, B 22, C fits, D 52, E fits.
  With the strip hidden while typing (as v1 does) all variants cost nothing with keyboard up; idle space is ample (547 vs 423 needed).

## (b) Pencil mark inside a 32 pt cell
Rendered 28 runes x 5 letters (a g w m y) in the top-right corner, pixel overlap test with the 68%-glyph at 32 pt:
| glyph size | 9px | 10px | 11px | 12px | 13px |  (combos touching of 140)
| 68% (v1) | 6 | 10 | 24 | 42 | 62 |
| 60% | 2 | 2 | 7 | 22 | 41 |
| 52% | 0 | 0 | 0 | 7 | 25 |
| 44% | 0 | 0 | 0 | 2 | 10 |
- With the v1 glyph (68%) the largest CLEAN mark is 9 pt (4% touch); an 11 pt mark touches the glyph in 17% of combos (24/140), 12 pt in 30%.
- Clean 11 pt mark requires the glyph at <= 52% (about 17 pt in a 32.9 cell) while a mark is present, or the mark replacing the glyph. 12 pt clean needs <= 44% (14.5 pt glyph).
- Ink size at 11 px bold: x-height letters 5.3-5.8 pt tall, 'g/y' 8 pt. Legible in the screenshot at 11 px; 9 px marks look cramped.
- So >= 11 pt IS reachable, but only by shrinking the glyph to ~52% in cells carrying a mark (or by showing the mark centered at 19 px as a dimmed "typed" letter, which has no overlap at all). It is not reachable by simply overlaying a corner letter on the v1 68% glyph.
- 9-cell rows (36.8 pt) have 4.8 pt more room: 11 px at 60% glyph is clean-ish; the 10-cell Expert row is the binding case.

## (c) Par header 'Listens n · Par p'
- Inline in the HUD slot where the streak span lives: with New board visible the title is squeezed to 49 pt (natural 80, ellipsised) and the Par text takes 90 pt. NOT acceptable (title clipped; also v1 title = 147.7 with no Par).
- Replacing the title text with 'Listens 1 · Par 2' inside the existing h2: fits at 147.7 pt, no clipping, 0 pt cost (title is redundant in play; note this removes the mode name from the HUD).
- Separate centred line under the HUD: 28 pt (20 line + 8 gap).

## (d) Duel header (two scores, whose turn, Board k of 2)
- One extra line (You 12 | Your turn . Board 1 of 2 | Sam 9, 0.85rem): 30 pt; fits 359 pt.
- Replacing the title with 'You 12 . Sam 9 . Bd 1/2': fits 147.7 pt, but whose-turn then needs its own place. Two-line block (scores+board / turn + instruction): 42 pt.

## Vertical budget, the real finding
Par and Duel start with every word silent, so every spoken row gets the 32 pt Listen head (rows grow from ~41 to ~76 pt). Measured content, 7 rows all with heads:
- 10 cells: 528 pt. 9 cells: 555 pt.
- Keyboard up: avail 471 -> scrolls by 57 (10 cells) / 84 (9 cells); with a Par line 85 / 112; Duel one line 87; Duel two lines 99 / 126.
- Keyboard down: avail 533 -> 10 cells fits (528, 5 spare); 9 cells scrolls 22; with Par/Duel line scrolls 23-64.
- Top inset 20: 423 avail with keyboard up (scroll 105).
So with the current per-row Listen head 7 rows do NOT stay visible with the keyboard up for Par or Duel at 375x667. Fixes measured:
 1. One shared action bar above the keyboard (Listen / Spell unheard for the selected row, 44 pt tall), no per-row heads: content 300 pt (10 cells) / 327 pt (9 cells), avail 389-419 with Par/Duel line + bar, keyboard up. FITS everywhere, even with the strip variant A or E (avail 339-347 vs 300-327). Leaves rows tappable to select (they already are).
 2. Compact 24 pt Listen button in each row: 480/507 pt, scrolls 9-36 kbUp (37-64 with Par line): does not fit, and 24 pt is below the 32 pt button v1 already uses.
 3. Letting the board scroll: it already does (overflow-y:auto), cells stay 32.9; selected row can be scrolled into view.

## STOP flags
- Cells < 28 pt: NONE. Min is 32.9 pt (Expert 10 cells), unchanged by any addition (all additions are vertical only; screen width is the constraint).
- Pencil mark < 11 pt: STOP-conditional. At the v1 glyph size (68%) a clean mark tops out at 9 pt. 11 pt needs glyph <= 52% (or centered mark replacing the glyph). Spec must say which, or the 11 pt floor is breached.
- Not a STOP but a hard conflict: per-row Listen heads + Par/Duel header + keyboard up overflows by 57-126 pt at 667 pt height; needs the shared-action-bar layout (fix 1) or accepted scrolling.
- Title ellipsis: inline Par/Duel text in the HUD streak slot squeezes the title to 49 pt; put the text in the title or a separate line.
- The existing v1 rune key hides while typing; a strip that stays visible with keyboard up costs 32-92 pt more (see (a)); only C/E fit the Expert row with no header addition, none fit with a header unless fix 1 is applied.

---

## v1.1 appendix: c14

# C14 Par trial (Hard, Expert), English, 10,000 seeds per tier

Verdict: no STOP. Both tiers 10,000/10,000; time p95 5.1 ms (Hard), 3.3 ms (Expert) on this Mac (Apple Silicon dev Mac, release build, single thread, includes pool filter + every failed attempt + par + gates + P5). iPhone p95 CANNOT be measured here; a Mac number only. Even at a 10x slower phone the p95 is ~50 ms, far under 1.5 s.

Code: scratch worktree off origin/main 27280a81, throwaway test src/spelluzzle/par_trial.rs (copy kept at scratchpad/spz11/par_trial.rs.txt), reuses crate::spelluzzle::{bank, lex, gates::count_solutions (v1 G8 solver), types}. No repo file modified; scratch worktree removed.

## Input caveat (read this)
The prototype dir and validity.txt named in the brief (scratchpad/spz/) did not exist, and there is no en_US.dic on this Mac. Stand-in large validity list = /usr/share/dict/words (Webster's 2nd, 235,976 lines, lowercase a-z kept) plus bank, with v1 seedgen's `expand` (s, es, ed, d, ing, er, ers, est, ly, y, ies...). That list is 2,586,198 strings: far larger and junkier than Hunspell would give, so it is the STRICT end. Three lists run, all 10,000/10,000:
 A web2 expanded (2.59M, primary), B web2 stems only (211k), C bank only (3,434).
Real Hunspell-derived list lies between A and C; Eric/peer should re-run with the real en_US.dic: `PAR_VALIDITY_FILE=<dic> cargo test --release --lib par_trial -- --ignored --nocapture` in a copy (the loader strips "/flags").

## Results, primary run A (validity 2,586,198; P5 checked for EVERY par set)
| | Hard | Expert | desk Hard | desk Expert |
|---|---|---|---|---|
| success | 10000/10000 | 10000/10000 | 1000/1000 | - |
| attempts p50/p95/max (cap 2000) | 6 / 23 / 63 | 8 / 34 / 90 | 31/131/362 | 36/157/428 |
| time per board p50/p95/p99/max, ms (Mac) | 2.1 / 5.1 / 7.2 / 34 | 0.9 / 3.3 / 7.7 / 46 | - | - |
| par 2 / par 3 | 84.8% / 15.2% | 97.8% / 2.2% | 78.0/22.0 | 82.8/17.2 |
| par sets min/median/max | 2 / 3 / 8 | 2 / 4 / 8 | 2/3/8 | - |
| most-cells-first reaches par, longest word | 47.2% (+0.59 Listens) | 48.4% (+0.56) | 74.3% (0.26) | 73.5% (0.27) |
| most-cells-first reaches par, greedy newly-decoded cells | 67.0% (+0.34) | 71.2% (+0.29) | same row | same row |
| random order, avg Listens over par | 0.95 | 0.87 | 0.93 | 0.93 |
| runes avg / max (min) | 15.19 / 16 (12) | 15.79 / 16 (12) | 15.7 | - |
| cells avg | 52.3 | 65.7 | | |
Pools: Hard 773 words in band+length 5-9, 756 with no collision group; Expert 536 in band+length 6-10, 529 with none (C13 eligible-with-no-group counts for hard/expert in this metric; both above the 400 floor).

Other lists (success all 10000/10000; attempts p50/p95/max Hard 6/23/63 in all three, Expert 8-9/34-36/90-93):
 B web2 stems: Hard par2 90.1% par3 9.9%, Expert 98.1/1.9; p95 1.0 / 0.7 ms; greedy reaches par 69.0 / 71.9%; longest 49.2 / 49.4%.
 C bank only: Hard par2 91.0 / par3 9.0, Expert 98.2/1.8; p95 0.5 / 0.6 ms.
 P5 "first par set only" instead of "all": identical boards (no board ever failed P5).

## Gates rejecting most (first failing gate, attempts incl. accepted; run A)
Hard (80,618 attempts, 10,000 accepted = 12.4%): construction stuck (no candidate word closes the 16-rune / single-held-rune constraint) 49.2%; G6b (one word decodes >60% of the secret) 32.8%; P4 too few par sets 3.7%; P4 more than 40% of sets are par sets 1.8%; P3 par<2 0.05%; P3 par>3 0.002%.
Expert (116,321 attempts, 8.6% accepted): G6b 55.7%; stuck 31.8%; P4 many 3.1%; P4 few 0.7%; P3 par<2 0.1%.
G1, G2, G4, P1, P2, G6a never fire (by construction: pool is already no-group, in band, length, <=16 runes, every rune in >=2 words). P5 never rejected a board in 20,000: it is implied by par (a one-short step fires only when exactly one validity word fits, which forces a unique full assignment), so P5 is a redundant cross-check of the par computation, not a filter. G6b is the real rejector; the 'stuck' share is a generator inefficiency not a gate (v1 counts it as an attempt too).

## Findings vs desk model
- Success, attempts, time all comfortably inside; attempts are LOWER than the desk (6/23/63 vs 31/131/362) because the sequential, single-held-weighted draw closes P2 quickly; time is dominated by the memoised one-short check.
- Par skews easier than desk: Hard 84.8% par 2 (desk 78.0), Expert 97.8% par 2 (desk 82.8). Par 3 is rare in Expert (2.2%): if Expert should feel harder the par-3 share is 2%, a design point (P3 allows 2 or 3 but the generator rarely produces 3 there). More rune sharing (avg 15.8 of 16 runes in Expert) makes every word one-short quickly.
- "Most cells first" is NOT 74% under my reading when it means the longest word: 47-48%, with 0.56-0.59 Listens over par vs random 0.87-0.95, i.e. barely better than random on Listens. Under the reading "Listen the word that decodes most still-hidden cells" it is 67.0% / 71.2%, 0.34 / 0.29 over par, close to the desk's 74.3/73.5% and 0.26/0.27. The desk number therefore matches the greedy-gain reading, not the longest-word reading. Eric should say which one the spec means.
- Par-set count: median 3 (Hard) / 4 (Expert), max 8 = the P4 cap for par 3 (<=40% of 20) so some boards are near the cap; the cap for par 2 is 6 of 15.

## Exact definitions implemented
Board: 7 words from the tier's pool (6 listenable + the secret, always the last drawn); runes = seeded permutation of the distinct units; runes <=16.
Listen: decodes every rune of that word (it is then finished). The secret has no Listen but is a word that must be finished.
Decoded set K = union of runes of listened words and of finished words.
ONE-SHORT STEP on an unfinished word w (secret included), given K:
 - u = distinct runes of w not in K.
 - u = 0 (zero undecoded runes): w is finished with no fit test (it is fully read; the only fit is the answer itself, which is in the validity list, so the exactly-one test would pass anyway).
 - u = 1 (a repeated rune counts once): finished iff EXACTLY ONE validity-list string fits. Fit = same isomorphism pattern as w; at every position whose rune is in K the unit equals the answer unit; at every undecoded position the unit is not any unit already decoded (units are one-to-one with runes). The validity list = bank (all tiers) + extra list (large list when supplied) + collision-set members; the answer itself always counts, so a 'fit' is any other string on the list that matches. Two fits (the answer plus any other validity word) = the step is blocked.
 - u >= 2: blocked.
Closure: repeat steps (finished words add their runes to K) until no progress; board is "finished" when all 7 are finished.
PAR = size of the smallest subset of the 6 listenable words whose closure finishes all 7. PAR SETS = every subset of that size that works (all 2^6 subsets tried, smallest first).
Gates (checker order): G1 (alphabetic, length in tier range 5-9 / 6-10, in the tier band, 7 distinct), G2 (<=16 runes), G4 (each word shares >=2 runes with the other six), G6a (every secret rune in another word), G6b (no single other word decodes >60% of secret cells), P1 (no word has a collision group), P2 (every rune in >=2 words, including secret), P3 (par 2 or 3), P4 (>=2 par sets and par sets <= 40% of C(6,par): <=6 of 15, <=8 of 20), P5 (v1 G8 solver gates::count_solutions with the par set as heard, other words must read as validity-list words, exactly 1 assignment; run for EVERY par set in the primary run, first only in a variant: no difference).
Generator: pool = lex.pool(tier) minus words with a collision group; first word uniform; each next word weighted k^3 (k = 1 + new coverage of runes held by only one word so far), union <=16 runes, >=2 runes shared with union; the 7th word must add no new rune and cover every single-held rune. One draw = one attempt (a stuck draw counts), cap 2000, seeds 0..9999, per-seed RNG mix(seed ^ fnv("en:par:<tier>"), 1).
Players: both deduce every available one-short step before each Listen. (1) longest-word: Listen the unfinished listenable word with most cells (tie: more undecoded cells, then lowest slot). (2) greedy: Listen the word maximising newly decoded cells across all unfinished words. Random: 10 random orders per board, Listens averaged. Listens over par counted until all 7 finished.
Time: wall clock around pool filter + whole generate loop + checker; the player simulation is excluded.

---

## v1.1 appendix: c15

# C15 Expedition trial (English, Medium / Hard / Expert, 10,000 seeded runs per tier)

Read-only for the repo. Scratch copy: worktree spell-wasm-spz15 at origin/main 27280a81 (removed at the end). Code kept as
`c15_run_trial.rs.txt` beside this file (a throwaway `#[cfg(test)] mod run_trial` in src/spelluzzle). Raw logs: `c15-guided.log`, `c15-unguided.log`.

## Verdict

- STOP does NOT fire for the generator with new-rune-weighted picks ("guided"): 10,000/10,000 at all three tiers, 0 runs above 24 runes, Mac run-time p95 0.02 / 0.13 / 0.85 s.
- STOP DOES fire for the plain v1-style generator ("unguided", the same cube weighting on single-held runes, nothing else): Expert 8,920/10,000 (1,080 runs never finished inside 40 redraws). Medium and Hard are 10,000/10,000 either way.
- Things the gates do to Expert, whichever generator: median 24 runes per run (the X1 ceiling itself), 72% of runs need at least one redraw from board 1, 26,302 cap hits in 10,000 runs, and the Vault is a ~1-in-500 to 1-in-4,000 draw. See "Findings".
- iPhone time cannot be measured here. Mac numbers only, and the Mac was shared with other sessions during the runs (load average 14-24), so the times are pessimistic.

## Setup

- Bank: real English bank via `bank::load("en", extra)`. Validity list: the Hunspell en_US.dic stems (38,406; sha256 f0b1a234...3647, the same file seeds-en.json records) with the seedgen expansion, plus bank and collision members = 469,516 words (identical count to `validity_words` in seeds-en.json). The file named in the brief (scratchpad/spz/validity.txt) did not exist; I rebuilt it from the Hunspell file in ~/repos/spell-wasm-purity/.corpus-cache/dicts/en/.
- Pools (G1, tier band, tier length range): Medium 783 words (4-8), Hard 773 (5-9), Expert 536 (6-10; Eric's amendment).
- Seeds: run k of a thread's chain uses seed k; `previous` = the words of the run before it in that chain (9 chains of ~1,112 runs, in-order). RNG is spelldoku::rng. Redraw policy as spec: 2,000 draws per board; a later board hitting the cap redraws the run from board 1; board 1 hitting the cap just keeps drawing a new run. My own bound: 40 redraws per run, then the run counts as a failure (that is what the 1,080 Expert failures are). Word sharing with the previous run is held to <=4 for the first 3 redraws of a run and then dropped ("fewest possible if thin" was not implementable literally; this is my reading).
- A "draw" = one construct call plus, if it produced 5/6/5 words, the check. A construct that finds no candidate counts as a draw.

## Definitions implemented (checker: independent of the generator; generator and checker share only the Lexicon API)

Run = 16 words: board 1 = S1..S4 spoken + K1 secret; board 2 = A1..A3 spoken + B1,B2 silent + K2 secret; Vault = V1..V4 silent + K3 secret. Unit = one rune across the whole run (a rune is identified with its unit; the seeded permutation of glyph numbers is irrelevant to every gate). `C1` = units of board 1; `C2` = units of boards 1+2 ("carried"; assumes the earlier boards were solved completely, secret included). Cells of a board = all cells of all its words including the secret.

- X1: every board <=16 units (board 1 uses its tier cap, below); whole run <=24 units.
- X2 (new words vs earlier words and each other): each word alphabetic, length in the tier's range, in the tier's band (v1 G1); all 16 distinct; no pair in one collision group (`lex.group`); no two share their first four units.
- X3 (board 1, 4 spoken + secret), in this order: unit count <=10 Medium / 11 Hard / 12 Expert; G4 each word shares >=2 distinct units with the union of the other four; G5 every unit of a spoken word is in >=2 of the five words; G6a every unit of K1 is in some other word; G6b no single non-secret word's units cover more than 60% of K1's cells (d*10 > 6*len fails); G10 earned share >= 2/5, earned = sum over units of (cells of the unit in the 4 spoken words / spoken words holding it) / spoken cells, exact (denominator 12); G11 mean over the 4 spoken words of (cells of the other 4 words decoded by that word's units / those cells) >= 1/4, exact rationals; G8 exactly one full assignment (below). G3 is the X2 collision rule applied to the five. G1 is X2. G7/G9 have no silent word on board 1 and are not applied.
- X4: board 2 has >=4 units not in C1; Vault has >=3 units not in C2.
- X5: units of C1 decode <=60% of board 2's cells (d*5 <= 3*cells); K2 not fully decoded by C1. Vault: C2 decodes <=75% of its cells (d*4 <= 3*cells); no Vault word (secret included) fully decoded by C2.
- X6: every board-2 spoken-word unit is in C1 or in >=2 words of board 2.
- X7: silent word decoded cells / its cells >= tier share (Medium 6/10, Hard 5/10, Expert 4/10, from `Shape.silent_share`) and < all. Board 2: B1,B2 against C1 + units of A1..A3. Vault: V1..V4 against C2 alone (no spoken word exists).
- X8: v1 G10's earned sum over spoken words of board 2 with every unit of C1 skipped (counted as decoded), divided by spoken cells, >= 1/4 (12*spc <= 4*sum, exact).
- X9: board 2 and Vault: every unit of the secret is carried or in another word of that board.
- X10 (own solver, `count_assign`): with every carried unit fixed to itself, the number of injective assignments of the other units such that each spoken word reads as a member of its answer's collision group of the same isomorphism pattern (or itself if none) and each silent/secret word reads as a validity-list word of the same pattern, with no free unit taking a carried unit, is exactly 1 (counted to 2). Board 2: spoken = A1..A3. Vault: all five from the list. Board 1's G8 is the same function with nothing carried.
- X11: `one_fit(w, known)` = exactly one validity-list word of w's pattern equals w on every known unit and uses no known unit in an unknown position. Closure from the starting `known`: take any unsolved silent word whose decoded share >= tier share and that has exactly one fit; add its units; repeat. Vault: known = C2, all 4 silent words must close and the secret's units must then be all known. Board 2: known = C1 + A1..A3's units, both silent words must close. "Vault finishes with no Listen" is this closure run on the final run: all 10,000 per tier (it is a gate, so it equals the success count).
- Gate order (so the "first failing gate" columns mean what they say): X2, X1/cap, X3 gates in the order listed, then for board 2: X1, X4, X5, X6, X9, X7, X8, X10, X11; Vault: X1, X4, X5, X9, X7, X10, X11.
- Whole-run check (`check_run`): re-runs all three board checkers with no short-circuit on the 16 final words from scratch plus the 24-unit rule. It agreed with the generator on 10,000/10,000 at every tier and mode.

Generator: v1's construct generalised. Per pick: words in the tier pool, not banned (picked, collision-group mates of picked, same first four units), sharing <=4 with the previous run, board unit union within the board cap and run union within 24; B1 and B2 words must share >=2 units with the words already chosen; B2 and Vault silent words must meet the X7 share and be < all decoded; the secret must lie inside union + carried (X9) and, on B1/B2, close every unit still held by one spoken word; weight (1 + |units held by only one spoken word|)^3. "Guided" additionally multiplies the weight of every non-secret B2 pick by (1 + new units)^2 and Vault silent picks by (1 + new units)^3 (new = not carried). A mask-level screen of X4/X5 runs before the checker (the checker still decides). This is a generator choice, not a spec change; the spec's own search method is unspecified.

Self-checks: my solver vs `gates::count_solutions` (carried emulated by an extra spoken pseudo-slot, restricted to carried units present on the board) on 600 candidate boards per tier, 0 mismatches (e.g. Medium 173/228/199 on board 1/2/Vault; Vault Expert only 97, rest hit the >16-unit skip). One-fit vs a linear scan of the whole validity list: 1,800 per tier, 0 disagreements.

## Results, guided generator (the numbers to use)

| | Medium | Hard | Expert |
|---|---|---|---|
| success of 10,000 | 10,000 | 10,000 | 10,000 |
| no redraw needed | 9,956 | 9,249 | 2,755 |
| runs needing >=1 redraw / max redraws | 44 / 1 | 751 / 4 | 7,245 / 36 |
| cap hits (2,000-draw board failures) | 44 | 805 | 26,302 |
| runs sharing >4 words with previous run | 0 | 0 | 34 (max 7) |
| attempts board 1, accepted pass, p50/p95/max | 4 / 14 / 37 | 11 / 47 / 172 | 36 / 152 / 505 |
| attempts board 2, accepted pass | 1 / 5 / 22 | 2 / 10 / 153 | 22 / 668 / 2,000 |
| attempts Vault, accepted pass | 8 / 77 / 1,838 | 50 / 773 / 1,998 | 481 / 1,777 / 2,000 |
| attempts board 1, summed over redraws | 4 / 14 / 57 | 12 / 52 / 172 | 130 / 553 / 2,168 |
| attempts board 2, summed over redraws | 2 / 5 / 22 | 2 / 11 / 153 | 164 / 3,067 / 12,161 |
| attempts Vault, summed over redraws | 8 / 82 / 2,175 | 61 / 2,029 / 8,010 | 4,016 / 16,798 / 69,480 |
| run time p50 / p95 / max, one thread, 300 runs (Mac) | 0.002 / 0.023 / 0.214 s | 0.006 / 0.128 / 0.292 s | 0.250 / 0.846 / 1.875 s |
| run time p50 / p95 / max, 9 threads, all 10,000 | 0.003 / 0.042 / 0.315 s | 0.013 / 0.190 / 0.776 s | 0.323 / 1.452 / 5.451 s |
| runes per run median / max | 22 / 24 | 23 / 24 | 24 / 24 |
| runs >24 runes | 0 | 0 | 0 |
| board 2 decoded at load, p10 / p50 / p90 | 35.3 / 45.9 / 55.6 % | 44.4 / 53.3 / 58.7 % | 52.7 / 57.7 / 59.6 % |
| Vault decoded at load, p10 / p50 / p90 | 69.2 / 73.3 / 75.0 % | 71.4 / 74.3 / 75.0 % | 72.7 / 74.4 / 75.0 % |
| Vault finished with no Listen (X11) | 10,000 | 10,000 | 10,000 |

Gate that rejects most (first failing gate; counts over all drawn candidates):

| | Medium | Hard | Expert |
|---|---|---|---|
| board 1 | construct 23,655 / X3 G6b 18,420 (G10 16, G8 5) | G6b 103,999 / construct 59,551 (G10 413) | G6b 1,215,673 / construct 606,301 (G10 10,346) |
| board 2 | X10 4,151 / X5 60% 2,071 / X8 1,480 | X5 60% 16,421 / X8 4,752 | X5 60% 5,612,984 / construct 1,727,241 / X8 60,658 |
| Vault | X5 75% 185,434 / X4 131,637 | X5 75% 2,263,255 / X4 981,571 | X5 75% 33,681,482 / X4 21,613,791 |

X8, X10 and X11 are rare rejectors at Hard and Expert; X10 on the Vault rejected 1 candidate in 55 million at Expert (so uniqueness is not the binding gate; the carried-share window is).

## Results, unguided generator (v1 weighting only)

| | Medium | Hard | Expert |
|---|---|---|---|
| success of 10,000 | 10,000 | 10,000 | **8,920** |
| no redraw needed | 9,970 | 7,951 | 545 |
| cap hits | 30 | 2,582 | 153,764 |
| attempts Vault, accepted pass p50/p95/max | 19 / 185 / 1,988 | 227 / 1,533 / 1,998 | (summed) 17,518 / 56,640 / 75,035 |
| attempts board 2, summed | 2 / 9 / 65 | 5 / 49 / 1,051 | 4,983 / 18,398 / 31,690 |
| run time p95 one thread (Mac, loaded) | 0.040 s | -- | 2.609 s (max 3.148 s) |
| runes per run median / max | 21 / 24 | 22 / 24 | 23 / 24 |
| board 2 decoded p50 | 50.0 % | 55.6 % | 58.2 % |
| Vault decoded p50 | 74.1 % | 74.4 % | 74.4 % |

## Desk model beside mine (guided)

| | Desk | Mine |
|---|---|---|
| board 2 decoded at load p50 (M/H/E) | 49 / 52 / 54 % | 45.9 / 53.3 / 57.7 % |
| Vault decoded at load p50 | 69 / 70 / 72 % | 73.3 / 74.3 / 74.4 % |
| runes per run median (max) | 18 (20) / 19 (21) / 20 (22) | 22 (24) / 23 (24) / 24 (24) |
| run time (Python desk, Mac Rust mine) | 0.2 / 0.7 / 2.1 s | p95 0.023 / 0.128 / 0.846 s (one thread) |
| success | not stated (1000/1000 style) | 10,000 / 10,000 / 10,000 |

The desk model is off in the direction that matters: it has the Vault 3-5 points below the 75% X5 ceiling, mine sits against it. Runes are 4 higher at every tier, and Expert is at the 24 ceiling.

## Findings

1. Expert is at the rune ceiling. Median 24 runes, max 24, and the unguided version also 23 / 24. The 26-letter alphabet leaves the Vault at most 2-6 letters that no earlier board used, X4 asks for >=3 of them, X7 asks each Vault word to be >=40% decoded from carried, and X5 keeps the Vault <=75% decoded. Together they make the Vault a window of ~3 pp wide at the top of carried share (p10/p50/p90 = 72.7 / 74.4 / 75.0%). No run broke 24 because X1 forbids it, but there is no headroom: any tier or word-length change that lifts board 1 above 12 runes will make Expert infeasible.
2. Expert cost is draws, not seconds. A full Expert run draws a median ~4,300 and p95 ~20,000 candidates summed over redraws; that is cheap on the Mac only because my checker rejects X4/X5 from masks in microseconds. The accepted pass of the Vault is p95 1,777 draws and hits the 2,000 cap on a visible share of runs, so the cap, not the gates, decides how many runs redraw from board 1 (72% at Expert). If the cap stays at 2,000 per board, expect the iPhone to spend about 5x the Mac time (my guess; not measured), i.e. Expert p95 on the order of 4 s with the cheap screen and worse without it. That is over the 3 s line IF the iPhone is ~4x slower. This is the main open risk.
3. A smarter generator buys a lot: guided moves Expert from 8,920 to 10,000, cuts Vault summed-attempts p50 from 17,518 to 4,016 and unguided one-thread p95 from 2.6 s to 0.85 s. The remaining cost is picking silent words toward the 72-75% window; picking toward "decoded share just under 75%" explicitly (not tried) would likely cut Vault draws further.
4. X10/X11 are not the problem. After the share and new-rune gates pass, uniqueness and the stepwise solve almost never reject (Expert Vault: 3 rejections of 55 million). X8 matters on board 2 at Hard/Expert (4,752 / 60,658 rejections) but is minor.
5. Word reuse: at Expert, 34 runs (0.34%) shared 5-7 words with the previous run after 3 redraws because the "<=4" rule was relaxed to finish; Medium and Hard never needed it.
6. Medium and Hard are comfortable: board 2 median 1-2 draws, Vault median 8 / 50, 99.6% / 92.5% of runs finish without a redraw.

## STOP flags (strict reading of the brief)

- Any tier <10,000/10,000: not for guided; YES for the unguided generator at Expert (8,920).
- A run >24 runes: none.
- Run time p95 >3 s (Mac): no (Expert guided 0.85 s one-thread, 1.45 s with 9 threads on a loaded machine; unguided Expert one-thread p95 2.6 s, max 3.1 s).
- Not measured: iPhone time; Spanish/other languages (only English was asked).

---

## v1.1 appendix: c16-c18

# C16 Duel and C18 My Words census (2026-10-10)

Method: throwaway ignored Rust tests in a private scratch worktree of origin/main (27280a81), release build, run through the real v1 `Lexicon`, `bank::load("en", extra)` and `gates::count_solutions`. Repo worktree spz11 untouched; scratch worktree removed. Test source kept at `scratchpad/spz11/c1618.rs.txt`; raw output `c16-raw.txt`, `c18-raw-{30,150,600,2000}.txt`.

Deviations to know:
- The Python prototype dir (`.../scratchpad/spz/`, validity.txt) does not exist in this session. Large list = Hunspell en_US.dic stems + the same suffix expansion `seedgen.rs` uses (`/Users/eric/repos/spell-wasm/.corpus-cache/dicts/en/en_US.dic`, the same source as `assets/spelluzzle/seeds-en.json`).
- English only. Bank words after the v1 G1 filters (profanity, audio servable) are the pool.

## C16 Duel

### Definitions implemented
- Board: 6 spoken-slot words + secret = 7 words, one rune per unit. Gates: v1 G1 (alphabetic, in the tier's band, length in the tier's `shape().lo..=hi`: Easy 3-6, Medium 4-8, Hard 5-9, Expert 6-10, no duplicates), G2 (<=16 runes), G4 (each word shares >=2 distinct runes with the rest), G6a (every secret rune appears in another word), G6b (no single word decodes more than 60% of the secret's cells), P1 (no word has a collision group). Checker is separate from the generator (generator: sequential weighted draw, share>=2 with the union, union<=16, cube weight on shared runes, secret must be a subset of the union; cap 2,000 draws).
- Pool per tier: no-collision words in band and length: Easy 533, Medium 709, Hard 756, Expert 529. 2,000 boards generated per tier; a match draws two different boards at random from that tier's 2,000 (1,000+ distinct words reused, so a leg repeats words across matches; boards are not repeated within a match).
- Turn (assumed readings, spec was ambiguous): every slot starts unheard, nothing decoded. Listen-and-spell on any non-secret open slot: correct -> slot solved, its newly decoded runes claimed by the player at 1 point each; wrong -> entry cleared, slot marked heard, but it STAYS open and can be listened to again by either player (assumption). Spell unheard (only slots never listened to, including the secret): correct -> 2 points per newly decoded rune (3 for the secret); wrong -> nothing changes, slot stays unheard and can be tried again. A slot whose runes are all decoded closes unscored. Leg ends when all runes are decoded (equivalent to every slot solved or locked), or two passes in a row.
- NPC: each turn takes the visible-state option with the highest expected points: Listen = acc x undecoded runes of that slot; Unheard = 0.8 x acc x 2 (3 for secret) x undecoded runes, allowed only when the slot was never listened to, <=2 runes undecoded and exactly one validity-list word fits (same pattern, decoded runes match, undecoded runes take units not already decoded: the v1 G9 `solvable_given` rule on the large list). Ties broken at random. Success drawn at random with p = acc (heard) or 0.8 acc (unheard). The choice never touches the answer; a unique fit is by construction the answer.
- Match: two legs, scores carried, other player starts leg 2, leg-1 starter random. Draws count half. 10,000 matches per tier and accuracy, 3 accuracies.
- Claim check: ledger per rune; a rune claimed twice, or a decoded rune with no claimant, or a fully solved leg with a rune unclaimed, counts as a violation.

### Equal NPCs (10,000 matches per row)
| tier | acc | leg-1 starter match win (draw 1/2) | draw rate (match) | starter win, leg 1 alone (draw 1/2) | single-board draw | turns per match | claim violations | legs fully solved |
|---|---|---|---|---|---|---|---|---|
| Easy | 0.60 | 50.7% | 6.6% | 63.6% | 8.5% | 15.1 | 0 | 20000/20000 |
| Easy | 0.75 | 49.5% | 8.3% | 69.7% | 8.5% | 12.2 | 0 | 20000/20000 |
| Easy | 0.90 | 49.7% | 12.6% | 81.3% | 8.4% | 10.1 | 0 | 20000/20000 |
| Medium | 0.60 | 49.4% | 4.5% | 64.9% | 4.6% | 17.1 | 0 | 20000/20000 |
| Medium | 0.75 | 50.3% | 4.9% | 72.1% | 4.9% | 13.7 | 0 | 20000/20000 |
| Medium | 0.90 | 48.7% | 7.1% | 80.6% | 4.7% | 11.4 | 0 | 20000/20000 |
| Hard | 0.60 | 50.6% | 3.7% | 65.4% | 4.0% | 16.5 | 0 | 20000/20000 |
| Hard | 0.75 | 50.5% | 4.3% | 72.5% | 3.7% | 13.3 | 0 | 20000/20000 |
| Hard | 0.90 | 49.7% | 6.2% | 82.0% | 3.2% | 11.1 | 0 | 20000/20000 |
| Expert | 0.60 | 49.7% | 3.5% | 66.3% | 3.6% | 15.1 | 0 | 20000/20000 |
| Expert | 0.75 | 49.5% | 4.5% | 74.9% | 3.3% | 12.0 | 0 | 20000/20000 |
| Expert | 0.90 | 49.6% | 6.2% | 85.1% | 2.6% | 10.1 | 0 | 20000/20000 |

Board generation: 2,000/2,000 per tier, attempts p50/p95/max Easy 3/10/25, Medium 2/7/15, Hard 5/21/57, Expert 13/51/118. Runes per board avg 10.7 / 14.2 / 15.3 / 15.7; cells 27 / 40 / 52 / 66.

**STOP: not triggered.** All equal-NPC match rows lie in 48.7%-50.7% (standard error about 0.5 points). Versus the desk model: two-leg 49.8-51.0% desk, measured 48.7-50.7%, within noise except Medium 0.90 (48.7%, about 2.6 SE below 50, not a stop). Single board 63.6-85.1% vs desk 63-83%: Expert 0.90 is 2 points above the desk top. Draw rate 2.6%-12.6% (match 3.5%-12.6%) vs desk 3.5-7.6%: Easy at 0.75/0.90 (8.3%, 12.6%) is above the desk band because Easy boards are small (10.7 runes, 27 cells), so equal totals come up more often. Flag for Eric: Easy 0.90 draws 1 match in 8.

### O6: simulated player (fixed accuracy) vs NPC level, player match win rate (draw 1/2) / draw rate
NPC accuracy = clamp((prior_hits + player hits)/(prior_attempts + player attempts) - delta, 0.30, 0.95), re-read every NPC turn from the player's running record. This is `npc_acc_milli` (`src/boardgame/engine.rs:559-562`) over `human_acc_milli` (`src/boardgame/mod.rs:486-495`), floor/ceiling 300/950 at `engine.rs:10-11`, deltas Easy/Normal/Tough = 300/200/100 thousandths and Full prior 3.0/2.0 (`src/boardgame/rules.rs:44-52`, FULL at rules.rs:99-125; Sprint prior 0.75/0.5 at rules.rs:~138-155; Jr's flat 250 not used). NOT called: `npc_acc_milli` is a method on `BoardGameState` and needs a whole game state with players, so I copied the formula and constants (see C20). The player's record counts every spell attempt of the Duel (heard and unheard). Record starts fresh per match and carries into leg 2.

Full prior (3 hits / 2 attempts, i.e. NPC starts at the 0.95 clamp and decays toward player accuracy minus delta):
| tier | player acc | vs Easy | vs Normal | vs Tough |
|---|---|---|---|---|
| Easy | 0.60 | 41.8% / 9.3% | 30.0% / 8.8% | 20.0% / 7.0% |
| Easy | 0.75 | 49.9% / 11.1% | 38.9% / 11.6% | 30.8% / 9.9% |
| Easy | 0.90 | 58.6% / 13.8% | 50.5% / 15.0% | 46.0% / 14.7% |
| Medium | 0.60 | 47.4% / 5.6% | 35.4% / 5.5% | 23.9% / 4.9% |
| Medium | 0.75 | 54.8% / 6.5% | 44.0% / 7.0% | 34.5% / 6.3% |
| Medium | 0.90 | 62.9% / 7.3% | 54.7% / 8.3% | 48.3% / 8.0% |
| Hard | 0.60 | 47.5% / 4.9% | 35.2% / 4.9% | 25.1% / 4.0% |
| Hard | 0.75 | 55.3% / 5.4% | 44.4% / 5.6% | 34.9% / 5.2% |
| Hard | 0.90 | 63.7% / 6.1% | 55.4% / 7.1% | 49.7% / 6.9% |
| Expert | 0.60 | 46.1% / 4.7% | 34.5% / 4.7% | 25.0% / 4.2% |
| Expert | 0.75 | 52.9% / 5.7% | 43.8% / 5.8% | 35.8% / 5.7% |
| Expert | 0.90 | 62.6% / 6.4% | 54.5% / 6.6% | 48.4% / 6.8% |

Sprint prior (0.75/0.5; NPC tracks the player faster, so it is stronger early... and the NPC reaches the clamp-less steady state sooner):
| tier | player acc | vs Easy | vs Normal | vs Tough |
|---|---|---|---|---|
| Easy | 0.60 | 67.6% | 57.8% | 47.1% |
| Easy | 0.75 | 73.8% | 61.6% | 51.7% |
| Easy | 0.90 | 76.5% | 65.3% | 56.2% |
| Medium | 0.60 | 70.7% | 63.2% | 51.4% |
| Medium | 0.75 | 77.4% | 66.8% | 57.0% |
| Medium | 0.90 | 80.0% | 71.5% | 61.6% |
| Hard | 0.60 | 70.3% | 62.3% | 52.5% |
| Hard | 0.75 | 76.9% | 67.7% | 57.5% |
| Hard | 0.90 | 80.3% | 71.7% | 63.6% |
| Expert | 0.60 | 69.4% | 61.4% | 51.7% |
| Expert | 0.75 | 74.1% | 65.6% | 56.0% |
| Expert | 0.90 | 78.6% | 70.6% | 61.8% |

Reading for O6: the choice of prior is the whole story. With the Full prior a player at 0.75 beats Easy about half the time and Normal/Tough only 35-44%; with the Sprint prior the same player wins 52-77%. In the Full rows the NPC begins at 0.95 and takes about 10 of the player's spells to settle, which is a whole Duel (10-17 turns), so the Duel NPC never reaches its nominal trailing accuracy; the adaptive formula was designed for 80+ tile games. A 0.90 player at Tough is a coin flip (48-50%) under Full. If the Duel is to use the board game's NPC as it stands, pick a prior deliberately (one extra constant). Easy/Normal/Tough ordering holds in every row.

## C18 My Words

### Definitions implemented
- Real English bank: all four bands (easy, medium, hard, expert), v1 G1 eligibility (profanity, audio servable), 3-9 alphabetic cells, deduplicated: 2,606 list-candidate words. Top-up pool (Easy+Medium bands, 3-9 cells): 1,507. Jr top-up pool (words in the `Tier::Jr` eligible set, i.e. kid filter passed, Easy+Medium): 1,493, of which 1,240 have no collision group. Each synthetic list: N distinct words drawn uniformly from the candidates.
- Board: 5 spoken + secret. Gates implemented independently: G2, G3 (no two words in one collision group), G4, G5, G6a, G6b, G10 (earned >= 2/5, exact integer), G11 (cascade >= 1/4), G8 via `gates::count_solutions` on the large list (spoken slots read as their collision group, secret as any validity word). G1, G7, G9 not applied (list words are not band-limited; no silent words).
- Generator: for k = min(6, N) down to 3 list words; for each k try a top-up as the secret first (k <= 5), then a list word as the secret; each attempt draws k list words at random and fills the rest with top-ups chosen by weight (cube of 1 + single-held spoken runes covered + secret runes still needed), union <= 16 runes, share >= 2 runes with the words so far, last pick must close G5/G6a; then the full gate check. First success is served; none at k=3 means no board.
- Jr variant: board words (list and top-ups) must have no collision group (v1 G3 Jr rule), and top-ups come from the Jr-served set. "Jr + kid-allowed" additionally requires the list words themselves to pass the kid filter.
- 1,000 lists per size, per variant.

### The result depends on the draw budget (not given in the spec). Standard variant, share of lists by list words on the board (none / 3 / 4 / 5 / 6):
| size | 30 draws per (k, secret kind) | 150 draws | 600 draws | 2,000 draws |
|---|---|---|---|---|
| 3 | 19.1 / 80.9 / - / - / - | 5.6 / 94.4 / - / - / - | 1.9 / 98.1 / - / - / - | 0.9 / 99.1 / - / - / - |
| 5 | 7.6 / 29.2 / 59.4 / 3.8 / - | 0.8 / 19.2 / 76.2 / 3.8 / - | 0.1 / 13.5 / 82.6 / 3.8 / - | 0.0 / 11.7 / 84.5 / 3.8 / - |
| 8 | 0.9 / 12.8 / 43.7 / 42.5 / 0.1 | 0.0 / 1.5 / 38.5 / 59.8 / 0.2 | 0.0 / 0.0 / 37.0 / 62.8 / 0.2 | 0.0 / 0.0 / 36.3 / 63.5 / 0.2 |
| 12 | 0.7 / 7.0 / 35.2 / 57.0 / 0.1 | 0.0 / 0.0 / 8.5 / 90.7 / 0.8 | 0.0 / 0.0 / 1.6 / 94.6 / 3.8 | 0.0 / 0.0 / 0.7 / 90.2 / 9.1 |
| 20 | 0.1 / 4.3 / 27.5 / 67.6 / 0.5 | 0.0 / 0.0 / 2.0 / 95.9 / 2.1 | 0.0 / 0.0 / 0.0 / 92.8 / 7.2 | 0.0 / 0.0 / 0.0 / 80.7 / 19.3 |

(A draw budget of 150 per (k, secret kind) is about 1,350 draws worst case for a size-20 list, the nearest match to v1's 2,000-draw cap; 2,000 per phase is a generous ceiling. Columns are "no board / 3 / 4 / 5 / 6" as % of lists; none = fewer than 3.)

Headline at 150 draws (all three variants, % of lists: fair board | 3 / 4 / 5 / 6 list words):
| size | Standard fair / mix | Jr fair / mix | Jr + kid fair / mix |
|---|---|---|---|
| 3 | 94.4% | 3 only: 94.4 | 91.0% | 3 only: 91.0 | 92.8% | 3 only: 92.8 |
| 5 | 99.2% | 19.2 / 76.2 / 3.8 / 0 | 99.7% | 15.3 / 78.8 / 5.6 / 0 | 99.6% | 18.2 / 76.5 / 4.9 / 0 |
| 8 | 100% | 1.5 / 38.5 / 59.8 / 0.2 | 100% | 1.7 / 38.6 / 59.4 / 0.3 | 100% | 1.7 / 37.6 / 59.9 / 0.8 |
| 12 | 100% | 0 / 8.5 / 90.7 / 0.8 | 100% | 0.3 / 8.0 / 89.7 / 2.0 | 100% | 0 / 9.9 / 88.2 / 1.9 |
| 20 | 100% | 0 / 2.0 / 95.9 / 2.1 | 100% | 0 / 1.9 / 95.9 / 2.2 | 100% | 0.1 / 3.1 / 94.5 / 2.3 |

Versus the desk model (3/4/5/6): size 3 no board 5% desk, measured 5.6% at 150 draws (0.9% at 2,000); size 5 19/71/10 desk vs 19.2/76.2/3.8 (5-of-5 is never above 3.8%; the extra 5-word boards in the desk are not reached); size 8 10/62.5/26.5/1 vs 1.5/38.5/59.8/0.2 (generator finds 5 far more often than the desk expected); size 12 1/54.5/43.5/1 vs 0/8.5/90.7/0.8; size 20 2.5/44.5/48.5/4.5 vs 0/2.0/95.9/2.1. The desk model was pessimistic about how many list words fit; with real search the typical board holds 5 list words for any list of 8 or more, and a 3-word list gets 4+ never (a 3-word list holds exactly 3 and one top-up set). A size-5 list can never hold 6 words (the secret is the 6th word only if a sixth list word exists). Jr costs 3 to 5 points at size 3 and almost nothing elsewhere.

Reading: no size is below 91% fair at 150 draws, and every size of 5 or more is at least 99.2%. The only weak case is a 3-word list (5.6-9.0% unserved at 150 draws; 0.9-3.0% with 2,000 draws), so a policy of "3 words is the minimum list" works but ask for 5.

## v1.1 Phase F results: Par boards (2026-10-10)

Par (F15) and the board-type row (F22) are built behind `spell_flag_spelluzzle_par`, default off, and need `spelluzzle`. Hard and Expert only; never Spell Jr; English only.

- **Rules as built:** the one-short rule and par are computed by `src/spelluzzle/par.rs`; its par and par sets match an independent definition written in the tests (linear scan, own structures). Expert requires par 3 and Hard allows 2 or 3 (census decision 6). Gates are G1, G2, G4, G6 and P1 to P5.
- **Build step:** `scripts/spelluzzle-seeds.sh` now also writes verified Par boards to `assets/spelluzzle/seeds-en.json` as `[seed, par, seven pool indexes]`, proved under the 469,516-word list. Hard: 1,500 kept of 1,800 seeds (137 rejected by the large list). Expert: 1,500 kept of 1,800 (0 rejected; 129 could not be generated inside the 2,000-draw cap, which is the cost of requiring par 3). The device builds each board from its record, so it does no searching. The existing Medium to Expert seeds and the bank fingerprint did not change.
- **Audit:** `audit_par` (ignored, needs the dictionary) re-checked 300 sampled records under the large list: the recorded par holds, with one answer for every par set, and the independent definition agrees on the first eight of each tier.
- **Play:** every word is silent at load; one shared Listen bar above the keyboard replaces per-row Listen buttons (census C21); header "Listens n . Par p"; replays are free; the secret has no Listen. Stars: Sharp ear is "every slot you listened to was right the first time you spelled it after the Listen", Codebreaker is "solved at par or fewer Listens"; the result line reads Par, n under par or n over par.
- **Tests:** 44 Rust tests (A24 par is exact, A25 one answer with a par set heard, A26 any wrong word clashes, play rules and stars, offering rules, records current, freshness) and 7 browser tests; the whole app suite is 310 of 310. At 375x667 an Expert Par board keeps 7 rows of 10 cells at 32 pt or more with the keyboard up and no scrolling.
- **Not done in this phase:** the Maestro script for Par (A27: a scripted solve needs the seam), and the Par stores' per-tier best stars are written but not yet shown anywhere.
