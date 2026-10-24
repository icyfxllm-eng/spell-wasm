//! Dev door (TEMPORARY — removed at activation of these modes).
//!
//! Five quick taps on the SPELL logo open a tiny menu to the still-hidden,
//! REVIEW-GATED screen (Spell Racing) so Eric can test it on-device.
//! Invisible to normal testers: no visible entry, no hub tile, and it takes a
//! deliberate five-tap burst on one element to reveal.
//!
//! This is the ONLY place that routes to those screens' hidden entries; the screens
//! themselves stay untouched. Delete this module (and the `#devMenu` DOM) when the
//! modes are activated.

use std::cell::RefCell;

use wasm_bindgen::JsCast;

use crate::{dom, App};

thread_local! {
    /// `(consecutive_taps, last_tap_ms)` for the logo tap-burst detector.
    static BRAND_TAPS: RefCell<(u32, f64)> = const { RefCell::new((0, 0.0)) };
}

/// Taps needed, and the max gap (ms) between them, to open the dev menu.
const TAPS_NEEDED: u32 = 5;
const TAP_GAP_MS: f64 = 1200.0;

/// Record one logo tap at `now_ms`; return true on the `TAPS_NEEDED`-th tap within
/// `TAP_GAP_MS` of each prior one (then reset). Too long a gap restarts the count.
/// Pure over `BRAND_TAPS` so it is unit-testable.
fn register_tap(now_ms: f64) -> bool {
    BRAND_TAPS.with(|c| {
        let (count, last) = *c.borrow();
        let count = if now_ms - last <= TAP_GAP_MS { count + 1 } else { 1 };
        if count >= TAPS_NEEDED {
            *c.borrow_mut() = (0, now_ms); // consume; next tap starts fresh
            true
        } else {
            *c.borrow_mut() = (count, now_ms);
            false
        }
    })
}

fn open_menu() {
    dom::add_class("devMenu", "show");
}

fn close_menu() {
    dom::remove_class("devMenu", "show");
}

/// Wire the dev door once at startup.
pub fn wire(app: &App) {
    dom::on_click("brandMark", move || {
        if register_tap(js_sys::Date::now()) {
            open_menu();
        }
    });
    dom::on_click("devMenuClose", close_menu);

    let a_race = app.clone();
    dom::on_click("devOpenRacing", move || {
        close_menu();
        crate::racing::screen::open(&a_race);
    });

    // CC-CJK-INK F1: the ink gate is dev-only until real handwriting has been
    // measured, which is the whole point of it.
    let a_ink = app.clone();
    dom::on_click("devOpenInk", move || {
        close_menu();
        crate::ink_probe::open(&a_ink);
    });

    // Arabic shortcut: jump straight into the freshly ungated language without
    // scrolling the picker. Sets langSel and fires its own change event, so the
    // ONE existing language-switch handler does all the work (RTL dir, keyboard,
    // prefs, round reset) — no duplicated switching logic here.
    dom::on_click("devOpenArabic", || {
        close_menu();
        dom::select("langSel").set_value("ar");
        if let Ok(ev) = web_sys::Event::new("change") {
            let _ = dom::el("langSel").dispatch_event(&ev);
        }
    });

    // "Test entitlements": resolve the audit maximum (Complete) on this device
    // so premium-gated surfaces (photo camera, its hub tile) stay testable
    // before the purchase adapters exist. Storage-backed; read by
    // `play_hub::live_entitlements`. Consumer-build only by the resolver's own
    // rule (education ignores audit overrides).
    dom::input("devEntitlements")
        .set_checked(crate::storage::get_raw("spell_dev_entitlements").as_deref() == Some("on"));
    let a_ent = app.clone();
    dom::on::<web_sys::Event, _>("devEntitlements", "change", move |_| {
        let on = dom::input("devEntitlements").checked();
        crate::storage::set_raw("spell_dev_entitlements", if on { "on" } else { "off" });
        // Re-reflect the surfaces that consult entitlements live.
        crate::photo_list::reflect_visibility(&a_ent);
    });
    // CC-ECLIPSE-HOTFIX F4: the mic readout's switch, default off. Release builds
    // cut both the switch and the readout (scripts/cut-diag.py) and carry none of
    // this code, so not even its storage key is in a player's binary (I7).
    #[cfg(feature = "dev_preview")]
    if dom::exists("devDiagBar") {
        dom::input("devDiagBar").set_checked(crate::storage::get_raw("spell_diag_bar").as_deref() == Some("on"));
        dom::on::<web_sys::Event, _>("devDiagBar", "change", |_| {
            let on = dom::input("devDiagBar").checked();
            crate::storage::set_raw("spell_diag_bar", if on { "on" } else { "off" });
        });
    }
    // CC-VISUAL-ECLIPSE A5: the AT-19 frame logger. The switch persists, so a
    // measurement started here survives a relaunch; restore() resumes it.
    crate::frame_log::restore();
    dom::input("devFrameLog").set_checked(crate::frame_log::is_on());
    dom::on::<web_sys::Event, _>("devFrameLog", "change", |_| {
        crate::frame_log::set_on(dom::input("devFrameLog").checked());
    });
    let show_report = || {
        dom::set_text("devFrameReportText", &crate::frame_log::report());
        dom::set_hidden("devFrameReportBox", false);
    };
    dom::on_click("devFrameReport", show_report);
    dom::on_click("devFrameReset", move || {
        crate::frame_log::clear();
        show_report();
    });
    // Copy through the async clipboard where WebKit allows it; the report is
    // also selectable text, so a long-press copy always works.
    dom::on_click("devFrameCopy", || {
        let text = crate::frame_log::report();
        let ok = web_sys::window()
            .and_then(|w| js_sys::Reflect::get(&w.navigator(), &"clipboard".into()).ok())
            .filter(|c| !c.is_undefined())
            .and_then(|c| {
                let f = js_sys::Reflect::get(&c, &"writeText".into()).ok()?.dyn_into::<js_sys::Function>().ok()?;
                f.call1(&c, &text.into()).ok()
            })
            .is_some();
        dom::set_text("devFrameCopy", if ok { "Copied" } else { "Select the text to copy" });
    });

    // Spell Aloud is no longer a dev-door overlay: it's a real Play-hub mode
    // (CC-SPELL-ALOUD-INTEGRATION G-INT-1). Only Spell Racing remains dev-gated.

    // Tapping the scrim backdrop closes the menu.
    dom::on::<web_sys::Event, _>("devMenu", "click", |e| {
        if dom::is_self_target(&e, "devMenu") {
            close_menu();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_menu_opens_only_on_fast_five_taps() {
        BRAND_TAPS.with(|c| *c.borrow_mut() = (0, 0.0));
        for t in [0.0, 200.0, 400.0, 600.0] {
            assert!(!register_tap(t), "tap at {t} should not open");
        }
        assert!(register_tap(800.0), "fifth in-window tap opens");
        assert!(!register_tap(900.0), "consumed — a lone tap after does nothing");

        // a slow sequence never reaches five (each gap too long → resets to 1)
        BRAND_TAPS.with(|c| *c.borrow_mut() = (0, 0.0));
        for t in [0.0, 2000.0, 4000.0, 6000.0, 8000.0] {
            assert!(!register_tap(t), "slow tap at {t} keeps resetting");
        }
    }
}
