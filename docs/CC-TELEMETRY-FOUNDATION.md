# CC-TELEMETRY-FOUNDATION v1.1

**Status:** Phases A, B and C BUILT on branch `cc-telemetry` (2026-09-18). Not deployed: the Worker needs Eric's first deploy (`workers/telemetry/README.md`) and the R8 server change needs `deploy.sh`. Nothing ships to players before the F7 disclosures are signed.
**Supersedes v1.** v1.1 covers **crash reporting (F1) and performance (F5) only**, per Eric's D-TEL sign-off (2026-09-18, "add with the recommendations").
**Blast radius:** one new network endpoint plus error and performance hooks. Zero gameplay behavior change. Must not delay the next TestFlight build (R10b: the v1.1 draft said "build 56"; TestFlight was at build 226).

> **§0 census:** `docs/census/telemetry_census.md` (v1 run, then the v1.1 update).
> **Rulings signed 2026-09-18 (Eric):** R1–R7 ("signed as recommended"), then R8–R11 ("signed as recommended, start Phase A"). Under v1.1: R2, R5, R6 don't apply; R1 is replaced by R9.

---

## Why v1.1 exists

v1 conflicted with signed decisions:
- **CC-LEARNING-ENGINE D5:** all learning data stays on the device; zero telemetry; no engagement optimization.
- **CC-LEARNING-ENGINE / CC-LEARNING-MODES:** analytics on the device only. (CC-LEARNING-MODES is not in `docs/`, R10c.)
- **CC-PRACTICE** (`CC-PRACTICE-PLAN.md`, `-V2-PLAN.md`): no hint or miss telemetry.
- v1's F4 duplicated the global Freshness Invariant (CC-PERSIAN-FOUNDATION F6). That invariant is spec-only; no exposure ledger exists in `src/` yet.
- v1's audio-clarity signal was meant to avoid overlapping CC-AUDIO-REPLAY. The "F6" cite was wrong (R10c): that file is the replay-banner fix and has no clarity flags.

Crashes and performance are not learning data, so they fit D5 as annotated by **R9**: "zero telemetry" means zero learning or gameplay data. The annotation is recorded in `CC-LEARNING-ENGINE.md` D5 and `CC-BUY-DRIVERS.md` S1. Everything else from v1 is under **Deferred** below. Any of it can only come back through an explicit, signed amendment of CC-LEARNING-ENGINE D5.

## Intent

SpellGame keeps finding broken builds through players and paid auditors instead of through the app. An auditor hitting a crash wastes a paid pass. The backend (the **Mac mini behind the Cloudflare Tunnel**, R10a; the v1.1 draft said "Pi") is a single point of failure nobody is watching. v1.1 lets Eric see "is it broken" and "is it slow", and nothing about how anyone plays.

---

## Ownership boundaries

| Concern | Owner | This file's role |
|---|---|---|
| Jr / age resolution | CC-ONBOARD-JR (`experience.rs`) | Reads `experience::audience()` (R3, added under CC-ONBOARD-JR). Never computes age. |
| Audio resolver | CC-BUILD219-FIXES (`api.rs` `LAST_SOURCE`) | Observes resolution outcome counts only (F5). |
| Repeats / freshness | CC-PERSIAN-FOUNDATION F6 | **Not touched.** |
| Learning data (misses, attempts, funnels) | CC-LEARNING-ENGINE (on the device, D5) | **Never read or sent.** |
| Remote flags | None existed (census §6) | **R4:** the Worker serves `GET /telemetry/v1/flags`; the client caches it in `spell_flag_telemetry_enabled`. |

If any boundary no longer holds: **stop and ask.**

---

## Features

### F1. Crash and error reporting
**Intent:** find broken languages and modes before auditors or App Store reviewers do.
**Census finding:** Apple's channels hold nothing (0 TestFlight crash reports; no Organizer diagnostics for builds 215–226). They could never see JS or WASM errors anyway, because those don't crash the app process.
**Behavior:**
- Custom reports:
  - uncaught JS exceptions and unhandled rejections
  - WASM panics (the Rust panic hook, keyed by source location)
  - WASM boot failure after all retries
  - offline language-pack load failures (keyed by stage)
- MetricKit (Phase B): a subscriber registered at launch keeps each crash or hang diagnostic as a kind plus a 64-bit signature (exception type, signal, the attributed thread's top 8 frames as binary + text-segment offset). The Rust core drains them at launch and maps them to `native_crash` / `native_hang`. The Swift lives in `NativeLanguageKitPlugin.swift` so that `project.pbxproj` is untouched.
- Payload: `build`, `platform`, `lang`, `mode`, `error_code` (enum), `stack_hash`. The hash is FNV-1a-64 of the normalized stack: JS function names or the Rust file:line:col, never message text. Standard players add `session_id`: random per launch, held only in memory.

### F5. Performance and backend health
**Intent:** know the backend is slow before players feel it.
**Behavior:**
- **Server side first (R8):** existing logs record none of this. Add one structured line per `/api/speak` request: `lang, variant, cache=hit|miss, ms, status`, with no IP and no word. A daily script reports TTS p50/p95, cache hit rate and error rate per language.
- **Server line (built):** `backend/speak_metrics.py` writes one line per request into daily files. `scripts/speak_report.py` prints p50/p95, disk-cache hit rate and error rate per language per day, and `--prune` deletes files older than 90 days. Requests Cloudflare's edge answers from cache never reach the server.
- **Client (built):** histograms, never raw ms: `wasm_init_ms`, `tap_to_audio_ms` and `audio_resolution{resolved|unavailable}`, per language for standard players, without language in aggregates. `tap_to_audio` is timed only for the pack and server clip, which report when playback starts. The on-device voice reports when it has finished speaking, so it counts as resolved but untimed.

### F6. Spell Jr handling
**Intent:** collect nothing about any child.
**Behavior:** when `audience()` is Junior or Unknown, and always in the Education edition, only aggregate counts are kept: crashes by `error_code`, and performance buckets. They're sent at most once per day, at a random time 24–48 h after the previous send, with no identifier of any kind.

### F7. Consent surface and disclosures
- **Toggle (built):** Settings → "Help improve SpellGame" (`telemetryToggle`, stored in `spell_telemetry_opt_v1`).
  - Defaults: on for consumers (D3), off in Education (D7).
  - Off stops all sending and clears everything held locally.
  - For a Jr or not-yet-answered player the switch snaps back, and only the parent gate (`parent_gate_then`) can change it. That works on web and in the app; the guardian dashboard is app-only.
  - It renders disabled when the server's kill switch is off.
  - It's declared in `config/settings-effects.json`. Its label is translated into all 15 locales; those translations need native review like other UI copy.
- **`docs/telemetry/privacy_label_answers.md` (written):** Diagnostics → Crash Data + Performance Data, *Not Linked to You*, *Not Used for Tracking*, App Functionality. Eric enters these manually. It also lists the non-telemetry flows the label should be checked against.
- **`docs/telemetry/privacy_policy_draft.md` (written, REVIEW-GATED, don't publish):** the new section plus a sentence for the short version. It depends on R7 landing first and on the Cloudflare IP check.

---

## Transport and storage

- The event schema is defined once in the Rust core (`src/telemetry/schema.rs`). The JS constants (`telemetry-schema.js`) and the Worker's validator input (`workers/telemetry/schema.json`) are generated from it, and a test fails if they're stale.
- The queue is capped at 200 events or 128 KB, with the oldest dropped. It flushes when the page goes hidden, every 10 minutes, and at launch. Bodies are gzip JSON POSTs where `CompressionStream` exists, plain JSON otherwise.
- The endpoint is the Cloudflare Worker `spell-telemetry` on `spellgame.net/telemetry/*` (D5), in front of the tunnel. It never reads the IP, `request.cf` or the User-Agent. Observability is off, and there's no request logging.
- Raw events are deleted after 90 days by a daily cron. The per-day counts in `daily_counts` are kept.

---

## Invariants

- **I1 — Never in the way.** Fully asynchronous. A failed, slow or disabled endpoint changes nothing the player sees.
- **I2 — No free text.** Every field is an enum, number, bucket, boolean or hash. `FieldType` can't express a string.
- **I3 — No learning data.** No word IDs, answers, attempts, misses, hints, outcomes, mode starts or completions, or funnels.
- **I4 — Nothing personal.** No audio, photos, OCR output, My Words, account email or account ID.
- **I5 — No account join.** The telemetry store has no field that can join to accounts.
- **I6 — Unknown means Jr.**
- **I7 — Kill switch.** `telemetry_enabled=false` stops all sending and clears the queue on the next launch, and immediately when the answer arrives mid-session. Until the server has answered once, errors are held on the device and nothing is sent.
- **I8 — Observe, never act.** Nothing in this file changes runtime behavior.
- **I9 — Single source.** One schema, in the Rust core.

---

## Decisions

| # | Decision | Status |
|---|---|---|
| D-TEL | Scope to F1 + F5 only; learning data stays on the device per LEARNING-ENGINE D5 | Signed |
| D1 | First-party endpoint; self-hosted Sentry/TelemetryDeck acceptable for F1 only | Signed |
| D2 | Jr: aggregate crash/perf counts only, daily, no identifier | Signed |
| D3 | Standard players: on by default with toggle | Signed |
| D4 | No `install_id`; per-launch `session_id` only | Signed |
| D5 | Endpoint host: **Cloudflare Worker** | **Signed** (Eric, 2026-09-18; R11) |
| D6 | Raw 90 days, aggregates indefinitely | Signed |
| D7 | Education Edition: off by default, school-controlled; when on, aggregate only | Signed |
| D10 | spellgame.net included (F1 + F5) | Signed |
| D11 | Toggle name "Help improve SpellGame" | Signed |
| R3 | `experience::audience()` → Junior / Standard / Unknown | Signed, built |
| R4 | Kill switch served by the Worker, cached in `spell_flag_telemetry_enabled` | Signed, built |
| R7 | Fix `privacy.html` (audio, typed answer, install id) independently | Signed; separate task |
| R8 | One structured `/api/speak` log line for F5 | Signed; Phase B |
| R9 | Annotate D5 / S1 instead of superseding them | Signed, recorded |
| R10 | Correct the Pi, "build 56" and dead-cite facts | Signed, applied above |
| R11 | Record D5 as signed | Signed, recorded |

## Phases

- **Phase A (built, `07cbd205`):**
  1. R3 `audience()`
  2. schema + generated bindings
  3. transport, routing and kill switch
  4. F1 (JS, WASM panic, boot failure, pack load)
  5. Worker + D1 schema + retention cron
  6. R9 annotations
- **Phase B (built):**
  1. R8 server log line and report (`8f0e8e65`)
  2. F5 client buckets (`08cec32e`)
  3. MetricKit bridge (`82ff60f2`)
  4. Acceptance 7 and 5-timing; telemetry randomness moved to `crypto` (I8)
- **Phase C (built):**
  1. F7 toggle row
  2. `privacy_label_answers.md`
  3. privacy-policy draft
  4. acceptance 4
  5. the usefulness report: `scripts/telemetry_report.sh` + `workers/telemetry/reports/*.sql`, checked against a SQLite built from the migrations

## Deferred (not in scope; each needs a signed CC-LEARNING-ENGINE D5 amendment first)

- **v1 F2** word-level difficulty and integrity stats.
- **v1 F3** mode funnels. The no-telemetry alternative is App Store Connect's own opt-in usage metrics.
- **v1 F6** Jr aggregate *learning* counters.
- **v1 F4** repeat watchdog: **dropped**, not deferred. If a violation signal is wanted, it becomes a debug assertion inside the PERSIAN F6 exposure ledger once that ledger exists.

---

## Acceptance tests (done = all pass)

| # | Test | Where | Phase A |
|---|---|---|---|
| 1 | Schema lint: no unbounded string; no learning-data field name | `telemetry::schema::tests::schema_lint` | ✅ |
| 2 | Jr capture | e2e `telemetry` `jr_sends_only_one_daily_aggregate` | ✅ |
| 3 | Unknown-age capture | e2e `telemetry` `unknown_age_sends_only_one_daily_aggregate` | ✅ |
| 4 | Personal-data capture (Say-It, Snap a List, My Words, login) | e2e `personal_data_never_reaches_a_payload` | ✅ for My Words, sign-in (email + password typed) and the Snap a List review sheet. Say It needs the iOS speech bridge and can't run in the browser; the schema has no field that could carry audio (`schema_lint`). |
| 5 | Endpoint down: invisible, queue kept and capped, no added latency | e2e `endpoint_down_is_invisible_and_keeps_the_queue`, `endpoint_down_costs_no_round_latency` + `queue_caps_drop_oldest` | ✅ Tolerance is max(5%, one 60 Hz frame); medians are about 50 ms, where 5% can't be measured. It measures verdict-paint latency, not frame time. |
| 6 | Kill switch | e2e `kill_switch_sends_nothing_and_clears_the_queue` | ✅ (the flags GET itself continues; it carries no data) |
| 7 | Observe-never-act | e2e `observe_never_act_replay_is_identical_on_and_off` | ✅ Mutation-checked: telemetry stealing one `Math.random` changes the served words, and the test fails |
| 8 | Single source | `telemetry::schema::tests::single_source`, `bindings_are_current` | ✅ |
| 9 | Usefulness | `scripts/telemetry_report.sh` | Report built; the pass/fail needs two weeks of TestFlight data after deploy |

## Non-goals

- Any learning, gameplay or engagement data (see Deferred).
- Third-party analytics SDKs, ad attribution, App Tracking Transparency, IDFA.
- Session replay, keystroke logging, A/B testing.
- Dashboards beyond the census and the usefulness report.

*Not legal advice. Verify COPPA (amended rule, April 2026 compliance) and Apple Kids Category guidance before release.*
