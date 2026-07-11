//! Native share sheet for a player's chain result. Uses `@capacitor/share`
//! (`window.Capacitor.Plugins.Share`) on the app build, falling back to the
//! Web Share API (`navigator.share`) on mobile web. Where neither exists
//! (desktop browsers) `available()` is false and the Share button stays
//! hidden.
//!
//! The shared URL doubles as the deep link (App/Universal Links, later): the
//! link opens the app when installed, or the site/store page otherwise — the
//! share-card flywheel from CLAUDE.md.

use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};

/// Where the shared link points. This `/app` path is a verified Android App
/// Link (see AndroidManifest + /.well-known/assetlinks.json): it opens the app
/// when installed, and falls back to the web game otherwise.
const SHARE_URL: &str = "https://spellgame.net/app";

fn get(obj: &JsValue, key: &str) -> Option<JsValue> {
    let v = Reflect::get(obj, &JsValue::from_str(key)).ok()?;
    if v.is_undefined() || v.is_null() {
        None
    } else {
        Some(v)
    }
}

/// A native Capacitor plugin proxy by name, if present.
fn cap_plugin(name: &str) -> Option<JsValue> {
    let win = web_sys::window()?;
    let cap = get(&win, "Capacitor")?;
    let plugins = get(&cap, "Plugins")?;
    get(&plugins, name)
}

fn cap_share() -> Option<JsValue> {
    cap_plugin("Share")
}

fn call_method(obj: &JsValue, name: &str, arg: &JsValue) -> Option<JsValue> {
    let f = get(obj, name)?.dyn_into::<Function>().ok()?;
    f.call1(obj, arg).ok()
}

/// `navigator.share`, if present (mobile web).
fn nav_share() -> Option<(JsValue, Function)> {
    let win: JsValue = web_sys::window()?.into();
    let nav = get(&win, "navigator")?;
    let f = get(&nav, "share")?.dyn_into::<Function>().ok()?;
    Some((nav, f))
}

/// True when some share mechanism exists (native app or mobile web).
pub fn available() -> bool {
    cap_share().is_some() || nav_share().is_some()
}

fn build_message(streak: u32, best: u32) -> String {
    use crate::i18n::tp;
    if streak > 1 {
        tp("share.chain", &[("n", &streak.to_string()), ("best", &best.to_string())])
    } else if best > 0 {
        tp("share.chainBest", &[("best", &best.to_string())])
    } else {
        crate::i18n::t("share.tagline")
    }
}

fn base_opts(text: &str) -> Object {
    let opts = Object::new();
    let _ = Reflect::set(&opts, &JsValue::from_str("title"), &JsValue::from_str("Spell"));
    let _ = Reflect::set(&opts, &JsValue::from_str("text"), &JsValue::from_str(text));
    let _ = Reflect::set(&opts, &JsValue::from_str("url"), &JsValue::from_str(SHARE_URL));
    // Capacitor's Share also uses `dialogTitle` for the Android chooser.
    let _ = Reflect::set(&opts, &JsValue::from_str("dialogTitle"), &JsValue::from_str(&crate::i18n::t("share.button")));
    opts
}

/// Text-only share (native Capacitor Share, or Web Share on mobile web).
fn share_text(text: &str) {
    let opts = base_opts(text);
    if let Some(share) = cap_share() {
        if let Some(f) = get(&share, "share").and_then(|f| f.dyn_into::<Function>().ok()) {
            let _ = f.call1(&share, &opts);
            return;
        }
    }
    if let Some((nav, f)) = nav_share() {
        // navigator.share rejects if the user dismisses the sheet — ignored.
        let _ = f.call1(&nav, &opts);
    }
}

/// Share a Daily Challenge result (text + link; best-effort).
pub fn share_daily(correct: u32, total: u32, streak: u32) {
    use crate::i18n::tp;
    let text = if streak > 1 {
        tp("share.dailyStreak", &[("c", &correct.to_string()), ("t", &total.to_string()), ("n", &streak.to_string())])
    } else {
        tp("share.daily", &[("c", &correct.to_string()), ("t", &total.to_string())])
    };
    share_text(&text);
}

/// Open the native share sheet with the player's current chain result.
/// Prefers a rendered result-card image (native app); falls back to a
/// text + link share. Best-effort — does nothing if no share sheet exists.
pub fn share_result(streak: u32, best: u32) {
    let text = build_message(streak, best);

    // Native app: render a card image, cache it, and share the file.
    if let (Some(fs), Some(share)) = (cap_plugin("Filesystem"), cap_share()) {
        if let Some(data_url) = render_card_png(streak, best) {
            if let Some(b64) = data_url.split(",").nth(1) {
                share_image(fs, share, text.clone(), b64.to_string());
                return;
            }
        }
    }

    // Web / fallback: text + link only.
    share_text(&text);
}

/// Write the PNG (base64) to the app cache, then open the share sheet with the
/// image file attached. Runs async; on any failure it falls back to a text
/// share so the button always does something.
fn share_image(fs: JsValue, share: JsValue, text: String, b64: String) {
    let write = Object::new();
    let _ = Reflect::set(&write, &JsValue::from_str("path"), &JsValue::from_str("share/spell-chain.png"));
    let _ = Reflect::set(&write, &JsValue::from_str("data"), &JsValue::from_str(&b64));
    let _ = Reflect::set(&write, &JsValue::from_str("directory"), &JsValue::from_str("CACHE"));
    let _ = Reflect::set(&write, &JsValue::from_str("recursive"), &JsValue::TRUE);

    let Some(promise) = call_method(&fs, "writeFile", &write).and_then(|p| p.dyn_into::<js_sys::Promise>().ok())
    else {
        share_text(&text);
        return;
    };

    spawn_local(async move {
        // Filesystem.writeFile resolves with { uri }.
        let uri = match JsFuture::from(promise).await {
            Ok(res) => get(&res, "uri"),
            Err(_) => None,
        };
        let Some(uri) = uri else {
            share_text(&text);
            return;
        };
        let opts = base_opts(&text);
        let files = Array::new();
        files.push(&uri);
        let _ = Reflect::set(&opts, &JsValue::from_str("files"), &files);
        if let Some(f) = get(&share, "share").and_then(|f| f.dyn_into::<Function>().ok()) {
            let _ = f.call1(&share, &opts);
        }
    });
}

/// Render a square result card (streak/best + branding) to a PNG data URL.
/// Uses an offscreen canvas; returns None if canvas isn't available.
fn render_card_png(streak: u32, best: u32) -> Option<String> {
    let doc = web_sys::window()?.document()?;
    let canvas: web_sys::HtmlCanvasElement = doc.create_element("canvas").ok()?.dyn_into().ok()?;
    canvas.set_width(1080);
    canvas.set_height(1080);
    let ctx: web_sys::CanvasRenderingContext2d =
        canvas.get_context("2d").ok()??.dyn_into().ok()?;

    let cx = 540.0;
    // Background.
    ctx.set_fill_style_str("#1c1830");
    ctx.fill_rect(0.0, 0.0, 1080.0, 1080.0);

    // Orb with highlight + "S", echoing the app icon.
    ctx.begin_path();
    let _ = ctx.arc(cx, 340.0, 150.0, 0.0, std::f64::consts::PI * 2.0);
    ctx.set_fill_style_str("#ffb14d");
    ctx.fill();
    ctx.begin_path();
    let _ = ctx.arc(495.0, 300.0, 52.0, 0.0, std::f64::consts::PI * 2.0);
    ctx.set_fill_style_str("#ffd9a0");
    ctx.fill();
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    ctx.set_fill_style_str("#3a2416");
    ctx.set_font("800 165px sans-serif");
    let _ = ctx.fill_text("S", cx, 355.0);

    // Hero number: the current chain, or the best if the chain is 0.
    let (hero, label) = if streak > 0 {
        (streak, "WORD CHAIN")
    } else {
        (best, "BEST CHAIN")
    };
    ctx.set_fill_style_str("#ffffff");
    if hero > 0 {
        ctx.set_font("800 300px sans-serif");
        let _ = ctx.fill_text(&hero.to_string(), cx, 620.0);
        ctx.set_fill_style_str("#ffb14d");
        ctx.set_font("700 48px sans-serif");
        let _ = ctx.fill_text(label, cx, 800.0);
        if streak > 0 && best > 0 {
            ctx.set_fill_style_str("#b9b3d0");
            ctx.set_font("500 40px sans-serif");
            let _ = ctx.fill_text(&format!("your best: {best} words"), cx, 872.0);
        }
    } else {
        ctx.set_font("800 130px sans-serif");
        let _ = ctx.fill_text("Spell", cx, 600.0);
        ctx.set_fill_style_str("#ffb14d");
        ctx.set_font("600 44px sans-serif");
        let _ = ctx.fill_text("Can you keep the chain?", cx, 730.0);
    }

    // Brand footer.
    ctx.set_fill_style_str("#ffffff");
    ctx.set_font("700 42px sans-serif");
    let _ = ctx.fill_text("hear it \u{b7} spell it \u{b7} keep the chain", cx, 975.0);
    ctx.set_fill_style_str("#7c7796");
    ctx.set_font("500 36px sans-serif");
    let _ = ctx.fill_text("spellgame.net", cx, 1030.0);

    canvas.to_data_url_with_type("image/png").ok()
}
