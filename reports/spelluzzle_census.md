# CC-SPELLUZZLE §4 census

**Run 2026-10-09 against `origin/main` c052f07c. Read-only: no repo code, bank or registry changed.**
**STOP — Phases A–D are blocked on Eric's review and on O1–O3.**

Compiled from three read-only surveys (appendices). C7 ran a Python prototype generator and an independent checker on the real English bank; it is a feasibility measurement, not the Rust implementation.

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
