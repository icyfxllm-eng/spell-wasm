//! CC-VISUAL-ECLIPSE F9 -- earnable themes, behind `visual.themes`.
//!
//! A theme is a token set and nothing else (F9.1): `:root[data-theme=<id>]`
//! overrides in design/tokens.json, plus a Jr variant under `body.kid`. No
//! layout or logic changes; Randospell's table and Board Game's ring keep their
//! own art.
//!
//! Themes are earned through play and never sold (V-D5): a locked theme shows
//! its condition, never a price, never a store link. Unlocks (V-D19, E-Q8):
//! - Chalkboard: a 7-day Daily streak (from the Daily record's best streak);
//! - Typewriter: 500 lifetime correct words;
//! - Spelling Bee Stage: 100 correct words at Hard or above.
//! They are retroactive, stored once earned and never revoked. The counts are a
//! baseline frozen from the existing base-game stats the first time this runs,
//! plus a counter of every correct answer from then on in any mode (and, for
//! Hard+, in the base game, the only place a tier is known). The per-language
//! reset button wipes the stats, never this store.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Eclipse,
    Daybreak,
    Chalkboard,
    Typewriter,
    BeeStage,
}

pub const ALL: [Theme; 5] = [Theme::Eclipse, Theme::Daybreak, Theme::Chalkboard, Theme::Typewriter, Theme::BeeStage];

impl Theme {
    pub fn id(self) -> &'static str {
        match self {
            Theme::Eclipse => "eclipse",
            Theme::Daybreak => "daybreak",
            Theme::Chalkboard => "chalkboard",
            Theme::Typewriter => "typewriter",
            Theme::BeeStage => "beestage",
        }
    }
    pub fn from_id(s: &str) -> Option<Theme> {
        ALL.into_iter().find(|t| t.id() == s)
    }
    /// The i18n key of its name.
    pub fn name_key(self) -> String {
        format!("theme.{}", self.id())
    }
    /// Free themes need no unlock.
    pub fn free(self) -> bool {
        matches!(self, Theme::Eclipse | Theme::Daybreak)
    }
    /// The i18n key of how it is earned, for a locked row.
    pub fn condition_key(self) -> Option<&'static str> {
        match self {
            Theme::Chalkboard => Some("theme.unlock.chalkboard"),
            Theme::Typewriter => Some("theme.unlock.typewriter"),
            Theme::BeeStage => Some("theme.unlock.beestage"),
            _ => None,
        }
    }
}

pub const STREAK_FOR_CHALKBOARD: u32 = 7;
pub const CORRECT_FOR_TYPEWRITER: u32 = 500;
pub const HARD_FOR_BEESTAGE: u32 = 100;

/// What progress a player has, for the unlock rules.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Progress {
    pub best_daily_streak: u32,
    pub lifetime_correct: u32,
    pub hard_correct: u32,
}

/// The rule (V-D19).
pub fn earned(t: Theme, p: Progress) -> bool {
    match t {
        Theme::Eclipse | Theme::Daybreak => true,
        Theme::Chalkboard => p.best_daily_streak >= STREAK_FOR_CHALKBOARD,
        Theme::Typewriter => p.lifetime_correct >= CORRECT_FOR_TYPEWRITER,
        Theme::BeeStage => p.hard_correct >= HARD_FOR_BEESTAGE,
    }
}

/// The player's choice: "system" follows the OS appearance (F9.3, the default).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    System,
    Fixed(Theme),
}

impl Choice {
    pub fn id(self) -> &'static str {
        match self {
            Choice::System => "system",
            Choice::Fixed(t) => t.id(),
        }
    }
    pub fn from_id(s: &str) -> Choice {
        Theme::from_id(s).map(Choice::Fixed).unwrap_or(Choice::System)
    }
}

/// The theme to draw: System follows the OS; a theme no longer unlocked (it
/// never happens, unlocks are never revoked, but stored data can be edited)
/// falls back to System.
pub fn resolve(choice: Choice, os_light: bool, unlocked: &[Theme]) -> Theme {
    match choice {
        Choice::Fixed(t) if t.free() || unlocked.contains(&t) => t,
        _ => {
            if os_light {
                Theme::Daybreak
            } else {
                Theme::Eclipse
            }
        }
    }
}

/// The stored themes record: never wiped by the stats reset.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct Store {
    #[serde(default)]
    pub unlocked: Vec<String>,
    #[serde(default)]
    pub choice: String,
    /// Frozen the first time: what the base-game stats already held.
    #[serde(default)]
    pub baseline: Option<(u32, u32)>,
    /// Every correct answer since, in any mode.
    #[serde(default)]
    pub correct_since: u32,
    /// Base-game correct answers at Hard or above since.
    #[serde(default)]
    pub hard_since: u32,
}

impl Store {
    pub fn progress(&self, best_daily_streak: u32) -> Progress {
        let (base_all, base_hard) = self.baseline.unwrap_or((0, 0));
        Progress {
            best_daily_streak,
            lifetime_correct: base_all + self.correct_since,
            hard_correct: base_hard + self.hard_since,
        }
    }

    /// Grant every theme the progress earns. Returns the newly unlocked ones.
    /// Never removes one.
    pub fn grant(&mut self, p: Progress) -> Vec<Theme> {
        let mut new = Vec::new();
        for t in ALL {
            if !t.free() && earned(t, p) && !self.unlocked.iter().any(|u| u == t.id()) {
                self.unlocked.push(t.id().to_string());
                new.push(t);
            }
        }
        new
    }

    pub fn unlocked_themes(&self) -> Vec<Theme> {
        self.unlocked.iter().filter_map(|s| Theme::from_id(s)).collect()
    }
}

/// The base-game stats as (all correct, Hard+ correct), for the baseline.
pub fn stats_totals(stats: &std::collections::HashMap<String, std::collections::HashMap<String, crate::model::TierStat>>) -> (u32, u32) {
    let mut all = 0;
    let mut hard = 0;
    for tiers in stats.values() {
        for (tier, st) in tiers {
            all += st.correct;
            if tier == "hard" || tier == "expert" {
                hard += st.correct;
            }
        }
    }
    (all, hard)
}

// ---------- the web side ----------

const KEY: &str = "spell_themes_v1";

pub fn load() -> Store {
    crate::storage::get_json(KEY).unwrap_or_default()
}

fn save(s: &Store) {
    crate::storage::set_json(KEY, s);
}

fn on() -> bool {
    crate::flags::visual_themes()
}

fn os_light() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: light)").ok().flatten())
        .map(|m| m.matches())
        .unwrap_or(false)
}

/// At boot: freeze the baseline once, grant what is already earned (the
/// retroactive half of E-Q8), and draw the chosen theme.
pub fn init(state: &crate::model::AppState) {
    let mut s = load();
    if s.baseline.is_none() {
        s.baseline = Some(stats_totals(&state.stats));
    }
    let streak = crate::daily::load().best_streak;
    s.grant(s.progress(streak));
    save(&s);
    apply();
    install_os_listener();
}

/// A correct answer in any mode.
pub fn note_correct() {
    let mut s = load();
    s.correct_since = s.correct_since.saturating_add(1);
    after_progress(s);
}

/// A base-game correct answer at `tier`.
pub fn note_tier_correct(tier: &str) {
    if tier == "hard" || tier == "expert" {
        let mut s = load();
        s.hard_since = s.hard_since.saturating_add(1);
        after_progress(s);
    }
}

/// The Daily streak moved.
pub fn note_daily_streak() {
    after_progress(load());
}

fn after_progress(mut s: Store) {
    let new = s.grant(s.progress(crate::daily::load().best_streak));
    save(&s);
    if on() {
        for t in new {
            // A theme unlock is a big celebration (V-D18).
            crate::celebrate::ask(
                crate::celebrate::Trigger::ThemeUnlock,
                crate::i18n::tp("theme.unlocked", &[("theme", &crate::i18n::t(&t.name_key()))]),
            );
        }
        render_picker();
    }
}

pub fn current() -> Theme {
    let s = load();
    resolve(Choice::from_id(&s.choice), os_light(), &s.unlocked_themes())
}

/// Set `data-theme` on the root (only with the flag on; off, no attribute).
pub fn apply() {
    let Some(root) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.document_element()) else { return };
    if !on() {
        let _ = root.remove_attribute("data-theme");
        return;
    }
    let _ = root.set_attribute("data-theme", current().id());
    // F9.8: a theme face only for scripts it covers. The typewriter face is
    // Latin; Vietnamese stacks marks it does not draw, so it is not "latin" here.
    let latin = matches!(crate::i18n::current().as_str(), "en" | "es" | "fr" | "de" | "pt" | "pl" | "fil" | "sw");
    let _ = root.set_attribute("data-script", if latin { "latin" } else { "other" });
}

/// The player picked a choice in Settings.
pub fn choose(id: &str) {
    let mut s = load();
    let choice = Choice::from_id(id);
    if let Choice::Fixed(t) = choice {
        if !t.free() && !s.unlocked.iter().any(|u| u == t.id()) {
            return; // A locked theme cannot be chosen.
        }
    }
    s.choice = choice.id().to_string();
    save(&s);
    apply();
    render_picker();
}

fn install_os_listener() {
    use wasm_bindgen::{closure::Closure, JsCast};
    let Some(mq) = web_sys::window().and_then(|w| w.match_media("(prefers-color-scheme: light)").ok().flatten()) else { return };
    let cb = Closure::<dyn FnMut()>::new(apply);
    let _ = mq.add_event_listener_with_callback("change", cb.as_ref().unchecked_ref());
    cb.forget();
}

/// The Appearance rows in Settings: System, then each theme. A locked one shows
/// how it is earned -- never a price, never a store link (F9.4).
pub fn render_picker() {
    use crate::dom;
    if !dom::exists("themePick") {
        return;
    }
    let s = load();
    let chosen = Choice::from_id(&s.choice);
    let unlocked = s.unlocked_themes();
    let mut h = String::new();
    let row = |id: &str, name: &str, sub: Option<String>, locked: bool, active: bool| {
        format!(
            "<button type=\"button\" class=\"theme-row{}{}\" data-theme-choice=\"{id}\"{} aria-pressed=\"{}\">\
             <span class=\"theme-chip\" data-chip=\"{id}\" aria-hidden=\"true\"></span>\
             <span class=\"theme-text\"><b>{}</b>{}</span></button>",
            if locked { " locked" } else { "" },
            if active { " active" } else { "" },
            if locked { " aria-disabled=\"true\"" } else { "" },
            active,
            dom::escape_html(name),
            sub.map(|t| format!("<small>{}</small>", dom::escape_html(&t))).unwrap_or_default(),
        )
    };
    h.push_str(&row("system", &crate::i18n::t("theme.system"), Some(crate::i18n::t("theme.systemSmall")), false, chosen == Choice::System));
    for t in ALL {
        let locked = !t.free() && !unlocked.contains(&t);
        let sub = if locked { t.condition_key().map(crate::i18n::t) } else { None };
        h.push_str(&row(t.id(), &crate::i18n::t(&t.name_key()), sub, locked, chosen == Choice::Fixed(t)));
    }
    dom::set_html("themePick", &h);
}

/// One delegated click handler for the rows.
pub fn wire_picker() {
    use wasm_bindgen::{closure::Closure, JsCast};
    let Some(el) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("themePick")) else { return };
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        let row = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|t| t.closest("[data-theme-choice]").ok().flatten());
        if let Some(id) = row.and_then(|r| r.get_attribute("data-theme-choice")) {
            choose(&id);
        }
    });
    let _ = el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
    cb.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(streak: u32, all: u32, hard: u32) -> Progress {
        Progress { best_daily_streak: streak, lifetime_correct: all, hard_correct: hard }
    }

    #[test]
    fn the_unlock_rules_are_v_d19() {
        assert!(earned(Theme::Chalkboard, p(7, 0, 0)) && !earned(Theme::Chalkboard, p(6, 9999, 9999)));
        assert!(earned(Theme::Typewriter, p(0, 500, 0)) && !earned(Theme::Typewriter, p(99, 499, 499)));
        assert!(earned(Theme::BeeStage, p(0, 0, 100)) && !earned(Theme::BeeStage, p(99, 9999, 99)));
        assert!(earned(Theme::Eclipse, p(0, 0, 0)) && earned(Theme::Daybreak, p(0, 0, 0)));
    }

    // AT-23: a player who already qualifies gets the theme the first time.
    #[test]
    fn existing_progress_grants_retroactively() {
        let mut s = Store { baseline: Some((620, 130)), ..Store::default() };
        let new = s.grant(s.progress(9));
        assert_eq!(new, vec![Theme::Chalkboard, Theme::Typewriter, Theme::BeeStage]);
    }

    #[test]
    fn the_baseline_plus_the_counter_reaches_the_bar() {
        let mut s = Store { baseline: Some((490, 95)), correct_since: 10, hard_since: 5, ..Store::default() };
        assert_eq!(s.progress(0), p(0, 500, 100));
        assert_eq!(s.grant(s.progress(0)), vec![Theme::Typewriter, Theme::BeeStage]);
    }

    #[test]
    fn an_unlock_is_never_revoked() {
        let mut s = Store { baseline: Some((600, 0)), ..Store::default() };
        s.grant(s.progress(0));
        // The stats reset wipes the base-game history; the store keeps the theme.
        assert!(s.grant(p(0, 0, 0)).is_empty());
        assert!(s.unlocked_themes().contains(&Theme::Typewriter));
    }

    #[test]
    fn granting_twice_adds_nothing() {
        let mut s = Store::default();
        s.grant(p(7, 0, 0));
        assert!(s.grant(p(7, 0, 0)).is_empty());
        assert_eq!(s.unlocked, vec!["chalkboard"]);
    }

    #[test]
    fn system_follows_the_os_and_a_locked_choice_falls_back() {
        assert_eq!(resolve(Choice::System, false, &[]), Theme::Eclipse);
        assert_eq!(resolve(Choice::System, true, &[]), Theme::Daybreak);
        assert_eq!(resolve(Choice::Fixed(Theme::Daybreak), false, &[]), Theme::Daybreak);
        assert_eq!(resolve(Choice::Fixed(Theme::Typewriter), false, &[]), Theme::Eclipse, "not unlocked");
        assert_eq!(resolve(Choice::Fixed(Theme::Typewriter), false, &[Theme::Typewriter]), Theme::Typewriter);
    }

    #[test]
    fn stats_totals_count_hard_and_expert_as_hard_plus() {
        use std::collections::HashMap;
        let mut stats: HashMap<String, HashMap<String, crate::model::TierStat>> = HashMap::new();
        let mut en = HashMap::new();
        for (tier, c) in [("easy", 40), ("medium", 30), ("hard", 20), ("expert", 10)] {
            en.insert(tier.to_string(), crate::model::TierStat { correct: c, ..Default::default() });
        }
        stats.insert("en".into(), en);
        assert_eq!(stats_totals(&stats), (100, 30));
    }

    // ---- F10 / AT-25: accents never read as a verdict ----

    fn lab(hex: &str) -> (f64, f64, f64) {
        let h = hex.trim_start_matches('#');
        let c = |i: usize| {
            let v = u8::from_str_radix(&h[i..i + 2], 16).unwrap() as f64 / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        let (r, g, b) = (c(0), c(2), c(4));
        let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
        let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
        let f = |t: f64| if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 };
        (116.0 * f(y) - 16.0, 500.0 * (f(x) - f(y)), 200.0 * (f(y) - f(z)))
    }

    /// CIEDE2000 (Sharma, Wu and Dalal's formulation).
    fn de2000(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
        use std::f64::consts::PI;
        let (l1, a1, b1) = a;
        let (l2, a2, b2) = b;
        let c1 = (a1 * a1 + b1 * b1).sqrt();
        let c2 = (a2 * a2 + b2 * b2).sqrt();
        let cm = (c1 + c2) / 2.0;
        let g = 0.5 * (1.0 - (cm.powi(7) / (cm.powi(7) + 25f64.powi(7))).sqrt());
        let (a1p, a2p) = (a1 * (1.0 + g), a2 * (1.0 + g));
        let (c1p, c2p) = ((a1p * a1p + b1 * b1).sqrt(), (a2p * a2p + b2 * b2).sqrt());
        let hp = |b: f64, a: f64| { let h = b.atan2(a).to_degrees(); if h < 0.0 { h + 360.0 } else { h } };
        let (h1p, h2p) = (hp(b1, a1p), hp(b2, a2p));
        let dl = l2 - l1;
        let dc = c2p - c1p;
        let mut dh = h2p - h1p;
        if c1p * c2p != 0.0 {
            if dh > 180.0 { dh -= 360.0 } else if dh < -180.0 { dh += 360.0 }
        } else {
            dh = 0.0;
        }
        let dhh = 2.0 * (c1p * c2p).sqrt() * (dh.to_radians() / 2.0).sin();
        let lm = (l1 + l2) / 2.0;
        let cmp = (c1p + c2p) / 2.0;
        let mut hm = h1p + h2p;
        if c1p * c2p != 0.0 {
            if (h1p - h2p).abs() > 180.0 { hm += if hm < 360.0 { 360.0 } else { -360.0 } }
            hm /= 2.0;
        }
        let t = 1.0 - 0.17 * ((hm - 30.0).to_radians()).cos() + 0.24 * ((2.0 * hm).to_radians()).cos()
            + 0.32 * ((3.0 * hm + 6.0).to_radians()).cos() - 0.20 * ((4.0 * hm - 63.0).to_radians()).cos();
        let dtheta = 30.0 * (-(((hm - 275.0) / 25.0).powi(2))).exp();
        let rc = 2.0 * (cmp.powi(7) / (cmp.powi(7) + 25f64.powi(7))).sqrt();
        let sl = 1.0 + 0.015 * (lm - 50.0).powi(2) / (20.0 + (lm - 50.0).powi(2)).sqrt();
        let sc = 1.0 + 0.045 * cmp;
        let sh = 1.0 + 0.015 * cmp * t;
        let rt = -(2.0 * dtheta * PI / 180.0).sin() * rc;
        ((dl / sl).powi(2) + (dc / sc).powi(2) + (dhh / sh).powi(2) + rt * (dc / sc) * (dhh / sh)).sqrt()
    }

    #[test]
    fn ciede2000_matches_a_published_pair() {
        // Sharma et al. test data, pair 1: ΔE00 = 2.0425.
        let d = de2000((50.0, 2.6772, -79.7751), (50.0, 0.0, -82.7485));
        assert!((d - 2.0425).abs() < 1e-3, "{d}");
    }

    #[test]
    fn every_language_accent_is_far_from_correct_and_wrong() {
        use crate::tokens::*;
        let accents = [
            ("en", LANG_ACCENT_EN), ("es", LANG_ACCENT_ES), ("fr", LANG_ACCENT_FR), ("de", LANG_ACCENT_DE),
            ("pt", LANG_ACCENT_PT), ("pl", LANG_ACCENT_PL), ("vi", LANG_ACCENT_VI), ("ko", LANG_ACCENT_KO),
            ("ja", LANG_ACCENT_JA), ("zh", LANG_ACCENT_ZH), ("ru", LANG_ACCENT_RU), ("ar", LANG_ACCENT_AR),
            ("sw", LANG_ACCENT_SW), ("hi", LANG_ACCENT_HI), ("fil", LANG_ACCENT_FIL),
        ];
        // The verdict colours in every theme: the dark base and the light themes' darker pair.
        let verdicts = [COLOR_SUCCESS.value, COLOR_CAUTION.value, "#13806b", "#b5540a"];
        let mut worst = (f64::MAX, "", "");
        for (lang, t) in accents {
            for v in verdicts {
                let d = de2000(lab(t.value), lab(v));
                if d < worst.0 {
                    worst = (d, lang, v);
                }
                assert!(d >= 20.0, "{lang} accent {} is only ΔE {d:.1} from {v}", t.value);
            }
        }
        assert!(worst.0 >= 20.0);
    }

    #[test]
    fn choice_ids_round_trip() {
        assert_eq!(Choice::from_id("system"), Choice::System);
        assert_eq!(Choice::from_id("typewriter"), Choice::Fixed(Theme::Typewriter));
        assert_eq!(Choice::from_id("nonsense"), Choice::System);
        assert_eq!(Choice::from_id(""), Choice::System);
    }
}
