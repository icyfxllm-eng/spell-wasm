//! BD-4 CC-FAMILY-VOICES V1 — Apple Personal Voice as orb voice.
//!
//! ALL policy lives here, not in Swift: which voices a parent approved,
//! whether a voice passed the readout gate (I2), and language honesty
//! (a voice only ever reads words in its own language — never a degraded
//! cross-language readout). The plugin reports facts and renders audio.
//!
//! I1: device-local by nature. I3: the approval UI renders ONLY inside
//! the guardian dash body, which sits behind the parent gate. I5:
//! app-only — every DOM touch is dom::exists-guarded, and the module
//! resolves to "no voice" wherever the plugin is absent.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

const APPROVED_KEY: &str = "spell_pv_approved_v1"; // Vec<(id, lang)>
const GATE_KEY: &str = "spell_pv_gate_v1"; // Vec<(id, bool passed)>
const PICK_KEY: &str = "spell_pv_pick_v1"; // Vec<(lang, id)>

/// The audited calibration list (readme #4): short words rich in the
/// minimal-pair confusions the QA harness cares about. Fixed — the gate
/// means nothing if the material drifts.
pub const CALIBRATION_EN: [&str; 10] =
    ["cat", "bat", "made", "ship", "sheep", "light", "right", "tree", "three", "done"];
/// Pass bar: 8 of 10 transcripts must match after normalization.
pub const CALIBRATION_PASS: usize = 8;

// ---------------- pure policy (unit-tested) ----------------

pub fn approved() -> Vec<(String, String)> {
    crate::storage::get_json(APPROVED_KEY).unwrap_or_default()
}

pub fn set_approved(id: &str, lang: &str, on: bool) {
    let mut v = approved();
    v.retain(|(i, _)| i != id);
    if on {
        v.push((id.to_string(), lang.to_string()));
    }
    crate::storage::set_json(APPROVED_KEY, &v);
}

pub fn gate_passed(id: &str) -> Option<bool> {
    let v: Vec<(String, bool)> = crate::storage::get_json(GATE_KEY).unwrap_or_default();
    v.iter().find(|(i, _)| i == id).map(|(_, p)| *p)
}

pub fn set_gate(id: &str, passed: bool) {
    let mut v: Vec<(String, bool)> = crate::storage::get_json(GATE_KEY).unwrap_or_default();
    v.retain(|(i, _)| i != id);
    v.push((id.to_string(), passed));
    crate::storage::set_json(GATE_KEY, &v);
}

pub fn pick_for(lang: &str) -> Option<String> {
    let v: Vec<(String, String)> = crate::storage::get_json(PICK_KEY).unwrap_or_default();
    v.iter().find(|(l, _)| l == lang).map(|(_, id)| id.clone())
}

pub fn set_pick(lang: &str, id: Option<&str>) {
    let mut v: Vec<(String, String)> = crate::storage::get_json(PICK_KEY).unwrap_or_default();
    v.retain(|(l, _)| l != lang);
    if let Some(id) = id {
        v.push((lang.to_string(), id.to_string()));
    }
    crate::storage::set_json(PICK_KEY, &v);
}

/// Language honesty: `voice_lang` is a BCP-47 tag ("en-US"); it may read
/// for session `lang` only when the primary subtags match.
pub fn lang_matches(voice_lang: &str, lang: &str) -> bool {
    let sub = |s: &str| s.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    !lang.is_empty() && sub(voice_lang) == sub(lang)
}

/// THE resolution rule (I2 + honesty), the only door to word readout:
/// the kid picked it, a parent approved it for a matching language, and
/// it passed the readout gate. Anything less -> None (standard voice).
pub fn orb_voice(lang: &str) -> Option<String> {
    let id = pick_for(lang)?;
    let ap = approved();
    let (_, voice_lang) = ap.iter().find(|(i, _)| *i == id)?;
    if !lang_matches(voice_lang, lang) {
        return None;
    }
    if gate_passed(&id) != Some(true) {
        return None;
    }
    Some(id)
}

/// Judge a calibration run: normalized transcript equality, pass at the
/// fixed bar. Normalization mirrors the loopback oracle's: lowercase,
/// strip everything but letters (the recognizer loves trailing periods).
pub fn judge_calibration(results: &[(String, String)]) -> bool {
    let norm = |s: &str| {
        s.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect::<String>()
    };
    let hits = results.iter().filter(|(w, heard)| norm(w) == norm(heard)).count();
    hits >= CALIBRATION_PASS && results.len() >= CALIBRATION_EN.len()
}

// ---------------- plugin plumbing ----------------

fn kit() -> Option<js_sys::Object> {
    let win = web_sys::window()?;
    let cap = js_sys::Reflect::get(&win, &"Capacitor".into()).ok()?;
    let plugins = js_sys::Reflect::get(&cap, &"Plugins".into()).ok()?;
    js_sys::Reflect::get(&plugins, &"NativeLanguageKit".into()).ok()?.dyn_into().ok()
}

async fn call(method: &str, args: &JsValue) -> Option<JsValue> {
    let kit = kit()?;
    let f: js_sys::Function = js_sys::Reflect::get(&kit, &method.into()).ok()?.dyn_into().ok()?;
    let promise: js_sys::Promise = f.call1(&kit, args).ok()?.dyn_into().ok()?;
    JsFuture::from(promise).await.ok()
}

fn get(v: &JsValue, key: &str) -> Option<JsValue> {
    js_sys::Reflect::get(v, &key.into()).ok()
}

pub struct PvVoice {
    pub id: String,
    pub name: String,
    pub lang: String,
}

pub async fn status() -> (String, Vec<PvVoice>) {
    let Some(r) = call("pvStatus", &js_sys::Object::new()).await else {
        return ("unsupported".into(), Vec::new());
    };
    let status = get(&r, "status").and_then(|s| s.as_string()).unwrap_or_else(|| "unsupported".into());
    let mut voices = Vec::new();
    if let Some(arr) = get(&r, "voices") {
        for v in js_sys::Array::from(&arr).iter() {
            if let (Some(id), Some(name), Some(lang)) = (
                get(&v, "id").and_then(|x| x.as_string()),
                get(&v, "name").and_then(|x| x.as_string()),
                get(&v, "lang").and_then(|x| x.as_string()),
            ) {
                voices.push(PvVoice { id, name, lang });
            }
        }
    }
    (status, voices)
}

pub async fn request_auth() -> String {
    call("pvRequestAuth", &js_sys::Object::new())
        .await
        .and_then(|r| get(&r, "status").and_then(|s| s.as_string()))
        .unwrap_or_else(|| "unsupported".into())
}

/// Run the mic-free loopback for one voice and store the verdict.
pub async fn calibrate(id: &str, lang: &str) -> bool {
    let args = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&args, &"voiceId".into(), &id.into());
    let _ = js_sys::Reflect::set(&args, &"lang".into(), &lang.into());
    let words = js_sys::Array::new();
    for w in CALIBRATION_EN {
        words.push(&w.into());
    }
    let _ = js_sys::Reflect::set(&args, &"words".into(), &words);
    let mut results = Vec::new();
    if let Some(r) = call("pvCalibrate", &args).await {
        if let Some(arr) = get(&r, "results") {
            for v in js_sys::Array::from(&arr).iter() {
                if let (Some(w), Some(h)) = (
                    get(&v, "word").and_then(|x| x.as_string()),
                    get(&v, "heard").and_then(|x| x.as_string()),
                ) {
                    results.push((w, h));
                }
            }
        }
    }
    let passed = judge_calibration(&results);
    set_gate(id, passed);
    passed
}

// ---------------- surfaces ----------------

/// The parent-side section, rendered INSIDE gdashBody (already behind the
/// parent gate — I3 by construction).
pub fn dash_section_into(html: &mut String) {
    // Placeholder synchronously; filled async (plugin round-trip).
    html.push_str("<div class=\"gd-sec\" id=\"fvSec\"></div>");
}

pub fn fill_dash_section() {
    if !crate::dom::exists("fvSec") {
        return;
    }
    wasm_bindgen_futures::spawn_local(async {
        let (status, voices) = status().await;
        if !crate::dom::exists("fvSec") {
            return;
        }
        let t = |k: &str| crate::i18n::t(k);
        let mut html = format!("<div class=\"gd-h\">{}</div>", t("fv.title"));
        match status.as_str() {
            "authorized" if voices.is_empty() => {
                html.push_str(&format!("<div class=\"gd-row\">{}</div>", t("fv.none")));
            }
            "authorized" => {
                for v in &voices {
                    let ap = approved().iter().any(|(i, _)| *i == v.id);
                    let verdict = match gate_passed(&v.id) {
                        Some(true) => format!("<span class=\"gd-chip\">{}</span>", t("fv.pass")),
                        Some(false) => format!("<span class=\"gd-chip\">{}</span>", t("fv.celebOnly")),
                        None => String::new(),
                    };
                    html.push_str(&format!(
                        "<div class=\"gd-row\">{} <small>({})</small> {verdict} \
                         <button class=\"ghost\" data-fv-ap=\"{}\" data-fv-lang=\"{}\">{}</button> \
                         <button class=\"ghost\" data-fv-cal=\"{}\" data-fv-lang=\"{}\">{}</button></div>",
                        crate::dom::escape_html(&v.name), v.lang,
                        v.id, v.lang, if ap { t("fv.approved") } else { t("fv.approve") },
                        v.id, v.lang, t("fv.check"),
                    ));
                }
            }
            "notDetermined" => {
                html.push_str(&format!(
                    "<div class=\"gd-row\"><button class=\"ghost\" id=\"fvAuth\">{}</button></div>",
                    t("fv.request")
                ));
            }
            // denied / unsupported: one honest line, no nagging (accept. #2).
            _ => {
                html.push_str(&format!("<div class=\"gd-row\">{}</div>", t("fv.none")));
            }
        }
        crate::dom::set_html("fvSec", &html);
    });
}

/// The kid-side picker (setup panel): shown ONLY when at least one voice
/// has fully earned readout for the session language — no badge, no tease
/// otherwise. The kid chooses freely among earned voices (D2).
pub fn render_kid_row(app: &crate::App) {
    if !crate::dom::exists("fvKidSec") {
        return;
    }
    let lang = app.borrow().lang.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let (_, voices) = status().await;
        if !crate::dom::exists("fvKidSec") {
            return;
        }
        let ap = approved();
        let eligible: Vec<&PvVoice> = voices
            .iter()
            .filter(|v| {
                ap.iter().any(|(i, _)| *i == v.id)
                    && lang_matches(&v.lang, &lang)
                    && gate_passed(&v.id) == Some(true)
            })
            .collect();
        if eligible.is_empty() {
            crate::dom::add_class("fvKidSec", "btn-hide");
            return;
        }
        let pick = pick_for(&lang);
        let mut html = format!(
            "<button class=\"ghost{}\" data-fv-pick=\"\">{}</button>",
            if pick.is_none() { " on" } else { "" },
            crate::i18n::t("fv.standard")
        );
        for v in eligible {
            html.push_str(&format!(
                "<button class=\"ghost{}\" data-fv-pick=\"{}\">{}</button>",
                if pick.as_deref() == Some(v.id.as_str()) { " on" } else { "" },
                v.id,
                crate::dom::escape_html(&v.name)
            ));
        }
        crate::dom::set_html("fvKidRow", &html);
        crate::dom::remove_class("fvKidSec", "btn-hide");
    });
}

/// Wire the section's clicks (delegated once, on the dash body).
pub fn wire(app: &crate::App) {
    if !crate::dom::exists("gdashBody") {
        return;
    }
    {
        let a = app.clone();
        crate::dom::on::<web_sys::MouseEvent, _>("fvKidRow", "click", move |e| {
            let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
            if let Some(id) = el.get_attribute("data-fv-pick") {
                let lang = a.borrow().lang.clone();
                set_pick(&lang, if id.is_empty() { None } else { Some(&id) });
                render_kid_row(&a);
            }
        });
        render_kid_row(app);
    }
    crate::dom::on::<web_sys::MouseEvent, _>("gdashBody", "click", |e| {
        let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        if el.get_attribute("id").as_deref() == Some("fvAuth") {
            wasm_bindgen_futures::spawn_local(async {
                request_auth().await;
                fill_dash_section();
            });
        } else if let (Some(id), Some(lang)) = (el.get_attribute("data-fv-ap"), el.get_attribute("data-fv-lang")) {
            let on = !approved().iter().any(|(i, _)| *i == id);
            set_approved(&id, &lang, on);
            fill_dash_section();
        } else if let (Some(id), Some(lang)) = (el.get_attribute("data-fv-cal"), el.get_attribute("data-fv-lang")) {
            wasm_bindgen_futures::spawn_local(async move {
                calibrate(&id, &lang).await;
                fill_dash_section();
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{judge_calibration, lang_matches, CALIBRATION_EN};

    fn run(hits: usize) -> Vec<(String, String)> {
        CALIBRATION_EN
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let heard = if i < hits { format!("{}.", w.to_uppercase()) } else { "xyzzy".into() };
                (w.to_string(), heard)
            })
            .collect()
    }

    #[test]
    fn gate_passes_at_the_bar_and_not_below() {
        assert!(judge_calibration(&run(10)));
        assert!(judge_calibration(&run(8)), "8/10 is the bar");
        assert!(!judge_calibration(&run(7)), "7/10 fails");
        assert!(!judge_calibration(&[]), "an empty run can never pass");
        // A truncated run (plugin died mid-way) can't sneak under the bar.
        assert!(!judge_calibration(&run(8)[..8].to_vec()));
    }

    #[test]
    fn language_honesty_is_subtag_exact() {
        assert!(lang_matches("en-US", "en"));
        assert!(lang_matches("en_GB", "en"));
        assert!(!lang_matches("en-US", "es"), "never a cross-language readout");
        assert!(!lang_matches("", "en"));
        assert!(!lang_matches("en-US", ""));
    }
}
