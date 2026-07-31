# CC-LEARNING-ENGINE — On-Device Adaptive Learner Model

**Status: GREENLIT by Eric. REVIEW-GATED** — Eric reviews this file (including
D1–D6 sign-offs) **before code**. Then Phases L0–L2 are EXECUTABLE (pure
Rust/config/tool-side, base game + curated Spell Picture). L3 (spoken spelling)
is executable after L1 ships. Nothing here touches photo-upload gating.

Parent files: CC-PICTURE-BANK (bands, monotonicity law), CC-WORD-PICTURE
v6/v8.1/v8.2 (layout law), Voice Studio pipeline, CC-SCAN-STACK (D8 pinning
discipline, adopted here wholesale). Authority order:
**trust > layout law > learning fidelity > speed.**

## Intent

Spell Picture's bands decide what is *legal* to ask; nothing yet decides what
is *best* to ask this player right now. This file adds a **Learner Model**: one
on-device, per-player-per-language state that tracks mastery per hazard skill
and drives word selection, review timing and feedback. It turns the game into a
curriculum without changing what the game looks like — and it is the
SBIR/STARTALK innovation centrepiece: *adaptive per-hazard curriculum embedded
in a game, fully on-device.*

> **Governing law, stated once and enforced everywhere:** the Learner Model
> selects **within** the band. CC-PICTURE-BANK's monotonicity law, the solver
> and rendering are never influenced by learner state. A struggling player gets
> gentler picks inside the same band; the picture's difficulty curve never lies.

## Architecture: the Learner Model contract

One serialised state per (player profile × language), stored on-device:

- skill list, from the hazard taxonomy (CC-PICTURE-BANK D2)
- per-skill BKT parameters + mastery posterior
- per-skill FSRS scheduling state (stability, difficulty, due date)
- attempt log (bounded ring buffer)
- placement status, schema version

The contract is **pure data** (Rust structs, serde); every component below reads
and writes only through it. Deterministic: state + seed + attempt stream →
identical next state on any device. Exportable and deletable via the existing
progress-reset flow. Schema migrations are explicit and tested.

## Features

**1. Skill-based knowledge tracing (Phase L0).** Track "ie/ei order," not "the
word *receive*." Every bank word carries a hazard vector (extends the
CC-PICTURE-BANK manifest/word-bank schema); one attempt updates every skill the
word exercises. Per-skill Bayesian Knowledge Tracing maintains mastery, with
parameters initialised from per-language defaults (D2). Pure Rust in the core
crate, kilobytes of state, no neural net.

**2. FSRS-style spaced-repetition scheduler (Phase L0).** Skills come due; the
game reviews them without ever feeling like flashcards. FSRS scheduling state
per skill feeds due-ness into selection. Implemented in Rust — port the
published FSRS algorithm, pin the parameter set (D3). Review pressure is
invisible: it only biases which *legal* word appears next.

**3. Word selection policy (Phase L1).** The next word is the one this player
most needs, chosen from words the layer already permits. Within the current
layer's band — band membership is CC-PICTURE-BANK law — score candidates by
due-ness × mastery uncertainty × hazard coverage × recency-avoidance; pick
argmax with a seeded tie-break. Deterministic given state + seed. A
no-learner-state fallback (fresh install, reset) degrades to today's
band-random behaviour: **the game never blocks on the model.**

**4. Error diagnosis engine (Phase L1).** A miss is evidence, not just a red X.
Grapheme-level alignment — weighted edit distance over a per-language confusion
matrix, config-defined like the banding rules — classifies each misspelling
against the hazard taxonomy; ties go to a tiny pinned classifier only if rules
cannot decide (D4). Output: skill tags → BKT updates, plus the player insight
surface ("your pattern: silent endings") and the parent/teacher report line.
Speech-input attempts carry a channel flag so hearing misses never update
spelling skills.

**5. Placement (Phase L1).** Adaptivity by picture two, not week two. The first
session per language runs a short placement word set spanning the hazard space,
authored per language in word-bank config, native audit required (Tagalog →
Paul). It initialises BKT priors and is skippable — in which case
language-default priors stand.

**6. Player and guardian feedback surfaces (Phase L2).** The model earns trust
by showing its work, gently. In-game: one-line pattern insight, opt-in, never
mid-word, phrasing through the audit gate, Kid Mode variant reviewed by Eric.
Guardian report: per-skill mastery summary rendered on-device from the Learner
Model; nothing leaves the device except by the user's explicit share action.

**7. Spoken spelling input (Phase L3, after L1).** The mic button becomes a real
input mode — spell aloud, letter by letter. On-device `SFSpeechRecognizer` with
a letter-name-only grammar per language; per-language acoustic confusion pairs
(English b/v/d/e etc.) defined in config, native-audited. Below the confidence
threshold: show the best-guess letter and require a tap to confirm — **never
silently commit**. Audio never leaves the device and is never persisted. The
diagnosis engine receives `channel=speech`.

**8. Voice Studio QA harness (parallel track, tool-side).** Expert tier plays
the word once — the voice must carry it. A tool-side harness renders every bank
word in each language's custom voice, sequences confusable minimal pairs
back-to-back, and produces an audit playlist. A language's release flag requires
recorded native sign-off (Tagalog → Paul). System TTS remains the fallback where
a custom voice isn't ready; the harness gates custom voices only.

## Decisions

| # | Decision | Status |
| --- | --- | --- |
| D1 | Algorithm choice | **DECIDED**: BKT per skill + FSRS scheduler. Not IRT (needs population data we refuse to collect), not deep knowledge tracing (opaque, heavy, unpinnable). A later swap is a new reviewed file. |
| D2 | Priors and hazard taxonomies | English taxonomy + default BKT priors drafted first as the template (extends CC-PICTURE-BANK D2). Every non-English taxonomy, confusion matrix, placement set and letter-name grammar requires native-speaker audit before its language's release flag opens. **PROPOSED owner list needs Eric's sign-off** (Tagalog → Paul; others TBD). |
| D3 | Pinning | **DECIDED**: FSRS parameters, confusion matrices, BKT priors, tie-break classifier and speech grammars are version-pinned config/artifacts (CC-SCAN-STACK D8 discipline). Changes re-run the full eval suite as their own reviewed change. |
| D4 | Classifier scope | **DECIDED**: rules first. The tie-break classifier is optional, tiny, on-device, pinned, and consulted only when rules return ambiguous. If eval #2 passes rules-only, ship no classifier at all. |
| D5 | Privacy | **DECIDED**: all learner state on-device only; zero telemetry; export/delete via the existing progress-reset flow; guardian report shares only by explicit user action; speech audio never stored. Any A/B retention study (eval #1b) runs on-device with Eric-reviewed, opt-in, aggregate-only reporting — or is a TestFlight survey instead. **No silent experimentation on players.** |
| D6 | Placement length | **PROPOSED**: 10–14 words per language, ≤3 minutes, skippable. **Needs Eric's sign-off.** |
| D7 | Selection supremacy boundary | **DECIDED** (the law restated as a decision so no executor misses it): learner state may influence word choice within a band and feedback text **only**. Any code path where learner state reaches layout, rendering, band assignment, layer gating, scoring or audio-gate tiers is a **build-failing violation**. |

## Constraints and non-goals

- No cloud, no telemetry, no generative models, no population-level training.
  Everything runs and stays on-device.
- Do not modify the solver, layout law, band definitions, monotonicity CI,
  audio-gate tiers or scoring. This engine **consumes** bands; it never defines
  them.
- **No engagement-optimisation objectives.** The model optimises recall and
  mastery, never session length or streaks. Dark-pattern scheduling — "come back
  or lose progress" — is banned.
- Speech input is additive; typed input never degrades. Mic permission denied →
  the feature hides and everything else is unaffected.
- Do not touch CC-SCAN-STACK modules, photo-upload code, or the tracer beyond
  the QA harness in #8.

## Done when

1. **Tracing/scheduling evals (L0).** (a) Simulated-learner suite: BKT
   posteriors converge correctly on scripted mastery/forgetting traces;
   scheduler due-dates match the FSRS reference implementation exactly for the
   pinned parameters. (b) Calibration: on a recorded real-play attempt log
   (Eric's own play + TestFlight volunteers under D5), predicted P(correct)
   beats a frequency-only baseline on Brier score. (c) Determinism: same state +
   seed + attempt stream → byte-identical next state on macOS and iOS.
2. **Diagnosis eval (L1).** ≥90% agreement with Eric's hand labels on a
   200-misspelling English fixture set; the same bar per additional language
   before its flag opens; rules-vs-classifier contribution reported (D4).
3. **Selection eval (L1).** Property tests prove every selected word is
   band-legal (10k seeded runs, zero violations — D7 enforced); the fallback
   path activates correctly with empty learner state; argmax reproducibility
   across platforms.
4. **Placement eval (L1).** The placement set spans ≥90% of the language's
   hazard taxonomy; completing it shifts priors as specified in fixture
   scenarios; the skip path leaves defaults intact.
5. **Feedback surfaces (L2).** Insight strings pass the audit gate; Kid Mode
   variants Eric-reviewed; the guardian report renders correctly from three
   fixture learner states; sharing requires explicit action.
6. **Speech eval (L3).** ≥95% letter-name accuracy per language on a recorded
   child + adult test set; every configured confusion pair triggers the confirm
   flow below threshold; zero audio files on disk after a session, verified by
   test.
7. **Voice QA (#8).** The harness produces the minimal-pair playlist for
   Tagalog; Paul's sign-off recorded; CI blocks the custom-voice flag without it.
8. **Reset/migration.** Progress-reset deletes learner state completely
   (verified); a schema v1→v2 migration test exists from day one (migrating an
   empty vN file), so future migrations have a harness waiting.
9. **Eric's review of this file** with the D2 owner list and D6 signed off,
   recorded before Phase L0 code is written.

---

## Status against this file (2026-07-31)

**No code written, and none will be until Eric's review lands** — Done #9 is
this file's own precondition and it names the review as the gate.

Dependencies worth flagging now, because they change the critical path:

- **This engine's foundation does not exist yet.** Feature 1 needs a hazard
  vector per word; the hazard taxonomy is CC-PICTURE-BANK D2, and
  CC-PICTURE-BANK's own manifest schema is unbuilt. L0 cannot start before that
  manifest lands, regardless of when D1–D6 are signed.
- The existing word banks have frequency/tier data but **no hazard annotation
  and no per-skill anything** — feature 1's hazard vector is new schema on every
  bank, not a field toggle.
- Progress-reset exists and is the right hook for D5/Done #8.
- The Spell Aloud pipeline already owns a mic surface and a per-surface answer
  state (`spell_aloud.rs`), which is where L3's `channel=speech` flag would
  attach — feature 7 is an extension of that surface, not a new one.
