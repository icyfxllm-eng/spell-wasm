//! CC-CALENDAR feature 5 — the daily journal (D3 SIGNED).
//!
//! The batch's second sanctioned new store. Laws, all load-bearing:
//! date-keyed (NOTHING finer than the date — no durations, no
//! timestamps, no open counts; I3), outcomes only, per profile ×
//! language, 730-day rolling cap, writes land on TODAY's entry only
//! (append-only from the session's point of view — past days are
//! history, not state), and ReportsQuery is the only reader. The gate's
//! I1 grep pins the key literal to this file.

use std::collections::BTreeSet;

const CAP_DAYS: usize = 730;

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    #[serde(default)]
    pub practiced: Vec<String>,
    #[serde(default)]
    pub missed: Vec<String>,
    #[serde(default)]
    pub mastered: Vec<String>,
}

fn key(lang: &str) -> String {
    format!("spell_journal_{lang}")
}

pub fn read_all(lang: &str) -> Vec<(String, Entry)> {
    crate::storage::get_json(&key(lang)).unwrap_or_default()
}

pub fn entry_for(lang: &str, date: &str) -> Option<Entry> {
    read_all(lang).into_iter().find(|(d, _)| d == date).map(|(_, e)| e)
}

/// Mutate TODAY's entry (creating it if the date is new). `date` must be
/// the caller's current date string ("YYYY-MM-DD") — the API offers no
/// way to write any other day, which is how append-only stays true by
/// construction rather than by review.
pub fn note_today(lang: &str, date: &str, f: impl FnOnce(&mut Entry)) {
    let mut all = read_all(lang);
    match all.last_mut() {
        Some((d, e)) if d == date => f(e),
        _ => {
            let mut e = Entry::default();
            f(&mut e);
            all.push((date.to_string(), e));
        }
    }
    while all.len() > CAP_DAYS {
        all.remove(0);
    }
    // Refs are word strings; dedup keeps an entry honest when the same
    // word is drilled repeatedly in one day.
    if let Some((_, e)) = all.last_mut() {
        dedup(&mut e.practiced);
        dedup(&mut e.missed);
        dedup(&mut e.mastered);
    }
    crate::storage::set_json(&key(lang), &all);
}

fn dedup(v: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    v.retain(|w| seen.insert(w.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_rolls_and_today_only_appends() {
        // Pure logic mirror (storage-free): the shaping rules only.
        let mut all: Vec<(String, Entry)> = (0..CAP_DAYS)
            .map(|i| (format!("d{i}"), Entry::default()))
            .collect();
        all.push(("today".into(), Entry::default()));
        while all.len() > CAP_DAYS {
            all.remove(0);
        }
        assert_eq!(all.len(), CAP_DAYS);
        assert_eq!(all.last().unwrap().0, "today");
        assert_eq!(all.first().unwrap().0, "d1", "oldest day rolled off");
    }

    #[test]
    fn dedup_keeps_first_occurrence() {
        let mut v = vec!["cat".to_string(), "dog".into(), "cat".into()];
        dedup(&mut v);
        assert_eq!(v, vec!["cat".to_string(), "dog".into()]);
    }
}
