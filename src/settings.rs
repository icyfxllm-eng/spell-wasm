use wasm_bindgen::JsCast;

use crate::audio_boost;
use crate::dom;
use crate::drawing;
use crate::model::{AppState, Prefs, PREFS_KEY};
use crate::storage;
use crate::App;

pub fn load_prefs(state: &mut AppState) {
    let p: Prefs = storage::get_json(PREFS_KEY).unwrap_or_default();
    state.glow = p.glow.unwrap_or_else(|| "#ffb14d".to_string());
    state.bg_color = p.bg_color.unwrap_or_else(|| "#1c1830".to_string());
    state.orb_color = p.orb_color.unwrap_or_else(|| "#ffb14d".to_string());
    state.last_lang = p.last_lang;
    state.kid = p.kid;
    state.readable = p.readable;
    state.big_text = p.big_text;
    state.slow = p.slow;
    state.rate = if state.slow { 0.7 } else { 0.9 };
    state.volume = audio_boost::clamp_gain(p.volume.unwrap_or(1.0));
    state.remind = p.remind;
    state.remind_time = p.remind_time.filter(|t| !t.is_empty()).unwrap_or_else(|| "17:00".to_string());
}

pub fn save_prefs(state: &AppState) {
    let p = Prefs {
        glow: Some(state.glow.clone()),
        bg_color: Some(state.bg_color.clone()),
        orb_color: Some(state.orb_color.clone()),
        last_lang: Some(state.lang.clone()),
        kid: state.kid,
        readable: state.readable,
        big_text: state.big_text,
        slow: state.slow,
        volume: Some(state.volume),
        remind: state.remind,
        remind_time: Some(state.remind_time.clone()),
    };
    storage::set_json(PREFS_KEY, &p);
}

/// (Re)apply the daily-reminder schedule from current state. Kid Mode
/// suppresses it regardless of the toggle (CLAUDE.md: off in Kid Mode).
pub fn apply_reminder(state: &AppState) {
    crate::notifications::apply(state.remind && !state.kid, &state.remind_time);
}

/// Marks whichever swatch under `container_id` matches `color` as active,
/// clearing the others.
fn sync_swatches(container_id: &str, selector: &str, color: &str) {
    let full = format!("#{container_id} {selector}");
    if let Ok(list) = dom::doc().query_selector_all(&full) {
        for i in 0..list.length() {
            if let Some(node) = list.get(i) {
                if let Some(el) = node.dyn_ref::<web_sys::Element>() {
                    let matches = el.get_attribute("data-c").map(|c| c.eq_ignore_ascii_case(color)).unwrap_or(false);
                    let _ = el.class_list().toggle_with_force("active", matches);
                }
            }
        }
    }
}

pub fn set_glow(app: &App, color: &str) {
    app.borrow_mut().glow = color.to_string();
    dom::set_css_var("--glow", color);
    dom::input("glowColor").set_value(color);
    drawing::set_color(color);
    sync_swatches("glowPick", ".swatch[data-c]", color);
    save_prefs(&app.borrow());
}

pub fn set_bg_color(app: &App, color: &str) {
    app.borrow_mut().bg_color = color.to_string();
    dom::set_css_var("--bg-color", color);
    // Text that sits directly on the page background (not inside a
    // --panel/--panel-2 card, which stays a fixed color regardless of this
    // choice) needs to flip to whichever of light/dark ink actually
    // contrasts better against the chosen color, or it can wash out to the
    // point of being unreadable — a fixed brightness threshold isn't
    // enough, since saturated hues (a medium blue, say) can *look*
    // reasonably bright while still contributing little to perceived
    // luminance, making light text the wrong call despite "looking dark".
    dom::set_css_var("--ink-on-bg", best_contrast_ink(color));
    // --muted-on-bg is derived from --ink-on-bg via CSS color-mix() (see
    // index.html), so it automatically stays legible relative to whichever
    // ink color was just picked, instead of a second independent guess.
    dom::input("bgColorInput").set_value(color);
    sync_swatches("bgPick", ".theme-swatch[data-c]", color);
    save_prefs(&app.borrow());
}

/// WCAG relative luminance (0 = black, 1 = white) for a `#rrggbb` color;
/// `None` if it doesn't parse as one. Unlike a plain weighted RGB average,
/// this gamma-corrects each channel first, which is what makes it accurate
/// for saturated/mid-brightness hues (a pure blue "looks" brighter to the
/// eye than this formula's blue weight alone would suggest — the gamma
/// correction is what accounts for that).
fn rel_luminance(hex: &str) -> Option<f64> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let linearize = |c: f64| {
        let c = c / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let component = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0) as f64;
    let (r, g, b) = (linearize(component(0)), linearize(component(2)), linearize(component(4)));
    Some(0.2126 * r + 0.7152 * g + 0.0722 * b)
}

/// WCAG contrast ratio between two relative luminances (1.0 = no contrast,
/// 21.0 = max, black on white).
fn contrast_ratio(l1: f64, l2: f64) -> f64 {
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

const LIGHT_INK: &str = "#efeaf7";
const DARK_INK: &str = "#191527";

/// Picks whichever of a light or dark ink color has better contrast against
/// `hex`, by actual contrast ratio rather than a brightness guess.
fn best_contrast_ink(hex: &str) -> &'static str {
    let Some(bg_lum) = rel_luminance(hex) else { return LIGHT_INK };
    let light_lum = rel_luminance(LIGHT_INK).unwrap_or(0.9);
    let dark_lum = rel_luminance(DARK_INK).unwrap_or(0.01);
    if contrast_ratio(bg_lum, light_lum) >= contrast_ratio(bg_lum, dark_lum) {
        LIGHT_INK
    } else {
        DARK_INK
    }
}

/// Picks readable orb-glyph text (dark or light) for whatever orb color the
/// user chose, by the same actual-contrast-ratio logic as the page
/// background above (previously a fixed threshold, same blind spot for
/// saturated hues).
fn contrasting_text(hex: &str) -> &'static str {
    if best_contrast_ink(hex) == DARK_INK {
        "#2a1a06"
    } else {
        "#fdf6ec"
    }
}

pub fn set_orb_color(app: &App, color: &str) {
    app.borrow_mut().orb_color = color.to_string();
    dom::set_css_var("--orb-color", color);
    dom::set_css_var("--orb-text", contrasting_text(color));
    dom::input("orbColorInput").set_value(color);
    sync_swatches("orbPick", ".theme-swatch[data-c]", color);
    save_prefs(&app.borrow());
}

/// Sets the shared playback volume/boost (0.5-2.0), persists it, and syncs
/// the settings slider to match.
pub fn set_volume(app: &App, value: f32) {
    let value = audio_boost::clamp_gain(value);
    app.borrow_mut().volume = value;
    audio_boost::set_gain(value);
    dom::input("volumeSlider").set_value(&value.to_string());
    save_prefs(&app.borrow());
}

pub fn apply_settings(app: &App) {
    {
        let mut s = app.borrow_mut();
        s.rate = if s.slow { 0.7 } else { 0.9 };
    }
    let s = app.borrow();
    let body = dom::doc().body().unwrap();
    let _ = body.class_list().toggle_with_force("kid", s.kid);
    let _ = body.class_list().toggle_with_force("readable", s.readable);
    let _ = body.class_list().toggle_with_force("big-text", s.big_text);
    dom::input("kidToggle").set_checked(s.kid);
    dom::input("readToggle").set_checked(s.readable);
    dom::input("bigTextToggle").set_checked(s.big_text);
    dom::input("slowToggle").set_checked(s.slow);
    dom::input("remindToggle").set_checked(s.remind);
    dom::input("remindTime").set_value(&s.remind_time);
    dom::input("volumeSlider").set_value(&s.volume.to_string());
    audio_boost::set_gain(s.volume);
    drop(s);
    // Kid Mode hides the account/leaderboard entry points entirely.
    crate::climb::reflect_auth();
}
