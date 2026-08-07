//! CC-OFFLINE-PACKS (BD-2) — the pack manager. Decisions baked in (Eric,
//! 2026-08-02): per-language packs (BD-D2), 150 MB budget enforced
//! server-side, auto-SUGGEST never auto-download (D3). Laws: a pack is
//! verified-active or absent (I2); ONE audio resolution order lives in
//! api.rs and this module only answers "is there a pack src?" (I3);
//! everything here is app-only — every native call no-ops on the web
//! shell (I5).

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Ed25519 public half of the server's pack-signing key (data/
/// pack-signing.pem on the Mac; raw 32 bytes, base64). A manifest that
/// fails this signature quarantines the whole pack (I4).
const PACK_PUB_KEY: &str = "YK9maxkoaZM2YaSRWmYqU7S/Y8wVo5GvU5/Fw/yR/x4=";

thread_local! {
    /// Languages with a verified-ACTIVE pack (mirrors native state; the
    /// native filesystem is the truth, this is the render cache).
    static ACTIVE: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    /// (lang, fetched, total) while a download runs — drives the row UI.
    static PROGRESS: std::cell::RefCell<Option<(String, u32, u32)>> = const { std::cell::RefCell::new(None) };
}

fn kit() -> Option<js_sys::Object> {
    let win = web_sys::window()?;
    let cap = js_sys::Reflect::get(&win, &JsValue::from_str("Capacitor")).ok()?;
    if cap.is_undefined() {
        return None;
    }
    let plugins = js_sys::Reflect::get(&cap, &JsValue::from_str("Plugins")).ok()?;
    let kit = js_sys::Reflect::get(&plugins, &JsValue::from_str("NativeLanguageKit")).ok()?;
    if kit.is_undefined() {
        None
    } else {
        kit.dyn_into().ok()
    }
}

async fn call(method: &str, args: &js_sys::Object) -> Result<JsValue, JsValue> {
    let kit = kit().ok_or_else(|| JsValue::from_str("no native"))?;
    let f: js_sys::Function = js_sys::Reflect::get(&kit, &JsValue::from_str(method))?.dyn_into()?;
    let promise: js_sys::Promise = f.call1(&kit, args)?.dyn_into()?;
    JsFuture::from(promise).await
}

fn obj(pairs: &[(&str, &str)]) -> js_sys::Object {
    let o = js_sys::Object::new();
    for (k, v) in pairs {
        let _ = js_sys::Reflect::set(&o, &JsValue::from_str(k), &JsValue::from_str(v));
    }
    o
}

pub fn available() -> bool {
    kit().is_some()
}

pub fn is_active(lang: &str) -> bool {
    ACTIVE.with(|a| a.borrow().iter().any(|l| l == lang))
}

pub fn progress_for(lang: &str) -> Option<(u32, u32)> {
    PROGRESS.with(|p| p.borrow().as_ref().filter(|(l, _, _)| l == lang).map(|(_, d, t)| (*d, *t)))
}

/// Refresh the ACTIVE mirror from the native filesystem, then re-render
/// whatever pack UI is open.
pub async fn refresh_states() {
    let Ok(v) = call("packStates", &js_sys::Object::new()).await else { return };
    let mut langs = Vec::new();
    if let Ok(packs) = js_sys::Reflect::get(&v, &JsValue::from_str("packs")) {
        let arr = js_sys::Array::from(&packs);
        for item in arr.iter() {
            if let Ok(l) = js_sys::Reflect::get(&item, &JsValue::from_str("lang")) {
                if let Some(s) = l.as_string() {
                    langs.push(s);
                }
            }
        }
    }
    ACTIVE.with(|a| *a.borrow_mut() = langs);
    render_rows();
}

/// The pack src for a word, or None — api.rs asks this FIRST and falls
/// through to cache -> network on None (the single resolution order).
pub async fn src_for(lang: &str, word: &str, variant: &str) -> Option<String> {
    if !is_active(lang) {
        return None;
    }
    let v = call("packSrcFor", &obj(&[("lang", lang), ("word", word), ("variant", variant)]))
        .await
        .ok()?;
    let src = js_sys::Reflect::get(&v, &JsValue::from_str("src")).ok()?.as_string()?;
    if src.is_empty() {
        None
    } else {
        Some(src)
    }
}

#[derive(serde::Deserialize)]
struct ManifestFile {
    name: String,
    sha256: String,
}

#[derive(serde::Deserialize)]
struct Manifest {
    files: Vec<ManifestFile>,
    version: u32,
}

/// Download + verify + activate one language pack. Sequentially fetches
/// only what staging is missing (resume-by-construction), verifies each
/// file's sha256 natively, stores the SIGNED manifest beside the audio,
/// and activates atomically. One honest audited error on any failure.
pub async fn download(lang: String) {
    if PROGRESS.with(|p| p.borrow().is_some()) {
        return; // one download at a time keeps the UI honest
    }
    let base = format!("{}/packs/{}/v1", crate::api::api_base(), lang);
    let manifest_txt = match fetch_text(&format!("{base}/manifest.json")).await {
        Some(t) => t,
        None => return fail(&lang),
    };
    let sig_b64 = match fetch_b64(&format!("{base}/manifest.sig")).await {
        Some(s) => s,
        None => return fail(&lang),
    };
    let v = call("packVerifyManifest", &obj(&[("manifest", &manifest_txt), ("sig", &sig_b64), ("pubKey", PACK_PUB_KEY)])).await;
    let valid = v
        .ok()
        .and_then(|r| js_sys::Reflect::get(&r, &JsValue::from_str("valid")).ok())
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    if !valid {
        return fail(&lang);
    }
    let Ok(man) = serde_json::from_str::<Manifest>(&manifest_txt) else { return fail(&lang) };
    let _ = call("packStoreManifest", &obj(&[("lang", &lang), ("manifest", &manifest_txt)])).await;
    let total = man.files.len() as u32;
    for (i, f) in man.files.iter().enumerate() {
        PROGRESS.with(|p| *p.borrow_mut() = Some((lang.clone(), i as u32, total)));
        if i % 64 == 0 {
            render_rows();
        }
        let url = format!("{base}/{}", f.name);
        if call("packFetch", &obj(&[("url", &url), ("lang", &lang), ("name", &f.name), ("sha256", &f.sha256)])).await.is_err() {
            return fail(&lang);
        }
    }
    if call("packActivate", &obj(&[("lang", &lang)])).await.is_err() {
        return fail(&lang);
    }
    let _ = man.version;
    PROGRESS.with(|p| *p.borrow_mut() = None);
    refresh_states().await;
}

pub async fn delete(lang: String) {
    let _ = call("packDelete", &obj(&[("lang", &lang)])).await;
    refresh_states().await;
}

fn fail(lang: &str) {
    PROGRESS.with(|p| *p.borrow_mut() = None);
    let _ = lang;
    crate::dom::show_toast(&crate::i18n::t("packs.error"));
    render_rows();
}

async fn fetch_text(url: &str) -> Option<String> {
    let win = web_sys::window()?;
    let resp: web_sys::Response = JsFuture::from(win.fetch_with_str(url)).await.ok()?.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    JsFuture::from(resp.text().ok()?).await.ok()?.as_string()
}

/// Local base64 (RFC 4648) — packs may not import the Spell Picture
/// subtree (the wall scanner enforces the direction of that dependency),
/// and fifteen lines beat an architectural exception.
fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

async fn fetch_b64(url: &str) -> Option<String> {
    let win = web_sys::window()?;
    let resp: web_sys::Response = JsFuture::from(win.fetch_with_str(url)).await.ok()?.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    let buf = JsFuture::from(resp.array_buffer().ok()?).await.ok()?;
    let bytes = js_sys::Uint8Array::new(&buf).to_vec();
    Some(b64(&bytes))
}

/// The settings rows: one per Active language, download/progress/delete.
/// App-shell-only (dom::exists), audited strings only.
pub fn render_rows() {
    // F15/D9: hidden entirely until there is a server to talk to. Note
    // this hides the SECTION, not just the buttons — a heading over an
    // empty box still promises a feature that does not exist.
    if !crate::flags::offline_packs() {
        crate::dom::toggle_class("packSection", "btn-hide", true);
        return;
    }
    if !crate::dom::exists("packRows") || !available() {
        return;
    }
    crate::dom::toggle_class("packSection", "btn-hide", false);
    let mut html = String::new();
    for (code, info) in crate::words::LANGUAGES.iter() {
        if !crate::consts::is_active_lang(code) {
            continue;
        }
        let state = if let Some((d, t)) = progress_for(code) {
            format!("{d}/{t}")
        } else if is_active(code) {
            format!("<button class=\"ghost pk-del\" data-pack-del=\"{code}\">{}</button>", crate::i18n::t("packs.delete"))
        } else {
            format!("<button class=\"ghost pk-dl\" data-pack-dl=\"{code}\">{}</button>", crate::i18n::t("packs.download"))
        };
        html.push_str(&format!(
            "<div class=\"pack-row\"><span>{}</span><span class=\"pk-state\">{}</span></div>",
            info.name, state
        ));
    }
    crate::dom::set_html("packRows", &html);
}

pub fn wire(app: &crate::App) {
    if !crate::dom::exists("packRows") {
        return;
    }
    if !crate::flags::offline_packs() {
        crate::dom::toggle_class("packSection", "btn-hide", true);
        return; // no listeners, no auto-suggest toast, no dead affordance
    }
    let _ = app;
    crate::dom::on::<web_sys::MouseEvent, _>("packRows", "click", |e| {
        let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        if let Some(lang) = el.get_attribute("data-pack-dl") {
            wasm_bindgen_futures::spawn_local(download(lang));
        } else if let Some(lang) = el.get_attribute("data-pack-del") {
            wasm_bindgen_futures::spawn_local(delete(lang));
        }
    });
    wasm_bindgen_futures::spawn_local(async {
        refresh_states().await;
        // D3 (signed): the home-grant language auto-SUGGESTS its pack once.
        // Never a download — one dismissible toast, one storage flag.
        if crate::storage::get_raw("spell_pack_suggested").is_none() {
            crate::storage::set_raw("spell_pack_suggested", "1");
            crate::dom::show_toast(&crate::i18n::t("packs.suggest"));
        }
    });
}
