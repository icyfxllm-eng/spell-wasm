CC-CALENDAR v2 — Spell Calendar: Plan It, Recap It, Conquer 5 New Words
[stashed verbatim from Eric 2026-08-02 — full text as pasted in chat]
Status: REVIEW-GATED. v2: D1 resolved by Eric — goal-origination model final (kid surfaces only; parent = read + cheer; Sunday ritual with kid-tap confirm; I7 CI-enforced).

## FULL TEXT (recovered from session transcript 2026-08-05 — the stub had pointed at the transcript only)

CC-CALENDAR v2 — Spell Calendar: Plan It, Recap It, Conquer 5 New Words
Status: REVIEW-GATED. v2: D1 resolved by Eric — goal-origination model below is final. Reads through ReportsQuery (CC-REPORTS); coordinates with the FSRS scheduler (CC-LEARNING-ENGINE); goal ring may join the widget snapshot (CC-IOS-SURFACES, D5). Inherits CC-BUY-DRIVERS shared law S1–S5.
Intent
The FSRS scheduler already plans every day's ideal review — invisibly. The Spell Calendar makes the plan visible and steerable: look back at any day ("what did I miss Tuesday?"), look forward at what's coming, drag words you chooseonto future days, and set a goal like "learn to spell 5 new words this week." Ownership is the point — a kid who planned Thursday's words shows up for Thursday.
Two governing laws, fixed:

1. The planner steers, the engine drives. Planned words join a session as guaranteed inclusions only if band-legal; the scheduler's due reviews are never displaced. The calendar has zero authority over word legality, difficulty bands, or scoring (CC-LEARNING-ENGINE D7 extended).
2. Goals are quests, never debts. An unmet goal quietly rolls forward. No red badges, no broken-streak shaming, no guilt copy, no minutes-based goals ever. The moment a calendar makes a kid feel behind, it has failed regardless of engagement numbers.

Features
1. Calendar view (month + week)
Per profile × language. Day cells carry outcome dots (practiced / words mastered / goal progress) from the daily journal (feature 5). Past days tap into Recap; today taps into the session launcher; future days tap into the Planner. Unplayed past days render empty, not marked failed — no X's, no grayed-out shame states.
2. Daily Recap (past days)
Tap a day → that day's story from journal + ReportsQuery: words practiced, words missed that day (each with a "Take it on" deep link into smart-review — reports-are-doors law), words mastered, goal progress that day. Conquest-register strings only (CC-REPORTS I4 pool shared).
3. Forward Planner
Drag/add words onto future days from four sources: Ready-for-a-rematch (FSRS due forecast *[Annotated 2026-09-19, CC-LEARNING-ENGINE-L0 census C8: as built, this reads the Leitner misses queue through `LearnerQuery::at_risk_set` (formerly `reports::rematch_set`), not FSRS; it becomes FSRS when L0 R2 moves the misses queue to per-word FSRS]*), Words to Conquer, Trap Boss word sets, and My Words (existing custom-word path — already profanity-screened at import; the planner adds no new free-text entry). Planned words for a day become guaranteed session inclusions, band-legality filtered — an illegal-for-band word is declined at planning time with one honest audited reason, never silently dropped at session time. Future days also preview the scheduler's own forecast ("8 reviews coming due"), visually distinct from kid-planned words.
4. Word Goals — the dealt hand
No blank goal form exists anywhere; a "set a goal" empty form is homework and kills the feature. Instead, each week the app deals 2–3 goal cards generated from the kid's own ReportsQuery data — e.g. "Conquer 3 boss words", "Learn 5 new words" (default N=5/week — D4), "Rematch your review set", "Complete N Daily Challenges" — and the kid taps one. Suggestion kills friction; selection preserves authorship: a chosen goal is theirs even though the system drafted the options. All cards conquest-framed and word-denominated (never time-denominated). Progress ring on the calendar header; completion fires the existing ceremony assets (D6 — no new reward economy). Unmet goal → rolls into next week silently, and its card may be re-dealt. One active goal per type max; goals are per language. Card generation is deterministic from learner state (testable), always includes at least one "small win" card, and never deals a card the current band makes impossible.
5. Daily journal (the one new persistence — D3 sign-off required)
Date-keyed, on-device, per profile × language: words practiced (refs), missed (refs), mastered (refs), goal snapshots. Learning outcomes only — no session durations, no timestamps finer than the date, no counts of app opens (I3). Capped at 730 days rolling; append-only from session end; ReportsQuery is its only reader. This is authoritative new data, so it is explicitly gated on D3 — CC-REPORTS I2's exception discipline applies: build nothing until signed.
6. Widget tie-in (conditional on D5)
Goal ring + today's planned-word count join the App Group snapshot (CC-IOS-SURFACES D2 machinery, read-only, no new extension capability).
7. Cheer cards + the Sunday ritual (the parent's role)
Involvement flows up, support flows down — pressure direction inverted by construction:

* Cheer cards: Guardian Dash shows the kid's chosen goal and progress read-only, with exactly one action: send a cheer — a canned line from a new small audited cheer pool (15-language, hard-capped) that appears on the kid's calendar as a card ("Dad's cheering for this one"). Max one active cheer per goal; no free text, ever. Checking for a cheer is the pull that brings kids back to the calendar.
* Sunday ritual: a "pick this week's goal together" flow built for one device and two people: the hand is dealt, discussed out loud, and the kid taps confirm. Parents shape goals through conversation — the thing they actually wanted — while the software records only kid authorship. We never build the assignment button; we build the couch moment.
* Anticipation reveals: planned future days render as wrapped "Thursday's words" previews that unwrap when the day arrives. Forward-pull comes from what's waiting, never from being behind (I4).

Decisions

* D1 (RESOLVED by Eric, v2): Goals originate on kid surfaces only, via the dealt hand (feature 4); parent surfaces are read-and-cheer only (feature 7); the Sunday ritual is the sanctioned co-setting path with the kid tapping confirm. No goal create/edit/delete affordance exists behind the parental gate — CC-GUARDIAN-DASH's ban on parent-assigned goals stands unamended. This resolution is I7 and is CI-enforced, not policy-remembered.
* D2 (proposed): Free/Complete split: calendar view + Daily Recap + goals free (they compound the daily habit that sells everything else); multi-week Forward Planner (beyond the current week) Complete. Sign off.
* D3 (open, blocking feature 5): Daily journal persistence as specced — on-device, date-granularity, outcomes-only, 730-day cap. Without it, Recap can only show what current analytics retain and the file ships reduced. Sign off before any journal code.
* D4 (proposed): Default goal = 5 new words per week, adjustable 3–15. Your "5" could also read as per-day; per-day 5 = 35/week is a heavy default for a kid. Confirm the unit.
* D5 (proposed): Widget goal ring in/out.
* D6 (decided): Goal completion reuses existing ceremony assets; no coins, stars, chests, or any new reward currency, ever.

Invariants

* I1: Planner writes only to its own plan/goal/journal store — zero writes to learner model, scheduler state, or analytics (build-fail boundary, same CI pattern as CC-REPORTS I1).
* I2: Session composition order is fixed and singular: FSRS due set + band-legal planned words + engine fill. No mode may reimplement it.
* I3: No time-denominated anything: no minutes goals, no duration display, no session-length storage (extends GUARDIAN-DASH D3 / REPORTS I5).
* I4: No guilt states: unplayed days empty, unmet goals roll silently, zero loss-framed strings (conquest pool + lint, shared with CC-REPORTS).
* I5: All strings from audited pools; unaudited languages get dots-and-numbers fallback (defs-dark pattern).
* I6: On-device, zero network; app-only (`platforms: ["app"]`); absent from Little Speller entirely; local notifications (if any goal/plan reminders) opt-in, off by default, max one per day.
* I7 (goal origination): Goal create/edit/delete affordances exist on kid surfaces only. Gated (parent) surfaces expose read + cheer and nothing else — enforced by the same CI symbol/affordance scan pattern as the zero-purchase-surface invariant, so a violation is a build failure. Cheer content comes exclusively from the audited cheer pool; no free-text path to a kid surface exists anywhere in this feature.

Non-goals
Do not touch FSRS/BKT internals, band logic, scoring, or Daily Challenge seeding. No shared/family calendars, no external calendar (EventKit) integration v1, no rescheduling of the scheduler's own reviews (you can add words, never remove or defer due reviews), no web.
Acceptance tests / Done

1. Plan fixture: plan 3 band-legal words for tomorrow → tomorrow's session word queue contains all 3 + the full FSRS due set (order per I2), verified against scheduler output.
2. Illegal-plan fixture: band-illegal word declined at planning time with the audited reason; session runs unaffected.
3. Recap correctness: scripted 5-day fixture play history → each day's Recap lists exactly that day's misses/masteries; day-attribution exact at date granularity.
4. Goal engine: "learn 5 new words" fixture crosses 5 mid-week → ring completes, ceremony fires once; 4/5 at week end → rolls silently, zero loss-framed strings rendered (snapshot + lint).
5. CI boundaries: planner write to learner/scheduler state fails build; journal reader other than ReportsQuery fails build.
6. Journal cap: 731st day evicts day 1; store size bounded (measured).
7. Empty states: unplayed past day renders empty cell; no-goal state renders invitation, not warning.
8. Proxy run: full calendar + recap + planner + goal flows = zero network requests.
9. Little Speller build: zero calendar symbols (symbol scan); widget ring present only if D5 signed (snapshot test).
10. D3 sign-off recorded in this file before any feature 5 (journal) code. (D1 resolved in v2.)
11. Dealt-hand determinism: fixture learner state → exact expected 3-card hand, including one small-win card; band-impossible goal type never dealt (planted fixture).
12. I7 scan: a test goal-edit affordance on a gated surface fails CI; a test free-text cheer path fails CI; cheer flow end-to-end = parent sends pool cheer → card renders on kid calendar → max-one-per-goal enforced.
13. Sunday ritual: co-set flow completes only via kid-context confirm; parent-context confirm attempt is structurally impossible (no such control renders), snapshot-verified.
14. Reveal states: future planned day renders wrapped preview; unwraps on its date; snapshot pair.
## DECISIONS SIGNED (Eric, 2026-08-05, same verdict: "Lets greenlight
all these for the app not the website")
- D2 SIGNED as proposed: calendar + Recap + goals free; multi-week
  planner Complete. (Calendar was already app-only by I6.)
- D4 SIGNED as proposed: 5 new words PER WEEK default, adjustable 3-15.
- D5 SIGNED: widget goal ring IN (rides CC-IOS-SURFACES D2 snapshot).

## D3 SIGNED (Eric, 2026-08-05): "D3"
Daily journal persistence as specced — on-device, date-granularity,
outcomes-only, 730-day rolling cap, ReportsQuery its only reader. The
journal shipped in 125 built to exactly this shape; the signature now
matches the tree, closing the one place where the ledger contradicted
the code.
