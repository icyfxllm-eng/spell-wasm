//! CC-LAUNCH-EN F4 / D11: the paywall.
//!
//! When a standard player opens a locked tier or mode, the adult gate opens
//! first and the paywall follows it. A Little Speller profile never reaches the
//! paywall (D24), and an education build compiles it to nothing (CC-EDITIONS
//! D4). The sheet offers what [`offers`] says, priced only by StoreKit (I4.7),
//! and Restore Purchases is always on it (C5.9). Off iOS it says the unlocks are
//! in the iPhone app (D7) and sells nothing.
//!
//! The sheet decides nothing about access. A purchase or restore updates the
//! owned set in `entitlements_adapter`, and the screens re-ask the core.

use crate::dom;
use crate::entitlements::Owned;
use crate::App;

/// The App Store listing, for the off-iOS "unlocks are in the app" line.
pub(crate) const APP_STORE_URL: &str = "https://apps.apple.com/app/id6788927897";

/// Which products the sheet offers, in display order. English is offered until
/// English or Complete is owned. The Complete row is whichever of complete and
/// complete_upgrade [`crate::entitlements::complete_offer`] allows (D5), so
/// nobody sees both. A product App Store Connect does not sell (Complete before
/// O4 is decided) never comes back from StoreKit, and so never renders.
#[cfg(not(feature = "education"))]
pub fn offers(o: Owned) -> Vec<&'static str> {
    offers_with(o, crate::entitlements::COMPLETE_ON_SALE)
}

/// [`offers`] with the sale switch passed in, so tests cover both values.
/// With `complete_on_sale` false the sheet offers English or nothing.
#[cfg(not(feature = "education"))]
fn offers_with(o: Owned, complete_on_sale: bool) -> Vec<&'static str> {
    let mut v = Vec::new();
    if !o.english && !o.complete {
        v.push(crate::entitlements::ENGLISH_PRODUCT_ID);
    }
    if complete_on_sale {
        if let Some(c) = crate::entitlements::complete_offer(o) {
            v.push(c);
        }
    }
    v
}

/// Open the paywall for a standard player: adult gate first, then the sheet.
pub fn open(app: &App) {
    if app.borrow().kid {
        return; // D24: Little Speller is free at every difficulty; nothing is locked.
    }
    #[cfg(not(feature = "education"))]
    crate::parent_gate_then(Box::new(show));
}

#[cfg(not(feature = "education"))]
fn show() {
    dom::set_text("paywallStatus", "");
    dom::add_class("paywallScrim", "show");
    if !crate::entitlements_adapter::store_available() {
        let html = format!(
            "<p class=\"pw-note\">{}</p><a class=\"btn btn-check\" href=\"{APP_STORE_URL}\" target=\"_blank\" rel=\"noopener\">SpellGame</a>",
            dom::escape_html(&crate::i18n::t("paywall.getApp"))
        );
        dom::set_html("paywallProducts", &html);
        return;
    }
    load_products();
}

#[cfg(not(feature = "education"))]
fn load_products() {
    dom::set_html("paywallProducts", "");
    wasm_bindgen_futures::spawn_local(async {
        match crate::entitlements_adapter::products().await {
            Ok(products) => render(&products),
            Err(()) => render_price_error(),
        }
    });
}

/// A4.8: no price means a clear retry, never a blank row or "$0.00".
#[cfg(not(feature = "education"))]
fn render_price_error() {
    let html = format!(
        "<p class=\"pw-note\">{}</p><button class=\"btn btn-icon\" data-pw-retry=\"1\">{}</button>",
        dom::escape_html(&crate::i18n::t("paywall.priceError")),
        dom::escape_html(&crate::i18n::t("paywall.retry")),
    );
    dom::set_html("paywallProducts", &html);
}

#[cfg(not(feature = "education"))]
fn render(products: &[crate::entitlements_adapter::StoreProduct]) {
    let want = offers(crate::entitlements_adapter::owned());
    let mut html = String::new();
    for id in &want {
        let Some(p) = products.iter().find(|p| p.id == *id) else { continue };
        let label = crate::i18n::tp("paywall.buy", &[("price", &p.display_price)]);
        html.push_str(&format!(
            "<button class=\"btn btn-check\" data-pw-buy=\"{}\"><strong>{}</strong><span>{}</span></button>",
            dom::escape_html(&p.id),
            dom::escape_html(&p.display_name),
            dom::escape_html(&label),
        ));
        if *id != crate::entitlements::ENGLISH_PRODUCT_ID {
            // D3: the one forward-looking sentence, on the Complete row only.
            html.push_str(&format!(
                "<p class=\"pw-note\">{}</p>",
                dom::escape_html(&crate::i18n::t("complete.includesFutureLanguages"))
            ));
        }
    }
    if want.is_empty() {
        // Everything on sale is owned: no rows, Restore stays.
        html = format!("<p class=\"pw-note\">{}</p>", dom::escape_html(&crate::i18n::t("paywall.allOwned")));
    } else if html.is_empty() {
        render_price_error();
        return;
    }
    dom::set_html("paywallProducts", &html);
}

fn close() {
    dom::remove_class("paywallScrim", "show");
}

/// Wire the sheet's buttons once at boot.
pub fn wire(app: &App) {
    dom::on_click("paywallClose", close);
    #[cfg(not(feature = "education"))]
    {
        let a = app.clone();
        dom::on_click("paywallRestore", move || restore(&a));
        // D5: the Settings "Purchases" row is the standing route to the sheet,
        // so an English owner can reach the upgrade and anyone can Restore.
        // Only where purchases can happen; a kid profile never sees it (CSS
        // body.kid hides it, and open() refuses a kid anyway, D24).
        dom::toggle_class("purchasesRow", "btn-hide", !crate::entitlements_adapter::store_available());
        let a = app.clone();
        dom::on_click("purchasesBtn", move || open(&a));
        let a = app.clone();
        dom::on::<web_sys::MouseEvent, _>("paywallProducts", "click", move |ev| {
            use wasm_bindgen::JsCast;
            let Some(t) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
            let Ok(Some(btn)) = t.closest("[data-pw-buy],[data-pw-retry]") else { return };
            if btn.has_attribute("data-pw-retry") {
                load_products();
            } else if let Some(id) = btn.get_attribute("data-pw-buy") {
                buy(&a, id);
            }
        });
    }
    #[cfg(feature = "education")]
    let _ = app;
}

#[cfg(not(feature = "education"))]
fn buy(app: &App, id: String) {
    use crate::entitlements_adapter::PurchaseOutcome;
    let a = app.clone();
    dom::set_text("paywallStatus", "");
    wasm_bindgen_futures::spawn_local(async move {
        match crate::entitlements_adapter::purchase(&id).await {
            PurchaseOutcome::Success => unlocked(&a),
            PurchaseOutcome::Pending => dom::set_text("paywallStatus", &crate::i18n::t("paywall.pending")),
            PurchaseOutcome::Cancelled => {}
            PurchaseOutcome::Failed | PurchaseOutcome::Unavailable => {
                dom::set_text("paywallStatus", &crate::i18n::t("paywall.failed"))
            }
        }
    });
}

#[cfg(not(feature = "education"))]
fn restore(app: &App) {
    let a = app.clone();
    dom::set_text("paywallStatus", "");
    wasm_bindgen_futures::spawn_local(async move {
        if !crate::entitlements_adapter::restore().await {
            dom::set_text("paywallStatus", &crate::i18n::t("paywall.failed"));
            return;
        }
        // I4.5: the restored set is exactly what StoreKit reports.
        if crate::entitlements_adapter::owned() == Owned::NONE {
            dom::set_text("paywallStatus", &crate::i18n::t("paywall.restoreNone"));
        } else {
            unlocked(&a);
        }
    });
}

/// After a purchase or restore: say so, and redraw what the core now opens.
#[cfg(not(feature = "education"))]
fn unlocked(app: &App) {
    dom::set_text("paywallStatus", &crate::i18n::t("paywall.done"));
    refresh_after_change(app);
    close();
}

/// Redraw every surface that shows locks. Also called when StoreKit pushes a
/// change (Ask to Buy approved, a refund), so nothing waits for a relaunch.
pub fn refresh_after_change(app: &App) {
    crate::game::build_level_options(app);
}

#[cfg(all(test, not(feature = "education")))]
mod tests {
    use super::*;
    use crate::entitlements::{COMPLETE_PRODUCT_ID, COMPLETE_UPGRADE_PRODUCT_ID, ENGLISH_PRODUCT_ID};

    /// D5 / I4.3 on the sheet: English plus full Complete for a new player,
    /// only the upgrade for an English owner, nothing once Complete is owned.
    const ALL_OWNED: [Owned; 4] =
        [Owned::NONE, Owned::ENGLISH, Owned::COMPLETE, Owned { english: true, complete: true }];

    /// D5 / I4.3, for when Complete goes on sale: English plus full Complete for
    /// a new player, only the upgrade for an English owner, nothing once
    /// Complete is owned.
    #[test]
    fn sheet_offers_with_complete_on_sale() {
        assert_eq!(offers_with(Owned::NONE, true), vec![ENGLISH_PRODUCT_ID, COMPLETE_PRODUCT_ID]);
        assert_eq!(offers_with(Owned::ENGLISH, true), vec![COMPLETE_UPGRADE_PRODUCT_ID]);
        assert!(offers_with(Owned::COMPLETE, true).is_empty());
        for o in ALL_OWNED {
            let v = offers_with(o, true);
            assert!(!(v.contains(&COMPLETE_PRODUCT_ID) && v.contains(&COMPLETE_UPGRADE_PRODUCT_ID)), "never both");
        }
    }

    /// Eric 2026-11-08: the release profile never offers Complete or the
    /// upgrade. The sheet is English and Restore, or "everything on sale is
    /// yours" plus Restore.
    #[test]
    fn release_profile_never_offers_complete_or_upgrade() {
        assert!(!crate::entitlements::COMPLETE_ON_SALE, "Complete is held from sale in this release");
        for o in ALL_OWNED {
            let v = offers(o);
            assert!(!v.contains(&COMPLETE_PRODUCT_ID), "{o:?} offered Complete");
            assert!(!v.contains(&COMPLETE_UPGRADE_PRODUCT_ID), "{o:?} offered the upgrade");
        }
        assert_eq!(offers(Owned::NONE), vec![ENGLISH_PRODUCT_ID]);
        assert!(offers(Owned::ENGLISH).is_empty(), "an English owner is offered nothing (Settings > Purchases: all owned)");
    }
}
