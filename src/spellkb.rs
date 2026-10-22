//! CC-SPELL-KEYBOARD — the one shared Spell keyboard (Phase A: KF1, KF2, KF10).
//!
//! The app already had its own keyboards, eight of them: the main game keyboard
//! and a separate key grid in Bee, Word Chains, BoardGame (shared with
//! RandoSpell and Spelluzzle), SpellDoku, Spell Search and Spell Cross. Each
//! built its own buttons, its own labels and its own listeners. This module is
//! the single place keys are drawn and pressed, so themed keycaps, key feel and
//! accessibility (later phases) are written once.
//!
//! Signing sheet K-Q1: consolidation keeps every surface's behaviour. So what a
//! surface does with a key stays the surface's business (its `sink`); what
//! varies between surfaces today — which rows, whether keys long-press, which
//! attribute tags a key for its tests and styles — is a [`Surface`] setting,
//! never a branch in here. Behind `keyboard.core` (default off); off, nothing in
//! this module runs.
//!
//! The two rules that come before everything else hold by construction:
//! nothing here sees the target word (I2, the keyboard never helps), and a key
//! hands its sink exactly the unit on the cap (I1, the keyboard never corrects).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use unicode_normalization::UnicodeNormalization;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::dom;

// ---------- keys (the data model, moved here from boardgame_input) ----------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    /// Append this unit (for Korean, feed it to the Hangul automaton).
    Unit(String),
    /// Vietnamese tone mark applied to the last vowel (`viet::retone`).
    Tone(char),
    /// Japanese modifier: dakuten, handakuten or small kana, applied to the last
    /// typed kana (BoardGame D29: modifier keys are allowed in that mode's own
    /// key grid; the main keyboard uses long-press).
    Mod(Modifier),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    Dakuten,
    Handakuten,
    Small,
}

const DAKUTEN: char = '\u{3099}';
const HANDAKUTEN: char = '\u{309a}';
/// Large kana and the small form it takes (the small-kana key swaps them).
pub(crate) const SMALL_PAIRS: [(char, char); 10] =
    [('あ', 'ぁ'), ('い', 'ぃ'), ('う', 'ぅ'), ('え', 'ぇ'), ('お', 'ぉ'), ('つ', 'っ'), ('や', 'ゃ'), ('ゆ', 'ゅ'), ('よ', 'ょ'), ('わ', 'ゎ')];

/// What the modifier makes of `c`, or None when `c` cannot take it. Voicing and
/// semi-voicing toggle (は ば は; は ぱ は); the small key swaps large and small.
pub fn apply_mod(c: char, m: Modifier) -> Option<char> {
    match m {
        Modifier::Small => SMALL_PAIRS.iter().find_map(|&(big, small)| if c == big { Some(small) } else if c == small { Some(big) } else { None }),
        Modifier::Dakuten | Modifier::Handakuten => {
            let mark = if m == Modifier::Dakuten { DAKUTEN } else { HANDAKUTEN };
            let d: Vec<char> = c.nfd().collect();
            let single = |v: &[char]| -> Option<char> {
                let n: Vec<char> = v.iter().copied().nfc().collect();
                (n.len() == 1).then(|| n[0])
            };
            match d.as_slice() {
                [base, x] if *x == mark => Some(*base),
                [base] => single(&[*base, mark]).filter(|r| *r != c),
                _ => None,
            }
        }
    }
}

impl Key {
    /// What the keycap shows. A tone mark rides on a dotted circle.
    pub fn face(&self) -> String {
        match self {
            Key::Unit(u) if u == " " => "\u{2423}".to_string(),
            Key::Unit(u) => u.clone(),
            Key::Tone(m) => format!("\u{25cc}{m}"),
            Key::Mod(Modifier::Dakuten) => "\u{309b}".to_string(),
            Key::Mod(Modifier::Handakuten) => "\u{309c}".to_string(),
            Key::Mod(Modifier::Small) => "\u{5c0f}".to_string(),
        }
    }

    /// The `data-k` attribute value in the prefixed form.
    pub fn data(&self) -> String {
        match self {
            Key::Unit(u) => format!("u:{u}"),
            Key::Tone(m) => format!("t:{:x}", *m as u32),
            Key::Mod(Modifier::Dakuten) => "m:d".to_string(),
            Key::Mod(Modifier::Handakuten) => "m:h".to_string(),
            Key::Mod(Modifier::Small) => "m:s".to_string(),
        }
    }

    pub fn parse(d: &str) -> Option<Key> {
        if let Some(u) = d.strip_prefix("u:") {
            return Some(Key::Unit(u.to_string()));
        }
        match d {
            "m:d" => return Some(Key::Mod(Modifier::Dakuten)),
            "m:h" => return Some(Key::Mod(Modifier::Handakuten)),
            "m:s" => return Some(Key::Mod(Modifier::Small)),
            _ => {}
        }
        let hex = d.strip_prefix("t:")?;
        char::from_u32(u32::from_str_radix(hex, 16).ok()?).map(Key::Tone)
    }

    /// The VoiceOver label (KF10): the character itself, a name for the keys
    /// whose face is not one, and the language's own name for each tone and kana
    /// mark, so the five Vietnamese tone keys are no longer all "tone mark".
    pub fn label(&self) -> String {
        match self {
            Key::Unit(u) => match u.as_str() {
                " " => "space".to_string(),
                "'" => "apostrophe".to_string(),
                "-" => "hyphen".to_string(),
                _ => u.to_lowercase(),
            },
            Key::Tone(m) => tone_name(*m).to_string(),
            Key::Mod(Modifier::Dakuten) => "濁点".to_string(),
            Key::Mod(Modifier::Handakuten) => "半濁点".to_string(),
            Key::Mod(Modifier::Small) => "小".to_string(),
        }
    }
}

/// The names Vietnamese gives its five tone marks: content, not UI, so the
/// same in every interface language.
pub fn tone_name(m: char) -> &'static str {
    match m {
        '\u{300}' => "huyền",
        '\u{301}' => "sắc",
        '\u{309}' => "hỏi",
        '\u{303}' => "ngã",
        '\u{323}' => "nặng",
        _ => "tone",
    }
}

// ---------- surfaces ----------

/// How a surface's keys are tagged. Each surface keeps the attribute its tests
/// and styles already select on, so turning `keyboard.core` on changes how keys
/// are built and pressed, not what anything outside the keyboard can see.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attr {
    /// The unit itself (`data-k="q"`); a tone is its mark character.
    Raw(&'static str),
    /// [`Key::data`] (`data-k="u:q"`, `t:309`, `m:d`).
    Prefixed(&'static str),
    /// The unit itself; a tone is its code point in hex (the main tone row).
    ToneHex(&'static str),
}

impl Attr {
    fn name(self) -> &'static str {
        match self {
            Attr::Raw(n) | Attr::Prefixed(n) | Attr::ToneHex(n) => n,
        }
    }

    pub fn value(self, k: &Key) -> String {
        match (self, k) {
            (Attr::Prefixed(_), k) => k.data(),
            (_, Key::Unit(u)) => u.clone(),
            (Attr::ToneHex(_), Key::Tone(m)) => format!("{:x}", *m as u32),
            (_, Key::Tone(m)) => m.to_string(),
            (_, Key::Mod(_)) => k.data(),
        }
    }

    pub fn parse(self, v: &str) -> Option<Key> {
        match self {
            Attr::Prefixed(_) => Key::parse(v),
            _ if v.starts_with("m:") => Key::parse(v),
            Attr::ToneHex(_) if v.chars().count() > 1 => {
                u32::from_str_radix(v, 16).ok().and_then(char::from_u32).map(Key::Tone)
            }
            _ => {
                let mut cs = v.chars();
                match (cs.next(), cs.next()) {
                    (Some(c), None) if crate::viet::TONE_MARKS.contains(&c) => Some(Key::Tone(c)),
                    _ => Some(Key::Unit(v.to_string())),
                }
            }
        }
    }
}

/// One mounted keyboard: where it draws and how its keys are tagged.
#[derive(Clone, Copy, Debug)]
pub struct Surface {
    /// The element the rows are drawn into.
    pub host: &'static str,
    /// The surface's existing row and key classes, kept beside `skb-row` /
    /// `skb-key` so its current layout rules still apply.
    pub row_class: &'static str,
    pub key_class: &'static str,
    pub attr: Attr,
    /// Whether keys carrying alternates open the long-press popover.
    pub long_press: bool,
}

/// One keycap.
#[derive(Clone, Debug)]
pub struct Cap {
    pub key: Key,
    /// Long-press alternates (only drawn where the surface long-presses).
    pub accents: String,
    /// An extra class for this cap (the BoardGame modifier tint, a shift key).
    pub class: &'static str,
}

impl Cap {
    pub fn key(key: Key) -> Cap {
        Cap { key, accents: String::new(), class: "" }
    }
    pub fn unit(c: char) -> Cap {
        Cap::key(Key::Unit(c.to_string()))
    }
}

/// Raw markup placed around the last row: the surface's own controls (a shift
/// key before the letters; delete and go after them), which keep their own
/// wiring and are not keys.
#[derive(Clone, Copy, Debug, Default)]
pub struct Edges<'a> {
    pub lead: &'a str,
    pub tail: &'a str,
}

/// The markup for `rows` on `s`. A surface with no row class draws its keys
/// flat, straight into the host (a host that is itself one row).
pub fn markup(s: &Surface, rows: &[Vec<Cap>], edges: Edges) -> String {
    let mut html = String::new();
    let last = rows.len().saturating_sub(1);
    let wrap = !s.row_class.is_empty();
    for (ri, row) in rows.iter().enumerate() {
        if wrap {
            html.push_str(&format!("<div class=\"skb-row {}\">", s.row_class));
        }
        if ri == last {
            html.push_str(edges.lead);
        }
        for cap in row {
            html.push_str(&button(s, cap));
        }
        if ri == last {
            html.push_str(edges.tail);
        }
        if wrap {
            html.push_str("</div>");
        }
    }
    html
}

fn button(s: &Surface, cap: &Cap) -> String {
    let val = dom::escape_html(&s.attr.value(&cap.key));
    let acc = if s.long_press && !cap.accents.is_empty() {
        format!(" data-acc=\"{}\"", dom::escape_html(&cap.accents))
    } else {
        String::new()
    };
    let hint = if acc.is_empty() { "" } else { " has-acc" };
    let mod_cls = if matches!(cap.key, Key::Mod(_)) { " skb-mod" } else if matches!(cap.key, Key::Tone(_)) { " skb-tone" } else { "" };
    format!(
        "<button type=\"button\" class=\"skb-key {}{}{}{} {}\" {}=\"{}\"{} aria-label=\"{}\">{}</button>",
        s.key_class,
        hint,
        mod_cls,
        if cap.class.is_empty() { "" } else { " " },
        cap.class,
        s.attr.name(),
        val,
        acc,
        dom::escape_html(&cap.key.label()),
        dom::escape_html(&cap.key.face()),
    )
}

// ---------- pressing ----------

type Sink = Rc<dyn Fn(&Key)>;

struct Mounted {
    surface: Surface,
    sink: Sink,
}

thread_local! {
    static MOUNTED: RefCell<HashMap<&'static str, Mounted>> = RefCell::new(HashMap::new());
    static WIRED: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

/// Draw `rows` into `s.host` and route its presses to `sink`. Safe to call on
/// every re-render: the host's listener is installed once and the latest sink
/// is the one a press reaches.
pub fn mount(s: Surface, rows: &[Vec<Cap>], edges: Edges, sink: impl Fn(&Key) + 'static) {
    dom::set_html(s.host, &markup(&s, rows, edges));
    MOUNTED.with(|m| {
        m.borrow_mut().insert(s.host, Mounted { surface: s, sink: Rc::new(sink) });
    });
    let first = WIRED.with(|w| {
        let mut w = w.borrow_mut();
        if w.contains(&s.host) {
            false
        } else {
            w.push(s.host);
            true
        }
    });
    if first {
        wire_host(s.host);
    }
    wire_long_press();
}

fn sink_for(host: &str) -> Option<(Surface, Sink)> {
    MOUNTED.with(|m| m.borrow().get(host).map(|x| (x.surface, x.sink.clone())))
}

fn wire_host(host: &'static str) {
    let Some(el) = dom::doc().get_element_by_id(host) else { return };
    let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        if HOLD.with(|h| std::mem::take(&mut h.borrow_mut().suppress_click)) {
            return; // the popover already typed this press
        }
        let Some((s, sink)) = sink_for(host) else { return };
        let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        let Some(key) = t.closest(".skb-key").ok().flatten() else { return };
        let Some(k) = key.get_attribute(s.attr.name()).and_then(|v| s.attr.parse(&v)) else { return };
        // The click keeps bubbling, exactly as before: a screen may do other work
        // on any tap (Spelluzzle settles its decode ripple). A surface's own key
        // handler steps aside for shared keys instead (`is_shared_key`).
        sink(&k);
    });
    let _ = el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
    cb.forget();
}

/// True when `el` is a key the shared keyboard already handled. A surface's own
/// delegated key handler asks this and steps aside, so a press is typed once.
pub fn is_shared_key(el: &web_sys::Element) -> bool {
    el.class_list().contains("skb-key")
}

// ---------- long-press ----------

#[derive(Default)]
struct Hold {
    handle: Option<i32>,
    _cb: Option<Closure<dyn FnMut()>>,
    open: bool,
    suppress_click: bool,
    host: Option<&'static str>,
    anchor: Option<web_sys::Element>,
    options: Vec<char>,
    sel: usize,
}

thread_local! {
    static HOLD: RefCell<Hold> = RefCell::new(Hold::default());
    static LP_WIRED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The hold before alternates open (K-D16).
pub const HOLD_MS: i32 = 350;

fn wire_long_press() {
    if LP_WIRED.with(|w| w.replace(true)) {
        return;
    }
    dom::on_window::<web_sys::PointerEvent, _>("pointerdown", move |e| {
        // A new press always starts clean: a mouse release over the popover never
        // sends the click the flag waits for, and must not eat the next key.
        HOLD.with(|h| h.borrow_mut().suppress_click = false);
        cancel_timer();
        let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        let Some(key) = t.closest(".skb-key[data-acc]").ok().flatten() else { return };
        let host = MOUNTED.with(|m| {
            m.borrow().keys().copied().find(|h| key.closest(&format!("#{h}")).ok().flatten().is_some())
        });
        let Some(host) = host else { return };
        let Some((s, _)) = sink_for(host) else { return };
        let Some(Key::Unit(base)) = key.get_attribute(s.attr.name()).and_then(|v| s.attr.parse(&v)) else { return };
        let Some(base) = base.chars().next() else { return };
        let mut options = vec![base];
        options.extend(key.get_attribute("data-acc").unwrap_or_default().chars());
        HOLD.with(|h| {
            let mut h = h.borrow_mut();
            h.open = false;
            h.host = Some(host);
            h.anchor = Some(key.clone());
            h.options = options;
            h.sel = 0;
        });
        let cb = Closure::<dyn FnMut()>::new(open_popover);
        let handle = dom::window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), HOLD_MS)
            .unwrap_or(0);
        HOLD.with(|h| {
            let mut h = h.borrow_mut();
            h.handle = Some(handle);
            h._cb = Some(cb);
        });
    });
    dom::on_window::<web_sys::PointerEvent, _>("pointermove", move |e| {
        if HOLD.with(|h| h.borrow().open) {
            update_selection(e.client_x() as f64);
        }
    });
    dom::on_window::<web_sys::PointerEvent, _>("pointerup", move |_| commit());
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

fn open_popover() {
    let (options, anchor) = HOLD.with(|h| {
        let h = h.borrow();
        (h.options.clone(), h.anchor.clone())
    });
    let Some(anchor) = anchor else { return };
    if options.len() < 2 {
        return;
    }
    let rect = anchor.get_bounding_client_rect();
    let mut html = String::new();
    for (i, c) in options.iter().enumerate() {
        let cls = if i == 0 { "skb-acc kb-acc sel" } else { "skb-acc kb-acc" };
        let s = c.to_string();
        html.push_str(&format!(
            "<button type=\"button\" class=\"{cls}\" data-i=\"{i}\" aria-label=\"{}\">{}</button>",
            dom::escape_html(&s),
            dom::escape_html(&s)
        ));
    }
    let doc = dom::doc();
    let pop = doc.get_element_by_id("skbPop").unwrap_or_else(|| {
        let el = doc.create_element("div").unwrap();
        el.set_id("skbPop");
        let _ = el.set_attribute("role", "listbox");
        if let Some(body) = doc.body() {
            let _ = body.append_child(&el);
        }
        el
    });
    pop.set_inner_html(&html);
    if let Some(p) = pop.dyn_ref::<web_sys::HtmlElement>() {
        let _ = p.style().set_property("left", &format!("{}px", rect.left() + rect.width() / 2.0));
        let _ = p.style().set_property("top", &format!("{}px", rect.top() - 6.0));
    }
    let _ = pop.set_attribute("class", "kb-popover skb-popover show");
    HOLD.with(|h| {
        let mut h = h.borrow_mut();
        h.open = true;
        h.sel = 0;
    });
    crate::haptics::key_tap();
}

fn update_selection(client_x: f64) {
    let Some(pop) = dom::doc().get_element_by_id("skbPop") else { return };
    let Ok(btns) = pop.query_selector_all(".skb-acc") else { return };
    let mut best = 0usize;
    let mut best_dist = f64::MAX;
    for i in 0..btns.length() {
        let Some(b) = btns.get(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) else { continue };
        let r = b.get_bounding_client_rect();
        let d = (r.left() + r.width() / 2.0 - client_x).abs();
        if d < best_dist {
            best_dist = d;
            best = i as usize;
        }
    }
    HOLD.with(|h| h.borrow_mut().sel = best);
    for i in 0..btns.length() {
        if let Some(b) = btns.get(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
            let _ = b.class_list().toggle_with_force("sel", i as usize == best);
        }
    }
}

fn commit() {
    let (open, ch, host) = HOLD.with(|h| {
        let h = h.borrow();
        (h.open, h.options.get(h.sel).copied(), h.host)
    });
    cancel_timer();
    if !open {
        return; // released before the popover opened: the click types the base
    }
    close_popover();
    let (Some(c), Some(host)) = (ch, host) else { return };
    let Some((_, sink)) = sink_for(host) else { return };
    HOLD.with(|h| h.borrow_mut().suppress_click = true);
    sink(&Key::Unit(c.to_string()));
}

fn close_popover() {
    HOLD.with(|h| h.borrow_mut().open = false);
    if let Some(pop) = dom::doc().get_element_by_id("skbPop") {
        let _ = pop.set_attribute("class", "kb-popover skb-popover");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: Surface = Surface { host: "x", row_class: "bee-row", key_class: "bee-key", attr: Attr::Raw("data-k"), long_press: true };

    #[test]
    fn attributes_round_trip_in_every_form() {
        let keys = [Key::Unit("q".into()), Key::Unit(" ".into()), Key::Unit("ñ".into()), Key::Tone('\u{309}'), Key::Mod(Modifier::Small)];
        for attr in [Attr::Raw("data-k"), Attr::Prefixed("data-k"), Attr::ToneHex("data-tone")] {
            for k in &keys {
                assert_eq!(attr.parse(&attr.value(k)).as_ref(), Some(k), "{attr:?} {k:?}");
            }
        }
    }

    #[test]
    fn each_form_keeps_the_value_its_tests_select_on() {
        let q = Key::Unit("q".into());
        assert_eq!(Attr::Raw("data-k").value(&q), "q");
        assert_eq!(Attr::Prefixed("data-k").value(&q), "u:q");
        assert_eq!(Attr::Raw("data-sd-key").value(&Key::Tone('\u{309}')), "\u{309}");
        assert_eq!(Attr::ToneHex("data-tone").value(&Key::Tone('\u{309}')), "309");
    }

    #[test]
    fn every_key_has_a_label_and_the_tones_are_told_apart() {
        let tones: Vec<String> = crate::viet::TONE_MARKS.iter().map(|&m| Key::Tone(m).label()).collect();
        let mut uniq = tones.clone();
        uniq.dedup();
        assert_eq!(uniq.len(), 5, "{tones:?}");
        assert!(tones.iter().all(|t| !t.is_empty() && t != "tone"));
        assert_eq!(Key::Unit("Q".into()).label(), "q");
        assert_eq!(Key::Unit(" ".into()).label(), "space");
    }

    #[test]
    fn markup_carries_the_surface_classes_and_long_press_only_where_asked() {
        let rows = vec![vec![Cap { key: Key::Unit("e".into()), accents: "éè".into(), class: "" }, Cap::unit('q')]];
        let h = markup(&S, &rows, Edges { lead: "<i id=\"shift\"></i>", tail: "<button id=\"del\"></button>" });
        assert!(h.contains("class=\"skb-row bee-row\""));
        assert!(h.contains("skb-key bee-key has-acc"));
        assert!(h.contains("data-k=\"e\" data-acc=\"éè\""));
        assert!(h.contains("aria-label=\"q\""));
        assert!(h.starts_with("<div class=\"skb-row bee-row\"><i id=\"shift\"></i><button"));
        assert!(h.ends_with("<button id=\"del\"></button></div>"));
        let no_lp = Surface { long_press: false, ..S };
        assert!(!markup(&no_lp, &rows, Edges::default()).contains("data-acc"), "a surface without long-press draws no alternates");
        let flat = Surface { row_class: "", ..S };
        assert!(markup(&flat, &rows, Edges::default()).starts_with("<button"), "no row class: keys go straight into the host");
    }

    #[test]
    fn the_keyboard_never_sees_a_word() {
        // I2 by construction: nothing that draws keys takes a target. If a
        // parameter for one is ever added, this signature check is where it shows.
        let _: fn(&Surface, &[Vec<Cap>], Edges) -> String = markup;
    }
}
