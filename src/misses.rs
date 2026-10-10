use crate::model::{AppState, MissEntry, MISS_CAP, MISS_KEY};
use crate::storage;

fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub fn miss_key(word: &str, lang: &str) -> String {
    // CC-RUSSIAN-STRESS v3 I8: identity lives in ONE place now. Sense 0 emits
    // the byte-identical legacy key, so every miss record in the field keeps
    // working and no migration runs. A homograph (sense > 0) would key apart --
    // which is the point, since за́мок and замо́к must not share a miss record.
    crate::word_id::word_id0(lang, word)
}

pub fn load(state: &mut AppState) {
    state.misses = storage::get_json::<Vec<MissEntry>>(MISS_KEY).unwrap_or_default();
    carry_over_all(&mut state.misses);
}

/// L0 R2 carry-over: an entry stored by a pre-R2 build gets a `review`
/// schedule from its Leitner box, keeping its due time. Idempotent.
pub fn carry_over_all(list: &mut [MissEntry]) {
    for e in list.iter_mut().filter(|e| e.review.is_none()) {
        e.review = Some(crate::review::carry_over(e.box_, e.due));
    }
}

fn save(state: &AppState) {
    let capped: Vec<&MissEntry> = state.misses.iter().take(MISS_CAP).collect();
    storage::set_json(MISS_KEY, &capped);
}

pub fn due_misses(state: &AppState) -> Vec<usize> {
    let now = now_ms();
    state
        .misses
        .iter()
        .enumerate()
        .filter(|(_, m)| m.due <= now)
        .map(|(i, _)| i)
        .collect()
}

pub fn add_miss(state: &mut AppState, word: &str, lang: &str, tier: &str) {
    add_miss_at(state, word, lang, tier, now_ms());
}

/// In-memory miss recording with an injected timestamp. Split out from `now_ms`
/// so the spaced-rep data-integrity rules (CC-ATTEMPTS-SHIELDS Feature 1) can be
/// unit-tested on a non-wasm target, where `js_sys::Date::now()` panics. The
/// wasm `add_miss` wrapper is byte-for-byte the previous behavior.
pub fn add_miss_at(state: &mut AppState, word: &str, lang: &str, tier: &str, now: f64) {
    let key = miss_key(word, lang);
    if let Some(e) = state.misses.iter_mut().find(|x| miss_key(&x.word, &x.lang) == key) {
        e.misses += 1;
        e.ts = now;
        let r = e.review.get_or_insert_with(|| crate::review::carry_over(e.box_, e.due));
        e.due = crate::review::on_miss(r, now);
    } else {
        let (r, due) = crate::review::start(now);
        state.misses.insert(
            0,
            MissEntry {
                word: word.to_string(),
                lang: lang.to_string(),
                tier: tier.to_string(),
                misses: 1,
                box_: 0,
                due,
                ts: now,
                review: Some(r),
            },
        );
        if state.misses.len() > MISS_CAP {
            state.misses.truncate(MISS_CAP);
        }
    }
    save(state);
}

/// A correct answer on a queued word, graded Hard or Good (L0 D3), through
/// `review.rs`. Returns true if the word graduated and the list is now empty.
pub fn promote_miss(state: &mut AppState, word: &str, lang: &str, grade: crate::review::Grade) -> bool {
    // CC-PICK-QUEUE D3: flag OFF, or a word the lineup did not serve, is
    // `promote_miss_at` exactly as before.
    let picked = crate::pick_queue::served_by_lineup(word);
    promote_miss_picked_at(state, word, lang, grade, now_ms(), picked)
}

/// CC-PICK-QUEUE D3/I6 (O-G): a correct answer on a word the LINEUP served
/// counts as practice, but when that miss is not yet due it neither graduates
/// nor advances its schedule -- the next spaced check stays exactly where it
/// was. `picked == false`, or a due miss, is `promote_miss_at` unchanged, so
/// the ordinary path is byte-identical. The schedule is never computed here:
/// skipping is the absence of a call into `review.rs` (L0 I5).
pub fn promote_miss_picked_at(
    state: &mut AppState,
    word: &str,
    lang: &str,
    grade: crate::review::Grade,
    now: f64,
    picked: bool,
) -> bool {
    if picked {
        let key = miss_key(word, lang);
        if let Some(e) = state.misses.iter().find(|x| miss_key(&x.word, &x.lang) == key) {
            if e.due > now {
                return false;
            }
        }
    }
    promote_miss_at(state, word, lang, grade, now)
}

/// `promote_miss` with an injected clock (host tests have no JS clock).
pub fn promote_miss_at(state: &mut AppState, word: &str, lang: &str, grade: crate::review::Grade, now: f64) -> bool {
    let key = miss_key(word, lang);
    let Some(idx) = state.misses.iter().position(|x| miss_key(&x.word, &x.lang) == key) else {
        return false;
    };
    let outcome = {
        let e = &mut state.misses[idx];
        let r = e.review.get_or_insert_with(|| crate::review::carry_over(e.box_, e.due));
        crate::review::on_correct(r, grade, now)
    };
    let cleared;
    if let crate::review::Outcome::Due(due) = outcome {
        state.misses[idx].due = due;
        cleared = false;
    } else {
        // CC-REPORTS: graduation IS the redemption moment — record it
        // before the entry vanishes (3+ misses = a conquered boss).
        if state.misses[idx].misses >= 3 {
            let mut reds: Vec<crate::reports::Redemption> =
                crate::storage::get_json(crate::reports::REDEMPTION_KEY).unwrap_or_default();
            reds.push(crate::reports::Redemption {
                word: state.misses[idx].word.clone(),
                lang: state.misses[idx].lang.clone(),
                misses: state.misses[idx].misses,
                mastered_ts: now,
            });
            while reds.len() > crate::reports::REDEMPTION_CAP {
                reds.remove(0);
            }
            crate::storage::set_json(crate::reports::REDEMPTION_KEY, &reds);
        }
        {
            let (y, m, d) = crate::yearbook::ymd_pub((now / 86_400_000.0) as u32);
            let date = format!("{y:04}-{m:02}-{d:02}");
            let w = state.misses[idx].word.clone();
            let lang = state.misses[idx].lang.clone();
            crate::journal::note_today(&lang, &date, |e| e.mastered.push(w));
        }
        state.misses.remove(idx);
        cleared = state.misses.is_empty();
    }
    save(state);
    cleared
}

#[cfg(test)]
mod pick_tests {
    use super::*;
    use crate::review::Grade;

    const T0: f64 = 1_000_000_000_000.0;

    fn state_with(word: &str) -> AppState {
        let mut st = AppState::default();
        add_miss_at(&mut st, word, "en", "easy", T0);
        st
    }

    /// A5/I6/D3: a correct answer on a lineup pick that is NOT due counts as
    /// practice but changes nothing about its schedule and does not clear it.
    #[test]
    fn a_picked_correct_answer_on_a_not_due_miss_leaves_its_schedule_alone() {
        let mut st = state_with("cat");
        // Move it into the future the ordinary way: one Good answer.
        promote_miss_at(&mut st, "cat", "en", Grade::Good, T0 + 1_000.0);
        let before = st.misses[0].clone();
        assert!(before.due > T0 + 2_000.0, "now scheduled for later");
        let cleared = promote_miss_picked_at(&mut st, "cat", "en", Grade::Good, T0 + 2_000.0, true);
        assert!(!cleared);
        assert_eq!(st.misses.len(), 1, "not cleared");
        assert_eq!(st.misses[0].due, before.due, "next spaced check not skipped");
        assert_eq!(st.misses[0].review.as_ref().map(|r| r.step), before.review.as_ref().map(|r| r.step));
        assert_eq!(st.misses[0].misses, before.misses);
    }

    /// The ordinary path is untouched: not picked, same call advances the
    /// schedule exactly as `promote_miss_at` always did (grading untouched, A12).
    #[test]
    fn not_picked_is_exactly_promote_miss_at() {
        let mut a = state_with("cat");
        let mut b = state_with("cat");
        promote_miss_at(&mut a, "cat", "en", Grade::Good, T0 + 1_000.0);
        promote_miss_at(&mut b, "cat", "en", Grade::Good, T0 + 1_000.0);
        promote_miss_at(&mut a, "cat", "en", Grade::Good, T0 + 2_000.0);
        promote_miss_picked_at(&mut b, "cat", "en", Grade::Good, T0 + 2_000.0, false);
        assert_eq!(a.misses[0].due, b.misses[0].due);
        assert_eq!(
            a.misses[0].review.as_ref().map(|r| r.step),
            b.misses[0].review.as_ref().map(|r| r.step)
        );
    }

    /// A picked word that IS due is graded like any other: it advances (and may
    /// graduate), so what makes a Miss "learned" is untouched (I1, A12).
    #[test]
    fn a_picked_due_miss_is_graded_normally() {
        let mut a = state_with("cat");
        let mut b = state_with("cat");
        promote_miss_at(&mut a, "cat", "en", Grade::Good, T0);
        promote_miss_picked_at(&mut b, "cat", "en", Grade::Good, T0, true);
        assert_eq!(a.misses.len(), b.misses.len());
        assert_eq!(a.misses.first().map(|m| m.due), b.misses.first().map(|m| m.due));
    }

    /// A wrong answer on a picked word records a miss exactly as before
    /// (add_miss_at has no lineup input at all).
    #[test]
    fn a_wrong_answer_is_recorded_the_same() {
        let mut st = state_with("cat");
        add_miss_at(&mut st, "cat", "en", "easy", T0 + 5_000.0);
        assert_eq!(st.misses[0].misses, 2);
        assert_eq!(st.misses[0].due, T0 + 5_000.0);
    }
}
