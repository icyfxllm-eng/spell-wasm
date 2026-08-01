//! CC-MODE-HUB F2 — the Play hub: one tile per visible mode, rendered from
//! `config/modes.json` and nothing else.
//!
//! # What a tile is allowed to promise
//! Three kinds, because the six things CC-MODE-HUB calls "modes" are not the
//! same kind of thing:
//!
//!   * **launcher** — the mode has a real entry point (Say It's `sayItBtn`,
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
const N_LAUNCH: usize = 10;
#[cfg(feature = "web")]
const N_LAUNCH: usize = 9;
const LAUNCH: [(&str, Option<&str>); N_LAUNCH] = [
    ("practice", Some("practiceOpen")), // CC-PRACTICE: the front porch, first (D9)
    // Races inside The Climb, but HAS its own screen: it shows the ghost you're
    // racing (your best run for this language) and starts a Climb run.
    ("ghost_racing", Some("ghostOpenBtn")),
    ("syllable_replay", None),    // fires on a miss, on the reveal surface
    ("say_it", Some("sayItBtn")), // a real session mode
    ("photo_list", None),         // a camera button on My Words
    ("spell_aloud", Some("spellAloudEnter")), // promoted to a real mode (G-INT-1): enters play with the voice mic
    ("word_stories", None),       // after-answer flourish; hidden anyway
    ("online_spelloff", Some("soBtn")),
    ("def_match", Some("defMatchOpen")), // CC-DEF-MATCH P3: the floating-cards loop
    // CC-WORD-PICTURE v5: calligram picker. Compiled out with the mode
    // (I1): the registry row is deleted on web, so this would be a dead
    // table entry -- but a dead entry still puts the mode's name in the
    // bundle, which is exactly what the wall scan polices.
    #[cfg(not(feature = "web"))]
    ("word_picture", Some("wordPicOpen")),
];

fn launch_for(id: &str) -> Option<&'static str> {
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
fn ctx(app: &App) -> modes::HubCtx {
    let (kid, lang) = {
        let s = app.borrow();
        (s.kid, s.lang.clone())
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
    if ent.custom_lists_unlimited() {
        premium.push("custom_lists_unlimited".to_string());
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

/// A mode's availability for the CURRENT language. `spell_aloud` is voice-spell-gated:
/// live only where the language registry's `voice_spell` flag holds (en/es); elsewhere
/// it renders as an "unavailable / coming soon" tile — shown, not hidden (A7).
fn unavailable_reason(m: &Mode, lang: &str) -> Option<&'static str> {
    if m.id == "spell_aloud" && !crate::consts::voice_spell(lang) {
        Some("tools.spellaloud.avail") // "iPhone · English or Spanish"
    } else if m.id == "practice" && !crate::consts::practice(lang) {
        // D7: curriculum drafted but not yet cleared for this language.
        Some("tools.practice.avail")
    } else if m.id == "def_match" && !crate::consts::def_match(lang) {
        // Pool not landed/pinned for this language yet (Invariant 8) — a
        // non-tappable teaser, never a dead button.
        Some("tools.defmatch.avail")
    } else {
        None
    }
}

fn tile_html(m: &Mode, lang: &str) -> String {
    let name = t(&m.name_key);
    let ico = &m.icon;
    // An unavailable (language-gated) mode shows a one-line reason instead of the desc.
    let desc = match unavailable_reason(m, lang) {
        Some(reason_key) => t(reason_key),
        None => t(&m.desc_key),
    };
    let body = format!(
        "<span class=\"mt-ico\" aria-hidden=\"true\">{ico}</span>\
         <span class=\"mt-name\">{name}</span>\
         <small class=\"mt-desc\">{desc}</small>"
    );
    // Voice-spell unavailable for this language → a non-interactive teaser, never the
    // dead-end screen and never a live button (A7).
    if unavailable_reason(m, lang).is_some() {
        return format!("<div class=\"mode-tile teaser\" data-mode=\"{}\">{body}</div>", m.id);
    }
    match (m.status, launch_for(&m.id)) {
        // A teaser is never interactive, and carries no notify-me hook (D7).
        (Status::ComingSoon, _) => format!("<div class=\"mode-tile teaser\" data-mode=\"{}\">{body}</div>", m.id),
        // A real destination: a button that routes to the existing entry point.
        (Status::Live, Some(_)) => format!(
            "<button type=\"button\" class=\"mode-tile\" id=\"modeTile_{0}\" data-mode=\"{0}\">{body}</button>",
            m.id
        ),
        // An aid: informational, deliberately not a button.
        (Status::Live, None) => format!("<div class=\"mode-tile info\" data-mode=\"{}\">{body}</div>", m.id),
        (Status::Hidden, _) => String::new(), // unreachable: `visible` filtered it
    }
}

/// Render the hub and wire each launcher tile. Idempotent — safe to call on any
/// state change (Kid Mode, language, flags), which is how the hub stays correct
/// without anyone remembering to refresh it.
pub fn reflect(app: &App) {
    let all = modes::all();
    let shown = modes::visible(&all, &ctx(app));
    // Never render an empty box. Every mode can legitimately filter out at once —
    // e.g. a Preview language on web: ghost_racing/online_spelloff need `full`,
    // syllable_replay is es-only, and say_it/photo_list/spell_aloud are iOS-only.
    // Without this the hub opened as a blank panel with no explanation.
    let lang = app.borrow().lang.clone();
    let html: String = if shown.is_empty() {
        format!("<p class=\"hub-empty\">{}</p>", t("hub.empty"))
    } else {
        shown.iter().map(|m| tile_html(m, &lang)).collect()
    };
    dom::set_html("playHubGrid", &html);

    // Tapping a tile routes to the mode's OWN entry point rather than
    // reimplementing it — the hub is discovery, not a second copy of each mode.
    for m in &shown {
        if m.status != Status::Live {
            continue;
        }
        // A language-unavailable mode is a teaser, not a live button — don't wire it.
        if unavailable_reason(m, &lang).is_some() {
            continue;
        }
        if let Some(target) = launch_for(&m.id) {
            let tile = format!("modeTile_{}", m.id);
            let target = target.to_string();
            dom::on_click(&tile, move || {
                close();
                dom::click(&target);
            });
        }
    }
}

pub fn open(app: &App) {
    reflect(app);
    dom::add_class("playHub", "show");
}

pub fn close() {
    dom::remove_class("playHub", "show");
}

/// Wire the hub's entry + dismiss once at startup.
pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("playHubBtn", move || open(&a));
    dom::on_click("playHubClose", close);
    dom::on_click("playHubScrim", close);
}

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
    #[test]
    fn launch_table_covers_every_registered_mode() {
        let ids: Vec<String> = modes::all().iter().map(|m| m.id.clone()).collect();
        let bound: Vec<String> = LAUNCH.iter().map(|(k, _)| k.to_string()).collect();
        assert_eq!(bound, ids, "LAUNCH must list every mode, in registry order");
    }

    /// Only genuine session modes get a destination. If this changes, someone has
    /// decided an in-round aid is tappable — which needs a real answer to "tap it
    /// and what happens?", not a silent edit.
    #[test]
    fn only_session_modes_have_a_destination() {
        assert_eq!(launch_for("say_it"), Some("sayItBtn"));
        assert_eq!(launch_for("online_spelloff"), Some("soBtn"));
        // Spell Racing got a real destination deliberately: tapping it opens the
        // ghost screen (ghost::wire_screen), which shows the best run you're
        // racing for this language and routes into The Climb. It is no longer a
        // tile that goes nowhere — which is exactly what this test guards.
        assert_eq!(launch_for("ghost_racing"), Some("ghostOpenBtn"));
        // Spell Aloud is now a real mode (CC-SPELL-ALOUD-INTEGRATION G-INT-1): tapping
        // it enters play with the voice mic. It is no longer an in-round aid.
        assert_eq!(launch_for("spell_aloud"), Some("spellAloudEnter"));
        for aid in ["syllable_replay", "photo_list", "word_stories"] {
            assert_eq!(launch_for(aid), None, "{aid} is an in-round aid with no destination");
        }
    }

    fn spell_aloud_mode() -> Mode {
        modes::all().into_iter().find(|m| m.id == "spell_aloud").expect("spell_aloud in registry")
    }

    /// A7 (rewritten for CC-HUB-CLEANUP): Spell It left the game menu — its
    /// front door is the home tile (sayItBtn, D1) — so its hub tile renders as
    /// NOTHING for every language. The per-language availability logic stays
    /// intact behind it (mic-everywhere: all registered languages supported).
    #[test]
    fn a7_spell_aloud_tile_is_hidden_from_the_hub_for_everyone() {
        let m = spell_aloud_mode();
        for lang in ["en", "es", "fr", "de", "ja", "ar", "hi", "zh"] {
            assert!(unavailable_reason(&m, lang).is_none(), "{lang} supports voice spell");
            assert!(tile_html(&m, lang).is_empty(), "{lang}: no hub tile (home-tile front door)");
        }
        // An unregistered code still reports unavailable (defensive), and still
        // renders no hub tile either way.
        assert!(unavailable_reason(&m, "xx").is_some(), "xx does not support voice spell");
    }
}
