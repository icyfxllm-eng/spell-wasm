//! CC-MODE-HUB F2 — the Play hub: one tile per visible mode, rendered from
//! `config/modes.json` and nothing else.
//!
//! # What a tile is allowed to promise
//! Three kinds, because the six things CC-MODE-HUB calls "modes" are not the
//! same kind of thing:
//!
//!   * **launcher** — the mode has a real entry point (Spell It's `spellItBtn`,
//!     Spell-Off's `soBtn`). Renders as a `<button>` that clicks it.
//!   * **info** — the mode is an in-round AID with no destination. Spell Racing
//!     happens inside The Climb; syllable replay fires when you miss a word;
//!     word stories are an after-answer flourish (its own review doc says
//!     "not a session mode"). Renders as a non-interactive card that tells the
//!     player the aid is on and where it shows up.
//!   * **teaser** — `coming_soon`. Non-tappable, no notify-me hook (D7).
//!
//! Only a launcher is a `<button>`. An aid is a `<div>`. That is deliberate: a
//! tappable tile that goes nowhere is a lie the markup itself would tell, so the
//! element type makes it unrepresentable rather than merely discouraged.
//!
//! # No new copy
//! Tiles reuse the shipped `tools.*` catalog (F2: "names reuse existing localized
//! mode strings; no new auditable content"), so the hub is fully localized in all
//! 12 locales the moment it renders, and adds nothing for a translator to chase.
//! There is no title or subtitle — chrome would need new strings in 12 locales.

use crate::dom;
use crate::entitlements;
use crate::i18n::t;
use crate::modes::{self, Mode, Status};
use crate::native_lang;
use crate::App;

/// Mode id -> the element id of its EXISTING entry point, if it has one.
///
/// Lives here, not in `modes.json`: the registry holds what a mode IS, this holds
/// how this particular frontend reaches it. Keeping DOM ids out of the registry
/// is what lets the same file describe a mode for a future surface that has no
/// such element. `None` = an in-round aid with no destination.
#[cfg(not(feature = "web"))]
const N_LAUNCH: usize = 19; // + spell_cross (CC-WORDGRID Phase B, app only)
#[cfg(feature = "web")]
const N_LAUNCH: usize = 9;
const LAUNCH: [(&str, Option<&str>); N_LAUNCH] = [
    ("practice", Some("practiceOpen")), // CC-PRACTICE: the front porch, first (D9)
    // Races inside The Climb, but HAS its own screen: it shows the ghost you're
    // racing (your best run for this language) and starts a Climb run.
    ("ghost_racing", Some("ghostOpenBtn")),
    ("syllable_replay", None),    // fires on a miss, on the reveal surface
    // CC-HUB-NAV v1.3.1 H3: it HAS a door -- photoBtn, on the utility row the
    // Phase C cut-over retired. Naming it here is what gives the drawer row
    // something to proxy to; a your_words row with no target renders and then
    // does nothing when tapped.
    ("photo_list", Some("photoBtn")),
    ("spell_aloud", Some("spellAloudEnter")), // promoted to a real mode (G-INT-1): enters play with the voice mic
    ("word_stories", None),       // after-answer flourish; hidden anyway
    ("online_spelloff", Some("soBtn")),
    ("def_match", Some("defMatchOpen")), // CC-DEF-MATCH P3: the floating-cards loop
    // CC-LETTER-FORGE F1. App-only, and not by preference: `mod forge` — the
    // generator this screen drives — is itself cfg'd out of the web build, so
    // a web row here would bind a tile to a mode whose engine does not exist.
    #[cfg(not(feature = "web"))]
    ("letter_forge", Some("forgeOpen")),
    // CC-WORD-CHAINS F1. App-only for the same reason as the forge: its
    // engine depends on word_index, which the web build compiles out.
    #[cfg(not(feature = "web"))]
    ("word_chains", Some("chainsOpen")),
    // CC-IMPOSTOR F1. App-only: its generator depends on word_index.
    #[cfg(not(feature = "web"))]
    ("impostor", Some("impostorOpen")),
    // CC-BEE-SIM F1. App-only: the ladder reads the bundled banks and the
    // ceremony leans on local TTS, both of which the web build omits.
    #[cfg(not(feature = "web"))]
    ("bee_sim", Some("beeOpen")),
    // CC-WORD-PICTURE v5: calligram picker. Compiled out with the mode
    // (I1): the registry row is deleted on web, so this would be a dead
    // table entry -- but a dead entry still puts the mode's name in the
    // bundle, which is exactly what the wall scan polices.
    #[cfg(not(feature = "web"))]
    ("word_picture", Some("wordPicOpen")),
    // CC-REPORTS: app-exclusive like the rest of the batch — the row is
    // deleted on web so the mode name stays out of the web bundle.
    #[cfg(not(feature = "web"))]
    ("reports", Some("repOpenBtn")),
    #[cfg(not(feature = "web"))]
    ("calendar", Some("calOpenBtn")),
    #[cfg(not(feature = "web"))]
    ("translate", Some("trOpenBtn")),
    #[cfg(not(feature = "web"))]
    ("spelldoku", Some("sdOpenBtn")),
    #[cfg(not(feature = "web"))]
    ("spell_search", Some("wsOpenBtn")),
    #[cfg(not(feature = "web"))]
    ("spell_cross", Some("xwOpenBtn")),
];

/// The mode -> entry-point join. `pub(crate)` so the CC-HUB-NAV drawer routes
/// to the same destination the sheet does rather than keeping a second table.
/// The ids themselves stay behind this file's cfg split, which is what keeps
/// app-only launchers out of the site bundle (CC-PICTURE-BANK I1).
pub(crate) fn launch_for(id: &str) -> Option<&'static str> {
    LAUNCH.iter().find(|(k, _)| *k == id).and_then(|(_, v)| *v)
}

/// The app's LIVE entitlement resolution — the ONE impure call every surface
/// shares (hub tiles, the photo camera button), so no two surfaces can disagree
/// about what's owned. The purchase / region adapters are later phases
/// (CC-ENTITLEMENTS), so today this resolves FREE_TIER — unless the dev-door
/// "test entitlements" grant is on, which resolves the audit maximum so gated
/// features stay testable on the floor device (the override is consumer-build
/// only and invisible to normal users, like the rest of the dev door).
pub fn live_entitlements() -> entitlements::EntitlementSet {
    let dev = crate::storage::get_raw("spell_dev_entitlements").as_deref() == Some("on");
    entitlements::resolve_entitlements(false, &[], dev)
}

/// Gather the live context the pure rule needs. The only impure part of the hub.
/// `pub(crate)` so the CC-HUB-NAV drawer gates its rows through the SAME
/// context the sheet does. A2 compares the two counts; sharing this makes them
/// equal by construction rather than by coincidence.
/// The language the MODE GATE should read, given the study language.
///
/// Pure so it can be tested without an App; see ctx for why it exists.
pub(crate) fn hub_lang(app_lang: &str, mine: Option<&'static str>) -> String {
    if app_lang == crate::consts::MINE {
        return mine.unwrap_or(crate::consts::EN).to_string();
    }
    if crate::consts::is_active_lang(app_lang) {
        return app_lang.to_string();
    }
    crate::consts::EN.to_string()
}

pub(crate) fn ctx(app: &App) -> modes::HubCtx {
    let (kid, lang) = {
        let s = app.borrow();
        // MINE and REVIEW are word SOURCES wearing a language code, and no
        // registry row lists either, so passing one straight through filters
        // EVERY mode out. The sheet merely rendered its "nothing here" panel
        // and nobody looked twice; the drawer is the navigation, so the same
        // bug empties a player's entire menu the moment they save a word list
        // or open their misses. Resolve to the real language behind the
        // source. My Words has one -- the speak language those words are read
        // in. The review queue mixes languages and has none, so it falls back
        // to the boot default like any unrecognised code; that is a guess
        // about mode AVAILABILITY only, and a wrong-but-populated menu beats
        // no menu.
        (s.kid, hub_lang(&s.lang, crate::game::mine_lang(&s)))
    };
    // When the purchase adapters land they feed the same call — the hub does
    // not re-derive entitlement, it asks.
    let ent = live_entitlements();
    let mut premium = Vec::new();
    if ent.photo_ocr {
        premium.push("photo_ocr".to_string());
    }
    if ent.multiple_profiles {
        premium.push("multiple_profiles".to_string());
    }
    if ent.progress_reports {
        premium.push("progress_reports".to_string());
    }
    modes::HubCtx {
        kid,
        // PLATFORM gate for the hub: is this iOS at all? (not "is the speech plugin
        // resolved" — that flaky check hid every iOS-only tile when the plugin proxy
        // was momentarily unavailable, leaving no way to reach the mode). Per-feature
        // availability is surfaced inside each mode.
        native: native_lang::is_native_platform(),
        level: ent.lang_level(&lang),
        lang,
        premium,
        enabled: modes::all()
            .iter()
            .map(|m| m.id.clone())
            .filter(|id| crate::flags::is_on(id))
            .collect(),
    }
}

#[cfg(not(feature = "web"))]
fn spelldoku_playable(lang: &str) -> bool {
    crate::spelldoku_ui::playable(lang)
}

#[cfg(feature = "web")]
fn spelldoku_playable(_lang: &str) -> bool {
    false
}

#[cfg(not(feature = "web"))]
fn spell_search_playable(lang: &str) -> bool {
    crate::wordsearch_ui::playable(lang)
}

#[cfg(feature = "web")]
fn spell_search_playable(_lang: &str) -> bool {
    false
}

#[cfg(not(feature = "web"))]
fn spell_cross_playable(lang: &str) -> bool {
    crate::wordcross_ui::playable(lang)
}

#[cfg(feature = "web")]
fn spell_cross_playable(_lang: &str) -> bool {
    false
}

/// A mode's availability for the CURRENT language. `spell_aloud` is voice-spell-gated:
/// live only where the language registry's `voice_spell` flag holds (en/es); elsewhere
/// it renders as an "unavailable / coming soon" tile — shown, not hidden (A7).
pub fn unavailable_reason(m: &Mode, lang: &str) -> Option<&'static str> {
    if m.id == "spell_aloud" && !crate::consts::voice_spell(lang) {
        Some("tools.spellaloud.avail") // "iPhone · English or Spanish"
    } else if m.id == "practice" && !crate::consts::practice(lang) {
        // D7: curriculum drafted but not yet cleared for this language.
        Some("tools.practice.avail")
    } else if m.id == "spelldoku" && !spelldoku_playable(lang) {
        // CC-SPELLDOKU D3: no servable number-word table for this language in
        // this build -- the tile says so instead of opening onto nothing.
        Some("sd.avail")
    } else if m.id == "spell_search" && !spell_search_playable(lang) {
        // CC-WORDGRID v1: outside the launch set (ko and zh never, D4).
        Some("ws.avail")
    } else if m.id == "spell_cross" && !spell_cross_playable(lang) {
        // CC-WORDGRID v1: outside the launch set (ko and zh never, D4).
        Some("xw.avail")
    } else if m.id == "def_match" && !crate::consts::def_match(lang) {
        // Pool not landed/pinned for this language yet (Invariant 8) — a
        // non-tappable teaser, never a dead button.
        Some("tools.defmatch.avail")
    } else {
        None
    }
}

// A13 — the gamepad sheet is DELETED here, not merely hidden.
//
// `tile_html`, `reflect`, `open`, `close`, `body_scroll_lock` and `wire`
// rendered and drove the "Ways to play" sheet behind the gamepad icon. The
// drawer replaced every one of those jobs, and a second surface that resolves
// modes is the exact thing CC-HUB-NAV I2 forbids ("no second list"). Its
// markup goes with it.
//
// What stays is everything that was never about the sheet: the LAUNCH table
// (the drawer routes through `launch_for`), `ctx`, `live_entitlements`, and
// `unavailable_reason` -- which the drawer now consults for the same reason
// the sheet did, so a language with no board for a mode shows no row for it.


// The site build deletes app-only modes from the registry (D1: absent, not
// filtered), so these assertions about registry CONTENTS are app-config
// truths. The web config gets its own assertion below rather than a version
// of these that changes its expectations based on cfg -- a test that moves
// its own goalposts proves whichever config you happened to run.
#[cfg(not(feature = "web"))]
#[cfg(test)]
mod tests {
    use super::*;

    /// The binding table must cover the registry exactly — a mode with no entry
    /// here would silently render as an aid, which is a lie if it has a
    /// destination.
    ///
    /// Core entries are the exception (CC-ONBOARD-JR): the base game, the
    /// Climb and the Daily are registered so one registry governs their
    /// juniorPolicy, and they are never hub tiles. They are excluded rather
    /// than bound to None, which would claim they are in-round aids.
    #[test]
    fn launch_table_covers_every_registered_mode() {
        let ids: Vec<String> = modes::all()
            .iter()
            .filter(|m| m.status != Status::Core)
            .map(|m| m.id.clone())
            .collect();
        let bound: Vec<String> = LAUNCH.iter().map(|(k, _)| k.to_string()).collect();
        assert_eq!(bound, ids, "LAUNCH must list every mode, in registry order");
    }

    /// Only genuine session modes get a destination. If this changes, someone has
    /// decided an in-round aid is tappable — which needs a real answer to "tap it
    /// and what happens?", not a silent edit.
    #[test]
    fn only_session_modes_have_a_destination() {
        assert_eq!(launch_for("online_spelloff"), Some("soBtn"));
        // Spell Racing got a real destination deliberately: tapping it opens the
        // ghost screen (ghost::wire_screen), which shows the best run you're
        // racing for this language and routes into The Climb. It is no longer a
        // tile that goes nowhere — which is exactly what this test guards.
        assert_eq!(launch_for("ghost_racing"), Some("ghostOpenBtn"));
        // Spell Aloud is now a real mode (CC-SPELL-ALOUD-INTEGRATION G-INT-1): tapping
        // it enters play with the voice mic. It is no longer an in-round aid.
        assert_eq!(launch_for("spell_aloud"), Some("spellAloudEnter"));
        // From a photo is not an in-round aid and never was one -- it opens the
        // camera and builds a word list. It sat in this list because it was
        // `hidden`, so its lack of a destination cost nothing; v1.3.1 H3 found
        // it reachable from nowhere at all once the utility row was retired,
        // and a your_words row with no target renders and then does nothing.
        assert_eq!(launch_for("photo_list"), Some("photoBtn"));
        for aid in ["syllable_replay", "word_stories"] {
            assert_eq!(launch_for(aid), None, "{aid} is an in-round aid with no destination");
        }
    }

    fn spell_aloud_mode() -> Mode {
        modes::all().into_iter().find(|m| m.id == "spell_aloud").expect("spell_aloud in registry")
    }

    /// The bug that emptied the drawer. Saving a word list sets the study
    /// language to __mine, and no registry row lists __mine, so every mode
    /// filtered out and the menu rendered nothing but Account and Settings.
    /// Found by routing the My Words e2e through the drawer, which is exactly
    /// what routing them through it was for.
    #[test]
    fn a_word_source_is_not_a_language() {
        // My Words resolves to the language those words are read in.
        assert_eq!(hub_lang(crate::consts::MINE, Some("es")), "es");
        // ...and to the default when the speak language is one we do not ship.
        assert_eq!(hub_lang(crate::consts::MINE, None), crate::consts::EN);
        // The review queue mixes languages: populated menu over an empty one.
        assert_eq!(hub_lang(crate::consts::REVIEW, None), crate::consts::EN);
        // A real language is passed straight through, untouched.
        assert_eq!(hub_lang("en", None), "en");
        // and neither pseudo-code can ever reach the gate.
        for src in [crate::consts::MINE, crate::consts::REVIEW] {
            assert!(!hub_lang(src, Some("fr")).starts_with("__"), "{src} leaked to the mode gate");
        }
    }

    /// A7 (rewritten for CC-HUB-CLEANUP, then again for CC-HUB-NAV A13):
    /// Spell It left the game menu — its front door is the home tile (spellItBtn,
    /// D1). The surface half of that law now belongs to the drawer, which
    /// renders no row for it because the registry marks it `unlisted`; what
    /// stays here is the per-language availability logic behind it
    /// (mic-everywhere: all registered languages supported).
    #[test]
    fn a7_spell_aloud_is_available_in_every_registered_language() {
        let m = spell_aloud_mode();
        for lang in ["en", "es", "fr", "de", "ja", "ar", "hi", "zh"] {
            assert!(unavailable_reason(&m, lang).is_none(), "{lang} supports voice spell");
        }
        // An unregistered code still reports unavailable (defensive), and still
        // renders no hub tile either way.
        assert!(unavailable_reason(&m, "xx").is_some(), "xx does not support voice spell");
    }
}
