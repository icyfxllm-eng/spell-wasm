//! CC-DEF-MATCH P3 — the frontend loop: lanes, catch, reveal beat, timeout.
//!
//! The screen consumes fully-resolved [`crate::defmatch::Round`]s — card
//! strings, correct index, owning words, lanes, spawn order — and NEVER selects
//! content (Placement). Pools arrive from `/api/defpool` (built by
//! scripts/build-def-pools.py) and are fed to the core engine verbatim.
//!
//! Timing model: cards are CSS-animated (translateY bottom→top). Spawn stagger
//! and dwell are tier-scaled per Feature 2; the ONLY time pressure is float
//! speed (D2 — no visible countdown). All feedback is class toggles, so
//! tap→input-ready stays far under the 200ms budget (Invariant 5); the reveal
//! beat (Feature 4) is the sanctioned exception and is tap-skippable (D7).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::defmatch::{self, DefRow, ExclusionSets, Round};
use crate::{api, dom, haptics, i18n, misses, App};

/// Tier-scaled float timing (ms): (spawn stagger, dwell spawn→top-exit).
/// Feature 2: leisurely on easy, genuinely tight on expert. Kid Mode slows the
/// clamped tier a touch further (calm, not sedated).
fn timing(tier: &str, kid: bool) -> (u32, u32) {
    let (stagger, dwell) = match tier {
        "easy" => (900, 9500),
        "medium" => (800, 7800),
        "hard" => (650, 6200),
        _ => (520, 4600),
    };
    if kid {
        (stagger + 200, dwell + 1500)
    } else {
        (stagger, dwell)
    }
}

thread_local! {
    /// (lang, tier) -> pool rows + exclusions, fetched once per session.
    static POOLS: RefCell<HashMap<(String, String), (Vec<DefRow>, ExclusionSets)>> =
        RefCell::new(HashMap::new());
    static ROUND: RefCell<Option<Round>> = const { RefCell::new(None) };
    static TARGET: RefCell<String> = const { RefCell::new(String::new()) };
    static TIER: RefCell<String> = const { RefCell::new(String::new()) };
    static USED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static STREAK: Cell<u32> = const { Cell::new(0) };
    static ROUND_NO: Cell<u64> = const { Cell::new(0) };
    static SESSION_SEED: Cell<u64> = const { Cell::new(0) };
    /// Round phase: 0 = floating/awaiting tap, 1 = reveal beat (skippable),
    /// 2 = resolved/advancing. Timeout + tap paths both respect it.
    static PHASE: Cell<u8> = const { Cell::new(2) };
    static ESCAPED: Cell<u32> = const { Cell::new(0) };
    static OPEN: Cell<bool> = const { Cell::new(false) };
    /// P4 (D9): entered from a Climb level selection — catches forge shields
    /// through the shared adapter; casual sessions never touch shield state.
    static CLIMB: Cell<bool> = const { Cell::new(false) };
}

pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("defMatchOpen", move || open(&a));
    let a = app.clone();
    dom::on_click("dmExit", move || close(&a));
    // Orb replay (Feature 1): tap anytime to hear the word again.
    let a = app.clone();
    dom::on_click("dmOrb", move || speak_target(&a));
    // Reveal-beat skip (D7) — tapping anywhere on the field during the beat.
    // Guarded by the reveal's start time: the SAME tap that caused the reveal
    // bubbles here too, and must not skip what it just started.
    let a = app.clone();
    dom::on::<web_sys::Event, _>("dmField", "click", move |_| {
        let old_enough = js_sys::Date::now() - REVEAL_T0.with(Cell::get) > 250.0;
        if PHASE.with(Cell::get) == 1 && old_enough {
            advance(&a);
        }
    });
}

fn close(app: &App) {
    OPEN.with(|c| c.set(false));
    PHASE.with(|c| c.set(2));
    dom::remove_class("defMatch", "show");
    api::stop();
    let _ = app; // symmetry with open(); nothing app-side to restore yet (P4 recap).
}

/// Enter the mode: resolve tier (entitlement depth + Kid clamp), fetch the
/// pool if needed, then start the first round.
pub fn open(app: &App) {
    let (lang, kid, mut tier) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid, s.level.clone())
    };
    if !crate::consts::def_match(&lang) {
        return; // tile renders unavailable; belt-and-suspenders.
    }
    // Climb variant (P4/D9): the level picker's "climb" plays medium-tier
    // rounds that FORGE SHIELDS via the shared adapter — same earn streak,
    // same cap, same reset rules as core spelling. Unranked (D4): no
    // leaderboard posting, no climb-band movement.
    let climb = tier == "climb";
    CLIMB.with(|c| c.set(climb));
    if climb {
        tier = "medium".to_string();
    }
    // Entitlement depth (acceptance #9): clamp to the deepest allowed tier.
    let level = crate::play_hub::live_entitlements().lang_level(&lang);
    let allowed = defmatch::allowed_tiers(level);
    if allowed.is_empty() {
        return;
    }
    if !allowed.contains(&tier.as_str()) {
        tier = allowed.last().unwrap().to_string();
    }
    let tier = defmatch::effective_tier(&tier, kid).to_string();

    TIER.with(|t| *t.borrow_mut() = tier.clone());
    USED.with(|u| u.borrow_mut().clear());
    STREAK.with(|c| c.set(0));
    ROUND_NO.with(|c| c.set(0));
    SESSION_SEED.with(|c| c.set(js_sys::Date::now() as u64));
    OPEN.with(|c| c.set(true));
    render_streak();
    dom::add_class("defMatch", "show");
    dom::set_html("dmField", "");
    dom::set_text("dmStatus", &i18n::t("climb.loading"));

    let app = app.clone();
    spawn_local(async move {
        if ensure_pool(&lang, &tier).await {
            dom::set_text("dmStatus", "");
            next_round(&app);
        } else {
            dom::set_text("dmStatus", &i18n::t("so.errNetwork"));
        }
    });
}

/// Fetch + cache the (lang, tier) pool from /api/defpool. True when usable.
async fn ensure_pool(lang: &str, tier: &str) -> bool {
    let key = (lang.to_string(), tier.to_string());
    if POOLS.with(|p| p.borrow().contains_key(&key)) {
        return true;
    }
    let url = format!("{}/api/defpool?lang={}&tier={}", api::api_base(), lang, tier);
    let Ok(text) = crate::storage::fetch_text(&url).await else {
        return false;
    };
    let Ok(v) = js_sys::JSON::parse(&text) else { return false };
    let get = |o: &wasm_bindgen::JsValue, k: &str| js_sys::Reflect::get(o, &k.into()).ok();
    let mut rows = Vec::new();
    if let Some(arr) = get(&v, "rows").and_then(|r| r.dyn_into::<js_sys::Array>().ok()) {
        for item in arr.iter() {
            let s = |k: &str| get(&item, k).and_then(|x| x.as_string()).unwrap_or_default();
            let b = |k: &str| get(&item, k).and_then(|x| x.as_bool()).unwrap_or(false);
            rows.push(DefRow {
                word: s("word"),
                definition: s("definition"),
                prompt_grade: b("prompt_grade"),
                audit_pass: true, // rows in the artifact passed the interim prescreen
                kid_register: b("kid_register"),
                tier: tier.to_string(),
                topic: String::new(), // craft metadata arrives with richer artifacts
                root: String::new(),
            });
        }
    }
    let mut excl = ExclusionSets::default();
    if let Some(obj) = get(&v, "exclusions") {
        if let Ok(keys) = js_sys::Reflect::own_keys(&obj) {
            for k in keys.iter() {
                let Some(word) = k.as_string() else { continue };
                if let Some(arr) = js_sys::Reflect::get(&obj, &k).ok().and_then(|a| a.dyn_into::<js_sys::Array>().ok()) {
                    let set: HashSet<String> = arr.iter().filter_map(|x| x.as_string()).collect();
                    excl.0.insert(word, set);
                }
            }
        }
    }
    if rows.iter().filter(|r| r.prompt_grade).count() < defmatch::POOL_FLOOR {
        return false;
    }
    POOLS.with(|p| p.borrow_mut().insert(key, (rows, excl)));
    true
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn next_round(app: &App) {
    if !OPEN.with(Cell::get) {
        return;
    }
    let (lang, kid) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid)
    };
    let tier = TIER.with(|t| t.borrow().clone());
    let key = (lang.clone(), tier.clone());
    let round = POOLS.with(|p| {
        let pools = p.borrow();
        let (rows, excl) = pools.get(&key)?;
        // Session no-repeat word pick, seeded per round (deterministic within
        // the session; the SESSION seed varies session to session).
        let eligible = defmatch::eligible(rows, &tier, kid);
        let fresh: Vec<&&DefRow> = eligible
            .iter()
            .filter(|r| USED.with(|u| !u.borrow().contains(&r.word)))
            .collect();
        if fresh.is_empty() {
            USED.with(|u| u.borrow_mut().clear());
        }
        let pick_from = if fresh.is_empty() { eligible.iter().collect::<Vec<_>>() } else { fresh };
        let mut st = SESSION_SEED.with(Cell::get) ^ ROUND_NO.with(Cell::get).wrapping_mul(0x9E37);
        let target = pick_from[(splitmix(&mut st) % pick_from.len() as u64) as usize];
        let seed = splitmix(&mut st);
        defmatch::generate(rows, excl, &tier, &target.word, seed, kid)
    });
    let Some(round) = round else {
        dom::set_text("dmStatus", &i18n::t("so.errGeneric"));
        return;
    };
    ROUND_NO.with(|c| c.set(c.get() + 1));
    let word = round.words[round.correct].clone();
    USED.with(|u| {
        u.borrow_mut().insert(word.clone());
    });
    TARGET.with(|t| *t.borrow_mut() = word);
    ROUND.with(|r| *r.borrow_mut() = Some(round.clone()));
    PHASE.with(|c| c.set(0));
    ESCAPED.with(|c| c.set(0));
    ROUND_T0.with(|c| c.set(js_sys::Date::now()));
    render_round(app, &round, &tier, kid);
    speak_target(app);
}

/// Feature 1: the orb speaks the word (cached server clip, same path as core
/// play; native/browser fallbacks included).
fn speak_target(app: &App) {
    let lang = app.borrow().lang.clone();
    let word = TARGET.with(|t| t.borrow().clone());
    if word.is_empty() {
        return;
    }
    let rate = app.borrow().rate;
    dom::add_class("dmOrb", "pulse");
    let w = word.clone();
    let code = format!("{}-{}", lang, lang.to_uppercase());
    api::play_word(&word, "normal", rate as f64, &lang, move || {
        crate::speech_out::speak(&w, 0.9, &code)
    });
}

fn render_round(app: &App, round: &Round, tier: &str, kid: bool) {
    let (stagger, dwell) = timing(tier, kid);
    let n = round.cards.len();
    let sway = matches!(tier, "easy" | "medium");
    let mut html = String::new();
    for i in 0..n {
        let lane = round.lanes[i] as usize;
        let spawn_pos = round.spawn_order.iter().position(|&c| c as usize == i).unwrap_or(0);
        let delay = spawn_pos as u32 * stagger;
        let left = (lane as f32) * (100.0 / n as f32);
        let width = 100.0 / n as f32;
        html.push_str(&format!(
            "<button type=\"button\" class=\"dm-card{sway}\" id=\"dmCard{i}\" data-card=\"{i}\" \
               style=\"left:{left:.2}%;width:{width:.2}%;animation-duration:{dwell}ms;animation-delay:{delay}ms\">\
               <span class=\"dm-def\">{def}</span><span class=\"dm-owner\" id=\"dmOwner{i}\"></span>\
             </button>",
            sway = if sway { " sway" } else { "" },
            def = dom::escape_html(&round.cards[i]),
        ));
    }
    dom::set_html("dmField", &html);
    for i in 0..n {
        let a = app.clone();
        let id = format!("dmCard{i}");
        dom::on_click(&id, move || tap(&a, i));
        // Timeout accounting: a card that finishes its float has escaped.
        let a2 = app.clone();
        dom::on::<web_sys::Event, _>(&id, "animationend", move |e| {
            // Sway also emits animationend; only the float (translate) counts.
            if let Some(t) = e.dyn_ref::<web_sys::AnimationEvent>() {
                if t.animation_name() != "dmFloat" {
                    return;
                }
            }
            on_escape(&a2);
        });
    }
}

fn tap(app: &App, idx: usize) {
    match PHASE.with(Cell::get) {
        1 => {
            advance(app); // D7: any tap skips the reveal beat
            return;
        }
        0 => {}
        _ => return,
    }
    let Some(round) = ROUND.with(|r| r.borrow().clone()) else { return };
    let kid = app.borrow().kid;
    if idx == round.correct {
        PHASE.with(|c| c.set(2));
        // Feature 3: the catch. Last-15%-of-travel taps get the style flash.
        let late = card_progress(idx) > 0.85;
        dom::add_class(&format!("dmCard{idx}"), "caught");
        for i in 0..round.cards.len() {
            if i != idx {
                dom::add_class(&format!("dmCard{i}"), "dissolve");
            }
        }
        dom::add_class("dmOrb", "good");
        haptics::key_tap();
        if late {
            dom::set_text("dmStatus", &i18n::t("defmatch.caught"));
        }
        STREAK.with(|c| c.set(c.get() + 1));
        render_streak();
        // P4: forging — a first-tap catch is a validated correct.
        if CLIMB.with(Cell::get) {
            let earned = defmatch::climb_outcome(&mut app.borrow_mut(), true);
            crate::game::update_shield_hud(app);
            if earned {
                dom::set_text("dmStatus", &i18n::t("shield.earned"));
            }
        }
        after(650, {
            let app = app.clone();
            move || {
                dom::remove_class("dmOrb", "good");
                dom::set_text("dmStatus", "");
                next_round(&app);
            }
        });
    } else {
        // Feature 4: the miss that teaches. Wrong tap → reveal beat.
        PHASE.with(|c| c.set(1));
        REVEAL_T0.with(|c| c.set(js_sys::Date::now()));
        STREAK.with(|c| c.set(0));
        render_streak();
        record_miss(app);
        if !kid {
            dom::add_class(&format!("dmCard{idx}"), "wrong");
        }
        // Show which word the tapped distractor actually defines (both strings
        // audited-pool rows — Invariant 1), and glow the correct card gold
        // (kid: the same calm highlight, no red anywhere).
        dom::set_text(&format!("dmOwner{idx}"), &round.words[idx]);
        dom::add_class(&format!("dmCard{}", round.correct), if kid { "reveal-calm" } else { "reveal-gold" });
        dom::add_class(&format!("dmCard{}", round.correct), "paused");
        dom::add_class(&format!("dmCard{idx}"), "paused");
        after(if kid { 2000 } else { 1500 }, {
            let app = app.clone();
            move || {
                if PHASE.with(Cell::get) == 1 {
                    advance(&app);
                }
            }
        });
    }
}

/// How far along its float a card is (0.0 spawn → 1.0 top-exit), from the CSS
/// animation clock — cheap, no layout read.
fn card_progress(idx: usize) -> f64 {
    let doc = dom::doc();
    let Some(el) = doc.get_element_by_id(&format!("dmCard{idx}")) else { return 0.0 };
    let Some(win) = web_sys::window() else { return 0.0 };
    let Ok(style) = win.get_computed_style(&el) else { return 0.0 };
    let Some(style) = style else { return 0.0 };
    let dur: f64 = style
        .get_property_value("animation-duration")
        .ok()
        .and_then(|v| v.trim_end_matches('s').parse().ok())
        .unwrap_or(0.0);
    let delay: f64 = style
        .get_property_value("animation-delay")
        .ok()
        .and_then(|v| v.trim_end_matches('s').parse().ok())
        .unwrap_or(0.0);
    if dur <= 0.0 {
        return 0.0;
    }
    // Elapsed since round render ≈ animation clock; the round timestamp rides
    // on the element as data-t0 set at render... simpler: use the animation's
    // own currentTime via getAnimations is unstable — approximate with round
    // start time.
    let t0 = ROUND_T0.with(Cell::get);
    let elapsed = js_sys::Date::now() - t0 - delay * 1000.0;
    (elapsed / (dur * 1000.0)).clamp(0.0, 1.0)
}

thread_local! {
    static ROUND_T0: Cell<f64> = const { Cell::new(0.0) };
    /// When the current reveal beat began — the D7 skip ignores taps younger
    /// than 250ms so the triggering tap can't skip its own reveal.
    static REVEAL_T0: Cell<f64> = const { Cell::new(0.0) };
}

/// A card floated off the top. When ALL cards escape un-tapped, that's the
/// timeout path: record the miss, slide the correct card back for a brief
/// reveal, then advance (Feature 4).
fn on_escape(app: &App) {
    if PHASE.with(Cell::get) != 0 {
        return;
    }
    let Some(round) = ROUND.with(|r| r.borrow().clone()) else { return };
    let n = round.cards.len() as u32;
    let escaped = ESCAPED.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    if escaped < n {
        return;
    }
    PHASE.with(|c| c.set(1));
    REVEAL_T0.with(|c| c.set(js_sys::Date::now()));
    STREAK.with(|c| c.set(0));
    render_streak();
    record_miss(app);
    let cid = format!("dmCard{}", round.correct);
    dom::remove_class(&cid, "sway");
    dom::add_class(&cid, "returned");
    dom::add_class(&cid, if app.borrow().kid { "reveal-calm" } else { "reveal-gold" });
    after(1600, {
        let app = app.clone();
        move || {
            if PHASE.with(Cell::get) == 1 {
                advance(&app);
            }
        }
    });
}

/// Every miss (wrong tap or timeout) enters spaced repetition, as v1 — and in
/// the Climb variant it resets the shield earn streak (PD2, shared adapter).
fn record_miss(app: &App) {
    let word = TARGET.with(|t| t.borrow().clone());
    let (lang, tier) = (app.borrow().lang.clone(), TIER.with(|t| t.borrow().clone()));
    misses::add_miss(&mut app.borrow_mut(), &word, &lang, &tier);
    if CLIMB.with(Cell::get) {
        let _ = defmatch::climb_outcome(&mut app.borrow_mut(), false);
        crate::game::update_shield_hud(app);
    }
}

fn advance(app: &App) {
    PHASE.with(|c| c.set(2));
    next_round(app);
}

fn render_streak() {
    let n = STREAK.with(Cell::get);
    dom::set_text("dmStreak", &if n > 0 { format!("🔥 {n}") } else { String::new() });
    // Feature 5: escalating orb glow at 3 / 5 / 10.
    for (t, cls) in [(3, "heat3"), (5, "heat5"), (10, "heat10")] {
        dom::toggle_class("dmOrb", cls, n >= t);
    }
}

fn after(ms: i32, f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    if let Some(win) = web_sys::window() {
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms);
    }
}
