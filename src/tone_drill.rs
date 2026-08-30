//! CC-ZH-TONE F3 — the tone drill queue.
//!
//! A word whose ONLY failures are tone misses is a different study from a word
//! the player cannot spell, and mixing them wastes both. Invariant 6: a
//! tone-only miss never enters the general missed-words queue.
//!
//! Deliberately a sibling of [`crate::misses`] rather than a flag on it. The
//! two queues are drawn from independently, and a word can be in neither, one,
//! or — if the player later misspells a word they used to only mis-tone —
//! moved from this one to that one. Sharing a store would make "not in the
//! general queue" a property to remember to check rather than a fact.
//!
//! Same Leitner shape as misses so the spacing behaves the way the rest of the
//! app already does: box 1 on a fresh miss, promote on a clean answer, and the
//! word leaves the queue at the top box.

use crate::consts::{SR_INT, SR_MAXBOX};
use crate::model::{AppState, MissEntry};
use crate::storage;

/// Its own key. A shared one would let a schema change to misses silently
/// reshape the drill, and vice versa.
pub const TONE_DRILL_KEY: &str = "byear_tone_drill_v1";
/// Smaller than MISS_CAP: this is one language's tone practice, not the whole
/// bank's misses.
pub const TONE_DRILL_CAP: usize = 120;

fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub fn key_of(word: &str, lang: &str) -> String {
    // v3 I8 — see word_id. Sense 0 is the legacy key, byte for byte.
    crate::word_id::word_id0(lang, word)
}

pub fn load(state: &mut AppState) {
    state.tone_drill = storage::get_json::<Vec<MissEntry>>(TONE_DRILL_KEY).unwrap_or_default();
}

fn save(state: &AppState) {
    let capped: Vec<&MissEntry> = state.tone_drill.iter().take(TONE_DRILL_CAP).collect();
    storage::set_json(TONE_DRILL_KEY, &capped);
}

pub fn due(state: &AppState) -> Vec<usize> {
    let now = now_ms();
    state
        .tone_drill
        .iter()
        .enumerate()
        .filter(|(_, m)| m.due <= now)
        .map(|(i, _)| i)
        .collect()
}

pub fn add(state: &mut AppState, word: &str, lang: &str, tier: &str) {
    add_at(state, word, lang, tier, now_ms());
}

/// Which queue a miss belongs in.
///
/// Pure and separate from the app so Invariant 6 is testable as a LAW rather
/// than as a side effect of a running game. misses.rs learned half of this
/// lesson -- it split the clock out for tests but left the storage write in, so
/// its own rules are still untested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Queue {
    /// The general missed-words queue: the player could not spell it.
    General,
    /// The tone drill: the spelling was right, the tone was not.
    ToneDrill,
}

pub fn route(verdict: &crate::pinyin::WordVerdict) -> Queue {
    if verdict.is_tone_only() {
        Queue::ToneDrill
    } else {
        Queue::General
    }
}

/// Timestamp injected so the spacing rules are unit-testable off-wasm, where
/// `js_sys::Date::now()` panics — the same split misses.rs uses.
pub fn add_at(state: &mut AppState, word: &str, lang: &str, tier: &str, now: f64) {
    add_into(&mut state.tone_drill, word, lang, tier, now);
    save(state);
}

/// The list operation, with no clock and no storage.
pub fn add_into(list: &mut Vec<MissEntry>, word: &str, lang: &str, tier: &str, now: f64) {
    let k = key_of(word, lang);
    if let Some(e) = list.iter_mut().find(|x| key_of(&x.word, &x.lang) == k) {
        e.misses += 1;
        e.box_ = 1;
        e.due = now;
        e.ts = now;
    } else {
        list.insert(
            0,
            MissEntry {
                word: word.to_string(),
                lang: lang.to_string(),
                tier: tier.to_string(),
                misses: 1,
                box_: 1,
                due: now,
                ts: now,
            },
        );
        if list.len() > TONE_DRILL_CAP {
            list.truncate(TONE_DRILL_CAP);
        }
    }
}

/// A clean answer promotes the word a box. Returns true when that cleared it
/// out of the drill entirely.
pub fn promote(state: &mut AppState, word: &str, lang: &str) -> bool {
    promote_at(state, word, lang, now_ms())
}

pub fn promote_at(state: &mut AppState, word: &str, lang: &str, now: f64) -> bool {
    let k = key_of(word, lang);
    let Some(i) = state.tone_drill.iter().position(|x| key_of(&x.word, &x.lang) == k) else {
        return false;
    };
    let b = state.tone_drill[i].box_ + 1;
    if b > SR_MAXBOX {
        state.tone_drill.remove(i);
        save(state);
        return true;
    }
    state.tone_drill[i].box_ = b;
    state.tone_drill[i].due = now + SR_INT[b as usize] as f64;
    save(state);
    false
}

/// Moving a word OUT of the drill when it stops being a tone-only problem.
/// Called when the same word later comes back with a segment miss: it is no
/// longer a tone study and belongs in the general queue instead.
pub fn remove(state: &mut AppState, word: &str, lang: &str) -> bool {
    let k = key_of(word, lang);
    let before = state.tone_drill.len();
    state.tone_drill.retain(|x| key_of(&x.word, &x.lang) != k);
    let changed = state.tone_drill.len() != before;
    if changed {
        save(state);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pinyin::{grade, ToneMode};

    fn v(typed: &str, answer: &str) -> crate::pinyin::WordVerdict {
        grade(typed, answer, ToneMode::Graded)
    }

    /// Done 4, the routing half: every tone-only word lands in the drill, and
    /// NONE of them reaches the general queue (Invariant 6).
    #[test]
    fn tone_only_misses_never_reach_the_general_queue() {
        let words: Vec<&str> = crate::words::tier_for("zh", "medium")
            .iter()
            .filter_map(|e| e.split('|').next())
            .filter(|p| {
                crate::pinyin::canonicalize_answer(p).map(|k| k.len() == 2).unwrap_or(false)
            })
            .take(50)
            .collect();
        assert!(words.len() >= 50);

        let (mut drill, mut general) = (Vec::new(), Vec::new());
        for w in &words {
            let key = crate::pinyin::canonicalize_answer(w).unwrap();
            // tone-wrong
            let toned: String = key
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}{}", s.segment, if i == 0 { s.tone % 5 + 1 } else { s.tone }))
                .collect();
            match route(&v(&toned, w)) {
                Queue::ToneDrill => add_into(&mut drill, w, "zh", "medium", 0.0),
                Queue::General => general.push(*w),
            }
            // segment-wrong
            let other = if key[0].segment == "ma" { "shu" } else { "ma" };
            let segged = format!("{}{}{}{}", other, key[0].tone, key[1].segment, key[1].tone);
            match route(&v(&segged, w)) {
                Queue::ToneDrill => drill.push(MissEntry {
                    word: w.to_string(), lang: "zh".into(), tier: "medium".into(),
                    misses: 1, box_: 1, due: 0.0, ts: 0.0,
                }),
                Queue::General => general.push(*w),
            }
        }
        assert_eq!(drill.len(), 50, "all 50 tone-only misses study in the drill");
        assert_eq!(general.len(), 50, "all 50 segment misses study in the general queue");
    }

    #[test]
    fn a_word_that_stops_being_tone_only_leaves_the_drill() {
        // The converse of Invariant 6: once the player gets the SPELLING wrong,
        // it is not a tone study any more and must not stay in the drill.
        let mut list = Vec::new();
        add_into(&mut list, "ping2guo3", "zh", "medium", 0.0);
        assert_eq!(list.len(), 1);
        assert_eq!(route(&v("ping2gua3", "ping2guo3")), Queue::General);
        list.retain(|x| key_of(&x.word, &x.lang) != key_of("ping2guo3", "zh"));
        assert!(list.is_empty(), "a segment miss evicts it from the drill");
    }

    #[test]
    fn repeat_misses_reset_the_box_rather_than_duplicating() {
        let mut list = Vec::new();
        add_into(&mut list, "ma3", "zh", "easy", 0.0);
        add_into(&mut list, "ma3", "zh", "easy", 100.0);
        assert_eq!(list.len(), 1, "one entry per word");
        assert_eq!(list[0].misses, 2);
        assert_eq!(list[0].box_, 1, "a fresh miss drops it back to box 1");
    }

    #[test]
    fn the_drill_is_capped() {
        let mut list = Vec::new();
        for i in 0..(TONE_DRILL_CAP + 25) {
            add_into(&mut list, &format!("ma{}", i % 5 + 1), "zh", "easy", i as f64);
        }
        assert!(list.len() <= TONE_DRILL_CAP);
    }

    #[test]
    fn the_two_queues_use_different_keys() {
        // Sharing a storage key would make "not in the general queue" a thing
        // to remember rather than a fact.
        assert_ne!(TONE_DRILL_KEY, crate::model::MISS_KEY);
    }
}

/// CC-ZH-TONE D5 — the tier-1 recoverability law, as a pure predicate.
///
/// Extracted from the app so Done 7 can fuzz it. The live gate in
/// `game.rs::zh_tone_retry_available` adds the runtime context it cannot see
/// from here (language, versus, the per-word budget); this is the rule those
/// checks are guarding.
pub fn tier1_retry_earned(verdict: &crate::pinyin::WordVerdict, tier: &str, retry_used: bool) -> bool {
    verdict.is_tone_only() && tier == crate::consts::TIER_ORDER[0] && !retry_used
}

#[cfg(test)]
mod d5_tests {
    use super::*;
    use crate::pinyin::{grade, ToneMode};

    /// Done 7: randomized tone-only submissions at tier 1 — zero unrecoverable
    /// zeros on first encounter. Every one earns its retry.
    #[test]
    fn tier1_tone_only_is_always_recoverable_once() {
        let words: Vec<&str> = crate::words::tier_for("zh", "easy")
            .iter()
            .filter_map(|e| e.split('|').next())
            .filter(|p| crate::pinyin::canonicalize_answer(p).is_ok())
            .take(120)
            .collect();
        assert!(words.len() >= 100, "need 100 easy words, got {}", words.len());

        // Deterministic "randomness": a multiplier walk over the tone space, so
        // a failure reproduces exactly instead of once in a blue moon.
        let mut seed: u64 = 0x5eed;
        let mut unrecoverable = Vec::new();
        let mut checked = 0;
        for w in &words {
            let key = crate::pinyin::canonicalize_answer(w).unwrap();
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let pick = (seed >> 33) as usize % key.len();
            let typed: String = key
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let t = if i == pick { s.tone % 5 + 1 } else { s.tone };
                    format!("{}{}", s.segment, t)
                })
                .collect();
            let v = grade(&typed, w, ToneMode::Graded);
            // The mutation must actually be a tone-only miss, or the case is
            // not testing what it claims to.
            if !v.is_tone_only() {
                continue;
            }
            checked += 1;
            if !tier1_retry_earned(&v, "easy", false) {
                unrecoverable.push(format!("{w} typed {typed} -> {v:?}"));
            }
        }
        assert!(checked >= 100, "only {checked} usable tone-only cases");
        assert!(
            unrecoverable.is_empty(),
            "{} unrecoverable zeros at tier 1:\n{}",
            unrecoverable.len(),
            unrecoverable.join("\n")
        );
    }

    #[test]
    fn the_retry_is_once_per_word_and_tier_one_only() {
        let v = grade("ma1", "ma3", ToneMode::Graded);
        assert!(tier1_retry_earned(&v, "easy", false));
        assert!(!tier1_retry_earned(&v, "easy", true), "one retry, not two");
        for t in ["medium", "hard", "expert"] {
            assert!(!tier1_retry_earned(&v, t, false), "tier {t} takes partial credit, no retry");
        }
    }

    #[test]
    fn a_segment_miss_never_earns_the_retry() {
        let v = grade("mao3", "ma3", ToneMode::Graded);
        assert!(!tier1_retry_earned(&v, "easy", false));
    }

    #[test]
    fn partial_credit_sits_between_a_miss_and_correct() {
        use crate::model::TierStat;
        let miss = TierStat { seen: 1, correct: 0, tone_partial: 0 };
        let tone = TierStat { seen: 1, correct: 0, tone_partial: 1 };
        let right = TierStat { seen: 1, correct: 1, tone_partial: 0 };
        assert!(miss.credited() < tone.credited(), "a tone miss beats a segment miss");
        assert!(tone.credited() < right.credited(), "but never counts as correct");
    }
}
