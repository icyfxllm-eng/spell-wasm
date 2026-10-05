# CC-SNAP-SCHEDULE 3.2 — what a test date would cost L0

Run 2026-10-05 against `main` at `3b04bd5b`. Read-only; nothing built,
nothing decided.

CC-SNAP-ROADMAP 3.2 says a list's missed words are "scheduled through
CC-LEARNING-ENGINE-L0 R2 (FSRS) with a due-by constraint = test date − 1
day", and gates itself on "L0 R2 must have landed". The roadmap's own open
list then notes that this is a change request to L0 rather than a read-only
use of it.

That note is right, and this file says how much of a change.

---

## 0 — Two corrections to the premises first

**The gate is satisfied. R2 landed.** `src/review.rs` is R2 as built: "THE
rule that decides when a missed word comes back (I5: exactly one)", running
FSRS-4.5 through `learner::fsrs_review_graded`, having replaced the Leitner
boxes. `misses.rs` and `tone_drill.rs` both schedule through it and neither
computes a due time of its own. 3.2 does not HALT on its stated gate.

**But a comment says otherwise, and it is stale.** `learner_query.rs:21`
reads:

> **Until R2.** The due queue and the at-risk set read the Leitner misses
> queue (`misses.rs`), as `reports::rematch_set` did.

The due queue does still read `misses.rs` — but that queue has been
FSRS-scheduled since 2026-09-19. Anyone checking 3.2's gate by reading the
consumer rather than the engine would conclude R2 had not landed. Corrected
in the same commit as this file; no behaviour changed.

---

## 1 — Why a due-by is not a read-only use

FSRS answers a forward question: *given this memory state, when should the
next review be?* A test date asks a backward one: *by day D, make these N
words retrievable.* The engine has no way to express the second.

The one rule ends every review with:

```rust
f.due_day = day + interval_days(f.stability);
```

`interval_days` is the interval at which retrievability falls to
`RETENTION`, and `RETENTION` is a private crate constant (`0.9`), not a
parameter. There is no argument anywhere in the path that could carry "but
not later than D". Confirmed by search: the only `deadline` in the tree is
the game's round timer, and there is no `due_by`, `due_date` or equivalent.

## 2 — The naive implementation is actively harmful, not merely incomplete

The obvious move is to clamp: after FSRS sets `due_day`, lower it to
`test_date − 1`. **That corrupts the learner's state**, because `due_day`
is not just an output — it is the only record of when the word was last
seen. The next review reconstructs elapsed time from it:

```rust
let elapsed = day.saturating_sub(f.due_day.saturating_sub(interval_days(f.stability)));
let r = retrievability(elapsed.max(0.0), f.stability);
```

There is no `last_review` field; the last review day is inferred as
`due_day − interval_days(stability)`. Clamp `due_day` earlier and that
inference moves earlier too, so `elapsed` is overstated, `r` understated,
and the stability increment — which scales with `(e^(w10·(1−R)) − 1)` —
comes out **too large**. The learner would record the word as better known
than it is, permanently, and the error compounds at every subsequent review.

So any due-by work must add an explicit last-review field before it touches
`due_day`. That is a schema change to the learner's stored state, which is
the thing L0's own migration rules govern.

## 3 — And the feature fights the formula

Even implemented correctly, pulling reviews earlier buys less than it looks.
The stability increment is

```
inc = e^w8 · (11−D) · S^−w9 · (e^(w10·(1−R)) − 1) · hard
```

As a review moves earlier, retrievability `R` rises toward 1, `(1−R)` falls
toward 0, and `inc` falls toward **zero**. A word reviewed well before it
was due gains almost no durable stability. This is not a claim about
learning science; it is this function, and it is why cramming a list before
Friday will produce many reviews that each count for very little.

That does not make 3.2 a bad feature — a child who passes Friday's test has
got what the family wanted. It does mean the honest framing is "extra
practice before a date", not "FSRS scheduling with a deadline", and the
on-device streak 3.2 describes ("lists completed before test day") measures
the former.

## 4 — The smallest change that is not wrong

Offered as a bound on the work, not a design; the file that owns this does
not exist yet.

1. **Add `last_review_day` to `FsrsState`**, with L0's migration rules for a
   stored-state change. Derive `elapsed` from it instead of from `due_day`.
   This is worth doing whether or not 3.2 ships: the current inference is
   correct only because nothing writes `due_day` from outside, which is an
   invariant nobody states and nothing checks.
2. **Give the one rule the constraint as a parameter**, not a second path.
   `review.rs` is I5's single rule and must stay single: a due-by that
   bypasses it is a build failure by L0's own invariant. Its purity (I7,
   "pure functions of (state, grade, now)") becomes
   `(state, grade, now, due_by)` — still pure, but the signature of the
   most consequential function in the learner changes, and every caller
   with it.
3. **Decide what the constraint does.** Clamping the interval is one
   option; scheduling extra non-FSRS practice sessions that do not feed
   `fsrs_review_graded` at all is another, and keeps the memory model
   honest. The second is more code and less damage.

## 5 — Two questions 3.2 does not currently answer

- **A word can be in two lists.** `WordList` has no date field, nothing
  stops the same text appearing in several lists, and scheduling is
  per-word. If Monday's list and Friday's list share `because`, which date
  governs? The roadmap does not say, and the engine has one `FsrsState` per
  word to put an answer in.
- **Graduation races the test date.** `GRADUATE_AFTER_DAYS = 7`: a word
  whose next interval exceeds a week leaves the queue entirely. A word
  learned early in a two-week run graduates before the test and stops being
  reviewed. Does a test date hold it back, and if so, is it still
  "mastered" for the journal and redemption, which fire on graduation?

## 6 — What I am not going to do

Pick between clamping and parallel practice. That is a judgement about what
the product owes a child the night before a spelling test, and it trades a
real cost (a corrupted-or-complicated memory model, in the engine that every
other mode reads) against a real benefit (passing Friday). It needs Eric,
and it should be decided before CC-SNAP-SCHEDULE is written rather than
discovered while writing it.
