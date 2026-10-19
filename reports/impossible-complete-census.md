# CC-IMPOSSIBLE-COMPLETE v1: Phase A census (C1-C10)

Run 2026-10-18 against origin/main 308faa84 in a throwaway worktree. Read-only: no bank, flag, Climb or app code changed. Scripts live outside git (scratchpad); this report is the only artifact.

## 0. HALT check and stop-and-ask triggers

| Check | Result |
|---|---|
| Phase M-A pool-currency check | **Passes** (`build_pools.py --check`: all 15 artifacts match a fresh build) |
| C3 cost approval | Only the 200-word sample was run, as the spec requires. Full-run projection is in C3; **it needs Eric's approval before Phase B** |
| Rules net over 60% of Expert | **No.** Highest is fr at 16.6% |
| Conflict with a signed section 2 decision | **None with section 2.** Several conflicts between the spec and the code (F8, F9, I10, D-C15, vi audio, de capitals), listed in section 12 |
| 150-175 band after rules net | None. Smallest rules-net pool is hi at 271. Loopback numbers below may move this |
| Loopback flag rate above 25% | None |
| Climb miss costs more than one word (section 10) | **Yes. Trigger fired.** See C9 and section 12 item 1 |
| Misses behaviour for Impossible unspecified | Specified: Mix spec M9 (tagged `impossible` + language, re-served at Impossible rules in its own language). No stop |

## C1. Per-language state

Expert rows are the served tier. en uses the re-tier Expert (`en.retier` must be on). zh/ja show 0 in the pool artifact because the builder returns before reading the tier; true row counts are in C5.

| Lang | Expert rows | Table status | Sound-alike sets | Pool now | Locked reason now |
|---|---|---|---|---|---|
| en | 279 | verified | 280 | 271 | open |
| es | 2999 | verified | 7 | 2995 | open |
| ru | 3000 | verified | 33 | 2997 | open |
| fr | 3000 | missing | 0 | 0 | table-missing |
| de | 2996 | missing | 0 | 0 | table-missing |
| pt | 3000 | missing | 0 | 0 | table-missing |
| pl | 2999 | missing | 0 | 0 | table-missing |
| vi | 2500 | missing | 0 | 0 | table-missing |
| ko | 1455 | missing | 0 | 0 | table-missing |
| fil | 2000 | missing | 0 | 0 | table-missing |
| ar | 3000 | missing | 0 | 0 | table-missing |
| sw | 683 | missing | 0 | 0 | table-missing |
| hi | 287 | missing | 0 | 0 | table-missing |
| ja | 2,344 (see C5) | missing | 0 | 0 | drawn-entry |
| zh | 2,984 (see C5) | missing | 0 | 0 | drawn-entry |

Only en (271), es (2,995) and ru (2,997) have pools. Free tier reaches English alone, so Mix reads "Needs two Impossible languages" today, which is the wrong reason for a free player (F3).

## C2. Rules-net dry run (10 table-less languages)

The nets are deliberately coarse sound keys (rules in the script; each merges what the active voice could merge). A group of two or more bank Expert words with one key is flagged in full. Counts are rules net only, **before** the loopback net and before anything Eric reviews.

| Lang | Expert | Flagged groups | Flagged words | % | Pool after rules net |
|---|---|---|---|---|---|
| fr | 3000 | 231 | 499 | 16.6 | 2501 |
| de | 2996 | 6 | 12 | 0.4 | 2984 |
| pt | 3000 | 123 | 247 | 8.2 | 2753 |
| pl | 2999 | 18 | 36 | 1.2 | 2963 |
| vi | 2500 | 1 | 2 | 0.1 | 2498 |
| ko | 1455 | 11 | 22 | 1.5 | 1433 |
| fil | 2000 | 20 | 40 | 2.0 | 1960 |
| ar | 3000 | 33 | 66 | 2.2 | 2934 |
| sw | 683 | 1 | 2 | 0.3 | 681 |
| hi | 287 | 8 | 16 | 5.6 | 271 |

Five sample groups per language, for Eric's review (random, seeded):

- **fr**: transport/transports; spécialisé/spécialisée; éprouver/éprouvé; commencer/commencée; spécimen/spécimens
- **de**: bayerischen/bayrischen; sehnen/söhnen; übernahmen/übernehmen; stehenden/ständen; gefordert/gefördert
- **pt**: profissão/profissões; evidência/evidências; estratégia/estratégias; população/populações; indústria/indústrias
- **pl**: niezależne/niezależnie; specjalne/specjalnie; automatyczne/automatycznie; genetyczne/genetycznie; butelka/pudełka
- **vi**: chính sách/chính xác
- **ko**: 학장/학창; 교재/교제; 조개/쪼개; 바이어/파이어; 뻥/펑
- **fil**: kombinasyon/kumbinasyon; sunod-sunod/sunud-sunod; pinag-aaralan/pinag-aralan; inihahanda/inihanda; inhenyeriya/inhinyeriya
- **ar**: شبيه/شبيهة; أبقى/أبكي; الأسبانية/الإسبانية; الإستراتيجية/الاستراتيجية; الأفريقية/الإفريقية
- **sw**: unaonekana/unaoonekana
- **hi**: अंतरराष्ट्रीय/अन्तरराष्ट्रीय; उष्णकटिबंधीय/उष्णकटिबन्धीय; प्रस्तुतिकरण/प्रस्तुतीकरण; अर्धचंद्राकार/अर्धचन्द्राकार; परिणामस्वरुप/परिणामस्वरूप

Reading these:
- **fr, pt**: most flags are inflection pairs the voice cannot tell apart (transport/transports, profissão/profissões). They are real sound-alikes, not noise.
- **pl**: the ś/ć/ni merger over-flags adjective/adverb pairs (niezależne/niezależnie). Coarse by design; it costs pool words, not fairness.
- **de**: the net finds only 6 groups (bayerischen/bayrischen, sehnen/söhnen, übernahmen/übernehmen...). No case pairs exist anywhere in the German bank (C4).
- **vi**: one group (chính sách / chính xác, from the tr/ch and s/x merger). The net keeps tones. The rule is per syllable; a first version applied it once per word and found nothing, which was a bug, now fixed.
- **ko**: liaison, nasalisation and palatalisation are modelled; plain/tense/aspirate are folded together (도래/또래, 교재/교제, 조개/쪼개). Tensification across syllables is NOT modelled, so ko may under-flag until Phase B tunes it.
- **sw, hi**: tiny Expert tiers (683 and 287). hi stays at 271 after the net.
- **ar**: ة/ه and hamza-seat merging gives 33 groups. The deeper Arabic spelling issue is in C4.

## C3. Loopback net on a 200-word sample

**Method and two honest deviations from the spec.**
1. Each sampled Expert word is fetched from `/api/speak` (the clip the player hears, shipping voice) and transcribed by **whisper.cpp ggml-small**, one transcript. The spec says top-3 transcripts; whisper-cli exposes no n-best list. So these rates are a **lower bound** on what a top-3 Google STT run would flag. A word is flagged when the transcript (the clip says the word twice; I collapse that) equals a **different** word in the language's full bank (all four tiers).
2. Vietnamese bank rows contain a space and `/api/speak` rejects spaces (HTTP 400, `validate_word`), so I sent the syllables joined with a hyphen. See section 12 item 5.

Whisper-small is at its floor for fil/hi/vi (memory: spell-audio-measurement-limits), so "self-heard" is the sanity column: the share of clips where the transcript contained the spoken word itself. **Where self-heard is low (hi 6/200, vi 37/200, ar 63/200) the flag rate says nothing about sound-alikes, only that the recogniser cannot hear the word; treat those three rates as uninformative, not as "clean".** The first pass matched single tokens inside a multi-token transcript and flagged vi at 67%; that was my matching bug, fixed to whole-transcript matching and rescored offline.

| Lang | Sampled | TTS failed | Flagged | Flag rate | Self-heard |
|---|---|---|---|---|---|
| ko | 200 | 0 | 3 | 1.5% | 129/200 |
| fr | 200 | 0 | 37 | 18.5% | 88/200 |
| pt | 200 | 0 | 0 | 0.0% | 173/200 |
| pl | 200 | 0 | 0 | 0.0% | 175/200 |
| de | 200 | 0 | 5 | 2.5% | 162/200 |
| hi | 200 | 0 | 0 | 0.0% | 6/200 |
| vi | 200 | 0 | 1 | 0.5% | 37/200 |
| fil | 200 | 0 | 2 | 1.0% | 103/200 |
| ar | 200 | 0 | 3 | 1.5% | 63/200 |
| sw | 200 | 0 | 1 | 0.5% | 111/200 |

Flagged examples (spoken word -> different bank word heard):

- **ko**: 뜬 -> 등; 단면 -> 반면; 즙 -> 집
- **fr**: musicales -> musical; prisonniers -> prisonnier; militaires -> militaire; éloignées -> éloigner; propriétaires -> propriétaire
- **pt**: (none)
- **pl**: (none)
- **de**: erschienenen -> erschienen; bestimmten -> bestimmen; bayrischen -> bayerischen; Expertin -> Experten; Wales -> weils
- **hi**: (none)
- **vi**: khăng khăng -> khăn
- **fil**: anung -> anong; industriyal -> industrial
- **ar**: اصل -> أسلوب; الكومبيوتر -> الكمبيوتر; الأفق -> الأفضل
- **sw**: hutengeneza -> kutengeneza

**Extrapolated pool after both nets** (rules-net pool x (1 - sample flag rate); the two nets overlap, so this is conservative):

| Lang | Rules-net pool | Sample flag rate | Extrapolated pool | vs 150 floor |
|---|---|---|---|---|
| fr | 2501 | 18.5% | 2038 | clear |
| de | 2984 | 2.5% | 2909 | clear |
| pt | 2753 | 0.0% | 2753 | clear |
| pl | 2963 | 0.0% | 2963 | clear |
| vi | 2498 | 0.5% | 2485 | clear |
| ko | 1433 | 1.5% | 1411 | clear |
| fil | 1960 | 1.0% | 1940 | clear |
| ar | 2934 | 1.5% | 2889 | clear |
| sw | 681 | 0.5% | 677 | clear |
| hi | 271 | 0.0% | 271 | clear |

**Projected full-run cost (all 10 languages, 21,920 Expert words).**
- TTS: 186,416 characters, spoken twice per clip (the "normal" variant) = about 373k billed characters. At the Google WaveNet/Neural2 list price I recall ($16 per million; **unverified**) that is about **$6**. sw is Azure, same order. Many clips will be cached after this sample; the cache key does not change for a sample.
- STT with whisper.cpp locally: $0, roughly 2 s per clip on this Mac, so several hours for the full set (the 2,000-clip sample took roughly 35 minutes, so the full set is about 6-7 hours). Large-v3 would be several times slower.
- STT with Google (`/api/stt`, which today returns only 2 alternatives): about 21,920 clips x $0.006 per 15 s billing unit (list price, **unverified**) = roughly **$130**.
- Recommendation: approve the TTS spend and run whisper-small + large-v3 locally; or add an n-best parameter to `/api/stt` and spend the $130 if you want a true top-3. Your call; nothing is approved yet.

## C4. Voice-invisible spelling distinctions

Counts are pair groups in the whole bank (all tiers, because a competing form in any tier is a valid bank word) and how many touch an Expert row. "Expert words affected" is what strict + pair-excluded would remove.

| Lang | Distinction | Whole-bank pair groups | Touching Expert | Expert words affected | Examples |
|---|---|---|---|---|---|
| de | noun case (Weg/weg) | **0** | 0 | 0 | none in the bank |
| ar | final ة/ه | 17 | 12 | 12 | موجة/موجه, لعبة/لعبه |
| ar | hamza seats and tashkeel (one fold) | 80 | 60 | 81 | الإلكترونية/الالكترونية, أقوى/اقوى |
| hi | nukta (ज/ज़) | 7 | 0 | 0 | none in Expert |
| hi | chandrabindu vs anusvara | 2 | 0 | 0 | none in Expert |
| hi | nasal conjunct vs anusvara (न्द vs ंद) | 34 | 10 | 16 | उष्णकटिबंधीय/उष्णकटिबन्धीय |
| es | acute accent (tú/tu style) | 49 | 19 | 24 | incremento/incrementó, indice/índice |
| fr | accents (cible/ciblé) | 133 | 43 | 67 | constitue/constitué |
| pt | accents | 21 | 13 | 14 | distancia/distância |
| vi | tone-only pairs | 284 | 22 | 44 | nghị viên/nghị viện |
| ru | ё/е | 28 | 2 | 2 | четырех/четырёх |
| pl | ó/u, ż/rz, ch/h (spelling-only pairs) | 0 | 0 | 0 | none |

Confirmed candidates: ar ة/ه and hamza seats, hi nasal conjunct. **Removed candidates:** de case (no pairs exist), hi nukta and chandrabindu (no Expert rows). **Additions you did not list:** fr and pt accent pairs, vi tone-only pairs, ru ё/е. Of these, fr/pt/es accents are heard (a vowel quality differs: ciblé vs cible) so they are not voice-invisible and need no ruling; vi tones are audible and are kept in the key; ru ё/е is audible when stressed. I have not ruled on any of this. O-C2 stays at its build default (lenient) until you rule.

**German bigger issue:** 950 of the 2,996 de Expert rows are capitalised (nouns). `build_pools.py` fails the build on any capitalised row (v1 F2 step 6, proper-noun assertion). The moment German gets an exclusion gate, the pool build will abort. That assertion must be split into proper nouns vs common nouns for de, or the pool lowercases (which defeats D-C4). Needs your decision in Phase B.

## C5. zh and ja

- **Drawn-entry build status**: `quickbee_draw::LANGS` has zh, ja and ko, flags `quickbee_draw_zh/ja/ko`, default OFF on main (`src/flags.rs`). My memory notes say build 283 shipped with zh/ja drawn ON on a release branch, pending Eric's handwriting pass (Phase D); I did not verify that branch.
- **zh** (`assets/words/zh/expert.txt`, 2,984 rows, `reading|hanzi`): rows sharing a *toned* reading with another bank word: **250** (pool after exclusion 2,734). Sharing a *toneless* reading: **586** (pool 2,398). Unclear which one F1 needs; toned is the true homophone. Either clears 150 by a wide margin.
- **ja** (`JA_EXPERT`, 2,344 rows, all kana): a kana-normalising key (づ/ず, ぢ/じ, を/お, long vowels) finds **0** colliding rows, since the bank holds readings, not kanji. Pool would be 2,344. The long-vowel merges (こおり/こうり) are not collisions *between* bank words but are still voice-invisible *inside* a word; that is a C4-style ruling (O-C1 area), not an exclusion.
- Both tables are `missing`; neither gets a gate until Phase B.

## C6. Mix availability function

- **Location**: `src/impossible.rs`. `lang_availability(lang, &Inputs) -> Availability` and `mix_status(surface, &Inputs) -> MixStatus`. Pure functions over `Inputs`; one storage read in `with_live_inputs`.
- **Return types today**: `Availability { Hidden, Locked(LockReason), Available(usize) }` with `LockReason { NotEnoughWords, DrawnEntry, NeedsComplete }`; `MixStatus { Hidden, NeedsTwoLanguages, Available(Vec<&str>) }`.
- **Callers**: **none outside `src/impossible.rs` and its tests.** The only other mention is a doc comment in `src/flags.rs:77`. No surface calls it yet, so F3 can reshape the return type without a migration.
- **Do reason codes fit?** Mostly, with three gaps:
  1. `Hidden` merges `flag_off` and `profile_blocked`, and `mix_status` has no `needs_complete`, so a free English-only profile gets `NeedsTwoLanguages`. That is exactly the wrong copy F3 describes.
  2. `NotEnoughWords` merges `pool_below_floor` and `no_exclusion_gate`.
  3. Mix also needs `language_locked:*` carried through for the F11 report. The enum is small and untouched by callers, so the change is local.

## C7. Flag storage and the signed kill file

- **Storage**: `src/flags.rs`. Every flag is a `pub fn -> bool` reading `localStorage['spell_flag_<name>']` through `resolve(stored, default)`; the file's own comment says flags are local, not remote (v1 F12 said remote and was never built).
- **Backend**: `backend/app.py` (Flask, gunicorn) behind Caddy (`~/spellgame-server/Caddyfile`, `/api/*` to :8000, everything else a static file server) behind a Cloudflare Tunnel. Routes: `/api/health`, `/speak`, `/check`, `/meaning`, `/defpool`, `/sentence-audio`, `/stt`, `/notify`, plus Climb and auth blueprints.
- **Can it serve a static signed file?** Yes, two ways: Caddy already serves the web root statically, so `flags.json` + `flags.sig` can be dropped there with no Flask change; or a new Flask route. There is **no existing signing code** in the backend (no HMAC or signature use found in `app.py`). A scheme needs a keypair: a public key compiled into the build and the private key kept on the Mac mini. Every client (web, iOS, Android) already knows `api_base()`.

## C8. Normal-practice replay, attempt and Misses paths

- **Replay**: `game::note_replay` only sets `aids.replayed = true` (`src/attempts.rs:44`). **There is no replay cap anywhere**; "one replay per word" (I10) is new code, not a change to existing code.
- **Attempts**: `game::extra_attempt_ctx` (`src/game.rs:2600`) grants a second try when `flags::attempts_shields() && s.extra_attempts` and the level is not climb, versus, daily or review. It is **ON by default in Kid Mode and a user toggle otherwise**. Impossible must be added to that exclusion list or I10 fails for any player with the toggle on. The Climb path also has a shield prompt (`on_wrong_climb`).
- **Misses write path**: `game::record_miss_routed` -> `misses::add_miss` (plus the zh tone-drill router and `pick_queue::note_miss`), called from `record_first_miss`, which is the one place a first-submission miss is recorded. F5's Misses write should call that function with an `impossible` tag per Mix spec M9.

## C9. Climb scoring, Daily seed, BoardGame trap (read-only locations)

- **Climb client**: `src/game.rs` `note_climb` (line 320) and `on_wrong_climb` (2878); submission in `src/climb.rs:355` (`POST /api/climb/submit-chain`). **Server**: `backend/climb.py` `submit_chain`.
- **What the Climb actually scores**: a **chain length** (consecutive correct words, `s.streak`), not points. There are **three boards** (`VALID_DIFFICULTIES = medium, hard, expert`) keyed by difficulty and locale. A correct word is +1; the band promotes after 3 correct (`CLIMB_PROMOTE`). A **miss drops the band, resets progress and breaks the streak to zero**, unless a shield is spent (`attempts::spend_shield`). The server rejects `chain > MAX_CHAIN (500)`, `wordCount < chain` and `durationMs < chain x 800`.
- **Daily seed**: `src/daily.rs` `build_words(lang, date, kid)` and `build_words_from_seed(lang, tier, seed, count)`, with `walk_seed`. Randospell and Forge have their own seeds (`randospell/daily.rs`, `forge_screen.rs`).
- **BoardGame trap**: `src/boardgame/engine.rs:472` (`SpellKind::DoubleExpert`, second site at 538 and 674). Tests in `src/boardgame/tests.rs:603`.

Break-even for x1.5 (informational; D-C9 stays initialed): see section 12 item 1.

## C10. Answer-field settings

| Platform | Finding |
|---|---|
| All (iOS, Android, web) | The round has **no answer `<input>`**. Letters come from the custom on-screen keyboard plus physical `keydown` (`src/lib.rs:715`, `src/keyboard.rs`). The system keyboard, autocorrect and predictive text never open during a round. |
| Hidden `prInput`, `wpInput` | `inputmode="none"`, autocomplete/autocorrect/autocapitalize off already (`index.html`). |
| Other text fields | `nameInput`, search and import fields set autocorrect and autocapitalize off. |

So D-C15 and acceptance test A-5 as written (assert the attributes on every Impossible answer field) are **vacuous**: there is nothing to configure. The meaningful test is that an Impossible round never mounts a text input. A physical or Bluetooth keyboard still delivers `keydown` and bypasses the on-screen board; no autocorrect applies, but case then comes from the Shift key, which matters for German case at Impossible.

## 12. Conflicts between the spec and the code (needs your ruling)

1. **F9 Climb multiplier does not map onto the Climb.** The Climb scores an integer chain, not per-word points; a miss zeroes the chain (cost = the whole chain, not one word); there are three boards (medium/hard/expert) per locale, not one universal board; the server rejects `chain > wordCount`, so a x1.5 chain would be refused as a cheat. This is the section 10 trigger. Break-even for a per-word-credit reading: if per-word accuracy is p, expected chain is p/(1-p). Expert at 90% gives 9. An Impossible run at x1.5 matches that at p = 0.857, so Impossible only pays if the player holds 86% or better. D-C9 stays initialed, but F9 needs a rewrite before Phase E: either no Climb effect, or Impossible as a fourth board (excluded by the carried "no separate Impossible Climb board"), or an explicit server-side rule for credited chain. **Not touched.**
2. **F8 trap is two words, not one.** `DoubleExpert` serves two Expert words in a row (steps 0 and 1) and a miss costs the next turn. The spec says it "becomes one Impossible word". That halves the trap and changes its name ("Double expert" is a localised string in every locale). Needs a ruling: one Impossible word, or two.
3. **I10 vs the existing extra-attempt toggle** (C8). Not a spec conflict, but a required exclusion.
4. **D-C15/A-5 is vacuous** (C10).
5. **Vietnamese audio path.** All 2,500 vi Expert rows are two-syllable and space-separated; `/api/speak` rejects spaces with HTTP 400. `src/api.rs::speak_url` passes the word raw, and I found no vi special-case. Either the app plays vi through native on-device TTS (a source named `native-tts` exists) or vi server audio is broken. I did not resolve which; it decides whether a server loopback is even the right test for vi.
6. **German capitals** (C4): the builder's proper-noun assertion will abort the de pool build.
7. **TTS billing is no longer lapsed**: fr, pl and (through the sample) all ten Google languages synthesised new clips. My memory note of 2026-09-30 is stale on that point.

## Next

Phase B is blocked until you (a) approve the C3 full-run cost and engine choice, (b) rule on items 1, 2 and 6 above, and (c) review the C2 sample groups and C3 flags per language.
