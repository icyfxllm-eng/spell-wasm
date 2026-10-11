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
//! Phase D (F4, behind its own flag `pick_queue_d`): a lined-up or starred word
//! spelled wrong re-enters the SAME session 3-5 draws later (`requeue_offset`,
//! deterministic from the draw index and a stable hash of the word). The
//! re-queue is SESSION MEMORY only: the lineup itself persists, but the session
//! (what was served, where it is up to) has never been persisted, so a
//! force-quit starts a fresh session over the surviving lineup, as it always did.
//! At most `REQUEUE_MAX_PER_WORD` re-entries per word per session.
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

    /// CC-PICK-QUEUE Phase C ("Toughest 5"): append `picks` in the order given,
    /// skipping any word already lined up (those keep their position and
    /// number) and stopping at `cap` when there is one. Returns how many were
    /// appended and whether the cap cut the list short. Appending is the ONLY
    /// thing it does: nothing already picked moves (I5).
    pub fn add_many(&mut self, id: &str, surface: &str, list: &str, lang: &str, picks: Vec<Pick>, cap: Option<usize>) -> (usize, bool) {
        let mut added = 0;
        let mut cut = false;
        for p in picks {
            if self.picks(id).iter().any(|x| x.key == p.key) {
                continue;
            }
            match self.toggle(id, surface, list, lang, p, cap) {
                Toggled::Added(_) => added += 1,
                Toggled::Refused => {
                    cut = true;
                    break;
                }
                Toggled::Removed => {}
            }
        }
        (added, cut)
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
    decide_full(picks, stars, &Requeue::default(), served, just_these, live)
}

// ---------- Phase D: F4, missed picks come back ----------

/// A re-queued miss comes back no sooner than this many draws after the draw
/// that missed it (D8: never fewer than three).
pub const REQUEUE_MIN: usize = 3;
/// ... and no later than this many.
pub const REQUEUE_MAX: usize = 5;
/// A word that keeps being missed re-enters at most this many times per
/// session, then is an ordinary word again. Generous on purpose: the session's
/// own end rules (the player answers, or ends it) already terminate the loop;
/// this only stops a lineup of one from becoming a treadmill with no way out
/// but the exit control.
pub const REQUEUE_MAX_PER_WORD: u32 = 5;

/// FNV-1a over the word's identity key: stable across runs, platforms and
/// languages (no `DefaultHasher`, whose output is not guaranteed).
fn stable_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// D8: the re-queue offset, deterministic, always within
/// `REQUEUE_MIN..=REQUEUE_MAX`. `draw` is the session position (0-based draw
/// index) of the word that was just missed; `key` its identity. Same inputs,
/// same offset: no clock, no RNG. The two terms keep one word from always
/// landing at the same distance and different words from moving in lockstep.
pub fn requeue_offset(draw: usize, key: &str) -> usize {
    let span = REQUEUE_MAX - REQUEUE_MIN + 1;
    REQUEUE_MIN + ((stable_hash(key) % span as u64) as usize + draw % span) % span
}

#[derive(Debug, PartialEq, Clone)]
pub struct Queued {
    pub pick: Pick,
    /// The draw index at (or after) which it is served again.
    pub due_at: usize,
}

/// The in-session re-queue: SESSION MEMORY ONLY (see the module notes). One
/// source of truth for Misses, My Words, lineup picks and stars (I8).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Requeue {
    /// Draws the session has made so far (the next draw's index).
    pub draws: usize,
    pub pending: Vec<Queued>,
    /// How many times each word has been re-queued this session.
    pub counts: Vec<(String, u32)>,
}

impl Requeue {
    /// The word served at draw `draws - 1` was spelled wrong. Returns the draw
    /// index it will come back at, or `None` when it is already waiting or has
    /// used up its re-queues.
    pub fn note_miss(&mut self, pick: &Pick) -> Option<usize> {
        if self.draws == 0 || self.pending.iter().any(|q| q.pick.key == pick.key) {
            return None;
        }
        let used = self.counts.iter().position(|(k, _)| *k == pick.key);
        if used.map(|i| self.counts[i].1).unwrap_or(0) >= REQUEUE_MAX_PER_WORD {
            return None;
        }
        match used {
            Some(i) => self.counts[i].1 += 1,
            None => self.counts.push((pick.key.clone(), 1)),
        }
        let at = self.draws - 1;
        let due_at = at + requeue_offset(at, &pick.key);
        self.pending.push(Queued { pick: pick.clone(), due_at });
        Some(due_at)
    }

    pub fn is_pending(&self, key: &str) -> bool {
        self.pending.iter().any(|q| q.pick.key == key)
    }

    fn earliest(&self, live: &impl Fn(&Pick) -> bool, only_due: bool) -> Option<&Queued> {
        self.pending
            .iter()
            .filter(|q| live(&q.pick) && (!only_due || q.due_at <= self.draws))
            .min_by_key(|q| q.due_at)
    }
}

/// The ONE ordering rule including re-queued misses (F4). Order of precedence:
///
/// 1. a re-queued miss whose slot has arrived (the I2 exception: it jumps the
///    rest of the lineup, because it is a miss, not a new pick);
/// 2. the lineup's picks, in tap order;
/// 3. "Just these" with the picks done: any re-queued miss still waiting is
///    served now (the lineup's end is "the end" of F4's shorter-than-offset
///    rule), and only when none is left is the session `Exhausted` (A9);
/// 4. the focus stars not already served;
/// 5. the normal queue (`Open`).
///
/// With an empty `Requeue` this is exactly Phase B's rule.
pub fn decide_full(
    picks: &[Pick],
    stars: &[Pick],
    rq: &Requeue,
    served: &[String],
    just_these: bool,
    live: impl Fn(&Pick) -> bool,
) -> Serve {
    if let Some(q) = rq.earliest(&live, true) {
        return Serve::Pick(q.pick.clone());
    }
    if let Some(p) = picks.iter().find(|p| !served.contains(&p.key) && live(p)) {
        return Serve::Pick(p.clone());
    }
    if just_these {
        if let Some(q) = rq.earliest(&live, false) {
            return Serve::Pick(q.pick.clone());
        }
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
    /// Phase D: the whole pick that was served last (its key and language), so
    /// a miss can be re-queued under the same identity it was served with.
    last_pick: Option<Pick>,
    /// Phase B: the language whose Misses stars lead this session. Empty for a
    /// My Words session (its stars come from the lists, each in its own language).
    star_lang: String,
    /// Phase D: this session's re-queued misses. Memory only (see the module notes).
    rq: Requeue,
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

/// Phase C: append several picks at once (the Toughest 5 quick pick), through
/// the same cap as a single tap.
pub fn add_many(id: &str, surface: &str, list: &str, lang: &str, picks: Vec<Pick>, kid: bool) -> (usize, bool) {
    let cap = crate::experience::pick_caps(crate::experience::of_kid(kid)).max_picks;
    write(|l| l.add_many(id, surface, list, lang, picks, cap))
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
            last_pick: None,
            star_lang: read(|l| l.scopes.get(scope).map(|s| s.lang.clone()).unwrap_or_default()),
            rq: Requeue::default(),
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
            last_pick: None,
            star_lang: lang.to_string(),
            rq: Requeue::default(),
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
            s.last_pick = None;
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
    let (served, just, rq) = SESSION.with(|c| {
        let b = c.borrow();
        let s = b.as_ref().expect("session");
        (s.served.clone(), s.just_these, s.rq.clone())
    });
    let r = decide_full(&picks, stars, &rq, &served, just && use_picks, &live);
    SESSION.with(|c| {
        if let Some(s) = c.borrow_mut().as_mut() {
            match &r {
                Serve::Pick(p) => {
                    if s.rq.is_pending(&p.key) {
                        // A re-queued miss coming back: it is already in
                        // `served`, and its slot is spent.
                        s.rq.pending.retain(|q| q.pick.key != p.key);
                    } else {
                        s.served.push(p.key.clone());
                        s.served_words.push(p.word.clone());
                    }
                    s.last_word = Some(p.word.clone());
                    s.last_pick = Some(p.clone());
                    s.rq.draws += 1;
                }
                Serve::Open => {
                    s.last_word = None;
                    s.last_pick = None;
                    s.rq.draws += 1;
                }
                Serve::Exhausted => {
                    s.last_word = None;
                    s.last_pick = None;
                }
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
    if !use_picks && stars.is_empty() && !has_pending_requeue() {
        return Serve::Open;
    }
    next_from(&sc, use_picks, &stars, |p| list_words.contains(&p.word))
}

/// Phase D (F4) is on: its own flag, and only while Phase A is.
pub fn requeue_enabled() -> bool {
    enabled() && crate::flags::pick_queue_d()
}

fn has_pending_requeue() -> bool {
    requeue_enabled() && SESSION.with(|c| c.borrow().as_ref().map(|s| !s.rq.pending.is_empty()).unwrap_or(false))
}

/// F4: the word the session served last was just spelled wrong (the miss has
/// been recorded exactly as ever, I1). If the lineup or a star served it, it
/// re-enters this session 3-5 draws later. Anything else, or the flag off, is a
/// no-op. Returns the draw index it will come back at.
pub fn note_miss(word: &str) -> Option<usize> {
    if !requeue_enabled() {
        return None;
    }
    SESSION.with(|c| {
        let mut b = c.borrow_mut();
        let s = b.as_mut()?;
        let p = s.last_pick.clone().filter(|p| p.word == word)?;
        s.rq.note_miss(&p)
    })
}

/// A re-queued miss is waiting for its slot: the ordinary queue must not hand it
/// out early (that would be a random reappearance, not the 3-5 rule).
pub fn requeue_pending(key: &str) -> bool {
    requeue_enabled() && SESSION.with(|c| c.borrow().as_ref().map(|s| s.rq.is_pending(key)).unwrap_or(false))
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

    // ---------- Phase D: F4, missed picks come back ----------

    /// Serve `n` draws from a Misses session; return the words (Open = "_").
    #[cfg(not(feature = "web"))]
    fn draw(n: usize) -> Vec<String> {
        (0..n)
            .map(|_| match next_for_misses(false, "en", |_| true) {
                Serve::Pick(p) => p.word,
                Serve::Open => "_".to_string(),
                Serve::Exhausted => "END".to_string(),
            })
            .collect()
    }

    #[cfg(not(feature = "web"))]
    fn session_with(words: &[&str], just: bool) {
        crate::flags::set_test_override(Some("on"));
        let mut l = Lineups::default();
        let id = sid();
        for w in words {
            l.toggle(&id, SURFACE_MISSES, "", "en", pk(w), None);
        }
        l.set_just_these(&id, just);
        set_for_test(l);
        begin_session(&id, false);
    }

    /// D8: deterministic and always 3..=5, for every draw index and a spread of keys.
    #[test]
    fn the_requeue_offset_is_deterministic_and_within_three_to_five() {
        let mut seen = std::collections::BTreeSet::new();
        for d in 0..60 {
            for k in ["en::cat", "en::dog", "de::Haus", "__mine::x", "zh::\u{4f60}\u{597d}", ""] {
                let o = requeue_offset(d, k);
                assert!((REQUEUE_MIN..=REQUEUE_MAX).contains(&o), "{d} {k} -> {o}");
                assert_eq!(o, requeue_offset(d, k), "same inputs, same offset");
                seen.insert(o);
            }
        }
        assert_eq!(seen.len(), 3, "all of 3, 4 and 5 are reachable");
        // Pinned values: this is the rule, not a sample (change it knowingly).
        assert_eq!(requeue_offset(0, "en::cat"), requeue_offset(3, "en::cat"));
    }

    /// A4: a missed pick reappears 3-5 draws later, never sooner (never < 3).
    #[cfg(not(feature = "web"))]
    #[test]
    fn a_missed_pick_reappears_three_to_five_words_later() {
        for start in 0..3usize {
            session_with(&["cat"], false);
            // Some ordinary draws first so draw indexes vary.
            let _ = (0..start).map(|_| ()).count();
            assert_eq!(draw(1), vec!["cat"]); // draw 0
            assert_eq!(note_miss("cat"), Some(requeue_offset(0, "en::cat")));
            let seq = draw(8);
            let at = seq.iter().position(|w| w == "cat").expect("it comes back");
            let gap = at + 1; // draws after the missed one
            assert!((3..=5).contains(&gap), "gap {gap} in {seq:?}");
            assert!(seq[..at].iter().all(|w| w == "_"), "only ordinary words in between");
            assert_eq!(seq.iter().filter(|w| *w == "cat").count(), 1, "once, not twice (I3)");
        }
        crate::flags::set_test_override(None);
    }

    /// F4 sits behind its own flag: with D off a miss is not re-queued.
    #[cfg(not(feature = "web"))]
    #[test]
    fn flag_off_means_no_requeue() {
        session_with(&["cat"], false);
        assert_eq!(draw(1), vec!["cat"]);
        // The test override is global (one value for every flag), so exercise
        // the gate through the same function the game calls.
        crate::flags::set_test_override(Some("off"));
        assert_eq!(note_miss("cat"), None);
        crate::flags::set_test_override(Some("on"));
        assert!(draw(8).iter().all(|w| w != "cat"));
        crate::flags::set_test_override(None);
    }

    /// A word answered correctly (no note_miss) is never served again (I3).
    #[cfg(not(feature = "web"))]
    #[test]
    fn a_correct_pick_does_not_repeat() {
        session_with(&["cat", "dog"], false);
        assert_eq!(draw(6), vec!["cat", "dog", "_", "_", "_", "_"]);
        crate::flags::set_test_override(None);
    }

    /// A word the session did not serve as a pick (an ordinary draw) is not re-queued.
    #[cfg(not(feature = "web"))]
    #[test]
    fn only_a_served_pick_is_requeued() {
        session_with(&["cat"], false);
        assert_eq!(draw(2), vec!["cat", "_"]);
        assert_eq!(note_miss("cat"), None, "the last serve was an ordinary word");
        assert_eq!(note_miss("zzz"), None);
        crate::flags::set_test_override(None);
    }

    /// I2: the lineup is served before anything unpicked, EXCEPT a re-queued
    /// miss, which takes its slot ahead of picks still waiting.
    #[cfg(not(feature = "web"))]
    #[test]
    fn the_i2_exception_is_only_for_requeued_misses() {
        session_with(&["a", "b", "c", "d", "e", "f", "g", "h"], false);
        let mut out = draw(1);
        assert_eq!(note_miss("a"), Some(requeue_offset(0, "en::a")));
        out.extend(draw(8));
        // Eight picks, one miss: "a" appears twice; nothing unpicked ("_") yet
        // because 9 serves of 8 picks + 1 requeue = 9 > 8 -> one "_" at the end.
        assert_eq!(out.iter().filter(|w| *w == "a").count(), 2);
        let first_open = out.iter().position(|w| w == "_");
        let last_pick = out.iter().rposition(|w| w != "_").unwrap();
        if let Some(o) = first_open {
            assert!(o > last_pick, "no unpicked word before the lineup is done: {out:?}");
        }
        // Every other pick appears exactly once.
        for w in ["b", "c", "d", "e", "f", "g", "h"] {
            assert_eq!(out.iter().filter(|x| *x == w).count(), 1, "{w}");
        }
        // The requeued "a" sits 3-5 after its miss, ahead of a pick that was waiting.
        let second = out.iter().skip(1).position(|w| w == "a").unwrap() + 1;
        assert!((3..=5).contains(&second), "{out:?}");
        crate::flags::set_test_override(None);
    }

    /// A starred word spelled wrong is re-queued too.
    #[cfg(not(feature = "web"))]
    #[test]
    fn a_starred_word_is_requeued() {
        crate::flags::set_test_override(Some("on"));
        crate::pick_stars::set_for_test(crate::pick_stars::Stars { v: 1, stars: vec![st("owl")] });
        set_for_test(Lineups::default());
        begin_star_session(false, "en");
        assert_eq!(draw(1), vec!["owl"]);
        assert!(note_miss("owl").is_some());
        let seq = draw(8);
        let at = seq.iter().position(|w| w == "owl").expect("the star comes back");
        assert!((2..=4).contains(&at), "{seq:?}");
        assert_eq!(seq.iter().filter(|w| *w == "owl").count(), 1);
        end_session();
        crate::flags::set_test_override(None);
    }

    /// Missed again: it re-enters again, up to REQUEUE_MAX_PER_WORD, and then it
    /// is an ordinary word (the bound is visible here).
    #[cfg(not(feature = "web"))]
    #[test]
    fn a_word_missed_again_reenters_up_to_the_bound() {
        session_with(&["cat"], false);
        let mut served = 0;
        assert_eq!(draw(1), vec!["cat"]);
        served += 1;
        loop {
            if note_miss("cat").is_none() {
                break;
            }
            // Draw one word at a time until it is back, so the miss can be
            // noted on the draw that served it.
            let mut gap = 0;
            loop {
                gap += 1;
                assert!(gap <= REQUEUE_MAX, "back within five draws");
                if draw(1) == vec!["cat"] {
                    break;
                }
            }
            assert!((REQUEUE_MIN..=REQUEUE_MAX).contains(&gap), "gap {gap}");
            served += 1;
            assert!(served < 50, "never loops forever");
        }
        assert_eq!(served as u32, 1 + REQUEUE_MAX_PER_WORD, "first serve + the bound");
        // After the bound the word is ordinary: not re-queued, not served again.
        assert!(draw(8).iter().all(|w| w == "_"));
        crate::flags::set_test_override(None);
    }

    /// "Just these": a miss stays inside the session; the session ends only once
    /// the lineup is exhausted including re-queues. Fewer picks left than the
    /// offset: the word goes to the end. A9 still holds with no misses.
    #[cfg(not(feature = "web"))]
    #[test]
    fn just_these_ends_after_requeues_and_a9_holds_without_misses() {
        // A9: three picks, no miss -> three serves, then the end.
        session_with(&["a", "b", "c"], true);
        assert_eq!(draw(4), vec!["a", "b", "c", "END"]);
        // One miss on the first of two picks: the queue is shorter than the
        // offset, so it goes to the end, and only then does the session end.
        session_with(&["a", "b"], true);
        assert_eq!(draw(1), vec!["a"]);
        assert!(note_miss("a").is_some());
        assert_eq!(draw(3), vec!["b", "a", "END"]);
        // Missed twice more in a row: still inside the session, still ends.
        session_with(&["a"], true);
        assert_eq!(draw(1), vec!["a"]);
        assert!(note_miss("a").is_some());
        assert_eq!(draw(1), vec!["a"]);
        assert!(note_miss("a").is_some());
        assert_eq!(draw(1), vec!["a"]);
        assert_eq!(draw(1), vec!["END"], "answered correctly this time");
        crate::flags::set_test_override(None);
    }

    /// Longer queue than the offset in Just these: back at its slot, not the end.
    #[cfg(not(feature = "web"))]
    #[test]
    fn just_these_requeue_lands_at_its_slot_when_the_queue_is_long_enough() {
        session_with(&["a", "b", "c", "d", "e", "f", "g", "h"], true);
        let mut out = draw(1);
        assert!(note_miss("a").is_some());
        out.extend(draw(9));
        assert_eq!(out.iter().filter(|w| *w == "a").count(), 2);
        let second = out.iter().skip(1).position(|w| w == "a").unwrap() + 1;
        assert!((3..=5).contains(&second), "{out:?}");
        assert_eq!(out.last().unwrap(), "END");
        crate::flags::set_test_override(None);
    }

    /// Zero picks, no stars: nothing changes, and nothing is ever re-queued (I4).
    #[cfg(not(feature = "web"))]
    #[test]
    fn zero_picks_is_identity() {
        crate::flags::set_test_override(Some("on"));
        set_for_test(Lineups::default());
        end_session();
        assert_eq!(next_for_misses(false, "en", |_| true), Serve::Open);
        assert_eq!(note_miss("cat"), None);
        assert!(!requeue_pending("en::cat"));
        // And the pure rule with an empty requeue is Phase B's rule.
        let picks = [pk("a"), pk("b")];
        let live = |_: &Pick| true;
        assert_eq!(
            decide_full(&picks, &[], &Requeue::default(), &[], false, live),
            decide_with_stars(&picks, &[], &[], false, live)
        );
        assert_eq!(decide_full(&[], &[], &Requeue::default(), &[], false, live), Serve::Open);
        crate::flags::set_test_override(None);
    }

    /// The pending word is held back from the ordinary queue until its slot.
    #[cfg(not(feature = "web"))]
    #[test]
    fn a_pending_requeue_is_reported_so_the_deck_can_hold_it_back() {
        session_with(&["cat"], false);
        assert_eq!(draw(1), vec!["cat"]);
        assert!(!requeue_pending("en::cat"));
        note_miss("cat");
        assert!(requeue_pending("en::cat"));
        let _ = draw(8);
        assert!(!requeue_pending("en::cat"), "served: slot spent");
        crate::flags::set_test_override(None);
    }

    /// A requeued word that is no longer live (graduated / removed) is dropped,
    /// never served.
    #[cfg(not(feature = "web"))]
    #[test]
    fn a_requeued_word_that_vanished_is_not_served() {
        session_with(&["cat"], true);
        assert_eq!(draw(1), vec!["cat"]);
        note_miss("cat");
        assert_eq!(next_for_misses(false, "en", |_| false), Serve::Exhausted);
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
