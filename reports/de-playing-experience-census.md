# German playing experience: telemetry scope and bank exposure (2026-11-04)

Read-only. No telemetry data was pulled. Everything below comes from the code and the repo's own docs.

## 1. What the live telemetry collects (answer to STOP 1)

Source: `src/telemetry/schema.rs` (the single schema, I9), `docs/CC-TELEMETRY-FOUNDATION.md` (v1.1, invariants I1 to I9).

| Record | Fields | Notes |
|---|---|---|
| `error_event` | `error_code` (enum: js_uncaught, js_unhandled_rejection, wasm_panic, wasm_init_failed, pack_load_failed, native_crash, native_hang), `lang`, `mode`, `stack_hash` | `stack_hash` is FNV-1a-64 of a normalised stack (function names or Rust file:line), never message text |
| `perf_row` / `agg_perf_row` | `metric` (wasm_init_ms, tap_to_audio_ms, audio_resolution), `bucket` (lt100 to ge5000, resolved, unavailable), `lang`, `count` | histograms, never raw ms |
| `event_batch` / `agg_batch` | `v`, `build`, `platform`, `session_id` (random per launch, in memory), lists of the rows above | Jr or not-yet-answered players send the aggregate form without language |
| `clip_report` | `lang`, `item_hash`, `variant` (normal/slow), `source` (server_cache, native_tts, human, pack, other) | **Player-initiated** "this clip is bad" report. `item_hash` is a 64-bit hash of `lang|word`, so it identifies WHICH bank word was flagged (reversible against the bank). It carries no answer, attempt, outcome or score |
| `flags` | `telemetry_enabled` | the kill switch |

`FieldType` has no string variant, and the schema lint (`schema_lint`) rejects field names such as word, answer, typed, attempt, miss, hint, outcome, score, start, complete, funnel, mastery, difficulty, review, placement. The doc's I3 says: no word IDs, answers, attempts, misses, hints, outcomes, mode starts or completions, or funnels. Deferred and unbuilt: word-level difficulty stats (v1 F2), mode funnels (F3), Jr learning counters (F6), each needing a signed CC-LEARNING-ENGINE D5 amendment.

Server side: the Cloudflare Worker `spell-telemetry` never reads the IP, `request.cf` or the User-Agent, and has no request logging. `/api/speak` logs `lang, variant, cache, ms, status` with no IP and no word. Raw events are deleted after 90 days.

**Finding:** I found no wrong answers and no per-word learning data in the telemetry. The one per-word item is `clip_report.item_hash`, which is a user-initiated clip complaint, not a learning record. I10 asks nothing about it, but I3's "no word IDs" can be read against it; that is your ruling, not mine.

**Not verified:** I read the code and docs, not the deployed Worker or its D1 tables. I did not look at the online-play game server (CC-GAME-SERVER), which is outside this telemetry, and I have not checked what it receives.

**Consequence:** telemetry cannot tell us which German answers players got wrong, so the measurement route is the three you named: the bank exposure count below, the German Gig B play audit, and the TestFlight local share-sheet export.

## 2. German bank exposure (assets/words/de, 6,262 rows)

| Tier | Rows | With ä ö ü ß | With a homophone partner (seed) | With a listed together/apart form |
|---|---|---|---|---|
| Easy | 272 | 25 | 0 | 0 |
| Medium | 1,000 | 163 | 11 | 1 |
| Hard | 2,000 | 349 | 3 | 1 |
| Expert | 2,990 | 560 | 1 | 2 |
| **Total** | **6,262** | **1,097 (17.5%)** | **15** | **4** |

- By letter (a row can hold several): ä 434, ö 213, ü 384, ß 104 rows.
- Easy and Spell Jr accept the stand-ins (signed); the 1,072 Medium+ umlaut or ß rows are the ones where `Strasse` or `Maedchen` shows "close".
- Homophone partners in the bank today: booten, boten, entstand, fielen, leeren, lehren, liegen, reist, reißt, statt, vielen, wahren, waren, wider, wieder (15). das and dass are in the seed but neither is in the bank.
- Listed together/apart words in the bank: dahingehend, hierzulande, sodass, weitgehend. The Regelwerk's § 34 E7 also lists kennenlernen, liegenbleiben, sitzenbleiben, stehenlassen; none is a bank row today, but they grade correctly if added.
