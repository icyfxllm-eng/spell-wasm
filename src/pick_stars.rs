//! CC-PICK-QUEUE v1 Phase B -- focus stars: a starred word leads every session
//! until it is learned.
//!
//! Two stores, one rule, one component:
//!
//! * **My Words** stars ARE `ListEntry.starred` (O-A) -- the star "In my order"
//!   has always had. Nothing is stored here for them. Removing a word from My
//!   Words is what "learned" means there (O-D), so the star goes with the entry.
//! * **Misses** has no star field, so its stars live in this module's own key,
//!   `spell_stars_v1` (versioned; a blob from a NEWER build is read as empty and
//!   never overwritten, as `spell_lineup_v1` does). No existing store is touched
//!   and no schema is bumped. A starred miss that graduates is pruned silently.
//!
//! The cap is [`MAX_STARS`] per language; the eleventh is refused and the UI says
//! so (A6). Standard players only: Spell Jr renders no star control and any
//! persisted star is ignored, decided by `experience::pick_caps(..).extras`, the
//! same single source the lineup's Jr rules use.
//!
//! Stars change ORDER only (I1). The serve guards live in `pick_queue`
//! (`decide_with_stars`) so one function owns the ordering rule.

use crate::pick_queue::Pick;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

pub const KEY: &str = "spell_stars_v1";
pub const VERSION: u32 = 1;
/// Stars one language may hold.
pub const MAX_STARS: usize = 10;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Stars {
    pub v: u32,
    /// Starred misses, oldest star first. `lang` lives on each [`Pick`].
    #[serde(default)]
    pub stars: Vec<Pick>,
}

impl Default for Stars {
    fn default() -> Self {
        Stars { v: VERSION, stars: Vec::new() }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StarToggled {
    Starred,
    Unstarred,
    /// The cap was reached: nothing changed and the player must be told (A6).
    Refused,
}

/// Whether one more star fits in a language that already holds `count`. Both
/// stores go through this one rule.
pub fn cap_allows(count: usize) -> bool {
    count < MAX_STARS
}

impl Stars {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Tolerant reader; the bool is "from a newer build" (read as empty, never
    /// written back).
    pub fn from_json(raw: &str) -> (Stars, bool) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
            return (Stars::default(), false);
        };
        let ver = v.get("v").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        if ver > VERSION {
            return (Stars::default(), true);
        }
        (serde_json::from_value::<Stars>(v).unwrap_or_default(), false)
    }

    pub fn has(&self, key: &str) -> bool {
        self.stars.iter().any(|p| p.key == key)
    }

    pub fn count(&self, lang: &str) -> usize {
        self.stars.iter().filter(|p| p.lang == lang).count()
    }

    pub fn of_lang(&self, lang: &str) -> Vec<Pick> {
        self.stars.iter().filter(|p| p.lang == lang).cloned().collect()
    }

    /// Tap: star if absent (cap permitting), unstar if present (always allowed).
    pub fn toggle(&mut self, pick: Pick) -> StarToggled {
        if let Some(i) = self.stars.iter().position(|p| p.key == pick.key) {
            self.stars.remove(i);
            return StarToggled::Unstarred;
        }
        if !cap_allows(self.count(&pick.lang)) {
            return StarToggled::Refused;
        }
        self.stars.push(pick);
        StarToggled::Starred
    }

    /// Drop stars whose miss is gone (graduated, evicted). True if changed.
    pub fn prune(&mut self, live: impl Fn(&Pick) -> bool) -> bool {
        let before = self.stars.len();
        self.stars.retain(|p| live(p));
        self.stars.len() != before
    }

    /// Reset-this-language (O-E): that language's stars go, nobody else's.
    pub fn clear_lang(&mut self, lang: &str) -> bool {
        let before = self.stars.len();
        self.stars.retain(|p| p.lang != lang);
        self.stars.len() != before
    }
}

// ---------- runtime store ----------

struct Store {
    stars: Stars,
    newer: bool,
    loaded: bool,
}

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store { stars: Stars::default(), newer: false, loaded: false });
}

fn ensure_loaded(s: &mut Store) {
    if s.loaded {
        return;
    }
    s.loaded = true;
    if let Some(raw) = crate::storage::get_raw(KEY) {
        let (st, newer) = Stars::from_json(&raw);
        s.stars = st;
        s.newer = newer;
    }
}

pub fn read<R>(f: impl FnOnce(&Stars) -> R) -> R {
    STORE.with(|c| {
        let mut s = c.borrow_mut();
        ensure_loaded(&mut s);
        f(&s.stars)
    })
}

/// Change the stars; written through on every call (a newer blob is not).
pub fn write<R>(f: impl FnOnce(&mut Stars) -> R) -> R {
    STORE.with(|c| {
        let mut s = c.borrow_mut();
        ensure_loaded(&mut s);
        let r = f(&mut s.stars);
        if !s.newer {
            crate::storage::set_json(KEY, &s.stars);
        }
        r
    })
}

#[cfg(test)]
pub fn set_for_test(st: Stars) {
    STORE.with(|c| *c.borrow_mut() = Store { stars: st, newer: false, loaded: true });
}

/// The flag: Phase B is its own switch, but the stars live on Phase A's
/// surfaces, so it does nothing unless Phase A is on too.
pub fn enabled() -> bool {
    crate::flags::pick_queue_a() && crate::flags::pick_queue_b()
}

/// Whether stars exist for this player at all: flag on AND a Standard player.
/// Spell Jr: no control rendered, persisted stars ignored (I7).
pub fn active(kid: bool) -> bool {
    enabled() && crate::experience::pick_caps(crate::experience::of_kid(kid)).extras
}

/// Star or unstar a miss. Prunes first so a graduated miss never holds a slot.
pub fn toggle_miss(pick: Pick, live: impl Fn(&Pick) -> bool) -> StarToggled {
    write(|s| {
        s.prune(&live);
        s.toggle(pick)
    })
}

/// Silent prune (O-D): a starred miss that graduated or was evicted.
pub fn prune_misses(live: impl Fn(&Pick) -> bool) {
    if !enabled() {
        return;
    }
    if read(|s| s.stars.iter().any(|p| !live(p))) {
        write(|s| s.prune(&live));
    }
}

/// A miss graduated: its star goes with it, silently. Cheap no-op flag off.
pub fn on_miss_graduated(key: &str) {
    if !enabled() {
        return;
    }
    if read(|s| s.has(key)) {
        write(|s| s.prune(|p| p.key != key));
    }
}

/// Reset this language: clear its Misses stars (the My Words stars are cleared
/// by `word_lists::clear_stars` at the same call site).
pub fn clear_lang(lang: &str) {
    if !enabled() {
        return;
    }
    if read(|s| s.stars.iter().any(|p| p.lang == lang)) {
        write(|s| s.clear_lang(lang));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pk(lang: &str, w: &str) -> Pick {
        Pick { key: format!("{lang}::{w}"), word: w.to_string(), lang: lang.to_string() }
    }

    #[test]
    fn toggle_stars_and_unstars() {
        let mut s = Stars::default();
        assert_eq!(s.toggle(pk("en", "cat")), StarToggled::Starred);
        assert!(s.has("en::cat"));
        assert_eq!(s.toggle(pk("en", "cat")), StarToggled::Unstarred);
        assert!(!s.has("en::cat"));
    }

    #[test]
    fn the_eleventh_star_in_a_language_is_refused_and_changes_nothing() {
        let mut s = Stars::default();
        for i in 0..MAX_STARS {
            assert_eq!(s.toggle(pk("en", &format!("w{i}"))), StarToggled::Starred);
        }
        let before = s.clone();
        assert_eq!(s.toggle(pk("en", "eleventh")), StarToggled::Refused);
        assert_eq!(s, before);
        // Unstarring is always allowed and frees the slot.
        assert_eq!(s.toggle(pk("en", "w0")), StarToggled::Unstarred);
        assert_eq!(s.toggle(pk("en", "eleventh")), StarToggled::Starred);
    }

    #[test]
    fn the_cap_is_per_language() {
        let mut s = Stars::default();
        for i in 0..MAX_STARS {
            s.toggle(pk("en", &format!("w{i}")));
        }
        assert_eq!(s.toggle(pk("en", "x")), StarToggled::Refused);
        // German is untouched by English being full (D9/A11).
        assert_eq!(s.toggle(pk("de", "hund")), StarToggled::Starred);
        assert_eq!(s.count("en"), 10);
        assert_eq!(s.count("de"), 1);
        assert_eq!(s.of_lang("de").len(), 1);
        // Reset this language clears one language only (O-E).
        assert!(s.clear_lang("en"));
        assert_eq!(s.count("en"), 0);
        assert_eq!(s.count("de"), 1);
        assert!(!s.clear_lang("en"));
    }

    #[test]
    fn persistence_round_trip_and_newer_blob_is_empty_and_flagged() {
        let mut s = Stars::default();
        s.toggle(pk("en", "cat"));
        s.toggle(pk("de", "hund"));
        let (back, newer) = Stars::from_json(&s.to_json());
        assert!(!newer);
        assert_eq!(back, s);
        let (empty, newer) = Stars::from_json("{\"v\":99,\"stars\":[1,2,3]}");
        assert!(newer);
        assert!(empty.stars.is_empty());
        assert!(Stars::from_json("not json").0.stars.is_empty());
    }

    #[test]
    fn a_graduated_miss_loses_its_star_and_frees_its_slot() {
        let mut s = Stars::default();
        s.toggle(pk("en", "cat"));
        s.toggle(pk("en", "dog"));
        assert!(s.prune(|p| p.word != "cat"));
        assert_eq!(s.count("en"), 1);
        assert!(!s.prune(|_| true));
    }

    #[cfg(not(feature = "web"))]
    #[test]
    fn jr_ignores_stars_through_the_one_experience_function() {
        crate::flags::set_test_override(Some("on"));
        assert!(active(false), "Standard sees stars");
        assert!(!active(true), "Spell Jr sees none");
        crate::flags::set_test_override(Some("off"));
        assert!(!active(false), "flag off: no stars for anyone");
        crate::flags::set_test_override(None);
    }

    #[cfg(not(feature = "web"))]
    #[test]
    fn runtime_store_round_trips_and_prunes_silently() {
        crate::flags::set_test_override(Some("on"));
        set_for_test(Stars::default());
        assert_eq!(toggle_miss(pk("en", "cat"), |_| true), StarToggled::Starred);
        assert!(read(|s| s.has("en::cat")));
        on_miss_graduated("en::cat");
        assert!(!read(|s| s.has("en::cat")));
        toggle_miss(pk("en", "dog"), |_| true);
        prune_misses(|p| p.word != "dog");
        assert_eq!(read(|s| s.count("en")), 0);
        crate::flags::set_test_override(Some("off"));
        toggle_miss(pk("en", "eel"), |_| true);
        on_miss_graduated("en::eel"); // flag off: a no-op
        assert!(read(|s| s.has("en::eel")));
        crate::flags::set_test_override(None);
    }

    /// O-F: the site build can never reach any of this.
    #[cfg(feature = "web")]
    #[test]
    fn the_site_build_has_the_stars_flag_permanently_off() {
        crate::flags::set_test_override(Some("on"));
        assert!(!enabled());
        assert!(!active(false));
        crate::flags::set_test_override(None);
    }
}
