//! CC-MYWORDS-LISTS v1 — dated word lists.
//!
//! The model and every rule that decides anything, with no DOM in sight, so the
//! whole thing is testable on the host. Screens call in; nothing here calls out.
//!
//! Scope is the DEVICE (signed 2026-09-17): there is no profile system, and the
//! census found none. `profile` is carried as an empty slot so a future profile
//! system can adopt these lists without a second migration.
//!
//! The invariant that matters most: **saving never deletes** (I1). Nothing in
//! this module removes an entry except the calls a player makes on purpose from
//! an opened list, and those are recoverable (F5).

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::model::CustomSet;
use crate::storage;

pub const LISTS_KEY: &str = "byear_word_lists_v1";
const VERSION: u32 = 1;

/// How long a soft-deleted list stays restorable (F5.3).
pub const RECENTLY_DELETED_DAYS: f64 = 30.0;
const DAY_MS: f64 = 86_400_000.0;

/// Where a list came from. Recorded, never inferred later.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ListSource {
    Photo,
    Translate,
    Manual,
    Migrated,
}

/// D3 (signed): how a list hands its words to a mode.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Order {
    /// The adaptive order the game already uses: misses weighted, spacing kept.
    #[default]
    Mixed,
    /// The player's own order, as dragged; starred entries lead.
    InMyOrder,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ListEntry {
    pub text: String,
    pub lang: String,
    #[serde(default, rename = "addedAt")]
    pub added_at: f64,
    /// D3: a starred entry sorts to the front under `Order::InMyOrder`.
    #[serde(default)]
    pub starred: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct WordList {
    pub id: String,
    /// Reserved for a future profile system; empty on a per-device install.
    #[serde(default)]
    pub profile: String,
    pub name: String,
    #[serde(rename = "createdAt")]
    pub created_at: f64,
    #[serde(rename = "updatedAt")]
    pub updated_at: f64,
    pub source: ListSource,
    pub entries: Vec<ListEntry>,
    #[serde(default, rename = "deletedAt")]
    pub deleted_at: Option<f64>,
    #[serde(default)]
    pub order: Order,
}

impl WordList {
    pub fn live(&self) -> bool {
        self.deleted_at.is_none()
    }
}

#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
pub struct Lists {
    #[serde(default)]
    pub v: u32,
    #[serde(default)]
    pub lists: Vec<WordList>,
    /// Monotonic, so an id is never reused by a restored list.
    #[serde(default, rename = "nextId")]
    pub next_id: u64,
    /// F6.3: the marker that stops the migration running twice.
    #[serde(default)]
    pub migrated: bool,
    /// F4.2: the lists chosen for play, remembered per device.
    #[serde(default)]
    pub selection: Vec<String>,
}

/// I4: every entry is NFC before it is stored or compared.
pub fn nfc(s: &str) -> String {
    s.nfc().collect()
}

/// I3's comparison key: same text (NFC, case-folded) in the same language.
fn entry_key(text: &str, lang: &str) -> (String, String) {
    (nfc(text).to_lowercase(), lang.to_string())
}

// ---------- names (F2) ----------

/// F2.2/F2.4: `base`, then "base (2)", "base (3)" — the first name free of a
/// clash. Names are compared as stored, trimmed.
pub fn unique_name(base: &str, taken: &[String]) -> String {
    let base = base.trim();
    let used = |c: &str| taken.iter().any(|t| t.trim() == c);
    if !used(base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let candidate = format!("{base} ({n})");
        if !used(&candidate) {
            return candidate;
        }
    }
    format!("{base} ({})", js_now_fallback())
}

/// Only reached if a player somehow holds 998 same-named lists.
fn js_now_fallback() -> u64 {
    1000
}

/// F2.4: a rename, trimmed to 1..=40 characters, made unique against the other
/// lists. Returns None when the trimmed name is empty.
pub fn rename_to(lists: &Lists, id: &str, name: &str) -> Option<String> {
    let name: String = name.trim().chars().take(40).collect();
    if name.is_empty() {
        return None;
    }
    let taken: Vec<String> =
        lists.lists.iter().filter(|l| l.id != id).map(|l| l.name.clone()).collect();
    Some(unique_name(&name, &taken))
}

// ---------- destination (F1 / D1) ----------

/// Where the save sheet points before the player touches it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Destination {
    /// A new list, named for today.
    New,
    /// Today's list, which already exists (D1, signed).
    Add(String),
}

/// D1 (signed): if a list was created today, adding to it is the default, so a
/// worksheet photographed in two pages lands in one list. Otherwise a new dated
/// list. `today` is the label F2 derives once, at creation.
pub fn default_destination(lists: &Lists, today: &str) -> Destination {
    match lists
        .lists
        .iter()
        .filter(|l| l.live() && (l.name == today || l.name.starts_with(&format!("{today} ("))))
        .max_by(|a, b| a.created_at.partial_cmp(&b.created_at).unwrap_or(std::cmp::Ordering::Equal))
    {
        Some(l) => Destination::Add(l.id.clone()),
        None => Destination::New,
    }
}

// ---------- creating and adding (F1, I1, I3, I4) ----------

pub fn create(lists: &mut Lists, name: &str, source: ListSource, now: f64) -> String {
    let taken: Vec<String> = lists.lists.iter().map(|l| l.name.clone()).collect();
    lists.next_id += 1;
    let id = format!("l{}", lists.next_id);
    lists.lists.push(WordList {
        id: id.clone(),
        profile: String::new(),
        name: unique_name(name, &taken),
        created_at: now,
        updated_at: now,
        source,
        entries: Vec::new(),
        deleted_at: None,
        order: Order::default(),
    });
    id
}

/// Add words to a list. Returns how many were NEW: a word the list already holds
/// is not added twice (I3) and nothing is ever removed (I1).
pub fn add_entries(lists: &mut Lists, id: &str, words: &[(String, String)], now: f64) -> usize {
    let Some(list) = lists.lists.iter_mut().find(|l| l.id == id) else { return 0 };
    let mut added = 0;
    for (text, lang) in words {
        let text = nfc(text);
        if text.trim().is_empty() {
            continue;
        }
        let key = entry_key(&text, lang);
        if list.entries.iter().any(|e| entry_key(&e.text, &e.lang) == key) {
            continue;
        }
        list.entries.push(ListEntry { text, lang: lang.clone(), added_at: now, starred: false });
        added += 1;
    }
    if added > 0 {
        list.updated_at = now;
    }
    added
}

// ---------- editing an opened list (D2, signed) ----------

/// Fix a word, or change the language it is spoken in. Refuses a change that
/// would duplicate another entry in the same list (I3), and refuses empty text.
pub fn edit_entry(lists: &mut Lists, id: &str, index: usize, text: &str, lang: &str, now: f64) -> bool {
    let Some(list) = lists.lists.iter_mut().find(|l| l.id == id) else { return false };
    let text = nfc(text);
    if text.trim().is_empty() || index >= list.entries.len() {
        return false;
    }
    let key = entry_key(&text, lang);
    if list.entries.iter().enumerate().any(|(i, e)| i != index && entry_key(&e.text, &e.lang) == key) {
        return false;
    }
    list.entries[index].text = text;
    list.entries[index].lang = lang.to_string();
    list.updated_at = now;
    true
}

/// F5.4: removing one entry needs no confirm; the caller offers Undo, which is
/// `add_entries` again (the entry is returned so it can be put back in place).
pub fn remove_entry(lists: &mut Lists, id: &str, index: usize, now: f64) -> Option<ListEntry> {
    let list = lists.lists.iter_mut().find(|l| l.id == id)?;
    if index >= list.entries.len() {
        return None;
    }
    let gone = list.entries.remove(index);
    list.updated_at = now;
    Some(gone)
}

/// Put a removed entry back where it was (Undo).
pub fn insert_entry(lists: &mut Lists, id: &str, index: usize, entry: ListEntry, now: f64) -> bool {
    let Some(list) = lists.lists.iter_mut().find(|l| l.id == id) else { return false };
    let key = entry_key(&entry.text, &entry.lang);
    if list.entries.iter().any(|e| entry_key(&e.text, &e.lang) == key) {
        return false;
    }
    let at = index.min(list.entries.len());
    list.entries.insert(at, entry);
    list.updated_at = now;
    true
}

/// D2: copy a word into another list. The source keeps it (a word may live in
/// several lists), and the target refuses a duplicate.
pub fn copy_entry(lists: &mut Lists, from: &str, index: usize, to: &str, now: f64) -> bool {
    let Some(src) = lists.lists.iter().find(|l| l.id == from) else { return false };
    let Some(entry) = src.entries.get(index).cloned() else { return false };
    add_entries(lists, to, &[(entry.text, entry.lang)], now) == 1
}

/// D3: the player's order, by dragging. Out-of-range moves are ignored.
pub fn move_entry(lists: &mut Lists, id: &str, from: usize, to: usize, now: f64) -> bool {
    let Some(list) = lists.lists.iter_mut().find(|l| l.id == id) else { return false };
    if from >= list.entries.len() || to >= list.entries.len() || from == to {
        return false;
    }
    let e = list.entries.remove(from);
    list.entries.insert(to, e);
    list.updated_at = now;
    true
}

pub fn set_star(lists: &mut Lists, id: &str, index: usize, on: bool, now: f64) -> bool {
    let Some(list) = lists.lists.iter_mut().find(|l| l.id == id) else { return false };
    let Some(e) = list.entries.get_mut(index) else { return false };
    e.starred = on;
    list.updated_at = now;
    true
}

pub fn set_order(lists: &mut Lists, id: &str, order: Order, now: f64) -> bool {
    let Some(list) = lists.lists.iter_mut().find(|l| l.id == id) else { return false };
    list.order = order;
    list.updated_at = now;
    true
}

// ---------- deleting, with a way back (F5 / D4) ----------

pub fn soft_delete(lists: &mut Lists, id: &str, now: f64) -> bool {
    let Some(list) = lists.lists.iter_mut().find(|l| l.id == id) else { return false };
    if list.deleted_at.is_some() {
        return false;
    }
    list.deleted_at = Some(now);
    lists.selection.retain(|s| s != id); // F4.2: a deleted list stops being played
    true
}

pub fn restore(lists: &mut Lists, id: &str) -> bool {
    let Some(list) = lists.lists.iter_mut().find(|l| l.id == id) else { return false };
    list.deleted_at = None;
    true
}

/// "Delete now" from Recently deleted, and the only other hard delete (I2).
pub fn delete_now(lists: &mut Lists, id: &str) -> bool {
    let before = lists.lists.len();
    lists.lists.retain(|l| l.id != id || l.deleted_at.is_none());
    lists.lists.len() < before
}

/// F5.3: the 30-day purge. Returns how many lists it removed.
pub fn purge_expired(lists: &mut Lists, now: f64) -> usize {
    let before = lists.lists.len();
    lists.lists.retain(|l| match l.deleted_at {
        Some(at) => now - at < RECENTLY_DELETED_DAYS * DAY_MS,
        None => true,
    });
    before - lists.lists.len()
}

// ---------- reading (F3, F4) ----------

/// F3.1: the cards, newest first. Soft-deleted lists are absent (AT5.2).
pub fn visible(lists: &Lists) -> Vec<&WordList> {
    let mut out: Vec<&WordList> = lists.lists.iter().filter(|l| l.live()).collect();
    out.sort_by(|a, b| b.created_at.partial_cmp(&a.created_at).unwrap_or(std::cmp::Ordering::Equal));
    out
}

pub fn recently_deleted(lists: &Lists) -> Vec<&WordList> {
    let mut out: Vec<&WordList> = lists.lists.iter().filter(|l| !l.live()).collect();
    out.sort_by(|a, b| b.deleted_at.partial_cmp(&a.deleted_at).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// D4 (signed): lists older than `fold_after_days` fold into "Older lists", so a
/// tidy screen never needs a delete. Returns (recent, older), both newest first.
pub fn folded(lists: &Lists, now: f64, fold_after_days: f64) -> (Vec<&WordList>, Vec<&WordList>) {
    let cutoff = now - fold_after_days * DAY_MS;
    let all = visible(lists);
    let (recent, older): (Vec<&WordList>, Vec<&WordList>) =
        all.into_iter().partition(|l| l.created_at >= cutoff);
    (recent, older)
}

/// F3.3: the distinct languages on a card, capped, with the overflow count.
pub fn language_chips(list: &WordList, cap: usize) -> (Vec<String>, usize) {
    let mut seen: Vec<String> = Vec::new();
    for e in &list.entries {
        if !seen.iter().any(|l| l == &e.lang) {
            seen.push(e.lang.clone());
        }
    }
    if seen.len() <= cap {
        return (seen, 0);
    }
    let rest = seen.len() - cap;
    seen.truncate(cap);
    (seen, rest)
}

/// F3.3: every live list's words, deduplicated by text + language, sorted A→Z
/// within each language.
pub fn all_words(lists: &Lists) -> Vec<ListEntry> {
    let mut out: Vec<ListEntry> = Vec::new();
    for l in visible(lists) {
        for e in &l.entries {
            let key = entry_key(&e.text, &e.lang);
            if !out.iter().any(|x| entry_key(&x.text, &x.lang) == key) {
                out.push(e.clone());
            }
        }
    }
    out.sort_by(|a, b| {
        a.lang.cmp(&b.lang).then_with(|| a.text.to_lowercase().cmp(&b.text.to_lowercase()))
    });
    out
}

/// F4.1/F4.3: the words a play session gets. Deduplicated across the chosen
/// lists; `Order::InMyOrder` keeps each list's own order with starred entries
/// first, `Mixed` hands them over in saved order for the adaptive picker.
pub fn play_entries(lists: &Lists, chosen: &[String]) -> Vec<ListEntry> {
    let mut out: Vec<ListEntry> = Vec::new();
    for id in chosen {
        let Some(l) = lists.lists.iter().find(|l| &l.id == id && l.live()) else { continue };
        let mut entries: Vec<ListEntry> = l.entries.clone();
        if l.order == Order::InMyOrder {
            let (starred, rest): (Vec<ListEntry>, Vec<ListEntry>) =
                entries.into_iter().partition(|e| e.starred);
            entries = starred.into_iter().chain(rest).collect();
        }
        for e in entries {
            let key = entry_key(&e.text, &e.lang);
            if !out.iter().any(|x| entry_key(&x.text, &x.lang) == key) {
                out.push(e);
            }
        }
    }
    out
}

/// F4.2: the remembered selection, with a deleted list falling back to the
/// newest live one. Empty only when the player has no lists at all.
pub fn play_selection(lists: &Lists) -> Vec<String> {
    let live: Vec<String> =
        lists.selection.iter().filter(|id| lists.lists.iter().any(|l| &&l.id == id && l.live())).cloned().collect();
    if !live.is_empty() {
        return live;
    }
    visible(lists).first().map(|l| vec![l.id.clone()]).unwrap_or_default()
}

pub fn remember_selection(lists: &mut Lists, chosen: &[String]) {
    lists.selection = chosen.to_vec();
}

// ---------- migration (F6 / I7) ----------

/// F6: move the flat My Words into one list, once. Idempotent through
/// `migrated`, which is set even when there was nothing to move (F6.2/AT6.4).
/// Order, text and per-word language are preserved exactly; nothing is dropped,
/// and no progress is touched (C3 keys progress by word + language, not by list).
pub fn migrate_flat(lists: &mut Lists, custom: &CustomSet, name: &str, now: f64) -> bool {
    if lists.migrated {
        return false;
    }
    lists.migrated = true;
    if custom.words.is_empty() {
        return false;
    }
    let id = create(lists, name, ListSource::Migrated, now);
    let words: Vec<(String, String)> = custom
        .words
        .iter()
        .map(|w| {
            let lang = custom
                .word_lang
                .get(w)
                .filter(|l| !l.is_empty())
                .cloned()
                .unwrap_or_else(|| custom.speak_lang.clone());
            (w.clone(), lang)
        })
        .collect();
    add_entries(lists, &id, &words, now);
    true
}

// ---------- the label, and boot ----------

/// F2.1/F2.3: the local calendar date, in the UI language's own short format,
/// derived ONCE at creation. A later time-zone change cannot rename a list,
/// because the label is stored, not recomputed.
pub fn today_label(now_ms: f64, ui_lang: &str) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(now_ms));
    let opts = js_sys::Object::new();
    for (k, v) in [("year", "numeric"), ("month", "short"), ("day", "numeric")] {
        let _ = js_sys::Reflect::set(
            &opts,
            &wasm_bindgen::JsValue::from_str(k),
            &wasm_bindgen::JsValue::from_str(v),
        );
    }
    let s: String = d.to_locale_date_string(ui_lang, &opts).into();
    if s.is_empty() {
        // Intl refused the locale tag: a stable fallback beats an unnamed list.
        return d.to_iso_string().as_string().unwrap_or_default().chars().take(10).collect();
    }
    s
}

/// Run at boot, before any screen reads lists: the one-time migration (F6) and
/// the 30-day purge (F5.3). Writes only when something changed.
pub fn boot(custom: &CustomSet, now: f64, migrated_name: &str) {
    let mut lists = load();
    let was_migrated = lists.migrated;
    let migrated = migrate_flat(&mut lists, custom, migrated_name, now);
    let purged = purge_expired(&mut lists, now);
    if migrated || purged > 0 || was_migrated != lists.migrated {
        store(&lists);
    }
}

// ---------- storage ----------

pub fn load() -> Lists {
    match storage::get_json::<Lists>(LISTS_KEY) {
        Some(l) if l.v == VERSION => l,
        // Unknown version or nothing stored: start clean, and leave the flat set
        // alone so the migration can still claim it.
        _ => Lists { v: VERSION, ..Default::default() },
    }
}

pub fn store(lists: &Lists) {
    let mut out = lists.clone();
    out.v = VERSION;
    storage::set_json(LISTS_KEY, &out);
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: f64 = 1_700_000_000_000.0;

    fn custom(words: &[&str], lang: &str) -> CustomSet {
        CustomSet {
            words: words.iter().map(|w| w.to_string()).collect(),
            speak_lang: lang.to_string(),
            ..Default::default()
        }
    }

    fn with_list(name: &str, now: f64) -> (Lists, String) {
        let mut l = Lists::default();
        let id = create(&mut l, name, ListSource::Photo, now);
        (l, id)
    }

    // ---- I1: saving never deletes ----

    #[test]
    fn saving_only_ever_adds() {
        let (mut l, a) = with_list("Sep 14, 2026", T0);
        add_entries(&mut l, &a, &[("cat".into(), "en".into()), ("dog".into(), "en".into())], T0);
        let b = create(&mut l, "Sep 21, 2026", ListSource::Photo, T0 + DAY_MS * 7.0);
        add_entries(&mut l, &b, &[("cat".into(), "en".into())], T0 + DAY_MS * 7.0);
        assert_eq!(l.lists[0].entries.len(), 2, "the older list is untouched by a later save");
        assert_eq!(l.lists[1].entries.len(), 1);
        // AT1.3: adding to an existing list grows only that list.
        add_entries(&mut l, &a, &[("fox".into(), "en".into())], T0);
        assert_eq!(l.lists[0].entries.len(), 3);
        assert_eq!(l.lists[1].entries.len(), 1);
    }

    // ---- I3 / I4 ----

    #[test]
    fn a_word_is_never_twice_in_one_list_but_may_be_in_several() {
        let (mut l, a) = with_list("A", T0);
        assert_eq!(add_entries(&mut l, &a, &[("cat".into(), "en".into())], T0), 1);
        assert_eq!(add_entries(&mut l, &a, &[("CAT".into(), "en".into())], T0), 0, "same word, same language");
        assert_eq!(add_entries(&mut l, &a, &[("cat".into(), "es".into())], T0), 1, "another language is another entry");
        let b = create(&mut l, "B", ListSource::Manual, T0);
        assert_eq!(add_entries(&mut l, &b, &[("cat".into(), "en".into())], T0), 1, "D2: a word may live in several lists");
    }

    #[test]
    fn entries_are_stored_nfc() {
        let (mut l, a) = with_list("A", T0);
        add_entries(&mut l, &a, &[("cafe\u{301}".into(), "fr".into())], T0);
        assert_eq!(l.lists[0].entries[0].text, "café");
        assert_eq!(add_entries(&mut l, &a, &[("café".into(), "fr".into())], T0), 0, "the decomposed form is the same word");
    }

    // ---- F2 names ----

    #[test]
    fn same_day_lists_number_themselves() {
        let mut l = Lists::default();
        for _ in 0..3 {
            create(&mut l, "Sep 14, 2026", ListSource::Photo, T0);
        }
        let names: Vec<String> = l.lists.iter().map(|x| x.name.clone()).collect();
        assert_eq!(names, vec!["Sep 14, 2026", "Sep 14, 2026 (2)", "Sep 14, 2026 (3)"]);
    }

    #[test]
    fn a_rename_is_trimmed_capped_and_made_unique() {
        let (mut l, a) = with_list("Sep 14, 2026", T0);
        create(&mut l, "Spelling", ListSource::Manual, T0);
        assert_eq!(rename_to(&l, &a, "  Spelling  ").as_deref(), Some("Spelling (2)"));
        assert_eq!(rename_to(&l, &a, "   "), None);
        assert_eq!(rename_to(&l, &a, &"x".repeat(60)).unwrap().chars().count(), 40);
    }

    // ---- D1 destination ----

    #[test]
    fn todays_list_is_the_default_destination_and_a_new_day_is_not() {
        let mut l = Lists::default();
        assert_eq!(default_destination(&l, "Sep 14, 2026"), Destination::New, "nothing saved today yet");
        let id = create(&mut l, "Sep 14, 2026", ListSource::Photo, T0);
        assert_eq!(default_destination(&l, "Sep 14, 2026"), Destination::Add(id.clone()),
            "a second page of the same worksheet joins the first");
        assert_eq!(default_destination(&l, "Sep 21, 2026"), Destination::New, "next week starts its own list");
        soft_delete(&mut l, &id, T0);
        assert_eq!(default_destination(&l, "Sep 14, 2026"), Destination::New, "a deleted list is not a destination");
    }

    // ---- D2 editing ----

    #[test]
    fn an_opened_list_can_be_edited() {
        let (mut l, a) = with_list("A", T0);
        add_entries(&mut l, &a, &[("becuase".into(), "en".into()), ("dog".into(), "en".into())], T0);
        assert!(edit_entry(&mut l, &a, 0, "because", "en", T0), "a misread is fixable");
        assert_eq!(l.lists[0].entries[0].text, "because");
        assert!(!edit_entry(&mut l, &a, 0, "dog", "en", T0), "an edit may not duplicate another entry");
        assert!(!edit_entry(&mut l, &a, 0, "  ", "en", T0), "and may not empty it");
        let gone = remove_entry(&mut l, &a, 1, T0).expect("removed");
        assert_eq!(l.lists[0].entries.len(), 1);
        assert!(insert_entry(&mut l, &a, 1, gone, T0), "Undo puts it back where it was");
        assert_eq!(l.lists[0].entries[1].text, "dog");
        let b = create(&mut l, "B", ListSource::Manual, T0);
        assert!(copy_entry(&mut l, &a, 0, &b, T0));
        assert_eq!(l.lists[0].entries.len(), 2, "copying leaves the source alone");
        assert_eq!(l.lists[1].entries.len(), 1);
    }

    // ---- D3 order ----

    #[test]
    fn my_order_leads_with_starred_words_and_mixed_keeps_saved_order() {
        let (mut l, a) = with_list("A", T0);
        add_entries(&mut l, &a, &[("one".into(), "en".into()), ("two".into(), "en".into()), ("three".into(), "en".into())], T0);
        assert_eq!(play_entries(&l, &[a.clone()]).iter().map(|e| e.text.clone()).collect::<Vec<_>>(),
            vec!["one", "two", "three"], "Mixed hands over saved order");
        set_order(&mut l, &a, Order::InMyOrder, T0);
        move_entry(&mut l, &a, 2, 0, T0);
        set_star(&mut l, &a, 2, true, T0); // "two" after the move
        let got: Vec<String> = play_entries(&l, &[a.clone()]).iter().map(|e| e.text.clone()).collect();
        assert_eq!(got, vec!["two", "three", "one"], "starred first, then the player's order");
    }

    // ---- F4 selection ----

    #[test]
    fn play_falls_back_to_the_newest_list_when_the_remembered_one_is_gone() {
        let mut l = Lists::default();
        let old = create(&mut l, "Sep 7", ListSource::Photo, T0);
        let new = create(&mut l, "Sep 14", ListSource::Photo, T0 + DAY_MS * 7.0);
        remember_selection(&mut l, &[old.clone()]);
        assert_eq!(play_selection(&l), vec![old.clone()]);
        soft_delete(&mut l, &old, T0 + DAY_MS * 8.0);
        assert_eq!(play_selection(&l), vec![new], "AT4.2: the newest live list");
    }

    #[test]
    fn playing_two_lists_deduplicates_across_them() {
        let mut l = Lists::default();
        let a = create(&mut l, "A", ListSource::Photo, T0);
        let b = create(&mut l, "B", ListSource::Photo, T0 + 1.0);
        add_entries(&mut l, &a, &[("cat".into(), "en".into()), ("dog".into(), "en".into())], T0);
        add_entries(&mut l, &b, &[("cat".into(), "en".into()), ("fox".into(), "en".into())], T0);
        let got: Vec<String> = play_entries(&l, &[a, b]).iter().map(|e| e.text.clone()).collect();
        assert_eq!(got, vec!["cat", "dog", "fox"]);
    }

    // ---- F3 reading ----

    #[test]
    fn cards_are_newest_first_and_all_words_is_a_deduplicated_union() {
        let mut l = Lists::default();
        let sep1 = create(&mut l, "Sep 1", ListSource::Photo, T0);
        let sep14 = create(&mut l, "Sep 14", ListSource::Photo, T0 + DAY_MS * 13.0);
        let sep7 = create(&mut l, "Sep 7", ListSource::Photo, T0 + DAY_MS * 6.0);
        add_entries(&mut l, &sep1, &[("cat".into(), "en".into())], T0);
        add_entries(&mut l, &sep7, &[("gato".into(), "es".into()), ("cat".into(), "en".into())], T0);
        add_entries(&mut l, &sep14, &[("dog".into(), "en".into())], T0);
        assert_eq!(visible(&l).iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), vec!["Sep 14", "Sep 7", "Sep 1"]);
        let all: Vec<String> = all_words(&l).iter().map(|e| format!("{}:{}", e.lang, e.text)).collect();
        assert_eq!(all, vec!["en:cat", "en:dog", "es:gato"], "AT3.2: one cat, sorted within each language");
    }

    #[test]
    fn language_chips_cap_with_a_remainder() {
        let (mut l, a) = with_list("A", T0);
        for (i, lang) in ["en", "es", "fr", "de", "ja", "ko"].iter().enumerate() {
            add_entries(&mut l, &a, &[(format!("w{i}"), (*lang).to_string())], T0);
        }
        let (chips, more) = language_chips(&l.lists[0], 4);
        assert_eq!(chips, vec!["en", "es", "fr", "de"]);
        assert_eq!(more, 2, "AT3.3: +2");
    }

    #[test]
    fn older_lists_fold_away_instead_of_being_deleted() {
        let mut l = Lists::default();
        let now = T0 + DAY_MS * 40.0;
        create(&mut l, "old", ListSource::Photo, T0);
        create(&mut l, "fresh", ListSource::Photo, now - DAY_MS);
        let (recent, older) = folded(&l, now, 21.0);
        assert_eq!(recent.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), vec!["fresh"]);
        assert_eq!(older.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), vec!["old"]);
    }

    // ---- F5 delete / undo / purge ----

    #[test]
    fn delete_is_recoverable_and_invisible_meanwhile() {
        let (mut l, a) = with_list("A", T0);
        add_entries(&mut l, &a, &[("cat".into(), "en".into())], T0);
        let before = l.clone();
        remember_selection(&mut l, &[a.clone()]);
        assert!(soft_delete(&mut l, &a, T0 + 1.0));
        assert!(visible(&l).is_empty(), "AT5.2: absent from My Words");
        assert!(all_words(&l).is_empty(), "and from All words");
        assert!(play_entries(&l, &[a.clone()]).is_empty(), "and from play");
        assert!(restore(&mut l, &a));
        let mut after = l.clone();
        after.selection = before.selection.clone();
        assert_eq!(after, before, "AT5.1: Undo restores the list exactly");
    }

    #[test]
    fn recently_deleted_purges_after_thirty_days() {
        let (mut l, a) = with_list("A", T0);
        soft_delete(&mut l, &a, T0);
        assert_eq!(purge_expired(&mut l, T0 + DAY_MS * 29.0), 0, "still restorable");
        assert_eq!(recently_deleted(&l).len(), 1);
        assert_eq!(purge_expired(&mut l, T0 + DAY_MS * 31.0), 1, "AT5.3");
        assert!(l.lists.is_empty());
    }

    #[test]
    fn delete_now_only_touches_a_soft_deleted_list() {
        let (mut l, a) = with_list("A", T0);
        assert!(!delete_now(&mut l, &a), "I2: no hard delete without a soft delete first");
        soft_delete(&mut l, &a, T0);
        assert!(delete_now(&mut l, &a));
        assert!(l.lists.is_empty());
    }

    // ---- F6 migration ----

    #[test]
    fn migration_is_lossless() {
        let c = custom(&["cat", "dog", "fox"], "en-US");
        let mut l = Lists::default();
        assert!(migrate_flat(&mut l, &c, "Saved before Sep 14, 2026", T0));
        assert_eq!(l.lists.len(), 1);
        let got: Vec<String> = l.lists[0].entries.iter().map(|e| e.text.clone()).collect();
        assert_eq!(got, vec!["cat", "dog", "fox"], "AT6.1: same order, same words");
        assert_eq!(l.lists[0].source, ListSource::Migrated);
        assert!(l.lists[0].entries.iter().all(|e| e.lang == "en-US"), "the set's speak language carries over");
    }

    #[test]
    fn migration_keeps_each_words_own_language() {
        let mut c = custom(&["cat", "gato"], "en-US");
        c.word_lang.insert("gato".into(), "es-ES".into());
        let mut l = Lists::default();
        migrate_flat(&mut l, &c, "Saved before", T0);
        assert_eq!(l.lists[0].entries[1].lang, "es-ES");
    }

    #[test]
    fn migration_runs_once_and_survives_an_empty_set() {
        let c = custom(&["cat"], "en-US");
        let mut l = Lists::default();
        migrate_flat(&mut l, &c, "Saved before", T0);
        let after_first = l.clone();
        assert!(!migrate_flat(&mut l, &c, "Saved before", T0 + 1.0), "AT6.2: the marker stops it");
        assert_eq!(l, after_first);

        let mut empty = Lists::default();
        assert!(!migrate_flat(&mut empty, &custom(&[], "en-US"), "Saved before", T0));
        assert!(empty.lists.is_empty(), "AT6.4: no list, no crash");
        assert!(empty.migrated, "and it does not run again tomorrow");
    }
}
