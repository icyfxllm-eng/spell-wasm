//! CC-HUB-NAV F1/F3 — the navigation drawer. Phase B: shell + the Play group.
//!
//! Phase B renders and navigates; it changes nothing that exists. The three
//! round icons, the gamepad sheet and the utility row all stay where they are,
//! and this is reachable only in a `dev_preview` build. The cut-over is Phase C.
//!
//! Deliberately NOT here: the timer pause. F1 asks for it, but the phase table
//! puts the hook in Phase D and D-N7 is still open, so a drawer that silently
//! burned a timed round would be the exact failure that decision is guarding
//! against. Behind the flag, with no player reachable, the honest move is to
//! leave the timer alone until Eric signs rather than guess at pause semantics.
//!
//! # Why the rows cannot drift from the sheet
//!
//! A2 wants the drawer's row count to equal the registry entries passing gating
//! for the current profile, edition and language. That is true by construction
//! rather than by test: the rows come from `modes::catalog(&modes::all(), ctx)`
//! with `play_hub::ctx` — the sheet's own gate and the sheet's own context,
//! differing by one deliberate line. `catalog` admits the `core` surfaces the
//! hub tiles exclude, because D-N6 lists the base game, The Climb and the Daily
//! under "Spell it" and a catalog that omitted them would not be a map of the
//! app. There is no second filter to fall out of step.
//!
//! # Why a row does not navigate yet
//!
//! Phase B is render only: A4, the tap-goes-to-the-route test, is Phase C's.
//! That boundary earns its keep here. The Climb has no start route to wire —
//! `climbBtn` opens the LEADERBOARD, and the Climb itself is a `LEVEL_OPTS`
//! entry chosen from the setup chip — so wiring "the obvious destination"
//! would have sent a player somewhere the row did not promise. Rows close the
//! drawer; Phase C decides where each one goes, with a test to hold it.

use crate::{dom, i18n::t, modes, App};
use wasm_bindgen::JsCast;

const MARKER: &str = "navDrawer";
const PANEL: &str = "navDrawerPanel";
const BURGER: &str = "navBurger";
const CLOSE: &str = "navDrawerClose";

/// F2 asks for a fixed row height in every uiLang, because secondary text wraps
/// differently across fifteen languages and ragged rows look broken (D-N3).
const ROW_PX: u32 = 52;

/// D-N6's Play sub-headers, in the order the decision lists them.
const PLAY: [(modes::Group, &str); 3] = [
    (modes::Group::SpellIt, "nav.spellIt"),
    (modes::Group::WordPuzzles, "nav.wordPuzzles"),
    (modes::Group::Meaning, "nav.meaning"),
];

/// One drawer row. Emoji + name only (D-N3) — the tagline moved to the mode's
/// start screen, so there is deliberately no description field here.
pub struct Row {
    pub id: String,
    pub icon: String,
    pub name_key: String,
}

pub struct Section {
    pub header_key: &'static str,
    pub rows: Vec<Row>,
}

/// The Play group, grouped and ordered by D-N6. Pure, so A2 can be checked
/// without a browser. An empty section is dropped rather than rendered as a
/// bare header with nothing under it.
pub fn play_sections(shown: &[modes::Mode]) -> Vec<Section> {
    PLAY.iter()
        .map(|(group, header_key)| Section {
            header_key,
            rows: shown
                .iter()
                .filter(|m| m.group == *group)
                .map(|m| Row {
                    id: m.id.clone(),
                    icon: m.icon.clone(),
                    name_key: m.name_key.clone(),
                })
                .collect(),
        })
        .filter(|s| !s.rows.is_empty())
        .collect()
}

/// A name that already opens with its own icon should not get a second one.
/// `top.theClimb` is "🏔 The Climb" because the hub button has no separate
/// glyph slot; the drawer does, so rendering both gave "🏔🏔 The Climb".
/// Deliberately narrow: only when the first character IS the icon.
fn dedupe_icon<'a>(icon: &str, name: &'a str) -> &'a str {
    match name.strip_prefix(icon) {
        Some(rest) => rest.trim_start(),
        None => name,
    }
}

/// F4 — the player's own material and progress, off the play surface. Same
/// registry, same gate; only the group differs from Play.
pub fn your_words_rows(shown: &[modes::Mode]) -> Vec<Row> {
    shown
        .iter()
        .filter(|m| m.group == modes::Group::YourWords)
        .map(|m| Row { id: m.id.clone(), icon: m.icon.clone(), name_key: m.name_key.clone() })
        .collect()
}

/// F2 — exactly one account state, never both (I1).
///
/// The identity row is a username, never an email, so CC-ONBOARD-JR D2 holds
/// by construction. A Jr profile gets no Sign out row at all (I7): a child
/// cannot sign themselves out of a parent's account.
pub struct Account {
    pub identity: Option<String>,
    pub action: Option<(&'static str, &'static str)>, // (i18n key, element to click)
}

pub fn account(signed_in: bool, jr: bool, name: Option<String>) -> Account {
    if !signed_in {
        // D1: never a prerequisite for anything else in the drawer.
        return Account { identity: None, action: Some(("top.signIn", "accountBtn")) };
    }
    Account {
        identity: name,
        // F2 wants this labelled Sign out, and the shipped string is
        // `acct.logout` = "Log out". Renaming it is a 15-locale edit rather
        // than a new key, so the existing string is used and the rename is
        // flagged for the next round instead of inventing a seventh new key.
        action: if jr { None } else { Some(("acct.logout", "acctLogout")) },
    }
}

/// Where a row goes. The surface's own element when the registry names one,
/// else the sheet's entry point. Clicks are PROXIED to that element rather
/// than calling a handler, because every handler here is an inline closure
/// registered on its button; `climbBtn` already sets the precedent of an
/// element kept in the DOM purely so something else can click it.
fn target_for(m: &modes::Mode) -> Option<String> {
    m.hub_tile
        .as_ref()
        .map(|t| t.element.clone())
        .or_else(|| crate::play_hub::launch_for(&m.id).map(str::to_string))
}

/// One row. Shared by every group so a Your Words row cannot drift from a
/// Play row — same height, same shape, same absence of secondary text (D-N3).
fn row_html(id: &str, icon: &str, name: &str) -> String {
    format!(
        "<button type=\"button\" class=\"nav-row\" id=\"navRow_{0}\" data-mode=\"{0}\" \
         style=\"height:{1}px;min-height:{1}px;max-height:{1}px;\
         display:flex;align-items:center;gap:10px;width:100%;\
         padding:0 14px;margin:0 0 2px;text-align:start\">\
         <span class=\"nav-ico\" aria-hidden=\"true\">{2}</span>\
         <span class=\"nav-name\">{3}</span></button>",
        dom::escape_html(id), ROW_PX, dom::escape_html(icon), dom::escape_html(name),
    )
}

fn body(sections: &[Section]) -> String {
    let mut h = String::new();
    for s in sections {
        h.push_str(&format!(
            "<h3 class=\"nav-group\" role=\"heading\" aria-level=\"2\">{}</h3>",
            dom::escape_html(&t(s.header_key))
        ));
        for r in &s.rows {
            h.push_str(&format!(
                // Width and display are pinned here for the same reason the
                // height is: these buttons sit inside `.modal`, whose own
                // button styling makes them shrink-to-fit cards that flow side
                // by side. Measuring height alone said the rows were fine and
                // a screenshot said otherwise.
                "<button type=\"button\" class=\"nav-row\" id=\"navRow_{0}\" data-mode=\"{0}\" \
                 style=\"height:{1}px;min-height:{1}px;max-height:{1}px;\
                 display:flex;align-items:center;gap:10px;width:100%;\
                 padding:0 14px;margin:0 0 2px;text-align:start\">\
                 <span class=\"nav-ico\" aria-hidden=\"true\">{2}</span>\
                 <span class=\"nav-name\">{3}</span></button>",
                dom::escape_html(&r.id),
                ROW_PX,
                dom::escape_html(&r.icon),
                dom::escape_html(dedupe_icon(&r.icon, &t(&r.name_key))),
            ));
        }
    }
    h
}

fn is_open() -> bool {
    dom::doc()
        .get_element_by_id(MARKER)
        .map(|e| e.class_name().contains("show"))
        .unwrap_or(false)
}

pub fn close() {
    dom::remove_class(MARKER, "show");
    if let Some(b) = dom::doc().body() {
        let _ = b.class_list().toggle_with_force("hub-open", false);
    }
}

pub fn open(app: &App) {
    render(app);
    dom::add_class(MARKER, "show");
    // The page behind must not scroll under the drawer (F1). Same body class
    // the sheet uses, so the two can never both think they own the scroll.
    if let Some(b) = dom::doc().body() {
        let _ = b.class_list().toggle_with_force("hub-open", true);
    }
    // First focus lands on the X (F1).
    if let Some(x) = dom::doc().get_element_by_id(CLOSE) {
        if let Ok(h) = x.dyn_into::<web_sys::HtmlElement>() {
            let _ = h.focus();
        }
    }
}

fn render(app: &App) {
    let all = modes::all();
    let shown = modes::catalog(&all, &crate::play_hub::ctx(app));
    let (jr, signed_in) = (app.borrow().kid, crate::climb::is_logged_in());
    let acct = account(signed_in, jr, crate::climb::username());
    let play = play_sections(&shown);
    let yours = your_words_rows(&shown);

    let mut h = format!(
        "<div class=\"nav-head\"><button type=\"button\" class=\"ghost hub-x\" id=\"{CLOSE}\" \
         aria-label=\"{}\">\u{2715}</button></div>",
        dom::escape_html(&t("aria.close"))
    );
    // Account first (F2), then Play, then Your Words. Help & Settings is F5,
    // which is Phase D.
    if let Some(name) = &acct.identity {
        h.push_str(&format!(
            "<p class=\"nav-identity\" style=\"padding:0 14px;opacity:.8\">{}</p>",
            dom::escape_html(name)
        ));
    }
    if let Some((key, _)) = acct.action {
        // Same dedupe as a mode row: top.signIn already carries its own glyph.
        let label = t(key);
        h.push_str(&row_html("acct", "\u{1F464}", dedupe_icon("\u{1F464}", &label)));
    }
    h.push_str(&body(&play));
    if !yours.is_empty() {
        h.push_str(&format!(
            "<h3 class=\"nav-group\" role=\"heading\" aria-level=\"2\">{}</h3>",
            dom::escape_html(&t("aria.yourWords"))
        ));
        for r in &yours {
            h.push_str(&row_html(&r.id, &r.icon, dedupe_icon(&r.icon, &t(&r.name_key))));
        }
    }
    dom::set_html(
        PANEL,
        &format!(
            "<div style=\"max-height:100%;padding:14px 0\">{h}</div>"
        ),
    );
    dom::on_click(CLOSE, close);

    // Account action proxies to its existing control.
    if let Some((_, element)) = acct.action {
        let el = element.to_string();
        dom::on_click(&format!("navRow_acct"), move || {
            close();
            dom::click(&el);
        });
    }
    // Play rows still go nowhere: A4 is Phase C's test and the routes are not
    // all decided (The Climb has none). Your Words rows DO route, because the
    // cut-over removes their only other door.
    for sct in &play {
        for r in &sct.rows {
            dom::on_click(&format!("navRow_{}", r.id), close);
        }
    }
    for r in &yours {
        let Some(m) = shown.iter().find(|m| m.id == r.id) else { continue };
        let target = target_for(m);
        dom::on_click(&format!("navRow_{}", r.id), move || {
            close();
            if let Some(t) = &target {
                dom::click(t);
            }
        });
    }
}

/// Build the drawer and its burger once at startup.
///
/// `dev_preview` only in Phase B: this is the single thing that makes the
/// drawer reachable, so with it compiled out no player build has a burger to
/// press and nothing that exists today moves. The module around it is NOT
/// gated, so `play_sections` is covered by the ordinary gate rather than by a
/// feature nobody runs tests with.
#[cfg(feature = "dev_preview")]
pub fn wire(app: &App) {
    let doc = dom::doc();

    if doc.get_element_by_id(MARKER).is_none() {
        if let (Ok(scrim), Some(body_el)) = (doc.create_element("div"), doc.body()) {
            scrim.set_id(MARKER);
            let _ = scrim.set_attribute("class", "scrim nav-scrim");
            let _ = scrim.set_attribute(
                "style",
                "justify-content:flex-end;align-items:stretch;padding:0",
            );
            if let Ok(panel) = doc.create_element("div") {
                panel.set_id(PANEL);
                let _ = panel.set_attribute("class", "modal nav-panel");
                let _ = panel.set_attribute("role", "dialog");
                let _ = panel.set_attribute("aria-modal", "true");
                let _ = panel.set_attribute(
                    "style",
                    // D-N2: slides from the right edge. Its own scroller, so
                    // the page behind never becomes the one that moves.
                    "width:min(320px,85vw);max-width:min(320px,85vw);height:100%;\
                     margin:0;border-radius:0;overflow-y:auto;overflow-x:hidden;\
                     display:block;text-align:start",
                );
                let _ = scrim.append_child(&panel);
            }
            let _ = body_el.append_child(&scrim);
        }
    }

    // Scrim tap closes; a tap inside the panel must not (F1).
    dom::on::<web_sys::MouseEvent, _>(MARKER, "click", |e| {
        if dom::is_self_target(e.as_ref(), MARKER) {
            close();
        }
    });

    // D-N2: top-right, the reachable corner and the same one the retired
    // sheet's X uses. Phase B adds it beside the existing icons rather than
    // replacing them — the three icons go in Phase C.
    if let Some(corner) = doc.query_selector(".meta-corner").ok().flatten() {
        if doc.get_element_by_id(BURGER).is_none() {
            if let Ok(b) = doc.create_element("button") {
                b.set_id(BURGER);
                let _ = b.set_attribute("type", "button");
                let _ = b.set_attribute("class", "icon-btn");
                let _ = b.set_attribute("aria-label", &t("nav.menu"));
                b.set_text_content(Some("\u{2630}"));
                let _ = corner.append_child(&b);
            }
        }
    }
    let a = app.clone();
    dom::on_click(BURGER, move || open(&a));

    // Escape closes (F1, web). Tab is trapped inside the panel while open.
    dom::on_window::<web_sys::KeyboardEvent, _>("keydown", move |e| {
        if !is_open() {
            return;
        }
        match e.key().as_str() {
            "Escape" => close(),
            "Tab" => {
                let Some(panel) = dom::doc().get_element_by_id(PANEL) else { return };
                let Ok(list) = panel.query_selector_all("button:not([disabled])") else { return };
                let n = list.length();
                if n == 0 {
                    return;
                }
                // Compared by id rather than node identity: every control the
                // panel renders has one, and it keeps this free of casts.
                let active = dom::doc().active_element().map(|e| e.id()).unwrap_or_default();
                let at = |i: u32| {
                    list.item(i)
                        .and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok())
                        .map(|h| h.id())
                        .is_some_and(|id| !id.is_empty() && id == active)
                };
                let focus = |i: u32| {
                    if let Some(h) = list.item(i).and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok()) {
                        let _ = h.focus();
                    }
                };
                if e.shift_key() && at(0) {
                    e.prevent_default();
                    focus(n - 1);
                } else if !e.shift_key() && at(n - 1) {
                    e.prevent_default();
                    focus(0);
                }
            }
            _ => {}
        }
    });
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::entitlements::AccessLevel;
    use crate::modes::{Group, HubCtx};

    fn ctx() -> HubCtx {
        HubCtx {
            kid: false,
            native: true,
            lang: "en".to_string(),
            level: AccessLevel::Full,
            premium: vec!["photo_ocr".to_string()],
            enabled: modes::all().iter().map(|m| m.id.clone()).collect(),
        }
    }

    /// A2's arithmetic, on the pure half. Every mode the gate lets through and
    /// that D-N6 puts in Play appears exactly once — no mode is dropped by the
    /// grouping, and none is rendered twice.
    #[test]
    fn every_permitted_play_mode_appears_exactly_once() {
        let all = modes::all();
        let shown = modes::catalog(&all, &ctx());
        let sections = play_sections(&shown);

        let mut got: Vec<String> = sections.iter().flat_map(|s| s.rows.iter().map(|r| r.id.clone())).collect();
        let mut want: Vec<String> = shown
            .iter()
            .filter(|m| matches!(m.group, Group::SpellIt | Group::WordPuzzles | Group::Meaning))
            .map(|m| m.id.clone())
            .collect();
        got.sort();
        want.sort();
        assert_eq!(got, want, "the drawer must render exactly the permitted Play modes");
    }

    /// Nothing outside Play leaks into this group: Your Words is F4 and Phase
    /// C, and `unlisted` is not in the drawer at all.
    #[test]
    fn your_words_and_unlisted_never_appear() {
        let all = modes::all();
        let shown = modes::catalog(&all, &ctx());
        for s in play_sections(&shown) {
            for r in s.rows {
                let m = shown.iter().find(|m| m.id == r.id).unwrap();
                assert!(matches!(m.group, Group::SpellIt | Group::WordPuzzles | Group::Meaning),
                        "{} is {:?} and must not be in the Play group", m.id, m.group);
            }
        }
    }

    #[test]
    fn a_name_that_opens_with_its_own_icon_does_not_get_two() {
        assert_eq!(dedupe_icon("\u{1F3D4}", "\u{1F3D4} The Climb"), "The Climb");
        // Untouched when the glyph is not the icon, or is part of the phrase.
        assert_eq!(dedupe_icon("\u{1F4F7}", "Photo \u{2192} word list"), "Photo \u{2192} word list");
        assert_eq!(dedupe_icon("\u{1F331}", "Practice"), "Practice");
    }

    /// I1: the two account states never coexist.
    #[test]
    fn exactly_one_account_state_renders() {
        let out = account(false, false, None);
        assert_eq!(out.action.map(|a| a.0), Some("top.signIn"));
        assert!(out.identity.is_none(), "signed out shows no identity");

        let inn = account(true, false, Some("ada".into()));
        assert_eq!(inn.identity.as_deref(), Some("ada"));
        assert_eq!(inn.action.map(|a| a.0), Some("acct.logout"));
    }

    /// I7 / CC-ONBOARD-JR D2: a Jr profile gets no Sign out, and the identity
    /// it shows is a username, so it can never be an email.
    #[test]
    fn a_jr_profile_has_no_sign_out_and_no_email() {
        let jr = account(true, true, Some("Sam".into()));
        assert!(jr.action.is_none(), "a child cannot sign out of a parent's account");
        assert!(!jr.identity.unwrap_or_default().contains('@'));
    }

    /// F4 renders the player's own surfaces, including the two that are
    /// surfaces rather than modes and exist in the registry only so a drawer
    /// row can resolve through it (I2).
    #[test]
    fn your_words_holds_the_players_own_surfaces() {
        let all = modes::all();
        let shown = modes::catalog(&all, &ctx());
        let ids: Vec<String> = your_words_rows(&shown).iter().map(|r| r.id.clone()).collect();
        for want in ["my_words", "misses", "translate", "calendar"] {
            assert!(ids.iter().any(|i| i == want), "{want} belongs in Your Words; got {ids:?}");
        }
        // Play modes must not leak into it.
        assert!(!ids.iter().any(|i| i == "practice"));
    }

    /// Every Your Words row must have somewhere to go, or the cut-over would
    /// strand the surface it replaced — My Words has no other door once the
    /// utility row is gone.
    #[test]
    fn every_your_words_row_resolves_to_a_target() {
        let all = modes::all();
        let shown = modes::catalog(&all, &ctx());
        for r in your_words_rows(&shown) {
            let m = shown.iter().find(|m| m.id == r.id).unwrap();
            assert!(target_for(m).is_some(), "{} has no target; its row would go nowhere", r.id);
        }
    }

    /// D-N6's order, which is the order a player reads.
    #[test]
    fn sections_are_in_the_signed_order() {
        let all = modes::all();
        let shown = modes::catalog(&all, &ctx());
        let keys: Vec<&str> = play_sections(&shown).iter().map(|s| s.header_key).collect();
        assert_eq!(keys, ["nav.spellIt", "nav.wordPuzzles", "nav.meaning"]);
    }

    /// A header with nothing under it is a promise the drawer cannot keep. A
    /// Preview language can filter a whole section away.
    #[test]
    fn an_empty_section_is_dropped_not_rendered() {
        let all = modes::all();
        let none = HubCtx { enabled: vec![], ..ctx() };
        assert!(play_sections(&modes::catalog(&all, &none)).is_empty());
    }

    /// D-N3: emoji + name, and nothing else. The tagline moved to the mode's
    /// start screen, so a row that grew a description would be a silent
    /// reversal of a signed decision.
    #[test]
    fn a_row_carries_no_secondary_text() {
        let all = modes::all();
        let shown = modes::catalog(&all, &ctx());
        let html = body(&play_sections(&shown));
        assert!(!html.contains("mt-desc") && !html.contains("nav-desc"),
                "a drawer row must not render a description (D-N3)");
        // Every row is pinned to the fixed height in the markup itself, so the
        // measurement A6 takes cannot disagree with the intent.
        let rows = html.matches("class=\"nav-row\"").count();
        let pin = format!("height:{ROW_PX}px;min-height:{ROW_PX}px;max-height:{ROW_PX}px");
        assert_eq!(html.matches(&pin).count(), rows, "every row pins its height in the markup");
        // The height was right and the LAYOUT was wrong: inside `.modal` these
        // buttons default to shrink-to-fit cards that flow two to a line. A
        // screenshot caught what measuring height could not, so the width and
        // display are pinned as deliberately as the height.
        assert_eq!(html.matches("display:flex;align-items:center;gap:10px;width:100%").count(), rows,
                   "every row must be a full-width list row, not a shrink-to-fit card");
        assert!(rows > 0, "the fixture registry should produce rows");
    }
}
