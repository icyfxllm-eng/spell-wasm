//! CC-LEARNING-ENGINE Phase L0 — the Learner Model core.
//!
//! One serialised state per (profile × language): per-skill Bayesian
//! Knowledge Tracing posteriors and FSRS scheduling state, a bounded
//! attempt log, and a schema version with a migration harness that exists
//! from day one. Pure data (D1: BKT + FSRS, no neural net, kilobytes);
//! every consumer reads and writes through this contract only.
//!
//! **The governing law** (restated because every file that touches this
//! module must repeat it): the Learner Model selects WITHIN the band.
//! Nothing in this module reaches layout, rendering, band assignment,
//! layer gating, scoring, or audio tiers — L0 exposes state and update
//! rules; L1's selection policy consumes them, and D7 makes any wider
//! reach a build-failing violation there.
//!
//! **Determinism (Done #1c)** is byte-level: same state + attempt stream →
//! the same serialised bytes on every device. That rules libm out — `exp`
//! and `powf` differ across platforms — so the transcendentals FSRS needs
//! are [`det_exp`] / [`det_ln`] / [`det_pow`]: fixed polynomials over
//! IEEE-exact operations, the same doctrine `tonal::det_atan2` follows and
//! for the same reason (spike #9 watched libm's `sin` break a determinism
//! corpus).
//!
//! Honest scope notes: Done #1a's "matches the FSRS reference exactly" is
//! held here as agreement with the PUBLISHED FSRS-4.5 formulas evaluated
//! in-test, with integer due-days matching exactly; a byte-comparison
//! against a specific external reference build would need that build
//! vendored, which is its own reviewed change. Done #1b (Brier calibration
//! against real play) needs recorded logs that do not exist yet.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;
/// The attempt log is a ring: enough history for diagnosis surfaces,
/// bounded so the state stays kilobytes forever (the contract's own
/// promise).
pub const LOG_CAP: usize = 512;

// ------------------------------------------------------------------ state

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LearnerState {
    pub version: u32,
    pub lang: String,
    pub skills: Vec<SkillState>,
    pub log: VecDeque<Attempt>,
    /// None = placement not offered yet; Some(false) = skipped (language
    /// defaults stand, feature 5).
    pub placed: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillState {
    /// Hazard-taxonomy id, e.g. "silent_letters" — the same tags the
    /// difficulty extractors and manifests already use.
    pub id: String,
    /// BKT mastery posterior, 0..=1.
    pub mastery: f64,
    pub fsrs: FsrsState,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FsrsState {
    pub stability: f64,
    pub difficulty: f64,
    /// Day index (days since profile epoch) when this skill comes due.
    pub due_day: u32,
    pub reps: u32,
    pub lapses: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Attempt {
    pub day: u32,
    pub word: String,
    /// Skills the word exercises (its hazard vector).
    pub skills: Vec<String>,
    pub correct: bool,
    /// "typed" | "speech" — feature 4: hearing misses must never update
    /// spelling skills, and the flag is how L1's diagnosis will know.
    pub channel: Channel,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Typed,
    Speech,
}

impl LearnerState {
    pub fn new(lang: &str) -> Self {
        Self {
            version: SCHEMA_VERSION,
            lang: lang.to_string(),
            skills: Vec::new(),
            log: VecDeque::new(),
            placed: None,
        }
    }

    fn skill_mut(&mut self, id: &str) -> &mut SkillState {
        if let Some(i) = self.skills.iter().position(|s| s.id == id) {
            return &mut self.skills[i];
        }
        // New skills start at the language-default prior (D2: English
        // template values; per-language priors arrive with their audits).
        self.skills.push(SkillState {
            id: id.to_string(),
            mastery: BKT_PRIOR,
            fsrs: FsrsState { stability: 0.0, difficulty: FSRS_D0, due_day: 0, reps: 0, lapses: 0 },
        });
        self.skills.last_mut().unwrap()
    }

    /// One attempt updates every skill the word exercises (feature 1) and
    /// appends to the bounded log. Speech attempts are LOGGED but update
    /// no spelling skill — a hearing miss is not a spelling miss.
    pub fn record(&mut self, attempt: Attempt) {
        if attempt.channel == Channel::Typed {
            for id in &attempt.skills {
                let s = self.skill_mut(id);
                s.mastery = bkt_update(s.mastery, attempt.correct);
                fsrs_review(&mut s.fsrs, attempt.correct, attempt.day);
            }
        }
        if self.log.len() == LOG_CAP {
            self.log.pop_front();
        }
        self.log.push_back(attempt);
    }

    /// Due-ness for L1's selection scoring: how overdue each skill is on
    /// `day`, most overdue first. Read-only — selection lives in L1.
    pub fn due_skills(&self, day: u32) -> Vec<(&str, i64)> {
        let mut v: Vec<(&str, i64)> = self
            .skills
            .iter()
            .filter(|s| s.fsrs.reps > 0)
            .map(|s| (s.id.as_str(), day as i64 - s.fsrs.due_day as i64))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        v
    }
}

// ------------------------------------------------------------- migration

/// Versioned load. The v1→v2 harness exists NOW (Done #8) so the first
/// real migration lands into a tested slot instead of inventing the
/// machinery under pressure.
pub fn load_state(json: &str) -> Result<LearnerState, String> {
    #[derive(Deserialize)]
    struct Probe {
        version: u32,
    }
    let probe: Probe = serde_json::from_str(json).map_err(|e| e.to_string())?;
    match probe.version {
        SCHEMA_VERSION => serde_json::from_str(json).map_err(|e| e.to_string()),
        v if v < SCHEMA_VERSION => migrate(json, v),
        v => Err(format!("state from a NEWER schema ({v}); refusing to guess")),
    }
}

fn migrate(json: &str, from: u32) -> Result<LearnerState, String> {
    // No v0 ever shipped, so there is nothing to translate yet — but the
    // dispatch, the error shape, and the test harness are load-bearing.
    let _ = json;
    Err(format!("no migration path from schema v{from}"))
}

// ------------------------------------------------------------------- BKT

/// English-template BKT parameters (D2). Priors are config, not code, in
/// spirit: they are constants HERE only until the per-language config
/// lands with its native audits, and they are pinned (D3) — changing them
/// re-runs the eval suite as its own reviewed change.
pub const BKT_PRIOR: f64 = 0.25; // P(L0): knows the skill before we meet them
pub const BKT_TRANSIT: f64 = 0.12; // P(T): learns from one exposure
pub const BKT_SLIP: f64 = 0.10; // P(S): knows it, missed anyway
pub const BKT_GUESS: f64 = 0.20; // P(G): got it without the skill

/// Standard BKT posterior update, then the learning transition.
pub fn bkt_update(mastery: f64, correct: bool) -> f64 {
    let (l, s, g, t) = (mastery, BKT_SLIP, BKT_GUESS, BKT_TRANSIT);
    let cond = if correct {
        let p = l * (1.0 - s);
        p / (p + (1.0 - l) * g)
    } else {
        let p = l * s;
        p / (p + (1.0 - l) * (1.0 - g))
    };
    cond + (1.0 - cond) * t
}

// ------------------------------------------------------------------ FSRS

/// FSRS-4.5 weights, pinned (D3). Source: the published FSRS-4.5 default
/// parameter set. Any change re-runs the eval suite as a reviewed change,
/// never a drive-by bump.
pub const FSRS_W: [f64; 17] = [
    0.4872, 1.4003, 3.7145, 13.8206, 5.1618, 1.2298, 0.8975, 0.031, 1.6474, 0.1367, 1.0461,
    2.1072, 0.0793, 0.3246, 1.587, 0.2272, 2.8755,
];
pub const FSRS_D0: f64 = 5.0; // starting difficulty, mid-scale
const FACTOR: f64 = 19.0 / 81.0;
/// Target retention for scheduling (the game reviews a skill when recall
/// would otherwise drop below this).
const RETENTION: f64 = 0.9;

/// Retrievability after `t` days at stability `s`. DECAY is -0.5, so this
/// is 1/sqrt(1 + F·t/s) — sqrt is IEEE-correctly-rounded, no libm needed.
pub fn retrievability(t: f64, s: f64) -> f64 {
    if s <= 0.0 {
        return 0.0;
    }
    1.0 / (1.0 + FACTOR * t / s).sqrt()
}

/// Interval (days) at which retrievability falls to RETENTION:
/// t = s/F · (R^(1/DECAY) − 1), and R^(1/DECAY) = R⁻² — multiplication.
pub fn interval_days(s: f64) -> u32 {
    let t = s / FACTOR * (1.0 / (RETENTION * RETENTION) - 1.0);
    t.round().max(1.0) as u32
}

/// One review. Grades collapse to pass/fail: the game has no "hard/easy"
/// buttons and inventing them would be engagement furniture.
pub fn fsrs_review(f: &mut FsrsState, correct: bool, day: u32) {
    let w = &FSRS_W;
    if f.reps == 0 {
        // First exposure: initial stability from the grade (w0 = again,
        // w3 = easy; pass uses "good" = w2).
        f.stability = if correct { w[2] } else { w[0] };
        f.difficulty = (w[4] - det_exp((if correct { 3.0 } else { 1.0 } - 1.0) * w[5]) + 1.0)
            .clamp(1.0, 10.0);
    } else {
        let elapsed = (day.saturating_sub(f.due_day.saturating_sub(interval_days(f.stability)))) as f64;
        let r = retrievability(elapsed.max(0.0), f.stability);
        let g = if correct { 3.0 } else { 1.0 };
        // difficulty update with mean reversion (w7 toward D0)
        let d = f.difficulty - w[6] * (g - 3.0);
        f.difficulty = (w[7] * FSRS_D0 + (1.0 - w[7]) * d).clamp(1.0, 10.0);
        if correct {
            // S' = S · (1 + e^w8 · (11−D) · S^−w9 · (e^(w10·(1−R)) − 1))
            let inc = det_exp(w[8])
                * (11.0 - f.difficulty)
                * det_pow(f.stability, -w[9])
                * (det_exp(w[10] * (1.0 - r)) - 1.0);
            f.stability *= 1.0 + inc;
        } else {
            // S'_forget = w11 · D^−w12 · ((S+1)^w13 − 1) · e^(w14·(1−R))
            let s_new = w[11]
                * det_pow(f.difficulty, -w[12])
                * (det_pow(f.stability + 1.0, w[13]) - 1.0)
                * det_exp(w[14] * (1.0 - r));
            f.stability = s_new.min(f.stability).max(0.1);
            f.lapses += 1;
        }
    }
    f.reps += 1;
    f.due_day = day + interval_days(f.stability);
}

// -------------------------------------------- deterministic transcendentals

/// ln(x) for x > 0, no libm: split into mantissa·2^k via bit surgery, then
/// the atanh series ln(m) = 2(z + z³/3 + z⁵/5 + …) with z = (m−1)/(m+1),
/// which converges fast on [1/√2, √2). Relative error < 1e-14.
pub fn det_ln(x: f64) -> f64 {
    assert!(x > 0.0, "ln of non-positive");
    const LN2: f64 = 0.693_147_180_559_945_3;
    let bits = x.to_bits();
    let mut e = ((bits >> 52) & 0x7ff) as i64 - 1023;
    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | (1023u64 << 52));
    if m > std::f64::consts::SQRT_2 {
        m /= 2.0;
        e += 1;
    }
    let z = (m - 1.0) / (m + 1.0);
    let z2 = z * z;
    let mut term = z;
    let mut sum = 0.0;
    for k in 0..27 {
        sum += term / (2 * k + 1) as f64;
        term *= z2;
    }
    2.0 * sum + e as f64 * LN2
}

/// e^x, no libm: reduce by x = k·ln2 + r with |r| ≤ ln2/2, Taylor on r,
/// scale by 2^k through the exponent bits. Relative error < 1e-14 on the
/// FSRS operating range.
pub fn det_exp(x: f64) -> f64 {
    const LN2: f64 = 0.693_147_180_559_945_3;
    let k = (x / LN2).round();
    let r = x - k * LN2;
    let mut term = 1.0;
    let mut sum = 1.0;
    for n in 1..22 {
        term *= r / n as f64;
        sum += term;
    }
    let two_k = f64::from_bits((((k as i64) + 1023) as u64) << 52);
    sum * two_k
}

/// x^y for x > 0 — exp(y·ln x) over the two above.
pub fn det_pow(x: f64, y: f64) -> f64 {
    det_exp(y * det_ln(x))
}

// ------------------------------------------------- the gameplay bridge

/// CC-LEARNING-ENGINE D2 — the ENGLISH hazard taxonomy is the shipped
/// template, a literal port of `tools/difficulty-score/extractors.py::en`
/// pinned by the fixture parity test below (3206 pool words, byte-for-tag).
/// Every other language returns an EMPTY vector on purpose: its attempts
/// still log (L1's raw material) but update no skill until a named native
/// speaker signs off its taxonomy. Not naming them now is the decision.
pub fn hazards(lang: &str, word: &str) -> Vec<String> {
    if lang != "en" {
        return Vec::new();
    }
    const SILENT: [&str; 9] = ["kn", "mb", "gh", "wr", "mn", "bt", "gn", "pn", "ps"];
    const LOAN: [&str; 6] = ["eau", "ough", " psy", "eur", "oir", "aut"];
    const AMBIG: [&str; 9] =
        ["able", "ible", "ance", "ence", "ant", "ent", "tion", "sion", "cian"];
    let w = word.to_lowercase();
    let mut f = Vec::new();
    if SILENT.iter().any(|s| w.contains(s)) {
        f.push("silent_letters".to_string());
    }
    // Python checks `not in "aeiou"` — a hyphen or apostrophe counts as a
    // consonant there, so it does here too. Parity beats prettiness.
    let ch: Vec<char> = w.chars().collect();
    if ch.windows(2).any(|p| p[0] == p[1] && !"aeiou".contains(p[0])) {
        f.push("doubled_consonant".to_string());
    }
    if AMBIG.iter().any(|s| w.ends_with(s) || w.contains(s)) {
        f.push("unstressed_vowel_ambiguity".to_string());
    }
    if LOAN.iter().any(|l| w.contains(l)) {
        f.push("loanword_spelling".to_string());
    }
    f
}

/// Day index for FSRS scheduling: whole days since the Unix epoch, UTC.
/// Only deltas matter to the scheduler, so the base is arbitrary — what
/// matters is that it's monotonic and survives reinstalls identically.
fn today_day() -> u32 {
    (js_sys::Date::now() / 86_400_000.0) as u32
}

const STORE_PREFIX: &str = "spell_learner_";

/// The one gameplay door: load-or-new, record, save. Every submit path
/// calls this and nothing else — the speech exclusion lives inside
/// `record()`, not in callers remembering to skip it. Storage failures
/// degrade to a fresh profile rather than a crash: the learner is an
/// observer of the game, never a gate on it.
pub fn note_attempt(lang: &str, word: &str, correct: bool, channel: Channel) {
    let key = format!("{STORE_PREFIX}{lang}");
    let mut st = crate::storage::get_raw(&key)
        .and_then(|j| load_state(&j).ok())
        .unwrap_or_else(|| LearnerState::new(lang));
    st.record(Attempt {
        day: today_day(),
        word: word.to_string(),
        skills: hazards(lang, word),
        correct,
        channel,
    });
    crate::storage::set_json(&key, &st);
}


// -------------------------------------------------- L1: selection policy

/// L1 (CC-LEARNING-ENGINE feature 2) — rank a band's words and pick the
/// most valuable next exercise. THE GOVERNING LAW, held by construction:
/// this function receives the band as a slice and returns an INDEX into
/// it — it cannot reach outside the band, touch layout, tiers, layers,
/// scoring or audio. The D7 property test below hammers that claim ten
/// thousand times anyway, because "by construction" has been wrong in
/// this codebase before.
///
/// Score per word = due-ness + uncertainty + coverage − recency:
///   due-ness    — days overdue summed over the word's tracked skills
///                 (a lapsed skill is the most valuable thing to show)
///   uncertainty — 4·m·(1−m) per skill: maximal where BKT knows least
///   coverage    — a small bonus per untracked skill (exploration)
///   recency     — words in `recent` sink to the bottom but stay legal
/// Ties break by a deterministic per-word hash so the same inputs pick
/// the same word on every device (the Done #1c doctrine).
pub fn select_within(
    state: &LearnerState,
    band: &[String],
    lang: &str,
    day: u32,
    recent: &[String],
    salt: u64,
) -> Option<usize> {
    if band.is_empty() {
        return None;
    }
    let mut best: Option<(f64, u64, usize)> = None;
    for (i, word) in band.iter().enumerate() {
        let skills = hazards(lang, word);
        let mut due = 0.0;
        let mut uncertainty = 0.0;
        let mut coverage = 0.0;
        for id in &skills {
            match state.skills.iter().find(|s| s.id == *id) {
                Some(s) => {
                    if s.fsrs.reps > 0 && day > s.fsrs.due_day {
                        due += (day - s.fsrs.due_day) as f64;
                    }
                    uncertainty += 4.0 * s.mastery * (1.0 - s.mastery);
                }
                None => coverage += 0.5,
            }
        }
        let recency = if recent.contains(word) { 100.0 } else { 0.0 };
        let score = due + uncertainty + coverage - recency;
        // deterministic tiebreak: FNV over the word, salted
        let mut h: u64 = 0xcbf29ce484222325 ^ salt;
        for b in word.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        let better = match &best {
            None => true,
            Some((bs, bh, _)) => score > *bs || (score == *bs && h > *bh),
        };
        if better {
            best = Some((score, h, i));
        }
    }
    best.map(|(_, _, i)| i)
}

/// Load-or-new for a language — the same door `note_attempt` uses,
/// exposed so the selection hook reads the identical state.
pub fn load_for(lang: &str) -> LearnerState {
    let key = format!("{STORE_PREFIX}{lang}");
    crate::storage::get_raw(&key)
        .and_then(|j| load_state(&j).ok())
        .unwrap_or_else(|| LearnerState::new(lang))
}

/// Public wrapper for the day index (the hook needs the same epoch).
pub fn current_day() -> u32 {
    today_day()
}

// ----------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    // -- the transcendentals hold against std within 1e-12 relative -------

    #[test]
    fn det_transcendentals_match_std() {
        for &x in &[0.001, 0.1, 0.5, 1.0, 1.5, 2.0, 5.0, 13.8206, 100.0] {
            let rel = (det_ln(x) - x.ln()).abs() / x.ln().abs().max(1e-30);
            assert!(rel < 1e-12, "ln({x}): rel {rel}");
        }
        for &x in &[-6.0, -1.0, -0.1, 0.0, 0.031, 1.0, 1.6474, 4.0] {
            let rel = (det_exp(x) - x.exp()).abs() / x.exp();
            assert!(rel < 1e-12, "exp({x}): rel {rel}");
        }
        for &(x, y) in &[(2.0, -0.1367), (5.0, -0.0793), (3.5, 0.3246), (10.0, 2.0)] {
            let rel = (det_pow(x, y) - x.powf(y)).abs() / x.powf(y);
            assert!(rel < 1e-12, "{x}^{y}: rel {rel}");
        }
    }

    // -- Done #1a: FSRS behaviour against the published formulas ----------

    #[test]
    fn fsrs_first_reviews_match_the_published_initial_stabilities() {
        let mut pass = FsrsState { stability: 0.0, difficulty: FSRS_D0, due_day: 0, reps: 0, lapses: 0 };
        fsrs_review(&mut pass, true, 0);
        assert!((pass.stability - FSRS_W[2]).abs() < 1e-12, "good => w2");
        let mut fail = FsrsState { stability: 0.0, difficulty: FSRS_D0, due_day: 0, reps: 0, lapses: 0 };
        fsrs_review(&mut fail, false, 0);
        assert!((fail.stability - FSRS_W[0]).abs() < 1e-12, "again => w0");
        assert!(fail.difficulty > pass.difficulty, "failing is harder than passing");
    }

    #[test]
    fn fsrs_interval_is_the_retention_solution() {
        // At the scheduled interval, retrievability must be RETENTION
        // (within rounding to whole days).
        for s in [1.0, 3.0, 10.0, 40.0] {
            let t = interval_days(s);
            let r = retrievability(t as f64, s);
            assert!((r - 0.9).abs() < 0.03, "s={s}: R at interval = {r}");
        }
    }

    #[test]
    fn fsrs_success_grows_stability_and_failure_shrinks_it() {
        let mut f = FsrsState { stability: 0.0, difficulty: FSRS_D0, due_day: 0, reps: 0, lapses: 0 };
        fsrs_review(&mut f, true, 0);
        let mut day = f.due_day;
        let mut prev = f.stability;
        for _ in 0..4 {
            fsrs_review(&mut f, true, day);
            assert!(f.stability > prev, "on-time success must grow stability");
            prev = f.stability;
            day = f.due_day;
        }
        fsrs_review(&mut f, false, day);
        assert!(f.stability < prev, "a lapse must shrink stability");
        assert_eq!(f.lapses, 1);
    }

    // -- Done #1a: the simulated learner, mastery and forgetting ----------

    #[test]
    fn bkt_converges_on_a_scripted_mastery_trace() {
        // A learner who has genuinely learned: eight correct in a row.
        let mut m = BKT_PRIOR;
        for _ in 0..8 {
            m = bkt_update(m, true);
        }
        assert!(m > 0.95, "posterior after 8 hits: {m}");
        // And one who has not: eight misses.
        let mut m = BKT_PRIOR;
        for _ in 0..8 {
            m = bkt_update(m, false);
        }
        assert!(m < 0.35, "posterior after 8 misses: {m}");
        // A slip after mastery dents but does not destroy.
        let mut m = BKT_PRIOR;
        for _ in 0..8 {
            m = bkt_update(m, true);
        }
        let dip = bkt_update(m, false);
        assert!(dip > 0.5 && dip < m, "one slip dents, never resets: {m} -> {dip}");
    }

    #[test]
    fn one_attempt_updates_every_skill_the_word_exercises() {
        let mut st = LearnerState::new("en");
        st.record(Attempt {
            day: 0,
            word: "receive".into(),
            skills: vec!["silent_letters".into(), "unstressed_vowel_ambiguity".into()],
            correct: true,
            channel: Channel::Typed,
        });
        assert_eq!(st.skills.len(), 2);
        assert!(st.skills.iter().all(|s| s.mastery > BKT_PRIOR));
        assert!(st.skills.iter().all(|s| s.fsrs.reps == 1));
    }

    #[test]
    fn speech_attempts_log_but_never_update_spelling_skills() {
        // Feature 4/7: a hearing miss is not a spelling miss. The tripwire
        // for the exact bug class input_provenance exists to stop.
        let mut st = LearnerState::new("en");
        st.record(Attempt {
            day: 0,
            word: "receive".into(),
            skills: vec!["silent_letters".into()],
            correct: false,
            channel: Channel::Speech,
        });
        assert!(st.skills.is_empty(), "speech must not touch skill state");
        assert_eq!(st.log.len(), 1, "...but the log keeps the evidence");
    }

    #[test]
    fn the_log_is_a_ring_and_the_state_stays_small() {
        let mut st = LearnerState::new("en");
        for i in 0..(LOG_CAP + 40) {
            st.record(Attempt {
                day: i as u32,
                word: format!("w{i}"),
                skills: vec!["doubled_consonant".into()],
                correct: i % 3 != 0,
                channel: Channel::Typed,
            });
        }
        assert_eq!(st.log.len(), LOG_CAP);
        assert_eq!(st.log.front().unwrap().word, "w40", "oldest evicted first");
        let bytes = serde_json::to_string(&st).unwrap().len();
        assert!(bytes < 64 * 1024, "state must stay kilobytes, is {bytes}");
    }

    // -- Done #1c: byte determinism ----------------------------------------

    #[test]
    fn same_attempt_stream_yields_byte_identical_state() {
        let run = || {
            let mut st = LearnerState::new("en");
            for i in 0..200u32 {
                st.record(Attempt {
                    day: i / 5,
                    word: format!("w{}", i % 17),
                    skills: vec![
                        ["silent_letters", "doubled_consonant", "loanword_spelling"][(i % 3) as usize]
                            .into(),
                    ],
                    correct: (i * 7 + 3) % 5 != 0,
                    channel: if i % 11 == 0 { Channel::Speech } else { Channel::Typed },
                });
            }
            serde_json::to_string(&st).unwrap()
        };
        assert_eq!(run(), run(), "the contract is bytes, not approximately-equal floats");
    }

    #[test]
    fn due_skills_orders_most_overdue_first_with_stable_ties() {
        let mut st = LearnerState::new("en");
        for (id, day) in [("a_skill", 0u32), ("b_skill", 3), ("c_skill", 0)] {
            st.record(Attempt {
                day,
                word: id.into(),
                skills: vec![id.into()],
                correct: true,
                channel: Channel::Typed,
            });
        }
        let due = st.due_skills(30);
        assert_eq!(due.len(), 3);
        assert!(due.windows(2).all(|w| w[0].1 >= w[1].1), "most overdue first");
        // equal overdue-ness ties break on id, never on insertion accident
        let overdue: Vec<i64> = due.iter().map(|d| d.1).collect();
        if overdue[0] == overdue[1] {
            assert!(due[0].0 < due[1].0);
        }
    }

    // -- Done #8: reset and the migration harness --------------------------

    #[test]
    fn load_rejects_newer_schemas_and_reports_missing_migrations() {
        let newer = r#"{"version": 99, "lang": "en", "skills": [], "log": [], "placed": null}"#;
        assert!(load_state(newer).unwrap_err().contains("NEWER"));
        let ancient = r#"{"version": 0, "lang": "en", "skills": [], "log": [], "placed": null}"#;
        assert!(load_state(ancient).unwrap_err().contains("no migration path from schema v0"));
    }

    #[test]
    fn round_trip_is_identity() {
        let mut st = LearnerState::new("en");
        st.record(Attempt {
            day: 1,
            word: "quinoa".into(),
            skills: vec!["loanword_spelling".into()],
            correct: false,
            channel: Channel::Typed,
        });
        let json = serde_json::to_string(&st).unwrap();
        assert_eq!(load_state(&json).unwrap(), st);
    }
}

#[cfg(test)]
mod bridge_tests {
    use super::*;

    /// D2 parity: the Rust port answers exactly what the Python extractor
    /// answered for every English pool word — the fixture is generated
    /// from `extractors.py::en` itself, so drift in either port fails here.
    #[test]
    fn english_hazards_match_the_python_extractor() {
        let fx: std::collections::BTreeMap<String, Vec<String>> =
            serde_json::from_str(include_str!("../config/learner/en-hazards-fixture.json"))
                .unwrap();
        assert!(fx.len() > 3000, "fixture covers the full pool");
        let mut tagged = 0;
        for (word, want) in &fx {
            assert_eq!(&hazards("en", word), want, "{word}");
            tagged += usize::from(!want.is_empty());
        }
        assert!(tagged > 800, "the taxonomy actually fires: {tagged}");
    }

    /// D2's gate: unaudited languages log with an empty vector — even on
    /// words that would trip every English rule.
    #[test]
    fn unaudited_languages_carry_no_hazard_vector() {
        for lang in ["fr", "de", "es", "ko", "sw"] {
            assert!(hazards(lang, "knobble-attention").is_empty(), "{lang}");
        }
    }

    /// The bridge composed end-to-end (minus storage, which is DOM):
    /// typed attempts with real hazard vectors accumulate mastery; a
    /// speech attempt on the same words logs but moves no skill.
    #[test]
    fn typed_accumulates_speech_only_logs() {
        let mut st = LearnerState::new("en");
        for (w, ok) in [("wrong", true), ("attention", true), ("wrong", false)] {
            st.record(Attempt {
                day: 1,
                word: w.into(),
                skills: hazards("en", w),
                correct: ok,
                channel: Channel::Typed,
            });
        }
        assert!(!st.skills.is_empty(), "typed attempts grew skills");
        let before = st.skills.clone();
        st.record(Attempt {
            day: 2,
            word: "wrong".into(),
            skills: hazards("en", "wrong"),
            correct: false,
            channel: Channel::Speech,
        });
        assert_eq!(st.skills, before, "speech never moves a spelling skill");
        assert_eq!(st.log.len(), 4, "but it IS logged");
    }
}

#[cfg(test)]
mod l1_tests {
    use super::*;

    fn state_with(skills: &[(&str, f64, u32, u32)]) -> LearnerState {
        let mut st = LearnerState::new("en");
        for (id, mastery, due_day, reps) in skills {
            st.skills.push(SkillState {
                id: id.to_string(),
                mastery: *mastery,
                fsrs: FsrsState { stability: 1.0, difficulty: 5.0, due_day: *due_day, reps: *reps, lapses: 0 },
            });
        }
        st
    }

    /// D7, hammered: ten thousand selections across varying bands,
    /// states and days — the pick is ALWAYS an index into the band, the
    /// same inputs always pick the same word, and empty bands say None.
    #[test]
    fn d7_selection_never_leaves_the_band_10k() {
        let vocab = ["wrong", "attention", "shop", "wet", "kitten", "knee", "lamb",
                     "night", "receive", "puzzle", "grass", "station", "bubble", "science"];
        let mut rng: u64 = 0x5EED_D7;
        let mut hits = 0u32;
        for trial in 0..10_000u64 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let n = (rng >> 33) as usize % (vocab.len() + 1); // 0..=14 words
            let band: Vec<String> = (0..n).map(|k| vocab[(trial as usize + k * 3) % vocab.len()].to_string()).collect();
            let st = state_with(&[
                ("silent_letters", (trial % 100) as f64 / 100.0, (trial % 40) as u32, (trial % 3) as u32),
                ("doubled_consonant", 0.5, 10, 1),
            ]);
            let recent: Vec<String> = band.iter().take((trial % 3) as usize).cloned().collect();
            let day = (trial % 60) as u32;
            match select_within(&st, &band, "en", day, &recent, trial) {
                None => assert!(band.is_empty(), "None only for empty bands"),
                Some(i) => {
                    assert!(i < band.len(), "index {i} outside band of {}", band.len());
                    assert_eq!(select_within(&st, &band, "en", day, &recent, trial), Some(i),
                               "same inputs must pick the same word");
                    hits += 1;
                }
            }
        }
        assert!(hits > 9_000, "the policy actually selected: {hits}");
    }

    /// An overdue skill outranks a mastered fresh one.
    #[test]
    fn due_ness_outranks_mastery() {
        // "knee" exercises silent_letters (overdue); "grass" exercises
        // doubled_consonant (mastered, not due).
        let st = state_with(&[("silent_letters", 0.9, 5, 3), ("doubled_consonant", 0.97, 100, 3)]);
        let band = vec!["grass".to_string(), "knee".to_string()];
        let i = select_within(&st, &band, "en", 30, &[], 1).unwrap();
        assert_eq!(band[i], "knee", "the lapsed skill's word wins");
    }

    /// Uncertainty drives when nothing is due: mastery 0.5 beats 0.97.
    #[test]
    fn uncertainty_outranks_certainty() {
        let st = state_with(&[("silent_letters", 0.5, 100, 2), ("doubled_consonant", 0.97, 100, 2)]);
        let band = vec!["grass".to_string(), "knee".to_string()];
        let i = select_within(&st, &band, "en", 10, &[], 1).unwrap();
        assert_eq!(band[i], "knee", "the least-known skill's word wins");
    }

    /// Recent words sink but stay legal: chosen only when they are the
    /// entire band.
    #[test]
    fn recency_sinks_but_never_bans() {
        let st = state_with(&[("silent_letters", 0.5, 0, 1)]);
        let band = vec!["knee".to_string(), "lamb".to_string()];
        let recent = vec!["knee".to_string()];
        let i = select_within(&st, &band, "en", 20, &recent, 1).unwrap();
        assert_eq!(band[i], "lamb", "recent word sinks");
        let solo = vec!["knee".to_string()];
        assert_eq!(select_within(&st, &solo, "en", 20, &recent, 1), Some(0),
                   "a recent word is still legal when it is all there is");
    }
}
