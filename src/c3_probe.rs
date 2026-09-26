//! CC-FEEDBACK §0 C3 — can feedback sounds follow the ring/silent switch while
//! word audio keeps playing?
//!
//! §0 says prove it with a device test, not documentation, and HALT rather than
//! pick a fallback. This is that device test, reduced to one tap, because the
//! person holding the phone is the instrument and their time is the cost.
//!
//! # What the code already tells us, and what it cannot
//!
//! `AVAudioSession` is ONE object per process. `AppDelegate` sets `.playback`
//! at launch and the speech paths re-assert it. So D4's "feedback on ambient,
//! word audio on playback" cannot be ASSIGNED — there is only ever one category
//! in force. The open question is narrower and genuinely unknown: does
//! SWITCHING the category around each sound work, and does WebAudio inside the
//! WKWebView honour the switch at all?
//!
//! Nothing in the repo answers that, and guessing is what §0 forbids.
//!
//! # Why the tone is `audio_boost::chime`
//!
//! Because F3's feedback sounds will be WebAudio through the shared
//! `AudioContext`, and a probe that used a different audio path would measure
//! something the real feature will not do. The word plays through the ordinary
//! resolver for the same reason.

use crate::App;
use crate::{api, audio_boost, dom, native_lang};
use wasm_bindgen_futures::{spawn_local, JsFuture};

const MARKER: &str = "c3Probe";

/// Ask for a category and report back what the session actually says. The
/// distinction matters: a refused switch must read as a refusal, not as a
/// result.
async fn set_category(which: &str) -> String {
    let Some(p) = native_lang::set_audio_category(which) else {
        return "no bridge (not an iOS build)".into();
    };
    match JsFuture::from(p).await {
        Ok(v) => {
            let got = js_sys::Reflect::get(&v, &wasm_bindgen::JsValue::from_str("category"))
                .ok()
                .and_then(|x| x.as_string())
                .unwrap_or_else(|| "?".into());
            let err = js_sys::Reflect::get(&v, &wasm_bindgen::JsValue::from_str("error"))
                .ok()
                .and_then(|x| x.as_string())
                .unwrap_or_default();
            if err.is_empty() { got } else { format!("{got} (error: {err})") }
        }
        Err(_) => "rejected".into(),
    }
}

fn say(msg: &str) {
    dom::set_text(&format!("{MARKER}Out"), msg);
}

/// The word the probe plays, and its language.
///
/// The live word when there is one, so the probe exercises exactly what a
/// player hears. Otherwise the first easy word of the current language --
/// because the probe is usually opened from the hub, and "start a round first"
/// is a stumbling block in a test whose whole point is to cost ten minutes.
fn current(app: &App) -> (String, String) {
    let s = app.borrow();
    let lang = s.cur_lang.clone();
    let live = if s.spoken.is_empty() { s.word.clone() } else { s.spoken.clone() };
    if !live.is_empty() {
        return (live, lang);
    }
    drop(s);
    let fallback = crate::words::tier_for(&lang, "easy")
        .first()
        .map(|w| w.split('|').next().unwrap_or(w).to_string())
        .unwrap_or_default();
    (fallback, lang)
}

fn play_word_now(app: &App) {
    let (word, lang) = current(app);
    if word.is_empty() {
        say("No word loaded — start a round first, then reopen this.");
        return;
    }
    api::play_word(&word, "normal", 1.0, &lang, || {});
}

/// The D4 sequence: ambient, tone, gap, playback, word.
///
/// The gap is 600 ms rather than F3's 250 ms so the two sounds cannot be
/// confused for one another by ear. This measures whether the split is
/// possible at all, not the final timing.
fn run_split(app: &App) {
    let a = app.clone();
    spawn_local(async move {
        let c1 = set_category("ambient").await;
        say(&format!("category → {c1}; tone now…"));
        audio_boost::chime();
        let a2 = a.clone();
        dom::after_ms(600, move || {
            spawn_local(async move {
                let c2 = set_category("playback").await;
                say(&format!("tone was on {c1}, word is on {c2} — which did you hear?"));
                play_word_now(&a2);
            });
        });
    });
}

fn body() -> String {
    format!(
        r#"<h4>C3 — does the silent switch reach feedback sounds?</h4>
<p style="opacity:.75;font-size:13px;line-height:1.45">One AVAudioSession per process, so the two cannot hold
different categories at once. This measures whether <i>switching</i> around each
sound works, and whether WebAudio in the WebView honours it at all.</p>
<ol style="font-size:13px;line-height:1.6;padding-inline-start:20px">
  <li><b>Control.</b> Ring switch <b>NORMAL</b> → tap <i>Run the split</i>. You
      should hear a tone, then a word. If you do not, the rig is wrong and
      nothing below means anything.</li>
  <li><b>The test.</b> Ring switch <b>SILENT</b> → tap <i>Run the split</i> again.</li>
  <li>Report which one you heard.</li>
</ol>
<div style="display:flex;gap:8px;flex-wrap:wrap;margin:10px 0">
  <button class="ghost" id="{MARKER}Split">Run the split (tone → word)</button>
  <button class="ghost" id="{MARKER}Tone">Tone only</button>
  <button class="ghost" id="{MARKER}Word">Word only</button>
</div>
<div style="display:flex;gap:8px;flex-wrap:wrap;margin:10px 0">
  <button class="ghost" id="{MARKER}Amb">Set ambient</button>
  <button class="ghost" id="{MARKER}Play">Set playback</button>
</div>
<p style="min-height:20px;font-family:var(--mono);font-size:13px;opacity:.9" id="{MARKER}Out">—</p>
<table style="font-size:12.5px;line-height:1.5;border-collapse:collapse">
  <tr><td style="padding:2px 10px 2px 0;white-space:nowrap"><b>word only</b></td><td>D4 works: feedback follows the switch, the word still plays</td></tr>
  <tr><td style="padding:2px 10px 2px 0;white-space:nowrap"><b>both</b></td><td>ambient did not silence WebAudio — D4 fails as written</td></tr>
  <tr><td style="padding:2px 10px 2px 0;white-space:nowrap"><b>neither</b></td><td>the switch back to playback did not take, or it silences everything</td></tr>
  <tr><td style="padding:2px 10px 2px 0;white-space:nowrap"><b>tone only</b></td><td>backwards — report this one, it means something is very wrong</td></tr>
</table>
<p style="opacity:.75;font-size:13px;line-height:1.45">Whatever you hear, D4 is <i>gated on this</i>: §0 says HALT
rather than pick a fallback, so tell Claude the result and nothing gets guessed.</p>"#
    )
}

fn render(app: &App) {
    dom::set_html(
        MARKER,
        &format!(
            "<div style=\"max-height:80vh;overflow:auto;padding:14px;background:var(--panel);border-radius:14px\">{}<button class=\"ghost\" id=\"{MARKER}Close\">Close</button></div>",
            body()
        ),
    );
    dom::add_class(MARKER, "show");
    dom::on_click(&format!("{MARKER}Close"), || dom::remove_class(MARKER, "show"));

    let a = app.clone();
    dom::on_click(&format!("{MARKER}Split"), move || run_split(&a));
    dom::on_click(&format!("{MARKER}Tone"), || {
        audio_boost::chime();
        say("tone played (category unchanged)");
    });
    let a = app.clone();
    dom::on_click(&format!("{MARKER}Word"), move || {
        play_word_now(&a);
        say("word played (category unchanged)");
    });
    dom::on_click(&format!("{MARKER}Amb"), || {
        spawn_local(async { let c = set_category("ambient").await; say(&format!("category → {c}")); })
    });
    dom::on_click(&format!("{MARKER}Play"), || {
        spawn_local(async { let c = set_category("playback").await; say(&format!("category → {c}")); })
    });
}

/// Build the panel and its dev-menu door. Called once at startup, only in a
/// `dev_preview` build.
pub fn wire(app: &App) {
    let doc = dom::doc();
    if doc.get_element_by_id(MARKER).is_none() {
        if let (Ok(panel), Some(body_el)) = (doc.create_element("div"), doc.body()) {
            panel.set_id(MARKER);
            let _ = panel.set_attribute("class", "scrim");
            let _ = body_el.append_child(&panel);
        }
    }
    let btn_id = format!("{MARKER}Open");
    if let Some(menu) = doc.query_selector("#devMenu .modal").ok().flatten() {
        if doc.get_element_by_id(&btn_id).is_none() {
            if let Ok(btn) = doc.create_element("button") {
                btn.set_id(&btn_id);
                let _ = btn.set_attribute("class", "ghost");
                btn.set_text_content(Some("C3 audio-session probe"));
                let _ = menu.append_child(&btn);
            }
        }
    }
    let a = app.clone();
    dom::on_click(&btn_id, move || {
        dom::remove_class("devMenu", "show");
        render(&a);
    });
}
