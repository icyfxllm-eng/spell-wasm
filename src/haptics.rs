//! Tactile feedback on the native build via `@capacitor/haptics`, reached
//! (no bundler) through `window.Capacitor.Plugins.Haptics`. Fire-and-forget:
//! every call is a no-op on the web (plugin absent) and never throws, so game
//! code can call it unconditionally.
//!
//! CC-VISUAL-ECLIPSE F6: this is the ONE haptic module (AT-17,
//! scripts/haptics-check.mjs fails on any other path to the plugin). Every call
//! names a [`Kind`]; [`pattern`] is the single mapping table.
//!
//! With `visual.haptics` OFF the table is today's: a light tap for a correct
//! spelling and for every other tick, an ERROR notification for a miss (a
//! single medium tap in Kid Mode), no cap. With it ON (signed V-D17 and E-Q3):
//! - correct = light impact; wrong = two light impacts 80 ms apart (gentle for
//!   everyone, so the kid softening has nothing left to soften);
//!   milestone = medium impact; picker selection = selection tick; Board Game
//!   hops keep their D-P6 light tick.
//! - keystrokes are their own class, governed by the Key feel toggle and exempt
//!   from the cap; everything else is governed by the Haptics toggle;
//! - never more than 2 capped haptics per word (a word is whatever the audio
//!   router last played, or the base game's round; surfaces with no word, like
//!   Letter Forge's submissions, are uncapped and fire at most once per action);
//! - on the web the no-op is logged once per session (V-D21).

use std::cell::{Cell, RefCell};

use js_sys::{Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

/// What a haptic means. Call sites name the meaning; the table picks the feel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A correct spelling.
    Correct,
    /// A wrong spelling.
    Wrong,
    /// A milestone (F7).
    Milestone,
    /// A picker or scroll selection.
    Selection,
    /// A keystroke on any on-screen keyboard (E-Q3: its own class).
    Key,
    /// Any other confirming tick: a duplicate, an accepted find, a tile landing.
    Tap,
    /// A Board Game piece hop (D-P6 mapping, preserved).
    Hop,
}

/// One physical pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    Impact(&'static str),
    /// Two impacts of this style, this many ms apart.
    Double(&'static str, i32),
    Notification(&'static str),
    Selection,
}

/// The mapping table. `signed` is `visual.haptics`.
pub fn pattern(kind: Kind, signed: bool, kid: bool) -> Pattern {
    use Pattern::{Double, Impact, Notification};
    match (kind, signed) {
        (Kind::Correct, _) | (Kind::Key, _) | (Kind::Tap, _) | (Kind::Hop, _) => Impact("LIGHT"),
        (Kind::Wrong, true) => Double("LIGHT", 80),
        (Kind::Wrong, false) if kid => Impact("MEDIUM"),
        (Kind::Wrong, false) => Notification("ERROR"),
        (Kind::Milestone, _) => Impact("MEDIUM"),
        (Kind::Selection, true) => Pattern::Selection,
        (Kind::Selection, false) => Impact("LIGHT"),
    }
}

/// The per-word cap (F6.5).
pub const PER_WORD: u32 = 2;

/// Whether a kind counts toward the cap. Keystrokes are exempt (E-Q3); so are
/// Board Game hops, which follow the walk after the word, one per square.
pub fn capped(kind: Kind) -> bool {
    !matches!(kind, Kind::Key | Kind::Hop)
}

/// The pure decision: does this haptic fire? `count` is how many capped haptics
/// this word has had; `word_known` is whether a word is in play at all.
pub fn allow(kind: Kind, signed: bool, haptics_on: bool, key_feel: bool, word_known: bool, count: u32) -> bool {
    if !signed {
        return true;
    }
    match kind {
        Kind::Key => key_feel,
        _ => haptics_on && (!capped(kind) || !word_known || count < PER_WORD),
    }
}

thread_local! {
    static HAPTICS_ON: Cell<bool> = const { Cell::new(true) };
    static KEY_FEEL: Cell<bool> = const { Cell::new(true) };
    static KID: Cell<bool> = const { Cell::new(false) };
    static WORD: RefCell<Option<String>> = const { RefCell::new(None) };
    static COUNT: Cell<u32> = const { Cell::new(0) };
    static FIRED: Cell<u32> = const { Cell::new(0) };
    static LOGGED_NOOP: Cell<bool> = const { Cell::new(false) };
}

/// The two toggles and Kid Mode, from settings.
pub fn configure(haptics_on: bool, key_feel: bool, kid: bool) {
    HAPTICS_ON.with(|c| c.set(haptics_on));
    KEY_FEEL.with(|c| c.set(key_feel));
    KID.with(|c| c.set(kid));
}

/// A word came into play. The same word again (a replay) keeps its count.
pub fn word_started(key: &str) {
    let same = WORD.with(|w| w.borrow().as_deref() == Some(key));
    if !same {
        WORD.with(|w| *w.borrow_mut() = Some(key.to_string()));
        COUNT.with(|c| c.set(0));
    }
}

/// How many haptics actually fired this session (the seam and AT-17).
pub fn fired() -> u32 {
    FIRED.with(Cell::get)
}

/// Fire `kind`, through the table, the toggles and the cap.
pub fn fire(kind: Kind) {
    let signed = crate::flags::visual_haptics();
    let word_known = WORD.with(|w| w.borrow().is_some());
    let ok = allow(
        kind,
        signed,
        HAPTICS_ON.with(Cell::get),
        KEY_FEEL.with(Cell::get),
        word_known,
        COUNT.with(Cell::get),
    );
    if !ok {
        return;
    }
    if signed && capped(kind) {
        COUNT.with(|c| c.set(c.get() + 1));
    }
    FIRED.with(|c| c.set(c.get() + 1));
    let Some(h) = plugin() else {
        if signed && !LOGGED_NOOP.with(|c| c.replace(true)) {
            web_sys::console::info_1(&"haptics: no haptic engine here (web); every haptic is a no-op this session".into());
        }
        return;
    };
    match pattern(kind, signed, KID.with(Cell::get)) {
        Pattern::Impact(style) => call(&h, "impact", Some(("style", style))),
        Pattern::Notification(t) => call(&h, "notification", Some(("type", t))),
        Pattern::Selection => {
            call(&h, "selectionStart", None);
            call(&h, "selectionChanged", None);
            call(&h, "selectionEnd", None);
        }
        Pattern::Double(style, gap) => {
            call(&h, "impact", Some(("style", style)));
            let h2 = h.clone();
            let cb = wasm_bindgen::closure::Closure::once_into_js(move || call(&h2, "impact", Some(("style", style))));
            if let Some(win) = web_sys::window() {
                let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), gap);
            }
        }
    }
}

fn plugin() -> Option<JsValue> {
    let win = web_sys::window()?;
    let cap = Reflect::get(&win, &JsValue::from_str("Capacitor")).ok()?;
    if cap.is_undefined() || cap.is_null() {
        return None;
    }
    let plugins = Reflect::get(&cap, &JsValue::from_str("Plugins")).ok()?;
    let h = Reflect::get(&plugins, &JsValue::from_str("Haptics")).ok()?;
    if h.is_undefined() || h.is_null() {
        None
    } else {
        Some(h)
    }
}

/// Calls `Haptics.<method>({ <key>: <value> })`, ignoring the returned promise.
fn call(h: &JsValue, method: &str, arg: Option<(&str, &str)>) {
    let Ok(f) = Reflect::get(h, &JsValue::from_str(method)) else { return };
    let Ok(f) = f.dyn_into::<Function>() else { return };
    let opts = Object::new();
    if let Some((k, v)) = arg {
        let _ = Reflect::set(&opts, &JsValue::from_str(k), &JsValue::from_str(v));
    }
    let _ = f.call1(h, &opts);
}

// The names the call sites already use, kept so the meaning reads at the site.

/// Light tap confirming a correct spelling.
pub fn correct() {
    fire(Kind::Correct);
}

/// A keystroke on an on-screen keyboard (the Key feel class).
pub fn key_tap() {
    fire(Kind::Key);
}

/// Any other confirming tick (a duplicate, an accepted find, a tile landing).
pub fn tap() {
    fire(Kind::Tap);
}

/// A Board Game piece hop (its D-P6 mapping).
pub fn hop() {
    fire(Kind::Hop);
}

/// A wrong spelling. `kid` only matters with `visual.haptics` off, where Kid
/// Mode still softens the ERROR buzz to a medium tap.
pub fn incorrect(kid: bool) {
    KID.with(|c| c.set(kid));
    fire(Kind::Wrong);
}

/// A wrong spelling, with Kid Mode read from settings (`configure`). The three
/// modes that used to hard-code the kid flag (Randospell, Quick Bee Draw,
/// Spelluzzle; signing sheet V-D17) call this.
pub fn wrong() {
    fire(Kind::Wrong);
}

/// A milestone (F7 drives this).
#[allow(dead_code)]
pub fn milestone() {
    fire(Kind::Milestone);
}

/// A picker or scroll selection.
#[allow(dead_code)]
pub fn selection() {
    fire(Kind::Selection);
}

#[cfg(test)]
mod tests {
    use super::*;
    use Kind::*;

    #[test]
    fn off_the_table_is_todays() {
        assert_eq!(pattern(Correct, false, false), Pattern::Impact("LIGHT"));
        assert_eq!(pattern(Key, false, false), Pattern::Impact("LIGHT"));
        assert_eq!(pattern(Wrong, false, false), Pattern::Notification("ERROR"));
        assert_eq!(pattern(Wrong, false, true), Pattern::Impact("MEDIUM"), "Kid Mode softened the buzz");
    }

    #[test]
    fn on_the_table_is_v_d17() {
        assert_eq!(pattern(Correct, true, false), Pattern::Impact("LIGHT"));
        assert_eq!(pattern(Wrong, true, false), Pattern::Double("LIGHT", 80));
        assert_eq!(pattern(Wrong, true, true), Pattern::Double("LIGHT", 80), "one gentle pattern for everyone");
        assert_eq!(pattern(Milestone, true, false), Pattern::Impact("MEDIUM"));
        assert_eq!(pattern(Selection, true, false), Pattern::Selection);
        assert_eq!(pattern(Hop, true, false), Pattern::Impact("LIGHT"), "D-P6 kept");
    }

    #[test]
    fn off_nothing_is_capped_or_toggled() {
        for k in [Correct, Wrong, Tap, Key] {
            assert!(allow(k, false, false, false, true, 99));
        }
    }

    #[test]
    fn keystrokes_follow_key_feel_and_ignore_the_cap() {
        assert!(allow(Key, true, false, true, true, 99), "Haptics off does not silence keys");
        assert!(!allow(Key, true, true, false, true, 0), "Key feel off does");
    }

    #[test]
    fn haptics_off_silences_everything_but_keys() {
        for k in [Correct, Wrong, Tap, Milestone, Selection, Hop] {
            assert!(!allow(k, true, false, true, true, 0), "{k:?}");
        }
    }

    // AT-17: never more than 2 haptics per word in scripted play.
    #[test]
    fn a_scripted_word_never_gets_more_than_two() {
        // A word: keys, a wrong answer, a retry, a correct answer, a milestone.
        let script = [Key, Key, Key, Wrong, Key, Key, Correct, Milestone, Tap, Key];
        let mut count = 0;
        let mut fired_capped = 0;
        for k in script {
            if allow(k, true, true, true, true, count) && capped(k) {
                count += 1;
                fired_capped += 1;
            }
        }
        assert_eq!(fired_capped, PER_WORD);
    }

    #[test]
    fn a_surface_with_no_word_is_not_capped() {
        assert!(allow(Tap, true, true, true, false, 10));
    }
}
