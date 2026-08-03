CC-GUARDIAN-DASH — The Reason Parents Pay $12.99 (BD-3)
Status: REVIEW-GATED. Blocked on CC-LEARNING-ENGINE L1 insight surfaces existing. Inherits CC-BUY-DRIVERS shared law S1–S5.
Intent
Parents don't buy game modes; they buy knowing their kid is learning. The learner model (BKT skills + FSRS scheduler + grapheme error diagnosis) already computes everything a great progress report needs — this file only surfaces it, behind the parental gate, in plain language. The dashboard becomes the App Store screenshot for Complete. The characteristic failure mode here is scope creep into the engine: this file has zero authority over the learner model (CC-LEARNING-ENGINE D7 boundary — read-only, build-fail on violation).
Features
1. Guardian view (parental-gated)
Per kid profile × language:
* Mastery map: skill list with BKT mastery states, grouped how a parent thinks (sounds, letter patterns, word lengths) not how the model thinks.
* Trouble spots: top current struggles from the grapheme-diagnosis data + existing heatmap analytics, each with one plain-language line ("mixes up -tion and -sion").
* What's next: the FSRS scheduler's upcoming review set, framed as "this week we're working on…".
* Placement summary: where the kid landed and movement since.
2. Weekly digest (opt-in, local)
One local notification per week per profile, generated on-device from the same data ("Maya mastered 14 new words in Spanish"). Off by default. No remote push, no server.
3. Printable report
Client-side PDF (Noto/OFL fonts, practice-sheet precedent): one page per language, mastery map + trouble spots + week-over-week. Export via share sheet/Files, metadata-free.
4. Plain-language layer
All parent-facing interpretation strings come from a hard-capped audited template pool; the learner model fills slots (skill names, counts, words). Free-composed model text never renders (flag-only agent doctrine's sibling: template-only surface text).
Decisions
* D1 = BD-D3 (proposed): Free tier gets the full dashboard for English only — real data, no teaser blur; Complete unlocks all languages. This makes the free experience honest and the upgrade obvious. Sign off.
* D2 (decided): No cross-child comparison anywhere. Each kid's view stands alone.
* D3 (decided): No time-on-app metrics, session counts, or engagement stats on the dashboard. Learning outcomes only. We do not teach parents to optimize screen time.
* D4 (proposed): Education edition: teacher role sees the same view per rostered profile — recorded as design intent, executes nothing until rostering exists (currently deferred).
* D5 (decided): Report PDF carries the SpellGame wordmark (it will be shown to teachers/family — that's earned marketing; CC-FINALE D2 alignment).
Invariants
* I1: Strictly read-only over learner-model state — no writes, no recompute triggers, no feedback loop from dashboard viewing into scheduling (CI-enforced module boundary).
* I2: Everything on-device; the dashboard and digest generate zero network traffic (S1).
* I3: Not reachable from Kid Mode or Little Speller; parental gate on every entry path including deep links.
* I4: Every rendered sentence traceable to an audited template; the definitions-dark machinery pattern applies — a language's dashboard interpretation strings render only when that pool is audit-covered, else numbers-and-skill-names-only fallback layout.
Non-goals
Do not modify the learner model, scheduler, placement, or heatmap computation. No goals/rewards parents set for kids (no external-pressure mechanics). No email/export automation. No web version.
Acceptance tests / Done
1. Dashboard renders correctly from three fixture learner states (new kid, mid-progress, long-history) × 2 languages, snapshot-tested.
2. CI boundary test: a dashboard-module write against learner state fails the build.
3. Network assertion: full dashboard + PDF export flow under a proxy shows zero requests.
4. Parental gate: Kid Mode session cannot reach any dashboard route, including via deep link.
5. PDF golden: fixture profile produces a deterministic, metadata-free PDF; opens in Preview/Files.
6. Digest test: opt-in fixture generates exactly one correctly-worded local notification; opt-out generates none.
7. Template coverage: unaudited-language fixture renders the fallback layout with zero interpretation sentences.
