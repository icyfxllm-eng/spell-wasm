//! F4 — deep-link router (custom scheme `spellgame://`).
//!
//! ONE entry path for every external launch into a specific surface: F3's
//! home-screen widgets (`spellgame://daily`, `spellgame://streak`) and F4's App
//! Intents (`spellgame://daily`, `spellgame://missed`, `spellgame://streak`) all
//! arrive here. F3 registered the scheme in the App's Info.plist and pointed its
//! widgets at it, but left the in-app ROUTING unwired — this module is that
//! router, so intents and widgets share exactly one plumbing (no divergent entry
//! paths).
//!
//! ## How a URL reaches this module
//!
//! On the native (Capacitor iOS) build, `AppDelegate.application(_:open:)` hands
//! the opened URL to the WKWebView as `window.SpellHandleUrl("spellgame://…")`
//! (see `ios/App/App/AppDelegate.swift`). That JS shim ([`install`] installs its
//! Rust half as `window.SpellRouter.open`) forwards the string here. A URL that
//! arrives during a cold launch — before the WASM module has installed the
//! router — is buffered by the shim in `window.__spellPendingUrl` and drained by
//! [`install`] the moment the router comes up. Off iOS there is no URL opener, so
//! this is inert (the router installs but is never called).
//!
//! ## NOT feature-flagged
//!
//! The router itself is always live — routing a widget/intent tap to the right
//! surface is harmless and F3's widgets already ship. The `flags::app_intents()`
//! gate (default OFF) is about ADVERTISING the App Intents, not about the router;
//! see that flag's docs and the PR for the "AppShortcuts are always registered"
//! nuance.

use crate::App;

/// The surfaces an external launch can request. Anything unrecognized is
/// [`Route::None`] — a deliberate no-op so a stray/garbage URL never disturbs the
/// running game.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Route {
    /// Today's Daily Challenge (`spellgame://daily`).
    Daily,
    /// Missed-words review (`spellgame://missed`).
    Review,
    /// Streak surface (`spellgame://streak`). The streak value is answered from
    /// the App Group by the widget / the "Check My Streak" intent; the launch
    /// itself just foregrounds the app to home, so dispatch is a no-op.
    Streak,
    /// Unknown scheme/host — no navigation.
    None,
}

/// Pure map from a `spellgame://<host>` URL to a [`Route`]. The scheme must be
/// `spellgame` (case-insensitive); the host is matched case-insensitively and any
/// path/query/fragment is ignored. Everything else → [`Route::None`]. Pure (no
/// DOM, no clock) so it is unit-testable under host `cargo test`.
pub fn parse_route(url: &str) -> Route {
    let rest = match url.trim().split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("spellgame") => rest,
        _ => return Route::None,
    };
    // Host = everything up to the first path/query/fragment delimiter.
    let host = rest
        .split(|c| c == '/' || c == '?' || c == '#')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match host.as_str() {
        "daily" => Route::Daily,
        // "missed" is F4's added route; accept the F3-era synonyms too so a widget
        // built against either spelling still lands on the review surface.
        "missed" | "misses" | "review" => Route::Review,
        "streak" => Route::Streak,
        _ => Route::None,
    }
}

/// Navigate the running app to the surface a [`Route`] names. Mirrors exactly what
/// tapping the in-app Daily / Misses affordances does, so intents and widgets
/// reuse the app's own entry logic (no parallel gameplay path).
pub fn dispatch(app: &App, route: Route) {
    match route {
        Route::Daily => {
            // Same branch the Daily button takes: today's result if already done,
            // otherwise start today's run.
            if crate::daily::is_done_today() {
                crate::game::show_today_result(app);
            } else {
                crate::game::enter_daily(app);
            }
        }
        // enter_review self-guards (no misses / none due -> a friendly note, no
        // mode switch), exactly like tapping the Misses button.
        Route::Review => crate::game::enter_review(app),
        // Foregrounding to home IS the whole action; the streak number is read
        // from the App Group by the widget / intent, not recomputed here.
        Route::Streak => {}
        Route::None => {}
    }
}

/// Parse + dispatch in one call — the single funnel every external URL flows
/// through.
pub fn handle(app: &App, url: &str) {
    dispatch(app, parse_route(url));
}

/// Install the router's JS half: `window.SpellRouter.open(url)` forwards a URL
/// (from `window.SpellHandleUrl` in the page shell) into [`handle`]. Also drains
/// any URL the shell buffered in `window.__spellPendingUrl` before the router was
/// ready (cold-launch-via-URL). No-op off the browser/wasm host (`window()` is
/// `None`), so `cargo test` never touches this.
pub fn install(app: &App) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsValue;

    let Some(win) = web_sys::window() else { return };

    let obj = js_sys::Object::new();
    {
        let a = app.clone();
        let cb = Closure::<dyn Fn(String)>::new(move |url: String| handle(&a, &url));
        let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("open"), &cb.into_js_value());
    }
    let _ = js_sys::Reflect::set(&win, &JsValue::from_str("SpellRouter"), &obj);

    // Drain a cold-launch URL captured before the router existed.
    if let Ok(pending) = js_sys::Reflect::get(&win, &JsValue::from_str("__spellPendingUrl")) {
        if let Some(url) = pending.as_string() {
            let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__spellPendingUrl"), &JsValue::NULL);
            if !url.is_empty() {
                handle(app, &url);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_three_intent_routes() {
        assert_eq!(parse_route("spellgame://daily"), Route::Daily);
        assert_eq!(parse_route("spellgame://missed"), Route::Review);
        assert_eq!(parse_route("spellgame://streak"), Route::Streak);
    }

    #[test]
    fn unknown_and_malformed_are_noops() {
        assert_eq!(parse_route("spellgame://nope"), Route::None);
        assert_eq!(parse_route("spellgame://"), Route::None);
        assert_eq!(parse_route("https://spellgame.net/daily"), Route::None);
        assert_eq!(parse_route("daily"), Route::None);
        assert_eq!(parse_route(""), Route::None);
        assert_eq!(parse_route("otherscheme://daily"), Route::None);
    }

    #[test]
    fn host_is_case_insensitive_and_ignores_path_query() {
        assert_eq!(parse_route("SpellGame://Daily"), Route::Daily);
        assert_eq!(parse_route("spellgame://DAILY/"), Route::Daily);
        assert_eq!(parse_route("spellgame://missed?src=siri"), Route::Review);
        assert_eq!(parse_route("spellgame://streak#now"), Route::Streak);
    }

    #[test]
    fn review_synonyms_all_land_on_review() {
        // F3-era widgets may have used a different spelling; all reach review.
        assert_eq!(parse_route("spellgame://misses"), Route::Review);
        assert_eq!(parse_route("spellgame://review"), Route::Review);
    }
}
