# CC-TELEMETRY-FOUNDATION §0 census

> **Resolved 2026-09-18:** Eric signed R1–R7, then R8–R11, as recommended. Phase A
> is built on `cc-telemetry`; see `docs/CC-TELEMETRY-FOUNDATION.md`.

## v1.1 update (2026-09-18, after R1–R7 were signed and v1.1 rescoped to F1 + F5)

**Outcome: STOP for review again.** v1.1's §0 items 1, 2 and 4 are answered
below (unchanged from v1). Items 3 and 5 are new, answered here. One of v1.1's
stop triggers fires (Jr unknown state), but the signed R3 already covers it.
Four rulings are still needed, listed at the end of this section.

### Item 3 — what Apple already captures

Checked read-only through the App Store Connect API (fastlane key), app `6788927897`, builds 215–226:

| Channel | Data today |
|---|---|
| TestFlight crash feedback (`betaFeedbackCrashSubmissions`) | **0 submissions, ever** |
| TestFlight screenshot feedback | 1 (2026-08-27, iPhone18,3, iOS 26.6) |
| Xcode Organizer diagnostics (`builds/{id}/diagnosticSignatures`) | **none**: 404 on all 12 recent builds |
| Organizer metrics (`perfPowerMetrics`) | empty (Apple needs enough opted-in devices) |
| MetricKit in the app | not linked |
| Local Organizer cache | an archive of 1.1 (146) only, with no downloaded crash logs |

**The gap is structural, not just a lack of volume.** Apple's channels see native process crashes only.
- A JS exception or a WASM panic in the WKWebView doesn't crash the app process. At most it kills the web-content process, and Apple attributes that to WebKit, not to the app.
- spellgame.net (D10) has no Apple channel at all.

So F1's custom code has to cover all JS and WASM errors and all pack-load failures. MetricKit adds native hangs, launch time and memory kills for free, and is still worth bridging.

### Item 5 — what the server logs record

`~/spellgame-server/logs/` holds four LaunchAgent logs: backend, caddy, tunnel and awake.
- **Caddy has no `log` directive** (`~/spellgame-server/Caddyfile`), so no access log exists.
- Gunicorn has no access log either.
- The backend log (802 lines since 2026-07-27) holds only startup lines and `app.logger` errors, for example `TTS failed for '<word>' (<variant>)`.
- The tunnel log holds only cloudflared connectivity.

| F5 metric | Recorded today? |
|---|---|
| TTS latency p50/p95 | **no** |
| Cache hit rate | **no**: `cache_path_for` hits and misses aren't logged |
| Error rate per language | partly: TTS failures are logged with the word, but not the lang, and with no request count to divide by |
| Client IPs | **not stored anywhere** (good) |

So F5's "server side first, from existing logs" can't be done from existing logs. It needs one structured log line added to `/api/speak` (R8).

**Also found:** in stub mode (no `RESEND_API_KEY`), `auth.py` logs the recipient email **and the verify-email link with its token** in plain text. Today's log has one such entry, for `cc_smoketest@example.com`. If real sign-ups ever ran in stub mode, their emails and tokens are in that file.

### Boundary check (v1.1 table)

| Boundary | Holds? |
|---|---|
| Jr resolver (CC-ONBOARD-JR) | **No unknown state** (§4 below). Covered by signed R3: add a three-state `audience()` to CC-ONBOARD-JR that telemetry reads |
| Audio resolver | Holds. Outcome is `api::last_audio_source() == "none"`, and counts per language are observable |
| Freshness (CC-PERSIAN-FOUNDATION F6) | Spec only. **No exposure ledger exists in `src/`.** Not touched by v1.1, but the "F4 → debug assertion in the ledger" note depends on it being built |
| Clarity (CC-AUDIO-REPLAY F6) | **No F6 exists.** CC-AUDIO-REPLAY is the replay-banner fix with no clarity flags. Not touched, so harmless, but the cite is wrong |
| Learning data (CC-LEARNING-ENGINE D5) | Holds. v1.1 reads none of it |
| Remote flags ("existing flag system") | **Doesn't exist** (§6). Covered by signed R4: `GET /flags` on the Worker |
| Cited docs | `CC-LEARNING-MODES` isn't in `docs/`. CC-PRACTICE exists as `CC-PRACTICE-PLAN.md` / `-V2-PLAN.md` |

### Earlier rulings under v1.1

| Ruling | Status under v1.1 |
|---|---|
| R1 supersede zero telemetry | **Conflicts with v1.1's D-TEL**, which keeps D5 and argues compatibility instead. Needs a choice (R9) |
| R2 derived word IDs | moot: v1.1 sends no word IDs |
| R3 Jr three-state `audience()` | **still needed**, for I6 |
| R4 `/flags` on the Worker | **still needed**, for I7 |
| R5 outcome enum | moot: I3 forbids outcomes |
| R6 F4 Daily window | moot: F4 dropped |
| R7 privacy.html fix | **still needed**. Separate task filed |

### New rulings needed (recommendations)

| # | Question | Recommendation |
|---|---|---|
| R8 | F5 has no server logs to read | Add one structured line per `/api/speak`: `lang, variant, cache=hit\|miss, ms, status`, with **no IP and no word**. A daily script over it gives p50/p95, hit rate and error rate per language. This is server-only and changes nothing in the client |
| R9 | R1 vs D-TEL | **D-TEL wins** (it's newer and narrower). Replace R1 with an annotation on CC-LEARNING-ENGINE D5 and CC-BUY-DRIVERS S1: "zero telemetry = zero learning or gameplay data; crash and performance diagnostics per CC-TELEMETRY-FOUNDATION v1.1 are permitted." Without it, both say "zero telemetry" literally, and the next spec will cite them against F1 |
| R10 | Stale facts in v1.1 | (a) The backend is the **Mac mini + Cloudflare Tunnel**, not a Pi. D5's reasoning holds unchanged. (b) "Must not delay build 56": TestFlight is at **build 226**. Name the release meant. (c) Fix the AUDIO-REPLAY F6 and LEARNING-MODES cites |
| R11 | D5 status | v1.1 marks D5 **OPEN**, but Eric signed **D5 = Cloudflare Worker** on 2026-09-18 (v1 thread). Record it as signed |

With R8–R11 ruled, the v1.1 gate is met: D5 is signed and CC-ONBOARD-JR Phase A is merged (`9030e88f` on `main`). Phase A would start with the R3 `audience()` amendment, then the R4 `/flags` Worker, then the schema.

---

## v1 census (original run, still accurate for items 1, 2 and 4)

Run 2026-09-18 on branch `cc-spelldoku` (HEAD `82fdde48` = `main` + 3 commits;
nothing telemetry-related differs from `main`). Read-only: no code written.
Every load-bearing claim below was re-checked against `main` by hand.

**Outcome: STOP.** Two of the spec's own stop-and-ask triggers fired (Q3 stable
IDs, Q4 Jr resolver). Four ownership boundaries the spec assumes don't hold
(§B). Nothing past §0 is built.

---

## Stop-and-ask triggers

| Trigger | Fired? | Summary |
|---|---|---|
| Any language <100% stable word IDs | **Yes, strictly** | No bank row stores an ID in any language. The IDs are derived from the spelling (§3). For zh, the key the stores already use is ambiguous. |
| Third-party SDK already transmitting | No | The client has no analytics, crash or attribution SDK (§2). |
| Jr resolver not a definite Jr/standard/unknown | **Yes** | It has two states. **An unanswered age gate resolves to Standard** (§4). |

## Boundaries that don't hold (spec: "stop and ask")

- **B1. Zero-telemetry posture is already SIGNED the other way.**
  - `CC-LEARNING-ENGINE.md:110` D5 is marked **DECIDED**: "zero telemetry … No silent experimentation on players."
  - `CC-BUY-DRIVERS.md:15` S1 makes that global.
  - The posture has been enforced since:
    - `CC-TRANSLATE-SCREEN` F14 was **cut** for it (2026-09-12).
    - `CC-WORDGRID` E5 refused pooled player data for it (2026-09-18).
  - This spec, and signed D3 (on by default) in particular, reverses a signed decision. Supersede it explicitly (recommended below) before any code.
- **B2. No remote flag system exists.**
  - The spec's rule "Reuses it for the kill switch. No new flag system" can't be met (§6).
- **B3. No engine outcome enum exists.**
  - `TRAP_MISS` and `MISSPELLED` are not base-game outcomes.
  - `valid_other_sense` is a thread-local bool.
  - F2's rule "logs the class the engine already computed" can't be met without creating that class (§5).
- **B4. The owner specs named in the ownership table are mostly absent from `docs/`.**
  - Absent: `CC-BUILD219-FIXES`, `CC-RU-ORTHO`, `CC-SENSE-CUE`.
  - `CC-ONBOARD-JR` has only `onboard-jr-inventory.md`.
  - The boundaries can't be checked against their text.

---

## 1. Existing egress

Every live request goes to `https://spellgame.net` (`index.html:2823`, `api_base()` at `src/api.rs:132`). The client makes no third-party calls.

| Endpoint | Trigger | Sends |
|---|---|---|
| `/api/speak` (+ prefetch) | play/replay a word; warm next word | word, lang, variant |
| `/api/check` POST | English answer submit (`game.rs:1599`) | word, **player's typed answer** (`api.rs:476`) |
| `/api/meaning`, `/api/sentence-audio` | definition / sentence hint | word, lang |
| `/api/defpool` | enter Definition Match | lang, tier |
| `/api/notify` POST | "Notify me" on a coming-soon language; queue flushed every launch (`lib.rs:264`) | language, **persistent random install id** `spell_install_id_v1` (`notify.rs:12`) |
| `/api/auth/*` | sign-in, account settings | email, password, code, username, bearer token |
| `/api/climb/leaderboard`, `submit-chain`, `report-name` | leaderboard, end of run while logged in | token, difficulty, locale, chain, wordCount, durationMs |
| `/api/match*` | Online Spell-Off (flag off by default) | token, lang, tier, score, elapsed_ms |
| `/packs/{lang}/v1/*` | offline pack download (flag off by default) | lang |
| `/api/stt` POST (Swift) | iOS Spell It Out Loud "server" rung; one-time consent card; not in Kid Mode or the Education build (`spell_aloud.rs:672`) | **base64 raw mic audio**, lang, context phrases |
| wasm, js, fonts, sw precache | boot | nothing (all same-origin; no CDN, no Google Fonts) |

**Capacitor plugins:** none of them make network calls. This covers native-audio, filesystem, haptics, local-notifications (local only, no push entitlement) and share.

**Server-side third parties:**
- Google TTS and Azure TTS receive the word.
- **Google Speech-to-Text receives mic audio** (`backend/app.py:691`).
- dictionaryapi.dev and Wiktionary receive the word.

**Out of scope, but found:**
- `privacy.html` says "Spell does not record audio" and that only the dictionary word is sent. Both are false today:
  - The consented `/api/stt` path uploads mic audio.
  - `/api/check` sends the typed answer.
  - `/api/notify` sends a persistent install id.
- `android/app/src/main/assets/public/` is stale (Jul 21). It still loads `ocr-shim.js`, which fetches Tesseract from cdn.jsdelivr.net and tessdata.projectnaptha.com. `npm run android` re-syncs it, but a build made without a sync would ship that egress.

## 2. Third-party SDKs

- **None present, transitive included.** I checked package-lock, Cargo.lock, SPM `Package.resolved` (capacitor-swift-pm and ion-ios-filesystem only), Android gradle and the index.html scripts.
- The `com.google.gms:google-services` classpath is declared (`android/build.gradle:11`) but is **inert**. It only applies when `google-services.json` exists, and there's no such file.
- **F1 prerequisites:**
  - No MetricKit, no crash reporting, and no app-level `PrivacyInfo.xcprivacy`. The only privacy manifest is Capacitor's own, which declares nothing collected.
  - Xcode Organizer crash reports arrive with no code for TestFlight and App Store users who share analytics with developers. The first F1 step therefore adds nothing to the binary.
  - The census can't tell what Organizer already shows. That needs Eric to look in Xcode → Organizer → Crashes.

## 3. Stable word IDs

**No row in any language has a stored ID.** A bank row is a bare `&'static str`: `assets/words/{lang}/{tier}.txt` → `scripts/build-wordlists.py` → `src/word_data.rs`. Chinese is the exception: `pinyin|hanzi` strings hand-embedded in `src/words.rs`.

Two IDs are **derived**:

| Scheme | Definition | Used by |
|---|---|---|
| `word_id::word_id0(lang, word)` (`src/word_id.rs:24`) | `"{lang}::{lowercase}"`, plus an optional `#sense` that nothing uses | misses, word stats, tone drill (the de-facto identity) |
| `wordid::word_id(word)` | FNV-1a-64 of the NFC spelling | Spell Racing ghosts only |

**Stability:**
- A derived ID survives rows being added, removed or reordered, and survives re-ingest, because it depends only on the spelling.
- It changes when the word's own spelling is edited. For F2 that is arguably right: a corrected word is a new word.
- Pipeline languages have zero duplicates under the fold, because the pipeline dedups.
- Homographs (ru замок) are one row with no sense column.

| lang | rows | ID stored | derived ID unique | survives re-ingest |
|---|---|---|---|---|
| en | 3170 | 0% | 100% | yes |
| es | 6097 | 0% | 100% | yes |
| fr | 6109 | 0% | 100% | yes |
| de | 6144 | 0% | 100% | yes |
| pt | 6112 | 0% | 100% | yes |
| pl | 6175 | 0% | 100% | yes |
| vi | 4094 | 0% | 100% | yes |
| ko | 6234 | 0% | 100% | yes |
| ja | 6160 | 0% | 100% | yes |
| fil | 4083 | 0% | 100% | yes |
| ru | 6208 | 0% | 100% by spelling; homographs merged | yes |
| sw | 2845 | 0% | 100% | yes |
| ar | 6238 | 0% | 100% | yes |
| hi | 2674 | 0% | 100% | yes |
| **zh** | 6182 | 0% | **no under the store key**: stores key on pinyin only, and 626 rows share 254 pinyin keys (九 / 酒 → `zh::jiu3`). The full `pinyin\|hanzi` entry is 100% unique. | yes, by full entry |

`fa` has a bank, but it's unregistered and unreachable. There is no `ur` bank. The draft banks (`assets/words-draft/`) aren't compiled.

**Custom words are distinguishable at scoring.** My Words plays under `cur_lang == "__mine"`, so I4's `word_id: "custom"` is easy. Translate's "Spell it" scores under the real language, but its target is always a bank word.

## 4. Jr resolver surface

- **Exposed:** `experience::of_kid(kid: bool) -> Experience { Junior, Standard }` (`src/experience.rs:72`). This is what production calls, at 10 call sites.
  - `experience::resolve(kid, age_locked) -> Resolved` (`:60`) is called only from tests.
  - The input is `AppState.kid` (`model.rs:200`).
- **Before the gate is answered:**
  - `Prefs::default()` has `kid = false`.
  - `agegate::is_kid_locked()` is `stored().map(..).unwrap_or(false)` (`agegate.rs:35`).
  - So the resolver returns **`Standard`**. The UI is built as Standard, and the gate scrim is laid over it at the end of `start()` (`lib.rs:279`).
  - **There is no unknown state.** The only signal is `agegate::stored().is_none()` (`agegate.rs:31`).
- **Storage:**
  - The verdict is in localStorage `byear_agegate_v1` (`{"verdict":"kid"|"full","checkedAt"}`; no birth date).
  - The flag is in `byear_prefs_v1.kid`.
  - Both are read synchronously in `start()` before any network call.
- **Change:**
  - Under 13 is locked. Leaving needs the parent gate plus a new birth date.
  - 13 and over can flip Jr freely.
  - Clearing data re-asks.
- **Education edition:** the resolver has **no edition branch**. D7 (off by default in Education) would have to read `consts::EDITION` itself.
- **Parent-gated area for the F7 Jr toggle:**
  - The Settings sheet is **not** gated.
  - Candidates:
    - the guardian dashboard (`guardian_dash.rs:197`), which already holds the `gdDigest` toggle and is the closest pattern
    - wrapping the row in `parent_gate_then` (`lib.rs:976`)

## 5. Outcome classes

There is no single per-round enum:

| Mode | Per-round result |
|---|---|
| Base game (all languages) | `bool` + thread-local `VALID_OTHER_SENSE` (`game.rs:2687`) |
| zh | + `pinyin::WordVerdict { Graded(Vec<SyllableVerdict{Exact, ToneMiss, SegmentMiss, Both}>), LengthMismatch, Unparseable }` |
| ko | + `jamo::Grade { score, correct }` |
| Spelldoku | `play::Verdict { Correct, Misspelled, WrongValue, WrongSystem }` |
| Impostor | bool; `Trap` labels the card |
| DefMatch, Bee, Chains, Forge, Say It, Racing, Daily | bool |

- In the base game, **timeout** and **give-up** are separate code paths (`game.rs:2541`, `:2560`), but they collapse into the same miss. Only the glyph passed to `finalize_incorrect` tells them apart.
- `reports::MissClass { Substitution, Omission, Insertion, Transposition }` classifies a miss after the fact, and could back F2's `edit_ops.kind`.
- **`attempts`:** there's no counter. There's at most one retry per word (`RunAids.retry_used: bool`, `attempts.rs:40`).
- **`replay_taps`:** not tracked. `replayBtn` → `game::speak_current` (`game.rs:1088`) counts nothing.
- F2 needs both as new observe-only counters.

## 6. Remote flag mechanism

**There is none.**
- `flags::*()` is `resolve(localStorage["spell_flag_<name>"], compiled default)` (`flags.rs:20-31`).
- The only writers are the Tools & Features hub switch and e2e tests.
- No backend route serves flags, so a server-side change never reaches a running app.
- Changing a default needs a release.
- "Definitions-dark unlock" is a compiled per-language gate (`api::meaning_supported`, `consts::def_match`), not a flag.

Reading a cached flag synchronously at the top of `start()`, before the first network call (`game::preload_pool`, then `notify::flush` at `lib.rs:264`), is trivial. `notify.rs` is already a localStorage queue flushed at launch, which is the pattern for the telemetry queue plus kill switch.

---

## F4 repeat windows (census: "no window defined" list)

| Mode | Window today | Status |
|---|---|---|
| Standard, Climb, Misses review, Spell It | shuffled deck per (lang, tier), with no repeat until the pool is exhausted, across restarts; `RECENT_CAP = 5` held back at reshuffle (`deck.rs:10`) | defined |
| Daily | cycle walk with no repeat for L = pool / words-per-day days; yesterday guard at the cycle boundary (`daily.rs:178-211`). **The 90/180-day exclusion in its spec is explicitly not applied** (`daily.rs:221`). Only `date → score` is persisted. | defined (derived L) |
| Spell Picture | `NO_REPEAT = 5` pictures; `RECENCY_PICS = 3` for words | defined |
| Spelldoku | `REPEAT_WINDOW_DAYS = 365` (`spelldoku/play.rs:75`); served-board ledger not yet built (in progress in this tree) | defined, not enforced |
| Spell Cross / Search | CC-WORDGRID D9 (signed): 20 puzzles or 14 days, whichever is longer; 90 days for the Daily Puzzle. Not implemented. | spec only |
| Definition Match | within session only | session only |
| Bee, Chains | within one bee / chain only | **no window defined** across sessions |
| Impostor | **none, not even within a set of 10** (`impostor.rs:508-515`) | **no window defined** |
| Say It | `Math.random`, so back-to-back repeats are possible (`say_it.rs:264`) | **no window defined** |
| Letter Forge, Racing, Online Spell-Off, Practice, My Words (in order) | by design (daily puzzle / fixed track / fixed curriculum / user order) | **no window defined** |

**The spec's "previous 7 days' Daily words" isn't a window any owner defines.**
- Daily's real window is L days, and it varies per language and tier.
- A device could rebuild past Daily sets with `daily::build_words(lang, date, kid)`. But `pool_hash` in the seed means any bank update makes past days rebuild differently, and local dates differ across time zones.
- **Recommendation:** the watchdog should keep its own device-local ledger of served words rather than reconstructing past days.

**Audio "unavailable" (F5):** there's no enum. It's `LAST_SOURCE = "none"` (`api.rs:239`), read through `api::last_audio_source()`. The resolver's sources are `Pack | ServerCache | NativeTts`.

---

## Rulings needed (recommendations)

| # | Question | Recommendation |
|---|---|---|
| R1 | Supersede CC-LEARNING-ENGINE D5 / CC-BUY-DRIVERS S1 "zero telemetry"? | **Yes, by a one-line amendment in both** ("superseded for standard players by CC-TELEMETRY-FOUNDATION; Jr stays at F6 aggregate only"). Otherwise every future spec cites a posture that's no longer true. |
| R2 | Accept derived IDs as "stable" for F2? | **Yes.** Use `word_id0` (`lang::lowercase`) for every pipeline language, which is the key the stores already use. For **zh**, telemetry uses the full `pinyin\|hanzi` entry, not the pinyin store key. No bank change needed. |
| R3 | Jr tri-state | **CC-ONBOARD-JR amendment** (owner, not telemetry): add `experience::audience() -> {Junior, Standard, Unknown}`, where Unknown = `agegate::stored().is_none()`, and Education always → Junior-equivalent for telemetry per D7. Telemetry only reads it. |
| R4 | Kill switch without a flag system | Serve `GET /flags` from the **same Worker** (D5), cache it into `spell_flag_telemetry_enabled`, and read it synchronously at next launch. This is a new but minimal mechanism, so it needs the "no new flag system" line waived. |
| R5 | `outcome_class` with no engine enum | Add one `OutcomeClass { Correct, CorrectOtherSense, Miss, Timeout, GiveUp }` in the core, **set at the existing branch points** (not re-derived), with zh `ToneMiss` / `SegmentMiss` as a sub-field. It's observe-only, but it's new engine surface: sign it or name an owner. |
| R6 | F4 Daily window | Replace "previous 7 days" with "Daily's own cycle length L, from a device-local served ledger". Leave the no-window modes out of F4 v1. |
| R7 | Out of scope, but urgent | `privacy.html` is already false about audio, the typed answer and the install id. Fix it independently of telemetry, before the F7 draft. |
