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
