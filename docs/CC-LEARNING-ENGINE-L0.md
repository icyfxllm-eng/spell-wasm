# CC-LEARNING-ENGINE-L0 v1

[stashed verbatim from Eric 2026-09-18. §0 census: docs/census/learning_l0_census.md]

**Status:** REVIEW-GATED. Phase 0 census DONE (`docs/census/learning_l0_census.md`). **Census rulings C1–C9 SIGNED as drafted (Eric, 2026-09-19)**; where this file and a ruling differ, the ruling governs. L0 is a retrofit over the shipped engine (C1). **Phase 1 UNBLOCKED (Eric, 2026-09-19): "Phase 1 may start; D3–D8 gate only the phases they name."** D3/D4 gate Phase 2, D5/D6 gate Phase 3, D8 gates CC-REPORTS' surface; D7 (deadline) gates no code.
**Relationship to CC-LEARNING-ENGINE:** this file **supersedes its scope**, not its decisions. The signed decisions there (D1 BKT+FSRS chosen over IRT/DKT, D5 all on-device zero telemetry, D7 learner state may touch word choice and feedback text only) carry forward unchanged. Everything in the original file that is not listed under **In scope** below moves to a named successor file (see Successors) and is explicitly **out of L0's scope**.
**Blast radius:** one new read-only interface + one scheduler + a dev-only inspector. No gameplay, scoring, layout, or bank changes.

---

## Why this file exists (intent)

CC-LEARNING-ENGINE has been sidestepped by every Claude Code session. The cause is structural, not behavioral:

1. **It is six subsystems in one file** (skill model, scheduler, error diagnosis, placement, insight surfaces, voice QA harness). No session can finish it, so every session finds something reachable instead.
2. **It has no gradable artifact.** Every SpellGame thread that succeeded had a render Eric could grade FAIL. Nobody can eyeball a probability estimate.
3. **D5 (zero telemetry) removes the validation path.** With no corpus, "correct" is undefined, so an agent cannot tell finished from broken.
4. **Four shipped-scope files already read the learner model** (CC-REPORTS, CC-CALENDAR, CC-GUARDIAN-DASH, CC-PLACEMENT-IDEMPOTENCE) and each assumed a shape for it. Every week of stubs raises the cost of the real thing.
5. **It has no deadline**, so it loses every scheduling contest against build 56, the audit round, and TestFlight bugs — correctly, every time.

L0 fixes the structure. It delivers one thing a player feels (better review timing) and one thing that stops the bleeding (a frozen read contract). The research half stays honest in its own file rather than blocking a scheduler that is ready to ship.

**Honest scope note:** FSRS is a published algorithm with reference test vectors — its correctness is checkable. The skill model is not, and L0 does not claim it is. That is why BKT is deferred, not cut.

---

## In scope (all of L0, nothing else)

- **R1** Frozen read contract (`LearnerQuery`) + deterministic fake
- **R2** FSRS scheduling for the missed-words queue
- **R3** Learner simulator as the validation path
- **R4** Dev-only inspector screen

## Successors (named, out of L0 scope, do not start)

| File | Holds |
|---|---|
| CC-LEARNING-SKILL | BKT skill model, skill decomposition, mastery estimates |
| CC-LEARNING-DIAGNOSIS | grapheme-alignment error diagnosis, optional tie-break classifier |
| CC-LEARNING-INSIGHT | player + guardian insight surfaces beyond what CC-REPORTS already owns |
| CC-LEARNING-SPOKEN | L3 letter-name-grammar spoken spelling |
| CC-VOICE-QA | Voice Studio minimal-pair QA harness gating custom-voice flags |

Placement stays owned by **CC-PLACEMENT-IDEMPOTENCE** (already written). L0 does not touch it beyond exposing its result through `LearnerQuery`.

---

## Ownership boundaries

| Concern | Owner | L0's role |
|---|---|---|
| Placement probe, PlacementRecord | CC-PLACEMENT-IDEMPOTENCE | **Amended by C7:** L0 stores `placed` (it lives inside the learner state); CC-PLACEMENT-IDEMPOTENCE owns what it means. Only placement code writes it (I6 scan). L0 never re-probes. |
| Reports surfaces (kid tab + Guardian Dash) | CC-REPORTS | Becomes a consumer of `LearnerQuery`. L0 builds no UI for it. |
| Planned days, due set display | CC-CALENDAR | Consumer. L0 supplies the due queue, not the calendar. |
| Word selection legality (bands, tiers) | existing selection code | L0 supplies *ordering within legal candidates* only (D7). |
| Freshness / no-repeat windows | CC-PERSIAN-FOUNDATION F6 exposure ledger | **Amended by C5:** F6 is unbuilt, so this is a recorded dependency, not an L0 requirement. Until F6 lands, freshness stays with the existing deck and selection exclusion. L0 adds no freshness mechanism of its own. |
| Entry identity | CC-RUSSIAN-STRESS v3 | Scheduler keys on it (I4). |
| Language scoping of stores | CC-LANG-SCOPE F5 | The scheduler store registers as `languageScoped`. |
| Telemetry | CC-TELEMETRY-FOUNDATION v1.1 | **Nothing here is ever transmitted** (D5, I2). |

If any boundary no longer holds: **stop and ask.**

---

## §0 Blocking census (executable now; report-only; then stop)

Produce `learning_l0_census.md`:

1. **Are CC-LEARNING-ENGINE D2 (owner list) and D6 (placement length) signed?** If not, report it — L0's Phase 1 does not need them, but the successor files do.
2. **What schedules missed-word review today?** Dump the current rule, its store, and its call sites. Name every place that decides *when a missed word comes back*.
3. **Stub inventory:** for each of CC-REPORTS, CC-CALENDAR, CC-GUARDIAN-DASH, CC-PLACEMENT-IDEMPOTENCE — what shape of learner data does it currently assume, and how does it access it? This is the input to R1's contract design.
4. **Existing learner state on device:** what is persisted today, under what key, and is it language-scoped per CC-LANG-SCOPE F5?
5. **Exposure ledger surface:** the API the PERSIAN F6 ledger exposes, so the scheduler consumes rather than duplicates it.

**Stop-and-ask triggers:** more than one existing review-scheduling rule (they must merge, not coexist); any consumer reaching into learner storage directly rather than through an interface; learner state not language-scoped.

---

## Features

### R1. Frozen read contract — `LearnerQuery`
**Intent:** stop four shipped-scope files from hardening around four different imagined learner models. This is worth doing even if the engine is never built.
**Behavior:**
- One read-only interface in the Rust core, exposing exactly: `dueQueue(lang, limit)`, `nextWordReason(wordId)` (an enum, not prose), `atRiskSet(lang, horizonDays)`, `reviewStats(lang)`.
- A deterministic **fake** implementation returning fixed fixture data. Consumers build and test against the fake.
- All four consumers migrate to `LearnerQuery`. A symbol scan fails the build on any direct access to learner storage outside the implementation module.
- The interface is additive-only after freeze: adding a method is allowed, changing or removing one is a stop-and-ask.
**Why first:** its cost rises every week. Ships before R2.

### R2. FSRS scheduling for missed words
**Intent:** the one thing in L0 a player feels. Review timing stops being ad-hoc.
**Behavior:**
- Implement FSRS in the Rust core, keyed on entry identity × language × profile. Pin the FSRS version and its default parameters; record both.
- The existing missed-words queue is scheduled by it. The old rule is **deleted**, not left alongside (census item 2 must show a single rule afterward).
- Grade mapping from SpellGame outcomes to FSRS ratings is declared in one table (see D3).
- Scheduler output is **ordering within already-legal candidates**. Band legality, tier, freshness, and language scope all still win (D7, I3).
- Parameter optimization from player history is **out of scope** — defaults only in L0 (D4).
**Why second:** it's the only part with an external correctness standard.

### R3. Learner simulator (validation path)
**Intent:** D5 leaves no corpus, so correctness must be established against synthetic players with known ground truth. This is the same trust pattern as the tracer: a number earns authority by reproducing known verdicts.
**Behavior:**
- Generate synthetic players with known skill and known forgetting rates; run simulated session histories; assert the scheduler's intervals track the known forgetting curve within a stated tolerance.
- Adversarial populations: always-correct, always-wrong, random, and long-absence. Assert no divergence, no unbounded intervals, no starved queue.
- Output `reports/simulator.md` with tolerances and results.
**Stated limitation (must appear in the report):** the simulator proves the implementation behaves correctly for players who behave as the model assumes. It does **not** establish that the model describes real children. Any grant claim must be worded to that limit (D6).

### R4. Dev-only inspector
**Intent:** give Eric a gradable artifact. Every file that succeeded here had one.
**Behavior:** a screen behind the existing DEV_PREVIEW flag showing, per language: current due queue, each item's next-review date and interval, and `nextWordReason` for the word just served. Compiled out of release, auditor, and education builds (same treatment as DEV_PREVIEW; CI symbol scan).

---

## Invariants

- **I1 — On-device only.** No learner data leaves the device, ever (CC-LEARNING-ENGINE D5). Not to telemetry, not to the Pi, not to reports.
- **I2 — Not telemetry-visible.** CC-TELEMETRY-FOUNDATION v1.1's schema lint must continue to fail on any learner field.
- **I3 — Ordering only.** The scheduler may reorder legal candidates. It may never make an illegal word legal, alter difficulty bands, change scoring, or affect layout (D7).
- **I4 — Identity-keyed.** Scheduling state keys on (profileId, languageCode, entry identity). Never on bare spelling.
- **I5 — One review rule.** After R2, exactly one code path decides when a missed word returns. A second is a build failure.
- **I6 — Contract-only access.** Consumers reach learner data only through `LearnerQuery` (symbol-scan enforced).
- **I7 — Deterministic.** Same profile state + same day = same queue. No randomness in scheduling.
- **I8 — Degrades safely.** With no learner state (new profile, cleared data), the queue falls back to the existing default order and nothing errors.

---

## Decisions

| # | Decision | Status |
|---|---|---|
| D1 | Split CC-LEARNING-ENGINE into L0 + five named successors | **Signed** (Eric, 2026-09-18, option 1) |
| D2 | Defer BKT; ship FSRS alone in L0 | **WITHDRAWN by C1 (2026-09-19).** BKT already ships and feeds Reports, Guardian Dash and Trap Boss; it stays as shipped, and CC-LEARNING-SKILL checks that shipped BKT is correct rather than building it. |
| D3 | Outcome → FSRS rating mapping. Rec: correct first try = Good; correct after replay or extra attempt = Hard; miss = Again; no Easy rating in v1 (avoids over-long intervals for kids) | **OPEN** |
| D4 | FSRS parameter optimization from player history | **OPEN** — rec: out of scope for L0, defaults only |
| D5 | Simulator tolerances (how close is close enough) | **OPEN** — rec: report-only in Phase 3, frozen after Eric reads the first report (v7.5 trust pattern) |
| D6 | SBIR/STARTALK wording of the simulator claim | **OPEN** — must state the synthetic-player limitation |
| D7 | Deadline: bind L0 to an SBIR FY2027 or STARTALK milestone | **OPEN** — without a date it loses every scheduling contest |
| D8 | Does the kid-facing Reports tab surface next-review dates? | **OPEN** — rec: no; it risks becoming a time-pressure mechanic, which CC-REPORTS' deficit-framing ban exists to prevent |

---

## Phases

- **Phase 0:** §0 census → stop-and-report.
- **Phase 1:** R1 contract + fake + consumer migration + symbol scan. **Ships alone**; no engine required.
- **Phase 2:** R2 FSRS behind the contract; old rule deleted.
- **Phase 3:** R3 simulator; first report to Eric; tolerances frozen after he reads it.
- **Phase 4:** R4 inspector; Eric's device pass closes L0.

---

## Acceptance tests (done = all pass)

1. **Contract freeze:** every consumer imports only `LearnerQuery`; a symbol scan finds zero direct learner-storage access outside the implementation module (I6).
2. **Fake parity:** all four consumers' test suites pass against the deterministic fake with no reference to the real implementation.
3. **FSRS reference vectors:** the implementation reproduces the pinned FSRS version's published test vectors exactly. *(C3: the pinned version is FSRS-4.5; the vectors are reference outputs from a published FSRS-4.5 implementation, vendored as a fixture.)*
4. **One rule:** a scan confirms exactly one review-scheduling code path exists; a deliberately reintroduced second path fails the build (I5).
5. **Ordering only:** a fixed-seed replay shows the scheduler never serves a word that band/tier/freshness/language-scope rules excluded (I3).
6. **Identity keying:** за́мок and замо́к schedule independently; рука and ру́ку schedule independently (I4).
7. ~~**Language scope:** the scheduler store is registered `languageScoped` and passes CC-LANG-SCOPE F5's classification CI.~~ *Dropped by C5: CC-LANG-SCOPE F5 is unbuilt. It's a recorded dependency, and the store is keyed per language by construction.*
8. **Telemetry isolation:** the telemetry schema lint fails on a deliberately added learner field (I2). *(C6: Phase 1 adds the learner terms to `schema_lint`'s forbidden list; the planted field is `mastery`.)*
9. **Determinism:** same profile state + same simulated date produces a byte-identical queue (I7).
10. **Cold start:** a new profile with zero learner state produces a valid queue with no error (I8).
11. **Simulator:** `reports/simulator.md` exists, covers all four adversarial populations, and states the synthetic-player limitation verbatim.
12. **Inspector excluded:** a symbol scan confirms the inspector is absent from release, auditor, and education builds.
13. **Device pass:** Eric plays a session in DEV_PREVIEW, sees why each word was served, and signs off that review timing is an improvement.

---

## Non-goals

- No BKT, skill estimates, error diagnosis, insight surfaces, spoken spelling, or voice QA (those are the successor files).
- No changes to scoring, difficulty bands, tier assignment, bank contents, or layout.
- No telemetry of any kind; no weakening of D5 to obtain a corpus.
- No new UI beyond the dev inspector; CC-REPORTS and CC-CALENDAR keep their own surfaces.
- No placement changes; CC-PLACEMENT-IDEMPOTENCE keeps that.
- No parameter optimization from player history in L0.
- No engagement optimization, streak pressure, or notification scheduling from learner state.
