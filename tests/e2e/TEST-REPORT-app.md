# TEST-REPORT — Web E2E (app build)

**19/19 passed** across 1 areas.

## drawer — 19/19
- ✅ every drawer row presses something that can receive it
- ✅ drawer: opens from the meta corner and renders registry rows
- ✅ drawer: every mode row has a destination that exists
- ✅ drawer: word_stories is never rendered (F8 hard gate)
- ✅ drawer: a hidden mode cannot be flagged back on
- ✅ home: the empty chains line reads as a sentence, not a column
- ✅ drawer: rows are localized with no new copy (es)
- ✅ drawer: scrolling the panel never scrolls the page behind it
- ✅ drawer: A2.3 — no locks, no upsell (Full-tier gating still UNPROVABLE)
- ✅ drawer: A2.2 — Little Speller sees only kidSafe rows, zero upsell
- ✅ placement: offered once, skip stands, no re-offer
- ✅ placement: Try serves the set through the real session
- ✅ placement: the offer never interrupts a live run
- ✅ placement: a COMPLETED probe never re-offers, across a reload
- ✅ learner: a NEWER build's stored state survives an older build, byte for byte
- ✅ learner: unreadable stored bytes are backed up exactly, then play recovers
- ✅ learner: a stored v1 state migrates to v2 on the next write, losing nothing
- ✅ learner surfaces render through LearnerQuery: Reports, Stats, Guardian Dash, Calendar
- ✅ learner: "reset this language" deletes its learner state and backup, only for that language
