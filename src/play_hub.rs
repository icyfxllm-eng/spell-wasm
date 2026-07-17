//! CC-MODE-HUB F2 — the Play hub: one tile per visible mode, rendered from
//! `config/modes.json` and nothing else.
//!
//! # What a tile is allowed to promise
//! Three kinds, because the six things CC-MODE-HUB calls "modes" are not the
//! same kind of thing:
//!
//!   * **launcher** — the mode has a real entry point (Say It's `sayItBtn`,
//!     Spell-Off's `soBtn`). Renders as a `<button>` that clicks it.
//!   * **info** — the mode is an in-round AID with no destination. Ghost racing
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
const LAUNCH: [(&str, Option<&str>); 7] = [
    ("ghost_racing", None),       // races inside The Climb
    ("syllable_replay", None),    // fires on a miss, on the reveal surface
    ("say_it", Some("sayItBtn")), // a real session mode
    ("photo_list", None),         // a camera button on My Words
    ("spell_aloud", None),        // a mic beside the answer field
    ("word_stories", None),       // after-answer flourish; hidden anyway
    ("online_spelloff", Some("soBtn")),
];

fn launch_for(id: &str) -> Option<&'static str> {
    LAUNCH.iter().find(|(k, _)| *k == id).and_then(|(_, v)| *v)
}

/// Gather the live context the pure rule needs. The only impure part of the hub.
fn ctx(app: &App) -> modes::HubCtx {
    let (kid, lang) = {
        let s = app.borrow();
        (s.kid, s.lang.clone())
    };
    // The purchase / region adapters are later phases (CC-ENTITLEMENTS), so today
    // this resolves FREE_TIER: not purchased, no regional grants, no audit
    // override. When those adapters land they feed the same call — the hub does
    // not re-derive entitlement, it asks.
    let ent = entitlements::resolve_entitlements(false, &[], false);
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
        native: native_lang::available(),
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

fn tile_html(m: &Mode) -> String {
    let name = t(&m.name_key);
    let desc = t(&m.desc_key);
    let ico = &m.icon;
    let body = format!(
        "<span class=\"mt-ico\" aria-hidden=\"true\">{ico}</span>\
         <span class=\"mt-name\">{name}</span>\
         <small class=\"mt-desc\">{desc}</small>"
    );
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
    let html: String = shown.iter().map(tile_html).collect();
    dom::set_html("playHubGrid", &html);

    // Tapping a tile routes to the mode's OWN entry point rather than
    // reimplementing it — the hub is discovery, not a second copy of each mode.
    for m in &shown {
        if m.status != Status::Live {
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
    let _ = app;
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
        for aid in ["ghost_racing", "syllable_replay", "photo_list", "spell_aloud", "word_stories"] {
            assert_eq!(launch_for(aid), None, "{aid} is an in-round aid with no destination");
        }
    }
}
