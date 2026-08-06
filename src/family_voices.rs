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

// ═══════════════════ V2 CUSTOM FAMILY VOICES — BD-D4: LOCAL MAC ONLY
//
// Eric, 2026-08-05: training runs on the household's own Mac. Recordings
// NEVER leave the home. There is no upload path in this module and none
// may be added — `no_upload_path_exists` is the standing test, and the
// build's symbol scan is the second lock. The app's only jobs are:
// record the scripted prompts locally, write a training BUNDLE to the
// user's own storage, and later IMPORT the finished voice as a pack.

/// One line of the scripted prompt set. The script is drawn from the
/// AUDITED pools (I5) — this module never composes prompt text.
#[derive(Debug, Clone, PartialEq)]
pub struct VoicePrompt {
    /// i18n key in the audited pool.
    pub key: String,
    /// Ordinal in the session (stable: a resumed session asks the same
    /// prompt in the same place).
    pub idx: usize,
}

/// Piper needs a spread of phonetics, not volume: a short, fixed script
/// the parent can finish in one sitting. Keys only — the strings live
/// in the audited pool and are rendered by the caller.
pub const PROMPT_KEYS: [&str; 12] = [
    "fv.script.01", "fv.script.02", "fv.script.03", "fv.script.04",
    "fv.script.05", "fv.script.06", "fv.script.07", "fv.script.08",
    "fv.script.09", "fv.script.10", "fv.script.11", "fv.script.12",
];

pub fn prompt_script() -> Vec<VoicePrompt> {
    PROMPT_KEYS
        .iter()
        .enumerate()
        .map(|(idx, key)| VoicePrompt { key: (*key).to_string(), idx })
        .collect()
}

/// What the companion Mac app consumes. It names LOCAL clip handles —
/// never bytes, never a URL — so the bundle itself cannot become an
/// upload payload.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrainingBundle {
    pub voice_id: String,
    pub lang: String,
    /// (prompt key, local clip handle) in script order.
    pub clips: Vec<(String, String)>,
    /// Schema version so the companion can refuse a future shape.
    pub v: u32,
}

pub const BUNDLE_V: u32 = 1;

/// A bundle is only complete when EVERY scripted prompt has a clip —
/// a partial recording session trains a bad voice, so it cannot export.
pub fn build_bundle(voice_id: &str, lang: &str, clips: &[(String, String)]) -> Option<TrainingBundle> {
    let script = prompt_script();
    if clips.len() != script.len() {
        return None;
    }
    for (i, p) in script.iter().enumerate() {
        let (key, handle) = clips.get(i)?;
        if key != &p.key || handle.trim().is_empty() {
            return None;
        }
    }
    Some(TrainingBundle {
        voice_id: voice_id.to_string(),
        lang: lang.to_string(),
        clips: clips.to_vec(),
        v: BUNDLE_V,
    })
}

/// The finished voice comes back as an offline voice pack (the
/// CC-OFFLINE-PACKS manifest shape, voice ID field). Import REFUSES a
/// pack whose voice id was never recorded on this device — a voice
/// cannot arrive from nowhere.
pub fn import_trained_voice(pack_voice_id: &str, recorded_ids: &[String]) -> bool {
    !pack_voice_id.is_empty() && recorded_ids.iter().any(|id| id == pack_voice_id)
}

/// A trained voice still faces V1's readout gate before it may read
/// words — training locally does not exempt it from quality (D4: it may
/// still do celebrations after a gate failure).
pub fn trained_voice_may_read(voice_id: &str) -> bool {
    gate_passed(voice_id).unwrap_or(false)
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

#[cfg(test)]
mod v2_tests {
    use super::*;

    #[test]
    fn bundle_requires_the_whole_script() {
        let script = prompt_script();
        assert_eq!(script.len(), PROMPT_KEYS.len());
        let full: Vec<(String, String)> = script
            .iter()
            .map(|p| (p.key.clone(), format!("clip-{}", p.idx)))
            .collect();
        assert!(build_bundle("v1", "en", &full).is_some(), "a complete session exports");
        assert!(build_bundle("v1", "en", &full[..5]).is_none(), "a partial session cannot export");
        let mut blank = full.clone();
        blank[3].1 = "  ".into();
        assert!(build_bundle("v1", "en", &blank).is_none(), "an empty clip is not a clip");
        let mut wrong = full.clone();
        wrong[2].0 = "fv.script.99".into();
        assert!(build_bundle("v1", "en", &wrong).is_none(), "script order is part of the contract");
    }

    #[test]
    fn a_voice_cannot_arrive_from_nowhere() {
        let recorded = vec!["mom-en".to_string()];
        assert!(import_trained_voice("mom-en", &recorded));
        assert!(!import_trained_voice("stranger", &recorded), "unrecorded id is refused");
        assert!(!import_trained_voice("", &recorded));
    }

    /// BD-D4 IS AN INVARIANT: recordings never leave the household. This
    /// module must contain no network symbol of any kind — if someone
    /// adds one, this fails before the build's own scan does.
    #[test]
    fn no_upload_path_exists() {
        // CODE only: comments discuss the ban, code must not embody it.
        let src = include_str!("family_voices.rs");
        let code: String = src
            .lines()
            .take_while(|l| !l.contains("mod v2_tests"))
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for banned in [
            "fetch_post", "fetch_json", "fetch_text", "XMLHttpRequest",
            "api_base", "https://", "http://", "Request::new",
        ] {
            assert!(
                !code.contains(banned),
                "BD-D4 is an invariant: family_voices code must not reference `{banned}`"
            );
        }
    }

    #[test]
    fn training_locally_does_not_skip_the_readout_gate() {
        assert!(!trained_voice_may_read("never-gated"), "ungated voice cannot read words");
    }
}
