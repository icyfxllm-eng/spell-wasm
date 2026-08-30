use crate::consts::{SR_INT, SR_MAXBOX};
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
        e.box_ = 1;
        e.due = now;
        e.ts = now;
    } else {
        state.misses.insert(
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
        if state.misses.len() > MISS_CAP {
            state.misses.truncate(MISS_CAP);
        }
    }
    save(state);
}

/// Returns true if this promotion cleared the word out of the misses list entirely.
pub fn promote_miss(state: &mut AppState, word: &str, lang: &str) -> bool {
    let key = miss_key(word, lang);
    let Some(idx) = state.misses.iter().position(|x| miss_key(&x.word, &x.lang) == key) else {
        return false;
    };
    state.misses[idx].box_ += 1;
    let cleared;
    if state.misses[idx].box_ > SR_MAXBOX {
        // CC-REPORTS: graduation IS the redemption moment — record it
        // before the entry vanishes (3+ misses = a conquered boss).
        if state.misses[idx].misses >= 3 {
            let mut reds: Vec<crate::reports::Redemption> =
                crate::storage::get_json(crate::reports::REDEMPTION_KEY).unwrap_or_default();
            reds.push(crate::reports::Redemption {
                word: state.misses[idx].word.clone(),
                lang: state.misses[idx].lang.clone(),
                misses: state.misses[idx].misses,
                mastered_ts: now_ms(),
            });
            while reds.len() > crate::reports::REDEMPTION_CAP {
                reds.remove(0);
            }
            crate::storage::set_json(crate::reports::REDEMPTION_KEY, &reds);
        }
        {
            let (y, m, d) = crate::yearbook::ymd_pub((now_ms() / 86_400_000.0) as u32);
            let date = format!("{y:04}-{m:02}-{d:02}");
            let w = state.misses[idx].word.clone();
            let lang = state.misses[idx].lang.clone();
            crate::journal::note_today(&lang, &date, |e| e.mastered.push(w));
        }
        state.misses.remove(idx);
        cleared = state.misses.is_empty();
    } else {
        let box_ = state.misses[idx].box_ as usize;
        state.misses[idx].due = now_ms() + SR_INT.get(box_).copied().unwrap_or(0) as f64;
        cleared = false;
    }
    save(state);
    cleared
}
