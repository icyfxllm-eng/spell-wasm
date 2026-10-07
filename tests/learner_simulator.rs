//! CC-LEARNING-ENGINE-L0 **R3 — the learner simulator** (Phase 3).
//!
//! WHY THIS EXISTS. D5 leaves no corpus of real children, and
//! `src/telemetry/schema.rs` bars every learner term from telemetry (I2, the
//! zero-egress posture) — so there is no field evidence about the scheduler
//! and there never will be. This is the only evaluation channel the design
//! permits. Correctness has to be established against synthetic players with
//! known ground truth, the same trust pattern as the tracer: a number earns
//! authority by reproducing known verdicts.
//!
//! WHAT IT PROVES, AND THE LIMIT. It proves the IMPLEMENTATION behaves
//! correctly for players who behave as the model assumes. It does **not**
//! establish that the model describes real children. The report states that
//! verbatim, and D6 (how a grant claim is worded) is Eric's.
//!
//! REPORT-ONLY, ON PURPOSE. D5 — the tolerances — is open, and the roadmap
//! says Phase 3 produces the first report and tolerances are frozen after
//! Eric reads it. So the retention figures below are REPORTED, not asserted
//! against a pass mark. What IS asserted is the set of invariants R3 names
//! outright: no divergence, no unbounded intervals, no starved queue. Those
//! need no tolerance to be wrong.
//!
//! THE REPORT IS A GOLDEN ARTIFACT. Everything here is seeded and
//! deterministic (I7), and the report carries no timestamp, so
//! `reports/simulator.md` is byte-identical run to run. It only changes when
//! the scheduler's behaviour changes — which is exactly when a diff is worth
//! reading. Regenerate with:
//!
//!     cargo test --test learner_simulator
//!
//! It lives in `tests/` rather than `src/` so it cannot reach a shipped
//! binary at all; no feature flag is needed to keep it out.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use spell_wasm::learner::{retrievability, Grade};
use spell_wasm::review::{on_correct, on_miss, start, Outcome, ReviewState};

const DAY_MS: f64 = 86_400_000.0;
/// A child takes a moment per word. Without this the clock never moves for a
/// player who is always wrong: `on_miss` returns `now + LEARN_STEPS_MS[0]`,
/// and that first step is zero.
const ANSWER_GAP_MS: f64 = 20_000.0;
/// FSRS's own target retention, the number the scheduler is aiming at.
const TARGET_RETENTION: f64 = 0.9;
/// No real child's review should ever be scheduled a decade out.
const SANE_INTERVAL_DAYS: u32 = 3650;
/// Enough reviews to reach graduation or expose a runaway, not so many that
/// the suite is slow.
const MAX_STEPS: usize = 400;

// ----------------------------------------------------------------- the dice

/// xorshift64*, so the run is seeded and reproducible with no dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_f64(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        // 53 bits, the mantissa, so the value is uniform in [0, 1).
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

// -------------------------------------------------------------- the players

/// How a synthetic player decides whether they remember a word.
#[derive(Clone, Copy, Debug)]
enum Player {
    /// Answers Good every time.
    AlwaysCorrect,
    /// Answers Again every time.
    AlwaysWrong,
    /// A coin flip, independent of how long it has been.
    Random,
    /// Plays well, then disappears for `gap_days`, then resumes.
    LongAbsence { gap_days: u32, before: usize },
    /// A real memory, and the point of the exercise. Recall after `t` days is
    /// FSRS's OWN retrievability curve at a true stability the player
    /// actually has — so this population behaves exactly as the model
    /// assumes, which is the scope R3 claims and no more. True stability
    /// grows by `growth` on recall and collapses to `s0` on a lapse.
    ModelMatched { s0: f64, growth: f64 },
    /// The same idea with a DIFFERENT curve shape: classic exponential
    /// forgetting, p = 2^(-t/H). Outside the model's assumptions, reported
    /// for information and claimed for nothing.
    Exponential { half_life0: f64, growth: f64 },
}

impl Player {
    fn name(self) -> &'static str {
        match self {
            Player::AlwaysCorrect => "always-correct",
            Player::AlwaysWrong => "always-wrong",
            Player::Random => "random",
            Player::LongAbsence { .. } => "long-absence",
            Player::ModelMatched { .. } => "model-matched",
            Player::Exponential { .. } => "exponential (off-model)",
        }
    }
}

/// What the player's true memory says, and what they answer.
struct Memory {
    true_s: f64,
    growth: f64,
    reset: f64,
    exponential: bool,
}

impl Memory {
    /// True probability of recall after `t` days.
    fn recall(&self, t: f64) -> f64 {
        if self.exponential {
            // 2^(-t/H), written as a root-free power for determinism.
            (-(t / self.true_s) * std::f64::consts::LN_2).exp()
        } else {
            retrievability(t, self.true_s)
        }
    }
    fn on_recall(&mut self) {
        self.true_s *= self.growth;
    }
    fn on_lapse(&mut self) {
        self.true_s = self.reset;
    }
}

// ----------------------------------------------------------------- the runs

#[derive(Default)]
struct Trace {
    /// One entry per FSRS-stage review: (true recall at that moment, correct).
    graded: Vec<(f64, bool)>,
    intervals: Vec<u32>,
    reviews: usize,
    lapses: usize,
    graduated: Option<usize>,
    max_interval: u32,
    /// Set if anything went non-finite or out of FSRS's own bounds.
    divergence: Option<String>,
    /// Set if the clock ever failed to advance.
    stall: Option<String>,
    /// For the long-absence run: what happened on the first review AFTER
    /// the player came back. That is what the absence actually tests.
    after_return: Option<ReturnOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ReturnOutcome {
    /// Came back in this many days.
    Due(u32),
    /// Left the queue outright on the strength of that one answer.
    Graduated,
}

fn check_state(r: &ReviewState, step: usize, t: &mut Trace) {
    let f = &r.fsrs;
    let bad = if !f.stability.is_finite() {
        Some(format!("stability is {} at step {step}", f.stability))
    } else if !f.difficulty.is_finite() {
        Some(format!("difficulty is {} at step {step}", f.difficulty))
    } else if f.stability <= 0.0 {
        Some(format!("stability fell to {} at step {step}", f.stability))
    } else if !(1.0..=10.0).contains(&f.difficulty) {
        Some(format!("difficulty left [1,10] at {} on step {step}", f.difficulty))
    } else {
        None
    };
    if bad.is_some() && t.divergence.is_none() {
        t.divergence = bad;
    }
}

fn run(player: Player, seed: u64) -> Trace {
    let mut rng = Rng::new(seed);
    let mut t = Trace::default();
    let (mut state, mut due) = start(0.0);
    let mut now = 0.0f64;

    let (mut mem, absence) = match player {
        Player::ModelMatched { s0, growth } => (
            Some(Memory { true_s: s0, growth, reset: s0, exponential: false }),
            None,
        ),
        Player::Exponential { half_life0, growth } => (
            Some(Memory { true_s: half_life0, growth, reset: half_life0, exponential: true }),
            None,
        ),
        Player::LongAbsence { gap_days, before } => (None, Some((gap_days, before))),
        _ => (None, None),
    };

    for step in 0..MAX_STEPS {
        // Advance to the due moment; never let the clock stand still.
        let next = due.max(now + ANSWER_GAP_MS);
        let elapsed_days = (next - now) / DAY_MS;
        if next <= now {
            t.stall = Some(format!("clock did not advance at step {step}"));
            break;
        }
        now = next;

        // The long-absence population vanishes after `before` reviews.
        let returning = matches!(absence, Some((_, before)) if step == before);
        if let Some((gap_days, before)) = absence {
            if step == before {
                now += gap_days as f64 * DAY_MS;
            }
        }

        let on_fsrs = state.step as usize >= 2; // LEARN_STEPS_MS.len()
        let correct = match (&mut mem, player) {
            (Some(m), _) => {
                let p = m.recall(elapsed_days.max(0.0));
                let hit = rng.next_f64() < p;
                if on_fsrs {
                    t.graded.push((p, hit));
                }
                if hit {
                    m.on_recall();
                } else {
                    m.on_lapse();
                }
                hit
            }
            (None, Player::AlwaysCorrect) => true,
            (None, Player::AlwaysWrong) => false,
            (None, Player::Random) => rng.next_f64() < 0.5,
            (None, Player::LongAbsence { .. }) => true,
            (None, p) => unreachable!("{p:?} has no answer rule"),
        };

        t.reviews += 1;
        if correct {
            match on_correct(&mut state, Grade::Good, now) {
                Outcome::Due(d) => {
                    let iv = ((d - now) / DAY_MS).round().max(0.0) as u32;
                    if returning {
                        t.after_return = Some(ReturnOutcome::Due(iv));
                    }
                    t.intervals.push(iv);
                    t.max_interval = t.max_interval.max(iv);
                    due = d;
                }
                Outcome::Graduated => {
                    if returning {
                        t.after_return = Some(ReturnOutcome::Graduated);
                    }
                    t.graduated = Some(step);
                    break;
                }
            }
        } else {
            t.lapses += 1;
            due = on_miss(&mut state, now);
        }
        check_state(&state, step, &mut t);
    }
    t
}

// ------------------------------------------------------------ the invariants

/// R3's three named invariants, on the four adversarial populations. These
/// are asserted, not reported: none of them needs a tolerance to be wrong.
#[test]
fn the_four_adversarial_populations_do_not_break_the_scheduler() {
    for player in adversarial() {
        for seed in [1u64, 7, 99] {
            let t = run(player, seed);
            let who = format!("{} seed {seed}", player.name());

            // No divergence.
            assert!(t.divergence.is_none(), "{who}: {}", t.divergence.unwrap());
            // No unbounded intervals.
            assert!(
                t.max_interval <= SANE_INTERVAL_DAYS,
                "{who}: scheduled {} days out",
                t.max_interval
            );
            // No starved queue: the player got reviews, and the clock moved.
            assert!(t.reviews > 0, "{who}: the queue served nothing");
            assert!(t.stall.is_none(), "{who}: {}", t.stall.unwrap());
        }
    }
}

/// A player who never gets it right must never be told they have mastered it,
/// and must keep being offered the word. This is the starved-queue invariant
/// from the other side: failure must not empty the queue either.
#[test]
fn always_wrong_never_graduates_and_keeps_being_served() {
    let t = run(Player::AlwaysWrong, 1);
    assert!(t.graduated.is_none(), "an always-wrong player graduated a word");
    assert_eq!(t.reviews, MAX_STEPS, "the queue stopped serving a failing player");
    assert_eq!(t.lapses, MAX_STEPS, "a wrong answer was not recorded as a lapse");
}

/// And a player who is always right must leave, or the queue grows forever.
#[test]
fn always_correct_graduates() {
    let t = run(Player::AlwaysCorrect, 1);
    let at = t.graduated.expect("an always-correct player never graduated");
    assert!(at < 20, "graduation took {at} reviews, which is not 'about five'");
}

/// Coming back after months must not produce a nonsense schedule.
#[test]
fn a_long_absence_does_not_explode_the_interval() {
    let t = run(Player::LongAbsence { gap_days: 90, before: 3 }, 1);
    assert!(t.divergence.is_none(), "{:?}", t.divergence);
    assert!(
        t.max_interval <= SANE_INTERVAL_DAYS,
        "a 90-day absence scheduled {} days out",
        t.max_interval
    );
}

/// PINNED BEHAVIOUR, NOT AN ENDORSEMENT. A 90-day absence followed by ONE
/// correct answer graduates the word — the long gap drives retrievability
/// down, and the stability increment scales with e^(w10*(1-R))-1, so recall
/// against the odds reads as strong evidence. Graduation also fires
/// redemption and the journal's "mastered", so this is Eric's call rather
/// than something to adjust quietly. Pinned so the answer cannot change
/// without someone saying so.
#[test]
fn a_long_absence_graduates_a_word_on_one_answer() {
    let t = run(Player::LongAbsence { gap_days: 90, before: 3 }, 1);
    assert_eq!(
        t.after_return,
        Some(ReturnOutcome::Graduated),
        "the absence no longer graduates on one answer -- update reports/simulator.md"
    );
}

/// Kept so the report still reads correctly if the behaviour changes and the
/// returning review starts producing an interval instead of a graduation.
fn return_due(d: u32) -> &'static str {
    Box::leak(
        format!(
            "After the gap the next interval was **{d} days**, and the word stayed in \
             the queue. FSRS reconstructs elapsed time as `due_day - \
             interval_days(stability)`, so the absence does register as a long gap."
        )
        .into_boxed_str(),
    )
}

fn adversarial() -> Vec<Player> {
    vec![
        Player::AlwaysCorrect,
        Player::AlwaysWrong,
        Player::Random,
        Player::LongAbsence { gap_days: 90, before: 3 },
    ]
}

fn retention_populations() -> Vec<Player> {
    vec![
        Player::ModelMatched { s0: 1.0, growth: 1.6 },
        Player::ModelMatched { s0: 3.0, growth: 1.6 },
        Player::ModelMatched { s0: 0.5, growth: 2.2 },
        Player::Exponential { half_life0: 1.0, growth: 1.6 },
        Player::Exponential { half_life0: 3.0, growth: 1.6 },
    ]
}

/// Many seeds per population, so the reported retention is a rate and not an
/// anecdote. 200 synthetic children each.
const COHORT: u64 = 200;

fn achieved_retention(player: Player) -> (f64, f64, usize) {
    let (mut hits, mut n, mut expected) = (0usize, 0usize, 0.0f64);
    for seed in 1..=COHORT {
        for (p, correct) in run(player, seed * 2_654_435_761).graded {
            expected += p;
            n += 1;
            if correct {
                hits += 1;
            }
        }
    }
    if n == 0 {
        return (f64::NAN, f64::NAN, 0);
    }
    (hits as f64 / n as f64, expected / n as f64, n)
}

// ---------------------------------------------------------------- the report

#[test]
fn writes_the_simulator_report() {
    let mut md = String::new();
    md.push_str(REPORT_HEAD);

    md.push_str("\n## 1. The four adversarial populations\n\n");
    md.push_str("Asserted, not reported. None of these needs a tolerance to be wrong.\n\n");
    md.push_str("| population | reviews | lapses | graduated | longest interval | diverged | stalled |\n");
    md.push_str("|---|---|---|---|---|---|---|\n");
    for player in adversarial() {
        let t = run(player, 1);
        let _ = writeln!(
            md,
            "| {} | {} | {} | {} | {} | {} | {} |",
            player.name(),
            t.reviews,
            t.lapses,
            t.graduated.map_or("no".into(), |s| format!("at review {}", s + 1)),
            if t.intervals.is_empty() {
                "— (never answered one right)".into()
            } else {
                format!("{} d", t.max_interval)
            },
            if t.divergence.is_none() { "no" } else { "YES" },
            if t.stall.is_none() { "no" } else { "YES" },
        );
    }
    let gap = run(Player::LongAbsence { gap_days: 90, before: 3 }, 1);
    let _ = writeln!(
        md,
        "\n### What the 90-day disappearance actually did\n\n{}",
        match gap.after_return {
            Some(ReturnOutcome::Graduated) =>
                "**The word graduated on the first answer after the gap.** A child who \
                 vanishes for three months, comes back and gets the word right ONCE is \
                 recorded as having mastered it, and the word leaves the queue.\n\n\
                 The mechanism is not a bug in the arithmetic. FSRS has no `last_review` \
                 field, so elapsed time is reconstructed as `due_day - \
                 interval_days(stability)`; the absence therefore registers as a long \
                 gap, retrievability falls close to zero, and the stability increment \
                 scales with `e^(w10*(1-R)) - 1` — so recall against the odds is read as \
                 strong evidence. For an adult grinding a review deck that is the \
                 intended reading. For a child's spelling game it means an absence \
                 LAUNDERS a word into 'mastered', and `GRADUATE_AFTER_DAYS` is only 7, \
                 so the bar is low.\n\n\
                 This is a question for Eric, not a defect to fix quietly: should a \
                 long absence be able to graduate a word on one answer, given \
                 graduation also fires redemption and the journal's 'mastered'? The \
                 same `due_day`-has-no-companion-field problem is what makes the CC-SNAP \
                 3.2 test-date work unsafe, so the two decisions share a root.",
            Some(ReturnOutcome::Due(d)) => return_due(d),
            None => "No review followed the gap in this run.",
        }
    );

    md.push_str(RETENTION_HEAD);
    md.push_str("| population | reviews scored | player's true recall | observed | harness | **vs the 0.90 aim** |\n");
    md.push_str("|---|---|---|---|---|---|\n");
    for player in retention_populations() {
        let (got, want, n) = achieved_retention(player);
        let label = match player {
            Player::ModelMatched { s0, growth } => format!("{} S0={s0} g={growth}", player.name()),
            Player::Exponential { half_life0, growth } => {
                format!("{} H0={half_life0} g={growth}", player.name())
            }
            _ => player.name().into(),
        };
        let _ = writeln!(
            md,
            "| {label} | {n} | {:.3} | {:.3} | {:+.3} | **{:+.3}** |",
            want,
            got,
            got - want,
            got - TARGET_RETENTION
        );
    }
    let _ = writeln!(
        md,
        "\n**Read the last two columns differently.** \"harness\" is observed \
         minus the player's own true recall: it should be ~0, and if it is not \
         the simulation is broken rather than the scheduler. The result is the \
         final column — observed minus the **{TARGET_RETENTION:.2}** the \
         scheduler aims at (`RETENTION` in `src/learner.rs`)."
    );

    md.push_str(FINDINGS);
    md.push_str(LIMITATION);

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("reports/simulator.md");
    fs::create_dir_all(path.parent().unwrap()).expect("reports/ is writable");
    fs::write(&path, md).expect("could not write reports/simulator.md");
}

const REPORT_HEAD: &str = r#"# CC-LEARNING-ENGINE-L0 — R3 simulator report

<!-- GENERATED by tests/learner_simulator.rs. Do not hand-edit.
     Regenerate with: cargo test --test learner_simulator
     Seeded and deterministic, and carrying no timestamp, so this file is
     byte-identical run to run. A diff here means the SCHEDULER changed. -->

**Status: the first report. Tolerances are NOT frozen** — D5 says they are
frozen after Eric reads this, so §2 reports numbers and asserts nothing
against a pass mark. §1's invariants are asserted, because none of them
needs a tolerance to be wrong.

**Why a simulator at all.** `src/telemetry/schema.rs` bars every learner term
from telemetry — `learner`, `mastery`, `fsrs`, `bkt`, `skill`, `stability`,
`difficulty` — with a planted-field test enforcing it. That is deliberate
(I2, the zero-egress posture), and it means there is no field evidence about
the scheduler and there never will be. This is the only evaluation channel
the design permits.
"#;

const RETENTION_HEAD: &str = r#"
## 2. Do the intervals track a known forgetting curve?

Each population is a cohort of 200 synthetic children with a KNOWN memory.
At every FSRS-stage review the player's true recall probability is computed
from their own curve, and they answer correctly with exactly that
probability. If the scheduler is choosing its intervals well, the share they
actually get right at review time should land near the retention it aims at.

**model-matched** players forget along FSRS's own retrievability curve, so
they behave precisely as the model assumes. This is the scope R3 claims and
no more. **exponential** players forget along 2^(-t/H) instead — a different
curve shape, outside the model's assumptions, reported for information and
claimed for nothing.

"#;

const FINDINGS: &str = r#"
## 3. Findings that are not numbers

**A failing player's clock does not advance on its own.** `on_miss` returns
`now + LEARN_STEPS_MS[0]`, and that first step is zero, so a word answered
wrong is due again immediately. Within a session that is intended — the word
comes straight back. But `review.rs` has no backstop of its own, so whatever
bounds the retry loop lives outside it, in the session. The simulator makes
the player take 20 seconds per answer so the clock moves; without that it
spins. Worth knowing before anything else schedules through R2.

**Graduation is reachable only through FSRS, not through learning.** A word
leaves the queue when a success produces an interval over
`GRADUATE_AFTER_DAYS`, which is checked only on the FSRS branch of
`on_correct`. A word cannot graduate out of the two same-day learning steps,
which is correct, and is worth stating because it is not obvious from the
field name.
"#;

const LIMITATION: &str = r#"
## 4. The stated limitation

> The simulator proves the implementation behaves correctly for players who
> behave as the model assumes. It does **not** establish that the model
> describes real children.

Any grant claim must be worded to that limit. **That wording is D6 and it is
Eric's** — nothing in this report is drafted for external use.
"#;
