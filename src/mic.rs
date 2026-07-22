use std::cell::{Cell, RefCell};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsValue;

use crate::speech_in::{self, SpeechRec};
use crate::{dom, game};
use crate::App;

thread_local! {
    static REC: RefCell<Option<SpeechRec>> = RefCell::new(None);
    static LISTENING: Cell<bool> = Cell::new(false);
}

pub fn setup(app: &App) {
    if !speech_in::is_supported() {
        dom::el("micBtn").set_attribute("hidden", "").ok();
        return;
    }
    dom::el("micBtn").remove_attribute("hidden").ok();
    let Some(rec) = SpeechRec::new() else { return };
    rec.set_max_alternatives(1);
    rec.set_interim_results(false);

    let app_result = app.clone();
    let onresult = Closure::<dyn FnMut(JsValue)>::new(move |ev: JsValue| {
        let raw = speech_in::extract_transcript(&ev).unwrap_or_default();
        let parsed = speech_in::parse_spoken(&raw);
        dom::input("guess").set_value(&parsed);
        game::render_letters(&app_result, true);
        dom::set_text("feedback", &format!("heard: \u{201c}{}\u{201d} \u{2014} edit if needed, then check", raw.trim()));
        dom::el("feedback").set_class_name("feedback");
    });
    rec.set_onresult(&onresult);
    onresult.forget();

    let onend = Closure::<dyn FnMut()>::new(|| {
        LISTENING.with(|l| l.set(false));
        dom::remove_class("micBtn", "listening");
    });
    rec.set_onend(&onend);
    onend.forget();

    let onerror = Closure::<dyn FnMut()>::new(|| {
        LISTENING.with(|l| l.set(false));
        dom::remove_class("micBtn", "listening");
    });
    rec.set_onerror(&onerror);
    onerror.forget();

    REC.with(|r| *r.borrow_mut() = Some(rec));
}

pub fn toggle(app: &App) {
    let (answered, active) = {
        let s = app.borrow();
        (s.answered, game::has_active_word(&s))
    };
    if answered || !active {
        return;
    }
    let listening = LISTENING.with(|l| l.get());
    REC.with(|r| {
        let borrow = r.borrow();
        let Some(rec) = borrow.as_ref() else { return };
        if listening {
            rec.stop();
            return;
        }
        let code = game::code_for(&app.borrow(), &app.borrow().cur_lang);
        rec.set_lang(&code);
        match rec.start() {
            Ok(_) => {
                LISTENING.with(|l| l.set(true));
                dom::add_class("micBtn", "listening");
            }
            Err(_) => {
                LISTENING.with(|l| l.set(false));
            }
        }
    });
}
