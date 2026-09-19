# CC-PLACEMENT-IDEMPOTENCE v1

[stashed verbatim from Eric's paste of 2026-08-30 (session d5d833c7), by CC-LEARNING-ENGINE-L0 census ruling C9. The Decisions D1–D6 below show no sign-off in any readable transcript; treat them as unsigned until Eric records otherwise. Built so far: F0 and F4 (game.rs, testseam.rs, learner.rs placement_f0).]

**Status:** REVIEW-GATED. Do not merge until Eric signs the Decisions section.
**Scope:** The placement probe only — the sheet whose player-facing string is
"Quick spelling check?" (internally referred to below as the *probe*; Eric calls
it the "quick bee"). Naming is settled in D6.
**Relationship to existing files**
- Amends **CC-LEARNING-ENGINE** (placement, D6 10–14 words). That file owns how a
  level is *estimated*. This file owns when the probe *runs* and what it draws.
  It does not touch the estimator.
- Consumes **CC-PERSIAN-FOUNDATION F6** (global Freshness Invariant: per-mode /
  language / tier rolling no-repeat window + exposure ledger). The probe becomes
  a registered consumer of that ledger, never a parallel path.
- Inherits the no-interruption rule from **CC-BLITZ** ("nothing may interrupt a
  run") and generalizes it to modal offers.
- Reuses the record discipline from **CC-SPELLPIC-AUDITPASS**: one record per key,
  writes replace, never append; plus a legacy-state sweep.
- Follows the step-0 prove-which-layer discipline used throughout the pipeline.
If any instruction here appears to reverse a signed item in those files, **stop
and ask** rather than inferring.
---
## Why this file exists
A player told Spell to find their level. It did. Then it asked again, mid-run,
over a live 20-word chain, with the same words.
Three separate defects are hiding inside that one sentence, and they have
different fixes. Fixing the visible one (same words) without separating them
will produce a build that still nags the player.
1. **Duplicate items inside one probe.** A repeated item carries no information
   about the player's level. It is measurement-worthless and reads as broken.
2. **The probe re-running at all.** A completed placement is not being recorded,
   or the offer trigger is not reading the record. Because the item draw appears
   deterministic, the re-run surfaces the *same* words — which is why the player
   described the bug as repetition rather than as a repeated prompt.
3. **Probe words reappearing in normal play.** The probe almost certainly does
   not write to the exposure ledger, so its words are outside the no-repeat
   window. This is the same class as the Daily-repeat bug F6 was written to kill.
And a fourth, visible in the screenshot: the sheet rendered **over an active
run**. The trigger evaluates on session start with no regard for in-progress
state, and — most likely — **Skip writes nothing**, so the offer re-fires every
launch until the player submits to it.
Intent of the whole file: **the probe is a cold-start bootstrap that happens once
per (profile, language), on the player's terms, and never shows a player a word
they have just seen.**
---
## F0 — Step 0: prove which layer, before changing anything
**Intent.** Causes 2 and 3 look identical from the outside and have opposite
fixes. A fix applied to the wrong layer will appear to work in one test and fail
in the next. Do not tune a dead code path.
**Mechanism.** Under a dev flag, emit a `PLACEMENT_TRACE` dump at these four
points and paste the output into the PR before any behavioral change:
| # | Question | Dump |
|---|----------|------|
| 0a | Does a completion record exist after a probe finishes? | Full record contents keyed by (profileId, language), or an explicit `NONE`. Include the write site's symbol name. |
| 0b | What does **Skip** write? | The record delta on tapping Skip, or `NONE`. Name the handler symbol. |
| 0c | What seeds the probe's item draw? | The seed value, its source (date / profileId / constant / unseeded), and the ordered item ids drawn. Run twice in one session and twice across a reinstall. |
| 0d | Do probe items reach the exposure ledger? | The ledger delta across a full probe, or `NONE`. Name the write site if one exists. |
| 0e | What predicate gates the offer? | The exact boolean expression and every field it reads. |
**Decision table — do not proceed past this until one row is confirmed:**
- 0a `NONE` → **cause 2 is a missing write.** F1 is the fix.
- 0a present but 0e ignores it → **cause 2 is a wrong predicate.** F2 is the fix.
- 0c seed is constant or profile-derived → the identical word set is *emergent*,
  not designed. F5 + D3 decide what replaces it.
- 0d `NONE` → **cause 3 confirmed.** F6 is the fix.
- 0b `NONE` → the nag loop is confirmed. F3 is the fix.
**If the dump shows something not on this table — in particular, if a placement
record already exists and is already read correctly — stop and ask.** That would
mean the real cause is elsewhere and this file's premise is wrong.
**Non-goal for F0:** do not fix anything during step 0. Dump, report, then fix.
---
## F1 — The Placement Record is the single source of truth
**Intent.** Everything downstream — whether to offer, what level to start at,
whether the player already said no — must resolve from one durable record.
Scattered session flags are how this bug happened.
**Mechanism.** One record per `(profileId, language)`:
```
PlacementRecord {
  profileId
  language
  state: COMPLETED | DECLINED | ABANDONED
  levelEstimate            // null unless COMPLETED
  itemIds: [String]        // items delivered this probe
  completedAt              // monotonic-safe timestamp
  probeVersion: Int        // bumped when item pool or length changes
}
```
Rules:
- **Writes replace, never append.** A re-placement overwrites the record for that
  key. There is never more than one row per key.
- Persisted to durable on-device storage, not session state. It survives
  force-quit, backgrounding, and app update.
- `ABANDONED` = the player entered the probe and left without finishing. It is a
  distinct state from `DECLINED` (never entered) because D2 may treat them
  differently; do not collapse them.
- No other module may write this record. Symbol scan lint: exactly one write
  site.
**Invariant (Placement Record Uniqueness).** For any (profileId, language) at
most one record exists. A test that force-writes two must fail.
---
## F2 — Placement Idempotence Invariant
**Intent.** A player who has answered this question should never be asked it
again by accident. Ever.
**Mechanism.** The offer's visibility is a **pure function of the record and
nothing else**:
```
shouldOfferProbe(profileId, language) =
    no PlacementRecord exists for (profileId, language)
```
That is the whole predicate at v1. Session count, launch count, "level not yet
set for this session," and time-since-install are all **illegal inputs** — they
are how the bug was born. Re-placement enters through F7 only, which writes the
record rather than bypassing the predicate.
**Invariant.** Rendering the probe offer while a `PlacementRecord` exists for the
current (profile, language) is a **build failure**, not a warning.
**Lints.**
- L1: the offer view has exactly one call site and it is guarded by
  `shouldOfferProbe`. Any other construction path fails the symbol scan.
- L2: `shouldOfferProbe` reads no field outside `PlacementRecord`. Enforced by a
  restricted signature — it takes the record, not a context object.
**Deliberate-failure test piece.** A fixture that adds a launch-count term to the
predicate must fail CI with a message naming the illegal input.
---
## F3 — Skip is a decision, and decisions are recorded
**Intent.** "Not now" and "never asked" are different states. Treating them the
same turns the offer into a nag, which is what the player actually experienced.
**Mechanism.** Tapping Skip writes `state: DECLINED` immediately, before the
sheet dismisses. There is no path from Skip to a re-offer except F7. The write is
synchronous with respect to dismissal — a crash during dismissal must not lose
the decline.
**Acceptance test.** Skip → force-quit → relaunch → same language: no offer. Ten
consecutive launches: no offer.
**Copy note.** If Skip's label implies "later" rather than "no," it now lies. Any
string change routes through **CC-LOCALE-TYPESET** and the audited-string path;
do not edit the string inline. If you conclude the label needs to change, **stop
and ask** — it is Eric's call, not a mechanical consequence of this file.
---
## F4 — No Interruption Invariant
**Intent.** The screenshot shows the offer landing on top of a live 20-word
chain. Whatever else is true, a modal that interrupts a run in progress is
unacceptable — a player who has 20 right in a row is having the exact experience
the app exists to produce.
**Mechanism.** Generalize the CC-BLITZ rule: **no modal offer may render while a
run is active.** The probe offer presents only at a run boundary — a fresh
session with no active run, or immediately after a run resolves. If the predicate
goes true mid-run, the offer queues until the boundary; it does not fire and does
not silently drop.
**Invariant.** Any modal presentation attempted while `runActive == true` fails
the build. This is registered as a general rule, not a probe-specific one, so
future modals inherit it.
**Test.** Start a run, force `shouldOfferProbe` true mid-run, assert zero
presentations until the run resolves, then exactly one.
---
## F5 — Probe Distinctness Invariant
**Intent.** A duplicate item inside a probe measures nothing and looks broken.
This must be impossible to construct, not merely unlikely to occur.
**Mechanism.** Probe construction returns a set with distinct `itemId`s, or it
fails. There is no dedupe-after-the-fact pass and no silent retry loop — a
generator that *can* emit a duplicate is the defect.
**Invariant.** For probe length N, the delivered item list has exactly N distinct
ids. A duplicate is a construction failure surfaced as a build/test failure, never
tolerated at runtime.
**Tests.**
- 1000 seeds × every registered language × probe length N → zero duplicate ids,
  zero short draws.
- **Pool-floor feasibility check** (borrowed from F6's machinery): for every
  (language, tier) the probe can draw from, assert the eligible pool ≥ N after the
  Freshness window is applied. A language whose pool cannot fill a distinct probe
  fails CI at build time, naming the language — it must never fail at runtime in
  front of a player.
- **Deliberate-failure piece:** a generator with sampling-with-replacement fails
  CI.
---
## F6 — Placement Exposure Invariant
**Intent.** The player's complaint was about *words*, and this is the part that is
literally about words. Probe items are words the player just saw. They belong
inside the no-repeat window like every other delivered word.
**Mechanism.** Probe items are written to the **CC-PERSIAN-FOUNDATION F6 exposure
ledger** on delivery, through the existing ledger API. The probe is a registered
consumer of the global mechanism, **not a parallel path** — no second ledger, no
probe-specific window.
If wiring the probe into the ledger appears to require changing the signed F6
mechanism, **stop and ask**.
**Invariant.** Every word delivered by the probe is recorded as exposed for
(profile, language, mode) at delivery time — not at probe completion, so an
abandoned probe still counts its delivered words.
**Acceptance test.** Complete a probe of N words, then play normal mode: the next
20 delivered words contain zero probe items (assuming the pool floor permits;
where it does not, the F6 pool-floor CI already fails the build).
---
## F7 — Re-placement has exactly one door
**Intent.** Some players will legitimately want to re-take it. That should be a
thing they choose, in a place they can find, not something the app decides for
them.
**Mechanism.** A single entry point in Settings — "Find my level again" or
equivalent, per D6 for wording. Tapping it clears the record for the current
(profile, language) and lets the F2 predicate go true naturally. It does **not**
bypass the predicate.
**Settings-truth binding.** This control is a rendered toggle/action and is
therefore bound by the **CC-AUG6-AUDITPASS settings-truth invariant**: it appears
in the effects manifest with a passing effect test proving it produces the probe
on the next boundary. A dead control here is a build failure like any other.
**Non-goal.** No automatic re-placement on drift, absence, or level change at v1.
See D2 for the reasoning.
---
## F8 — Legacy state sweep
**Intent.** Players already in the field have whatever half-state the current
build left them in. A correct predicate reading a garbage record still misbehaves.
**Mechanism.** A one-time migration on first launch of the fixed build:
- Any existing per-session or per-launch placement flag is read once, translated
  into a `PlacementRecord`, then deleted. Deletion is part of the migration, not
  a follow-up task.
- A player with evidence of a completed placement (a stored level estimate, a
  probe history entry — whatever 0a reveals) migrates to `COMPLETED`, preserving
  the level estimate. **Do not re-probe an existing player to recover state.**
- A player with no such evidence migrates to no record, and will be offered the
  probe once, at a run boundary. This is correct and acceptable.
- Migration is idempotent: running it twice produces the same result.
- If 0a's dump shows no recoverable evidence *at all* for existing players, that
  is a scope change (every existing player gets re-offered once) — **stop and
  ask** before shipping it.
**Test.** Fixture stores for each pre-migration shape → migrate → assert record
contents and assert the legacy keys are gone. Re-run migration → byte-identical
state.
---
## Decisions
Sign or amend each. Unstated decisions are where agents improvise.
- **D1 — Placement scope.** Per `(profile, language)`, not per profile.
  *Rationale:* a level estimate does not transfer across languages; a player
  placed in English has told you nothing about their Polish. **Recommended:
  per-language.**
- **D2 — Automatic re-placement.** Never at v1; Settings-initiated only (F7).
  *Rationale:* the learner model (BKT + FSRS, CC-LEARNING-ENGINE) already tracks
  skill drift continuously and with far more data than a 10–14 word probe. Auto
  re-placement is therefore both redundant and, as this bug demonstrates, the
  exact mechanism by which players get re-asked. **Recommended: never.**
- **D3 — Item draw on a deliberate re-run.** Fresh items under the F6 Freshness
  window, not a fixed anchor set.
  *Rationale and the tradeoff, stated honestly:* fixed anchor items make two
  ability estimates directly comparable, which is the textbook reason to keep
  them. That value is real but small here, because the probe is only a cold-start
  bootstrap — the ongoing estimate comes from the learner model, not from
  probe-to-probe comparison. Against it: a player who deliberately re-takes the
  test and sees the same words concludes the app is broken, which is precisely
  the report that produced this file. **Recommended: fresh.** If you want
  comparability later, the seam is `probeVersion` plus a tagged anchor subset;
  do not build it now.
- **D4 — Skip persistence.** `DECLINED` persists indefinitely; re-offer only via
  F7.
  *Rationale:* an expiring decline is a nag with a timer. **Recommended:
  permanent.**
- **D5 — `ABANDONED` handling.** A player who enters the probe and quits partway.
  Options: (a) treat as `DECLINED`, never re-offer; (b) re-offer once at the next
  run boundary, then treat as declined; (c) resume the partial probe.
  **Recommended: (b)** — an abandon is more often an interruption than a refusal,
  but one retry is the limit before it becomes a nag. Note (c) is rejected because
  a resumed probe measured across two sessions is a worse measurement than a
  fresh one. **Eric decides.**
- **D6 — Canonical name.** The internal identifier and the player-facing string
  currently disagree ("quick bee" vs. "Quick spelling check?"). Pick one internal
  name and use it in every symbol, key, and test. Any change to the *displayed*
  string routes through CC-LOCALE-TYPESET and the audited-string path.
  **Recommended internal name: `placementProbe`.** Player-facing string unchanged
  at v1.
---
## Done
Not "the tests pass." Done when all of these hold:
1. F0's five dumps are pasted in the PR and one decision-table row is explicitly
   confirmed in writing.
2. Probe → force-quit → relaunch → same language: **no offer.** Ten consecutive
   launches: no offer.
3. Skip → relaunch × 10: **no offer.**
4. Switch to a second language with no record: offer appears **once**, at a run
   boundary.
5. Offer forced true mid-run: **zero** presentations until the run resolves, then
   exactly one.
6. 1000 seeds × every registered language: zero intra-probe duplicate ids, zero
   short draws; pool-floor feasibility CI green for every language.
7. Post-probe normal play: first 20 delivered words contain zero probe items.
8. Settings re-placement action: effect test passes, probe appears at the next
   run boundary, record is replaced not duplicated.
9. All three deliberate-failure pieces (illegal predicate input, duplicate-capable
   generator, second record for one key) fail CI with messages naming the cause.
10. Migration fixtures pass and are idempotent; no legacy keys remain.
11. **Eric's device pass:** takes the probe, force-quits, relaunches, plays a run
    to 20, and is never re-asked and never sees a probe word again in that run.
    This gate is his call, not the suite's.
---
## Constraints and non-goals
- **Do not touch the estimator.** How a level is computed from probe answers is
  CC-LEARNING-ENGINE's, unchanged here.
- **Do not change probe length.** D6's 10–14 words stand.
- **Do not touch Daily's date-seeded determinism.** Daily is a different
  mechanism with a different correctness property.
- **Do not build a second freshness mechanism.** The probe consumes F6 or it
  stops and asks.
- **Do not add analytics or telemetry.** On-device only, per the standing COPPA
  posture.
- **Do not "improve" the probe's UX, copy, or visual design in this pass.** This
  file fixes when it runs and what it draws. Scope creep here is the
  characteristic failure mode; a helpful redesign will collide with
  CC-LOCALE-TYPESET and the audited-string path.
- **Do not implement anchor-set comparability, drift-triggered re-placement, or
  partial-probe resume.** Tagged as seams, explicitly out of scope at v1.
