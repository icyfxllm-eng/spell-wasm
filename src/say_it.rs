//! Feature F2 "Say It" — on-device pronunciation practice.
//!
//! Reverses the core loop: instead of hearing a word and typing it, the player
//! SEES the word and SAYS it aloud. An on-device speech recognizer transcribes
//! the utterance and we score correct/incorrect using [`crate::norm::spoken_matches`]
//! (exact token match after NFC + case fold — Decision D2). No fuzzy/phonetic/
//! confidence scoring, and this mode never touches solo/Climb/Daily/Versus
//! scoring, streaks, leaderboards, stats, or word lists — it keeps its own
//! ephemeral session tally.
//!
//! HARD invariants:
//! * **Ships dark.** The launcher is revealed and wired ONLY when
//!   [`crate::flags::say_it`] is on. Flag off = a true no-op (nothing shown,
//!   nothing wired, zero diff).
//! * **Disabled in Kid Mode, always** (COPPA D5). A hard gate independent of the
//!   flag and of any technical capability — [`is_offered`] returns false whenever
//!   Kid Mode is on.
//! * **On-device recognition ONLY.** Entry checks
//!   [`crate::native_lang::speech_capabilities`], whose `available` already
//!   requires `supportsOnDeviceRecognition`; if it isn't available the mode shows
//!   an UNAVAILABLE state and NEVER falls back to server recognition. A child's
//!   voice never leaves the phone.

use std::cell::{Cell, RefCell};

use wasm_bindgen_futures::spawn_local;

use crate::consts::{EN, ES, TH};
use crate::native_lang::{self, ListenOutcome};
use crate::App;

/// Correct/incorrect session tally. Deliberately tiny + pure so it is
/// host-unit-testable and provably independent of the app's real scoring.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tally {
    pub right: u32,
    pub wrong: u32,
}

impl Tally {
    pub fn record(&mut self, correct: bool) {
        if correct {
            self.right += 1;
        } else {
            self.wrong += 1;
        }
    }
}

/// The gate for whether the Say-It launcher is offered at all. Pure so the two
/// hard rules are unit-tested with no DOM: the feature flag must be ON **and**
/// Kid Mode must be OFF. (Native/on-device availability is checked separately —
/// synchronously via the bridge for visibility, and authoritatively per-language
/// on entry.)
pub fn is_offered(flag_on: bool, kid: bool) -> bool {
    flag_on && !kid
}

thread_local! {
    static TALLY: RefCell<Tally> = const { RefCell::new(Tally { right: 0, wrong: 0 }) };
    static WORD: RefCell<String> = const { RefCell::new(String::new()) };
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    /// True once the player has passed the pre-prompt this app-run, so re-opening
    /// the mode goes straight to play instead of re-showing the intro.
    static SESSION_READY: Cell<bool> = const { Cell::new(false) };
    /// True while a listen is in flight (mic tap toggles stop).
    static LISTENING: Cell<bool> = const { Cell::new(false) };
}

// ---- gating / wiring (called from lib.rs boot + settings apply) ----

/// Show or hide the launcher. Offered only when the flag is on, Kid Mode is off,
/// AND the native bridge is present (so on the web / non-iOS the button stays
/// hidden even with the flag flipped). Safe to call on every settings change.
pub fn reflect_gating(app: &App) {
    let kid = app.borrow().kid;
    let offered = is_offered(crate::flags::say_it(), kid) && native_lang::available();
    crate::dom::toggle_class("sayItBtn", "btn-hide", !offered);
}

/// Attach the mode's click handlers — ONLY when the flag is on. When the flag is
/// off this returns immediately, so nothing is wired and the mode is inert.
pub fn wire(app: &App) {
    if !crate::flags::say_it() {
        return;
    }
    let a = app.clone();
    crate::dom::on_click("sayItBtn", move || open(&a));
    let a = app.clone();
    crate::dom::on_click("sayItBegin", move || begin(&a));
    crate::dom::on_click("sayItCancel", || close());
    let a = app.clone();
    crate::dom::on_click("sayItMic", move || mic_tap(&a));
    let a = app.clone();
    crate::dom::on_click("sayItNext", move || next_word(&a));
    crate::dom::on_click("sayItExit", || exit());
    crate::dom::on_click("sayItUnavailableClose", || close());
    crate::dom::on_click("sayItNeedsMicClose", || close());
}

// ---- views ----

fn show_view(view: &str) {
    for v in ["sayItPre", "sayItPlay", "sayItUnavailable", "sayItNeedsMic"] {
        crate::dom::toggle_class(v, "btn-hide", v != view);
    }
}

/// The study language Say-It runs in. Only the active, on-device-capable
/// languages are reachable here; anything else surfaces as UNAVAILABLE on entry.
fn current_lang(app: &App) -> String {
    app.borrow().lang.clone()
}

/// Open the mode: verify on-device capability for the current language, then show
/// the pre-prompt (first use) or go straight to play.
pub fn open(app: &App) {
    // Belt-and-suspenders: never open in Kid Mode even if somehow wired.
    if app.borrow().kid {
        return;
    }
    let lang = current_lang(app);
    LANG.with(|l| *l.borrow_mut() = lang.clone());
    crate::dom::add_class("sayItScrim", "show");
    let app = app.clone();
    spawn_local(async move {
        let cap = native_lang::speech_capabilities(&lang).await;
        if !cap.available {
            show_view("sayItUnavailable");
            return;
        }
        if SESSION_READY.with(Cell::get) {
            show_view("sayItPlay");
            next_word(&app);
        } else {
            show_view("sayItPre");
        }
    });
}

/// Pre-prompt "Continue": enter play. The OS mic + speech permission prompts are
/// requested on the first actual mic tap (startListening), which the pre-prompt
/// has just explained.
pub fn begin(app: &App) {
    SESSION_READY.with(|c| c.set(true));
    show_view("sayItPlay");
    next_word(app);
}

fn set_status(key_or_text: &str, is_key: bool) {
    let text = if is_key { crate::i18n::t(key_or_text) } else { key_or_text.to_string() };
    crate::dom::set_text("sayItStatus", &text);
}

fn render_score() {
    TALLY.with(|t| {
        let t = t.borrow();
        crate::dom::set_text("sayItRight", &t.right.to_string());
        crate::dom::set_text("sayItWrong", &t.wrong.to_string());
    });
}

/// Pick the next word for the current language and reset the round UI.
pub fn next_word(app: &App) {
    let lang = current_lang(app);
    let word = pick_word(&lang);
    WORD.with(|w| *w.borrow_mut() = word.clone());
    crate::dom::set_text("sayItWord", &word);
    crate::dom::el("sayItWord").set_class_name("sayit-word");
    set_status("sayit.tapMic", true);
    crate::dom::set_disabled("sayItNext", true);
    crate::dom::remove_class("sayItMic", "listening");
    LISTENING.with(|c| c.set(false));
    render_score();
}

/// Mic button: start on-device listening, or stop/finalize if already listening.
pub fn mic_tap(app: &App) {
    if LISTENING.with(Cell::get) {
        native_lang::stop_listening();
        return;
    }
    let word = WORD.with(|w| w.borrow().clone());
    if word.is_empty() {
        return;
    }
    let lang = LANG.with(|l| l.borrow().clone());
    LISTENING.with(|c| c.set(true));
    crate::dom::add_class("sayItMic", "listening");
    set_status("sayit.listening", true);
    let app = app.clone();
    spawn_local(async move {
        let outcome = native_lang::start_listening_await(&lang).await;
        LISTENING.with(|c| c.set(false));
        crate::dom::remove_class("sayItMic", "listening");
        match outcome {
            ListenOutcome::Heard(transcript) => {
                let correct = crate::norm::spoken_matches(&transcript, &word);
                TALLY.with(|t| t.borrow_mut().record(correct));
                render_score();
                if correct {
                    crate::dom::el("sayItWord").set_class_name("sayit-word correct");
                    set_status("sayit.correct", true);
                } else {
                    crate::dom::el("sayItWord").set_class_name("sayit-word wrong");
                    set_status("sayit.tryAgain", true);
                }
                crate::dom::set_disabled("sayItNext", false);
                let _ = &app;
            }
            ListenOutcome::Error(code) => match code.as_str() {
                // Denied permission is a STATE, not an error screen shouting failure.
                "PERMISSION_DENIED" => show_view("sayItNeedsMic"),
                "UNAVAILABLE" => show_view("sayItUnavailable"),
                // NO_SPEECH / AUDIO_ERROR / BUSY: gentle retry, stay on the word.
                _ => {
                    set_status("sayit.didntCatch", true);
                    crate::dom::set_disabled("sayItNext", false);
                }
            },
        }
    });
}

/// Exit the mode (from within play): stop any capture and close.
pub fn exit() {
    native_lang::stop_listening();
    close();
}

/// Close the overlay and reset per-open transient UI state (keeps the running
/// session tally so re-opening resumes the count).
pub fn close() {
    native_lang::stop_listening();
    LISTENING.with(|c| c.set(false));
    crate::dom::remove_class("sayItScrim", "show");
}

/// Draw a word for the given active language. Say-It is only reachable for active,
/// on-device-capable languages; for anything unexpected we fall back to English so
/// the UI never shows an empty word (entry gating already prevents that path).
fn pick_word(lang: &str) -> String {
    let pool = say_it_pool(lang);
    if pool.is_empty() {
        return "word".to_string();
    }
    let idx = (js_sys::Math::random() * pool.len() as f64).floor() as usize % pool.len();
    // Mandarin stores "pinyin|hanzi"; show the hanzi (what you'd say). Other
    // languages are stored plain. Say-It's active set is en/th today, so this is
    // just defensive.
    let raw = pool[idx];
    raw.split_once('|').map(|(_, spoken)| spoken).unwrap_or(raw).to_string()
}

/// The word pool Say-It draws from: easy + medium words of the active language,
/// so the player says everyday words rather than expert spelling-bee terms.
fn say_it_pool(lang: &str) -> Vec<&'static str> {
    let lang = match lang {
        EN | ES | TH => lang,
        _ => EN,
    };
    let mut pool: Vec<&'static str> = Vec::new();
    for tier in ["easy", "medium"] {
        pool.extend_from_slice(crate::words::tier_for(lang, tier));
    }
    pool
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_off_means_mode_absent() {
        // The whole feature is gated on the flag: off -> never offered, whatever
        // Kid Mode says.
        assert!(!is_offered(false, false));
        assert!(!is_offered(false, true));
    }

    #[test]
    fn kid_mode_hard_disables_even_with_flag_on() {
        // COPPA D5: a hard gate. Flag on but Kid Mode on -> not offered.
        assert!(!is_offered(true, true));
    }

    #[test]
    fn offered_only_when_flag_on_and_not_kid() {
        assert!(is_offered(true, false));
    }

    #[test]
    fn tally_counts_correct_and_incorrect() {
        let mut t = Tally::default();
        t.record(true);
        t.record(true);
        t.record(false);
        assert_eq!(t, Tally { right: 2, wrong: 1 });
    }
}
