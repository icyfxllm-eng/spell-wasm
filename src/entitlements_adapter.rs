//! CC-LAUNCH-EN F4: the purchase adapter, the Rust side of `window.SpellStore`
//! (store-native.js, wrapping ios/App/App/StoreKit/SpellStorePlugin.swift).
//!
//! It gathers one signal, which product ids StoreKit says are owned, and turns
//! it into [`Owned`] through the core's [`owned_from_product_ids`]. It decides
//! nothing. That is the job of [`crate::entitlements::resolve_entitlements`].
//!
//! The owned set comes from StoreKit and from nowhere else (I4.4). No storage
//! key, URL parameter, flag or remote file feeds it. Until StoreKit answers on
//! launch, the player owns nothing. A refund disappears on the next launch,
//! because the set is re-read from `Transaction.currentEntitlements` and never
//! persisted (I4.6). Education builds never touch the bridge (CC-EDITIONS D4).

use std::cell::Cell;

use crate::entitlements::Owned;
#[cfg(not(feature = "education"))]
use crate::entitlements::{owned_from_product_ids, PRODUCT_IDS};

thread_local! {
    static OWNED: Cell<Owned> = const { Cell::new(Owned::NONE) };
}

/// What the player owns right now, as last reported by StoreKit.
pub fn owned() -> Owned {
    OWNED.with(|c| c.get())
}

/// The event the page receives whenever the owned set changes, so an open
/// screen can redraw its locks without a relaunch (A4.5).
pub const CHANGED_EVENT: &str = "spell-entitlements-changed";

#[cfg(not(feature = "education"))]
fn set_from_js(ids: &wasm_bindgen::JsValue) {
    let list: Vec<String> = js_sys::Array::from(ids)
        .iter()
        .filter_map(|v| v.as_string())
        .collect();
    let refs: Vec<&str> = list.iter().map(String::as_str).collect();
    let next = owned_from_product_ids(&refs);
    let prev = OWNED.with(|c| c.replace(next));
    if prev != next {
        if let Some(w) = web_sys::window() {
            if let Ok(ev) = web_sys::Event::new(CHANGED_EVENT) {
                let _ = w.dispatch_event(&ev);
            }
        }
    }
}

#[cfg(not(feature = "education"))]
mod bridge {
    use js_sys::{Function, Promise, Reflect};
    use wasm_bindgen::{JsCast, JsValue};

    pub fn obj() -> Option<JsValue> {
        let w = web_sys::window()?;
        let o = Reflect::get(&w, &JsValue::from_str("SpellStore")).ok()?;
        (!o.is_undefined() && !o.is_null()).then_some(o)
    }

    pub fn call(name: &str, args: &[JsValue]) -> Option<Promise> {
        let o = obj()?;
        let f = Reflect::get(&o, &JsValue::from_str(name)).ok()?.dyn_into::<Function>().ok()?;
        let arr = js_sys::Array::new();
        for a in args {
            arr.push(a);
        }
        f.apply(&o, &arr).ok()?.dyn_into::<Promise>().ok()
    }

    pub fn available() -> bool {
        (|| {
            let o = obj()?;
            let f = Reflect::get(&o, &JsValue::from_str("available")).ok()?.dyn_into::<Function>().ok()?;
            f.call0(&o).ok()?.as_bool()
        })()
        .unwrap_or(false)
    }
}

/// Whether purchases can happen on this platform (iOS with the plugin). False
/// on the web, where the paywall says "Get the app" instead (D7).
pub fn store_available() -> bool {
    #[cfg(not(feature = "education"))]
    {
        bridge::available()
    }
    #[cfg(feature = "education")]
    {
        false
    }
}

/// On launch, read the owned set from StoreKit and subscribe to changes.
pub fn init() {
    #[cfg(not(feature = "education"))]
    {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsValue;
        if let Some(p) = bridge::call("entitlements", &[]) {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(ids) = wasm_bindgen_futures::JsFuture::from(p).await {
                    set_from_js(&ids);
                }
            });
        }
        if let Some(o) = bridge::obj() {
            let cb = Closure::<dyn FnMut(JsValue)>::new(|ids: JsValue| set_from_js(&ids));
            if let Ok(f) = js_sys::Reflect::get(&o, &JsValue::from_str("onChange")) {
                if let Ok(f) = wasm_bindgen::JsCast::dyn_into::<js_sys::Function>(f) {
                    let _ = f.call1(&o, cb.as_ref());
                }
            }
            cb.forget();
        }
    }
}

/// One product as StoreKit priced it. Prices are only ever StoreKit's
/// `displayPrice`, never a literal (I4.7).
#[derive(Debug, Clone, PartialEq)]
pub struct StoreProduct {
    pub id: String,
    pub display_name: String,
    pub display_price: String,
}

/// The outcome of a purchase tap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseOutcome {
    Success,
    Cancelled,
    /// Ask to Buy: waiting on a parent. The owned set updates later via onChange.
    Pending,
    Failed,
    /// Not on iOS.
    Unavailable,
}

/// Load the core's products with StoreKit prices. `Err` means show a retry,
/// never a blank or "$0.00" (A4.8).
#[cfg(not(feature = "education"))]
pub async fn products() -> Result<Vec<StoreProduct>, ()> {
    use wasm_bindgen::JsValue;
    let ids = js_sys::Array::new();
    for id in PRODUCT_IDS {
        ids.push(&JsValue::from_str(id));
    }
    let p = bridge::call("products", &[ids.into()]).ok_or(())?;
    let v = wasm_bindgen_futures::JsFuture::from(p).await.map_err(|_| ())?;
    let get = |o: &JsValue, k: &str| {
        js_sys::Reflect::get(o, &JsValue::from_str(k)).ok().and_then(|x| x.as_string()).unwrap_or_default()
    };
    let out: Vec<StoreProduct> = js_sys::Array::from(&v)
        .iter()
        .map(|o| StoreProduct { id: get(&o, "id"), display_name: get(&o, "displayName"), display_price: get(&o, "displayPrice") })
        .filter(|p| PRODUCT_IDS.contains(&p.id.as_str()) && !p.display_price.is_empty())
        .collect();
    if out.is_empty() {
        Err(())
    } else {
        Ok(out)
    }
}

/// Buy one of the core's products. The fresh owned set is applied whatever the
/// outcome, so an interrupted purchase cannot leave partial state (A4.7).
#[cfg(not(feature = "education"))]
pub async fn purchase(id: &str) -> PurchaseOutcome {
    use wasm_bindgen::JsValue;
    if !PRODUCT_IDS.contains(&id) {
        return PurchaseOutcome::Failed;
    }
    let Some(p) = bridge::call("purchase", &[JsValue::from_str(id)]) else {
        return PurchaseOutcome::Unavailable;
    };
    match wasm_bindgen_futures::JsFuture::from(p).await {
        Ok(r) => {
            if let Ok(ids) = js_sys::Reflect::get(&r, &JsValue::from_str("ids")) {
                set_from_js(&ids);
            }
            let status = js_sys::Reflect::get(&r, &JsValue::from_str("status")).ok().and_then(|s| s.as_string()).unwrap_or_default();
            match status.as_str() {
                "success" => PurchaseOutcome::Success,
                "cancelled" => PurchaseOutcome::Cancelled,
                "pending" => PurchaseOutcome::Pending,
                _ => PurchaseOutcome::Failed,
            }
        }
        Err(_) => {
            refresh().await;
            PurchaseOutcome::Failed
        }
    }
}

/// Restore Purchases. `false` means the restore itself failed (show a retry).
#[cfg(not(feature = "education"))]
pub async fn restore() -> bool {
    let Some(p) = bridge::call("restore", &[]) else { return false };
    match wasm_bindgen_futures::JsFuture::from(p).await {
        Ok(ids) => {
            set_from_js(&ids);
            true
        }
        Err(_) => false,
    }
}

/// Re-read the owned set from StoreKit.
#[cfg(not(feature = "education"))]
pub async fn refresh() {
    if let Some(p) = bridge::call("entitlements", &[]) {
        if let Ok(ids) = wasm_bindgen_futures::JsFuture::from(p).await {
            set_from_js(&ids);
        }
    }
}
