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
//! # Where a row goes
//!
//! Every row routes (A4), and it has to: Phase C retires the gamepad sheet,
//! so for most modes this row is the only remaining door. Three shapes, in
//! [`Route`] — press the mode's launcher, pick a level, or arrive by closing.
//! The Climb is the reason that enum exists rather than a bare element id.
//! `climbBtn` is what the registry names for its hub TILE, and it opens the
//! LEADERBOARD; the Climb itself is a `LEVEL_OPTS` entry chosen from the setup
//! chip. Resolving the row "the obvious way" would have sent the player to a
//! scoreboard instead of a run, so the row sets the level and lets the one
//! change handler in lib.rs do everything that follows.

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
pub fn play_sections(shown: &[modes::Mode], lang: &str) -> Vec<Section> {
    PLAY.iter()
        .map(|(group, header_key)| Section {
            header_key,
            rows: shown
                .iter()
                .filter(|m| m.group == *group)
                // A mode with no board, pool or curriculum in THIS language is
                // absent, not greyed (I3). The retired sheet showed it as a
                // dashed teaser explaining why; a drawer row has no second line
                // to explain anything in (D-N3), so an unavailable mode would
                // be a row that opens onto nothing. Same predicate the sheet
                // used, so the two can never disagree about availability.
                .filter(|m| crate::play_hub::unavailable_reason(m, lang).is_none())
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
///
/// v1.3.1 F3 puts Misses first. It is the only row here whose contents change
/// on their own, and it is the one the burger's nudge dot is about, so it
/// leads rather than sitting wherever the registry file happens to list it.
pub fn your_words_rows(shown: &[modes::Mode]) -> Vec<Row> {
    let mut rows: Vec<Row> = shown
        .iter()
        .filter(|m| m.group == modes::Group::YourWords)
        .map(|m| Row { id: m.id.clone(), icon: m.icon.clone(), name_key: m.name_key.clone() })
        .collect();
    if let Some(i) = rows.iter().position(|r| r.id == "misses") {
        let misses = rows.remove(i);
        rows.insert(0, misses);
    }
    rows
}

/// F3's badge, as text. `999+` above the cap, and nothing at all at zero --
/// a badge reading 0 is noise on a row that is already telling you there is
/// nothing to do.
///
/// The number is the DUE count (Eric, 2026-09-27), the same one the retired
/// chip showed and the same one tapping the row will actually serve. A badge
/// showing the total saved would offer 181 and hand you three.
pub fn badge_text(due: usize) -> Option<String> {
    match due {
        0 => None,
        n if n > 999 => Some("999+".to_string()),
        n => Some(n.to_string()),
    }
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
    if jr {
        // "Kid Mode / age-locked: no accounts or leaderboard at all"
        // (climb::reflect_auth, which hides accountBtn on exactly this
        // condition). Before the drawer that rule was enforced by hiding the
        // icon; a drawer row would have walked straight around it and put a
        // Sign in door in front of a child, which CC-ONBOARD-JR I1/I2 forbid.
        // A signed-in grown-up's username still shows -- it is a username,
        // never an email (D2) -- but nothing on this row acts on the account.
        return Account { identity: if signed_in { name } else { None }, action: None };
    }
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
        action: Some(("acct.logout", "acctLogout")),
    }
}

/// F5 — the things a player looks for when stuck, in the last place they
/// scroll to.
///
/// Rows are (i18n key, element to click), and a row whose screen has not
/// shipped is ABSENT rather than a placeholder, which is F5's own rule for
/// Credits. "How to play" is absent for exactly that reason: it wants a
/// per-mode index that does not exist yet — only SpellDoku has its own
/// explainer — so offering the row would promise a screen nobody built.
///
/// Settings is here because Phase C retires the gear icon. Without this row
/// the cut-over would strand app settings entirely, which is why F5 could not
/// wait for Phase D the way the phase table implies.
pub fn help_rows() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut v = vec![("settings.title", "\u{2699}", "setBtn")];
    if dom::doc().get_element_by_id("creditsBtn").is_some() {
        v.push(("credits.title", "\u{1F399}", "creditsBtn"));
    }
    v
}

// ---------------------------------------------------------------- F6, the nudge

/// The watermark: the miss total as of the last time the drawer was opened.
/// Local, per profile, never transmitted, and it adds no telemetry event
/// (CC-TELEMETRY-FOUNDATION v1.1 I3).
const NUDGE_KEY: &str = "spell_nav_nudge_v1";

thread_local! {
    /// The last miss total the app told us about. `open()` needs it to set the
    /// watermark, and the drawer must not read the learner store itself (I8),
    /// so it is pushed in by `reflect_nudge` rather than pulled.
    static LAST_TOTAL: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// I-N7, as a pure function so the property test can enumerate every resolver
/// output instead of trusting a hand-picked pair.
///
/// Unknown age counts as Junior. That is the whole reason this takes
/// `age_known` rather than just an `Experience`: a player who has not answered
/// the gate resolves to Standard by the toggle, and nudging them would be
/// nudging a child we have simply not identified yet.
pub fn nudge_allowed(exp: crate::experience::Experience, age_known: bool) -> bool {
    age_known && exp == crate::experience::Experience::Standard
}

/// True when there are misses the player has not seen the drawer since.
///
/// The watermark defaults to the CURRENT total the first time it is asked,
/// never to zero: D-N17 says a player sitting on 181 old misses sees no dot,
/// and a fresh install of this feature on an old profile is exactly that
/// player. Zero would have lit the dot for everyone with a backlog, once, for
/// no reason.
pub fn nudge_due(total: usize, watermark: Option<usize>) -> bool {
    match watermark {
        Some(w) => total > w,
        None => false,
    }
}

fn watermark() -> Option<usize> {
    crate::storage::get_raw(NUDGE_KEY).and_then(|v| v.parse::<usize>().ok())
}

fn set_watermark(total: usize) {
    crate::storage::set_raw(NUDGE_KEY, &total.to_string());
}

/// Paint or remove the dot. Called by `game::refresh_mode_buttons`, which is
/// where the miss total already lives — the drawer is TOLD the number rather
/// than reading the learner store for it (I8).
pub fn reflect_nudge(total: usize, kid: bool) {
    LAST_TOTAL.with(|c| c.set(Some(total)));
    if watermark().is_none() {
        set_watermark(total); // first sight of this profile: everything is old
    }
    let exp = crate::experience::resolve(kid, crate::agegate::is_kid_locked()).experience;
    let allowed = nudge_allowed(exp, crate::agegate::stored().is_some());
    let show = allowed && nudge_due(total, watermark());
    let Some(b) = dom::doc().get_element_by_id(BURGER) else { return };
    let existing = b.query_selector(".nudge").ok().flatten();
    match (show, existing) {
        // I-N7: in Spell Jr the view does not EXIST, which is stronger than
        // hidden and is what the property test looks for.
        (false, Some(node)) => { let _ = node.remove(); }
        (true, None) => {
            if let Ok(d) = dom::doc().create_element("span") {
                let _ = d.set_attribute("class", "nudge");
                let _ = d.set_attribute("aria-hidden", "true");
                let _ = b.append_child(&d);
            }
        }
        _ => {}
    }
}

/// Opening the drawer is what clears it (D-N18) — visiting Misses is not
/// required, because the dot exists to get the drawer opened, not to force a
/// review.
fn clear_nudge() {
    if let Some(t) = LAST_TOTAL.with(|c| c.get()) {
        set_watermark(t);
    }
    if let Some(b) = dom::doc().get_element_by_id(BURGER) {
        if let Some(node) = b.query_selector(".nudge").ok().flatten() {
            let _ = node.remove();
        }
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

/// What a Play row actually does. Two of them are not a button press, and
/// pretending otherwise is how the drawer would lie about where it goes.
#[derive(Debug, PartialEq, Eq)]
pub enum Route {
    /// Proxy a click to the mode's launcher — every real screen.
    Press(String),
    /// Already there. The base game IS the surface the drawer opens over, and
    /// the burger only exists on it, so closing the drawer has arrived.
    Home,
    /// Pick a level. The Climb is not a screen: it is `LEVEL_OPTS[0]`, chosen
    /// in the setup sheet. `climbBtn` — the element the registry names for its
    /// hub TILE — opens the LEADERBOARD, which is a different place entirely,
    /// so a Play row that pressed it would take the player somewhere the row
    /// did not say. The row sets the level instead and lets the one change
    /// handler in lib.rs do the rest.
    Level(&'static str),
}

pub fn route_for(m: &modes::Mode) -> Route {
    if m.id == "climb" {
        return Route::Level("climb");
    }
    match target_for(m) {
        Some(el) => Route::Press(el),
        None => Route::Home,
    }
}

fn follow(r: &Route) {
    match r {
        Route::Press(el) => dom::click(el),
        Route::Home => {}
        Route::Level(v) => {
            // Only if the selector really offers it: offered_levels filters by
            // the Jr resolver, and forcing a value the select does not hold
            // would put app state and the control out of step.
            let offered = dom::doc()
                .query_selector(&format!("#levelSel option[value=\"{v}\"]"))
                .ok()
                .flatten()
                .is_some();
            if offered {
                dom::select("levelSel").set_value(v);
                dom::change("levelSel");
            }
        }
    }
}

/// One row. Shared by every group so a Your Words row cannot drift from a
/// Play row — same height, same shape, same absence of secondary text (D-N3).
fn row_html(id: &str, icon: &str, name: &str) -> String {
    row_html_with_badge(id, icon, name, None)
}

/// The badge is fixed-width (F3) so a row does not reflow between 9 and 10 due
/// words, and `margin-inline-start:auto` pins it to the trailing edge in both
/// writing directions.
fn row_html_with_badge(id: &str, icon: &str, name: &str, badge: Option<&str>) -> String {
    let badge_html = match badge {
        Some(b) => format!(
            "<b class=\"nav-badge\" style=\"margin-inline-start:auto;min-width:34px;\
             text-align:center;font-variant-numeric:tabular-nums\">{}</b>",
            dom::escape_html(b)
        ),
        None => String::new(),
    };
    format!(
        "<button type=\"button\" class=\"nav-row\" id=\"navRow_{0}\" data-mode=\"{0}\" \
         style=\"height:{1}px;min-height:{1}px;max-height:{1}px;\
         display:flex;align-items:center;gap:10px;width:100%;\
         padding:0 14px;margin:0 0 2px;text-align:start\">\
         <span class=\"nav-ico\" aria-hidden=\"true\">{2}</span>\
         <span class=\"nav-name\">{3}</span>{4}</button>",
        dom::escape_html(id), ROW_PX, dom::escape_html(icon), dom::escape_html(name), badge_html,
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

/// Whether an element is disabled right now. A DOM read, deliberately: it is
/// how the drawer honours I3 without touching the learner store (I8).
/// F3's badge text, taken from the chip the hub retired.
fn misses_badge_from_dom() -> Option<String> {
    let n = dom::doc()
        .get_element_by_id("missesBtn")?
        .query_selector(".dc-badge")
        .ok()
        .flatten()?
        .text_content()?
        .trim()
        .parse::<usize>()
        .ok()?;
    badge_text(n)
}

fn element_is_disabled(id: &str) -> bool {
    dom::doc()
        .get_element_by_id(id)
        .map(|e| e.has_attribute("disabled"))
        .unwrap_or(false)
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
    clear_nudge();
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
    let ctx = crate::play_hub::ctx(app);
    let shown = modes::catalog(&all, &ctx);
    let (jr, signed_in) = (app.borrow().kid, crate::climb::is_logged_in());
    let acct = account(signed_in, jr, crate::climb::username());
    // The context's language, never app.lang: ctx has already resolved the
    // word-SOURCE pseudo-codes (__mine, __review) to a real language, and
    // handing the raw one to the availability filter would drop every
    // language-gated mode the instant a player opened their own word list --
    // the same bug one layer down.
    let play = play_sections(&shown, &ctx.lang);
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
    // I3 wants absent, never greyed; I8 forbids the drawer any learner read.
    // Both hold at once by asking the DOM rather than the queue: game.rs
    // already disables missesBtn when nothing is due, so a disabled element is
    // the learner's answer, second-hand and read-only. Without this the row
    // rendered and did nothing, because a disabled button fires no click.
    let yours: Vec<Row> = yours
        .into_iter()
        .filter(|r| match target_for(shown.iter().find(|m| m.id == r.id).unwrap()) {
            Some(el) => !element_is_disabled(&el),
            None => true,
        })
        .collect();
    if !yours.is_empty() {
        h.push_str(&format!(
            "<h3 class=\"nav-group\" role=\"heading\" aria-level=\"2\">{}</h3>",
            dom::escape_html(&t("aria.yourWords"))
        ));
        for r in &yours {
            let name = t(&r.name_key);
            let label = dedupe_icon(&r.icon, &name);
            // F3/I-N5: one count, and the drawer does not compute it. The
            // retired chip is still rendered by game::refresh_mode_buttons, so
            // reading its badge is reading the same number the same store
            // produced -- second-hand and read-only, which is how I8 and a
            // live count hold at once.
            let badge = if r.id == "misses" { misses_badge_from_dom() } else { None };
            h.push_str(&row_html_with_badge(&r.id, &r.icon, label, badge.as_deref()));
        }
    }
    let help = help_rows();
    if !help.is_empty() {
        h.push_str(&format!(
            "<h3 class=\"nav-group\" role=\"heading\" aria-level=\"2\">{}</h3>",
            dom::escape_html(&t("settings.title"))
        ));
        for (key, icon, _) in &help {
            let label = t(key);
            h.push_str(&row_html(&format!("help_{key}"), icon, dedupe_icon(icon, &label)));
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
    // A4: every Play row routes. It has to now — Phase C retires the gamepad
    // sheet, so for most of these modes this row is the only door left.
    for sct in &play {
        for r in &sct.rows {
            let Some(m) = shown.iter().find(|m| m.id == r.id) else { continue };
            let route = route_for(m);
            dom::on_click(&format!("navRow_{}", r.id), move || {
                close();
                follow(&route);
            });
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
    for (key, _, element) in help {
        let el = element.to_string();
        dom::on_click(&format!("navRow_help_{key}"), move || {
            close();
            dom::click(&el);
        });
    }
}

/// Build the drawer and its burger once at startup.
///
/// Ungated as of Phase C: the three round icons and the utility row are
/// retired, so this IS the navigation. It stayed behind `dev_preview` for all
/// of Phase B precisely so the replacement could be built and proven before
/// anything was taken away.
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
                // v1.3.1 F4. It asked for `icon-btn` for its whole life and
                // no such rule exists in the stylesheet, so it rendered as a
                // bare 16pt button with a sub-minimum tap target.
                let _ = b.set_attribute("class", "burger");
                let _ = b.set_attribute("aria-label", &t("nav.menu"));
                b.set_text_content(Some("\u{2630}"));
                let _ = corner.append_child(&b);
            }
        }
    }
    let a = app.clone();
    dom::on_click(BURGER, move || {
        // D-N7 fallback: never open over a running clock. The pause hook is
        // Phase D and its decision is still open, so rather than ship a drawer
        // that can silently burn a timed round, the door is simply shut while
        // one is live. Replacing this with the real pause is a strictly
        // smaller change than undoing a lost round.
        // Belt as well as braces: the burger is disabled while a timer runs
        // (game::reflect_nav_burger), and a disabled button fires no click —
        // but a synthesised click would, so the guard is also checked here.
        if crate::game::timer_is_live() {
            return;
        }
        open(&a)
    });

    // F6: the burger is created here, and `refresh_mode_buttons` has already
    // run by now (lib.rs boots the hub before it wires the drawer), so its
    // reflect_nudge call found no burger to paint. Replay it with the total it
    // recorded, or a cold launch with new misses waiting would show no dot
    // until the next miss -- which is precisely the launch the dot is for.
    if let Some(total) = LAST_TOTAL.with(|c| c.get()) {
        reflect_nudge(total, app.borrow().kid);
    }

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
        let sections = play_sections(&shown, "en");

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
        for s in play_sections(&shown, "en") {
            for r in s.rows {
                let m = shown.iter().find(|m| m.id == r.id).unwrap();
                assert!(matches!(m.group, Group::SpellIt | Group::WordPuzzles | Group::Meaning),
                        "{} is {:?} and must not be in the Play group", m.id, m.group);
            }
        }
    }

    /// The second half of the same bug. ctx resolves __mine to a real
    /// language for the MODE gate; the availability filter has to read that
    /// resolved language too, or a player with their own word list open keeps
    /// Account and My Words but loses Practice and Definition Match -- every
    /// mode whose availability is per-language.
    #[test]
    fn a_word_source_never_reaches_the_availability_filter() {
        let all = modes::all();
        let shown = modes::catalog(&all, &ctx());
        let real = play_sections(&shown, "en");
        let source = play_sections(&shown, crate::consts::MINE);
        let names = |v: &Vec<Section>| -> Vec<String> {
            v.iter().flat_map(|s| s.rows.iter().map(|r| r.id.clone())).collect()
        };
        assert!(!names(&real).is_empty(), "the en menu must not be empty, or this proves nothing");
        assert_ne!(names(&real), names(&source),
            "play_sections is language-sensitive, so passing a pseudo-code straight in\n               must look different from a real language -- render therefore has to pass\n               ctx.lang, which is what the source-scan below checks");
        let src = include_str!("drawer.rs");
        let render = &src[src.find("fn render(app: &App)").unwrap()..];
        assert!(render.contains("play_sections(&shown, &ctx.lang)"),
                "render must hand play_sections the RESOLVED language from ctx");
        assert!(!render[..render.find("fn wire").unwrap_or(render.len())]
                    .contains("app.borrow().lang"),
                "render must not read the raw study language for gating");
    }

    /// A6's arithmetic, on the pure half.
    #[test]
    fn the_badge_caps_and_disappears() {
        assert_eq!(badge_text(0), None, "a badge reading 0 is noise");
        assert_eq!(badge_text(1).as_deref(), Some("1"));
        assert_eq!(badge_text(181).as_deref(), Some("181"));
        assert_eq!(badge_text(999).as_deref(), Some("999"));
        assert_eq!(badge_text(1000).as_deref(), Some("999+"));
        assert_eq!(badge_text(1203).as_deref(), Some("999+"));
    }

    /// F3: Misses leads its group whatever order the registry file is in.
    #[test]
    fn misses_is_the_first_your_words_row() {
        let all = modes::all();
        let shown = modes::catalog(&all, &ctx());
        let rows = your_words_rows(&shown);
        assert_eq!(rows.first().map(|r| r.id.as_str()), Some("misses"),
                   "got {:?}", rows.iter().map(|r| &r.id).collect::<Vec<_>>());
    }

    /// I-N3. The three surfaces and the four quick-play modes all have to
    /// resolve to a row, because v1.3.1 F1 and F2 take away every other door.
    /// Two of these had no row at all until the census went looking.
    #[test]
    fn every_door_the_hub_gave_up_exists_in_the_drawer() {
        let all = modes::all();
        // The app context: iOS, every flag on, full entitlement, photo premium.
        let c = HubCtx { native: true, ..ctx() };
        let shown = modes::catalog(&all, &c);
        let play: Vec<String> = play_sections(&shown, "en")
            .iter().flat_map(|s| s.rows.iter().map(|r| r.id.clone())).collect();
        let yours: Vec<String> = your_words_rows(&shown).iter().map(|r| r.id.clone()).collect();
        for id in ["versus", "daily", "word_picture", "say_it"] {
            assert!(play.contains(&id.to_string()),
                    "{id} lost its hub tile and has no Play row: {play:?}");
        }
        for id in ["misses", "my_words", "photo_list"] {
            assert!(yours.contains(&id.to_string()),
                    "{id} has no Your Words row: {yours:?}");
        }
        // ...and each one resolves to an element to proxy to, or the row is a
        // button that does nothing.
        for id in ["versus", "daily", "word_picture", "say_it", "misses", "my_words", "photo_list"] {
            let m = shown.iter().find(|m| m.id == id).unwrap();
            assert!(matches!(route_for(m), Route::Press(_) | Route::Level(_)),
                    "{id} routes nowhere");
        }
    }

    /// I-N7, over every output the Jr resolver can produce. A child is never
    /// nudged, and neither is a player whose age we have not established.
    #[test]
    fn a_child_is_never_nudged() {
        use crate::experience::{resolve, Experience};
        for kid in [false, true] {
            for locked in [false, true] {
                for age_known in [false, true] {
                    let exp = resolve(kid, locked).experience;
                    let allowed = nudge_allowed(exp, age_known);
                    if exp == Experience::Junior {
                        assert!(!allowed, "Jr was nudged (kid={kid} locked={locked})");
                    }
                    if !age_known {
                        assert!(!allowed, "an unknown age must count as Jr (kid={kid} locked={locked})");
                    }
                }
            }
        }
        // ...and the one case that IS allowed, so this cannot pass by refusing
        // everything.
        assert!(nudge_allowed(Experience::Standard, true));
    }

    /// D-N17: the dot means NEW misses, not any misses.
    #[test]
    fn the_dot_is_about_new_misses_only() {
        assert!(!nudge_due(181, Some(181)), "a backlog is not news");
        assert!(nudge_due(182, Some(181)), "one new miss is");
        assert!(!nudge_due(182, Some(182)), "and opening the drawer settles it");
        // A profile this feature has never seen starts level, never at zero:
        // otherwise everyone with a backlog gets one pointless dot on upgrade.
        assert!(!nudge_due(181, None));
        assert!(!nudge_due(0, None));
    }

    /// A4, on the pure half. Phase C retires the gamepad sheet, so a Play row
    /// with nowhere to go is a mode that has become unreachable.
    #[test]
    fn every_play_row_routes_somewhere_real() {
        let all = modes::all();
        let shown = modes::catalog(&all, &ctx());
        for s in play_sections(&shown, "en") {
            for r in &s.rows {
                let m = shown.iter().find(|m| m.id == r.id).unwrap();
                match route_for(m) {
                    Route::Press(el) => assert!(!el.is_empty(), "{} routes to an empty element", m.id),
                    // Only the base game is allowed to route nowhere, because
                    // "nowhere" is the screen the drawer is already over.
                    Route::Home => assert_eq!(m.id, "standard",
                        "{} has no route; retiring the sheet would strand it", m.id),
                    Route::Level(v) => assert_eq!(v, "climb"),
                }
            }
        }
    }

    /// The bug this route exists to avoid. `climbBtn` is the registry's hub
    /// TILE element and it opens the leaderboard, so resolving the Climb row
    /// the ordinary way sends the player to a scoreboard instead of a run.
    #[test]
    fn the_climb_row_does_not_press_the_leaderboard() {
        let all = modes::all();
        let climb = all.iter().find(|m| m.id == "climb").expect("climb is in the registry");
        assert_eq!(target_for(climb).as_deref(), Some("climbBtn"),
                   "if the registry stops naming climbBtn, re-check what this test is guarding");
        assert_eq!(route_for(climb), Route::Level("climb"));
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

    /// "Kid Mode: no accounts at all" — the rule climb::reflect_auth enforces
    /// by hiding the icon. A Sign in ROW would have walked around that hiding
    /// and put an account door in front of a child (CC-ONBOARD-JR I1/I2), and
    /// the e2e that guarded it was about to start passing for the wrong
    /// reason: accountBtn is display:none for EVERYONE now.
    #[test]
    fn a_signed_out_child_is_offered_no_account_door() {
        let jr = account(false, true, None);
        assert!(jr.action.is_none(), "a child must not be offered Sign in");
        assert!(jr.identity.is_none());
        // and the grown-up in the same state still is
        assert!(account(false, false, None).action.is_some());
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

    /// D-N7: the burger must be shut while a clock runs, and reflected at BOTH
    /// ends — disabling on start without re-enabling on stop would lock a
    /// player out of the only navigation they have after the cut-over.
    #[test]
    fn the_burger_is_reflected_when_a_timer_starts_and_stops() {
        let game = include_str!("game.rs");
        let starts = &game[game.find("pub fn start_timer").unwrap()..];
        let starts = &starts[..starts.find("\npub fn").unwrap_or(starts.len())];
        assert!(starts.contains("reflect_nav_burger()"), "start_timer must shut the burger");

        let stops = &game[game.find("pub fn stop_timer").unwrap()..];
        let stops = &stops[..stops.find("\n// ").unwrap_or(stops.len())];
        assert!(stops.contains("reflect_nav_burger()"), "stop_timer must reopen it");

        // And the click path checks too, because a synthesised click ignores
        // the disabled attribute.
        assert!(include_str!("drawer.rs").contains("crate::game::timer_is_live()"));
    }

    /// D-N6's order, which is the order a player reads.
    #[test]
    fn sections_are_in_the_signed_order() {
        let all = modes::all();
        let shown = modes::catalog(&all, &ctx());
        let keys: Vec<&str> = play_sections(&shown, "en").iter().map(|s| s.header_key).collect();
        assert_eq!(keys, ["nav.spellIt", "nav.wordPuzzles", "nav.meaning"]);
    }

    /// A header with nothing under it is a promise the drawer cannot keep. A
    /// Preview language can filter a whole section away.
    #[test]
    fn an_empty_section_is_dropped_not_rendered() {
        let all = modes::all();
        let none = HubCtx { enabled: vec![], ..ctx() };
        assert!(play_sections(&modes::catalog(&all, &none), "en").is_empty());
    }

    /// D-N3: emoji + name, and nothing else. The tagline moved to the mode's
    /// start screen, so a row that grew a description would be a silent
    /// reversal of a signed decision.
    #[test]
    fn a_row_carries_no_secondary_text() {
        let all = modes::all();
        let shown = modes::catalog(&all, &ctx());
        let html = body(&play_sections(&shown, "en"));
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
