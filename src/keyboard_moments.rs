//! CC-SPELL-KEYBOARD Phase C — the keyboard's moments.
//!
//! * KF5 letter flight: a ghost of a typed letter rises from its key into the
//!   answer. The real letter is already there (the key's sink ran first); the
//!   ghost is decoration, skipped for fast typing (K-D22) and under Reduce Motion.
//! * KF6 constellation: after a correct answer the keys it used light as stars,
//!   joined in typing order, then fade. A tap skips it.
//! * KF7 reactions: a wave across the keys for a correct answer, a brief
//!   caution-amber dim for a wrong one, streak warmth on the keycaps, and only a
//!   neutral "submitted" pulse where the mode shows no verdict.
//!
//! I7 / K-D23: nothing here may reveal a verdict a mode withholds, so every
//! reaction goes through [`react`], which reads the mode's feedback policy
//! (`modes::feedback_policy`). Each moment has its own flag, default off (K-D28).
//! The timings are motion tokens (`--motion-anim-key-*`, `--motion-anim-constellation`);
//! Reduce Motion is handled in the stylesheet (colour and opacity only).

use std::cell::Cell;

use unicode_normalization::UnicodeNormalization;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::dom;
use crate::modes::Verdict;
use crate::spellkb::{Attr, Key};

/// K-D22: a letter typed sooner than this after the previous one gets no flight.
/// A typing interval, not an animation length.
const FLIGHT_MIN_GAP_MS: f64 = 120.0;
/// How long a ghost element lives before it is removed (its animation is
/// `--motion-anim-key-flight`, 120 ms; this only cleans up the node).
const GHOST_LIFETIME_MS: i32 = 300;
/// How long the constellation overlay lives (its fade is
/// `--motion-anim-constellation`, 1.2 s; this only cleans up the node).
const CONSTELLATION_LIFETIME_MS: i32 = 1300;
/// How long a reaction class stays on the keys (<= 500 ms in all, KF7).
const REACTION_LIFETIME_MS: i32 = 500;
/// The wave's rings out from the centre are capped so the whole wave ends in
/// time: 280 ms per key + 20 ms per ring, rings <= 10.
const MAX_RINGS: f64 = 10.0;

thread_local! {
    static LAST_PRESS: Cell<f64> = const { Cell::new(f64::NEG_INFINITY) };
}

// ---------- KF5: letter flight ----------

/// A key was pressed on a surface whose answer is `field`: send its ghost there.
/// `face` overrides the key's own label (a long-press alternate flies as itself).
pub fn flight(key: &web_sys::Element, field: &str, face: Option<&str>) {
    let now = js_sys::Date::now();
    let gap = now - LAST_PRESS.with(|l| l.replace(now));
    if !crate::flags::keyboard_flight() || field.is_empty() || gap < FLIGHT_MIN_GAP_MS {
        return;
    }
    let doc = dom::doc();
    let Some(target) = doc.get_element_by_id(field) else { return };
    let t = target.get_bounding_client_rect();
    let k = key.get_bounding_client_rect();
    if t.width() <= 0.0 || k.width() <= 0.0 {
        return; // the answer is not on screen (Spell Picture borrows the keyboard)
    }
    let Ok(ghost) = doc.create_element("span") else { return };
    ghost.set_class_name("skb-ghost");
    let _ = ghost.set_attribute("aria-hidden", "true");
    let label = face.map(str::to_string).or_else(|| key.text_content());
    ghost.set_text_content(label.as_deref());
    if let Some(g) = ghost.dyn_ref::<web_sys::HtmlElement>() {
        let st = g.style();
        let _ = st.set_property("left", &format!("{}px", k.left()));
        let _ = st.set_property("top", &format!("{}px", k.top()));
        let _ = st.set_property("width", &format!("{}px", k.width()));
        let _ = st.set_property("height", &format!("{}px", k.height()));
        let dx = (t.left() + t.width() / 2.0) - (k.left() + k.width() / 2.0);
        let dy = (t.top() + t.height() / 2.0) - (k.top() + k.height() / 2.0);
        let _ = st.set_property("--dx", &format!("{dx}px"));
        let _ = st.set_property("--dy", &format!("{dy}px"));
    }
    if let Some(body) = doc.body() {
        let _ = body.append_child(&ghost);
    }
    later(GHOST_LIFETIME_MS, move || ghost.remove());
}

// ---------- KF6 + KF7: the verdict ----------

/// One answer's verdict, as the surface knows it.
pub struct Outcome<'a> {
    /// The keyboard host the answer was typed on.
    pub host: &'static str,
    /// The mode's policy (`modes::feedback_policy(..).verdict`).
    pub verdict: Verdict,
    pub correct: bool,
    /// The answer as typed (the constellation draws its keys).
    pub answer: &'a str,
    /// Consecutive correct answers, for streak warmth; 0 where a mode keeps none.
    pub streak: u32,
    /// Spelluzzle: the verdict exists only in Duel (D23).
    pub in_duel: bool,
}

/// What the keyboard may show for this verdict (I7 / K-D23).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// The mode shows the verdict: wave or dim, warmth, constellation.
    Verdict,
    /// The mode withholds it: a neutral pulse, identical for right and wrong.
    Neutral,
}

/// I7 in one place: is this verdict the player's to see?
pub fn shown(verdict: Verdict, in_duel: bool) -> Shown {
    match verdict {
        Verdict::PerWord => Shown::Verdict,
        Verdict::DuelOnly if in_duel => Shown::Verdict,
        Verdict::DuelOnly | Verdict::None => Shown::Neutral,
    }
}

/// The keyboard reacts to a submitted answer. Nothing happens on a keyboard
/// that is not on screen (several modes hide their keys the moment a word is
/// submitted); a hidden key measures as nothing and would put stars in a corner.
pub fn react(o: Outcome) {
    if !on_screen(o.host) {
        return;
    }
    let react = crate::flags::keyboard_react();
    match shown(o.verdict, o.in_duel) {
        Shown::Neutral => {
            if react {
                flash(o.host, "skb-pulse");
            }
        }
        Shown::Verdict if o.correct => {
            if react {
                wave(o.host);
                warm(o.host, o.streak);
            }
            if crate::flags::keyboard_constellation() {
                constellation(o.host, o.answer);
            }
        }
        Shown::Verdict => {
            if react {
                flash(o.host, "skb-dim");
                warm(o.host, 0);
            }
        }
    }
}

/// [`react`] for a surface that knows its mode id: the policy is looked up here.
pub fn react_for(mode: &str, host: &'static str, correct: bool, answer: &str, streak: u32) {
    let verdict = crate::modes::feedback_policy(mode, None).verdict;
    react(Outcome { host, verdict, correct, answer, streak, in_duel: false });
}

/// Only the neutral "submitted" pulse, for a surface that holds part of its
/// verdict back (SpellDoku keeps a logic error for Check Board on Hard and
/// Expert, so even a "correct" wave on a well-spelled answer would leak).
pub fn submitted(host: &'static str) {
    if crate::flags::keyboard_react() && on_screen(host) {
        flash(host, "skb-pulse");
    }
}

fn on_screen(host: &str) -> bool {
    host_el(host).is_some_and(|e| {
        let r = e.get_bounding_client_rect();
        r.width() > 0.0 && r.height() > 0.0
    })
}

fn host_el(host: &str) -> Option<web_sys::Element> {
    dom::doc().get_element_by_id(host)
}

/// Add a reaction class, restarting its animation if it is already running.
fn flash(host: &'static str, class: &'static str) {
    let Some(el) = host_el(host) else { return };
    let _ = el.class_list().remove_1(class);
    if let Some(h) = el.dyn_ref::<web_sys::HtmlElement>() {
        let _ = h.offset_width(); // restart the animation
    }
    let _ = el.class_list().add_1(class);
    later(REACTION_LIFETIME_MS, move || {
        if let Some(el) = host_el(host) {
            let _ = el.class_list().remove_1(class);
        }
    });
}

/// KF7 correct: each key gets its ring out from the keyboard's centre; the
/// stylesheet staggers the wave by ring.
fn wave(host: &'static str) {
    let Some(el) = host_el(host) else { return };
    let r = el.get_bounding_client_rect();
    let (cx, cy) = (r.left() + r.width() / 2.0, r.top() + r.height() / 2.0);
    let Ok(keys) = el.query_selector_all(".skb-key") else { return };
    let mut far = 1.0f64;
    let mut dists = Vec::new();
    for i in 0..keys.length() {
        let Some(k) = keys.get(i).and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok()) else { continue };
        let b = k.get_bounding_client_rect();
        let d = ((b.left() + b.width() / 2.0 - cx).powi(2) + (b.top() + b.height() / 2.0 - cy).powi(2)).sqrt();
        far = far.max(d);
        dists.push((k, d));
    }
    for (k, d) in dists {
        let ring = (d / far * MAX_RINGS).round();
        let _ = k.style().set_property("--skb-ring", &ring.to_string());
    }
    flash(host, "skb-wave");
}

/// KF7 streak warmth: the keycaps warm one step per consecutive correct
/// answer, capped at the three streak tokens, reset by a miss.
fn warm(host: &str, streak: u32) {
    let Some(el) = host_el(host) else { return };
    let step = streak.min(3);
    if step == 0 {
        let _ = el.remove_attribute("data-skb-warm");
    } else {
        let _ = el.set_attribute("data-skb-warm", &step.to_string());
    }
}

/// KF6: the keys of a correct answer, as stars joined in typing order.
fn constellation(host: &'static str, answer: &str) {
    let points = star_points(host, answer);
    if points.len() < 2 {
        return;
    }
    let doc = dom::doc();
    let poly: String = points.iter().map(|(x, y, _)| format!("{x:.1},{y:.1}")).collect::<Vec<_>>().join(" ");
    let mut svg = format!(
        "<svg class=\"skb-constellation\" aria-hidden=\"true\" xmlns=\"http://www.w3.org/2000/svg\"><polyline points=\"{poly}\"/>"
    );
    for (x, y, ring) in &points {
        svg.push_str(&format!("<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"3.5\"/>"));
        if *ring {
            // A long-press letter's star sits on its base key with a small ring (KF6.2).
            svg.push_str(&format!("<circle class=\"ring\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"8\"/>"));
        }
    }
    svg.push_str("</svg>");
    let Ok(wrap) = doc.create_element("div") else { return };
    wrap.set_inner_html(&svg);
    let Some(node) = wrap.first_element_child() else { return };
    if let Some(body) = doc.body() {
        let _ = body.append_child(&node);
    }
    // A tap skips it.
    let n2 = node.clone();
    let skip = Closure::<dyn FnMut()>::new(move || n2.remove());
    let _ = node.add_event_listener_with_callback("pointerdown", skip.as_ref().unchecked_ref());
    skip.forget();
    later(CONSTELLATION_LIFETIME_MS, move || node.remove());
}

/// Where each unit of `answer` was typed on `host`: the key's centre, and
/// whether the letter was a long-press alternate (its star gets a ring).
/// Korean is decomposed into the jamo that were pressed; a Vietnamese tone is
/// found on the tone row when the main keyboard has one.
fn star_points(host: &'static str, answer: &str) -> Vec<(f64, f64, bool)> {
    let Some(attr) = crate::spellkb::attr_of(host) else { return Vec::new() };
    let Some(el) = host_el(host) else { return Vec::new() };
    let center = |k: &web_sys::Element| {
        let b = k.get_bounding_client_rect();
        (b.left() + b.width() / 2.0, b.top() + b.height() / 2.0)
    };
    let find = |key: &Key| -> Option<web_sys::Element> {
        let v = attr.value(key).replace('"', "\\\"");
        el.query_selector(&format!(".skb-key[{}=\"{v}\"]", attr_name(attr))).ok().flatten()
    };
    let mut out = Vec::new();
    for unit in units(answer) {
        let key = Key::Unit(unit.clone());
        if let Some(k) = find(&key) {
            let (x, y) = center(&k);
            out.push((x, y, false));
            continue;
        }
        // A long-press alternate: its base key, with a ring.
        let base = el
            .query_selector_all(".skb-key[data-acc]")
            .ok()
            .and_then(|list| (0..list.length()).filter_map(|i| list.get(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok())).find(|k| k.get_attribute("data-acc").is_some_and(|a| a.contains(unit.as_str()))));
        if let Some(k) = base {
            let (x, y) = center(&k);
            out.push((x, y, true));
            continue;
        }
        // A tone mark: the tone row (main keyboard), when the letter's own key held none.
        let mut cs = unit.chars();
        if let (Some(m), None) = (cs.next(), cs.next()) {
            if crate::viet::TONE_MARKS.contains(&m) {
                if let Some(k) = dom::doc().query_selector(&format!("#kbToneRow .skb-key[data-tone=\"{:x}\"]", m as u32)).ok().flatten() {
                    let (x, y) = center(&k);
                    out.push((x, y, false));
                }
            }
        }
    }
    out
}

/// The units a player pressed for `answer`: Korean syllables back to jamo, a
/// Vietnamese toned vowel into its letter and its tone, everything else as is.
pub fn units(answer: &str) -> Vec<String> {
    let mut out = Vec::new();
    for c in answer.nfc() {
        if let Some((i, m, f)) = crate::hangul::parts(c) {
            for j in [i, m, f] {
                if j != '\0' {
                    out.push(j.to_string());
                }
            }
            continue;
        }
        let d: Vec<char> = c.nfd().collect();
        let tone = d.iter().copied().find(|x| crate::viet::TONE_MARKS.contains(x));
        match tone {
            Some(t) => {
                let letter: String = d.iter().filter(|x| **x != t).collect::<String>().nfc().collect();
                out.push(letter);
                out.push(t.to_string());
            }
            None => out.push(c.to_string()),
        }
    }
    out
}

fn attr_name(a: Attr) -> &'static str {
    match a {
        Attr::Raw(n) | Attr::Prefixed(n) | Attr::ToneHex(n) => n,
    }
}

fn later(ms: i32, f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    let _ = dom::window().set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_withheld_verdict_never_reaches_the_keys() {
        // I7 / K-D23: only per_word, and Spelluzzle's Duel, show right and wrong.
        assert_eq!(shown(Verdict::PerWord, false), Shown::Verdict);
        assert_eq!(shown(Verdict::None, false), Shown::Neutral);
        assert_eq!(shown(Verdict::None, true), Shown::Neutral);
        assert_eq!(shown(Verdict::DuelOnly, false), Shown::Neutral, "a solo Spelluzzle board withholds it");
        assert_eq!(shown(Verdict::DuelOnly, true), Shown::Verdict);
    }

    #[test]
    fn units_are_what_was_pressed() {
        assert_eq!(units("cat"), ["c", "a", "t"]);
        assert_eq!(units("học"), ["h", "o", "\u{323}", "c"], "a toned vowel is its letter, then its tone");
        assert_eq!(units("người"), ["n", "g", "ư", "ơ", "\u{300}", "i"]);
        assert_eq!(units("한"), ["ㅎ", "ㅏ", "ㄴ"], "a syllable is the jamo that built it");
    }
}
