//! CC-THEMES-COSMOS F1: the Theme Lab, a row in the tap-5 dev menu.
//!
//! Every theme in the registry, locked, hidden and unreleased included, as a
//! live tile; tap one to preview it app-wide. A "LAB · {theme}" chip shows on
//! every screen while a preview is on; tapping it clears the preview. The
//! preview survives a relaunch until cleared (T-D9).
//!
//! T-D10 / I-T10: this module exists only in builds with the theme_lab feature
//! (dev builds, and the internal-only Lab TestFlight build, which is never
//! submitted). Its DOM and its stylesheet are made here, so a store build holds
//! no trace of it (scripts/seam-absence-check.mjs, AT-T2).
//!
//! It writes nothing durable but its own preview key: never the unlock ledger,
//! progress or telemetry (AT-T3). Its clock override reaches only the look
//! resolver (themes_cosmos::look via themes::apply), so Daily seeding, streaks,
//! days played, the Climb, the no-repeat history and timers never see it (I-T11).

use serde::{Deserialize, Serialize};
use wasm_bindgen::{closure::Closure, JsCast};

use crate::astro::Band;
use crate::dom;
use crate::themes::{Theme, REGISTRY};
use crate::themes_cosmos::{self as cosmos, LocalNow, Pins, Planet, PLANETS};

/// The only state the Lab stores (F1.8), in a dev-only key.
const KEY: &str = "spell_dev_theme_lab";

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
struct State {
    #[serde(default)]
    theme: String,
    #[serde(default)]
    band: String,
    #[serde(default)]
    phase: Option<u8>,
    #[serde(default)]
    planet: String,
    /// "YYYY-MM-DDTHH:MM", or empty to follow the device clock.
    #[serde(default)]
    clock: String,
    #[serde(default)]
    jr: bool,
    #[serde(default)]
    rm: bool,
}

fn load() -> State {
    crate::storage::get_json(KEY).unwrap_or_default()
}

fn save(s: &State) {
    crate::storage::set_json(KEY, s);
}

/// "YYYY-MM-DDTHH:MM" as a local date and time.
fn parse_clock(s: &str) -> Option<LocalNow> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-').map(|x| x.parse::<u32>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let (h, min) = time.split_once(':')?;
    Some(LocalNow { y: y as i32, m, d: day, h: h.parse().ok()?, min: min.get(..2)?.parse().ok()? })
}

fn pins(s: &State) -> Pins {
    Pins { band: cosmos::band_from_id(&s.band), phase: s.phase, planet: Planet::from_id(&s.planet) }
}

/// What themes::apply draws instead of the player's own theme.
pub fn preview() -> Option<(Theme, Pins, Option<LocalNow>)> {
    let s = load();
    let t = Theme::from_id(&s.theme)?;
    Some((t, pins(&s), parse_clock(&s.clock)))
}

const STYLE: &str = "
#labChip{position:fixed;z-index:9999;inset-block-start:calc(env(safe-area-inset-top,0px) + 6px);inset-inline-end:8px;padding:4px 10px;border-radius:999px;border:1.5px solid var(--glow);background:var(--panel-raised);color:var(--ink);font:700 12px/1.3 var(--body);letter-spacing:.04em}
#themeLab .modal{max-width:min(720px,94vw);max-height:88vh;overflow:auto;text-align:start}
.lab-controls{display:grid;gap:8px;margin:6px 0 12px}
.lab-controls label{display:flex;align-items:center;gap:8px;color:var(--ink)}
.lab-controls select,.lab-controls input[type=datetime-local]{font:inherit;padding:4px 6px;border-radius:8px;border:1.5px solid var(--control-border);background:var(--panel);color:var(--ink)}
.lab-row{display:flex;flex-wrap:wrap;gap:8px;align-items:center}
.lab-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(132px,1fr));gap:10px}
.lab-tile{display:grid;gap:4px;padding:6px;border-radius:12px;border:1.5px solid var(--control-border);background:var(--panel-raised);color:var(--ink);font:inherit;text-align:start;cursor:pointer}
.lab-tile.on{border-color:var(--glow);box-shadow:0 0 0 2px color-mix(in srgb,var(--glow) 40%,transparent)}
.lab-face{position:relative;display:grid;place-items:center;height:150px;border-radius:9px;overflow:hidden;background-color:var(--bg-color);background-image:var(--page-image)}
.lab-art{position:absolute;inset:0;background-size:100% 100%;background-repeat:no-repeat}
.lab-orb{position:relative;width:54px;height:54px;border-radius:50%;background:var(--orb-art) center/100% 100% no-repeat,radial-gradient(circle at 35% 30%,color-mix(in srgb,var(--orb-color) 30%,var(--color-white)),var(--orb-color) 60%,color-mix(in srgb,var(--orb-color) 70%,var(--color-black)));box-shadow:0 0 18px color-mix(in srgb,var(--glow) 60%,transparent)}
.lab-word{position:absolute;inset-inline:10px;inset-block-end:28px;padding:2px 6px;border-radius:6px;background:var(--panel);color:var(--ink);font:700 13px/1.3 var(--display);text-align:center}
.lab-btn{position:absolute;inset-inline:22px;inset-block-end:6px;padding:1px 6px;border-radius:6px;background:var(--button-color);color:var(--button-ink);font:700 11px/1.4 var(--body);text-align:center}
.lab-hint{position:absolute;inset-block-start:8px;inset-inline:0;text-align:center;color:var(--muted-on-bg);font:600 11px/1.2 var(--mono);letter-spacing:.2em}
.lab-name{font-weight:700}
.lab-meta,.lab-state{color:var(--muted);font-size:12px}
.lab-fact{color:var(--ink);font-size:13px;margin:4px 0}
";

fn install_style() {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    if doc.get_element_by_id("themeLabStyle").is_some() {
        return;
    }
    if let (Ok(el), Some(body)) = (doc.create_element("style"), doc.body()) {
        el.set_id("themeLabStyle");
        el.set_text_content(Some(STYLE));
        let _ = body.append_child(&el);
    }
}

/// The dev-menu row, the panel and the chip: all made here (none in index.html).
pub fn wire() {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    install_style();
    if let Ok(Some(list)) = doc.query_selector("#devMenu .dev-list") {
        if let Ok(btn) = doc.create_element("button") {
            btn.set_id("devOpenThemeLab");
            btn.set_class_name("dev-item");
            let _ = btn.set_attribute("type", "button");
            btn.set_text_content(Some("Theme Lab"));
            let _ = list.prepend_with_node_1(&btn);
        }
        dom::on_click("devOpenThemeLab", || {
            dom::remove_class("devMenu", "show");
            open();
        });
    }
    if let (Ok(panel), Some(body)) = (doc.create_element("div"), doc.body()) {
        panel.set_id("themeLab");
        panel.set_class_name("scrim");
        let _ = body.append_child(&panel);
        let click = Closure::<dyn FnMut(web_sys::Event)>::new(on_panel_click);
        let _ = panel.add_event_listener_with_callback("click", click.as_ref().unchecked_ref());
        click.forget();
        let change = Closure::<dyn FnMut(web_sys::Event)>::new(on_panel_change);
        let _ = panel.add_event_listener_with_callback("change", change.as_ref().unchecked_ref());
        change.forget();
    }
    reflect();
}

fn open() {
    render();
    dom::add_class("themeLab", "show");
}

fn close() {
    dom::remove_class("themeLab", "show");
}

/// Apply the stored preview: the register and motion switches, the chip, then
/// the theme itself (themes::apply reads preview()).
fn reflect() {
    let s = load();
    let doc = web_sys::window().and_then(|w| w.document());
    if let Some(doc) = doc.as_ref() {
        if let Some(root) = doc.document_element() {
            let _ = if s.rm && !s.theme.is_empty() { root.set_attribute("data-rm", "") } else { root.remove_attribute("data-rm") };
        }
        // Jr register: the Lab adds body.kid only if it was not already there,
        // and takes away only what it added.
        if let Some(body) = doc.body() {
            let added = body.has_attribute("data-lab-jr");
            let want = s.jr && !s.theme.is_empty();
            if want && !body.class_list().contains("kid") {
                let _ = body.class_list().add_1("kid");
                let _ = body.set_attribute("data-lab-jr", "");
            } else if !want && added {
                let _ = body.class_list().remove_1("kid");
                let _ = body.remove_attribute("data-lab-jr");
            }
        }
        let chip = doc.get_element_by_id("labChip");
        match Theme::from_id(&s.theme) {
            Some(t) => {
                let chip = chip.or_else(|| {
                    let c = doc.create_element("button").ok()?;
                    c.set_id("labChip");
                    let _ = c.set_attribute("type", "button");
                    let _ = c.set_attribute("aria-label", "Clear the Theme Lab preview");
                    doc.body()?.append_child(&c).ok()?;
                    let cb = Closure::<dyn FnMut()>::new(clear);
                    let _ = c.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
                    cb.forget();
                    Some(c)
                });
                if let Some(c) = chip {
                    c.set_text_content(Some(&format!("LAB · {}", cosmos::entry(t).name)));
                }
            }
            None => {
                if let Some(c) = chip {
                    c.remove();
                }
            }
        }
    }
    crate::themes::apply();
}

fn clear() {
    save(&State::default());
    reflect();
    if dom::exists("themeLab") {
        render();
    }
}

fn set(f: impl FnOnce(&mut State)) {
    let mut s = load();
    f(&mut s);
    save(&s);
    reflect();
    render();
}

fn target_attr(e: &web_sys::Event, attr: &str) -> Option<(web_sys::Element, String)> {
    let el = e.target()?.dyn_into::<web_sys::Element>().ok()?.closest(&format!("[{attr}]")).ok()??;
    let v = el.get_attribute(attr)?;
    Some((el, v))
}

fn on_panel_click(e: web_sys::Event) {
    if dom::is_self_target(&e, "themeLab") {
        close();
        return;
    }
    if let Some((_, id)) = target_attr(&e, "data-lab-apply") {
        set(|s| {
            if s.theme != id {
                s.band.clear();
                s.phase = None;
                s.planet.clear();
            }
            s.theme = id;
        });
        return;
    }
    let Some((_, action)) = target_attr(&e, "data-lab") else { return };
    match action.as_str() {
        "close" => close(),
        "clear" => clear(),
        "clock-clear" => set(|s| s.clock.clear()),
        "unlock" => play_unlock_moment(),
        _ => {}
    }
}

fn on_panel_change(e: web_sys::Event) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
    let Some(which) = el.get_attribute("data-lab-set") else { return };
    let value = el.dyn_ref::<web_sys::HtmlSelectElement>().map(|s| s.value()).or_else(|| el.dyn_ref::<web_sys::HtmlInputElement>().map(|i| i.value())).unwrap_or_default();
    let checked = el.dyn_ref::<web_sys::HtmlInputElement>().is_some_and(|i| i.checked());
    set(|s| match which.as_str() {
        "band" => s.band = value,
        "phase" => s.phase = value.parse().ok(),
        "planet" => s.planet = value,
        "clock" => s.clock = value,
        "jr" => s.jr = checked,
        "rm" => s.rm = checked,
        _ => {}
    });
}

/// F1.5: the theme's unlock celebration and fact card, writing nothing.
fn play_unlock_moment() {
    let s = load();
    let Some(t) = Theme::from_id(&s.theme) else { return };
    let e = cosmos::entry(t);
    let mut msg = format!("{} unlocked", e.name);
    if let Some(f) = e.fact {
        msg.push_str(" · ");
        msg.push_str(f);
    }
    if t == Theme::Solar {
        let p = Planet::from_id(&s.planet).unwrap_or(Planet::Mercury);
        msg = format!("{} unlocked · {}", p.name(), p.fact());
    }
    close();
    crate::celebrate::preview_big(&msg);
}

fn commas(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// F1.7: read-only, the counter against the threshold.
fn unlock_state(t: Theme) -> String {
    let daily = crate::daily::load();
    let p = crate::themes::load().progress(daily.best_streak);
    let of = |n: u32, bar: u32, what: &str| format!("{} / {} {what}", commas(n), commas(bar));
    match t {
        Theme::Eclipse | Theme::Daybreak => "Free".into(),
        Theme::Chalkboard | Theme::Typewriter | Theme::BeeStage => {
            if crate::themes::load().unlocked.iter().any(|u| u == t.id()) { "Unlocked".into() } else { "Locked".into() }
        }
        Theme::Bronze => of(p.lifetime_correct, 250, "correct words"),
        Theme::Silver => of(p.lifetime_correct, 1_500, "correct words"),
        Theme::Gold => of(p.lifetime_correct, 5_000, "correct words"),
        Theme::Sky => of(daily.best_streak, 3, "day streak"),
        Theme::Solar => {
            let n = daily.history.len() as u32;
            format!("{n} Dailies: {} of 8 planets", (n / 5 + 1).min(8))
        }
        Theme::Diamond => "Impossible correct: no counter yet (census C4)".into(),
        Theme::Lunar => "Principal phases seen: no counter yet (C4)".into(),
        Theme::Aurora => "Days played: no counter yet (C4)".into(),
        Theme::Beach => "Sessions: no counter yet (C4)".into(),
        Theme::PinkMoon | Theme::BloodMoon => "Full moons played: no counter yet (C4)".into(),
        Theme::GreatWall => "Threshold not signed (O1, O2)".into(),
    }
}

fn opt(value: &str, label: &str, on: bool) -> String {
    format!("<option value=\"{value}\"{}>{}</option>", if on { " selected" } else { "" }, dom::escape_html(label))
}

fn render() {
    let s = load();
    let applied = Theme::from_id(&s.theme);
    let mut h = String::new();
    h.push_str("<div class=\"modal\"><button class=\"ghost hub-close\" type=\"button\" data-lab=\"close\">Close</button><h3>Theme Lab</h3>");
    h.push_str("<div class=\"lab-controls\">");
    let name = applied.map(|t| cosmos::entry(t).name).unwrap_or("None (the player's own theme)");
    h.push_str(&format!("<div class=\"lab-row\"><b>Applied:</b> {}", dom::escape_html(name)));
    if applied.is_some() {
        h.push_str(" <button class=\"ghost\" type=\"button\" data-lab=\"clear\">Clear</button> <button class=\"ghost\" type=\"button\" data-lab=\"unlock\">Play unlock moment</button>");
    }
    h.push_str("</div>");
    if let Some(t) = applied {
        if let Some(f) = cosmos::entry(t).fact {
            h.push_str(&format!("<div class=\"lab-fact\">{}</div>", dom::escape_html(f)));
        }
        match t {
            Theme::Sky => {
                let b = cosmos::band_from_id(&s.band);
                let mut sel = String::from("<label>Band <select data-lab-set=\"band\">");
                sel.push_str(&opt("", "Follow clock", b.is_none()));
                for band in [Band::Sunrise, Band::Day, Band::Sunset, Band::Night] {
                    let id = cosmos::band_id(band);
                    sel.push_str(&opt(id, &format!("{}{}", id[..1].to_uppercase(), &id[1..]), b == Some(band)));
                }
                sel.push_str("</select></label>");
                h.push_str(&sel);
            }
            Theme::Lunar => {
                let mut sel = String::from("<label>Phase <select data-lab-set=\"phase\">");
                sel.push_str(&opt("", "Follow date", s.phase.is_none()));
                for (i, n) in cosmos::PHASE_NAMES.iter().enumerate() {
                    sel.push_str(&opt(&i.to_string(), n, s.phase == Some(i as u8)));
                }
                sel.push_str("</select></label>");
                h.push_str(&sel);
            }
            Theme::Solar => {
                let chosen = Planet::from_id(&s.planet).unwrap_or(Planet::Mercury);
                let mut sel = String::from("<label>Planet <select data-lab-set=\"planet\">");
                for p in PLANETS {
                    sel.push_str(&opt(p.id(), p.name(), p == chosen));
                }
                sel.push_str("</select></label>");
                h.push_str(&sel);
                h.push_str(&format!("<div class=\"lab-fact\">{}: {}</div>", chosen.name(), dom::escape_html(chosen.fact())));
            }
            _ => {}
        }
        h.push_str(&format!(
            "<label><input type=\"checkbox\" data-lab-set=\"jr\"{}> Jr register</label><label><input type=\"checkbox\" data-lab-set=\"rm\"{}> Simulate Reduce Motion</label>",
            if s.jr { " checked" } else { "" },
            if s.rm { " checked" } else { "" }
        ));
    }
    h.push_str(&format!(
        "<label>Clock <input type=\"datetime-local\" data-lab-set=\"clock\" value=\"{}\"> <button class=\"ghost\" type=\"button\" data-lab=\"clock-clear\">Follow device</button></label>",
        dom::escape_html(&s.clock)
    ));
    h.push_str("<div class=\"lab-meta\">The clock reaches only the Sky band and the moon (I-T11). The Lab writes no unlocks or progress.</div>");
    h.push_str("</div><div class=\"lab-grid\">");
    let now = parse_clock(&s.clock).unwrap_or_else(crate::themes::local_now);
    for t in REGISTRY {
        let e = cosmos::entry(t);
        let look = cosmos::look(t, now, if Some(t) == applied { pins(&s) } else { Pins::default() });
        let mut scope = format!("data-theme-scope=\"{}\"", t.id());
        if let Some(b) = look.band {
            scope.push_str(&format!(" data-band=\"{}\"", cosmos::band_id(b)));
        }
        if let Some(p) = look.planet {
            scope.push_str(&format!(" data-planet=\"{}\"", p.id()));
        }
        let wave = if e.wave == 0 { "Classic".to_string() } else { format!("{} · wave {}", e.family.name(), e.wave) };
        h.push_str(&format!(
            "<button type=\"button\" class=\"lab-tile{}\" data-lab-apply=\"{}\"><span class=\"lab-face\" {scope}><span class=\"lab-art\" data-lab-art=\"{}\"></span><span class=\"lab-hint\">S _ E L L</span><span class=\"lab-orb\"></span><span class=\"lab-word\">spell</span><span class=\"lab-btn\">Check</span></span><span class=\"lab-name\">{}</span><span class=\"lab-meta\">{}</span><span class=\"lab-meta\">{}</span><span class=\"lab-state\">{}</span></button>",
            if Some(t) == applied { " on" } else { "" },
            t.id(),
            t.id(),
            dom::escape_html(e.name),
            dom::escape_html(&wave),
            dom::escape_html(e.unlock),
            dom::escape_html(&unlock_state(t)),
        ));
    }
    h.push_str("</div></div>");
    dom::set_html("themeLab", &h);
    // The art, at tile scale, through the DOM (its SVG holds quotes an attribute cannot).
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    for t in REGISTRY {
        let look = cosmos::look(t, now, if Some(t) == applied { pins(&s) } else { Pins::default() });
        let Ok(Some(tile)) = doc.query_selector(&format!("[data-lab-apply=\"{}\"] .lab-face", t.id())) else { continue };
        let Ok(face) = tile.dyn_into::<web_sys::HtmlElement>() else { continue };
        if let Some(svg) = crate::theme_art::orb_svg(&look) {
            let _ = face.style().set_property("--orb-art", &crate::theme_art::css_url(&svg));
        }
        if let (Some(svg), Ok(Some(art))) = (crate::theme_art::page_svg(&look, 130.0, 150.0), face.query_selector(".lab-art")) {
            if let Ok(art) = art.dyn_into::<web_sys::HtmlElement>() {
                let _ = art.style().set_property("background-image", &crate::theme_art::css_url(&svg));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_override_parses_a_datetime_local_value() {
        assert_eq!(parse_clock("2027-04-20T21:05"), Some(LocalNow { y: 2027, m: 4, d: 20, h: 21, min: 5 }));
        assert_eq!(parse_clock(""), None);
        assert_eq!(parse_clock("2027-04-20"), None);
    }

    #[test]
    fn commas_group_thousands() {
        assert_eq!(commas(5000), "5,000");
        assert_eq!(commas(250), "250");
        assert_eq!(commas(1_234_567), "1,234,567");
    }
}
