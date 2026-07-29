//! CC-WORD-PICTURE v5 screen — picker + calligram canvas.
//!
//! Rendering rules (D1/D4, Eric's curve ruling):
//!  * flow paths → SVG <textPath> for scripts that shape safely along curves;
//!    complex-shaping scripts (ar, hi) render the word STRAIGHT, rotated to
//!    the path's chord angle (joins survive; the spike showed textPath breaks
//!    them). CJK/hangul ride textPath too — no joining, glyphs place cleanly.
//!  * stacks → one <text> per NFC unit, top-down (script-universal).
//! Grow-only: placed words only ever accumulate (engine enforces).

use std::cell::{Cell, RefCell};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::{api, dom, haptics, i18n, wordpic, App};

thread_local! {
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    static PIC: RefCell<String> = const { RefCell::new(String::new()) };
    static FEED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static PLACED: Cell<u32> = const { Cell::new(0) };
    static OPEN: Cell<bool> = const { Cell::new(false) };
    static CARD_UP: Cell<bool> = const { Cell::new(false) };
    static HOW_STEP: Cell<u8> = const { Cell::new(0) };
    static MILESTONE: Cell<u32> = const { Cell::new(0) };
}

/// Scripts whose shaping must not ride a curved textPath (D4 ruling).
fn complex_script(lang: &str) -> bool {
    matches!(lang, "ar" | "hi")
}

pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("wordPicOpen", move || open_picker(&a));
    let a = app.clone();
    dom::on_click("wpPickerExit", move || {
        dom::remove_class("wpPicker", "show");
        let _ = &a;
    });
    let a = app.clone();
    dom::on_click("wpExit", move || close_play(&a));
    let a = app.clone();
    dom::on_click("wpReplay", move || replay(&a));
    let a = app.clone();
    dom::on_click("wpRestart", move || {
        if !CARD_UP.with(Cell::get) {
            dom::add_class("wpConfirm", "show");
            CARD_UP.with(|c| c.set(true));
        }
        let _ = &a;
    });
    let a = app.clone();
    dom::on_click("wpConfirmYes", move || {
        let (pic, lang) = (PIC.with(|p| p.borrow().clone()), LANG.with(|l| l.borrow().clone()));
        let mut s = wordpic::load();
        s.restart(&pic, &lang);
        wordpic::save(&s);
        dom::remove_class("wpConfirm", "show");
        CARD_UP.with(|c| c.set(false));
        open_play(&a, &pic);
    });
    dom::on_click("wpConfirmNo", || {
        dom::remove_class("wpConfirm", "show");
        CARD_UP.with(|c| c.set(false));
    });
    let a = app.clone();
    dom::on_click("wpHowNext", move || how_next(&a));
    let a = app.clone();
    dom::on_click("wpSurprise", move || {
        let lang = a.borrow().lang.clone();
        let mut s = wordpic::load();
        let startable: Vec<String> = startable_ids(&a, &s, &lang);
        if let Some(id) = wordpic::surprise(&mut s, &startable) {
            wordpic::save(&s);
            open_play(&a, &id);
        }
    });
    let a = app.clone();
    dom::on_click("wpShare", move || {
        let lang = LANG.with(|l| l.borrow().clone());
        let pic = PIC.with(|p| p.borrow().clone());
        let n = PLACED.with(Cell::get);
        crate::share::share_wordpic(&lang, &pic, n);
        let _ = &a;
    });
    let a = app.clone();
    dom::on_click("wpAgain", move || {
        // D15c: fresh seed over the same paths — a new artwork.
        let (pic, lang) = (PIC.with(|p| p.borrow().clone()), LANG.with(|l| l.borrow().clone()));
        let mut s = wordpic::load();
        s.restart(&pic, &lang);
        let _ = s.open(&pic, &lang);
        wordpic::save(&s);
        dom::remove_class("wpDone", "show");
        CARD_UP.with(|c| c.set(false));
        open_play(&a, &pic);
    });
    let a = app.clone();
    dom::on_click("wpDoneClose", move || {
        dom::remove_class("wpDone", "show");
        CARD_UP.with(|c| c.set(false));
        close_play(&a);
    });
    let a = app.clone();
    dom::on::<web_sys::Event, _>("wpInput", "input", move |_| on_typed(&a));
}

fn startable_ids(app: &App, state: &wordpic::State, lang: &str) -> Vec<String> {
    wordpic::picker_order(state, lang)
        .into_iter()
        .filter(|p| allowed(app, p, lang))
        .map(|p| p.id.clone())
        .collect()
}

/// D16 gating: free trio at PREVIEW, everything with Complete; Kid Mode uses
/// the computed eligibility.
fn allowed(app: &App, p: &wordpic::Picture, lang: &str) -> bool {
    if !wordpic::playable(p, lang) {
        return false;
    }
    let kid = app.borrow().kid;
    if kid && !wordpic::kid_ok(p, lang, 1) {
        return false;
    }
    let ent = crate::play_hub::live_entitlements();
    let full = matches!(ent.lang_level(lang), crate::entitlements::AccessLevel::Full);
    full || wordpic::manifest().free.contains(&p.id)
}

pub fn open_picker(app: &App) {
    let lang = app.borrow().lang.clone();
    LANG.with(|l| *l.borrow_mut() = lang.clone());
    let state = wordpic::load();
    let mut html = String::new();
    for p in wordpic::picker_order(&state, &lang) {
        let total = wordpic::slots(p);
        let (cls, prog) = match state.run(&p.id, &lang) {
            Some(r) if r.done => ("wp-tile done", i18n::t("wordpic.done")),
            Some(r) if !r.words.is_empty() => ("wp-tile", format!("{}/{}", r.words.len(), total)),
            _ => ("wp-tile", format!("{total}")),
        };
        let locked = !allowed(app, p, &lang);
        html.push_str(&format!(
            "<button type=\"button\" class=\"{}{}\" data-pic=\"{}\" {}><span class=\"ico\">{}</span><span class=\"prog\">{}</span></button>",
            cls,
            if locked { " locked" } else { "" },
            p.id,
            if locked { "disabled" } else { "" },
            p.icon,
            prog
        ));
    }
    dom::set_html("wpGrid", &html);
    // Delegate tile taps once per open (idempotent listener via fresh nodes).
    let a = app.clone();
    dom::on::<web_sys::MouseEvent, _>("wpGrid", "click", move |e| {
        let Some(el) = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|t| t.closest("[data-pic]").ok().flatten())
        else {
            return;
        };
        if let Some(id) = el.get_attribute("data-pic") {
            open_play(&a, &id);
        }
    });
    dom::add_class("wpPicker", "show");
}

fn open_play(app: &App, pic_id: &str) {
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(p) = wordpic::picture(pic_id) else { return };
    let mut s = wordpic::load();
    let run = s.open(pic_id, &lang);
    let first_time = !s.how_shown;
    wordpic::save(&s);
    PIC.with(|x| *x.borrow_mut() = pic_id.to_string());
    PLACED.with(|c| c.set(run.words.len() as u32));
    MILESTONE.with(|c| c.set(0));
    let feed = wordpic::word_feed(
        p,
        &lang,
        run.seed,
        wordpic::load().recent_words.get(&lang).map(|v| v.as_slice()).unwrap_or(&[]),
    );
    FEED.with(|f| *f.borrow_mut() = feed);
    dom::set_text("wpPlayTitle", &format!("{} {}", p.icon, i18n::t("tools.wordpic.name")));
    render_canvas(p, &lang, &run.words);
    reflect_count(p);
    OPEN.with(|c| c.set(true));
    dom::remove_class("wpPicker", "show");
    dom::add_class("wpPlay", "show");
    if let Ok(inp) = dom::el("wpInput").dyn_into::<web_sys::HtmlInputElement>() {
        inp.set_value("");
        let _ = inp.focus();
    }
    if first_time {
        HOW_STEP.with(|c| c.set(0));
        how_next(app); // shows card 1; advances on taps
    } else if run.words.len() < FEED.with(|f| f.borrow().len()) {
        replay(app);
    } else {
        show_done(app);
    }
}

fn how_next(app: &App) {
    let step = HOW_STEP.with(Cell::get);
    if step < 3 {
        dom::set_text("wpHowText", &i18n::t(&format!("wordpic.how{}", step + 1)));
        dom::add_class("wpHow", "show");
        CARD_UP.with(|c| c.set(true));
        HOW_STEP.with(|c| c.set(step + 1));
    } else {
        dom::remove_class("wpHow", "show");
        CARD_UP.with(|c| c.set(false));
        let mut s = wordpic::load();
        s.how_shown = true;
        wordpic::save(&s);
        replay(app);
    }
}

fn close_play(app: &App) {
    OPEN.with(|c| c.set(false));
    dom::remove_class("wpPlay", "show");
    for id in ["wpHow", "wpConfirm", "wpDone"] {
        dom::remove_class(id, "show");
    }
    CARD_UP.with(|c| c.set(false));
    api::stop();
    open_picker(app);
}

/// Chord endpoints of an SVG path `d` (first and last coordinate pairs).
fn chord(d: &str) -> ((f32, f32), (f32, f32)) {
    let nums: Vec<f32> = d
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    if nums.len() < 4 {
        return ((0.0, 0.0), (0.0, 0.0));
    }
    ((nums[0], nums[1]), (nums[nums.len() - 2], nums[nums.len() - 1]))
}

/// Build the SVG canvas: ghost guides for every path, placed words rendered,
/// the NEXT path highlighted.
fn render_canvas(p: &wordpic::Picture, lang: &str, words: &[String]) {
    let budgets = wordpic::slot_budgets(p);
    let mut svg = String::from(
        "<svg viewBox=\"0 0 512 512\" xmlns=\"http://www.w3.org/2000/svg\">",
    );
    // Defs: flow geometry (referenced by textPath).
    svg.push_str("<defs>");
    for (i, q) in p.paths.iter().enumerate() {
        if let Some(d) = &q.d {
            svg.push_str(&format!("<path id=\"wpp{i}\" d=\"{}\"/>", d));
        }
    }
    svg.push_str("</defs>");
    let next_slot = words.len();
    // Ghost guides (skip paths whose every slot is already worded).
    let mut slot_cursor = 0usize;
    for (i, q) in p.paths.iter().enumerate() {
        let nslots = q.slots() as usize;
        let done_here = words.len() >= slot_cursor + nslots;
        let is_next = (slot_cursor..slot_cursor + nslots).contains(&next_slot);
        if !done_here {
            match q.mode.as_str() {
                "stack" => {
                    let n = q.budget.1.min(6);
                    for k in 0..n {
                        svg.push_str(&format!(
                            "<rect class=\"wp-ghost{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"5\"/>",
                            if is_next { " next" } else { "" },
                            q.x - q.size * 0.42,
                            q.y + k as f32 * q.size - q.size * 0.72,
                            q.size * 0.84,
                            q.size * 0.84
                        ));
                    }
                }
                _ => {
                    svg.push_str(&format!(
                        "<use href=\"#wpp{i}\" class=\"wp-ghost{}\"/>",
                        if is_next { " next" } else { "" }
                    ));
                }
            }
        }
        slot_cursor += nslots;
    }
    // Placed words.
    let mut slot_cursor = 0usize;
    for (i, q) in p.paths.iter().enumerate() {
        let nslots = q.slots() as usize;
        for k in 0..nslots {
            let idx = slot_cursor + k;
            if idx >= words.len() {
                break;
            }
            let w = &words[idx];
            let newest = idx + 1 == words.len();
            let cls = if newest { "wp-word new" } else { "wp-word" };
            match q.mode.as_str() {
                "stack" => {
                    let units: Vec<char> = w.chars().collect();
                    let size = q.size.min(q.size * 6.0 / units.len().max(1) as f32).max(wordpic::MIN_FONT);
                    for (u, ch) in units.iter().enumerate() {
                        svg.push_str(&format!(
                            "<text class=\"{cls}\" x=\"{}\" y=\"{}\" font-size=\"{size}\" text-anchor=\"middle\">{}</text>",
                            q.x,
                            q.y + u as f32 * size,
                            dom::escape_html(&ch.to_string())
                        ));
                    }
                }
                _ if complex_script(lang) => {
                    // D4 ruling: straight word rotated to the chord angle.
                    let ((x0, y0), (x1, y1)) = chord(q.d.as_deref().unwrap_or(""));
                    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                    let ang = (y1 - y0).atan2(x1 - x0).to_degrees();
                    let seg_len = ((x1 - x0).hypot(y1 - y0)) / nslots as f32;
                    let off = (k as f32 + 0.5) / nslots as f32;
                    let (px, py) = (x0 + (x1 - x0) * off, y0 + (y1 - y0) * off);
                    let size = (seg_len / w.chars().count().max(1) as f32 * 1.5).clamp(16.0, 44.0);
                    let _ = (cx, cy);
                    svg.push_str(&format!(
                        "<text class=\"{cls}\" x=\"{px}\" y=\"{py}\" font-size=\"{size}\" text-anchor=\"middle\" transform=\"rotate({ang:.1} {px} {py})\">{}</text>",
                        dom::escape_html(w)
                    ));
                }
                _ => {
                    // Curved paths run ~1.5x their chord; center each segment's
                    // word at its midpoint (anchor middle) so arcs read as arcs.
                    let ((x0, y0), (x1, y1)) = chord(q.d.as_deref().unwrap_or(""));
                    let curvy = q.d.as_deref().map(|d| d.contains('Q') || d.contains('A')).unwrap_or(false);
                    let approx_len =
                        (x1 - x0).hypot(y1 - y0).max(40.0) * if curvy { 1.55 } else { 1.0 };
                    let seg = approx_len / nslots as f32;
                    let size = (seg / w.chars().count().max(1) as f32 * 1.5).clamp(16.0, 44.0);
                    let mid = ((k as f32 * 2.0 + 1.0) / (nslots as f32 * 2.0)) * 100.0;
                    svg.push_str(&format!(
                        "<text class=\"{cls}\" font-size=\"{size}\" text-anchor=\"middle\"><textPath href=\"#wpp{i}\" startOffset=\"{mid:.0}%\">{}</textPath></text>",
                        dom::escape_html(w)
                    ));
                }
            }
        }
        slot_cursor += nslots;
    }
    svg.push_str("</svg>");
    dom::set_html("wpStage", &svg);
}

fn reflect_count(p: &wordpic::Picture) {
    let total = wordpic::slots(p);
    dom::set_text("wpCount", &format!("{}/{}", PLACED.with(Cell::get), total));
}

fn current_word() -> Option<String> {
    FEED.with(|f| f.borrow().get(PLACED.with(Cell::get) as usize).cloned())
}

fn replay(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(w) = current_word() else { return };
    let code = format!("{}-{}", lang, lang.to_uppercase());
    let _ = app;
    api::play_word(&w.clone(), "slow", 1.0, &lang, move || {
        crate::speech_out::speak(&w, 0.55, &code)
    });
}

fn on_typed(app: &App) {
    if CARD_UP.with(Cell::get) {
        return;
    }
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(target) = current_word() else { return };
    let Ok(inp) = dom::el("wpInput").dyn_into::<web_sys::HtmlInputElement>() else { return };
    let value = inp.value();
    let (ok, complete) = crate::practice::check_prefix(&lang, &target, &value);
    if !crate::practice::prefix_viable(&lang, &target, &value) && !complete {
        // D5: pulse + replay + hold; the canvas is untouched (grow-only).
        let good: String = target.chars().take(ok).collect();
        inp.set_value(&good);
        dom::add_class("wpStatus", "pulse");
        after(400, || dom::remove_class("wpStatus", "pulse"));
        haptics::key_tap();
        replay(app);
        return;
    }
    if complete {
        inp.set_value("");
        place(app, &target);
    }
}

fn place(app: &App, word: &str) {
    let lang = LANG.with(|l| l.borrow().clone());
    let pic = PIC.with(|p| p.borrow().clone());
    let Some(p) = wordpic::picture(&pic) else { return };
    let total = wordpic::slots(p);
    let mut s = wordpic::load();
    let placed = s.place(&pic, &lang, word, total);
    wordpic::save(&s);
    PLACED.with(|c| c.set(placed));
    haptics::key_tap();
    let words = s.run(&pic, &lang).map(|r| r.words.clone()).unwrap_or_default();
    render_canvas(p, &lang, &words);
    reflect_count(p);
    // D6 milestones at 25/50/75%.
    let pct = placed * 100 / total.max(1);
    let stage = MILESTONE.with(Cell::get);
    for (i, gate) in [25u32, 50, 75].iter().enumerate() {
        if pct >= *gate && stage <= i as u32 {
            MILESTONE.with(|c| c.set(i as u32 + 1));
            dom::set_text("wpStatus", &format!("{} {gate}%", i18n::t("wordpic.milestone")));
            after(1800, || dom::set_text("wpStatus", ""));
            break;
        }
    }
    if placed >= total {
        show_done(app);
    } else {
        let a = app.clone();
        after(650, move || replay(&a));
    }
}

fn show_done(app: &App) {
    let pic = PIC.with(|p| p.borrow().clone());
    let n = PLACED.with(Cell::get);
    dom::set_text(
        "wpDoneBody",
        &i18n::tp("wordpic.doneBody", &[("n", &n.to_string())]),
    );
    dom::add_class("wpDone", "show");
    CARD_UP.with(|c| c.set(true));
    let _ = (app, pic);
}

fn after(ms: i32, f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    if let Some(win) = web_sys::window() {
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms);
    }
}
