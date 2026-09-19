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
    /// "typed" | "speech" | "trap" — feature 4: hearing misses must never update
    /// spelling skills, and the flag is how L1's diagnosis will know.
    pub channel: Channel,
    /// CC-REPORTS: the raw attempt text on MISSES only (None on correct
    /// answers and on pre-Reports log entries) — the grapheme diagnosis
    /// raw material. Additive with a default, so no migration.
    #[serde(default)]
    pub typed: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Typed,
    Speech,
    /// CC-WORDGRID F-S2: TRAP_MISS -- a Spell Search player picked a trap
    /// spelling. `typed` holds the decoy. It is a spelling miss (the player took
    /// the wrong spelling for the word), so it updates skills like a typed one.
    Trap,
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
        if matches!(attempt.channel, Channel::Typed | Channel::Trap) {
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

/// An FSRS rating, as the game can earn one (L0 D3, signed 2026-09-19).
/// There is no Easy: intervals never balloon for kids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    /// A miss, including a word rescued by the retry (CC-ATTEMPTS-SHIELDS I2).
    Again,
    /// Correct on the first submission, after replaying the audio.
    Hard,
    /// Correct on the first submission with no replay.
    Good,
}

impl Grade {
    fn g(self) -> f64 {
        match self {
            Grade::Again => 1.0,
            Grade::Hard => 2.0,
            Grade::Good => 3.0,
        }
    }
}

/// One review of the skill model. Skills see pass/fail only: an attempt
/// exercises several skills, and a replay is evidence about the word, not
/// about each skill it contains.
pub fn fsrs_review(f: &mut FsrsState, correct: bool, day: u32) {
    fsrs_review_graded(f, if correct { Grade::Good } else { Grade::Again }, day)
}

/// One FSRS-4.5 review at `grade` on `day`. The single FSRS implementation
/// in the crate (C3): the skill model and the per-word review queue
/// (`review.rs`) both call it.
pub fn fsrs_review_graded(f: &mut FsrsState, grade: Grade, day: u32) {
    let w = &FSRS_W;
    let g = grade.g();
    if f.reps == 0 {
        // First exposure: initial stability is w[G−1] (w0 again, w1 hard,
        // w2 good). FSRS-4.5 D0(G) = w4 − (G−3)·w5. The exponential form
        // w4 − e^(w5·(G−1)) + 1 is FSRS-5's and needs FSRS-5's weights: fed
        // these 4.5 weights it gave a pass −5.5, clamped to 1.0 (the floor).
        f.stability = w[g as usize - 1];
        f.difficulty = (w[4] - (g - 3.0) * w[5]).clamp(1.0, 10.0);
    } else {
        let elapsed = (day.saturating_sub(f.due_day.saturating_sub(interval_days(f.stability)))) as f64;
        let r = retrievability(elapsed.max(0.0), f.stability);
        // difficulty update with mean reversion (w7 toward D0(3) = w4, the
        // FSRS-4.5 target; FSRS-5 moved it to D0(4))
        let d = f.difficulty - w[6] * (g - 3.0);
        f.difficulty = (w[7] * w[4] + (1.0 - w[7]) * d).clamp(1.0, 10.0);
        if grade != Grade::Again {
            // S' = S · (1 + e^w8 · (11−D) · S^−w9 · (e^(w10·(1−R)) − 1) · hard)
            // where hard = w15 for a Hard rating, else 1.
            let hard = if grade == Grade::Hard { w[15] } else { 1.0 };
            let inc = det_exp(w[8])
                * (11.0 - f.difficulty)
                * det_pow(f.stability, -w[9])
                * (det_exp(w[10] * (1.0 - r)) - 1.0)
                * hard;
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
/// Where unreadable bytes go before anything overwrites them (census C10).
/// Same prefix, so every audit that polices `spell_learner_` covers it too.
const BACKUP_SUFFIX: &str = "_unreadable";

// ------------------------------------------- C10: a failed load never wipes
//
// Before C10, every door did `load_state(..).ok().unwrap_or_else(new)` and
// then saved: ANY load failure silently replaced the player's state with an
// empty one. Two failures need opposite handling:
//
//   * NEWER schema — a player on build N+1 reinstalled build N. The bytes
//     are fine; this build just can't read them. Never write: run on an
//     in-memory state so the data is intact when they update again. The
//     `placed` flag is read leniently (every schema carries it) so the
//     placement offer doesn't nag a player who already answered it.
//   * UNREADABLE — corrupt JSON or a failed migration. Waiting cannot fix
//     it, so copy the bytes to the backup key (never clobbering an earlier
//     backup), then continue fresh and save normally, or the player would
//     never record again.

#[derive(Debug, PartialEq)]
pub enum Loaded {
    /// Nothing stored: a fresh state, saved normally.
    Empty(LearnerState),
    /// Stored and read (migrated if it was older).
    Stored(LearnerState),
    /// Stored by a newer build: fresh in memory, NEVER written.
    Newer(LearnerState),
    /// Unreadable: back up `raw`, then continue fresh.
    Unreadable { fresh: LearnerState, raw: String },
}

impl Loaded {
    pub fn state(self) -> LearnerState {
        match self {
            Loaded::Empty(s) | Loaded::Stored(s) | Loaded::Newer(s) => s,
            Loaded::Unreadable { fresh, .. } => fresh,
        }
    }
    pub fn writable(&self) -> bool {
        !matches!(self, Loaded::Newer(_))
    }
}

/// Pure: what to do with the stored bytes. No storage access, so every
/// branch is unit-testable on the host.
pub fn resolve_load(raw: Option<&str>, lang: &str) -> Loaded {
    let Some(raw) = raw else { return Loaded::Empty(LearnerState::new(lang)) };
    match load_state(raw) {
        Ok(st) => Loaded::Stored(st),
        Err(_) => {
            #[derive(Deserialize)]
            struct Lenient {
                version: u32,
                #[serde(default)]
                placed: Option<bool>,
            }
            match serde_json::from_str::<Lenient>(raw) {
                Ok(p) if p.version > SCHEMA_VERSION => {
                    let mut st = LearnerState::new(lang);
                    st.placed = p.placed;
                    Loaded::Newer(st)
                }
                _ => Loaded::Unreadable { fresh: LearnerState::new(lang), raw: raw.to_string() },
            }
        }
    }
}

fn store_key(lang: &str) -> String {
    format!("{STORE_PREFIX}{lang}")
}

/// Load for a language, backing up unreadable bytes first. Returns the
/// state and whether it may be written back.
fn open(lang: &str) -> (LearnerState, bool) {
    let key = store_key(lang);
    let loaded = resolve_load(crate::storage::get_raw(&key).as_deref(), lang);
    if let Loaded::Unreadable { raw, .. } = &loaded {
        let bak = format!("{key}{BACKUP_SUFFIX}");
        if crate::storage::get_raw(&bak).is_none() {
            crate::storage::set_raw(&bak, raw);
        }
    }
    let writable = loaded.writable();
    (loaded.state(), writable)
}

/// The one save. A state loaded over a newer build's bytes is never written.
fn save(lang: &str, st: &LearnerState, writable: bool) {
    if !writable {
        return;
    }
    if let Ok(json) = serde_json::to_string(st) {
        crate::storage::set_raw(&store_key(lang), &json);
    }
}

/// Progress-reset for one language (CC-LEARNING-ENGINE Done #8, D5): the
/// learner state AND its C10 backup go, so nothing learned about this
/// language survives a reset. Other languages are untouched. Placement is
/// offered again, which is right after a reset.
pub fn reset(lang: &str) {
    let key = store_key(lang);
    crate::storage::remove(&key);
    crate::storage::remove(&format!("{key}{BACKUP_SUFFIX}"));
}

/// The one gameplay door: load-or-new, record, save. Every submit path
/// calls this and nothing else — the speech exclusion lives inside
/// `record()`, not in callers remembering to skip it. Storage failures
/// degrade to a fresh profile rather than a crash: the learner is an
/// observer of the game, never a gate on it.
pub fn note_attempt(lang: &str, word: &str, correct: bool, channel: Channel) {
    note_attempt_typed(lang, word, correct, channel, None)
}

/// CC-REPORTS: the submit path passes the raw attempt on MISSES so the
/// grapheme diagnosis has its material. Correct answers pass None — the
/// log never stores what didn't diverge.
pub fn note_attempt_typed(lang: &str, word: &str, correct: bool, channel: Channel, typed: Option<&str>) {
    let (mut st, writable) = open(lang);
    st.record(attempt(lang, word, correct, channel, typed, today_day()));
    save(lang, &st, writable);
}

/// Record an attempt another mode built with `attempt` (CC-WORDGRID F-X6):
/// the same door as every base-game answer, C10's `open` and `save`
/// included, so an unreadable store is never overwritten from here either.
pub fn note_built(lang: &str, a: Attempt) {
    let (mut st, writable) = open(lang);
    st.record(a);
    save(lang, &st, writable);
}

/// Today's day index, as the log stores it.
pub fn day_now() -> u32 {
    today_day()
}

/// One attempt as the log stores it. Every mode builds its record here, so a
/// misspelling looks the same in the log whichever mode it came from
/// (CC-WORDGRID F-X6).
pub fn attempt(lang: &str, word: &str, correct: bool, channel: Channel, typed: Option<&str>, day: u32) -> Attempt {
    Attempt {
        day,
        word: word.to_string(),
        skills: hazards(lang, word),
        correct,
        channel,
        typed: if correct { None } else { typed.map(|t| t.to_string()) },
    }
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
    open(lang).0
}

/// Public wrapper for the day index (the hook needs the same epoch).
pub fn current_day() -> u32 {
    today_day()
}


// ------------------------------------------ L2 feature 6: the insight
//
// "The model earns trust by showing its work, gently." ONE line, opt-in,
// never mid-word (the caller is the post-answer reveal), phrasing from
// the audited pool — no generated prose, ever. The Kid Mode variant is
// Eric's review gate, so the flag ships OFF until he reads the copy.

/// The pattern worth naming right now: the weakest skill the learner has
/// enough evidence on (>=MIN_REPS attempts) and that is genuinely shaky
/// (mastery < INSIGHT_MASTERY). None = say nothing, which is the common
/// case and the correct one — an insight every word is nagging.
pub const INSIGHT_MIN_REPS: u32 = 4;
pub const INSIGHT_MASTERY: f64 = 0.55;

pub fn insight_skill(state: &LearnerState) -> Option<&SkillState> {
    state
        .skills
        .iter()
        .filter(|s| s.fsrs.reps >= INSIGHT_MIN_REPS && s.mastery < INSIGHT_MASTERY)
        .min_by(|a, b| a.mastery.partial_cmp(&b.mastery).unwrap_or(std::cmp::Ordering::Equal))
}

/// The audited i18n key for a skill's insight line. Kid Mode takes the
/// `.kid` variant (softer register) — BOTH pools are audited strings;
/// this function never composes prose.
pub fn insight_key(skill_id: &str, kid_mode: bool) -> String {
    if kid_mode {
        format!("insight.{skill_id}.kid")
    } else {
        format!("insight.{skill_id}")
    }
}

/// The one-line insight for the post-answer moment, or None. Callers
/// must render it AFTER the answer is resolved (never mid-word) and
/// only when the flag is on.
pub fn insight_line(lang: &str, kid_mode: bool) -> Option<String> {
    if !crate::flags::learner_insight() {
        return None;
    }
    let st = load_for(lang);
    let skill = insight_skill(&st)?;
    let key = insight_key(&skill.id, kid_mode);
    let line = crate::i18n::t(&key);
    // An unaudited language (or a missing key) renders NOTHING — the
    // defs-dark pattern: silence beats a key name on screen.
    if line == key || line.is_empty() {
        return None;
    }
    Some(line)
}

// ---------------------------------------------- L1 feature 5: placement

/// The language's hazard taxonomy — the skill ids `hazards()` can emit.
/// En-only for the same D2 reason hazards() is: other languages join with
/// their native-speaker sign-off, not before.
pub fn taxonomy(lang: &str) -> Vec<&'static str> {
    if lang != "en" {
        return Vec::new();
    }
    vec!["silent_letters", "doubled_consonant", "unstressed_vowel_ambiguity", "loanword_spelling"]
}

/// Placement words per skill: two exemplars for every taxonomy skill plus
/// two hazard-free baselines, chosen DETERMINISTICALLY from the easy and
/// medium pools (first matches in pool order — same set on every device,
/// every install). Done #4's coverage bar is >=90% of the taxonomy; two
/// exemplars each makes it 100% with a diagnosis-grade signal.
pub fn placement_set(lang: &str) -> Vec<String> {
    let tax = taxonomy(lang);
    if tax.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::new();
    let mut per: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    let mut baseline = 0u32;
    for tier in ["easy", "medium"] {
        for w in crate::words::tier_for(lang, tier) {
            let word = w.split('|').next().unwrap_or(w).to_string();
            if word.contains(' ') || out.contains(&word) {
                continue;
            }
            let hz = hazards(lang, &word);
            if hz.is_empty() {
                if baseline < 2 {
                    baseline += 1;
                    out.push(word);
                }
                continue;
            }
            if hz.iter().any(|h| per.get(h.as_str()).copied().unwrap_or(0) < 2) {
                for h in &hz {
                    if let Some(t) = tax.iter().find(|t| **t == h.as_str()) {
                        *per.entry(t).or_insert(0) += 1;
                    }
                }
                out.push(word);
            }
            if baseline >= 2 && tax.iter().all(|t| per.get(t).copied().unwrap_or(0) >= 2) {
                return out;
            }
        }
    }
    out
}

/// Placement is offered exactly once per (profile x language): never
/// offered again after a completion OR a skip, and never offered at all
/// for languages without a taxonomy.
pub fn should_offer_placement(lang: &str) -> bool {
    !placement_set(lang).is_empty() && load_for(lang).placed.is_none()
}

/// Placement prior shifts (the fixture-pinned contract): a correct
/// placement answer lifts every exercised skill's mastery to at least
/// PLACE_HI; a miss caps it at PLACE_LO. Decisive on purpose — placement
/// exists to move priors faster than one ordinary BKT step — and clamped,
/// never crossed: a later placement answer on a shared skill can only
/// widen what an earlier one set, not silently undo it.
pub const PLACE_HI: f64 = 0.65;
pub const PLACE_LO: f64 = 0.12;

pub fn apply_placement(state: &mut LearnerState, lang: &str, word: &str, correct: bool, day: u32) {
    let skills = hazards(lang, word);
    for id in &skills {
        let s = state.skill_mut(id);
        if correct {
            s.mastery = s.mastery.max(PLACE_HI);
        } else {
            s.mastery = s.mastery.min(PLACE_LO);
        }
        if s.fsrs.reps == 0 {
            s.fsrs.reps = 1;
            s.fsrs.stability = if correct { 3.0 } else { 0.5 };
            s.fsrs.due_day = day + if correct { 3 } else { 1 };
        }
    }
    if state.log.len() == LOG_CAP {
        state.log.pop_front();
    }
    state.log.push_back(Attempt {
        day,
        word: word.to_string(),
        skills,
        correct,
        channel: Channel::Typed,
        typed: None,
    });
}

/// Storage-backed wrappers for the UI wave: one placement answer, and the
/// finish/skip that closes the offer forever. Skip changes NOTHING but
/// the `placed` flag — the eval's "defaults intact" clause, by
/// construction and by test.
pub fn note_placement(lang: &str, word: &str, correct: bool) {
    let (mut st, writable) = open(lang);
    let day = today_day();
    apply_placement(&mut st, lang, word, correct, day);
    save(lang, &st, writable);
}

pub fn finish_placement(lang: &str, took: bool) {
    let (mut st, writable) = open(lang);
    st.placed = Some(took);
    save(lang, &st, writable);
}

// ------------------------------------------- L2: the guardian report

/// The guardian report — pure data, rendered deterministically, shared
/// only by explicit action (this module renders; it never sends).
#[derive(Debug, PartialEq)]
pub struct GuardianReport {
    pub attempts: u32,
    pub correct: u32,
    /// (skill id, mastery, days overdue) — every tracked skill, sorted
    /// weakest first so the render needs no policy of its own.
    pub skills: Vec<(String, f64, i64)>,
    /// Skill ids with mastery >= 0.8 after 2+ reps.
    pub strengths: Vec<String>,
    /// Skill ids with mastery < 0.45, or more than 7 days overdue.
    pub focus: Vec<String>,
}

pub fn guardian_report(state: &LearnerState, day: u32) -> GuardianReport {
    let attempts = state.log.len() as u32;
    let correct = state.log.iter().filter(|a| a.correct).count() as u32;
    let mut skills: Vec<(String, f64, i64)> = state
        .skills
        .iter()
        .filter(|s| s.fsrs.reps > 0)
        .map(|s| (s.id.clone(), s.mastery, day as i64 - s.fsrs.due_day as i64))
        .collect();
    skills.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let strengths = state
        .skills
        .iter()
        .filter(|s| s.fsrs.reps >= 2 && s.mastery >= 0.8)
        .map(|s| s.id.clone())
        .collect();
    let focus = skills
        .iter()
        .filter(|(_, m, over)| *m < 0.45 || *over > 7)
        .map(|(id, _, _)| id.clone())
        .collect();
    GuardianReport { attempts, correct, skills, strengths, focus }
}

/// Render the report as audited-string HTML. Every human-readable string
/// rides i18n (the audit gate); skill ids map through `skill.<id>` keys.
/// No share affordance lives here — sharing is the SURFACE's explicit
/// button, never the renderer's initiative.
pub fn guardian_report_html(r: &crate::learner_query::ReviewStats, _lang: &str) -> String {
    use crate::i18n;
    let mut h = String::from("<div class=\"guardian\">");
    h.push_str(&format!("<h3>{}</h3>", i18n::t("guardian.title")));
    if r.attempts == 0 {
        h.push_str(&format!("<p class=\"g-empty\">{}</p>", i18n::t("guardian.empty")));
        h.push_str("</div>");
        return h;
    }
    h.push_str(&format!(
        "<p class=\"g-sum\">{}</p>",
        i18n::tp("guardian.summary", &[("n", &r.attempts.to_string()), ("c", &r.correct.to_string())])
    ));
    if !r.strengths.is_empty() {
        h.push_str(&format!("<h4>{}</h4><ul class=\"g-strong\">", i18n::t("guardian.strengths")));
        for id in &r.strengths {
            h.push_str(&format!("<li>{}</li>", i18n::t(&format!("skill.{id}"))));
        }
        h.push_str("</ul>");
    }
    if !r.focus.is_empty() {
        h.push_str(&format!("<h4>{}</h4><ul class=\"g-focus\">", i18n::t("guardian.focus")));
        for id in &r.focus {
            h.push_str(&format!("<li>{}</li>", i18n::t(&format!("skill.{id}"))));
        }
        h.push_str("</ul>");
    }
    h.push_str("</div>");
    h
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
    fn fsrs_difficulty_matches_the_published_4_5_formulas() {
        // D0(G) = w4 − (G−3)·w5: good = w4 = 5.1618, again = w4 + 2·w5 =
        // 7.6214. Values, not an ordering: an ordering test passed while a
        // version mix pinned every correct first answer to the 1.0 floor.
        let mut pass = FsrsState { stability: 0.0, difficulty: FSRS_D0, due_day: 0, reps: 0, lapses: 0 };
        fsrs_review(&mut pass, true, 0);
        assert!((pass.difficulty - 5.1618).abs() < 1e-9, "D0(good) = {}", pass.difficulty);
        let mut fail = FsrsState { stability: 0.0, difficulty: FSRS_D0, due_day: 0, reps: 0, lapses: 0 };
        fsrs_review(&mut fail, false, 0);
        assert!((fail.difficulty - 7.6214).abs() < 1e-9, "D0(again) = {}", fail.difficulty);

        // A good review leaves D at the reversion target w4: D − w6·0 = w4.
        let day = pass.due_day;
        fsrs_review(&mut pass, true, day);
        assert!((pass.difficulty - FSRS_W[4]).abs() < 1e-9, "good on target = {}", pass.difficulty);
        // An again review: D' = w7·w4 + (1−w7)·(D + 2·w6).
        let d = fail.difficulty;
        let day = fail.due_day;
        fsrs_review(&mut fail, false, day);
        let want = FSRS_W[7] * FSRS_W[4] + (1.0 - FSRS_W[7]) * (d + 2.0 * FSRS_W[6]);
        assert!((fail.difficulty - want.clamp(1.0, 10.0)).abs() < 1e-9, "again: {} vs {want}", fail.difficulty);
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
            channel: Channel::Typed, typed: None,
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
            channel: Channel::Speech, typed: None,
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
                typed: None,
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
                    typed: None,
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
                channel: Channel::Typed, typed: None,
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

    // -- C10: a failed load never wipes ------------------------------------

    #[test]
    fn c10_nothing_stored_is_empty_and_writable() {
        let l = resolve_load(None, "en");
        assert!(l.writable());
        assert_eq!(l, Loaded::Empty(LearnerState::new("en")));
    }

    #[test]
    fn c10_a_readable_state_is_stored_and_writable() {
        let mut st = LearnerState::new("en");
        st.placed = Some(false);
        let json = serde_json::to_string(&st).unwrap();
        let l = resolve_load(Some(&json), "en");
        assert!(l.writable());
        assert_eq!(l, Loaded::Stored(st));
    }

    #[test]
    fn c10_a_newer_schema_is_never_written_and_keeps_placed() {
        // A later build's state, with fields this build has never heard of.
        let newer = r#"{"version": 99, "lang": "en", "skills": [], "log": [],
                        "placed": true, "profile": 3, "something_new": {"x": 1}}"#;
        let l = resolve_load(Some(newer), "en");
        assert!(!l.writable(), "a newer build's bytes must never be overwritten");
        let st = l.state();
        assert_eq!(st.placed, Some(true), "placed is read leniently so the offer doesn't nag");
        assert!(st.skills.is_empty() && st.log.is_empty(), "runs on a fresh in-memory state");
    }

    #[test]
    fn c10_unreadable_bytes_are_handed_back_for_backup() {
        for bad in [
            "not json at all",
            r#"{"version": 1, "lang": "en", "skills": "oops"}"#, // right version, wrong shape
            r#"{"version": 0, "lang": "en", "skills": [], "log": [], "placed": null}"#, // no migration path
            r#"{"lang": "en"}"#, // no version at all
        ] {
            match resolve_load(Some(bad), "en") {
                Loaded::Unreadable { fresh, raw } => {
                    assert_eq!(raw, bad, "the backup must be the exact bytes");
                    assert_eq!(fresh, LearnerState::new("en"));
                }
                other => panic!("{bad:?} resolved to {other:?}, expected Unreadable"),
            }
            assert!(resolve_load(Some(bad), "en").writable(), "{bad:?}: must recover, not stall forever");
        }
    }

    #[test]
    fn round_trip_is_identity() {
        let mut st = LearnerState::new("en");
        st.record(Attempt {
            day: 1,
            word: "quinoa".into(),
            skills: vec!["loanword_spelling".into()],
            correct: false,
            channel: Channel::Typed, typed: None,
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
                channel: Channel::Typed, typed: None,
            });
        }
        assert!(!st.skills.is_empty(), "typed attempts grew skills");
        let before = st.skills.clone();
        st.record(Attempt {
            day: 2,
            word: "wrong".into(),
            skills: hazards("en", "wrong"),
            correct: false,
            channel: Channel::Speech, typed: None,
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

    /// Done #3's fallback clause: a brand-new learner (empty state) still
    /// gets a valid, deterministic selection — the coverage bonus carries
    /// the policy until BKT has data. The flag-off path is not tested here
    /// because it never reaches this code at all (the game.rs gate).
    #[test]
    fn empty_state_still_selects_and_is_deterministic() {
        let st = LearnerState::new("en");
        let band = vec!["knee".to_string(), "grass".to_string(), "shop".to_string()];
        let pick = select_within(&st, &band, "en", 0, &[], 7);
        assert!(matches!(pick, Some(i) if i < band.len()), "empty state must still select");
        assert_eq!(pick, select_within(&st, &band, "en", 0, &[], 7), "and deterministically");
    }
}

#[cfg(test)]
mod placement_tests {
    use super::*;

    /// Done #4 bar 1: the set spans >=90% of the taxonomy (ours spans
    /// 100%), deterministically, at a size a first-run flow can carry.
    #[test]
    fn placement_set_covers_the_taxonomy() {
        let set = placement_set("en");
        assert!(!set.is_empty() && set.len() <= 16, "carryable size, got {}", set.len());
        let tax = taxonomy("en");
        let mut covered: std::collections::HashSet<String> = Default::default();
        for w in &set {
            for h in hazards("en", w) {
                covered.insert(h);
            }
        }
        let frac = covered.len() as f64 / tax.len() as f64;
        assert!(frac >= 0.9, "coverage {frac} below the 90% bar");
        assert_eq!(set, placement_set("en"), "same set on every device");
        assert!(placement_set("es").is_empty(), "no taxonomy, no placement");
    }

    /// Done #4 bar 2, fixture A: acing placement lifts every exercised
    /// skill's prior to at least PLACE_HI.
    #[test]
    fn placement_all_correct_lifts_priors() {
        let mut st = LearnerState::new("en");
        for w in placement_set("en") {
            apply_placement(&mut st, "en", &w, true, 10);
        }
        for t in taxonomy("en") {
            let s = st.skills.iter().find(|s| s.id == t).expect(t);
            assert!(s.mastery >= PLACE_HI, "{t} prior {} not lifted", s.mastery);
            assert!(s.fsrs.reps >= 1, "{t} scheduled");
        }
    }

    /// Fixture B: missing everything caps every exercised prior at
    /// PLACE_LO — the learner starts where they actually are.
    #[test]
    fn placement_all_wrong_lowers_priors() {
        let mut st = LearnerState::new("en");
        for w in placement_set("en") {
            apply_placement(&mut st, "en", &w, false, 10);
        }
        for t in taxonomy("en") {
            let s = st.skills.iter().find(|s| s.id == t).expect(t);
            assert!(s.mastery <= PLACE_LO, "{t} prior {} not lowered", s.mastery);
        }
    }

    /// Fixture C, mixed: one skill aced, one missed — shifts are
    /// per-skill, and the clamp never lets a later shared-skill answer
    /// undo an earlier one silently.
    #[test]
    fn placement_mixed_fixture() {
        let mut st = LearnerState::new("en");
        apply_placement(&mut st, "en", "knee", true, 10);   // silent_letters aced
        apply_placement(&mut st, "en", "grass", false, 10); // doubled_consonant missed
        let hi = st.skills.iter().find(|s| s.id == "silent_letters").unwrap();
        let lo = st.skills.iter().find(|s| s.id == "doubled_consonant").unwrap();
        assert!(hi.mastery >= PLACE_HI && lo.mastery <= PLACE_LO);
        assert_eq!(st.log.len(), 2, "placement attempts are logged");
    }

    /// Done #4 bar 3: the skip path leaves DEFAULTS intact — nothing but
    /// the placed flag moves, and the next ordinary attempt starts from
    /// the untouched BKT prior.
    #[test]
    fn skip_leaves_defaults_intact() {
        let mut st = LearnerState::new("en");
        st.placed = Some(false);
        assert!(st.skills.is_empty() && st.log.is_empty(), "skip writes no skill state");
        st.record(Attempt {
            day: 5,
            word: "knee".into(),
            skills: hazards("en", "knee"),
            correct: true,
            channel: Channel::Typed, typed: None,
        });
        let s = st.skills.iter().find(|s| s.id == "silent_letters").unwrap();
        assert_eq!(s.mastery, bkt_update(BKT_PRIOR, true), "first update starts from the default prior");
    }
}

#[cfg(test)]
mod insight_tests {
    use super::*;

    fn shaky(id: &str, mastery: f64, reps: u32) -> SkillState {
        SkillState {
            id: id.into(),
            mastery,
            fsrs: FsrsState { stability: 1.0, difficulty: 5.0, due_day: 1, reps, lapses: 0 },
        }
    }

    /// The insight names the WEAKEST well-evidenced skill, and stays
    /// silent when the evidence is thin or the learner is fine.
    #[test]
    fn insight_picks_the_weakest_evidenced_skill() {
        let mut st = LearnerState::new("en");
        assert!(insight_skill(&st).is_none(), "a fresh learner is told nothing");
        st.skills.push(shaky("doubled_consonant", 0.40, 6));
        st.skills.push(shaky("silent_letters", 0.25, 5));
        st.skills.push(shaky("loanword_spelling", 0.10, 2)); // too little evidence
        st.skills.push(shaky("unstressed_vowel_ambiguity", 0.90, 9)); // fine
        let pick = insight_skill(&st).expect("a pattern worth naming");
        assert_eq!(pick.id, "silent_letters", "weakest with enough reps");
        let thin = LearnerState { skills: vec![shaky("silent_letters", 0.1, 1)], ..LearnerState::new("en") };
        assert!(insight_skill(&thin).is_none(), "one attempt is not a pattern");
        let strong = LearnerState { skills: vec![shaky("silent_letters", 0.8, 9)], ..LearnerState::new("en") };
        assert!(insight_skill(&strong).is_none(), "nothing to say to a learner who has it");
    }

    /// Both registers exist in the audited pool for every taxonomy
    /// skill — the Kid Mode variant is a REAL string, not a fallback.
    #[test]
    fn every_skill_has_both_audited_registers() {
        let en: serde_json::Value =
            serde_json::from_str(include_str!("i18n/locales/en.json")).unwrap();
        for skill in taxonomy("en") {
            for kid in [false, true] {
                let key = insight_key(skill, kid);
                assert!(en.get(&key).is_some(), "missing audited insight string: {key}");
            }
        }
    }
}

#[cfg(test)]
mod guardian_tests {
    use super::*;

    fn skill(id: &str, mastery: f64, due: u32, reps: u32) -> SkillState {
        SkillState {
            id: id.into(),
            mastery,
            fsrs: FsrsState { stability: 1.0, difficulty: 5.0, due_day: due, reps, lapses: 0 },
        }
    }

    /// Fixture 1 — a brand-new learner: the report is the honest empty
    /// state, no invented numbers.
    #[test]
    fn report_fresh_learner() {
        let st = LearnerState::new("en");
        let r = guardian_report(&st, 10);
        assert_eq!(r.attempts, 0);
        assert!(r.skills.is_empty() && r.strengths.is_empty() && r.focus.is_empty());
    }

    /// Fixture 2 — struggling: weak and overdue skills surface as focus,
    /// weakest first, and nothing lands in strengths.
    #[test]
    fn report_struggling_learner() {
        let mut st = LearnerState::new("en");
        st.skills.push(skill("silent_letters", 0.2, 5, 3));
        st.skills.push(skill("doubled_consonant", 0.6, 2, 3)); // 18 days overdue at day 20
        for i in 0..6 {
            st.record(Attempt { day: 20, word: format!("w{i}"), skills: vec![], correct: i % 3 == 0, channel: Channel::Typed, typed: None });
        }
        let r = guardian_report(&st, 20);
        assert_eq!(r.attempts, 6);
        assert_eq!(r.skills[0].0, "silent_letters", "weakest first");
        assert!(r.focus.contains(&"silent_letters".to_string()), "weak skill in focus");
        assert!(r.focus.contains(&"doubled_consonant".to_string()), "overdue skill in focus");
        assert!(r.strengths.is_empty());
    }

    /// Fixture 3 — thriving: mastered skills in strengths, focus empty.
    #[test]
    fn report_thriving_learner() {
        let mut st = LearnerState::new("en");
        st.skills.push(skill("silent_letters", 0.92, 30, 5));
        st.skills.push(skill("loanword_spelling", 0.85, 28, 4));
        for i in 0..10 {
            st.record(Attempt { day: 25, word: format!("w{i}"), skills: vec![], correct: true, channel: Channel::Typed, typed: None });
        }
        let r = guardian_report(&st, 25);
        assert_eq!(r.correct, 10);
        assert_eq!(r.strengths.len(), 2);
        assert!(r.focus.is_empty());
        // determinism: same state, same report
        assert_eq!(guardian_report(&st, 25), r);
    }
}

#[cfg(test)]
mod placement_f0 {
    //! CC-PLACEMENT-IDEMPOTENCE F0 — dump before changing anything.
    use super::*;

    #[test]
    fn f0c_the_draw_is_unseeded_and_deterministic() {
        for lang in ["en", "es", "fr", "de", "zh", "ja", "ko", "pt", "ru", "pl", "vi", "ar", "hi", "sw", "fil"] {
            let a = placement_set(lang);
            let b = placement_set(lang);
            assert_eq!(a, b, "{lang}: two draws in one session differ — there IS a seed");
            let mut uniq = a.clone();
            uniq.sort();
            uniq.dedup();
            println!("  {lang:<4} n={:<3} distinct={:<3} first={:?}",
                     a.len(), uniq.len(), a.first().map(|s| s.as_str()).unwrap_or("-"));
            assert_eq!(a.len(), uniq.len(), "{lang}: DUPLICATE item inside one probe");
        }
    }
}

