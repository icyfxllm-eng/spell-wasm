//! CC-ACCOUNTS Phase 3 -- the pure half of sync. No DOM, no network, no storage.
//!
//! What syncs: My Words lists, the missed-words queue with its review schedule, and the earned
//! themes. Each is a set of records with a global id; the merge is last-write-wins per record by
//! `updated_at`, deletes travel as tombstones, and nothing here ever removes a local record the
//! server did not explicitly tombstone (I5).
//!
//! Two device-local facts shape the design:
//!  * List ids are `l1`, `l2`, ... per device, so two devices both own an `l1`. A list therefore
//!    syncs under a GLOBAL id kept in `Meta::list_ids` (or `k:<key>` for a list with a caller-owned
//!    key, so the Guardian's weekly list cannot be created twice). The list model is untouched.
//!  * Most records carry no edit time of their own, so a change is found by comparing a content
//!    hash with the one last synced, and stamped with the clock at that moment.
//!
//! Not synced in this phase, on purpose: the Daily streak (its store is Daily Challenge state,
//! which this work may not touch) and device settings. Both are listed in the report.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::MissEntry;
use crate::word_lists::{Lists, WordList};

pub const KIND_LIST: &str = "list";
pub const KIND_MISS: &str = "miss";
pub const KIND_THEMES: &str = "themes";
pub const THEMES_ID: &str = "themes";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SyncRecord {
    pub kind: String,
    pub record_id: String,
    pub payload: Option<String>,
    pub updated_at: f64,
    pub deleted: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Known {
    pub ts: f64,
    pub hash: String,
}

/// Kept between cycles under one account-owned localStorage key. Holds no progress itself.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Meta {
    /// The server's pull cursor. Advanced only after a whole cycle succeeded.
    #[serde(default)]
    pub cursor: i64,
    /// `kind|record_id` -> what the server last held for it.
    #[serde(default)]
    pub known: BTreeMap<String, Known>,
    /// Local list id -> global id.
    #[serde(default)]
    pub list_ids: BTreeMap<String, String>,
}

pub fn kkey(kind: &str, id: &str) -> String {
    format!("{kind}|{id}")
}

/// FNV-1a, 64-bit, as hex. Change detection only; never a security measure.
pub fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// The edit time a record goes out with. Unchanged since the last sync: that sync's time. Changed:
/// the record's own time when it moved past the last sync, otherwise now.
fn stamp(item_ts: f64, payload: &str, known: Option<&Known>, now: f64) -> f64 {
    match known {
        Some(k) if k.hash == fnv(payload) => k.ts,
        Some(k) => {
            if item_ts > k.ts {
                item_ts
            } else {
                now
            }
        }
        None => {
            if item_ts > 0.0 {
                item_ts
            } else {
                now
            }
        }
    }
}

fn effective_ts(l: &WordList) -> f64 {
    l.updated_at.max(l.deleted_at.unwrap_or(0.0))
}

/// The global id for a local list, minting one the first time it is seen.
pub fn list_gid(l: &WordList, meta: &mut Meta, gen: &mut dyn FnMut() -> String) -> String {
    if let Some(k) = &l.key {
        return format!("k:{k}");
    }
    if let Some(g) = meta.list_ids.get(&l.id) {
        return g.clone();
    }
    let g = gen();
    meta.list_ids.insert(l.id.clone(), g.clone());
    g
}

/// Every list, a soft-deleted one included: "Recently deleted" must be restorable on every device,
/// so it travels as a live record carrying its `deletedAt`.
pub fn collect_lists(lists: &Lists, meta: &mut Meta, now: f64, gen: &mut dyn FnMut() -> String) -> Vec<SyncRecord> {
    let mut out = Vec::new();
    for l in &lists.lists {
        let gid = list_gid(l, meta, gen);
        let Ok(payload) = serde_json::to_string(l) else { continue };
        let ts = stamp(effective_ts(l), &payload, meta.known.get(&kkey(KIND_LIST, &gid)), now);
        out.push(SyncRecord { kind: KIND_LIST.into(), record_id: gid, payload: Some(payload), updated_at: ts, deleted: false });
    }
    out
}

pub fn collect_misses(misses: &[MissEntry], meta: &Meta, now: f64) -> Vec<SyncRecord> {
    let mut out = Vec::new();
    for m in misses {
        let id = crate::misses::miss_key(&m.word, &m.lang);
        let Ok(payload) = serde_json::to_string(m) else { continue };
        let ts = stamp(m.ts, &payload, meta.known.get(&kkey(KIND_MISS, &id)), now);
        out.push(SyncRecord { kind: KIND_MISS.into(), record_id: id, payload: Some(payload), updated_at: ts, deleted: false });
    }
    out
}

pub fn collect_themes(unlocked: &[String], meta: &Meta, now: f64) -> Vec<SyncRecord> {
    if unlocked.is_empty() {
        return Vec::new();
    }
    let mut sorted: Vec<&String> = unlocked.iter().collect();
    sorted.sort();
    sorted.dedup();
    let payload = serde_json::json!({ "unlocked": sorted }).to_string();
    let ts = stamp(0.0, &payload, meta.known.get(&kkey(KIND_THEMES, THEMES_ID)), now);
    vec![SyncRecord { kind: KIND_THEMES.into(), record_id: THEMES_ID.into(), payload: Some(payload), updated_at: ts, deleted: false }]
}

/// A record the server holds that this device no longer has: the player (or the 30-day purge, or
/// "Delete now") removed it here, so the delete must reach the other devices.
pub fn tombstones(current: &[SyncRecord], meta: &Meta, now: f64) -> Vec<SyncRecord> {
    let have: std::collections::BTreeSet<String> = current.iter().map(|r| kkey(&r.kind, &r.record_id)).collect();
    meta.known
        .keys()
        .filter(|k| !have.contains(*k))
        .filter_map(|k| k.split_once('|'))
        .filter(|(kind, _)| *kind != KIND_THEMES) // earned themes are never un-earned
        .map(|(kind, id)| SyncRecord { kind: kind.into(), record_id: id.into(), payload: None, updated_at: now, deleted: true })
        .collect()
}

/// What this cycle has to send: new records, changed records, tombstones.
pub fn to_push(current: &[SyncRecord], tombs: Vec<SyncRecord>, meta: &Meta) -> Vec<SyncRecord> {
    let mut out: Vec<SyncRecord> = current
        .iter()
        .filter(|r| match meta.known.get(&kkey(&r.kind, &r.record_id)) {
            None => true,
            Some(k) => r.updated_at > k.ts || r.payload.as_deref().map(fnv).as_deref() != Some(k.hash.as_str()),
        })
        .cloned()
        .collect();
    out.extend(tombs);
    out
}

/// The local edit times, by `kind|id`, for the apply step's comparisons.
pub fn ts_map(current: &[SyncRecord]) -> BTreeMap<String, f64> {
    current.iter().map(|r| (kkey(&r.kind, &r.record_id), r.updated_at)).collect()
}

/// Apply pulled list records. Returns the `kind|id` keys that were applied (for `Meta::known`).
pub fn apply_lists(lists: &mut Lists, meta: &mut Meta, local: &BTreeMap<String, f64>, remote: &[SyncRecord]) -> Vec<(String, f64)> {
    let mut applied = Vec::new();
    for r in remote.iter().filter(|r| r.kind == KIND_LIST) {
        let gid = &r.record_id;
        let local_id = meta
            .list_ids
            .iter()
            .find(|(_, g)| *g == gid)
            .map(|(l, _)| l.clone())
            .or_else(|| gid.strip_prefix("k:").and_then(|k| lists.lists.iter().find(|l| l.key.as_deref() == Some(k)).map(|l| l.id.clone())));
        let local_ts = local_id.as_ref().and_then(|_| local.get(&kkey(KIND_LIST, gid))).copied();
        if r.deleted {
            if let (Some(lid), Some(ts)) = (&local_id, local_ts) {
                if r.updated_at > ts {
                    lists.lists.retain(|l| &l.id != lid);
                    lists.selection.retain(|s| s != lid);
                    meta.list_ids.remove(lid);
                    applied.push((kkey(KIND_LIST, gid), r.updated_at));
                }
            }
            continue;
        }
        let Some(Ok(mut w)) = r.payload.as_deref().map(serde_json::from_str::<WordList>) else { continue };
        match local_id {
            Some(lid) => {
                if local_ts.is_some_and(|ts| r.updated_at <= ts) {
                    continue;
                }
                w.id = lid.clone();
                if let Some(slot) = lists.lists.iter_mut().find(|l| l.id == lid) {
                    *slot = w;
                }
                if lists.lists.iter().all(|l| l.live() || !lists.selection.contains(&l.id)) {
                    lists.selection.retain(|s| lists.lists.iter().any(|l| &l.id == s && l.live()));
                }
            }
            None => {
                lists.next_id += 1;
                let lid = format!("l{}", lists.next_id);
                w.id = lid.clone();
                lists.lists.push(w);
                meta.list_ids.insert(lid, gid.clone());
            }
        }
        applied.push((kkey(KIND_LIST, gid), r.updated_at));
    }
    applied
}

/// Apply pulled miss records. A remote record is never inserted past `cap`: the cap drops entries
/// from the end on save, and an entry lost to the cap must not read as a delete next cycle.
pub fn apply_misses(misses: &mut Vec<MissEntry>, local: &BTreeMap<String, f64>, remote: &[SyncRecord], cap: usize) -> Vec<(String, f64)> {
    let mut applied = Vec::new();
    for r in remote.iter().filter(|r| r.kind == KIND_MISS) {
        let key = kkey(KIND_MISS, &r.record_id);
        let at = misses.iter().position(|m| crate::misses::miss_key(&m.word, &m.lang) == r.record_id);
        if r.deleted {
            if let (Some(i), Some(ts)) = (at, local.get(&key)) {
                if r.updated_at > *ts {
                    misses.remove(i);
                    applied.push((key, r.updated_at));
                }
            }
            continue;
        }
        let Some(Ok(m)) = r.payload.as_deref().map(serde_json::from_str::<MissEntry>) else { continue };
        match at {
            Some(i) => {
                if local.get(&key).is_some_and(|ts| r.updated_at <= *ts) {
                    continue;
                }
                misses[i] = m;
            }
            None => {
                if misses.len() >= cap {
                    continue;
                }
                misses.insert(0, m);
            }
        }
        applied.push((key, r.updated_at));
    }
    applied
}

/// Earned themes only ever grow, so they merge by union rather than by clock.
pub fn apply_themes(unlocked: &mut Vec<String>, remote: &[SyncRecord]) -> bool {
    let mut changed = false;
    for r in remote.iter().filter(|r| r.kind == KIND_THEMES && !r.deleted) {
        let Some(p) = r.payload.as_deref().and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok()) else { continue };
        for t in p.get("unlocked").and_then(|u| u.as_array()).into_iter().flatten().filter_map(|t| t.as_str()) {
            if !unlocked.iter().any(|u| u == t) {
                unlocked.push(t.to_string());
                changed = true;
            }
        }
    }
    changed
}

/// After a successful cycle: remember what the server now holds, so only real changes go out next time.
pub fn commit(meta: &mut Meta, pushed: &[SyncRecord], pulled: &[SyncRecord], applied: &[(String, f64)], after: &[SyncRecord], cursor: i64) {
    for r in pushed {
        let k = kkey(&r.kind, &r.record_id);
        match (&r.payload, r.deleted) {
            (Some(p), false) => {
                meta.known.insert(k, Known { ts: r.updated_at, hash: fnv(p) });
            }
            _ => {
                meta.known.remove(&k);
            }
        }
    }
    for (k, ts) in applied {
        match after.iter().find(|r| kkey(&r.kind, &r.record_id) == *k) {
            Some(r) => {
                meta.known.insert(k.clone(), Known { ts: *ts, hash: fnv(r.payload.as_deref().unwrap_or("")) });
            }
            None => {
                meta.known.remove(k);
            }
        }
    }
    // Themes merge by union, so what we hold may be MORE than the server does. Remember what the server
    // holds, not what we hold now: the difference is exactly what has to go out next cycle.
    if let Some(r) = pulled.iter().rev().find(|r| r.kind == KIND_THEMES && !r.deleted) {
        meta.known.insert(
            kkey(KIND_THEMES, THEMES_ID),
            Known { ts: r.updated_at, hash: fnv(r.payload.as_deref().unwrap_or("")) },
        );
    }
    meta.cursor = cursor;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::word_lists::{ListEntry, ListSource, Order};

    fn list(id: &str, name: &str, words: &[&str], updated: f64) -> WordList {
        WordList {
            id: id.into(),
            profile: String::new(),
            name: name.into(),
            created_at: 1.0,
            updated_at: updated,
            source: ListSource::Manual,
            entries: words.iter().map(|w| ListEntry { text: (*w).into(), lang: "en".into(), added_at: 1.0, starred: false }).collect(),
            deleted_at: None,
            order: Order::Mixed,
            key: None,
        }
    }

    fn miss(word: &str, ts: f64) -> MissEntry {
        MissEntry { word: word.into(), lang: "en".into(), tier: "easy".into(), misses: 1, box_: 0, due: ts, ts, review: None }
    }

    /// A stand-in for the service: the same rules as backend/accounts.py's /sync.
    #[derive(Default)]
    struct Server {
        rows: BTreeMap<String, (SyncRecord, i64)>,
        seq: i64,
    }
    impl Server {
        fn sync(&mut self, since: i64, push: &[SyncRecord]) -> (Vec<SyncRecord>, i64) {
            for r in push {
                let k = kkey(&r.kind, &r.record_id);
                if self.rows.get(&k).is_some_and(|(c, _)| c.updated_at >= r.updated_at) {
                    continue;
                }
                self.seq += 1;
                self.rows.insert(k, (r.clone(), self.seq));
            }
            let mut out: Vec<(&SyncRecord, i64)> = self.rows.values().filter(|(_, s)| *s > since).map(|(r, s)| (r, *s)).collect();
            out.sort_by_key(|(_, s)| *s);
            (out.iter().map(|(r, _)| (*r).clone()).collect(), self.seq)
        }
    }

    /// One device: lists, misses, themes, meta, and a counter for global ids.
    struct Device {
        name: &'static str,
        lists: Lists,
        misses: Vec<MissEntry>,
        themes: Vec<String>,
        meta: Meta,
        n: u32,
    }
    impl Device {
        fn new(name: &'static str) -> Self {
            Device { name, lists: Lists::default(), misses: vec![], themes: vec![], meta: Meta::default(), n: 0 }
        }
        fn current(&mut self, now: f64) -> Vec<SyncRecord> {
            let name = self.name;
            let mut n = self.n;
            let mut gen = || {
                n += 1;
                format!("g-{name}-{n}")
            };
            let mut out = collect_lists(&self.lists, &mut self.meta, now, &mut gen);
            self.n = n;
            out.extend(collect_misses(&self.misses, &self.meta, now));
            out.extend(collect_themes(&self.themes, &self.meta, now));
            out
        }
        fn cycle(&mut self, server: &mut Server, now: f64) {
            let cur = self.current(now);
            let tombs = tombstones(&cur, &self.meta, now);
            let push = to_push(&cur, tombs, &self.meta);
            let (pulled, cursor) = server.sync(self.meta.cursor, &push);
            let local = ts_map(&cur);
            let mut applied = apply_lists(&mut self.lists, &mut self.meta, &local, &pulled);
            applied.extend(apply_misses(&mut self.misses, &local, &pulled, 300));
            apply_themes(&mut self.themes, &pulled);
            let after = self.current(now);
            commit(&mut self.meta, &push, &pulled, &applied, &after, cursor);
        }
        fn names(&self) -> Vec<String> {
            let mut v: Vec<String> = self.lists.lists.iter().filter(|l| l.live()).map(|l| l.name.clone()).collect();
            v.sort();
            v
        }
    }

    #[test]
    fn two_devices_each_with_an_l1_keep_both_lists() {
        let mut s = Server::default();
        let (mut a, mut b) = (Device::new("a"), Device::new("b"));
        a.lists.lists.push(list("l1", "Animals", &["cat", "dog"], 10.0));
        a.lists.next_id = 1;
        b.lists.lists.push(list("l1", "Food", &["pie"], 11.0));
        b.lists.next_id = 1;
        a.cycle(&mut s, 100.0);
        b.cycle(&mut s, 101.0);
        a.cycle(&mut s, 102.0);
        assert_eq!(a.names(), vec!["Animals", "Food"]);
        assert_eq!(b.names(), vec!["Animals", "Food"]);
        // No duplicates on a second round, and nothing churns.
        a.cycle(&mut s, 103.0);
        b.cycle(&mut s, 104.0);
        assert_eq!(a.lists.lists.len(), 2);
        assert_eq!(b.lists.lists.len(), 2);
    }

    #[test]
    fn two_hundred_local_words_survive_sign_up_exactly_once() {
        let mut s = Server::default();
        let mut a = Device::new("a");
        let words: Vec<String> = (0..200).map(|i| format!("word{i}")).collect();
        let refs: Vec<&str> = words.iter().map(String::as_str).collect();
        a.lists.lists.push(list("l1", "Big", &refs, 5.0));
        a.lists.next_id = 1;
        for t in 0..4 {
            a.cycle(&mut s, 100.0 + t as f64);
        }
        assert_eq!(a.lists.lists.len(), 1);
        assert_eq!(a.lists.lists[0].entries.len(), 200);
        let mut b = Device::new("b");
        b.cycle(&mut s, 200.0);
        assert_eq!(b.lists.lists[0].entries.len(), 200);
    }

    #[test]
    fn an_edit_travels_and_the_newer_edit_wins() {
        let mut s = Server::default();
        let (mut a, mut b) = (Device::new("a"), Device::new("b"));
        a.lists.lists.push(list("l1", "Animals", &["cat"], 10.0));
        a.lists.next_id = 1;
        a.cycle(&mut s, 100.0);
        b.cycle(&mut s, 101.0);
        // both edit; b edits later
        a.lists.lists[0].name = "Pets".into();
        a.lists.lists[0].updated_at = 200.0;
        let blid = b.lists.lists[0].id.clone();
        b.lists.lists[0].name = "Zoo".into();
        b.lists.lists[0].updated_at = 300.0;
        assert_eq!(blid, "l1");
        a.cycle(&mut s, 210.0);
        b.cycle(&mut s, 310.0);
        a.cycle(&mut s, 320.0);
        assert_eq!(a.names(), vec!["Zoo"]);
        assert_eq!(b.names(), vec!["Zoo"]);
    }

    #[test]
    fn a_hard_delete_travels_as_a_tombstone_and_a_replay_changes_nothing() {
        let mut s = Server::default();
        let (mut a, mut b) = (Device::new("a"), Device::new("b"));
        a.lists.lists.push(list("l1", "Gone", &["x"], 10.0));
        a.lists.next_id = 1;
        a.cycle(&mut s, 100.0);
        b.cycle(&mut s, 101.0);
        assert_eq!(b.names(), vec!["Gone"]);
        // a deletes it for good (the 30-day purge / "Delete now")
        a.lists.lists.clear();
        a.cycle(&mut s, 200.0);
        b.cycle(&mut s, 201.0);
        assert!(b.lists.lists.is_empty(), "the delete did not reach the other device");
        let rows_before = s.rows.len();
        a.cycle(&mut s, 300.0);
        b.cycle(&mut s, 301.0);
        assert!(a.lists.lists.is_empty() && b.lists.lists.is_empty());
        assert_eq!(s.rows.len(), rows_before);
    }

    #[test]
    fn soft_delete_and_restore_both_travel() {
        let mut s = Server::default();
        let (mut a, mut b) = (Device::new("a"), Device::new("b"));
        a.lists.lists.push(list("l1", "Trash me", &["x"], 10.0));
        a.lists.next_id = 1;
        a.cycle(&mut s, 100.0);
        b.cycle(&mut s, 101.0);
        a.lists.lists[0].deleted_at = Some(150.0);
        a.cycle(&mut s, 160.0);
        b.cycle(&mut s, 161.0);
        assert!(b.lists.lists[0].deleted_at.is_some(), "recently-deleted state did not travel");
        // restored on b: updatedAt unchanged, only deletedAt cleared. The hash catches it.
        b.lists.lists[0].deleted_at = None;
        b.cycle(&mut s, 400.0);
        a.cycle(&mut s, 401.0);
        assert!(a.lists.lists[0].deleted_at.is_none(), "the restore did not travel");
    }

    #[test]
    fn a_keyed_list_is_one_list_on_every_device() {
        let mut s = Server::default();
        let (mut a, mut b) = (Device::new("a"), Device::new("b"));
        let mut g = list("l1", "Week", &["a"], 10.0);
        g.key = Some("guardian:0:2026-10-19:en".into());
        a.lists.lists.push(g.clone());
        a.lists.next_id = 1;
        g.updated_at = 11.0;
        b.lists.lists.push(g);
        b.lists.next_id = 1;
        a.cycle(&mut s, 100.0);
        b.cycle(&mut s, 101.0);
        a.cycle(&mut s, 102.0);
        assert_eq!(a.lists.lists.len(), 1);
        assert_eq!(b.lists.lists.len(), 1);
    }

    #[test]
    fn misses_merge_by_word_and_the_cap_never_reads_as_a_delete() {
        let mut s = Server::default();
        let (mut a, mut b) = (Device::new("a"), Device::new("b"));
        a.misses = vec![miss("apple", 10.0), miss("pear", 11.0)];
        b.misses = vec![miss("apple", 20.0)];
        a.cycle(&mut s, 100.0);
        b.cycle(&mut s, 101.0);
        a.cycle(&mut s, 102.0);
        assert_eq!(a.misses.len(), 2);
        assert_eq!(b.misses.len(), 2);
        assert_eq!(a.misses.iter().find(|m| m.word == "apple").unwrap().ts, 20.0);
        // a device already at the cap takes no more, and does not delete what it could not hold
        let mut full = Device::new("full");
        full.misses = (0..300).map(|i| miss(&format!("w{i}"), 1.0)).collect();
        let before = s.rows.len();
        full.cycle(&mut s, 500.0);
        full.cycle(&mut s, 501.0);
        assert_eq!(full.misses.len(), 300);
        assert_eq!(s.rows.values().filter(|(r, _)| r.deleted).count(), 0);
        assert!(s.rows.len() > before);
    }

    #[test]
    fn mastering_a_word_deletes_it_everywhere() {
        let mut s = Server::default();
        let (mut a, mut b) = (Device::new("a"), Device::new("b"));
        a.misses = vec![miss("apple", 10.0)];
        a.cycle(&mut s, 100.0);
        b.cycle(&mut s, 101.0);
        a.misses.clear();
        a.cycle(&mut s, 200.0);
        b.cycle(&mut s, 201.0);
        assert!(b.misses.is_empty());
    }

    #[test]
    fn themes_only_grow() {
        let mut s = Server::default();
        let (mut a, mut b) = (Device::new("a"), Device::new("b"));
        a.themes = vec!["dusk".into()];
        b.themes = vec!["ocean".into()];
        a.cycle(&mut s, 100.0);
        b.cycle(&mut s, 101.0);
        a.cycle(&mut s, 102.0); // a learns "ocean" ...
        a.cycle(&mut s, 103.0); // ... and sends the union on its next cycle
        b.cycle(&mut s, 104.0);
        let mut at = a.themes.clone();
        at.sort();
        let mut bt = b.themes.clone();
        bt.sort();
        assert_eq!(at, vec!["dusk", "ocean"]);
        assert_eq!(bt, at);
        // an empty device never un-earns anything
        let mut c = Device::new("c");
        c.cycle(&mut s, 104.0);
        assert!(tombstones(&c.current(105.0), &c.meta, 105.0).is_empty());
    }

    #[test]
    fn an_interrupted_cycle_loses_nothing_and_the_retry_converges() {
        let mut s = Server::default();
        let (mut a, mut b) = (Device::new("a"), Device::new("b"));
        a.lists.lists.push(list("l1", "Animals", &["cat"], 10.0));
        a.lists.next_id = 1;
        a.cycle(&mut s, 100.0);
        // b's cycle dies after the server answered but before anything was applied or committed
        let cur = b.current(101.0);
        let tombs = tombstones(&cur, &b.meta, 101.0);
        let push = to_push(&cur, tombs, &b.meta);
        let _ = s.sync(b.meta.cursor, &push);
        assert!(b.lists.lists.is_empty() && b.meta.cursor == 0, "nothing was committed");
        b.cycle(&mut s, 102.0);
        assert_eq!(b.names(), vec!["Animals"]);
        b.cycle(&mut s, 103.0);
        assert_eq!(b.lists.lists.len(), 1);
    }

    #[test]
    fn unchanged_records_are_not_resent() {
        let mut s = Server::default();
        let mut a = Device::new("a");
        a.misses = vec![miss("apple", 10.0)];
        a.cycle(&mut s, 100.0);
        let cur = a.current(200.0);
        let tombs = tombstones(&cur, &a.meta, 200.0);
        assert!(to_push(&cur, tombs, &a.meta).is_empty());
    }
}
