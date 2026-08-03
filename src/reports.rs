//! CC-REPORTS — ReportsQuery, the ONE data path (feature 3). Strictly
//! read-only over learner state, misses, and stats (I1: any write from
//! here is a build failure — the gate greps for it). Two framings, one
//! computation (D5): the kid sees a quest log, the guardian sees a
//! diagnosis, and no deficit framing ever reaches a kid surface (I4).

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------- stores

/// Redemption record: written ONCE at miss-graduation (the only writer is
/// misses.rs's graduation path), read only here. Outcomes-only, capped —
/// the CAL-D3 sanction family.
#[derive(Serialize, Deserialize, Clone)]
pub struct Redemption {
    pub word: String,
    pub lang: String,
    pub misses: u32,
    pub mastered_ts: f64,
}

pub const REDEMPTION_KEY: &str = "spell_redemption_v1";
pub const REDEMPTION_CAP: usize = 200;

pub fn redemptions(lang: &str) -> Vec<Redemption> {
    let all: Vec<Redemption> = crate::storage::get_json(REDEMPTION_KEY).unwrap_or_default();
    all.into_iter().filter(|r| r.lang == lang).collect()
}

// ------------------------------------------------------------- kid queries

/// Words to Conquer: the most-missed list as CHALLENGES — max `cap`,
/// difficulty pips (1..=4) from the miss count, never a failure tally.
pub fn words_to_conquer(state: &crate::model::AppState, lang: &str, cap: usize) -> Vec<(String, u8)> {
    let mut v: Vec<(String, u32)> = state
        .misses
        .iter()
        .filter(|m| m.lang == lang)
        .map(|m| (m.word.clone(), m.misses))
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v.truncate(cap);
    v.into_iter().map(|(w, n)| (w, (n.min(4)) as u8)).collect()
}

/// Ready for a rematch (kid) / At risk this week (guardian) — the SAME
/// FSRS due window, two audited names (D5). Words currently in the miss
/// scheduler due within `horizon_ms`.
pub fn rematch_set(state: &crate::model::AppState, lang: &str, now_ms: f64, horizon_ms: f64) -> Vec<String> {
    let mut v: Vec<(f64, String)> = state
        .misses
        .iter()
        .filter(|m| m.lang == lang && m.due <= now_ms + horizon_ms)
        .map(|m| (m.due, m.word.clone()))
        .collect();
    v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    v.into_iter().map(|(_, w)| w).collect()
}

/// Trap Boss mastery bars: the learner model's per-skill mastery, 0..=100.
pub fn trap_mastery(lang: &str) -> Vec<(String, u8)> {
    let st = crate::learner::load_for(lang);
    crate::learner::taxonomy(lang)
        .into_iter()
        .map(|t| {
            let m = st.skills.iter().find(|s| s.id == t).map(|s| s.mastery).unwrap_or(0.0);
            (t.to_string(), (m * 100.0).round().clamp(0.0, 100.0) as u8)
        })
        .collect()
}

// ------------------------------------------------- grapheme alignment (v2)

/// One classified miss: how the typed attempt diverges from the target.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MissClass {
    Substitution,
    Omission,
    Insertion,
    Transposition,
}

/// Align typed vs target (classic edit script) and classify the FIRST
/// divergence with its position bucket. Pure and fixture-tested — the
/// confusion matrix and Error DNA both ride this one function.
pub fn classify_miss(typed: &str, target: &str) -> Option<(MissClass, char, char, Position)> {
    let t: Vec<char> = typed.chars().collect();
    let g: Vec<char> = target.chars().collect();
    if t == g {
        return None;
    }
    let mut i = 0;
    while i < t.len() && i < g.len() && t[i] == g[i] {
        i += 1;
    }
    let pos = bucket(i, g.len());
    // transposition: ab -> ba with the rest aligned — and ONLY at equal
    // lengths (a dropped double letter also matches the swap pattern but
    // is an omission; length disambiguates).
    if t.len() == g.len() && i + 1 < t.len() && i + 1 < g.len() && t[i] == g[i + 1] && t[i + 1] == g[i] {
        return Some((MissClass::Transposition, g[i], g[i + 1], pos));
    }
    // omission: typed skips target[i] (the rest realigns one ahead)
    if t.len() < g.len() && t.get(i) == g.get(i + 1) {
        return Some((MissClass::Omission, g[i], ' ', pos));
    }
    // insertion: typed has an extra char
    if t.len() > g.len() && t.get(i + 1) == g.get(i) {
        return Some((MissClass::Insertion, t[i], ' ', pos));
    }
    Some((MissClass::Substitution, g.get(i).copied().unwrap_or(' '), t.get(i).copied().unwrap_or(' '), pos))
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Position {
    Initial,
    Medial,
    Final,
}

fn bucket(i: usize, len: usize) -> Position {
    if i == 0 {
        Position::Initial
    } else if i + 1 >= len {
        Position::Final
    } else {
        Position::Medial
    }
}

/// Confusion pairs with counts and positions, from the enriched learner
/// log (entries that carry the typed text — capture began with this
/// wave; older entries simply don't contribute).

/// D3's first reader: words answered CORRECTLY but with long gaps between
/// keystrokes — "right but not yet fluent". Returns (word, avg gap ms),
/// slowest first.
pub fn hesitant_words(lang: &str, cap: usize) -> Vec<(String, u32)> {
    let ring: Vec<(String, Vec<u32>)> =
        crate::storage::get_json(&format!("spell_timing_{lang}")).unwrap_or_default();
    let mut best: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for (word, gaps) in ring {
        if gaps.is_empty() {
            continue;
        }
        let avg = (gaps.iter().map(|g| *g as u64).sum::<u64>() / gaps.len() as u64) as u32;
        // Latest entry wins: the ring is append-ordered, and fluency NOW is
        // the honest signal, not the worst historical run.
        best.insert(word, avg);
    }
    let mut v: Vec<(String, u32)> = best.into_iter().filter(|(_, a)| *a > 1800).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v.truncate(cap);
    v
}

pub fn confusion_pairs(lang: &str) -> Vec<((char, char), Position, u32)> {
    let st = crate::learner::load_for(lang);
    let mut counts: std::collections::BTreeMap<(char, char, u8), u32> = Default::default();
    for a in st.log.iter().filter(|a| !a.correct) {
        let Some(typed) = &a.typed else { continue };
        if let Some((MissClass::Substitution | MissClass::Transposition, x, y, pos)) =
            classify_miss(typed, &a.word)
        {
            let p = match pos { Position::Initial => 0, Position::Medial => 1, Position::Final => 2 };
            *counts.entry((x, y, p)).or_insert(0) += 1;
        }
    }
    let mut v: Vec<((char, char), Position, u32)> = counts
        .into_iter()
        .map(|((x, y, p), n)| {
            let pos = match p { 0 => Position::Initial, 1 => Position::Medial, _ => Position::Final };
            ((x, y), pos, n)
        })
        .collect();
    v.sort_by(|a, b| b.2.cmp(&a.2));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Acceptance #2's planted-error contract, at the classifier level.
    #[test]
    fn classifier_identifies_planted_errors() {
        // ie <-> ei medial transposition
        assert_eq!(classify_miss("recieve", "receive"),
                   Some((MissClass::Transposition, 'e', 'i', Position::Medial)));
        // dropped double-n -> omission
        let (cls, _, _, _) = classify_miss("runing", "running").unwrap();
        assert_eq!(cls, MissClass::Omission);
        // plain substitution, initial
        assert_eq!(classify_miss("kat", "cat"),
                   Some((MissClass::Substitution, 'c', 'k', Position::Initial)));
        // insertion
        let (cls, _, _, _) = classify_miss("caat", "cat").unwrap();
        assert_eq!(cls, MissClass::Insertion);
        // correct answers classify as nothing
        assert_eq!(classify_miss("cat", "cat"), None);
    }

    /// The kid framing law at the data layer: pips cap at 4 — a
    /// 30-times-missed word shows the same four pips as a 4-times one.
    #[test]
    fn conquer_pips_never_become_failure_tallies() {
        let mut st = crate::model::AppState::default();
        for (w, n) in [("gnome", 30u32), ("knee", 2)] {
            st.misses.push(crate::model::MissEntry {
                word: w.into(), lang: "en".into(), tier: "easy".into(),
                misses: n, box_: 1, due: 0.0, ts: 0.0,
            });
        }
        let v = words_to_conquer(&st, "en", 10);
        assert_eq!(v[0], ("gnome".to_string(), 4), "capped at four pips");
        assert_eq!(v[1], ("knee".to_string(), 2));
    }
}

/// CC-REPORTS I4 — the deep-link contract: every kid-row TYPE declares
/// its fix door here. A row type absent from this table is a build
/// failure (the test below is that CI).
pub const DEEP_LINKS: [(&str, &str); 4] = [
    ("conquer", "review"),      // Take it on -> smart review
    ("redemption", "review"),   // celebrate, replay path
    ("trap", "review"),         // INTERIM until CC-TRAP-BOSSES: class-filtered review
    ("rematch", "review"),      // the due set -> review session
];

#[cfg(test)]
mod kid_pool_lint {
    /// I4's lint: the conquest pool bans %, failure tallies, and
    /// comparison language on kid surfaces — checked against the shipped
    /// en pool (translations are audited to match its register).
    #[test]
    fn conquest_pool_has_no_deficit_framing() {
        let en = include_str!("i18n/locales/en.json");
        let d: serde_json::Value = serde_json::from_str(en).unwrap();
        let banned = ["%", "wrong", "failed", "failure", "behind", "worse", "error"];
        for (k, v) in d.as_object().unwrap() {
            if !k.starts_with("reports.") {
                continue;
            }
            let text = v.as_str().unwrap_or("").to_lowercase();
            for b in banned {
                assert!(!text.contains(b), "kid pool key {k} carries banned token {b:?}");
            }
        }
    }

    /// Every kid row type has a door (reports are doors, not verdicts).
    #[test]
    fn every_row_type_has_a_deep_link() {
        for row in ["conquer", "redemption", "trap", "rematch"] {
            assert!(super::DEEP_LINKS.iter().any(|(r, _)| *r == row), "{row} has no door");
        }
        for (_, target) in super::DEEP_LINKS {
            assert!(!target.is_empty());
        }
    }
}
