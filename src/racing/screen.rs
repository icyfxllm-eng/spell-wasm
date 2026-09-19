//! The Spell Racing screen (CC-SPELL-RACING Phase 5 — UI).
//!
//! The mode's "place": pick a circuit, then an opponent — a synthetic pace ghost
//! (Phase 4) or one of your garage ghosts (Phase 3) — then start. Data-driven from
//! `track::available` (Phase 2). Pace ghosts (Bronze–Platinum; Champion gated on D6)
//! mean a brand-new garage still always has something to race.
//!
//! REVIEW-GATED: reachable only via the hidden `spellRacingOpen` entry until
//! activation. The shipped `ghost_racing` mode is untouched. Starting a live race
//! (wiring the engine into the spelling loop, `game.rs`) is the remaining Phase 5
//! integration — Start currently surfaces `racing.soon`.
//!
//! No Climb/shield references (D2).

use std::cell::RefCell;

use crate::dom;
use crate::i18n::{t, tp};
use crate::racing::{format::RaceGhost, garage, pace, track};
use crate::racing::garage::SlotKind;
use crate::racing::track::Circuit;
use crate::App;

thread_local! {
    static SELECTED: RefCell<Option<Circuit>> = const { RefCell::new(None) };
}

/// The tier to race for the current app state. The Climb has no fixed tier, so it
/// defaults to medium; an explicit difficulty level is used as-is.
fn current_tier(level: &str) -> &'static str {
    match level {
        "easy" => "easy",
        "hard" => "hard",
        "expert" => "expert",
        _ => "medium",
    }
}

fn fmt_mmss(ms: u32) -> String {
    let secs = ms / 1000;
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn slot_key(kind: SlotKind) -> &'static str {
    match kind {
        SlotKind::FirstEver => "racing.slot.firstEver",
        SlotKind::PersonalBest => "racing.slot.personalBest",
        SlotKind::MostRecent => "racing.slot.mostRecent",
    }
}

pub fn open(app: &App) {
    crate::telemetry::set_mode(crate::telemetry::schema::Mode::Racing);
    reflect(app);
    dom::add_class("spellRacing", "show");
}

pub fn close() {
    dom::remove_class("spellRacing", "show");
}

/// Render the screen from live garage + track data. Idempotent.
pub fn reflect(app: &App) {
    let (lang, level) = {
        let s = app.borrow();
        (s.lang.clone(), s.level.clone())
    };
    let tier = current_tier(&level);

    dom::set_text("srTitle", &t("racing.title"));
    dom::set_text("srCircuitLabel", &t("racing.chooseCircuit"));
    dom::set_text("srOpponentLabel", &t("racing.chooseOpponent"));

    let circuits = track::available(&lang, tier);
    if circuits.is_empty() {
        // Small/gated bank: nothing to race here.
        dom::set_html("srCircuits", &format!("<p class=\"sr-empty\">{}</p>", t("racing.noCircuits")));
        dom::set_html("srOpponents", "");
        return;
    }

    // Keep the selection valid; default to the first available circuit.
    let selected = SELECTED.with(|c| {
        let mut c = c.borrow_mut();
        if !matches!(*c, Some(sel) if circuits.contains(&sel)) {
            *c = Some(circuits[0]);
        }
        c.unwrap()
    });

    let circuit_html: String = circuits
        .iter()
        .map(|c| {
            let sel = if *c == selected { " selected" } else { "" };
            format!(
                "<button type=\"button\" class=\"sr-circuit{sel}\" data-circuit=\"{id}\">\
                 <div class=\"c-name\">{name}</div><div class=\"c-laps\">{laps}</div></button>",
                id = c.id(),
                name = dom::escape_html(&t(&format!("racing.circuit.{}", circuit_key(*c)))),
                laps = dom::escape_html(&tp("racing.laps", &[("n", &c.laps().to_string())])),
            )
        })
        .collect();
    dom::set_html("srCircuits", &circuit_html);

    // Opponents for the selected circuit: pace ghosts (always available) + garage.
    let opponents = opponents_for(&lang, tier, selected);
    if opponents.is_empty() {
        dom::set_html("srOpponents", &format!("<p class=\"sr-empty\">{}</p>", t("racing.noGhosts")));
    } else {
        let rows: String = opponents.iter().enumerate().map(|(i, o)| opponent_row(i, o)).collect();
        dom::set_html("srOpponents", &rows);
    }
}

/// One selectable opponent on the screen — a synthetic pace ghost or a garage slot.
struct Opp {
    name: String,
    tag: Option<String>,
    ghost: RaceGhost,
}

/// Representative track seed for pace opponents. Stable, so the pace ghosts are
/// deterministic and identical every time the screen opens (Phase 4 / acceptance #2).
const PACE_SEED: u64 = 0;

/// The opponents for `(lang, tier, circuit)`: the shipped pace bands (Bronze–Platinum,
/// Champion gated on D6) racing a representative track, then the player's garage
/// ghosts. Pace ghosts guarantee a fresh garage still has something to race — the
/// single source both `reflect` (render) and the Start handler (launch) index into, so
/// their orders always match.
fn opponents_for(lang: &str, tier: &str, circuit: Circuit) -> Vec<Opp> {
    let mut out: Vec<Opp> = Vec::new();
    if let Some(words) = track::generate(lang, tier, circuit, PACE_SEED) {
        let lh = crate::wordid::list_hash(lang, tier);
        for band in pace::active_bands() {
            let ghost = pace::generate(&band, &words, lang, tier, lh, PACE_SEED);
            out.push(Opp {
                name: t(&format!("racing.pace.{}", band.id)),
                tag: Some(t("racing.pace.tag")),
                ghost,
            });
        }
    }
    for (kind, g) in garage::load().opponents(lang, tier, circuit) {
        out.push(Opp { name: t(slot_key(kind)), tag: None, ghost: g });
    }
    out
}

fn circuit_key(c: Circuit) -> &'static str {
    match c {
        Circuit::Sprint => "sprint",
        Circuit::GrandPrix => "grandPrix",
        Circuit::Endurance => "endurance",
    }
}

fn opponent_row(i: usize, o: &Opp) -> String {
    let best = tp("racing.best", &[("time", &fmt_mmss(garage::total_time_ms(&o.ghost)))]);
    let tag = o
        .tag
        .as_ref()
        .map(|s| format!("<div class=\"o-tag\">{}</div>", dom::escape_html(s)))
        .unwrap_or_default();
    format!(
        "<div class=\"sr-opponent\"><div><div class=\"o-name\">{name}</div>{tag}\
         <div class=\"o-best\">{best}</div></div>\
         <button type=\"button\" class=\"sr-start\" data-op=\"{i}\">{start}</button></div>",
        name = dom::escape_html(&o.name),
        best = dom::escape_html(&best),
        start = dom::escape_html(&t("racing.start")),
    )
}

/// Wire the screen once at startup. Delegated container listeners (attached once) so
/// re-rendering the lists on selection never stacks handlers.
pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("spellRacingOpen", move || open(&a));
    dom::on_click("srClose", close);

    // The dev door (5-tap logo → dev menu) lives in `crate::dev`; it routes here.
    dom::on::<web_sys::Event, _>("spellRacing", "click", |e| {
        if dom::is_self_target(&e, "spellRacing") {
            close();
        }
    });

    // Circuit selection (delegated on the container).
    let a2 = app.clone();
    dom::on::<web_sys::Event, _>("srCircuits", "click", move |e| {
        if let Some(id) = closest_attr(&e, "data-circuit") {
            if let Some(c) = Circuit::from_id(&id) {
                SELECTED.with(|sel| *sel.borrow_mut() = Some(c));
                reflect(&a2);
            }
        }
    });

    // Start (delegated): launch a race against the picked garage ghost on the
    // selected circuit. Rides game::start_race (Daily-flow bridge).
    let a3 = app.clone();
    dom::on::<web_sys::Event, _>("srOpponents", "click", move |e| {
        let Some(idx) = closest_attr(&e, "data-op").and_then(|s| s.parse::<usize>().ok()) else {
            return;
        };
        let (lang, level) = {
            let s = a3.borrow();
            (s.lang.clone(), s.level.clone())
        };
        let tier = current_tier(&level);
        let Some(circuit) = SELECTED.with(|c| *c.borrow()) else { return };
        // Reload the same opponent list the row was rendered from (same order).
        let opponents = opponents_for(&lang, tier, circuit);
        if let Some(o) = opponents.get(idx) {
            // Only close the screen if the race actually starts (start_race returns
            // false if a word can't resolve — never substitutes).
            if crate::game::start_race(&a3, circuit, o.ghost.clone()) {
                close();
            }
        }
    });
}

/// The value of `attr` on the clicked element or its nearest ancestor that has it.
fn closest_attr(e: &web_sys::Event, attr: &str) -> Option<String> {
    use wasm_bindgen::JsCast;
    let el = e.target()?.dyn_into::<web_sys::Element>().ok()?;
    let hit = el.closest(&format!("[{attr}]")).ok()??;
    hit.get_attribute(attr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_tier_maps_levels() {
        assert_eq!(current_tier("easy"), "easy");
        assert_eq!(current_tier("hard"), "hard");
        assert_eq!(current_tier("expert"), "expert");
        assert_eq!(current_tier("medium"), "medium");
        // The Climb (and anything else) falls back to medium.
        assert_eq!(current_tier("climb"), "medium");
        assert_eq!(current_tier("whatever"), "medium");
    }

    #[test]
    fn fmt_mmss_formats() {
        assert_eq!(fmt_mmss(0), "0:00");
        assert_eq!(fmt_mmss(83_000), "1:23");
        assert_eq!(fmt_mmss(600_000), "10:00");
    }

    #[test]
    fn circuit_key_and_from_id_round_trip() {
        for c in Circuit::ALL {
            // circuit_key is the i18n stem; id() is the data-attr value.
            assert_eq!(Circuit::from_id(c.id()), Some(c));
        }
        assert_eq!(circuit_key(Circuit::GrandPrix), "grandPrix");
    }
}
