# CC-LEARNING-ENGINE-L0 — Change Request CR-1: attempt instant, outcome class, mode tag, seen set

**Status:** DRAFT for Eric. Nothing is built.
**Raised by:** CC-GUARDIAN-REPORT v1 §0 C2 (HALT), 2026-10-26. That file says field gaps go to L0, never to it.
**Amends:** L0 R1 (`LearnerQuery`, additive only) and the stored `Attempt` record in `src/learner.rs`.
**Carries forward unchanged:** CC-LEARNING-ENGINE D5 (zero telemetry), D7, and L0 I1–I8.
**Census base:** origin/main `f61ca447`.

---

## Why

GUARDIAN-REPORT needs five things per attempt, plus a seen-word set per profile. Today `LearnerQuery::attempt_log` returns `AttemptView { word, day, correct }` (`src/learner_query.rs:96`). Underneath, `Attempt` is `{ day, word, skills, correct, channel, typed }` (`src/learner.rs:77`).

| Need | Today | Gap |
|---|---|---|
| UTC instant | `day` = `floor(Date.now() / 86_400_000)`, a UTC day index (`learner.rs:486`) | **Missing.** Local-midnight weeks can't be computed: in Indianapolis, Sun 20:00 local is already Monday UTC. |
| Word id | `word` is the display text | **Derivable.** `word_id::word_id0(lang, word)` (`src/word_id.rs:34`) is the I4 key, so no stored field is needed. |
| Language | Implicit: one store per language (`spell_learner_<lang>`) | **OK.** |
| Outcome class | `correct: bool` | **Missing.** `game::Outcome` (`src/game.rs:2590`) knows Correct / OtherSense / Misspelled / TimedOut / GaveUp / Voided, then collapses them to a bool. OtherSense is logged as `true`. |
| Mode tag | `channel`: Typed / Speech / Trap | **Missing.** Impossible goes through the same `game.rs:2247` submit as Classic. The game knows (`s.impossible_word`, `impossible::engine::is_level(&s.level)`, `game.rs:918,1446`), but the log doesn't. |
| Seen set | None. The log is a 512-entry ring per language (`LOG_CAP`, `learner.rs:41`). | **Missing.** |

Already correct, and no change needed:
- Voided "Can't hear it" rounds never reach the log. `void_round.rs:31` has a test enforcing that.
- Spell It doesn't log.
- Speech attempts carry `Channel::Speech`.

---

## Proposal

### P1 — Instant

- Add `Attempt.ts: Option<u64>` with `#[serde(default)]`, holding **whole UTC seconds**.
- `day` stays. FSRS and every existing reader keep using it.
- Invariant: `ts / 86_400 == day` whenever `ts` is present.
- Why seconds rather than milliseconds: week bucketing needs no more precision, and a coarser timeline is the smaller privacy footprint for a child's activity record. Flooring keeps A1's `23:59:59` attempt in its week.

### P2 — Outcome class

Add `Attempt.outcome: Option<OutcomeClass>` with `#[serde(default)]`. It is a closed enum written at the one builder, `learner::attempt()` (`learner.rs:634`):

```
Correct | OtherSense | Misspelled | TimedOut | GaveUp | Trap
```

- `correct` stays, because BKT and FSRS read it. Invariant, enforced by a unit test over every variant: `outcome.is_correct() == correct`, where Correct and OtherSense are correct and everything else is a miss.
- `Voided` is deliberately absent. Voided rounds record nothing today, and that stays true.
- **There is no `close` class in the game.** GUARDIAN-REPORT G-D16 excludes "close" outcomes. The nearest thing is Kid Mode's lenient fold (`norm::answer_matches_in`, `norm.rs:119`), which returns plain `Correct`. See L-D2.
- `TRAP_MISS(rule_id)` is **out of scope.** The rule ids belong to CC-RU-ORTHO F6 and CC-AR-HI-PREAUDIT F6, and neither has built them. Adding `rule` later is one more additive optional field.

### P3 — Mode tag

Add two fields, both `#[serde(default)]`:
- `Attempt.mode: Option<String>`: the `config/modes.json` registry id of the surface that logged the attempt. A lint fails on any id that isn't in the registry, which keeps it closed.
- `Attempt.impossible: bool`: true when the served word came from Impossible.

Impossible is a *level* inside Classic, not a mode, which is why it gets its own flag rather than a mode id. `skip_serializing_if` false keeps the stored size flat.

Write sites. All of them go through `learner::attempt()` or `note_attempt*`:

| Site | Mode |
|---|---|
| `game.rs:2247` | classic (+ `impossible`) |
| `game.rs:2328` | en path |
| `wordpic_screen.rs:2437, 2516` | |
| `wordsearch_ui.rs:467, 512` | |
| `wordcross_ui.rs:493, 535` | |

- Placement (`note_placement`) records as `mode: "placement"`.
- The callers' signatures change to pass the outcome and mode explicitly. No new thread-locals: `VALID_OTHER_SENSE` is already one too many.

### P4 — Seen set

- A separate key per language, `spell_learner_seen_<lang>`, holding a sorted set of `word_id0` strings.
- **Written on every logged attempt**, in any channel, including placement.
- It is a separate key, and not a field on `LearnerState`, because an older build that rewrites `LearnerState` would silently drop an unknown field. Older builds never touch a key they don't know.
- The `spell_learner_` prefix means every audit that polices learner storage covers it. The stray `BACKUP_SUFFIX` key is the precedent (`learner.rs:491`).
- **Backfill on first load** is the union of:
  - the current 512-entry log;
  - misses-queue entries (`MissEntry`, `model.rs:125`);
  - redemptions (`reports::REDEMPTION_KEY`).

  Words played before CR-1 and outside those three stores are lost, so they read as unseen. See L-D4.
- **Size:** bounded by the bank, with ru the largest at 6,830 rows. Worst case is about 140 KB of JSON per language, for a player who has seen the whole bank.
- `learner::reset(lang)` deletes this key as well (CC-LEARNING-ENGINE Done #8: nothing learned survives a reset).

### P5 — `LearnerQuery` additions (additive only, as the R1 freeze allows)

Existing methods and `AttemptView` are **unchanged**. Two methods are new:

```rust
/// One logged attempt with the CR-1 fields. Fields are None on entries
/// written before CR-1 or rewritten by an older build.
pub struct AttemptDetail {
    pub word: String, pub word_id: String, pub day: u32,
    pub ts: Option<u64>, pub outcome: Option<OutcomeClass>,
    pub channel: Channel, pub mode: Option<String>, pub impossible: bool,
}
fn attempt_detail_log(&self, p: ProfileId, lang: &str) -> Vec<AttemptDetail>;
fn seen_set(&self, p: ProfileId, lang: &str) -> BTreeSet<String>; // word_id0
```

- `Fake` gains fixtures for both, including entries with every field `None`.
- `ProfileId` stays `DEVICE`. Multi-profile is a separate, larger gap: GUARDIAN-REPORT C7 found no profile system.

---

## Schema and ship order

**There is no `SCHEMA_VERSION` bump. It stays at 2.** Every new `Attempt` field is optional with a serde default, and serde ignores unknown fields, since `learner.rs` uses no `deny_unknown_fields`. The table below compares a bump against no bump:

| Scenario | With a v3 bump | Additive at v2 (proposed) |
|---|---|---|
| A CR-1 build reads old state | migrate | reads; new fields are `None` |
| An older build reads CR-1 state | `Loaded::Newer`: fresh state, never written back (`learner.rs:549`). **Learner looks empty after a TestFlight downgrade.** | reads normally |
| An older build *writes* CR-1 state | n/a | drops `ts`, `outcome` and `mode` from that language's log. The seen set survives because it's a separate key. |

So there is no ship-order hazard. The cost is that a downgrade followed by an upgrade strips the new fields from old entries. The rule GUARDIAN-REPORT needs, which belongs in its F2: **an attempt with `ts: None` or `outcome: None` is never placed in a week window, and never enters W.** It shows less rather than guessing.

---

## Invariants (additions)

- **CR-I1.** `outcome.is_correct() == correct` on every attempt that has an outcome.
- **CR-I2.** `ts / 86_400 == day` on every attempt that has a `ts`.
- **CR-I3.** No new field reaches telemetry. The schema lint's planted-field test (`telemetry/schema.rs:558`) gains `attempt_ts`, `attempt_outcome`, `seen_words` and `learner_seen`.
- **CR-I4.** The seen set is written only from `learner.rs`'s single save path. The existing symbol scan (I6) covers the new key.
- **CR-I5.** `learner::reset(lang)` leaves no `spell_learner_*<lang>` key behind.

## Acceptance (CI)

1. A v2 log stored before CR-1 loads, with every new field `None`. The golden is a real pre-CR-1 blob.
2. Simulated older-build round trip: serialize through the old struct shape, reload, and check that the seen set is intact and no read panics.
3. Every `game::Outcome` except `Voided` maps to an `OutcomeClass`, and CR-I1 holds over all of them.
4. An Impossible-level word in Classic logs `impossible: true`. A plain Classic word logs `false`.
5. Every logged `mode` id exists in `config/modes.json`.
6. Seen-set backfill on a fixture with log, misses and redemptions equals their union.
7. Reset clears the seen key (CR-I5).
8. The telemetry planted-field test fails on each new term (CR-I3).
9. No `SCHEMA_VERSION` change, and an older-build fixture reads CR-1 state as `Stored`, not `Newer`.

---

## Decisions for Eric

| # | Decision | Recommendation |
|---|---|---|
| **L-D1** | Store `ts` at all? It adds a per-attempt timeline for a child, on device only (I1). | **Yes, whole seconds.** GUARDIAN F1 can't exist without it. |
| **L-D2** | GUARDIAN G-D16 excludes "close" outcomes, and no close class exists. | **Drop "close" from G-D16.** It's satisfied vacuously. Kid-lenient matches stay `Correct`. Alternative: add a `Lenient` class, which means plumbing a second result out of `answer_matches_in`. |
| **L-D3** | What counts as "seen"? | **Any logged attempt.** A served-but-voided word isn't seen, so it can be offered again. |
| **L-D4** | Backfill is lossy for words seen before CR-1 and outside the 512-entry ring. GUARDIAN A14 ("top-up never includes a seen word") can't be fully true for long-time players. | **Accept, and amend A14** to "seen since CR-1 or still in any learner store". The alternative is no top-up until a profile has 4 weeks of CR-1 history. |
| **L-D5** | `LOG_CAP` stays 512 per language. A heavy week, or C10's 4-week dry run, can outrun it. | **Keep 512.** GUARDIAN shows only what's present and needs no new label. Raising the cap is its own reviewed change. |
| **L-D6** | Does this ride L0 or wait for Phase 3? | **Ride now.** It's independent of the simulator and doesn't touch FSRS or BKT. |

## Non-goals

- No change to FSRS, BKT, selection, the misses queue or grade mapping.
- No `TRAP_MISS` rule ids.
- No multi-profile.
- No telemetry.
- No change to what voided rounds or Spell It record (nothing).
