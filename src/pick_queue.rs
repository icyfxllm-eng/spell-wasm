//! CC-PICK-QUEUE v1 Phase A -- the lineup: tap words to practice them first.
//!
//! One model, one store, no DOM. Misses and My Words both read and write the
//! SAME `Lineups` (D10/I8); there is no per-language branch anywhere in here.
//! A lineup is an ORDER overlay on the queue that already exists. It never
//! grades, scores, schedules or graduates anything (I1): the serve guards below
//! only choose WHICH legal word comes next, and fall through to today's code
//! the moment there is nothing to serve (I4/A2).
//!
//! Persistence (D5): its own key, `spell_lineup_v1`, written through on every
//! change. No existing store is touched, so no schema bump and no ship-order
//! risk. A blob written by a NEWER build is kept byte-for-byte and never
//! overwritten (the learner.rs `Loaded::Newer` rule).
//!
//! What clears a lineup (and only this): the player explicitly ending the
//! session ([`clear_scope`], called from the End control and the Misses exit),
//! or Reset-this-language ([`clear_lang`]). Sign-out does not. A pick whose
//! word graduates or is removed simply stops existing for the lineup
//! ([`Lineups::prune`], O-C / O-D).
//!
//! The whole module sits behind `flags::pick_queue_a()`, default OFF, and the
//! flag is permanently false in the site build (O-F).

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;

pub const KEY: &str = "spell_lineup_v1";
pub const VERSION: u32 = 1;

pub const SURFACE_MISSES: &str = "misses";
pub const SURFACE_MINE: &str = "mine";

/// One lined-up word. `key` is the word's `word_id0` identity in its own
/// language, `word` the text the queue serves, `lang` the word's language.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Pick {
    pub key: String,
    pub word: String,
    pub lang: String,
}

/// One lineup: ONE list in ONE language (D9). `picks` is tap order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Scope {
    /// "misses" or "mine".
    pub surface: String,
    /// The My Words list id; empty for Misses.
    #[serde(default)]
    pub list: String,
    pub lang: String,
    pub picks: Vec<Pick>,
    #[serde(default)]
    pub just_these: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Lineups {
    pub v: u32,
    #[serde(default)]
    pub scopes: BTreeMap<String, Scope>,
}

impl Default for Lineups {
    fn default() -> Self {
        Lineups { v: VERSION, scopes: BTreeMap::new() }
    }
}

/// The scope id for a surface/list/language triple. Two languages can never
/// share one (D9, O-H).
pub fn scope_id(surface: &str, list: &str, lang: &str) -> String {
    format!("{surface}|{list}|{lang}")
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Toggled {
    /// Lined up; the number is its 1-based badge.
    Added(usize),
    Removed,
    /// Jr's cap (I7): the pick was refused, nothing changed.
    Refused,
}

impl Lineups {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Tolerant reader. The bool is "this blob is from a newer build": the
    /// caller must behave as empty and never write it back.
    pub fn from_json(raw: &str) -> (Lineups, bool) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
            return (Lineups::default(), false);
        };
        let ver = v.get("v").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        if ver > VERSION {
            return (Lineups::default(), true);
        }
        (serde_json::from_value::<Lineups>(v).unwrap_or_default(), false)
    }

    pub fn picks(&self, scope: &str) -> &[Pick] {
        self.scopes.get(scope).map(|s| s.picks.as_slice()).unwrap_or(&[])
    }

    /// 1-based badge number of `key`, in tap order with no gaps (I5).
    pub fn number_of(&self, scope: &str, key: &str) -> Option<usize> {
        self.picks(scope).iter().position(|p| p.key == key).map(|i| i + 1)
    }

    pub fn just_these(&self, scope: &str) -> bool {
        self.scopes.get(scope).map(|s| s.just_these).unwrap_or(false)
    }

    /// Tap: add if absent, remove if present. `cap` is Jr's five (a SIXTH is
    /// refused, but removing is always allowed).
    pub fn toggle(&mut self, id: &str, surface: &str, list: &str, lang: &str, pick: Pick, cap: Option<usize>) -> Toggled {
        let sc = self.scopes.entry(id.to_string()).or_insert_with(|| Scope {
            surface: surface.to_string(),
            list: list.to_string(),
            lang: lang.to_string(),
            picks: Vec::new(),
            just_these: false,
        });
        if let Some(i) = sc.picks.iter().position(|p| p.key == pick.key) {
            sc.picks.remove(i);
            if sc.picks.is_empty() {
                self.scopes.remove(id);
            }
            return Toggled::Removed;
        }
        if let Some(c) = cap {
            if sc.picks.len() >= c {
                if sc.picks.is_empty() {
                    self.scopes.remove(id);
                }
                return Toggled::Refused;
            }
        }
        sc.picks.push(pick);
        Toggled::Added(sc.picks.len())
    }

    pub fn set_just_these(&mut self, id: &str, on: bool) {
        if let Some(s) = self.scopes.get_mut(id) {
            s.just_these = on;
        }
    }

    /// Drop picks whose word no longer exists (graduated, removed, evicted).
    /// Returns true if anything changed.
    pub fn prune(&mut self, id: &str, live: impl Fn(&Pick) -> bool) -> bool {
        let Some(s) = self.scopes.get_mut(id) else { return false };
        let before = s.picks.len();
        s.picks.retain(|p| live(p));
        let changed = s.picks.len() != before;
        if s.picks.is_empty() {
            self.scopes.remove(id);
        }
        changed
    }

    pub fn clear(&mut self, id: &str) -> bool {
        self.scopes.remove(id).is_some()
    }

    /// Reset-this-language (O-E): that language's lineups go; nobody else's.
    /// `MINE` (the My Words study language) stands for every My Words list.
    pub fn clear_lang(&mut self, lang: &str) -> bool {
        let before = self.scopes.len();
        let mine = lang == crate::consts::MINE;
        self.scopes.retain(|_, s| !(s.lang == lang || (mine && s.surface == SURFACE_MINE)));
        self.scopes.len() != before
    }

    /// Total picks across every scope (diagnostics and tests).
    pub fn total(&self) -> usize {
        self.scopes.values().map(|s| s.picks.len()).sum()
    }
}

// ---------- the queue guard (pure) ----------

#[derive(Debug, PartialEq, Clone)]
pub enum Serve {
    /// Serve this pick next.
    Pick(Pick),
    /// "Just these" and every pick has been served: the session is over (A9).
    Exhausted,
    /// Nothing for the lineup to say: run today's queue untouched (A2).
    Open,
}

/// The one ordering rule (I2/I3): the first pick, in tap order, that has not
/// been served this session and still exists. When there is none, "Just these"
/// ends the session; otherwise the normal queue takes over.
pub fn decide(picks: &[Pick], served: &[String], just_these: bool, live: impl Fn(&Pick) -> bool) -> Serve {
    decide_with_stars(picks, &[], served, just_these, live)
}

/// CC-PICK-QUEUE Phase B -- the ONE ordering rule when lineup picks and focus
/// stars both exist (I2/I3):
///
/// 1. the lineup's picks, in tap order;
/// 2. then the stars that are not already picked, in the order they were
///    starred (a star that is also a pick was served as a pick and is in
///    `served`, so it never comes round twice);
/// 3. then the normal queue (`Open`).
///
/// "Just these" means just the picks: once they are served the session ends and
/// stars do not extend it. With no stars this is `decide` exactly.
pub fn decide_with_stars(
    picks: &[Pick],
    stars: &[Pick],
    served: &[String],
    just_these: bool,
    live: impl Fn(&Pick) -> bool,
) -> Serve {
    if let Some(p) = picks.iter().find(|p| !served.contains(&p.key) && live(p)) {
        return Serve::Pick(p.clone());
    }
    if just_these {
        return Serve::Exhausted;
    }
    if let Some(p) = stars.iter().find(|p| !served.contains(&p.key) && live(p)) {
        return Serve::Pick(p.clone());
    }
    Serve::Open
}

// ---------- runtime state ----------

struct Store {
    lineups: Lineups,
    /// Written by a newer build: read as empty, never overwritten.
    newer: bool,
    loaded: bool,
}

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store { lineups: Lineups::default(), newer: false, loaded: false });
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

struct Session {
    scope: String,
    just_these: bool,
    served: Vec<String>,
    /// The same words by text, for the deck fall-through (I3).
    served_words: Vec<String>,
    /// The word the lineup served last, so a correct answer can be told apart
    /// from an ordinary serve (D3).
    last_word: Option<String>,
    /// Phase B: the language whose Misses stars lead this session. Empty for a
    /// My Words session (its stars come from the lists, each in its own language).
    star_lang: String,
}

fn ensure_loaded(s: &mut Store) {
    if s.loaded {
        return;
    }
    s.loaded = true;
    if let Some(raw) = crate::storage::get_raw(KEY) {
        let (l, newer) = Lineups::from_json(&raw);
        s.lineups = l;
        s.newer = newer;
    }
}

fn save(s: &Store) {
    if s.newer {
        return;
    }
    crate::storage::set_json(KEY, &s.lineups);
}

/// Read the lineups.
pub fn read<R>(f: impl FnOnce(&Lineups) -> R) -> R {
    STORE.with(|c| {
        let mut s = c.borrow_mut();
        ensure_loaded(&mut s);
        f(&s.lineups)
    })
}

/// Change the lineups; written through to storage on every call (A7).
pub fn write<R>(f: impl FnOnce(&mut Lineups) -> R) -> R {
    STORE.with(|c| {
        let mut s = c.borrow_mut();
        ensure_loaded(&mut s);
        let r = f(&mut s.lineups);
        save(&s);
        r
    })
}

/// Test seam: replace the in-memory store (host tests have no localStorage).
#[cfg(test)]
pub fn set_for_test(l: Lineups) {
    STORE.with(|c| *c.borrow_mut() = Store { lineups: l, newer: false, loaded: true });
    SESSION.with(|c| *c.borrow_mut() = None);
}

pub fn enabled() -> bool {
    crate::flags::pick_queue_a()
}

pub fn toggle(id: &str, surface: &str, list: &str, lang: &str, pick: Pick, kid: bool) -> Toggled {
    let cap = crate::experience::pick_caps(crate::experience::of_kid(kid)).max_picks;
    write(|l| l.toggle(id, surface, list, lang, pick, cap))
}

/// Sign-out deliberately has no counterpart here (O-B).
pub fn clear_lang(lang: &str) {
    if !enabled() {
        return;
    }
    if read(|l| l.scopes.values().any(|s| s.lang == lang || (lang == crate::consts::MINE && s.surface == SURFACE_MINE))) {
        write(|l| l.clear_lang(lang));
    }
    // A session over a cleared lineup has nothing left to serve.
    if let Some(sc) = session_scope() {
        if read(|l| l.scopes.get(&sc).is_none()) {
            end_session();
        }
    }
}

// ---------- the session ----------

/// Whether a stored "Just these" flag takes effect for this player. Junior has
/// no such control (D4), so a persisted true is ignored, never rewritten.
pub fn effective_just_these(stored: bool, kid: bool) -> bool {
    stored && crate::experience::pick_caps(crate::experience::of_kid(kid)).extras
}

pub fn begin_session(scope: &str, kid: bool) {
    let just = effective_just_these(read(|l| l.just_these(scope)), kid);
    SESSION.with(|c| {
        *c.borrow_mut() = Some(Session {
            scope: scope.to_string(),
            just_these: just,
            served: Vec::new(),
            served_words: Vec::new(),
            last_word: None,
            star_lang: read(|l| l.scopes.get(scope).map(|s| s.lang.clone()).unwrap_or_default()),
        })
    });
}

pub fn end_session() {
    SESSION.with(|c| *c.borrow_mut() = None);
}

pub fn session_scope() -> Option<String> {
    SESSION.with(|c| c.borrow().as_ref().map(|s| s.scope.clone()))
}

/// A LINEUP session is running (not merely a star session, whose scope is empty).
pub fn session_active() -> bool {
    enabled() && SESSION.with(|c| c.borrow().as_ref().map(|s| !s.scope.is_empty()).unwrap_or(false))
}

/// Phase B: a session with no lineup, so focus stars can lead and not repeat.
/// It carries an EMPTY scope, which no lineup ever has, and an existing lineup
/// session is left alone. Stars off (flag, or Spell Jr): nothing is created.
pub fn begin_star_session(kid: bool, lang: &str) {
    if !crate::pick_stars::active(kid) {
        return;
    }
    SESSION.with(|c| {
        let mut b = c.borrow_mut();
        if b.as_ref().map(|s| !s.scope.is_empty()).unwrap_or(false) {
            return;
        }
        *b = Some(Session {
            scope: String::new(),
            just_these: false,
            served: Vec::new(),
            served_words: Vec::new(),
            last_word: None,
            star_lang: lang.to_string(),
        });
    });
}

/// Point a running star session at another language (the Misses sheet's chip).
pub fn set_star_lang(lang: &str) {
    SESSION.with(|c| {
        if let Some(s) = c.borrow_mut().as_mut() {
            if s.scope.is_empty() {
                s.star_lang = lang.to_string();
            }
        }
    });
}

/// Called at the top of every draw: "the word the session served last" must
/// not outlive the serve it described, or a later ordinary answer on the same
/// text would be mistaken for a pick (D3). No session: nothing to do.
pub fn clear_last_served() {
    SESSION.with(|c| {
        if let Some(s) = c.borrow_mut().as_mut() {
            s.last_word = None;
        }
    });
}

/// The player explicitly ended the session: the lineup it ran goes with it.
pub fn clear_session_scope() {
    if let Some(sc) = session_scope().filter(|s| !s.is_empty()) {
        write(|l| l.clear(&sc));
    }
    end_session();
}

/// How many picks the running session's lineup holds right now.
pub fn session_pick_count() -> usize {
    match session_scope() {
        Some(sc) => read(|l| l.picks(&sc).len()),
        None => 0,
    }
}

fn next_from(scope: &str, use_picks: bool, stars: &[Pick], live: impl Fn(&Pick) -> bool) -> Serve {
    // Graduated / removed / evicted words leave the lineup silently (O-C/O-D).
    if !scope.is_empty() {
        write_if_changed(scope, &live);
    }
    let picks: Vec<Pick> = if use_picks && !scope.is_empty() { read(|l| l.picks(scope).to_vec()) } else { Vec::new() };
    let (served, just) = SESSION.with(|c| {
        let b = c.borrow();
        let s = b.as_ref().expect("session");
        (s.served.clone(), s.just_these)
    });
    let r = decide_with_stars(&picks, stars, &served, just && use_picks, &live);
    SESSION.with(|c| {
        if let Some(s) = c.borrow_mut().as_mut() {
            match &r {
                Serve::Pick(p) => {
                    s.served.push(p.key.clone());
                    s.served_words.push(p.word.clone());
                    s.last_word = Some(p.word.clone());
                }
                _ => s.last_word = None,
            }
        }
    });
    r
}

fn write_if_changed(scope: &str, live: &impl Fn(&Pick) -> bool) {
    let stale = read(|l| l.picks(scope).iter().any(|p| !live(p)));
    if stale {
        write(|l| l.prune(scope, live));
    }
}

/// Misses guard. `exists` says whether a pick still has a miss entry behind it.
/// `kid` and `study_lang` are for the focus stars (Phase B): Standard players
/// only, and only the stars of the session's language lead.
pub fn next_for_misses(kid: bool, study_lang: &str, exists: impl Fn(&Pick) -> bool) -> Serve {
    if !enabled() {
        return Serve::Open;
    }
    let Some(sc) = session_scope() else { return Serve::Open };
    if read(|l| l.scopes.get(&sc).map(|s| s.surface != SURFACE_MISSES).unwrap_or(false)) {
        return Serve::Open;
    }
    let stars: Vec<Pick> = if crate::pick_stars::active(kid) {
        let lang = SESSION.with(|c| c.borrow().as_ref().map(|s| s.star_lang.clone())).unwrap_or_default();
        let lang = if lang.is_empty() { study_lang.to_string() } else { lang };
        // A graduated starred miss is pruned here too, silently (O-D).
        crate::pick_stars::prune_misses(&exists);
        crate::pick_stars::read(|s| s.of_lang(&lang))
    } else {
        Vec::new()
    };
    next_from(&sc, true, &stars, exists)
}

/// My Words guard. `list_words` is the pool the draw would use; `selected` is
/// the list(s) the draw is reading. A lineup only speaks for the ONE list it
/// was made in (D9). `starred` is the focus stars of the chosen lists (Phase B,
/// Standard only); `in_order` is "In my order", where the pool already puts
/// starred words first so a second lead would only repeat them.
pub fn next_for_mine(
    kid: bool,
    list_words: &[String],
    in_order: bool,
    selected: impl FnOnce() -> Vec<String>,
    starred: impl FnOnce() -> Vec<Pick>,
) -> Serve {
    if !enabled() {
        return Serve::Open;
    }
    // A My Words draw with no session yet (booting straight into My Words, or
    // the language picker): the stars still lead, so open a star session.
    if session_scope().is_none() {
        begin_star_session(kid, crate::consts::MINE);
    }
    let Some(sc) = session_scope() else { return Serve::Open };
    let lineup_list = match read(|l| l.scopes.get(&sc).map(|s| (s.surface.clone(), s.list.clone()))) {
        Some((surface, list)) if surface == SURFACE_MINE => Some(list),
        Some(_) => return Serve::Open,
        None => None,
    };
    let sel = selected();
    let use_picks = match &lineup_list {
        Some(list) => sel.len() == 1 && sel[0] == *list,
        None => false,
    };
    let stars: Vec<Pick> = if crate::pick_stars::active(kid) && !in_order { starred() } else { Vec::new() };
    if !use_picks && stars.is_empty() {
        return Serve::Open;
    }
    next_from(&sc, use_picks, &stars, |p| list_words.contains(&p.word))
}

/// I3: a word the lineup already served this session must not come round again
/// from the ordinary queue (a miss may; that is a fresh miss, not a repeat).
pub fn already_served(word: &str) -> bool {
    enabled()
        && SESSION.with(|c| {
            c.borrow().as_ref().map(|s| !s.scope.is_empty() && s.served_words.iter().any(|w| w == word)).unwrap_or(false)
        })
}

/// Phase B, I3: a word a STAR already led this My Words session must not come
/// round again from the deck. Separate from `already_served` because a star
/// session can outlive My Words and must never filter another mode's draw; the
/// caller asks only inside the My Words draw.
pub fn star_served(word: &str) -> bool {
    enabled()
        && SESSION.with(|c| {
            c.borrow().as_ref().map(|s| s.scope.is_empty() && s.served_words.iter().any(|w| w == word)).unwrap_or(false)
        })
}

/// D3/I6: was this word just served BY the lineup? Only then does a correct
/// answer on a not-due miss leave its schedule alone.
pub fn served_by_lineup(word: &str) -> bool {
    enabled() && SESSION.with(|c| c.borrow().as_ref().and_then(|s| s.last_word.as_deref().map(|w| w == word)).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pk(w: &str) -> Pick {
        Pick { key: format!("en::{w}"), word: w.to_string(), lang: "en".into() }
    }
    fn sid() -> String {
        scope_id(SURFACE_MISSES, "", "en")
    }

    #[test]
    fn badges_follow_tap_order_and_renumber_without_gaps() {
        let mut l = Lineups::default();
        let id = sid();
        for w in ["cat", "dog", "eel"] {
            l.toggle(&id, SURFACE_MISSES, "", "en", pk(w), None);
        }
        assert_eq!(l.number_of(&id, "en::cat"), Some(1));
        assert_eq!(l.number_of(&id, "en::dog"), Some(2));
        assert_eq!(l.number_of(&id, "en::eel"), Some(3));
        // Tapping a lined-up word removes it; the rest close ranks (I5).
        assert_eq!(l.toggle(&id, SURFACE_MISSES, "", "en", pk("cat"), None), Toggled::Removed);
        assert_eq!(l.number_of(&id, "en::cat"), None);
        assert_eq!(l.number_of(&id, "en::dog"), Some(1));
        assert_eq!(l.number_of(&id, "en::eel"), Some(2));
        let nums: Vec<usize> = l.picks(&id).iter().filter_map(|p| l.number_of(&id, &p.key)).collect();
        assert_eq!(nums, vec![1, 2]);
    }

    #[test]
    fn persistence_round_trip_and_newer_blob() {
        let mut l = Lineups::default();
        let id = sid();
        l.toggle(&id, SURFACE_MISSES, "", "en", pk("cat"), None);
        l.toggle(&id, SURFACE_MISSES, "", "en", pk("dog"), None);
        l.set_just_these(&id, true);
        let (back, newer) = Lineups::from_json(&l.to_json());
        assert!(!newer);
        assert_eq!(back, l);
        // A build from the future: read as empty, flagged so it is never overwritten.
        let (empty, newer) = Lineups::from_json("{\"v\":99,\"scopes\":{\"x\":1}}");
        assert!(newer);
        assert_eq!(empty.total(), 0);
        // Garbage is just empty.
        assert_eq!(Lineups::from_json("not json").0.total(), 0);
    }

    #[test]
    fn jr_cap_refuses_the_sixth_but_always_allows_removal() {
        let mut l = Lineups::default();
        let id = sid();
        for w in ["a", "b", "c", "d", "e"] {
            assert!(matches!(l.toggle(&id, SURFACE_MISSES, "", "en", pk(w), Some(5)), Toggled::Added(_)));
        }
        assert_eq!(l.toggle(&id, SURFACE_MISSES, "", "en", pk("f"), Some(5)), Toggled::Refused);
        assert_eq!(l.picks(&id).len(), 5);
        assert_eq!(l.toggle(&id, SURFACE_MISSES, "", "en", pk("a"), Some(5)), Toggled::Removed);
        assert!(matches!(l.toggle(&id, SURFACE_MISSES, "", "en", pk("f"), Some(5)), Toggled::Added(5)));
    }

    #[test]
    fn standard_has_no_cap_and_jr_has_five_through_one_function() {
        assert_eq!(crate::experience::pick_caps(crate::experience::Experience::Standard).max_picks, None);
        assert_eq!(crate::experience::pick_caps(crate::experience::Experience::Junior).max_picks, Some(5));
    }

    #[test]
    fn jr_never_runs_just_these_even_when_the_store_says_so() {
        assert!(effective_just_these(true, false));
        assert!(!effective_just_these(true, true));
        assert!(!effective_just_these(false, false));
        // The persisted flag is untouched by the kid check.
        let mut l = Lineups::default();
        let id = sid();
        l.toggle(&id, SURFACE_MISSES, "", "en", pk("cat"), None);
        l.set_just_these(&id, true);
        assert!(l.just_these(&id));
        assert!(!effective_just_these(l.just_these(&id), true));
    }

    #[test]
    fn scopes_never_mix_languages_or_lists() {
        let mut l = Lineups::default();
        let en = scope_id(SURFACE_MISSES, "", "en");
        let de = scope_id(SURFACE_MISSES, "", "de");
        let mine_a = scope_id(SURFACE_MINE, "l1", "en");
        let mine_b = scope_id(SURFACE_MINE, "l2", "en");
        l.toggle(&en, SURFACE_MISSES, "", "en", pk("cat"), None);
        l.toggle(&de, SURFACE_MISSES, "", "de", Pick { key: "de::hund".into(), word: "hund".into(), lang: "de".into() }, None);
        l.toggle(&mine_a, SURFACE_MINE, "l1", "en", pk("cat"), None);
        assert_eq!(l.picks(&en).len(), 1);
        assert_eq!(l.picks(&de).len(), 1);
        assert_eq!(l.picks(&mine_a).len(), 1);
        assert!(l.picks(&mine_b).is_empty());
        // Numbering is per scope: the first pick of each is 1.
        assert_eq!(l.number_of(&de, "de::hund"), Some(1));
        // Reset this language clears that language's lineups and no other.
        l.clear_lang("de");
        assert!(l.picks(&de).is_empty());
        assert_eq!(l.picks(&en).len(), 1);
        assert_eq!(l.picks(&mine_a).len(), 1);
        // Resetting the My Words language clears every My Words lineup.
        l.clear_lang(crate::consts::MINE);
        assert!(l.picks(&mine_a).is_empty());
        assert_eq!(l.picks(&en).len(), 1);
    }

    #[test]
    fn guard_serves_lineup_first_in_tap_order_with_no_repeat() {
        let picks = vec![pk("cat"), pk("dog"), pk("eel")];
        let live = |_: &Pick| true;
        let mut served: Vec<String> = Vec::new();
        let mut order = Vec::new();
        while let Serve::Pick(p) = decide(&picks, &served, false, live) {
            served.push(p.key.clone());
            order.push(p.word);
        }
        assert_eq!(order, vec!["cat", "dog", "eel"]); // I2, I3
        // All served and not "Just these": today's queue takes over (A1).
        assert_eq!(decide(&picks, &served, false, live), Serve::Open);
    }

    #[test]
    fn just_these_ends_after_the_picks_and_zero_picks_is_open() {
        let picks = vec![pk("cat"), pk("dog"), pk("eel")];
        let live = |_: &Pick| true;
        let served = vec!["en::cat".to_string(), "en::dog".to_string(), "en::eel".to_string()];
        assert_eq!(decide(&picks, &served, true, live), Serve::Exhausted); // A9
        // Zero picks is byte-identical to today: the guard says nothing (A2).
        assert_eq!(decide(&[], &[], false, live), Serve::Open);
    }

    #[test]
    fn a_graduated_pick_leaves_silently_and_just_these_does_not_loop() {
        let mut l = Lineups::default();
        let id = sid();
        l.toggle(&id, SURFACE_MISSES, "", "en", pk("cat"), None);
        l.toggle(&id, SURFACE_MISSES, "", "en", pk("dog"), None);
        assert!(l.prune(&id, |p| p.word != "cat"));
        assert_eq!(l.number_of(&id, "en::dog"), Some(1));
        assert!(l.prune(&id, |_| false));
        assert!(l.picks(&id).is_empty());
        assert_eq!(decide(l.picks(&id), &[], true, |_| true), Serve::Exhausted);
    }

    #[cfg(not(feature = "web"))] // the site build's flag is permanently off
    #[test]
    fn runtime_session_serves_and_reports_what_it_served() {
        crate::flags::set_test_override(Some("on"));
        let mut l = Lineups::default();
        let id = sid();
        l.toggle(&id, SURFACE_MISSES, "", "en", pk("cat"), None);
        l.toggle(&id, SURFACE_MISSES, "", "en", pk("dog"), None);
        set_for_test(l);
        begin_session(&id, false);
        let live = |_: &Pick| true;
        assert_eq!(next_for_misses(false, "en", live), Serve::Pick(pk("cat")));
        assert!(served_by_lineup("cat"));
        assert!(!served_by_lineup("dog"));
        assert_eq!(next_for_misses(false, "en", live), Serve::Pick(pk("dog")));
        assert_eq!(next_for_misses(false, "en", live), Serve::Open);
        assert!(!served_by_lineup("dog"));
        // Flag OFF: the guard is silent whatever the stored lineup says (dual config).
        crate::flags::set_test_override(Some("off"));
        assert_eq!(next_for_misses(false, "en", live), Serve::Open);
        assert!(!served_by_lineup("cat"));
        crate::flags::set_test_override(None);
    }

    #[cfg(not(feature = "web"))] // the site build's flag is permanently off
    #[test]
    fn my_words_guard_serves_picks_first_only_for_the_one_list_it_was_made_in() {
        crate::flags::set_test_override(Some("on"));
        let mut l = Lineups::default();
        let id = scope_id(SURFACE_MINE, "l1", "en");
        l.toggle(&id, SURFACE_MINE, "l1", "en", pk("dog"), None);
        l.toggle(&id, SURFACE_MINE, "l1", "en", pk("cat"), None);
        set_for_test(l);
        begin_session(&id, false);
        let pool: Vec<String> = ["ant", "cat", "dog", "eel"].iter().map(|s| s.to_string()).collect();
        // A different list is selected: the lineup says nothing (D9).
        assert_eq!(next_for_mine(false, &pool, false, || vec!["l2".into()], Vec::new), Serve::Open);
        assert_eq!(next_for_mine(false, &pool, false, || vec!["l1".into(), "l2".into()], Vec::new), Serve::Open);
        // The right list: tap order, not pool order.
        assert_eq!(next_for_mine(false, &pool, false, || vec!["l1".into()], Vec::new), Serve::Pick(pk("dog")));
        assert_eq!(next_for_mine(false, &pool, false, || vec!["l1".into()], Vec::new), Serve::Pick(pk("cat")));
        assert_eq!(next_for_mine(false, &pool, false, || vec!["l1".into()], Vec::new), Serve::Open);
        assert!(already_served("dog") && already_served("cat") && !already_served("ant"));
        // A word removed from the list is not served (O-D), and just-these ends.
        let mut l = Lineups::default();
        l.toggle(&id, SURFACE_MINE, "l1", "en", pk("gone"), None);
        l.set_just_these(&id, true);
        set_for_test(l);
        begin_session(&id, false);
        assert_eq!(next_for_mine(false, &pool, false, || vec!["l1".into()], Vec::new), Serve::Exhausted);
        // No session at all (zero picks, never started): untouched queue (A2).
        end_session();
        assert_eq!(next_for_mine(false, &pool, false, || vec!["l1".into()], Vec::new), Serve::Open);
        crate::flags::set_test_override(None);
    }

    // ---------- Phase B: focus stars ----------

    fn st(w: &str) -> Pick {
        pk(w)
    }

    /// The ordering rule when both exist: picks (tap order), then stars not
    /// already picked, then the normal queue.
    #[test]
    fn picks_lead_then_stars_then_the_normal_queue() {
        let picks = vec![pk("dog"), pk("cat")];
        let stars = vec![st("cat"), st("eel"), st("fox")]; // cat is both picked and starred
        let live = |_: &Pick| true;
        let mut served: Vec<String> = Vec::new();
        let mut order = Vec::new();
        while let Serve::Pick(p) = decide_with_stars(&picks, &stars, &served, false, live) {
            served.push(p.key.clone());
            order.push(p.word);
        }
        assert_eq!(order, vec!["dog", "cat", "eel", "fox"], "picks in tap order, then unpicked stars; cat once");
        assert_eq!(decide_with_stars(&picks, &stars, &served, false, live), Serve::Open, "then today's queue");
    }

    #[test]
    fn stars_alone_lead_in_the_order_they_were_starred_and_do_not_repeat() {
        let stars = vec![st("fox"), st("ant")];
        let mut served: Vec<String> = Vec::new();
        let mut order = Vec::new();
        while let Serve::Pick(p) = decide_with_stars(&[], &stars, &served, false, |_| true) {
            served.push(p.key.clone());
            order.push(p.word);
        }
        assert_eq!(order, vec!["fox", "ant"]); // I2, I3
    }

    #[test]
    fn just_these_means_just_the_picks_stars_do_not_extend_it() {
        let picks = vec![pk("dog")];
        let stars = vec![st("eel")];
        assert_eq!(decide_with_stars(&picks, &stars, &[], true, |_| true), Serve::Pick(pk("dog")));
        assert_eq!(decide_with_stars(&picks, &stars, &["en::dog".to_string()], true, |_| true), Serve::Exhausted);
    }

    /// A dead star (graduated / removed) is never served.
    #[test]
    fn a_star_whose_word_is_gone_is_skipped() {
        let stars = vec![st("gone"), st("eel")];
        let r = decide_with_stars(&[], &stars, &[], false, |p| p.word != "gone");
        assert_eq!(r, Serve::Pick(st("eel")));
    }

    /// I4: zero stars is `decide` exactly.
    #[test]
    fn zero_stars_is_byte_identical_to_the_lineup_guard() {
        let picks = vec![pk("cat"), pk("dog")];
        for served in [vec![], vec!["en::cat".to_string()], vec!["en::cat".to_string(), "en::dog".to_string()]] {
            for just in [false, true] {
                assert_eq!(
                    decide(&picks, &served, just, |_| true),
                    decide_with_stars(&picks, &[], &served, just, |_| true)
                );
            }
        }
        assert_eq!(decide_with_stars(&[], &[], &[], false, |_| true), Serve::Open);
    }

    #[cfg(not(feature = "web"))]
    #[test]
    fn a_star_session_leads_misses_stars_once_and_gates_the_answer() {
        crate::flags::set_test_override(Some("on"));
        crate::pick_stars::set_for_test(crate::pick_stars::Stars {
            v: 1,
            stars: vec![st("cat"), Pick { key: "de::hund".into(), word: "hund".into(), lang: "de".into() }],
        });
        set_for_test(Lineups::default());
        begin_star_session(false, "en");
        let live = |_: &Pick| true;
        // Only the session's language leads (D9/A11).
        assert_eq!(next_for_misses(false, "en", live), Serve::Pick(st("cat")));
        assert!(served_by_lineup("cat"), "D3: the star-led serve is gated like a pick");
        assert_eq!(next_for_misses(false, "en", live), Serve::Open, "once, then today's queue");
        assert!(!served_by_lineup("cat"));
        // Spell Jr: stars are ignored even though they are persisted (I7).
        begin_star_session(true, "en");
        end_session();
        begin_star_session(false, "en");
        assert_eq!(next_for_misses(true, "en", live), Serve::Open);
        // The star session is not a lineup session: no End control, no "running" copy.
        assert!(!session_active());
        // Flag off: nothing leads and no session is made.
        crate::flags::set_test_override(Some("off"));
        end_session();
        begin_star_session(false, "en");
        assert!(session_scope().is_none());
        assert_eq!(next_for_misses(false, "en", live), Serve::Open);
        crate::flags::set_test_override(None);
    }

    #[cfg(not(feature = "web"))]
    #[test]
    fn my_words_stars_lead_after_picks_skip_in_order_lists_and_never_repeat() {
        crate::flags::set_test_override(Some("on"));
        set_for_test(Lineups::default());
        end_session();
        let pool: Vec<String> = ["ant", "cat", "dog", "eel"].iter().map(|s| s.to_string()).collect();
        let sel = || vec!["l1".to_string()];
        let starred = || vec![st("eel"), st("ant")];
        // No session at all yet: the draw opens a star session itself.
        assert_eq!(next_for_mine(false, &pool, false, sel, starred), Serve::Pick(st("eel")));
        assert!(already_served("eel") == false, "star words are tracked apart from a lineup's");
        assert!(star_served("eel") && !star_served("ant"));
        assert_eq!(next_for_mine(false, &pool, false, sel, starred), Serve::Pick(st("ant")));
        assert_eq!(next_for_mine(false, &pool, false, sel, starred), Serve::Open);
        // "In my order" already puts starred words first: no second lead.
        end_session();
        assert_eq!(next_for_mine(false, &pool, true, sel, starred), Serve::Open);
        // Spell Jr: ignored.
        end_session();
        assert_eq!(next_for_mine(true, &pool, false, sel, starred), Serve::Open);
        // A word removed from the list is not led (O-D).
        end_session();
        let gone = || vec![st("zzz")];
        assert_eq!(next_for_mine(false, &pool, false, sel, gone), Serve::Open);
        // Lineup picks precede stars in the list the lineup was made in.
        let mut l = Lineups::default();
        let id = scope_id(SURFACE_MINE, "l1", "en");
        l.toggle(&id, SURFACE_MINE, "l1", "en", pk("dog"), None);
        set_for_test(l);
        begin_session(&id, false);
        assert_eq!(next_for_mine(false, &pool, false, sel, starred), Serve::Pick(st("dog")));
        assert_eq!(next_for_mine(false, &pool, false, sel, starred), Serve::Pick(st("eel")));
        assert_eq!(next_for_mine(false, &pool, false, sel, starred), Serve::Pick(st("ant")));
        assert_eq!(next_for_mine(false, &pool, false, sel, starred), Serve::Open);
        // A different list selected: the picks stay silent, the stars still lead.
        begin_session(&id, false);
        assert_eq!(next_for_mine(false, &pool, false, || vec!["l2".into()], starred), Serve::Pick(st("eel")));
        crate::flags::set_test_override(None);
    }

    /// O-F: the site build can never reach any of this, whatever localStorage says.
    #[cfg(feature = "web")]
    #[test]
    fn the_site_build_has_the_flag_permanently_off() {
        crate::flags::set_test_override(Some("on"));
        assert!(!enabled());
        crate::flags::set_test_override(None);
    }
}
