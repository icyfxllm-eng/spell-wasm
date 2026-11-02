//! CC-ACCOUNTS Phase 2 -- the account sheet and Settings → Account.
//!
//! The sheet opens in exactly one place: the moment a signed-out player taps an ONLINE entry while
//! `accounts_online_gate` is on (F1). Nothing asks for an account at launch or in solo play, and
//! with the flags off this file does nothing at all: `gate_then` runs its callback at once and
//! Settings shows no account row (I1, I12).

use std::cell::{Cell, RefCell};

use wasm_bindgen_futures::spawn_local;

use crate::accounts_model::{self as am, Event, Problem, Step};
use crate::accounts_net as net;
use crate::{agegate, dom, i18n, storage};

const PANELS: [&str; 7] = ["acnAge", "acnUnder", "acnMethod", "acnEmail", "acnCode", "acnWorking", "acnErr"];

thread_local! {
    static STEP: RefCell<Step> = const { RefCell::new(Step::Closed) };
    /// What the player was doing when the gate stopped them; runs once, after a sign-in.
    static PENDING: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
    /// An under-18 answer holds for the rest of this launch, so changing the birthday and tapping again does nothing. Memory only.
    static UNDER_AGE: Cell<bool> = const { Cell::new(false) };
    static EMAIL: RefCell<String> = const { RefCell::new(String::new()) };
}

fn gate_on() -> bool {
    crate::flags::accounts_enabled() && crate::flags::accounts_online_gate()
}

/// F1: run `then` now if there is nothing to gate or the player already has a session; otherwise
/// open the sheet and run it after sign-in. "Not now" drops it.
pub fn gate_then(then: Box<dyn FnOnce()>) {
    if !gate_on() {
        then();
        return;
    }
    spawn_local(async move {
        if net::signed_in().await {
            then();
        } else {
            PENDING.with(|p| *p.borrow_mut() = Some(then));
            open();
        }
    });
}

fn open() {
    agegate::populate_into("acnMonth", "acnDay", "acnYear");
    dispatch(Event::Open);
    if UNDER_AGE.with(Cell::get) {
        dispatch(Event::AgeAnswered { age: 0 });
    }
}

fn dispatch(ev: Event) {
    let step = STEP.with(|s| {
        let next = am::next(&s.borrow(), &ev);
        *s.borrow_mut() = next.clone();
        next
    });
    render(&step);
}

fn render(step: &Step) {
    let shown = match step {
        Step::Closed => None,
        Step::Age => Some("acnAge"),
        Step::UnderAge => Some("acnUnder"),
        Step::Method => Some("acnMethod"),
        Step::Email => Some("acnEmail"),
        Step::Code => Some("acnCode"),
        Step::Working => Some("acnWorking"),
        Step::Failed(p) => {
            dom::set_text(
                "acnErrText",
                &i18n::t(match p {
                    Problem::Offline => "acn.err.offline",
                    Problem::BadCode => "acn.err.code",
                    Problem::Rejected => "acn.err.rejected",
                    Problem::Unavailable => "acn.err.unavailable",
                }),
            );
            Some("acnErr")
        }
    };
    for id in PANELS {
        dom::set_hidden(id, Some(id) != shown);
    }
    if shown.is_some() {
        dom::add_class("acnScrim", "show");
    } else {
        dom::remove_class("acnScrim", "show");
    }
}

fn close(signed_in: bool) {
    dispatch(if signed_in { Event::SignedIn } else { Event::Close });
    let then = PENDING.with(|p| p.borrow_mut().take());
    if signed_in {
        if let Some(f) = then {
            f();
        }
    }
}

fn succeeded(s: am::Session) {
    // F2: mark this device's progress "adopt me". Nothing is copied, moved or deleted here.
    let prev = storage::get_raw(am::LOCAL_KEYS[0]);
    if let Some(m) = am::adoption_marker(prev.as_deref(), s.user_id) {
        storage::set_raw(am::LOCAL_KEYS[1], &m);
        storage::set_raw(am::LOCAL_KEYS[0], &m);
    }
    crate::sync_store::kick();
    close(true);
}

fn failed(p: Problem) {
    dispatch(Event::Fail(p));
}

fn work<F: std::future::Future<Output = Result<am::Session, Problem>> + 'static>(f: F) {
    dispatch(if STEP.with(|s| *s.borrow() == Step::Method) { Event::PickApple } else { Event::Submit });
    spawn_local(async move {
        let result = f.await;
        // A sheet dismissed while this was in flight stays dismissed.
        if STEP.with(|s| *s.borrow() != Step::Working) {
            return;
        }
        match result {
            Ok(s) => succeeded(s),
            Err(p) => failed(p),
        }
    });
}

pub fn setup() {
    wire_sheet();
    wire_settings();
}

fn wire_sheet() {
    dom::on_click("acnAgeGo", || {
        let (y, m, d) = agegate::read_selection_from("acnMonth", "acnDay", "acnYear");
        if !agegate::valid_date(y, m, d) {
            dom::set_text("acnAgeErr", &i18n::t("age.badDate"));
            return;
        }
        dom::set_text("acnAgeErr", "");
        let age = agegate::age_from(y, m, d);
        if !am::is_adult(age) {
            UNDER_AGE.with(|u| u.set(true));
        }
        dispatch(Event::AgeAnswered { age });
        spawn_local(async { dom::set_hidden("acnApple", !net::siwa_available().await) });
    });
    for id in ["acnNotNow", "acnNotNow2", "acnUnderClose", "acnErrClose"] {
        dom::on_click(id, || close(false));
    }
    dom::on_click("acnApple", || work(net::sign_in_apple()));
    dom::on_click("acnEmailBtn", || dispatch(Event::PickEmail));
    dom::on_click("acnEmailBack", || dispatch(Event::Back));
    dom::on_click("acnCodeBack", || dispatch(Event::Back));
    dom::on_click("acnErrBack", || dispatch(Event::Back));
    dom::on_click("acnSend", || {
        let email = dom::input("acnEmailIn").value();
        if !am::plausible_email(&email) {
            dom::set_text("acnEmailErr", &i18n::t("acn.err.email"));
            return;
        }
        dom::set_text("acnEmailErr", "");
        EMAIL.with(|e| *e.borrow_mut() = email.trim().to_string());
        spawn_local(async move {
            let e = EMAIL.with(|e| e.borrow().clone());
            match net::request_code(&e).await {
                Ok(()) => dispatch(Event::CodeSent),
                Err(p) => failed(p),
            }
        });
    });
    dom::on_click("acnVerify", || {
        let Some(code) = am::clean_code(&dom::input("acnCodeIn").value()) else {
            dom::set_text("acnCodeErr", &i18n::t("acn.err.code"));
            return;
        };
        dom::set_text("acnCodeErr", "");
        let email = EMAIL.with(|e| e.borrow().clone());
        work(async move { net::verify_code(&email, &code).await });
    });
}

fn wire_settings() {
    let reflect = || {
        let on = crate::flags::accounts_enabled();
        dom::set_hidden("acnSetRow", !on);
        if !on {
            return;
        }
        spawn_local(async {
            let signed = net::signed_in().await;
            dom::set_text("acnSetState", &i18n::t(if signed { "acn.set.in" } else { "acn.set.out" }));
            dom::set_hidden("acnSignIn", signed);
            dom::set_hidden("acnSignOut", !signed);
            dom::set_hidden("acnDelete", !signed);
        });
    };
    reflect();
    dom::on_click("setBtn", reflect);
    dom::on_click("acnSignIn", || {
        PENDING.with(|p| *p.borrow_mut() = None);
        open();
    });
    dom::on_click("acnSignOut", move || {
        spawn_local(async {
            net::sign_out().await;
            dom::set_text("acnSetState", &i18n::t("acn.set.out"));
            dom::set_hidden("acnSignIn", false);
            dom::set_hidden("acnSignOut", true);
            dom::set_hidden("acnDelete", true);
        });
    });
    dom::on_click("acnDelete", || {
        dom::set_text("acnDeleteErr", "");
        dom::add_class("acnDeleteScrim", "show");
    });
    dom::on_click("acnDeleteCancel", || dom::remove_class("acnDeleteScrim", "show"));
    dom::on_click("acnDeleteConfirm", || {
        spawn_local(async {
            match net::delete_account().await {
                Ok(()) => {
                    dom::remove_class("acnDeleteScrim", "show");
                    dom::set_text("acnSetState", &i18n::t("acn.set.out"));
                    dom::set_hidden("acnSignIn", false);
                    dom::set_hidden("acnSignOut", true);
                    dom::set_hidden("acnDelete", true);
                }
                Err(_) => dom::set_text("acnDeleteErr", &i18n::t("acn.delete.failed")),
            }
        });
    });
}
