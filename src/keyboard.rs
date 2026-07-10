//! The custom on-screen keyboard — the only way to type an answer on touch
//! devices. There is no `<input>` anywhere in the answer flow, so the iOS
//! system keyboard (and its dictation key + autocorrect / QuickType bar) never
//! opens during a round: the anti-cheat property this module exists to
//! guarantee. Physical keyboards still work on desktop via a window-level
//! keydown that routes into the same answer state (game::type_char/backspace).
//!
//! The layout follows the active word language (§2.2): AZERTY for French,
//! QWERTZ + ä ö ü for German, extra ñ / ç / å ä ö keys, and long-press accent
//! popovers so every diacritic in a locale's word list is reachable in ≤2
//! gestures. My Words falls back to the English layout + apostrophe/hyphen keys.

use std::cell::RefCell;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::consts::MINE;
use crate::App;
use crate::{dom, game};

/// A keyboard layout: three base rows (lowercase, extra letter keys already
/// placed) plus long-press accent sets keyed by base character.
struct Layout {
    rows: [&'static str; 3],
    long_press: &'static [(char, &'static str)],
}

// Accent popovers per §2.2. The base key is prepended automatically, so these
// list only the accented alternatives.
const EN: Layout = Layout { rows: ["qwertyuiop", "asdfghjkl", "zxcvbnm"], long_press: &[] };
const ES: Layout = Layout {
    rows: ["qwertyuiop", "asdfghjklñ", "zxcvbnm"],
    long_press: &[('a', "á"), ('e', "é"), ('i', "í"), ('o', "ó"), ('u', "úü")],
};
const FR: Layout = Layout {
    rows: ["azertyuiop", "qsdfghjklm", "wxcvbn"],
    long_press: &[('e', "éèêë"), ('a', "àâ"), ('c', "ç"), ('i', "îï"), ('o', "ôœ"), ('u', "ùûü"), ('y', "ÿ")],
};
const DE: Layout = Layout { rows: ["qwertzuiopü", "asdfghjklöä", "yxcvbnm"], long_press: &[('s', "ß")] };
const PT: Layout = Layout {
    rows: ["qwertyuiop", "asdfghjklç", "zxcvbnm"],
    long_press: &[('a', "áàâã"), ('e', "éê"), ('i', "í"), ('o', "óôõ"), ('u', "ú")],
};
const IT: Layout = Layout {
    rows: ["qwertyuiop", "asdfghjkl", "zxcvbnm"],
    long_press: &[('a', "à"), ('e', "èé"), ('i', "ì"), ('o', "ò"), ('u', "ù")],
};
const NL: Layout = Layout {
    rows: ["qwertyuiop", "asdfghjkl", "zxcvbnm"],
    long_press: &[('e', "éë"), ('i', "ï"), ('o', "ö")],
};
const PL: Layout = Layout {
    rows: ["qwertyuiop", "asdfghjkl", "zxcvbnm"],
    long_press: &[('a', "ą"), ('c', "ć"), ('e', "ę"), ('l', "ł"), ('n', "ń"), ('o', "ó"), ('s', "ś"), ('z', "źż")],
};
const SV: Layout = Layout { rows: ["qwertyuiopå", "asdfghjklöä", "zxcvbnm"], long_press: &[] };
const NB: Layout = Layout { rows: ["qwertyuiopå", "asdfghjkløæ", "zxcvbnm"], long_press: &[] };
const TR: Layout = Layout {
    rows: ["qwertyuıopğ", "asdfghjklşi", "zxcvbnmöçü"],
    long_press: &[],
};

fn layout_for(locale: &str) -> &'static Layout {
    match locale {
        "es" => &ES,
        "fr" => &FR,
        "de" => &DE,
        "pt" => &PT,
        "it" => &IT,
        "nl" => &NL,
        "pl" => &PL,
        "sv" => &SV,
        "nb" => &NB,
        "tr" => &TR,
        _ => &EN,
    }
}

/// Which keyboard to show: My Words always uses the English layout (its words
/// are English + apostrophe/hyphen); built-in languages use their own.
fn keyboard_locale(app: &App) -> String {
    let lang = app.borrow().lang.clone();
    if lang == MINE {
        "en".to_string()
    } else {
        lang
    }
}

pub fn setup(app: &App) {
    build_keys(app);
    wire_actions(app);
    wire_physical(app);
    wire_long_press(app);
    wire_paste_guard();
    game::sync_keyboard(app);
}

/// Rebuild the letter keys for the active language (called on a language
/// change). Only the key buttons are re-created; the window-level handlers
/// (physical keyboard, long-press, paste guard) stay wired from `setup`.
pub fn rebuild(app: &App) {
    close_popover();
    build_keys(app);
    game::sync_keyboard(app);
}

fn key_button(ch: char, punct: bool, accents: &str) -> String {
    let cls = if punct { "kb-key kb-punct" } else { "kb-key" };
    let label = match ch {
        '\'' => "apostrophe".to_string(),
        '-' => "hyphen".to_string(),
        c => c.to_string(),
    };
    let face = if ch.is_alphabetic() {
        ch.to_uppercase().to_string()
    } else {
        ch.to_string()
    };
    let acc_attr = if accents.is_empty() {
        String::new()
    } else {
        format!(" data-acc=\"{}\"", dom::escape_html(accents))
    };
    let hint = if accents.is_empty() { "" } else { " has-acc" };
    format!("<button class=\"{cls}{hint}\" data-k=\"{ch}\"{acc_attr} aria-label=\"{label}\">{face}</button>")
}

fn build_keys(app: &App) {
    let locale = keyboard_locale(app);
    let layout = layout_for(&locale);
    let accents_for = |c: char| -> &'static str { layout.long_press.iter().find(|(k, _)| *k == c).map(|(_, a)| *a).unwrap_or("") };

    let row = |letters: &str, punct: bool| {
        let mut h = String::new();
        for c in letters.chars() {
            h.push_str(&key_button(c, false, accents_for(c)));
        }
        if punct {
            h.push_str(&key_button('\'', true, ""));
            h.push_str(&key_button('-', true, ""));
        }
        h
    };
    dom::set_html("kbRow1", &row(layout.rows[0], false));
    dom::set_html("kbRow2", &row(layout.rows[1], false));
    dom::set_html("kbRow3", &row(layout.rows[2], true));

    // Uniform key width is derived from the widest row's column count (10 for
    // QWERTY, 11 for QWERTZ/Swedish), so every key is the same size regardless
    // of language. The +2 accounts for the apostrophe/hyphen keys shown in My
    // Words so its bottom row still fits without shrinking the others.
    let base_cols = layout.rows.iter().map(|r| r.chars().count()).max().unwrap_or(10);
    let cols = base_cols.max(layout.rows[2].chars().count() + 2);
    if let Some(el) = dom::doc().get_element_by_id("gameKeyboard").and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = el.style().set_property("--kb-cols", &cols.to_string());
    }

    // Wire each letter/punctuation key: a tap types its base character (unless a
    // long-press popover consumed the gesture — see wire_long_press).
    if let Ok(list) = dom::doc().query_selector_all("#gameKeyboard .kb-key[data-k]") {
        for i in 0..list.length() {
            let Some(node) = list.get(i) else { continue };
            let Some(el) = node.dyn_ref::<web_sys::HtmlElement>() else { continue };
            let Some(ch) = el.get_attribute("data-k").and_then(|k| k.chars().next()) else { continue };
            let a = app.clone();
            let cb = Closure::<dyn FnMut()>::new(move || {
                if HOLD.with(|h| {
                    let mut h = h.borrow_mut();
                    let s = h.suppress_click;
                    h.suppress_click = false;
                    s
                }) {
                    return; // the popover already typed a character
                }
                game::type_char(&a, ch);
            });
            let _ = el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
            cb.forget();
        }
    }
}

// ---------- long-press accent popovers ----------

#[derive(Default)]
struct Hold {
    handle: Option<i32>,
    _cb: Option<Closure<dyn FnMut()>>,
    open: bool,
    suppress_click: bool,
    options: Vec<char>, // base char at index 0, then accents
    sel: usize,
}

thread_local! {
    static HOLD: RefCell<Hold> = RefCell::new(Hold::default());
}

const HOLD_MS: i32 = 350;

/// Attach pointer handling for long-press. Per-key `pointerdown` arms a 350 ms
/// timer that opens the accent popover; window-level `pointermove`/`pointerup`
/// drive selection and commit. Keys with no accents are untouched (a plain tap
/// still types via the click handler above).
fn wire_long_press(app: &App) {
    // pointerdown on any accent key -> arm the hold timer.
    {
        let a = app.clone();
        dom::on_window::<web_sys::PointerEvent, _>("pointerdown", move |e| {
            let Some(target) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
            let Some(key) = target.closest(".kb-key[data-acc]").ok().flatten() else {
                return;
            };
            let Some(base) = key.get_attribute("data-k").and_then(|k| k.chars().next()) else { return };
            let accents = key.get_attribute("data-acc").unwrap_or_default();
            let mut options = vec![base];
            options.extend(accents.chars());

            cancel_timer();
            HOLD.with(|h| {
                let mut h = h.borrow_mut();
                h.suppress_click = false;
                h.open = false;
                h.options = options;
                h.sel = 0;
            });
            let a2 = a.clone();
            let cb = Closure::<dyn FnMut()>::new(move || open_popover(&a2));
            let handle = dom::window()
                .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), HOLD_MS)
                .unwrap_or(0);
            HOLD.with(|h| {
                let mut h = h.borrow_mut();
                h.handle = Some(handle);
                h._cb = Some(cb);
            });
        });
    }
    // pointermove -> update selection while the popover is open.
    dom::on_window::<web_sys::PointerEvent, _>("pointermove", move |e| {
        if !HOLD.with(|h| h.borrow().open) {
            return;
        }
        update_selection(e.client_x() as f64);
    });
    // pointerup -> commit the selected accent, or (if still holding) let the tap
    // type the base char.
    {
        let a = app.clone();
        dom::on_window::<web_sys::PointerEvent, _>("pointerup", move |_| commit(&a));
    }
    dom::on_window::<web_sys::PointerEvent, _>("pointercancel", move |_| {
        cancel_timer();
        close_popover();
    });
}

fn cancel_timer() {
    HOLD.with(|h| {
        let mut h = h.borrow_mut();
        if let Some(handle) = h.handle.take() {
            dom::window().clear_timeout_with_handle(handle);
        }
        h._cb = None;
    });
}

fn open_popover(_app: &App) {
    let options = HOLD.with(|h| h.borrow().options.clone());
    if options.len() < 2 {
        return;
    }
    let base = options[0];
    // Locate the key button to anchor the popover above it.
    let sel = format!("#gameKeyboard .kb-key[data-k=\"{base}\"]");
    let Some(key) = dom::doc().query_selector(&sel).ok().flatten() else { return };
    let rect = key.get_bounding_client_rect();

    let mut html = String::new();
    for (i, c) in options.iter().enumerate() {
        let cls = if i == 0 { "kb-acc sel" } else { "kb-acc" };
        html.push_str(&format!("<button class=\"{cls}\" data-i=\"{i}\">{}</button>", c.to_uppercase()));
    }
    let doc = dom::doc();
    let pop = doc.get_element_by_id("kbPop").unwrap_or_else(|| {
        let el = doc.create_element("div").unwrap();
        el.set_id("kbPop");
        let _ = el.set_attribute("class", "kb-popover");
        if let Some(body) = doc.body() {
            let _ = body.append_child(&el);
        }
        el
    });
    pop.set_inner_html(&html);
    let left = rect.left() + rect.width() / 2.0;
    let top = rect.top() - 6.0;
    if let Some(style) = pop.dyn_ref::<web_sys::HtmlElement>() {
        let _ = style.style().set_property("left", &format!("{left}px"));
        let _ = style.style().set_property("top", &format!("{top}px"));
    }
    let _ = pop.set_attribute("class", "kb-popover show");
    HOLD.with(|h| {
        h.borrow_mut().open = true;
        h.borrow_mut().sel = 0;
    });
    crate::haptics::key_tap();
}

fn update_selection(client_x: f64) {
    let Some(pop) = dom::doc().get_element_by_id("kbPop") else { return };
    let Ok(btns) = pop.query_selector_all(".kb-acc") else { return };
    let mut best = 0usize;
    let mut best_dist = f64::MAX;
    for i in 0..btns.length() {
        let Some(b) = btns.get(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) else { continue };
        let r = b.get_bounding_client_rect();
        let center = r.left() + r.width() / 2.0;
        let d = (center - client_x).abs();
        if d < best_dist {
            best_dist = d;
            best = i as usize;
        }
    }
    HOLD.with(|h| h.borrow_mut().sel = best);
    for i in 0..btns.length() {
        if let Some(b) = btns.get(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
            let cls = if i as usize == best { "kb-acc sel" } else { "kb-acc" };
            let _ = b.set_attribute("class", cls);
        }
    }
}

fn commit(app: &App) {
    let (open, ch) = HOLD.with(|h| {
        let h = h.borrow();
        (h.open, h.options.get(h.sel).copied())
    });
    cancel_timer();
    if !open {
        return; // released before the popover opened -> the tap types the base
    }
    close_popover();
    if let Some(c) = ch {
        HOLD.with(|h| h.borrow_mut().suppress_click = true);
        game::type_char(app, c);
    }
}

fn close_popover() {
    HOLD.with(|h| h.borrow_mut().open = false);
    if let Some(pop) = dom::doc().get_element_by_id("kbPop") {
        let _ = pop.set_attribute("class", "kb-popover");
    }
}

fn wire_actions(app: &App) {
    {
        let a = app.clone();
        dom::on_click("kbBackspace", move || game::backspace(&a));
    }
    {
        let a = app.clone();
        dom::on_click("kbSubmit", move || game::submit_guess(&a));
    }
}

/// Physical keyboard (desktop): route letters / Backspace / Enter into the
/// answer state — unless focus is in a real text field (the name / import
/// boxes), so those keep working normally. Accepts any single alphabetic
/// character (including accents typed on a native locale keyboard).
fn wire_physical(app: &App) {
    let a = app.clone();
    dom::on_window::<web_sys::KeyboardEvent, _>("keydown", move |e| {
        if target_is_text_field(e.as_ref()) {
            return;
        }
        let key = e.key();
        if key == "Enter" {
            game::submit_guess(&a);
            return;
        }
        if key == "Backspace" {
            e.prevent_default();
            game::backspace(&a);
            return;
        }
        if e.ctrl_key() || e.meta_key() || e.alt_key() {
            return;
        }
        // Exactly one printable char, and one we accept.
        let mut chars = key.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            if c.is_alphabetic() || c == '\'' || c == '-' {
                e.prevent_default();
                let lower = c.to_lowercase().next().unwrap_or(c);
                game::type_char(&a, lower);
            }
        }
    });
}

/// Defense-in-depth: block paste on the game screen (dictation/paste insert a
/// whole word at once). A real text field like the import box still accepts it.
fn wire_paste_guard() {
    dom::on_window::<web_sys::Event, _>("paste", |e| {
        if !target_is_text_field(&e) {
            e.prevent_default();
        }
    });
}

fn target_is_text_field(e: &web_sys::Event) -> bool {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else {
        return false;
    };
    matches!(el.tag_name().to_uppercase().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn reachable(layout: &Layout) -> HashSet<char> {
        let mut s = HashSet::new();
        for row in layout.rows {
            s.extend(row.chars());
        }
        for (base, acc) in layout.long_press {
            s.insert(*base);
            s.extend(acc.chars());
        }
        s
    }

    /// The keyboard layout SSOT (assets/keyboards/{code}.json, consumed by the
    /// word-list pipeline's charset gate) must match the Rust layouts, so the
    /// gate and the runtime keyboard can never diverge.
    #[test]
    fn json_layouts_match_rust() {
        // (code, json) pairs — embedded at compile time from the SSOT files.
        let jsons: &[(&str, &str)] = &[
            ("en", include_str!("../assets/keyboards/en.json")),
            ("es", include_str!("../assets/keyboards/es.json")),
            ("fr", include_str!("../assets/keyboards/fr.json")),
            ("de", include_str!("../assets/keyboards/de.json")),
            ("pt", include_str!("../assets/keyboards/pt.json")),
            ("it", include_str!("../assets/keyboards/it.json")),
            ("nl", include_str!("../assets/keyboards/nl.json")),
            ("pl", include_str!("../assets/keyboards/pl.json")),
            ("sv", include_str!("../assets/keyboards/sv.json")),
            ("nb", include_str!("../assets/keyboards/nb.json")),
            ("tr", include_str!("../assets/keyboards/tr.json")),
        ];
        for (code, json) in jsons {
            let v: serde_json::Value = serde_json::from_str(json).unwrap();
            let layout = layout_for(code);
            let rows: Vec<String> = v["rows"].as_array().unwrap().iter().map(|r| r.as_str().unwrap().to_string()).collect();
            assert_eq!(rows, layout.rows.to_vec(), "{code} rows differ from JSON");
            let lp = &v["longPress"];
            assert_eq!(
                lp.as_object().map(|o| o.len()).unwrap_or(0),
                layout.long_press.len(),
                "{code} longPress count differs from JSON"
            );
            for (base, acc) in layout.long_press {
                let got = lp[base.to_string()].as_str().unwrap_or("");
                assert_eq!(got, *acc, "{code} longPress[{base}] differs from JSON");
            }
        }
    }

    /// §3.4 gate 1: every character in every built-in word (after the strict
    /// fold the player must reproduce) is reachable on that locale's keyboard.
    #[test]
    fn every_word_char_is_typeable() {
        for (code, _) in crate::consts::BUILTIN_LANGS {
            let reach = reachable(layout_for(code));
            for tier in ["easy", "medium", "hard", "expert"] {
                for w in crate::words::tier_for(code, tier) {
                    for c in crate::norm::fold_strict(w).chars() {
                        assert!(reach.contains(&c), "locale {code}: char {c:?} in {w:?} not reachable on keyboard");
                    }
                }
            }
        }
    }
}
